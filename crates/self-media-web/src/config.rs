use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;

use self_media_core::config::service::{PlatformConfig, UserPreferences};
use self_media_core::error::{AppError, *};
use self_media_core::types::Platform;

use crate::{ApiOk, AppState, AuthUser, WebError};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api-key", get(get_api_key).put(set_api_key))
        .route("/platforms", get(get_platforms))
        .route("/platforms/{platform}", put(set_platform))
        .route("/preferences", get(get_preferences).put(set_preferences))
        .route("/models", get(get_model_config).put(set_model_config))
}

#[derive(serde::Serialize)]
pub struct ApiKeyResponse {
    pub provider: String,
    pub key: String,
    pub region: String,
}

pub async fn get_api_key(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<ApiOk<ApiKeyResponse>, WebError> {
    let encrypted_api_key: Option<String> = sqlx::query_scalar(
        "SELECT minimax_api_key FROM users WHERE id = ?"
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| WebError(AppError::Internal(anyhow::anyhow!("数据库错误: {}", e))))?;

    // 返回脱敏后的 API Key 预览（保留前4位）
    let masked_key = if let Some(encrypted) = encrypted_api_key {
        // 解密获取原始 key
        let api_key = state.system_key.decrypt(&encrypted)
            .map_err(|e| WebError(AppError::Internal(anyhow::anyhow!("API Key 解密失败: {}", e))))?;
        if api_key.len() > 4 {
            format!("{}****", &api_key[..4])
        } else {
            "****".to_string()
        }
    } else {
        "".to_string()
    };

    Ok(ApiOk(ApiKeyResponse {
        provider: "minimax".to_string(),
        key: masked_key,
        region: "cn".to_string(),
    }))
}

#[derive(Deserialize)]
pub struct SetApiKeyRequest {
    pub provider: String,
    pub key: String,
    pub region: String,
}

pub async fn set_api_key(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<SetApiKeyRequest>,
) -> Result<ApiOk<()>, WebError> {
    let user_key = state
        .user_key_cache
        .get_or_derive(auth.user_id, &state.db, None)
        .await?;
    state
        .config_service
        .set_api_key(auth.user_id, &body.provider, &body.key, &body.region, &user_key)
        .await?;
    Ok(ApiOk(()))
}

#[derive(serde::Serialize)]
pub struct PlatformListResponse {
    pub platforms: Vec<PlatformConfig>,
}

pub async fn get_platforms(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<ApiOk<PlatformListResponse>, WebError> {
    let platforms = state.config_service.get_platform_configs(auth.user_id).await?;
    Ok(ApiOk(PlatformListResponse { platforms }))
}

#[derive(Deserialize)]
pub struct SetPlatformRequest {
    pub enabled: bool,
    pub image_count: u32,
    pub cookies: Option<String>,
    pub extra: Option<std::collections::HashMap<String, String>>,
}

pub async fn set_platform(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(platform): Path<String>,
    Json(body): Json<SetPlatformRequest>,
) -> Result<ApiOk<()>, WebError> {
    // 解析平台名称
    let platform_enum = match platform.to_lowercase().as_str() {
        "weibo" => Platform::Weibo,
        "bilibili" => Platform::Bilibili,
        "toutiao" => Platform::Toutiao,
        "xiaohongshu" => Platform::Xiaohongshu,
        "douyin" => Platform::Douyin,
        "wechatofficial" | "wechat" | "wechat_official" => Platform::WeChatOfficial,
        _ => return Err(WebError(AppError::validation(INPUT_001, "未知平台"))),
    };

    let config = PlatformConfig {
        platform: platform_enum,
        enabled: body.enabled,
        image_count: body.image_count,
        cookies: body.cookies,
        extra: body.extra.unwrap_or_default(),
    };

    state.config_service.set_platform_config(auth.user_id, &config).await?;
    Ok(ApiOk(()))
}

#[derive(serde::Serialize)]
pub struct PreferencesResponse {
    pub preferences: UserPreferences,
}

pub async fn get_preferences(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<ApiOk<PreferencesResponse>, WebError> {
    let preferences = state.config_service.get_preferences(auth.user_id).await?;
    Ok(ApiOk(PreferencesResponse { preferences }))
}

#[derive(Deserialize)]
pub struct SetPreferencesRequest {
    pub default_mode: Option<self_media_core::types::TaskMode>,
    pub default_tags: Option<Vec<String>>,
    pub auto_publish: Option<bool>,
}

pub async fn set_preferences(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<SetPreferencesRequest>,
) -> Result<ApiOk<()>, WebError> {
    let current = state.config_service.get_preferences(auth.user_id).await?;
    let updated = UserPreferences {
        default_mode: body.default_mode.unwrap_or(current.default_mode),
        default_tags: body.default_tags.unwrap_or(current.default_tags),
        auto_publish: body.auto_publish.unwrap_or(current.auto_publish),
    };

    state
        .config_service
        .set_preferences(auth.user_id, &updated)
        .await?;
    Ok(ApiOk(()))
}

#[derive(serde::Serialize)]
pub struct ModelConfigResponse {
    pub text_model: String,
    pub image_model: String,
    pub video_model: String,
    pub speech_model: String,
    pub music_model: String,
}

pub async fn get_model_config(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<ApiOk<ModelConfigResponse>, WebError> {
    let config = state.user_service.get_user_model_config(auth.user_id).await?;
    Ok(ApiOk(ModelConfigResponse {
        text_model: config.text_model,
        image_model: config.image_model,
        video_model: config.video_model,
        speech_model: config.speech_model,
        music_model: config.music_model,
    }))
}

#[derive(Deserialize)]
pub struct SetModelConfigRequest {
    pub text_model: String,
    pub image_model: String,
    pub video_model: String,
    pub speech_model: String,
    pub music_model: String,
}

pub async fn set_model_config(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<SetModelConfigRequest>,
) -> Result<ApiOk<()>, WebError> {
    let config = self_media_core::user::model::UserModelConfig {
        text_model: body.text_model,
        image_model: body.image_model,
        video_model: body.video_model,
        speech_model: body.speech_model,
        music_model: body.music_model,
    };
    state.user_service.update_user_model_config(auth.user_id, &config).await?;
    Ok(ApiOk(()))
}
