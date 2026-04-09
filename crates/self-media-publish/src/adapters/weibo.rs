use async_trait::async_trait;
use reqwest::Client;

use self_media_core::types::{
    ArticleContent, CookieStatus, Platform, PlatformCredential, PublishResult, VideoContent,
};

use crate::publisher::{PlatformPublisher, PublishError};

/// 微博发布适配器
pub struct WeiboPublisher {
    http: Client,
}

impl WeiboPublisher {
    pub fn new(http: Client) -> Self {
        Self { http }
    }
}

#[async_trait]
impl PlatformPublisher for WeiboPublisher {
    fn platform(&self) -> Platform {
        Platform::Weibo
    }

    async fn check_login_status(
        &self,
        credential: &PlatformCredential,
    ) -> Result<bool, PublishError> {
        let resp = self
            .http
            .get("https://weibo.com/ajax/profile/info")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .send()
            .await?;

        if resp.status().is_success() {
            let body: serde_json::Value = resp.json().await?;
            Ok(body["data"]["user"].is_object())
        } else {
            Ok(false)
        }
    }

    async fn login(
        &self,
        _credential: &PlatformCredential,
    ) -> Result<PlatformCredential, PublishError> {
        // 微博使用扫码登录，不实现密码登录
        Err(PublishError::PlatformError(
            "微博仅支持扫码登录，请使用 QR 码登录流程".into(),
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

        let resp = self
            .http
            .post("https://weibo.com/ajax/articles/create")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let result: serde_json::Value = resp.json().await?;

        if status.is_success() && result["ok"].as_i64() == Some(1) {
            let mid = result["data"]["mid"].as_str().unwrap_or("");
            Ok(PublishResult {
                platform: Platform::Weibo,
                success: true,
                post_id: Some(mid.to_string()),
                post_url: Some(format!("https://weibo.com/detail/{}", mid)),
                error_code: None,
                error_message: None,
            })
        } else {
            let error_code = result["errno"].as_i64().map(|c| c.to_string());
            let error_message = result["message"].as_str().unwrap_or("发布失败").to_string();
            Ok(PublishResult {
                platform: Platform::Weibo,
                success: false,
                post_id: None,
                post_url: None,
                error_code,
                error_message: Some(error_message),
            })
        }
    }

    async fn publish_video(
        &self,
        _credential: &PlatformCredential,
        _content: &VideoContent,
    ) -> Result<PublishResult, PublishError> {
        // 微博视频发布需要先上传视频文件，然后发布
        // 这里提供骨架实现
        Err(PublishError::PlatformError(
            "微博视频发布暂未实现，请使用图文发布".into(),
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

        let form = reqwest::multipart::Form::new().part("file", part);

        let resp = self
            .http
            .post("https://weibo.com/ajax/libs/image/upload")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .multipart(form)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;
        result["data"]["pic_pid"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PublishError::UploadFailed("图片上传失败：未返回 pid".into()))
    }

    fn rate_limit_seconds(&self) -> u64 {
        60 // 微博发布间隔至少 60 秒
    }
}
