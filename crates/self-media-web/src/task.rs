use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use self_media_core::draft::Draft;
use self_media_core::task::model::Task;
use self_media_core::task::executor::{TaskExecutor, ExecutionContext};
use self_media_core::types::{Platform, PlatformCredential, ArticleContent, PublishResult};

use crate::{ApiOk, AppState, AuthUser, WebError};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tasks).post(create_task))
        .route("/{id}", get(get_task).delete(cancel_task))
        .route("/{id}/execute", post(execute_task))
        .route("/{id}/save-to-draft", post(save_task_to_draft))
        .route("/{id}/generate", post(generate_to_draft))
}

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub mode: self_media_core::types::TaskMode,
    pub topic: String,
    pub platforms: Vec<self_media_core::types::Platform>,
    #[serde(default)]
    pub event_date: Option<chrono::NaiveDate>,
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
        .create_task(auth.user_id, body.mode, body.topic, body.platforms, body.event_date)
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

async fn save_task_to_draft(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiOk<Draft>, WebError> {
    let scheduler = state.task_scheduler.lock().await;
    let task = scheduler.get_task(&id).await?;

    if task.user_id != auth.user_id {
        return Err(WebError(self_media_core::error::AppError::auth(
            self_media_core::error::AUTH_006,
            "无权操作此任务",
        )));
    }

    if task.status != self_media_core::types::TaskStatus::Completed {
        return Err(WebError(self_media_core::error::AppError::validation(
            "TASK_004",
            "只能保存已完成的任务到草稿箱",
        )));
    }

    let platforms: Vec<Platform> = serde_json::from_str(&task.platforms).unwrap_or_default();
    let result_json = task.result.as_deref().unwrap_or("{}");
    let result_value: serde_json::Value = serde_json::from_str(result_json).unwrap_or_default();

    let original_content = result_value.get("generated_text").and_then(|v| v.as_str()).map(String::from);

    let adapted_contents: Vec<(Platform, String)> = result_value
        .get("adapted_contents")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let platform_str = item.get("platform")?.as_str()?;
                    let content = item.get("content")?.as_str()?;
                    let platform = match platform_str.to_lowercase().as_str() {
                        "xiaohongshu" => Platform::Xiaohongshu,
                        "douyin" => Platform::Douyin,
                        "wechatofficial" | "wechat" => Platform::WeChatOfficial,
                        "bilibili" => Platform::Bilibili,
                        "weibo" => Platform::Weibo,
                        "toutiao" => Platform::Toutiao,
                        _ => return None,
                    };
                    Some((platform, content.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    let generated_images: Vec<String> = result_value
        .get("generated_images")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let draft = state.draft_service
        .create_draft(
            auth.user_id,
            Some(task.id.clone()),
            &task.mode.to_string(),
            &task.topic,
            &platforms,
            original_content.as_deref(),
            &adapted_contents,
            &generated_images,
        )
        .await?;

    Ok(ApiOk(draft))
}

async fn generate_to_draft(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiOk<GenerateToDraftResponse>, WebError> {
    let scheduler = state.task_scheduler.lock().await;
    let task = scheduler.get_task(&id).await?;

    if task.user_id != auth.user_id {
        return Err(WebError(self_media_core::error::AppError::auth(
            self_media_core::error::AUTH_006,
            "无权操作此任务",
        )));
    }

    let user_key = state
        .user_key_cache
        .get_or_derive(auth.user_id, &state.db, None)
        .await
        .map_err(|e| WebError(e))?;

    let ctx = ExecutionContext {
        task: task.clone(),
        user_id: auth.user_id,
        user_key: (*user_key).clone(),
        config_service: state.config_service.clone(),
        user_service: state.user_service.clone(),
        db: state.db.clone(),
    };

    let generation_result = TaskExecutor::generate_content(&ctx).await;

    let generation_result = match generation_result {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("任务 {} 生成失败: {}", id, e);
            ctx.mark_failed(&e.to_string()).await.ok();
            return Err(WebError(e));
        }
    };

    let task_platforms: Vec<Platform> = serde_json::from_str(&task.platforms).unwrap_or_default();

    let draft = state.draft_service
        .create_draft(
            auth.user_id,
            Some(task.id.clone()),
            &task.mode.to_string(),
            &task.topic,
            &task_platforms,
            generation_result.generated_text.as_deref(),
            &[],
            &generation_result.generated_images,
        )
        .await?;

    Ok(ApiOk(GenerateToDraftResponse {
        success: generation_result.success,
        generated_text: generation_result.generated_text,
        generated_images: generation_result.generated_images,
        draft_id: Some(draft.id.clone()),
        error_message: generation_result.error_message,
    }))
}

#[derive(serde::Serialize)]
pub struct GenerateToDraftResponse {
    pub success: bool,
    pub generated_text: Option<String>,
    pub generated_images: Vec<String>,
    pub draft_id: Option<String>,
    pub error_message: Option<String>,
}

#[axum::debug_handler]
async fn execute_task(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiOk<PublishTaskResponse>, WebError> {
    let scheduler = state.task_scheduler.lock().await;
    
    let task = scheduler.get_task(&id).await?;
    if task.user_id != auth.user_id {
        return Err(WebError(self_media_core::error::AppError::auth(
            self_media_core::error::AUTH_006,
            "无权操作此任务",
        )));
    }
    
    let user_key = state
        .user_key_cache
        .get_or_derive(auth.user_id, &state.db, None)
        .await
        .map_err(|e| WebError(e))?;
    
    let ctx = ExecutionContext {
        task: task.clone(),
        user_id: auth.user_id,
        user_key: (*user_key).clone(),
        config_service: state.config_service.clone(),
        user_service: state.user_service.clone(),
        db: state.db.clone(),
    };
    
    let execution_result = match task.get_mode() {
        self_media_core::types::TaskMode::Text => {
            TaskExecutor::execute_text_mode(&ctx).await
        }
        self_media_core::types::TaskMode::Video => {
            TaskExecutor::execute_video_mode(&ctx).await
        }
    };

    let execution_result = match execution_result {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("任务 {} 执行出错: {}", id, e);
            ctx.mark_failed(&e.to_string()).await.ok();
            return Err(WebError(e));
        }
    };

    let mut publish_results = Vec::new();
    let mut saved_to_draft = false;
    let mut no_connected_warning = None;

    if execution_result.success && task.get_mode() == self_media_core::types::TaskMode::Text {
        let task_platforms: Vec<Platform> = serde_json::from_str(&task.platforms)
            .unwrap_or_default();

        let mut unconnected_platforms_content: Vec<(Platform, String)> = Vec::new();

        for (platform, content) in &execution_result.adapted_contents {
            let config = state.config_service.get_platform_config(auth.user_id, platform.clone()).await;

            let cookies = match config {
                Ok(cfg) if cfg.cookies.is_some() => cfg.cookies.unwrap(),
                _ => {
                    tracing::warn!("平台 {:?} 未配置cookies，跳过发布", platform);
                    unconnected_platforms_content.push((platform.clone(), content.clone()));
                    continue;
                }
            };

            let credential = PlatformCredential {
                platform: platform.clone(),
                cookies,
                extra: Default::default(),
            };

            let publisher = {
                let guard = state.publisher_registry.lock().await;
                guard.get(platform)
            };

            if let Some(publisher) = publisher {
                let article = ArticleContent {
                    title: task.topic.clone(),
                    body: content.clone(),
                    image_urls: execution_result.generated_images.clone(),
                    tags: vec![],
                    topic: Some(task.topic.clone()),
                };

                match publisher.publish_article(&credential, &article).await {
                    Ok(result) => {
                        tracing::info!("平台 {:?} 发布成功: {:?}", platform, result);
                        publish_results.push(result);
                    }
                    Err(e) => {
                        tracing::error!("平台 {:?} 发布失败: {:?}", platform, e);
                        publish_results.push(PublishResult {
                            platform: platform.clone(),
                            success: false,
                            post_id: None,
                            post_url: None,
                            error_code: Some("PUBLISH_ERROR".to_string()),
                            error_message: Some(e.to_string()),
                        });
                    }
                }
            } else {
                unconnected_platforms_content.push((platform.clone(), content.clone()));
            }
        }

        if !unconnected_platforms_content.is_empty() {
            let _ = state.draft_service
                .create_draft(
                    auth.user_id,
                    Some(task.id.clone()),
                    &task.mode.to_string(),
                    &task.topic,
                    &task_platforms,
                    execution_result.generated_text.as_deref(),
                    &unconnected_platforms_content,
                    &execution_result.generated_images,
                )
                .await
                .map_err(|e| tracing::error!("保存草稿失败: {}", e));
            saved_to_draft = true;
        }

        if publish_results.is_empty() && !unconnected_platforms_content.is_empty() {
            no_connected_warning = Some("所有目标平台均未连接登录，内容已保存到草稿箱".to_string());
        }
    }
    
    tracing::info!("任务 {} 执行完成，发布结果: {:?}", id, publish_results);

    Ok(ApiOk(PublishTaskResponse {
        success: execution_result.success,
        generated_text: execution_result.generated_text,
        generated_images: execution_result.generated_images,
        publish_results,
        error_message: execution_result.error_message,
        saved_to_draft,
        warning: no_connected_warning,
    }))
}

#[derive(serde::Serialize)]
pub struct PublishTaskResponse {
    pub success: bool,
    pub generated_text: Option<String>,
    pub generated_images: Vec<String>,
    pub publish_results: Vec<PublishResult>,
    pub error_message: Option<String>,
    pub saved_to_draft: bool,
    pub warning: Option<String>,
}
