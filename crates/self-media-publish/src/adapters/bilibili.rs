use async_trait::async_trait;
use reqwest::Client;

use self_media_core::types::{
    ArticleContent, CookieStatus, Platform, PlatformCredential, PublishResult, VideoContent,
};

use crate::publisher::{PlatformPublisher, PublishError};

/// B站发布适配器
pub struct BilibiliPublisher {
    http: Client,
}

impl BilibiliPublisher {
    pub fn new(http: Client) -> Self {
        Self { http }
    }

    /// 从 Cookie 中提取 CSRF Token (bili_jct)
    fn extract_csrf(credential: &PlatformCredential) -> Option<&str> {
        credential
            .cookies
            .split(';')
            .find_map(|c| c.trim().strip_prefix("bili_jct="))
    }
}

#[async_trait]
impl PlatformPublisher for BilibiliPublisher {
    fn platform(&self) -> Platform {
        Platform::Bilibili
    }

    async fn check_login_status(
        &self,
        credential: &PlatformCredential,
    ) -> Result<bool, PublishError> {
        let resp = self
            .http
            .get("https://api.bilibili.com/x/web-interface/nav")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .send()
            .await?;

        if resp.status().is_success() {
            let body: serde_json::Value = resp.json().await?;
            Ok(body["data"]["isLogin"].as_bool().unwrap_or(false))
        } else {
            Ok(false)
        }
    }

    async fn login(
        &self,
        _credential: &PlatformCredential,
    ) -> Result<PlatformCredential, PublishError> {
        Err(PublishError::PlatformError(
            "B站仅支持扫码登录".into(),
        ))
    }

    async fn publish_article(
        &self,
        credential: &PlatformCredential,
        content: &ArticleContent,
    ) -> Result<PublishResult, PublishError> {
        let csrf = Self::extract_csrf(credential)
            .ok_or_else(|| PublishError::CookieExpired("缺少 bili_jct (CSRF Token)".into()))?;

        // 先上传文章封面
        let banner_url = if let Some(first_img) = content.image_urls.first() {
            Some(first_img.clone())
        } else {
            None
        };

        let mut body = serde_json::Map::new();
        body.insert("title".into(), serde_json::Value::String(content.title.clone()));
        body.insert(
            "content".into(),
            serde_json::Value::String(content.body.clone()),
        );
        body.insert("csrf".into(), serde_json::Value::String(csrf.to_string()));
        if let Some(banner) = banner_url {
            body.insert("banner_url".into(), serde_json::Value::String(banner));
        }

        let resp = self
            .http
            .post("https://api.bilibili.com/x/article/creative/draft/addupdate")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;

        if result["code"].as_i64() == Some(0) {
            let aid = result["data"]["aid"].as_i64().map(|id| id.to_string());
            Ok(PublishResult {
                platform: Platform::Bilibili,
                success: true,
                post_id: aid.clone(),
                post_url: aid.map(|id| format!("https://www.bilibili.com/read/cv{}", id)),
                error_code: None,
                error_message: None,
            })
        } else {
            let error_message = result["message"].as_str().unwrap_or("B站发布失败").to_string();
            Ok(PublishResult {
                platform: Platform::Bilibili,
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
        // B站视频发布需要分片上传，实现复杂
        Err(PublishError::PlatformError(
            "B站视频发布暂未实现".into(),
        ))
    }

    async fn upload_image(
        &self,
        credential: &PlatformCredential,
        image_data: &[u8],
        filename: &str,
    ) -> Result<String, PublishError> {
        let csrf = Self::extract_csrf(credential)
            .ok_or_else(|| PublishError::CookieExpired("缺少 bili_jct (CSRF Token)".into()))?;

        let part = reqwest::multipart::Part::bytes(image_data.to_vec())
            .file_name(filename.to_string())
            .mime_str("image/jpeg")
            .map_err(|e| PublishError::UploadFailed(e.to_string()))?;

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("csrf", csrf.to_string());

        let resp = self
            .http
            .post("https://api.bilibili.com/x/article/creative/article/upcover")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .multipart(form)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;
        result["data"]["url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PublishError::UploadFailed("B站图片上传失败".into()))
    }

    fn rate_limit_seconds(&self) -> u64 {
        30
    }
}
