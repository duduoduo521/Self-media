use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::Utc;
use reqwest::Client;

use crate::error::*;
use crate::types::{Hotspot, HotspotSource};

pub struct HotspotService {
    http: Client,
    cache: Mutex<HashMap<HotspotSource, SourceCache>>,
    rate_limiter: SourceRateLimiter,
}

struct SourceCache {
    data: Vec<Hotspot>,
    last_fetch: Option<Instant>,
    ttl: Duration,
}

impl SourceCache {
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

impl Default for SourceCache {
    fn default() -> Self {
        Self::new()
    }
}

impl HotspotService {
    pub fn new(http: Client) -> Self {
        Self {
            http,
            cache: Mutex::new(HashMap::new()),
            rate_limiter: SourceRateLimiter::new(),
        }
    }

    /// 获取所有平台热点（带缓存）
    pub async fn fetch_all(&self, force_refresh: bool) -> Result<Vec<Hotspot>, AppError> {
        let sources = vec![
            HotspotSource::Weibo,
            HotspotSource::Bilibili,
            HotspotSource::Douyin,
            HotspotSource::Zhihu,
            HotspotSource::Toutiao,
            HotspotSource::Xiaohongshu,
        ];

        let mut results: Vec<Hotspot> = Vec::new();

        for source in sources {
            match self.fetch_by_source_internal(&source, force_refresh).await {
                Ok(hotspots) => results.extend(hotspots),
                Err(e) => {
                    tracing::warn!("热点数据源 {:?} 获取失败: {}", source, e);
                }
            }
        }

        results.sort_by(|a, b| b.hot_score.cmp(&a.hot_score));
        results.dedup_by(|a, b| a.title == b.title);

        Ok(results)
    }

    /// 获取指定来源的热点（使用缓存）
    pub async fn fetch_by_source(&self, source: HotspotSource, force_refresh: bool) -> Result<Vec<Hotspot>, AppError> {
        if !force_refresh {
            if let Some(cached) = {
                let cache = self.cache.lock().unwrap();
                cache.get(&source).filter(|c| c.is_valid()).map(|c| c.data.clone())
            } {
                return Ok(cached);
            }
        }

        self.fetch_by_source_internal(&source, force_refresh).await
    }

    /// 内部方法：获取指定来源的热点（强制刷新）
    async fn fetch_by_source_internal(&self, source: &HotspotSource, _force_refresh: bool) -> Result<Vec<Hotspot>, AppError> {
        self.rate_limiter.acquire(source).await;

        let hotspots = match source {
            HotspotSource::Weibo => self.fetch_weibo().await?,
            HotspotSource::Bilibili => self.fetch_bilibili().await?,
            HotspotSource::Douyin => self.fetch_douyin().await?,
            HotspotSource::Zhihu => self.fetch_zhihu().await?,
            HotspotSource::Toutiao => self.fetch_toutiao().await?,
            HotspotSource::Xiaohongshu => self.fetch_xiaohongshu().await?,
        };

        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(source.clone(), SourceCache {
                data: hotspots.clone(),
                last_fetch: Some(Instant::now()),
                ttl: Duration::from_secs(300),
            });
        }

        Ok(hotspots)
    }

    async fn fetch_weibo(&self) -> Result<Vec<Hotspot>, AppError> {
        let resp = self.http
            .get("https://weibo.com/ajax/side/hotSearch")
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .header("Referer", "https://weibo.com/")
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
            .get("https://s.search.bilibili.com/main/hotword")
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let mut hotspots = Vec::new();
        if let Some(data) = resp["data"].as_array() {
            for (idx, item) in data.iter().enumerate() {
                hotspots.push(Hotspot {
                    title: item["keyword"].as_str().unwrap_or_default().to_string(),
                    hot_score: item["hot_score"].as_u64().unwrap_or(((data.len() - idx) as u64) * 100),
                    source: HotspotSource::Bilibili,
                    url: None,
                    category: None,
                    fetched_at: Utc::now(),
                });
            }
        }
        Ok(hotspots)
    }

    /// 抖音热搜
    /// API: https://www.douyin.com/aweme/v1/web/hot/search/list/
    async fn fetch_douyin(&self) -> Result<Vec<Hotspot>, AppError> {
        let resp = self.http
            .get("https://www.douyin.com/aweme/v1/web/hot/search/list/")
            .header("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X)")
            .header("Referer", "https://www.douyin.com/")
            .query(&[
                ("device_platform", "webapp"),
                ("aid", "6383"),
                ("channel", "channel_pc_web"),
                ("detail_list", "1"),
            ])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let mut hotspots = Vec::new();
        if let Some(data) = resp["data"]["word_list"].as_array() {
            for item in data {
                hotspots.push(Hotspot {
                    title: item["word"].as_str().unwrap_or_default().to_string(),
                    hot_score: item["hot_value"].as_u64().unwrap_or(0),
                    source: HotspotSource::Douyin,
                    url: item["scheme"].as_str().map(|s| s.to_string()),
                    category: item["word_type"].as_i64().map(|i| match i {
                        1 => "新词".to_string(),
                        2 => "推荐".to_string(),
                        3 => "热搜".to_string(),
                        _ => "普通".to_string(),
                    }),
                    fetched_at: Utc::now(),
                });
            }
        }
        Ok(hotspots)
    }

    /// 知乎热榜
    /// API: https://www.zhihu.com/api/v3/feed/topstory/hot-lists/total
    async fn fetch_zhihu(&self) -> Result<Vec<Hotspot>, AppError> {
        let resp = self.http
            .get("https://www.zhihu.com/api/v3/feed/topstory/hot-lists/total")
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .header("Referer", "https://www.zhihu.com/")
            .header("Cookie", "zhalodata=undefined; qdr=1")
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let mut hotspots = Vec::new();
        if let Some(data) = resp["data"].as_array() {
            for (idx, item) in data.iter().enumerate() {
                let title = item["target"]["title"].as_str()
                    .or(item["title"].as_str())
                    .unwrap_or_default().to_string();

                let hot_score = item["score"].as_u64()
                    .or_else(|| item["hot_score"].as_u64())
                    .unwrap_or(((data.len() - idx) as u64) * 100);

                hotspots.push(Hotspot {
                    title,
                    hot_score,
                    source: HotspotSource::Zhihu,
                    url: item["url"].as_str().map(|s| s.to_string()),
                    category: item["type"].as_str().map(|s| s.to_string()),
                    fetched_at: Utc::now(),
                });
            }
        }
        Ok(hotspots)
    }

    /// 头条热搜
    /// API: https://www.toutiao.com/c/sr/250094470/
    async fn fetch_toutiao(&self) -> Result<Vec<Hotspot>, AppError> {
        let resp = self.http
            .get("https://www.toutiao.com/api/pc/feed/")
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .header("Referer", "https://www.toutiao.com/")
            .query(&[
                ("max_behot_time", "0"),
                ("cat", "250094470"),
                ("keep_items", "[]"),
            ])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let mut hotspots = Vec::new();
        if let Some(data) = resp["data"].as_array() {
            for item in data {
                let title = item["title"].as_str().unwrap_or_default().to_string();
                if title.is_empty() {
                    continue;
                }
                hotspots.push(Hotspot {
                    title,
                    hot_score: item["go_detail_count"].as_u64().unwrap_or(0),
                    source: HotspotSource::Toutiao,
                    url: item["article_url"].as_str().map(|s| s.to_string()),
                    category: item["tag"].as_str().map(|s| s.to_string()),
                    fetched_at: Utc::now(),
                });
            }
        }
        Ok(hotspots)
    }

    /// 小红书热榜（笔记搜索）
    /// API: https://edith.xiaohongshu.com/api/sns/web/v1/search/notes
    async fn fetch_xiaohongshu(&self) -> Result<Vec<Hotspot>, AppError> {
        // 小红书搜索 API
        let resp = self.http
            .post("https://edith.xiaohongshu.com/api/sns/web/v1/search/notes")
            .header("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X)")
            .header("Content-Type", "application/json")
            .header("Referer", "https://www.xiaohongshu.com/")
            .json(&serde_json::json!({
                "keyword": "热搜",
                "page": 1,
                "page_size": 20,
                "search_id": "",
                "sort": "general",
                "note_type": 0
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let mut hotspots = Vec::new();
        if let Some(items) = resp["data"]["items"].as_array() {
            for item in items {
                let note_card = &item["note_card"];
                let title = note_card["title"].as_str().unwrap_or_default().to_string();
                if title.is_empty() {
                    continue;
                }
                hotspots.push(Hotspot {
                    title,
                    hot_score: note_card["liked_count"].as_u64().unwrap_or(0),
                    source: HotspotSource::Xiaohongshu,
                    url: note_card["share_url"].as_str().map(|s| s.to_string()),
                    category: note_card["type"].as_str().map(|s| s.to_string()),
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

impl Default for SourceRateLimiter {
    fn default() -> Self {
        Self::new()
    }
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
