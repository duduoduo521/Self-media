use async_trait::async_trait;
use self_media_core::types::{
    ArticleContent, CookieStatus, Platform, PlatformCredential, PublishResult, VideoContent,
};

/// 平台发布适配器 Trait
#[async_trait]
pub trait PlatformPublisher: Send + Sync {
    /// 平台标识
    fn platform(&self) -> Platform;

    /// 检查登录态是否有效
    async fn check_login_status(&self, credential: &PlatformCredential) -> Result<bool, PublishError>;

    /// 触发登录流程
    async fn login(&self, credential: &PlatformCredential) -> Result<PlatformCredential, PublishError>;

    /// 发布图文/文章
    async fn publish_article(
        &self,
        credential: &PlatformCredential,
        content: &ArticleContent,
    ) -> Result<PublishResult, PublishError>;

    /// 发布视频
    async fn publish_video(
        &self,
        credential: &PlatformCredential,
        content: &VideoContent,
    ) -> Result<PublishResult, PublishError>;

    /// 上传图片
    async fn upload_image(
        &self,
        credential: &PlatformCredential,
        image_data: &[u8],
        filename: &str,
    ) -> Result<String, PublishError>;

    /// 获取平台发布频率限制（返回最小间隔秒数）
    fn rate_limit_seconds(&self) -> u64 {
        30
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("Cookie 过期: {0}")]
    CookieExpired(String),

    #[error("上传失败: {0}")]
    UploadFailed(String),

    #[error("发布限流: {0}")]
    RateLimited(String),

    #[error("签名验证失败: {0}")]
    SignatureFailed(String),

    #[error("平台错误: {0}")]
    PlatformError(String),

    #[error("网络错误: {0}")]
    Network(#[from] reqwest::Error),
}
