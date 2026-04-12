use serde::{Deserialize, Serialize};

use crate::types::TaskStatus;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: String,
    pub user_id: i64,
    pub task_type: String,
    pub status: TaskStatus,
    pub mode: String,
    pub topic: String,
    pub platforms: String,
    pub progress: u32,
    pub total_steps: u32,
    pub current_step: Option<String>,
    pub result: Option<String>,
    pub error: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub created_at: String,
    pub updated_at: String,
    #[sqlx(default)]
    pub event_date: Option<String>,
}

impl Task {
    pub fn get_event_date(&self) -> Option<chrono::NaiveDate> {
        self.event_date.as_ref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
    }

    pub fn get_mode(&self) -> crate::types::TaskMode {
        match self.mode.as_str() {
            "video" => crate::types::TaskMode::Video,
            _ => crate::types::TaskMode::Text,
        }
    }
}
