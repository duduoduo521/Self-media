use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use self_media_core::task::model::Task;
use self_media_core::task::executor::{TaskExecutor, ExecutionContext};

use crate::{ApiOk, AppState, AuthUser, WebError};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tasks).post(create_task))
        .route("/{id}", get(get_task).delete(cancel_task))
        .route("/{id}/execute", post(execute_task))
}

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub mode: self_media_core::types::TaskMode,
    pub topic: String,
    pub platforms: Vec<self_media_core::types::Platform>,
}

#[derive(serde::Serialize)]
pub struct TaskResponse {
    pub task: Task,
}

async fn create_task(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateTaskRequest>,
) -> Result<ApiOk<TaskResponse>, WebError> {
    let scheduler = state.task_scheduler.lock().await;
    let task = scheduler
        .create_task(auth.user_id, body.mode, body.topic, body.platforms)
        .await?;
    Ok(ApiOk(TaskResponse { task }))
}

#[derive(serde::Serialize)]
pub struct TaskListResponse {
    pub tasks: Vec<Task>,
}

async fn list_tasks(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<ApiOk<TaskListResponse>, WebError> {
    let scheduler = state.task_scheduler.lock().await;
    let tasks = scheduler.list_tasks(auth.user_id).await?;
    Ok(ApiOk(TaskListResponse { tasks }))
}

async fn get_task(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiOk<TaskResponse>, WebError> {
    let scheduler = state.task_scheduler.lock().await;
    let task = scheduler.get_task(&id).await?;
    // 权限校验：只能查看自己的任务
    if task.user_id != auth.user_id {
        return Err(WebError(self_media_core::error::AppError::auth(
            self_media_core::error::AUTH_006,
            "无权访问此任务",
        )));
    }
    Ok(ApiOk(TaskResponse { task }))
}

async fn cancel_task(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiOk<()>, WebError> {
    let scheduler = state.task_scheduler.lock().await;
    // 权限校验：只能取消自己的任务
    let task = scheduler.get_task(&id).await?;
    if task.user_id != auth.user_id {
        return Err(WebError(self_media_core::error::AppError::auth(
            self_media_core::error::AUTH_006,
            "无权操作此任务",
        )));
    }
    scheduler.cancel_task(&id).await?;
    Ok(ApiOk(()))
}

async fn execute_task(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiOk<()>, WebError> {
    let scheduler = state.task_scheduler.lock().await;
    
    // 权限校验：只能执行自己的任务
    let task = scheduler.get_task(&id).await?;
    if task.user_id != auth.user_id {
        return Err(WebError(self_media_core::error::AppError::auth(
            self_media_core::error::AUTH_006,
            "无权操作此任务",
        )));
    }
    
    // 获取用户密钥
    let user_key = state
        .user_key_cache
        .get_or_derive(auth.user_id, &state.db, None)
        .await
        .map_err(|e| WebError(e))?;
    
    // 创建执行上下文
    let ctx = ExecutionContext {
        task: task.clone(),
        user_id: auth.user_id,
        user_key: (*user_key).clone(),
        config_service: state.config_service.clone(),
        db: state.db.clone(),
    };
    
    // 根据任务模式执行
    let result = match task.mode {
        self_media_core::types::TaskMode::Text => {
            TaskExecutor::execute_text_mode(&ctx).await
        }
        self_media_core::types::TaskMode::Video => {
            TaskExecutor::execute_video_mode(&ctx).await
        }
    };
    
    match result {
        Ok(execution_result) => {
            if execution_result.success {
                tracing::info!("任务 {} 执行成功", id);
            } else {
                tracing::warn!("任务 {} 执行失败: {:?}", id, execution_result.error_message);
            }
        }
        Err(e) => {
            tracing::error!("任务 {} 执行出错: {}", id, e);
            ctx.mark_failed(&e.to_string()).await.ok();
            return Err(WebError(e));
        }
    }
    
    Ok(ApiOk(()))
}
