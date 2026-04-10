use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use self_media_core::types::Platform;

/// 发布频率控制器（线程安全）
pub struct RateLimiter {
    last_publish: Mutex<HashMap<Platform, Instant>>,
    active_requests: Mutex<HashMap<Platform, usize>>,
    max_concurrent: usize,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            last_publish: Mutex::new(HashMap::new()),
            active_requests: Mutex::new(HashMap::new()),
            max_concurrent: 3,
        }
    }

    /// 检查是否可以发起发布请求
    pub fn can_publish(&self, platform: &Platform, min_interval: Duration) -> Result<(), String> {
        let active = self.active_requests.lock().unwrap();
        if let Some(&count) = active.get(platform) {
            if count >= self.max_concurrent {
                return Err(format!("平台 {:?} 并发请求数已达上限 ({}/{})", platform, count, self.max_concurrent));
            }
        }
        drop(active);

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

    /// 记录一次发布开始
    pub fn record_publish_start(&self, platform: Platform) {
        let mut active = self.active_requests.lock().unwrap();
        *active.entry(platform).or_insert(0) += 1;
    }

    /// 记录一次发布结束
    pub fn record_publish_end(&self, platform: Platform) {
        let mut active = self.active_requests.lock().unwrap();
        if let Some(count) = active.get_mut(&platform) {
            if *count > 0 {
                *count -= 1;
            }
        }
    }

    /// 记录一次发布（兼容旧接口）
    pub fn record_publish(&self, platform: Platform) {
        let mut active = self.active_requests.lock().unwrap();
        if let Some(count) = active.get_mut(&platform) {
            if *count > 0 {
                *count -= 1;
            }
        }
        drop(active);

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

    /// 获取当前活跃请求数
    pub fn get_active_count(&self, platform: &Platform) -> usize {
        let active = self.active_requests.lock().unwrap();
        *active.get(platform).unwrap_or(&0)
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
