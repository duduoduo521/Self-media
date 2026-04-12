use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::AppError;
use crate::types::{Platform, PublishResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Draft {
    pub id: String,
    pub user_id: i64,
    pub task_id: Option<String>,
    pub mode: String,
    pub topic: String,
    pub platforms: Vec<Platform>,
    pub original_content: Option<String>,
    pub adapted_contents: Vec<(Platform, String)>,
    pub generated_images: Vec<String>,
    pub status: DraftStatus,
    pub publish_results: Vec<PublishResult>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DraftStatus {
    Draft,
    Published,
    PartiallyPublished,
}

impl std::fmt::Display for DraftStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DraftStatus::Draft => write!(f, "draft"),
            DraftStatus::Published => write!(f, "published"),
            DraftStatus::PartiallyPublished => write!(f, "partially_published"),
        }
    }
}

impl From<String> for DraftStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "published" => DraftStatus::Published,
            "partially_published" => DraftStatus::PartiallyPublished,
            _ => DraftStatus::Draft,
        }
    }
}

pub struct DraftService {
    db: SqlitePool,
}

impl DraftService {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn create_draft(
        &self,
        user_id: i64,
        task_id: Option<String>,
        mode: &str,
        topic: &str,
        platforms: &[Platform],
        original_content: Option<&str>,
        adapted_contents: &[(Platform, String)],
        generated_images: &[String],
    ) -> Result<Draft, AppError> {
        let id = uuid::Uuid::new_v4().to_string();
        let platforms_json = serde_json::to_string(platforms)?;
        let adapted_json = serde_json::to_string(adapted_contents)?;
        let images_json = serde_json::to_string(generated_images)?;
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            r#"
            INSERT INTO drafts (id, user_id, task_id, mode, topic, platforms, original_content, adapted_contents, generated_images, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'draft', ?, ?)
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(&task_id)
        .bind(mode)
        .bind(topic)
        .bind(&platforms_json)
        .bind(original_content)
        .bind(&adapted_json)
        .bind(&images_json)
        .bind(&now)
        .bind(&now)
        .execute(&self.db)
        .await?;

        Ok(Draft {
            id,
            user_id,
            task_id,
            mode: mode.to_string(),
            topic: topic.to_string(),
            platforms: platforms.to_vec(),
            original_content: original_content.map(String::from),
            adapted_contents: adapted_contents.to_vec(),
            generated_images: generated_images.to_vec(),
            status: DraftStatus::Draft,
            publish_results: vec![],
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub async fn get_draft(&self, id: &str, user_id: i64) -> Result<Draft, AppError> {
        let row: (String, i64, Option<String>, String, String, String, Option<String>, String, String, String, String, String) = sqlx::query_as(
            "SELECT id, user_id, task_id, mode, topic, platforms, original_content, adapted_contents, generated_images, status, created_at, updated_at FROM drafts WHERE id = ? AND user_id = ?"
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::config("DRAFT_001", "草稿不存在"))?;

        let platforms: Vec<Platform> = serde_json::from_str(&row.5).unwrap_or_default();
        let adapted_contents: Vec<(Platform, String)> = serde_json::from_str(&row.7).unwrap_or_default();
        let generated_images: Vec<String> = serde_json::from_str(&row.8).unwrap_or_default();
        let publish_results: Vec<PublishResult> = serde_json::from_str("[]").unwrap_or_default();

        Ok(Draft {
            id: row.0,
            user_id: row.1,
            task_id: row.2,
            mode: row.3,
            topic: row.4,
            platforms,
            original_content: row.6,
            adapted_contents,
            generated_images,
            status: DraftStatus::from(row.9),
            publish_results,
            created_at: row.10,
            updated_at: row.11,
        })
    }

    pub async fn list_drafts(&self, user_id: i64, status: Option<DraftStatus>) -> Result<Vec<Draft>, AppError> {
        let status_filter = status.map(|s| s.to_string());
        
        let rows: Vec<(String, i64, Option<String>, String, String, String, Option<String>, String, String, String, String, String)> = if let Some(ref s) = status_filter {
            sqlx::query_as(
                "SELECT id, user_id, task_id, mode, topic, platforms, original_content, adapted_contents, generated_images, status, created_at, updated_at FROM drafts WHERE user_id = ? AND status = ? ORDER BY created_at DESC"
            )
            .bind(user_id)
            .bind(s)
            .fetch_all(&self.db)
            .await?
        } else {
            sqlx::query_as(
                "SELECT id, user_id, task_id, mode, topic, platforms, original_content, adapted_contents, generated_images, status, created_at, updated_at FROM drafts WHERE user_id = ? ORDER BY created_at DESC"
            )
            .bind(user_id)
            .fetch_all(&self.db)
            .await?
        };

        let drafts: Vec<Draft> = rows
            .into_iter()
            .map(|row| {
                let platforms: Vec<Platform> = serde_json::from_str(&row.5).unwrap_or_default();
                let adapted_contents: Vec<(Platform, String)> = serde_json::from_str(&row.7).unwrap_or_default();
                let generated_images: Vec<String> = serde_json::from_str(&row.8).unwrap_or_default();
                let publish_results: Vec<PublishResult> = serde_json::from_str("[]").unwrap_or_default();

                Draft {
                    id: row.0,
                    user_id: row.1,
                    task_id: row.2,
                    mode: row.3,
                    topic: row.4,
                    platforms,
                    original_content: row.6,
                    adapted_contents,
                    generated_images,
                    status: DraftStatus::from(row.9),
                    publish_results,
                    created_at: row.10,
                    updated_at: row.11,
                }
            })
            .collect();

        Ok(drafts)
    }

    pub async fn update_draft_status(&self, id: &str, user_id: i64, status: DraftStatus, publish_results: &[PublishResult]) -> Result<(), AppError> {
        let results_json = serde_json::to_string(publish_results)?;
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query(
            "UPDATE drafts SET status = ?, publish_results = ?, updated_at = ? WHERE id = ? AND user_id = ?"
        )
        .bind(status.to_string())
        .bind(&results_json)
        .bind(&now)
        .bind(id)
        .bind(user_id)
        .execute(&self.db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::config("DRAFT_002", "草稿不存在或无权修改"));
        }

        Ok(())
    }

    pub async fn delete_draft(&self, id: &str, user_id: i64) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM drafts WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(&self.db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::config("DRAFT_003", "草稿不存在或无权删除"));
        }

        Ok(())
    }
}