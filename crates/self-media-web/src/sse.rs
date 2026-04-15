//! SSE 实时进度推送服务
//!
//! 用于任务执行进度的实时推送

use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::get,
    Router,
};
use std::time::Duration;
use tokio::sync::broadcast;

use crate::AppState;

/// SSE 事件类型
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "data")]
pub enum SseEvent {
    /// 任务进度更新
    TaskProgress {
        task_id: String,  // 修复：与 Task.id (String/UUID) 类型一致
        step: String,
        progress: u32,
        message: String,
    },
    /// 任务完成
    TaskCompleted {
        task_id: String,  // 修复：与 Task.id (String/UUID) 类型一致
        result: String,
    },
    /// 任务失败
    TaskFailed {
        task_id: String,  // 修复：与 Task.id (String/UUID) 类型一致
        error: String,
    },
    /// AI 生成进度
    AiGeneration {
        task_id: String,  // 修复：与 Task.id (String/UUID) 类型一致
        stage: String,
        content_type: String,
    },
    /// 平台发布进度
    PlatformPublish {
        task_id: String,  // 修复：与 Task.id (String/UUID) 类型一致
        platform: String,
        status: String,
    },
    /// 心跳
    Heartbeat,
}

impl SseEvent {
    pub fn to_sse_event(&self) -> Event {
        let data = serde_json::to_string(self).unwrap_or_default();
        Event::default()
            .event("message")
            .data(data)
    }
}

/// 发送 SSE 事件到频道
pub async fn broadcast_event(state: &AppState, event: SseEvent) {
    let _ = state.sse_sender.send(event);
}

/// SSE 流处理器
pub async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, axum::Error>>> {
    let mut rx = state.sse_sender.subscribe();
    
    let stream = async_stream::stream! {
        // 发送初始连接消息
        yield Ok::<Event, axum::Error>(
            Event::default()
                .event("connected")
                .data(r#"{"type":"connected"}"#)
        );
        
        loop {
            tokio::select! {
                biased;
                
                result = rx.recv() => {
                    match result {
                        Ok(event) => {
                            yield Ok(event.to_sse_event());
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("SSE lagged by {} messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    // 发送心跳
                    yield Ok::<Event, axum::Error>(
                        Event::default()
                            .event("heartbeat")
                            .data(r#"{"type":"heartbeat"}"#)
                    );
                }
            }
        }
    };
    
    Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(25))
                .text("keep-alive")
        )
}

/// SSE 路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events", get(sse_handler))
}
