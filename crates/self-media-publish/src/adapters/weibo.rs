use async_trait::async_trait;
use reqwest::Client;

use self_media_core::types::{
    ArticleContent, Platform, PlatformCredential, PublishResult, VideoContent,
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
        credential: &PlatformCredential,
        content: &VideoContent,
    ) -> Result<PublishResult, PublishError> {
        // 微博视频发布流程：
        // 1. 获取上传令牌 (upload_token)
        // 2. 上传视频文件
        // 3. 创建视频微博

        // 1. 获取上传凭证
        let token_resp = self.http
            .get("https://video.weibo.com/upload/get_upload_token.json")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .query(&[("type", "video")])
            .send()
            .await?;

        let token_data: serde_json::Value = token_resp.json().await
            .map_err(|e| PublishError::PlatformError(format!("获取上传令牌失败: {}", e)))?;

        let upload_token = token_data["upload_token"]
            .as_str()
            .ok_or_else(|| PublishError::PlatformError("无法获取上传令牌".to_string()))?;

        // 2. 读取并上传视频
        let video_data = tokio::fs::read(&content.video_path).await
            .map_err(|e| PublishError::PlatformError(format!("读取视频文件失败: {}", e)))?;

        // 微博视频大小限制 500MB
        let max_size = 500 * 1024 * 1024;
        if video_data.len() > max_size {
            return Ok(PublishResult {
                platform: Platform::Weibo,
                success: false,
                post_id: None,
                post_url: None,
                error_code: Some("SIZE_EXCEED".to_string()),
                error_message: Some("视频大小超过500MB限制".to_string()),
            });
        }

        // 上传视频
        let part = reqwest::multipart::Part::bytes(video_data)
            .file_name("video.mp4")
            .mime_str("video/mp4")
            .map_err(|e| PublishError::UploadFailed(e.to_string()))?;

        let form = reqwest::multipart::Form::new()
            .part("video", part)
            .text("upload_token", upload_token.to_string())
            .text("title", content.title.clone())
            .text("description", content.description.clone());

        let upload_resp = self.http
            .post("https://video.weibo.com/upload/upload.json")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .multipart(form)
            .send()
            .await
            .map_err(|e| PublishError::PlatformError(format!("视频上传失败: {}", e)))?;

        let upload_result: serde_json::Value = upload_resp.json().await
            .map_err(|e| PublishError::PlatformError(format!("解析上传响应失败: {}", e)))?;

        if upload_result["code"].as_i64() != Some(0) {
            let error_msg = upload_result["msg"].as_str().unwrap_or("视频上传失败");
            return Ok(PublishResult {
                platform: Platform::Weibo,
                success: false,
                post_id: None,
                post_url: None,
                error_code: upload_result["code"].as_i64().map(|c| c.to_string()),
                error_message: Some(error_msg.to_string()),
            });
        }

        let video_key = upload_result["video"]["video_id"]
            .as_str()
            .ok_or_else(|| PublishError::PlatformError("未获取到视频ID".to_string()))?;

        // 3. 发布微博
        let publish_body = serde_json::json!({
            "text": content.description.clone(),
            "video_id": video_key,
            "topic_id": content.tags.first(),
        });

        let resp = self.http
            .post("https://weibo.com/ajax/statuses/publish_video")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .header("Content-Type", "application/json")
            .json(&publish_body)
            .send()
            .await
            .map_err(|e| PublishError::PlatformError(format!("发布失败: {}", e)))?;

        let result: serde_json::Value = resp.json().await?;

        if result["ok"].as_i64() == Some(1) {
            let mid = result["mid"].as_str().unwrap_or_default();
            Ok(PublishResult {
                platform: Platform::Weibo,
                success: true,
                post_id: Some(mid.to_string()),
                post_url: Some(format!("https://weibo.com/detail/{}", mid)),
                error_code: None,
                error_message: None,
            })
        } else {
            Ok(PublishResult {
                platform: Platform::Weibo,
                success: false,
                post_id: None,
                post_url: None,
                error_code: result["errno"].as_i64().map(|c| c.to_string()),
                error_message: result["msg"].as_str().map(|s| s.to_string()),
            })
        }
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
