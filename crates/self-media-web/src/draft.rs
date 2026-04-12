use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use self_media_core::draft::{Draft, DraftStatus};

use crate::{ApiOk, AppState, AuthUser, WebError};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_drafts).post(create_draft))
        .route("/{id}", get(get_draft).delete(delete_draft))
        .route("/{id}/publish", post(publish_draft))
}

#[derive(Deserialize)]
pub struct ListDraftsQuery {
    pub status: Option<String>,
}

#[derive(Deserialize)]
pub struct PublishDraftRequest {
    pub platform: Option<String>,
}

async fn list_drafts(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<ListDraftsQuery>,
) -> Result<ApiOk<Vec<Draft>>, WebError> {
    let status = query.status.map(|s| DraftStatus::from(s));
    let drafts = state.draft_service.list_drafts(auth.user_id, status).await?;
    Ok(ApiOk(drafts))
}

async fn get_draft(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiOk<Draft>, WebError> {
    let draft = state.draft_service.get_draft(&id, auth.user_id).await?;
    Ok(ApiOk(draft))
}

#[derive(Deserialize)]
pub struct CreateDraftRequest {
    pub mode: String,
    pub topic: String,
    pub platforms: Vec<self_media_core::types::Platform>,
    pub original_content: Option<String>,
    pub adapted_contents: Vec<(self_media_core::types::Platform, String)>,
    pub generated_images: Vec<String>,
}

async fn create_draft(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateDraftRequest>,
) -> Result<ApiOk<Draft>, WebError> {
    let draft = state.draft_service
        .create_draft(
            auth.user_id,
            None,
            &req.mode,
            &req.topic,
            &req.platforms,
            req.original_content.as_deref(),
            &req.adapted_contents,
            &req.generated_images,
        )
        .await?;
    Ok(ApiOk(draft))
}

async fn delete_draft(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiOk<()>, WebError> {
    state.draft_service.delete_draft(&id, auth.user_id).await?;
    Ok(ApiOk(()))
}

async fn publish_draft(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<PublishDraftRequest>,
) -> Result<ApiOk<DraftPublishResponse>, WebError> {
    let draft = state.draft_service.get_draft(&id, auth.user_id).await?;

    let mut publish_results = Vec::new();
    let mut no_connected_warning = None;

    for (platform, content) in &draft.adapted_contents {
        if let Some(ref req_platform) = req.platform {
            if req_platform != &format!("{:?}", platform).to_lowercase() {
                continue;
            }
        }

        let config = state.config_service.get_platform_config(auth.user_id, platform.clone()).await?;

        let cookies = match config.cookies {
            Some(c) => c,
            None => {
                tracing::warn!("平台 {:?} 未配置cookies，跳过发布", platform);
                continue;
            }
        };

        let credential = self_media_core::types::PlatformCredential {
            platform: platform.clone(),
            cookies,
            extra: Default::default(),
        };

        let publisher = {
            let guard = state.publisher_registry.lock().await;
            guard.get(platform)
        };

        if let Some(publisher) = publisher {
            let article = self_media_core::types::ArticleContent {
                title: draft.topic.clone(),
                body: content.clone(),
                image_urls: draft.generated_images.clone(),
                tags: vec![],
                topic: Some(draft.topic.clone()),
            };

            match publisher.publish_article(&credential, &article).await {
                Ok(result) => {
                    tracing::info!("平台 {:?} 发布成功: {:?}", platform, result);
                    publish_results.push(result);
                }
                Err(e) => {
                    tracing::error!("平台 {:?} 发布失败: {:?}", platform, e);
                    publish_results.push(self_media_core::types::PublishResult {
                        platform: platform.clone(),
                        success: false,
                        post_id: None,
                        post_url: None,
                        error_code: Some("PUBLISH_ERROR".to_string()),
                        error_message: Some(e.to_string()),
                    });
                }
            }
        }
    }

    if publish_results.is_empty() && !draft.adapted_contents.is_empty() {
        no_connected_warning = Some("所选平台均未连接登录，请先在平台管理中连接平台".to_string());
    }

    let new_status = if publish_results.is_empty() {
        DraftStatus::Draft
    } else if publish_results.len() == draft.adapted_contents.len() {
        DraftStatus::Published
    } else {
        DraftStatus::PartiallyPublished
    };

    if !publish_results.is_empty() {
        state.draft_service.update_draft_status(&id, auth.user_id, new_status, &publish_results).await?;
    }

    Ok(ApiOk(DraftPublishResponse {
        publish_results,
        warning: no_connected_warning,
    }))
}

#[derive(serde::Serialize)]
pub struct DraftPublishResponse {
    pub publish_results: Vec<self_media_core::types::PublishResult>,
    pub warning: Option<String>,
}