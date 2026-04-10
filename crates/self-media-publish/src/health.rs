use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use self_media_core::types::{CookieStatus, Platform, PlatformCredential};
use chrono::Utc;

use crate::publisher::PlatformPublisher;

/// Cookie 健康检查器
pub struct CookieHealthChecker {
    publishers: HashMap<Platform, Box<dyn PlatformPublisher>>,
    last_check: Mutex<HashMap<Platform, chrono::DateTime<Utc>>>,
    #[allow(dead_code)]
    check_interval: Duration,
}

impl CookieHealthChecker {
    pub fn new(publishers: HashMap<Platform, Box<dyn PlatformPublisher>>) -> Self {
        Self {
            publishers,
            last_check: Mutex::new(HashMap::new()),
            check_interval: Duration::from_secs(3600), // 默认 1 小时检查一次
        }
    }

    /// 检查指定平台的 Cookie 有效性
    pub async fn check(
        &self,
        platform: &Platform,
        #[allow(unused_variables)] credential: &PlatformCredential,
    ) -> Result<CookieStatus, String> {
        let publisher = self
            .publishers
            .get(platform)
            .ok_or_else(|| format!("平台 {:?} 未注册", platform))?;

        let valid = publisher
            .check_login_status(credential)
            .await
            .map_err(|e| e.to_string())?;

        let now = Utc::now();
        {
            let mut last = self.last_check.lock().unwrap();
            last.insert(platform.clone(), now);
        }

        Ok(CookieStatus {
            platform: platform.clone(),
            valid,
            last_checked: now,
        })
    }

    /// 批量检查所有平台
    pub async fn check_all(
        &self,
        credentials: &[(Platform, PlatformCredential)],
    ) -> Vec<CookieStatus> {
        let mut results = Vec::new();
        for (platform, credential) in credentials {
            match self.check(platform, credential).await {
                Ok(status) => results.push(status),
                Err(e) => {
                    tracing::warn!("Cookie 检查失败: {:?} - {}", platform, e);
                    results.push(CookieStatus {
                        platform: platform.clone(),
                        valid: false,
                        last_checked: Utc::now(),
                    });
                }
            }
        }
        results
    }

    /// 获取上次检查时间
    pub fn last_checked(&self, platform: &Platform) -> Option<chrono::DateTime<Utc>> {
        self.last_check.lock().unwrap().get(platform).copied()
    }
}
