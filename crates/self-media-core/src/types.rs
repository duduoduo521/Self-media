use serde::{Deserialize, Serialize};

/// 发布平台枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum Platform {
    Xiaohongshu,
    Douyin,
    WeChatOfficial,
    Bilibili,
    Weibo,
    Toutiao,
}

/// 热点来源枚举（独立于 Platform，包含知乎等非发布平台）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HotspotSource {
    Weibo,
    Douyin,
    Xiaohongshu,
    Bilibili,
    Toutiao,
    Zhihu,
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 任务模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[serde(rename_all = "snake_case")]
pub enum TaskMode {
    Text,
    Video,
}

impl std::fmt::Display for TaskMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskMode::Text => write!(f, "text"),
            TaskMode::Video => write!(f, "video"),
        }
    }
}

/// MiniMax 区域
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MiniMaxRegion {
    CN,
    Global,
}

impl MiniMaxRegion {
    pub fn base_url(&self) -> &'static str {
        match self {
            Self::CN => "https://api.minimax.chat",
            Self::Global => "https://api.minimaxi.chat",
        }
    }
}

/// 统一 API 错误响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

/// 平台凭证
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCredential {
    pub platform: Platform,
    pub cookies: String,
    pub extra: std::collections::HashMap<String, String>,
}

/// 文章内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleContent {
    pub title: String,
    pub body: String,
    pub image_urls: Vec<String>,
    pub tags: Vec<String>,
    pub topic: Option<String>,
}

/// 视频内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoContent {
    pub title: String,
    pub description: String,
    pub video_path: String,
    pub cover_image: Option<String>,
    pub tags: Vec<String>,
}

/// 发布结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    pub platform: Platform,
    pub success: bool,
    pub post_id: Option<String>,
    pub post_url: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

/// Cookie 状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieStatus {
    pub platform: Platform,
    pub valid: bool,
    pub last_checked: chrono::DateTime<chrono::Utc>,
}

/// 热点条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hotspot {
    pub title: String,
    pub hot_score: u64,
    pub source: HotspotSource,
    pub url: Option<String>,
    pub category: Option<String>,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub event_date: Option<chrono::NaiveDate>,
}
