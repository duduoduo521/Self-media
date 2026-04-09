use axum::{
    extract::{Path, Query, State},
    routing::get,
    Router,
};
use serde::Deserialize;

use self_media_core::error::INPUT_001;
use self_media_core::types::{Hotspot, HotspotSource};

use crate::{ApiOk, AppState, AuthUser, WebError};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(fetch_all))
        .route("/{source}", get(fetch_by_source))
}

#[derive(Deserialize)]
pub struct FetchAllQuery {
    pub force_refresh: Option<bool>,
}

#[derive(serde::Serialize)]
pub struct HotspotListResponse {
    pub hotspots: Vec<Hotspot>,
}

async fn fetch_all(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(_query): Query<FetchAllQuery>,
) -> Result<ApiOk<HotspotListResponse>, WebError> {
    let hotspot_service = state.hotspot_service.lock().await;
    let hotspots = hotspot_service.fetch_all().await?;
    Ok(ApiOk(HotspotListResponse { hotspots }))
}

async fn fetch_by_source(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(source): Path<String>,
) -> Result<ApiOk<HotspotListResponse>, WebError> {
    let source: HotspotSource = serde_json::from_str(&format!("\"{}\"", source))
        .map_err(|_| WebError(self_media_core::error::AppError::validation(INPUT_001, &format!("未知热点源: {}", source))))?;

    let hotspot_service = state.hotspot_service.lock().await;
    let hotspots = hotspot_service.fetch_by_source(source).await?;
    Ok(ApiOk(HotspotListResponse { hotspots }))
}
