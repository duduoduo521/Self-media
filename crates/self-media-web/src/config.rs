use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;

use self_media_core::config::service::{PlatformConfig, UserPreferences};

use crate::{ApiOk, AppState, AuthUser, WebError};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api-key", put(set_api_key))
        .route("/platforms", get(get_platforms))
        .route("/platforms/{platform}", put(set_platform))
        .route("/preferences", get(get_preferences).put(set_preferences))
}

#[derive(Deserialize)]
pub struct SetApiKeyRequest {
    pub provider: String,
    pub key: String,
    pub region: String,
}

async fn set_api_key(
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

async fn get_platforms(
    _auth: AuthUser,
    State(_state): State<AppState>,
) -> Result<ApiOk<PlatformListResponse>, WebError> {
    Ok(ApiOk(PlatformListResponse {
        platforms: vec![],
    }))
}

#[derive(Deserialize)]
pub struct SetPlatformRequest {
    pub enabled: bool,
    pub image_count: u32,
    pub cookies: Option<String>,
}

async fn set_platform(
    _auth: AuthUser,
    State(_state): State<AppState>,
    Path(_platform): Path<String>,
    Json(_body): Json<SetPlatformRequest>,
) -> Result<ApiOk<()>, WebError> {
    Ok(ApiOk(()))
}

#[derive(serde::Serialize)]
pub struct PreferencesResponse {
    pub preferences: UserPreferences,
}

async fn get_preferences(
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

async fn set_preferences(
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
