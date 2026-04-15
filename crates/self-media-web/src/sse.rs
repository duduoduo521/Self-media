//! SSE 实时进度推送服务
//!
//! 用于任务执行进度的实时推送
//! 安全措施：需要认证，按 user_id 过滤事件

use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::get,
    Router,
};
use std::time::Duration;
use tokio::sync::broadcast;

use crate::{AppState, AuthUser};

/// SSE 事件类型
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "data")]
pub enum SseEvent {
    /// 任务进度更新
    TaskProgress {
        user_id: i64,     // 添加用户隔离字段
        task_id: String,
        step: String,
        progress: u32,
        message: String,
    },
    /// 任务完成
    TaskCompleted {
        user_id: i64,     // 添加用户隔离字段
        task_id: String,
        result: String,
    },
    /// 任务失败
    TaskFailed {
        user_id: i64,     // 添加用户隔离字段
        task_id: String,
        error: String,
    },
    /// AI 生成进度
    AiGeneration {
        user_id: i64,     // 添加用户隔离字段
        task_id: String,
        stage: String,
        content_type: String,
    },
    /// 平台发布进度
    PlatformPublish {
        user_id: i64,     // 添加用户隔离字段
        task_id: String,
        platform: String,
        status: String,
    },
    /// 心跳（无用户隔离）
    Heartbeat,
}

impl SseEvent {
    pub fn to_sse_event(&self) -> Event {
        let data = serde_json::to_string(self).unwrap_or_default();
        Event::default()
            .event("message")
            .data(data)
    }

    pub fn user_id(&self) -> Option<i64> {
        match self {
            SseEvent::TaskProgress { user_id, .. } => Some(*user_id),
            SseEvent::TaskCompleted { user_id, .. } => Some(*user_id),
            SseEvent::TaskFailed { user_id, .. } => Some(*user_id),
            SseEvent::AiGeneration { user_id, .. } => Some(*user_id),
            SseEvent::PlatformPublish { user_id, .. } => Some(*user_id),
            SseEvent::Heartbeat => None,
        }
    }
}

/// 发送 SSE 事件到频道
pub async fn broadcast_event(state: &AppState, event: SseEvent) {
    let _ = state.sse_sender.send(event);
}

/// SSE 流处理器（需要认证）
pub async fn sse_handler(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, axum::Error>>> {
    let mut rx = state.sse_sender.subscribe();
    let user_id = auth.user_id;
    
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
                            // 用户隔离：只推送当前用户的事件
                            if event.user_id().is_none() || event.user_id() == Some(user_id) {
                                yield Ok(event.to_sse_event());
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("SSE lagged by {} messages for user {}", n, user_id);
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

/// SSE 路由（需要认证）
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events", get(sse_handler))
}