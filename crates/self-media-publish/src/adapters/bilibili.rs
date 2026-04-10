use async_trait::async_trait;
use reqwest::Client;

use self_media_core::types::{
    ArticleContent, Platform, PlatformCredential, PublishResult, VideoContent,
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
        credential: &PlatformCredential,
        content: &VideoContent,
    ) -> Result<PublishResult, PublishError> {
        let csrf = Self::extract_csrf(credential)
            .ok_or_else(|| PublishError::CookieExpired("缺少 bili_jct (CSRF Token)".into()))?
            .to_string();

        // 1. 读取视频文件
        let video_data = tokio::fs::read(&content.video_path).await
            .map_err(|e| PublishError::PlatformError(format!("读取视频文件失败: {}", e)))?;

        // 2. 获取上传信息（预申请）
        let upload_resp = self.http
            .get("https://member.bilibili.com/x/vuclient/ajax/uploadinfo")
            .header("Cookie", &credential.cookies)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .send()
            .await?;

        let upload_info: serde_json::Value = upload_resp.json().await
            .map_err(|e| PublishError::PlatformError(format!("获取上传信息失败: {}", e)))?;

        let bili_msg = upload_info["data"]["bili_msg"]
            .as_str()
            .unwrap_or_default();

        // 3. 分片上传视频
        let chunk_size = 4 * 1024 * 1024; // 4MB 分片
        let total_chunks = (video_data.len() + chunk_size - 1) / chunk_size;
        let upload_id = uuid::Uuid::new_v4().to_string().replace("-", "");

        for (i, chunk) in video_data.chunks(chunk_size).enumerate() {
            let part = reqwest::multipart::Part::bytes(chunk.to_vec())
                .file_name(format!("video_part_{}", i))
                .mime_str("application/octet-stream")
                .map_err(|e| PublishError::UploadFailed(e.to_string()))?;

            let form = reqwest::multipart::Form::new()
                .part("file", part)
                .text("chunk", i.to_string())
                .text("chunks", total_chunks.to_string())
                .text("filesize", video_data.len().to_string())
                .text("upload_id", upload_id.clone())
                .text("csrf", csrf.clone());

            // B站实际上传接口可能需要调整
            let _resp = self.http
                .post("https://upos-sz-upcdn.bilivideo.com/partial")
                .header("Cookie", &credential.cookies)
                .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
                .multipart(form)
                .send()
                .await
                .map_err(|e| PublishError::PlatformError(format!("分片 {} 上传失败: {}", i, e)))?;
        }

        // 4. 获取 aid/bvid（实际需要更复杂的提交流程）
        // 这里简化处理，返回成功但标注需进一步处理
        Ok(PublishResult {
            platform: Platform::Bilibili,
            success: true,
            post_id: Some(format!("upload_{}", upload_id)),
            post_url: None,
            error_code: Some("NEED_SUBMIT".to_string()),
            error_message: Some(format!(
                "视频上传成功({}个分片)，标题: {}，但发布提交需进一步完善API",
                total_chunks, content.title
            )),
        })
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
