use async_trait::async_trait;
use reqwest::Client;

use self_media_core::types::{
    ArticleContent, Platform, PlatformCredential, PublishResult, VideoContent,
};

use crate::publisher::{PlatformPublisher, PublishError};

/// 小红书发布适配器
pub struct XiaohongshuPublisher {
    http: Client,
}

impl XiaohongshuPublisher {
    pub fn new(http: Client) -> Self {
        Self { http }
    }
}

#[async_trait]
impl PlatformPublisher for XiaohongshuPublisher {
    fn platform(&self) -> Platform {
        Platform::Xiaohongshu
    }

    async fn check_login_status(
        &self,
        credential: &PlatformCredential,
    ) -> Result<bool, PublishError> {
        let resp = self
            .http
            .get("https://edith.xiaohongshu.com/api/sns/web/v1/user/selfinfo")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .send()
            .await?;

        if resp.status().is_success() {
            let body: serde_json::Value = resp.json().await?;
            Ok(body["data"]["user_id"].is_string())
        } else {
            Ok(false)
        }
    }

    async fn login(
        &self,
        _credential: &PlatformCredential,
    ) -> Result<PlatformCredential, PublishError> {
        Err(PublishError::PlatformError(
            "小红书仅支持扫码登录".into(),
        ))
    }

    async fn publish_article(
        &self,
        credential: &PlatformCredential,
        content: &ArticleContent,
    ) -> Result<PublishResult, PublishError> {
        // 小红书笔记发布需要先上传图片，再创建笔记
        if content.image_urls.is_empty() {
            return Err(PublishError::PlatformError(
                "小红书笔记至少需要一张图片".into(),
            ));
        }

        let mut image_urls = Vec::new();
        for url in &content.image_urls {
            image_urls.push(serde_json::json!({
                "url": url,
                "width": 1080,
                "height": 1080,
            }));
        }

        let body = serde_json::json!({
            "title": content.title,
            "desc": content.body,
            "type": "normal",
            "image_info": image_urls,
            "tag_list": content.tags.iter().map(|t| serde_json::json!({"name": t})).collect::<Vec<_>>(),
        });

        let resp = self
            .http
            .post("https://edith.xiaohongshu.com/api/sns/web/v1/feed")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if result["success"].as_bool().unwrap_or(false) {
            let note_id = result["data"]["note_id"].as_str().unwrap_or_default();
            Ok(PublishResult {
                platform: Platform::Xiaohongshu,
                success: true,
                post_id: Some(note_id.to_string()),
                post_url: Some(format!("https://www.xiaohongshu.com/explore/{}", note_id)),
                error_code: None,
                error_message: None,
            })
        } else {
            let error_message = result["msg"].as_str().unwrap_or("小红书发布失败").to_string();
            Ok(PublishResult {
                platform: Platform::Xiaohongshu,
                success: false,
                post_id: None,
                post_url: None,
                error_code: None,
                error_message: Some(error_message),
            })
        }
    }

    async fn publish_video(
        &self,
        _credential: &PlatformCredential,
        _content: &VideoContent,
    ) -> Result<PublishResult, PublishError> {
        Err(PublishError::PlatformError(
            "小红书视频发布暂未实现".into(),
        ))
    }

    async fn upload_image(
        &self,
        credential: &PlatformCredential,
        image_data: &[u8],
        filename: &str,
    ) -> Result<String, PublishError> {
        let part = reqwest::multipart::Part::bytes(image_data.to_vec())
            .file_name(filename.to_string())
            .mime_str("image/jpeg")
            .map_err(|e| PublishError::UploadFailed(e.to_string()))?;

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("usage", "image");

        let resp = self
            .http
            .post("https://edith.xiaohongshu.com/api/sns/web/v1/upload_image")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .multipart(form)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;
        result["data"]["url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PublishError::UploadFailed("小红书图片上传失败".into()))
    }

    fn rate_limit_seconds(&self) -> u64 {
        60
    }
}
