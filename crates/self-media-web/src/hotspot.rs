use axum::{
    extract::{Path, Query, State},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};

use self_media_ai::client::MiniMaxClient;
use self_media_ai::error::AiError;
use self_media_ai::model::Message;
use self_media_core::error::INPUT_001;
use self_media_core::types::{Hotspot, HotspotSource};

use crate::{ApiOk, AppState, AuthUser, WebError};

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    #[serde(alias = "content", alias = "text")]
    content: String,
}

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

    tracing::info!("开始调用 MiniMax LLM 搜索热点，关键字: {}", query.keyword);

    let client = MiniMaxClient::new(api_key, "https://api.minimax.chat".to_string());

    match search_hotspots_via_llm(&client, &query.keyword).await {
        Ok(hotspots) => {
            tracing::info!("LLM 搜索成功，返回 {} 条热点", hotspots.len());
            Ok(ApiOk(HotspotListResponse { hotspots }))
        }
        Err(e) => {
            tracing::error!("LLM 搜索失败: {:?}", e);
            Err(WebError(self_media_core::error::AppError::ai(INPUT_001, format!("搜索失败: {}", e))))
        }
    }
}

async fn search_hotspots_via_llm(client: &MiniMaxClient, keyword: &str) -> Result<Vec<Hotspot>, AiError> {
    let prompt = format!(
        r#"请搜索并推荐10个与关键词"{}"相关的最热门话题，覆盖不同平台（微博、抖音、小红书、B站、头条、知乎等）。

请使用联网搜索获取每个话题的最新信息，确保数据真实可靠。

返回JSON数组格式，数组中每个对象必须包含以下字段：
- title: 话题标题
- snippet: 话题简介或摘要（50-100字）
- source: 来源平台，必须是以下之一：微博、抖音、小红书、B站、头条、知乎
- event_date: 事件发生日期，格式为 YYYY-MM-DD（如：2026-04-11），如果是当天的事件用"今天"

只返回JSON数组，不要包含其他文字。"#,
        keyword
    );

    #[derive(Debug, Serialize, Deserialize)]
    struct ChatRequest {
        model: String,
        messages: Vec<Message>,
        stream: bool,
    }

    let request = ChatRequest {
        model: "MiniMax-M2.7".to_string(),
        stream: false,
        messages: vec![
            Message {
                role: "user".to_string(),
                content: prompt,
            },
        ],
    };

    tracing::info!("发送请求到 MiniMax API (LLM 搜索)...");

    let resp: ChatResponse = client.chat_completions(&request).await?;

    tracing::info!("收到 MiniMax 响应: {:?}", resp);

    let content = resp.choices
        .first()
        .map(|c| c.message.content.as_str())
        .unwrap_or("");

    tracing::info!("LLM content: {}", content);

    let cleaned = content
        .trim()
        .replace("```json", "")
        .replace("```", "")
        .replace("
</think>

", "")
        .replace("]>", "");

    let json_part = cleaned.trim();

    let start_pos = json_part.find("[\n").or_else(|| json_part.find("[ "));
    let json_part = match start_pos {
        Some(pos) => &json_part[pos..],
        None => {
            if let Some(pos) = json_part.find("[\"") {
                &json_part[pos..]
            } else if let Some(pos) = json_part.find("[{") {
                &json_part[pos..]
            } else {
                tracing::error!("无法找到 JSON 数组开始位置");
                return Ok(Vec::new());
            }
        }
    };

    let json_part_trimmed = json_part.trim();
    let json_part_trimmed = json_part_trimmed.trim_start_matches('\u{FEFF}');
    let json_part_trimmed = json_part_trimmed.trim_start_matches('\u{200B}');
    let json_part_trimmed = json_part_trimmed.trim();
    let bytes = json_part_trimmed.as_bytes();
    let json_part = if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        json_part_trimmed[3..].trim()
    } else {
        json_part_trimmed
    };

    if json_part.is_empty() || json_part == "[]" {
        tracing::warn!("LLM 返回空内容");
        return Ok(Vec::new());
    }

    if !json_part.starts_with('[') {
        let truncated: String = json_part.chars().take(50).collect();
        tracing::error!("JSON 不是以 [ 开头: {}", truncated);
        return Ok(Vec::new());
    }

    tracing::info!("Cleaned JSON length: {}", json_part.len());
    if json_part.len() > 200 {
        let truncated: String = json_part.chars().take(200).collect();
        tracing::info!("Cleaned JSON (first 200): {}", truncated);
    } else {
        tracing::info!("Cleaned JSON: {}", json_part);
    }

    tracing::info!("开始解析 JSON...");

    let try_parse = |s: &str| -> Option<Vec<serde_json::Value>> {
        match serde_json::from_str::<serde_json::Value>(s) {
            Ok(serde_json::Value::Array(arr)) => {
                tracing::info!("直接解析为 JSON 数组，得到 {} 个元素", arr.len());
                return Some(arr);
            }
            Ok(serde_json::Value::String(content)) => {
                tracing::info!("外层是 JSON 字符串，需要二次解析");
                if let Ok(v) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                    return Some(v);
                }
                let unescaped = content.replace("\\\"", "\"").replace("\\\\", "\\");
                if let Ok(v) = serde_json::from_str::<Vec<serde_json::Value>>(&unescaped) {
                    tracing::info!("从转义字符串中解析数组，得到 {} 个元素", v.len());
                    return Some(v);
                }
                if let Some(start) = content.find('[') {
                    let inner = &content[start..];
                    if let Some(end) = inner.rfind(']') {
                        let arr_str = &inner[..=end];
                        let unescaped_arr = arr_str.replace("\\\"", "\"").replace("\\\\", "\\");
                        if let Ok(v) = serde_json::from_str::<Vec<serde_json::Value>>(&unescaped_arr) {
                            tracing::info!("从嵌套字符串中提取数组，得到 {} 个元素", v.len());
                            return Some(v);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("JSON 解析失败: {}", e);
                let preview: String = s.chars().take(50).collect();
                tracing::error!("JSON 内容预览 (前50字符): {:?}", preview);
                let bytes: Vec<u8> = s.bytes().take(100).collect();
                tracing::error!("JSON 字节预览: {:?}", bytes);
                let line_count = s.lines().take(3).count();
                if line_count >= 3 {
                    let chars: Vec<char> = s.chars().collect();
                    let line3_start = s.lines().take(3).map(|l| l.len() + 1).sum::<usize>();
                    let context_start = line3_start.saturating_sub(20);
                    let before: String = chars[context_start..line3_start].iter().collect();
                    let after: String = chars[line3_start..].iter().take(50).collect();
                    tracing::error!("第3行上下文 (位置 {}): {}...", line3_start, format!("{}{}", before, after));
                }
            }
            Ok(other) => {
                tracing::error!("Unexpected JSON type: {:?}", other);
            }
        }
        None
    };

    let parsed = try_parse(json_part).or_else(|| {
        tracing::info!("尝试查找并提取 JSON 数组...");
        let first_bracket = json_part.find('[')?;
        let extracted = &json_part[first_bracket..];
        let first_100: String = extracted.chars().take(100).collect();
        tracing::info!("提取的 JSON (前100字符): {}...", first_100);
        let mut depth = 0;
        let mut in_string = false;
        let mut escape_next = false;
        let mut end_pos = None;
        for (i, c) in extracted.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }
            if c == '\\' {
                escape_next = true;
                continue;
            }
            if c == '"' {
                in_string = !in_string;
                continue;
            }
            if in_string {
                continue;
            }
            match c {
                '[' | '{' => depth += 1,
                ']' | '}' => depth -= 1,
                _ => {}
            }
            if depth == 0 && (c == ']' || c == '}') {
                end_pos = Some(i);
                break;
            }
        }
        let end_pos = end_pos.unwrap_or(extracted.len() - 1);
        let clean = &extracted[..=end_pos];
        let clean_no_ws: String = clean.chars().filter(|c| !c.is_whitespace()).collect();
        let truncated: String = clean_no_ws.chars().take(100).collect();
        tracing::info!("清理后的 JSON: {}...", truncated);
        if let Ok(v) = serde_json::from_str::<Vec<serde_json::Value>>(&clean_no_ws) {
            tracing::info!("解析成功，得到 {} 个元素", v.len());
            return Some(v);
        }
        if let Ok(v) = serde_json::from_str::<Vec<serde_json::Value>>(clean) {
            tracing::info!("保留空白解析成功，得到 {} 个元素", v.len());
            return Some(v);
        }
        tracing::error!("无法解析 LLM 返回为 JSON 数组");
        None
    });

    let parsed = match parsed {
        Some(v) => v,
        None => {
            tracing::error!("无法解析 LLM 返回为 JSON 数组");
            return Ok(Vec::new());
        }
    };

    tracing::info!("解析到 {} 个元素", parsed.len());

    let today = chrono::Utc::now().date_naive();

    let hotspots: Vec<Hotspot> = parsed.into_iter().filter_map(|item| {
        let title = item.get("title")?.as_str()?.to_string();
        let snippet = item.get("snippet").and_then(|v| v.as_str()).map(|s| s.to_string());

        let source_str = item.get("source").and_then(|v| v.as_str()).unwrap_or("微博");
        let source = match source_str {
            "微博" | "weibo" | "新浪微博" => HotspotSource::Weibo,
            "抖音" | "douyin" | "字节跳动" => HotspotSource::Douyin,
            "小红书" | "xiaohongshu" | "redbook" => HotspotSource::Xiaohongshu,
            "B站" | "bilibili" | "哔哩哔哩" => HotspotSource::Bilibili,
            "头条" | "toutiao" | "今日头条" => HotspotSource::Toutiao,
            "知乎" | "zhihu" | "zhihu.com" => HotspotSource::Zhihu,
            _ => HotspotSource::Weibo,
        };

        let event_date = item.get("event_date").and_then(|v| v.as_str()).and_then(|s| {
            if s == "今天" {
                Some(today)
            } else {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
            }
        });

        Some(Hotspot {
            title,
            hot_score: 0,
            source,
            url: None,
            category: snippet.clone(),
            fetched_at: chrono::Utc::now(),
            event_date,
        })
    }).collect();

    tracing::info!("转换为 {} 个热点", hotspots.len());

    Ok(hotspots)
}
