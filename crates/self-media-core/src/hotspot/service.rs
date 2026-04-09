use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::Utc;
use reqwest::Client;

use crate::error::*;
use crate::types::{Hotspot, HotspotSource};

pub struct HotspotService {
    http: Client,
    cache: Mutex<HotspotCache>,
}

struct HotspotCache {
    data: Vec<Hotspot>,
    last_fetch: Option<Instant>,
    ttl: Duration,
}

impl HotspotCache {
    fn new() -> Self {
        Self {
            data: Vec::new(),
            last_fetch: None,
            ttl: Duration::from_secs(300),
        }
    }

    fn is_valid(&self) -> bool {
        match self.last_fetch {
            Some(last) => last.elapsed() < self.ttl,
            None => false,
        }
    }
}

impl HotspotService {
    pub fn new(http: Client) -> Self {
        Self {
            http,
            cache: Mutex::new(HotspotCache::new()),
        }
    }

    /// 获取所有平台热点（带缓存）
    pub async fn fetch_all(&self) -> Result<Vec<Hotspot>, AppError> {
        {
            let cache = self.cache.lock().unwrap();
            if cache.is_valid() {
                return Ok(cache.data.clone());
            }
        }

        let results: Vec<Result<Vec<Hotspot>, AppError>> = futures::future::join_all(vec![
            Box::pin(self.fetch_weibo()) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Hotspot>, AppError>> + Send>>,
            Box::pin(self.fetch_bilibili()) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Hotspot>, AppError>> + Send>>,
        ]).await;

        let mut all = Vec::new();
        for result in results {
            match result {
                Ok(hotspots) => all.extend(hotspots),
                Err(e) => {
                    tracing::warn!("热点数据源获取失败: {}", e);
                }
            }
        }

        all.sort_by(|a, b| b.hot_score.cmp(&a.hot_score));
        all.dedup_by(|a, b| a.title == b.title);

        {
            let mut cache = self.cache.lock().unwrap();
            cache.data = all.clone();
            cache.last_fetch = Some(Instant::now());
        }

        Ok(all)
    }

    /// 获取指定来源的热点
    pub async fn fetch_by_source(&self, source: HotspotSource) -> Result<Vec<Hotspot>, AppError> {
        match source {
            HotspotSource::Weibo => self.fetch_weibo().await,
            HotspotSource::Bilibili => self.fetch_bilibili().await,
            _ => Err(AppError::ai(AI_002, &format!("{:?} 热点源暂未实现", source))),
        }
    }

    async fn fetch_weibo(&self) -> Result<Vec<Hotspot>, AppError> {
        let resp = self.http
            .get("https://weibo.com/ajax/side/hotSearch")
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let mut hotspots = Vec::new();
        if let Some(data) = resp["data"]["realtime"].as_array() {
            for item in data {
                hotspots.push(Hotspot {
                    title: item["word"].as_str().unwrap_or_default().to_string(),
                    hot_score: item["num"].as_u64().unwrap_or(0),
                    source: HotspotSource::Weibo,
                    url: item["url"].as_str().map(|s| s.to_string()),
                    category: item["category"].as_str().map(|s| s.to_string()),
                    fetched_at: Utc::now(),
                });
            }
        }
        Ok(hotspots)
    }

    async fn fetch_bilibili(&self) -> Result<Vec<Hotspot>, AppError> {
        let resp = self.http
            .get("https://api.bilibili.com/x/web-interface/search/square?limit=50")
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let mut hotspots = Vec::new();
        if let Some(data) = resp["data"]["trending"].as_array() {
            for item in data {
                hotspots.push(Hotspot {
                    title: item["keyword"].as_str().unwrap_or_default().to_string(),
                    hot_score: item["heat_score"].as_u64().unwrap_or(0),
                    source: HotspotSource::Bilibili,
                    url: None,
                    category: None,
                    fetched_at: Utc::now(),
                });
            }
        }
        Ok(hotspots)
    }
}

/// 请求频率控制器
pub struct SourceRateLimiter {
    last_request: Mutex<HashMap<HotspotSource, Instant>>,
    min_interval: Duration,
}

impl SourceRateLimiter {
    pub fn new() -> Self {
        Self {
            last_request: Mutex::new(HashMap::new()),
            min_interval: Duration::from_secs(60),
        }
    }

    pub async fn acquire(&self, source: &HotspotSource) {
        let wait = {
            let map = self.last_request.lock().unwrap();
            let now = Instant::now();
            if let Some(last) = map.get(source) {
                let elapsed = now.duration_since(*last);
                if elapsed < self.min_interval {
                    Some(self.min_interval - elapsed)
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(wait) = wait {
            tokio::time::sleep(wait).await;
        }

        let mut map = self.last_request.lock().unwrap();
        map.insert(source.clone(), Instant::now());
    }
}
