use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use self_media_core::types::Platform;

/// 发布频率控制器
pub struct RateLimiter {
    last_publish: Mutex<HashMap<Platform, Instant>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            last_publish: Mutex::new(HashMap::new()),
        }
    }

    /// 检查是否可以发布（距上次发布是否超过限制）
    pub fn can_publish(&self, platform: &Platform, min_interval: Duration) -> Result<(), String> {
        let map = self.last_publish.lock().unwrap();
        if let Some(last) = map.get(platform) {
            let elapsed = last.elapsed();
            if elapsed < min_interval {
                let remaining = min_interval - elapsed;
                return Err(format!(
                    "发布频率限制：距下次可发布还需 {:.0} 秒",
                    remaining.as_secs_f64()
                ));
            }
        }
        Ok(())
    }

    /// 记录一次发布
    pub fn record_publish(&self, platform: Platform) {
        let mut map = self.last_publish.lock().unwrap();
        map.insert(platform, Instant::now());
    }

    /// 获取距离下次可发布的等待时间
    pub fn wait_duration(&self, platform: &Platform, min_interval: Duration) -> Duration {
        let map = self.last_publish.lock().unwrap();
        match map.get(platform) {
            Some(last) => {
                let elapsed = last.elapsed();
                if elapsed < min_interval {
                    min_interval - elapsed
                } else {
                    Duration::ZERO
                }
            }
            None => Duration::ZERO,
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
