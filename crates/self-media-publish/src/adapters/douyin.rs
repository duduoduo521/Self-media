use async_trait::async_trait;
use reqwest::Client;

use self_media_core::types::{
    ArticleContent, Platform, PlatformCredential, PublishResult, VideoContent,
};

use crate::publisher::{PlatformPublisher, PublishError};

/// 抖音发布适配器
pub struct DouyinPublisher {
    http: Client,
}

impl DouyinPublisher {
    pub fn new(http: Client) -> Self {
        Self { http }
    }
}

#[async_trait]
impl PlatformPublisher for DouyinPublisher {
    fn platform(&self) -> Platform {
        Platform::Douyin
    }

    async fn check_login_status(
        &self,
        credential: &PlatformCredential,
    ) -> Result<bool, PublishError> {
        let resp = self
            .http
            .get("https://creator.douyin.com/creator-micro/home")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .send()
            .await?;

        // 检查是否被重定向到登录页
        Ok(!resp.url().as_str().contains("login"))
    }

    async fn login(
        &self,
        _credential: &PlatformCredential,
    ) -> Result<PlatformCredential, PublishError> {
        Err(PublishError::PlatformError(
            "抖音仅支持扫码登录".into(),
        ))
    }

    async fn publish_article(
        &self,
        _credential: &PlatformCredential,
        _content: &ArticleContent,
    ) -> Result<PublishResult, PublishError> {
        // 抖音以视频为主，图文发布接口不同
        Err(PublishError::PlatformError(
            "抖音图文发布暂未实现，请使用视频模式".into(),
        ))
    }

    async fn publish_video(
        &self,
        credential: &PlatformCredential,
        content: &VideoContent,
    ) -> Result<PublishResult, PublishError> {
        // 抖音视频发布需要先上传视频文件，然后发布
        // 这里提供骨架实现
        let body = serde_json::json!({
            "title": content.title,
            "desc": content.description,
            "tags": content.tags,
        });

        let resp = self
            .http
            .post("https://creator.douyin.com/aweme/v1/creator/item/create/")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if result["status_code"].as_i64() == Some(0) {
            let item_id = result["item_id"].as_str().unwrap_or_default();
            Ok(PublishResult {
                platform: Platform::Douyin,
                success: true,
                post_id: Some(item_id.to_string()),
                post_url: Some(format!("https://www.douyin.com/video/{}", item_id)),
                error_code: None,
                error_message: None,
            })
        } else {
            let error_message = result["status_msg"]
                .as_str()
                .unwrap_or("抖音发布失败")
                .to_string();
            Ok(PublishResult {
                platform: Platform::Douyin,
                success: false,
                post_id: None,
                post_url: None,
                error_code: result["status_code"].as_i64().map(|c| c.to_string()),
                error_message: Some(error_message),
            })
        }
    }

    async fn upload_image(
        &self,
        _credential: &PlatformCredential,
        _image_data: &[u8],
        _filename: &str,
    ) -> Result<String, PublishError> {
        Err(PublishError::PlatformError(
            "抖音图片上传暂未实现".into(),
        ))
    }

    fn rate_limit_seconds(&self) -> u64 {
        60
    }
}
