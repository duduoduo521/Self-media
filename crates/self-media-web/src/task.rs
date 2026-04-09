use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;

use self_media_core::task::model::Task;
use self_media_core::types::Platform;

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
    pub platforms: Vec<Platform>,
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
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiOk<TaskResponse>, WebError> {
    let scheduler = state.task_scheduler.lock().await;
    let task = scheduler.get_task(&id).await?;
    Ok(ApiOk(TaskResponse { task }))
}

async fn cancel_task(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiOk<()>, WebError> {
    let scheduler = state.task_scheduler.lock().await;
    scheduler.cancel_task(&id).await?;
    Ok(ApiOk(()))
}

async fn execute_task(
    _auth: AuthUser,
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiOk<()>, WebError> {
    tracing::info!("任务执行请求: {}", id);
    Ok(ApiOk(()))
}
