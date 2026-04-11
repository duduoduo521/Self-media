use axum::{
    extract::{Path, Query, State},
    routing::get,
    Router,
};
use serde::Deserialize;

use self_media_ai::client::MiniMaxClient;
use self_media_ai::error::AiError;
use self_media_ai::model::{Message, TextRequest};
use self_media_core::error::INPUT_001;
use self_media_core::types::{Hotspot, HotspotSource};

use crate::{ApiOk, AppState, AuthUser, WebError};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(fetch_all))
        .route("/search", get(search_by_keyword))
        .route("/{source}", get(fetch_by_source))
}

#[derive(Deserialize)]
pub struct FetchAllQuery {
    #[allow(dead_code)]
    pub force_refresh: Option<bool>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub keyword: String,
}

#[derive(serde::Serialize)]
pub struct HotspotListResponse {
    pub hotspots: Vec<Hotspot>,
}

async fn fetch_all(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<FetchAllQuery>,
) -> Result<ApiOk<HotspotListResponse>, WebError> {
    let hotspot_service = state.hotspot_service.lock().await;
    let hotspots = hotspot_service.fetch_all(query.force_refresh.unwrap_or(false)).await?;
    Ok(ApiOk(HotspotListResponse { hotspots }))
}

async fn fetch_by_source(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(source): Path<String>,
    Query(query): Query<FetchAllQuery>,
) -> Result<ApiOk<HotspotListResponse>, WebError> {
    let source: HotspotSource = serde_json::from_str(&format!("\"{}\"", source))
        .map_err(|_| WebError(self_media_core::error::AppError::validation(INPUT_001, &format!("未知热点源: {}", source))))?;

    let hotspot_service = state.hotspot_service.lock().await;
    let hotspots = hotspot_service.fetch_by_source(source, query.force_refresh.unwrap_or(false)).await?;
    Ok(ApiOk(HotspotListResponse { hotspots }))
}

async fn search_by_keyword(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<ApiOk<HotspotListResponse>, WebError> {
    let api_key = state.user_service.get_user_minimax_key(auth.user_id).await
        .map_err(|e| WebError(e))?;

    let client = MiniMaxClient::new(api_key, "https://api.minimax.chat".to_string());

    let hotspots = search_hotspots_via_llm(&client, &query.keyword).await
        .map_err(|e| WebError(self_media_core::error::AppError::ai(INPUT_001, e.to_string())))?;

    Ok(ApiOk(HotspotListResponse { hotspots }))
}

async fn search_hotspots_via_llm(client: &MiniMaxClient, keyword: &str) -> Result<Vec<Hotspot>, AiError> {
    let prompt = format!(
        r#"你是一个专业的新媒体运营专家，擅长发现热点话题。请根据关键词 "{}" 推荐10个当前最热门的话题，这些话题应该：
1. 与关键词紧密相关
2. 具有讨论性和传播性
3. 是最近1-2周内的热点

请以JSON数组格式返回，每条格式为：
{{"title": "话题标题", "snippet": "话题简介", "source": "LLM"}}

只返回JSON数组，不要有其他文字。"#,
        keyword
    );

    let request = TextRequest {
        model: "abab6.5s-chat".to_string(),
        temperature: Some(0.7),
        stream: Some(false),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: prompt,
            },
        ],
    };

    let response = client.generate_text(request).await?;

    tracing::debug!("LLM response: {:?}", response);

    let content = response.choices
        .first()
        .map(|c| c.message.content.as_str())
        .unwrap_or("[]");

    tracing::debug!("LLM content: {}", content);

    let cleaned = content.trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    tracing::debug!("Cleaned JSON: {}", cleaned);

    let parsed: Vec<serde_json::Value> = match serde_json::from_str(cleaned) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Failed to parse LLM response as JSON: {}", e);
            return Ok(Vec::new());
        }
    };

    let hotspots: Vec<Hotspot> = parsed.into_iter().map(|item| {
        let title = item.get("title").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let snippet = item.get("snippet").and_then(|v| v.as_str()).map(|s| s.to_string());
        Hotspot {
            title,
            hot_score: 0,
            source: HotspotSource::Weibo,
            url: None,
            category: snippet.clone(),
            fetched_at: chrono::Utc::now(),
        }
    }).collect();

    tracing::debug!("Parsed {} hotspots", hotspots.len());

    Ok(hotspots)
}
