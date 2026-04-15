use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use base64::Engine;
use lru::LruCache;
use self_media_crypto::{UserKey, SystemKey};
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
    system_key: SystemKey,
}

impl ConfigService {
    pub fn new(db: SqlitePool, system_key: SystemKey) -> Self {
        Self { db, system_key }
    }

    /// 设置 API Key（加密存储到 users 表）
    pub async fn set_api_key(
        &self,
        user_id: i64,
        _provider: &str,
        key: &str,
        _region: &str,
        _user_key: &UserKey,
    ) -> Result<(), AppError> {
        if key.trim().is_empty() {
            return Err(AppError::validation(INPUT_001, "API Key 不能为空"));
        }

        // 使用 SystemKey 加密后存储
        let encrypted_key = self.system_key.encrypt(key)?;

        sqlx::query(
            "UPDATE users SET minimax_api_key = ? WHERE id = ?"
        )
        .bind(&encrypted_key)
        .bind(user_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// 获取 API Key（从 users 表读取并解密）
    pub async fn get_api_key(
        &self,
        user_id: i64,
        _provider: &str,
        _user_key: &UserKey,
    ) -> Result<(String, MiniMaxRegion), AppError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT minimax_api_key FROM users WHERE id = ? AND minimax_api_key IS NOT NULL AND minimax_api_key != ''"
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        if let Some((encrypted_key,)) = row {
            // 使用 SystemKey 解密
            let api_key = self.system_key.decrypt(&encrypted_key)?;
            return Ok((api_key, MiniMaxRegion::CN));
        }

        Err(AppError::config(CONFIG_001, "未配置 MiniMax API Key".to_string()))
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

    /// 获取用户所有平台配置
    pub async fn get_platform_configs(&self, user_id: i64) -> Result<Vec<PlatformConfig>, AppError> {
        let rows: Vec<(String, i32, i32, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT platform, enabled, image_count, cookies, extra FROM platform_configs WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        let mut configs = Vec::new();
        for (platform_str, enabled, image_count, cookies, extra) in rows {
            let platform: Platform = serde_json::from_str(&format!("\"{}\"", platform_str))
                .unwrap_or(Platform::Weibo);
            let extra_map: HashMap<String, String> = extra
                .and_then(|e| serde_json::from_str(&e).ok())
                .unwrap_or_default();
            
            configs.push(PlatformConfig {
                platform,
                enabled: enabled != 0,
                image_count: image_count as u32,
                cookies,
                extra: extra_map,
            });
        }
        Ok(configs)
    }

    /// 获取用户单个平台配置
    pub async fn get_platform_config(&self, user_id: i64, platform: Platform) -> Result<PlatformConfig, AppError> {
        let platform_str = serde_json::to_string(&platform)?;
        let platform_name = &platform_str[1..platform_str.len()-1];

        let row: Option<(String, i32, i32, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT platform, enabled, image_count, cookies, extra FROM platform_configs WHERE user_id = ? AND platform = ?"
        )
        .bind(user_id)
        .bind(platform_name)
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some((_, enabled, image_count, cookies, extra)) => {
                let extra_map: HashMap<String, String> = extra
                    .and_then(|e| serde_json::from_str(&e).ok())
                    .unwrap_or_default();
                Ok(PlatformConfig {
                    platform,
                    enabled: enabled != 0,
                    image_count: image_count as u32,
                    cookies,
                    extra: extra_map,
                })
            }
            None => Err(AppError::config("CONFIG_001", "平台配置不存在")),
        }
    }

    /// 设置/更新平台配置
    pub async fn set_platform_config(&self, user_id: i64, config: &PlatformConfig) -> Result<(), AppError> {
        let platform_str = serde_json::to_string(&config.platform)?;
        let extra_str = serde_json::to_string(&config.extra)?;
        
        sqlx::query(
            "INSERT INTO platform_configs (user_id, platform, enabled, image_count, cookies, extra) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT(user_id, platform) DO UPDATE SET \
             enabled = excluded.enabled, \
             image_count = excluded.image_count, \
             cookies = excluded.cookies, \
             extra = excluded.extra, \
             updated_at = datetime('now')"
        )
        .bind(user_id)
        .bind(&platform_str[1..platform_str.len()-1]) // 去掉引号
        .bind(config.enabled as i32)
        .bind(config.image_count as i32)
        .bind(&config.cookies)
        .bind(&extra_str)
        .execute(&self.db)
        .await?;
        Ok(())
    }
}

// ---- 用户密钥缓存 ----

pub struct UserKeyCache {
    cache: RwLock<LruCache<i64, Arc<UserKey>>>,
    system_key: SystemKey,
}

impl UserKeyCache {
    pub fn new(max_capacity: usize, system_key: SystemKey) -> Self {
        Self {
            cache: RwLock::new(LruCache::new(
                NonZeroUsize::new(max_capacity).unwrap_or(NonZeroUsize::new(100).unwrap()),
            )),
            system_key,
        }
    }

    /// 获取用户密钥，若缓存未命中则从数据库加载加密密钥并解密
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

        // 尝试从数据库加载加密的用户密钥
        let encrypted_user_key: Option<String> = sqlx::query_scalar(
            "SELECT encrypted_user_key FROM users WHERE id = ?"
        )
        .bind(user_id)
        .fetch_optional(db)
        .await?;

        if let Some(encrypted) = encrypted_user_key {
            if !encrypted.is_empty() {
                // 使用系统密钥解密用户密钥（得到 base64 编码的密钥）
                let user_key_b64 = self.system_key.decrypt(&encrypted)?;
                // base64 解码得到原始 32 字节密钥
                let user_key_bytes = base64::engine::general_purpose::STANDARD
                    .decode(&user_key_b64)
                    .map_err(|e| AppError::crypto(CRYPTO_001, format!("用户密钥解码失败: {}", e)))?;
                if user_key_bytes.len() == 32 {
                    let mut key_bytes = [0u8; 32];
                    key_bytes.copy_from_slice(&user_key_bytes);
                    let user_key = Arc::new(UserKey::from_bytes(key_bytes));
                    
                    {
                        let mut cache = self.cache.write().await;
                        cache.put(user_id, user_key.clone());
                    }
                    
                    return Ok(user_key);
                }
            }
        }

        // 如果数据库中没有加密的用户密钥，需要用户提供密码重新派生
        let password = password
            .ok_or(AppError::crypto(CRYPTO_001, "用户密钥未缓存，需重新登录"))?;

        let salt: String = sqlx::query_scalar(
            "SELECT salt FROM users WHERE id = ?"
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;

        let user_key = Arc::new(UserKey::derive_from_password(password, &salt)?);

        // 将用户密钥加密后存储到数据库，以便下次恢复
        let user_key_bytes = user_key.to_bytes();
        let user_key_b64 = base64::engine::general_purpose::STANDARD.encode(&user_key_bytes);
        let encrypted = self.system_key.encrypt(&user_key_b64)?;
        sqlx::query(
            "UPDATE users SET encrypted_user_key = ? WHERE id = ?"
        )
        .bind(&encrypted)
        .bind(user_id)
        .execute(db)
        .await?;

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

    /// 插入用户密钥到缓存（登录时调用）
    pub async fn insert(&self, user_id: i64, user_key: UserKey, db: &SqlitePool) -> Result<(), AppError> {
        // 将用户密钥加密后存储到数据库
        let user_key_bytes = user_key.to_bytes();
        let user_key_b64 = base64::engine::general_purpose::STANDARD.encode(&user_key_bytes);
        let encrypted = self.system_key.encrypt(&user_key_b64)?;
        sqlx::query(
            "UPDATE users SET encrypted_user_key = ? WHERE id = ?"
        )
        .bind(&encrypted)
        .bind(user_id)
        .execute(db)
        .await?;

        let mut cache = self.cache.write().await;
        cache.put(user_id, Arc::new(user_key));
        Ok(())
    }
}