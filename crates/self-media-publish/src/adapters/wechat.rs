use async_trait::async_trait;
use reqwest::Client;

use self_media_core::types::{
    ArticleContent, Platform, PlatformCredential, PublishResult, VideoContent,
};

use crate::publisher::{PlatformPublisher, PublishError};

/// 微信公众号发布适配器
pub struct WeChatPublisher {
    http: Client,
}

impl WeChatPublisher {
    pub fn new(http: Client) -> Self {
        Self { http }
    }
}

#[async_trait]
impl PlatformPublisher for WeChatPublisher {
    fn platform(&self) -> Platform {
        Platform::WeChatOfficial
    }

    async fn check_login_status(
        &self,
        credential: &PlatformCredential,
    ) -> Result<bool, PublishError> {
        let resp = self
            .http
            .get("https://mp.weixin.qq.com/cgi-bin/home?t=home/index&lang=zh_CN")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .send()
            .await?;

        // 公众号后台登录后会有特定页面特征
        Ok(!resp.url().as_str().contains("bizlogin"))
    }

    async fn login(
        &self,
        _credential: &PlatformCredential,
    ) -> Result<PlatformCredential, PublishError> {
        Err(PublishError::PlatformError(
            "微信公众号仅支持扫码登录".into(),
        ))
    }

    async fn publish_article(
        &self,
        credential: &PlatformCredential,
        content: &ArticleContent,
    ) -> Result<PublishResult, PublishError> {
        // 公众号发布分两步：1. 创建草稿 2. 提交发布
        // 第一步：上传图文素材到草稿箱
        let thumb_media_id = match content.image_urls.first() {
            Some(url) => self.upload_thumb_media(credential, url).await.ok(),
            None => None,
        };

        let mut articles = serde_json::Map::new();
        articles.insert("title".into(), serde_json::Value::String(content.title.clone()));
        articles.insert(
            "content".into(),
            serde_json::Value::String(content.body.clone()),
        );
        articles.insert(
            "thumb_media_id".into(),
            serde_json::Value::String(thumb_media_id.unwrap_or_default()),
        );
        articles.insert(
            "author".into(),
            serde_json::Value::String(String::new()),
        );
        articles.insert(
            "digest".into(),
            serde_json::Value::String(content.body.chars().take(64).collect()),
        );

        let body = serde_json::json!({ "articles": [articles] });

        let resp = self
            .http
            .post("https://mp.weixin.qq.com/cgi-bin/draft/add")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let result: serde_json::Value = resp.json().await?;

        if status.is_success() && result["media_id"].is_string() {
            let media_id = result["media_id"].as_str().unwrap_or_default();
            Ok(PublishResult {
                platform: Platform::WeChatOfficial,
                success: true,
                post_id: Some(media_id.to_string()),
                post_url: None,
                error_code: None,
                error_message: None,
            })
        } else {
            let error_message = result["errmsg"]
                .as_str()
                .unwrap_or("微信公众号发布失败")
                .to_string();
            Ok(PublishResult {
                platform: Platform::WeChatOfficial,
                success: false,
                post_id: None,
                post_url: None,
                error_code: result["errcode"].as_i64().map(|c| c.to_string()),
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
            "微信公众号视频发布暂未实现".into(),
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
            .text("type", "image");

        let resp = self
            .http
            .post("https://mp.weixin.qq.com/cgi-bin/filetransfer")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .multipart(form)
            .send()
            .await?;

        let result: serde_json::Value = resp.json().await?;
        result["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PublishError::UploadFailed("微信图片上传失败".into()))
    }

    fn rate_limit_seconds(&self) -> u64 {
        60 // 公众号每日发布次数有限
    }
}

impl WeChatPublisher {
    /// 上传缩略图素材
    async fn upload_thumb_media(
        &self,
        _credential: &PlatformCredential,
        _image_url: &str,
    ) -> Result<String, PublishError> {
        // 下载图片并上传为素材
        // 骨架实现
        Err(PublishError::UploadFailed("缩略图上传暂未实现".into()))
    }
}
