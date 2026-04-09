use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

use lru::LruCache;
use self_media_crypto::UserKey;
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::error::*;
use crate::types::{MiniMaxRegion, Platform, TaskMode};

// ---- 配置模型 ----

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlatformConfig {
    pub platform: Platform,
    pub enabled: bool,
    pub image_count: u32,
    pub cookies: Option<String>,
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserPreferences {
    pub default_mode: TaskMode,
    pub default_tags: Vec<String>,
    pub auto_publish: bool,
}

// ---- 配置服务 ----

pub struct ConfigService {
    db: SqlitePool,
}

impl ConfigService {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// 设置 API Key（加密存储）
    pub async fn set_api_key(
        &self,
        user_id: i64,
        provider: &str,
        key: &str,
        region: &str,
        user_key: &UserKey,
    ) -> Result<(), AppError> {
        if key.trim().is_empty() {
            return Err(AppError::validation(INPUT_001, "API Key 不能为空"));
        }

        let encrypted_key = user_key.encrypt(key)?;

        sqlx::query(
            "INSERT INTO api_keys (user_id, provider, encrypted_key, region) \
             VALUES (?, ?, ?, ?) \
             ON CONFLICT(user_id, provider) DO UPDATE SET \
             encrypted_key = excluded.encrypted_key, \
             region = excluded.region"
        )
        .bind(user_id)
        .bind(provider)
        .bind(&encrypted_key)
        .bind(region)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// 获取解密后的 API Key
    pub async fn get_api_key(
        &self,
        user_id: i64,
        provider: &str,
        user_key: &UserKey,
    ) -> Result<(String, MiniMaxRegion), AppError> {
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT encrypted_key, region FROM api_keys WHERE user_id = ? AND provider = ?"
        )
        .bind(user_id)
        .bind(provider)
        .fetch_optional(&self.db)
        .await?;

        let (encrypted_key, region_str) = row
            .ok_or(AppError::config(CONFIG_001, &format!("未配置 {} API Key", provider)))?;

        let decrypted_key = user_key.decrypt(&encrypted_key)?;
        let region = match region_str.as_str() {
            "global" => MiniMaxRegion::Global,
            _ => MiniMaxRegion::CN,
        };
        Ok((decrypted_key, region))
    }

    /// 获取用户偏好
    pub async fn get_preferences(&self, user_id: i64) -> Result<UserPreferences, AppError> {
        let row: Option<(String, String, i32)> = sqlx::query_as(
            "SELECT default_mode, default_tags, auto_publish FROM user_preferences WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some((default_mode, default_tags, auto_publish)) => Ok(UserPreferences {
                default_mode: match default_mode.as_str() {
                    "video" => TaskMode::Video,
                    _ => TaskMode::Text,
                },
                default_tags: serde_json::from_str(&default_tags).unwrap_or_default(),
                auto_publish: auto_publish != 0,
            }),
            None => Ok(UserPreferences {
                default_mode: TaskMode::Text,
                default_tags: vec![],
                auto_publish: false,
            }),
        }
    }

    /// 设置用户偏好
    pub async fn set_preferences(&self, user_id: i64, prefs: &UserPreferences) -> Result<(), AppError> {
        let mode_str = match prefs.default_mode {
            TaskMode::Text => "text",
            TaskMode::Video => "video",
        };
        sqlx::query(
            "INSERT INTO user_preferences (user_id, default_mode, default_tags, auto_publish) \
             VALUES (?, ?, ?, ?) \
             ON CONFLICT(user_id) DO UPDATE SET \
             default_mode = excluded.default_mode, \
             default_tags = excluded.default_tags, \
             auto_publish = excluded.auto_publish, \
             updated_at = datetime('now')"
        )
        .bind(user_id)
        .bind(mode_str)
        .bind(serde_json::to_string(&prefs.default_tags)?)
        .bind(prefs.auto_publish as i32)
        .execute(&self.db)
        .await?;
        Ok(())
    }
}

// ---- 用户密钥缓存 ----

pub struct UserKeyCache {
    cache: RwLock<LruCache<i64, Arc<UserKey>>>,
}

impl UserKeyCache {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            cache: RwLock::new(LruCache::new(
                NonZeroUsize::new(max_capacity).unwrap_or(NonZeroUsize::new(100).unwrap()),
            )),
        }
    }

    /// 获取用户密钥，若缓存未命中则从数据库加载并派生
    pub async fn get_or_derive(
        &self,
        user_id: i64,
        db: &SqlitePool,
        password: Option<&str>,
    ) -> Result<Arc<UserKey>, AppError> {
        {
            let mut cache = self.cache.write().await;
            if let Some(key) = cache.get(&user_id) {
                return Ok(key.clone());
            }
        }

        let password = password
            .ok_or(AppError::crypto(CRYPTO_001, "用户密钥未缓存，需重新登录"))?;

        let salt: String = sqlx::query_scalar(
            "SELECT salt FROM users WHERE id = ?"
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;

        let user_key = Arc::new(UserKey::derive_from_password(password, &salt)?);

        {
            let mut cache = self.cache.write().await;
            cache.put(user_id, user_key.clone());
        }

        Ok(user_key)
    }

    /// 用户修改密码或登出时清除缓存
    pub async fn invalidate(&self, user_id: i64) {
        let mut cache = self.cache.write().await;
        cache.pop(&user_id);
    }
}
