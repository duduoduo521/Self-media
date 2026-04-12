use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::error::*;
use crate::task::model::Task;
use crate::types::{Platform, TaskMode, TaskStatus};

pub struct TaskScheduler {
    db: SqlitePool,
    concurrent_limit: usize,
    active_count: AtomicUsize,
    cancel_tokens: Mutex<HashMap<String, CancellationToken>>,
}

impl TaskScheduler {
    pub fn new(db: SqlitePool, concurrent_limit: usize) -> Self {
        Self {
            db,
            concurrent_limit,
            active_count: AtomicUsize::new(0),
            cancel_tokens: Mutex::new(HashMap::new()),
        }
    }

    /// 创建任务
    pub async fn create_task(
        &self,
        user_id: i64,
        mode: TaskMode,
        topic: String,
        platforms: Vec<Platform>,
        event_date: Option<chrono::NaiveDate>,
    ) -> Result<Task, AppError> {
        let active = self.active_count.load(Ordering::Relaxed);
        if active >= self.concurrent_limit {
            return Err(AppError::task(TASK_003, format!(
                "并发任务数已达上限 ({}/{}), 请等待", active, self.concurrent_limit
            )));
        }

        if topic.trim().is_empty() {
            return Err(AppError::validation(INPUT_001, "主题不能为空"));
        }
        if topic.len() > 500 {
            return Err(AppError::validation(INPUT_001, "主题长度不能超过 500 字"));
        }

        let task_id = Uuid::new_v4().to_string();
        let steps = match mode {
            TaskMode::Text => vec!["generate_text", "generate_images", "adapt_content", "publish"],
            TaskMode::Video => vec!["generate_script", "generate_video", "generate_tts", "publish"],
        };
        let total_steps = steps.len();

        let mode_str = mode.to_string();
        let platforms_str = serde_json::to_string(&platforms)?;

        let event_date_str = event_date.map(|d| d.format("%Y-%m-%d").to_string());

        let task: Task = sqlx::query_as(
            "INSERT INTO tasks (id, user_id, task_type, status, mode, topic, platforms, total_steps, event_date) \
             VALUES (?, ?, 'generate_publish', 'Pending', ?, ?, ?, ?, ?) RETURNING *"
        )
        .bind(&task_id)
        .bind(user_id)
        .bind(&mode_str)
        .bind(&topic)
        .bind(&platforms_str)
        .bind(total_steps as i64)
        .bind(&event_date_str)
        .fetch_one(&self.db)
        .await?;

        for (i, step) in steps.iter().enumerate() {
            sqlx::query(
                "INSERT INTO task_steps (task_id, step_name, step_order) VALUES (?, ?, ?)"
            )
            .bind(&task_id)
            .bind(step)
            .bind(i as i64)
            .execute(&self.db)
            .await?;
        }

        self.cancel_tokens.lock().unwrap().insert(task_id.clone(), CancellationToken::new());

        Ok(task)
    }

    /// 取消任务
    pub async fn cancel_task(&self, task_id: &str) -> Result<(), AppError> {
        let task = self.get_task(task_id).await?;
        if task.status != TaskStatus::Pending && task.status != TaskStatus::Running {
            return Err(AppError::task(TASK_002, "只能取消待执行或执行中的任务"));
        }

        if let Some(token) = self.cancel_tokens.lock().unwrap().get(task_id) {
            token.cancel();
        }

        self.update_task_status(task_id, &TaskStatus::Cancelled).await?;
        Ok(())
    }

    /// 获取任务
    pub async fn get_task(&self, task_id: &str) -> Result<Task, AppError> {
        sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_optional(&self.db)
            .await?
            .ok_or(AppError::task(TASK_001, "任务不存在"))
    }

    /// 获取用户任务列表
    pub async fn list_tasks(&self, user_id: i64) -> Result<Vec<Task>, AppError> {
        let tasks = sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks WHERE user_id = ? ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;
        Ok(tasks)
    }

    /// 清理过期数据
    pub async fn cleanup(&self) -> Result<u64, AppError> {
        let result = sqlx::query(
            "DELETE FROM tasks WHERE status IN ('Completed', 'Cancelled', 'Failed') \
             AND created_at < datetime('now', '-30 days')"
        )
        .execute(&self.db)
        .await?;

        sqlx::query("DELETE FROM sessions WHERE expires_at < datetime('now')")
            .execute(&self.db)
            .await?;

        Ok(result.rows_affected())
    }

    // ---- 内部方法 ----

    async fn update_task_status(&self, task_id: &str, status: &TaskStatus) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE tasks SET status = ?, updated_at = datetime('now') WHERE id = ?"
        )
        .bind(status)
        .bind(task_id)
        .execute(&self.db)
        .await?;
        Ok(())
    }
}
