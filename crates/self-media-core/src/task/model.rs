use serde::{Deserialize, Serialize};

use crate::types::{Platform, TaskMode, TaskStatus};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: String,
    pub user_id: i64,
    pub task_type: String,
    pub status: TaskStatus,
    pub mode: TaskMode,
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
}
