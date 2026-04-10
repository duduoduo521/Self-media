use async_trait::async_trait;
use reqwest::Client;

use self_media_core::types::{
    ArticleContent, Platform, PlatformCredential, PublishResult, VideoContent,
};

use crate::publisher::{PlatformPublisher, PublishError};

/// 今日头条发布适配器
pub struct ToutiaoPublisher {
    http: Client,
}

impl ToutiaoPublisher {
    pub fn new(http: Client) -> Self {
        Self { http }
    }
}

#[async_trait]
impl PlatformPublisher for ToutiaoPublisher {
    fn platform(&self) -> Platform {
        Platform::Toutiao
    }

    async fn check_login_status(
        &self,
        credential: &PlatformCredential,
    ) -> Result<bool, PublishError> {
        let resp = self
            .http
            .get("https://mp.toutiao.com/auth/page/login/?redirect_url=/")
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
            "头条号仅支持扫码登录".into(),
        ))
    }

    async fn publish_article(
        &self,
        credential: &PlatformCredential,
        content: &ArticleContent,
    ) -> Result<PublishResult, PublishError> {
        let mut body = serde_json::Map::new();
        body.insert("title".into(), serde_json::Value::String(content.title.clone()));
        body.insert(
            "content".into(),
            serde_json::Value::String(content.body.clone()),
        );
        body.insert("save".into(), serde_json::Value::Number(0.into())); // 0=直接发布

        let resp = self
            .http
            .post("https://mp.toutiao.com/pgc/article/create")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let result: serde_json::Value = resp.json().await?;

        if status.is_success() {
            let post_id = result["data"]["article_id"].as_str().map(|s| s.to_string());
            Ok(PublishResult {
                platform: Platform::Toutiao,
                success: true,
                post_id: post_id.clone(),
                post_url: post_id.map(|id| format!("https://www.toutiao.com/article/{}", id)),
                error_code: None,
                error_message: None,
            })
        } else {
            let error_message = result["message"].as_str().unwrap_or("发布失败").to_string();
            Ok(PublishResult {
                platform: Platform::Toutiao,
                success: false,
                post_id: None,
                post_url: None,
                error_code: result["code"].as_i64().map(|c| c.to_string()),
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
            "头条视频发布暂未实现".into(),
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
            .text("use_watermark", "0");

        let resp = self
            .http
            .post("https://mp.toutiao.com/upload/image/")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .multipart(form)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;
        result["data"]["url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PublishError::UploadFailed("头条图片上传失败".into()))
    }

    fn rate_limit_seconds(&self) -> u64 {
        30
    }
}
