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
        credential: &PlatformCredential,
        content: &VideoContent,
    ) -> Result<PublishResult, PublishError> {
        // 小红书视频发布流程：
        // 1. 上传视频文件
        // 2. 上传视频封面
        // 3. 创建视频笔记

        // 1. 上传视频
        let video_data = tokio::fs::read(&content.video_path).await
            .map_err(|e| PublishError::PlatformError(format!("读取视频文件失败: {}", e)))?;

        // 小红书视频大小限制 1GB
        let max_size = 1024 * 1024 * 1024;
        if video_data.len() > max_size {
            return Ok(PublishResult {
                platform: Platform::Xiaohongshu,
                success: false,
                post_id: None,
                post_url: None,
                error_code: Some("SIZE_EXCEED".to_string()),
                error_message: Some("视频大小超过1GB限制".to_string()),
            });
        }

        let part = reqwest::multipart::Part::bytes(video_data)
            .file_name("video.mp4")
            .mime_str("video/mp4")
            .map_err(|e| PublishError::UploadFailed(e.to_string()))?;

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("usage", "video");

        let upload_resp = self.http
            .post("https://edith.xiaohongshu.com/api/sns/web/v1/upload_video")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .multipart(form)
            .send()
            .await
            .map_err(|e| PublishError::PlatformError(format!("视频上传失败: {}", e)))?;

        let upload_result: serde_json::Value = upload_resp.json().await
            .map_err(|e| PublishError::PlatformError(format!("解析上传响应失败: {}", e)))?;

        if !upload_result["success"].as_bool().unwrap_or(false) {
            let error_msg = upload_result["msg"].as_str().unwrap_or("视频上传失败");
            return Ok(PublishResult {
                platform: Platform::Xiaohongshu,
                success: false,
                post_id: None,
                post_url: None,
                error_code: None,
                error_message: Some(error_msg.to_string()),
            });
        }

        let video_url = upload_result["data"]["url"]
            .as_str()
            .ok_or_else(|| PublishError::PlatformError("未获取到视频URL".to_string()))?;

        let video_id = upload_result["data"]["video_id"]
            .as_str()
            .ok_or_else(|| PublishError::PlatformError("未获取到视频ID".to_string()))?;

        // 2. 上传视频封面（可选）
        let mut cover_url: Option<String> = None;
        if let Some(cover_path) = &content.cover_image {
            if let Ok(cover_data) = tokio::fs::read(cover_path).await {
                if let Ok(cover_part) = reqwest::multipart::Part::bytes(cover_data)
                    .file_name("cover.jpg")
                    .mime_str("image/jpeg")
                {
                    let cover_form = reqwest::multipart::Form::new()
                        .part("file", cover_part)
                        .text("usage", "image");

                    if let Ok(cover_resp) = self.http
                        .post("https://edith.xiaohongshu.com/api/sns/web/v1/upload_image")
                        .header("Cookie", &credential.cookies)
                        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
                        .multipart(cover_form)
                        .send()
                        .await
                    {
                        if let Ok(cover_result) = cover_resp.json::<serde_json::Value>().await {
                            cover_url = cover_result["data"]["url"].as_str().map(|s| s.to_string());
                        }
                    }
                }
            }
        }

        // 3. 创建视频笔记
        let mut note_body = serde_json::json!({
            "type": "video",
            "title": content.title,
            "desc": content.description,
            "video_id": video_id,
            "video": {
                "url": video_url,
            },
            "tag_list": content.tags.iter().map(|t| serde_json::json!({"Name": t})).collect::<Vec<_>>(),
        });

        if let Some(url) = cover_url {
            note_body["image_list"] = serde_json::json!([{
                "url": url,
                "width": 1080,
                "height": 1920,
            }]);
        }

        let resp = self.http
            .post("https://edith.xiaohongshu.com/api/sns/web/v1/feed")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .header("Content-Type", "application/json")
            .json(&note_body)
            .send()
            .await
            .map_err(|e| PublishError::PlatformError(format!("发布失败: {}", e)))?;

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
            Ok(PublishResult {
                platform: Platform::Xiaohongshu,
                success: false,
                post_id: None,
                post_url: None,
                error_code: None,
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
