//! 微博扫码登录处理器
//! 
//! 微博使用 OAuth2 扫码登录流程
//! API 文档: https://open.weibo.com/wiki/OAuth2/qrcode_authorize

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use self_media_core::types::{Platform, PlatformCredential};
use crate::publisher::PublishError;
use crate::qr_login::{QrCodeInfo, QrCodeStatus, QrLoginHandler};

/// 微博二维码状态查询响应
#[derive(Debug, Deserialize)]
struct WeiboQrStatusResponse {
    #[serde(rename = "status")]
    status: i32,  // 1=等待扫码, 2=已扫码待确认, 3=已授权
    #[serde(rename = "code")]
    code: Option<String>,  // 状态为3时返回，用于换取access_token
    #[serde(rename = "url")]
    url: Option<String>,
    #[serde(rename = "error")]
    error: Option<String>,
}

/// 微博 OAuth 令牌响应
#[derive(Debug, Deserialize)]
struct WeiboTokenResponse {
    #[serde(rename = "access_token")]
    access_token: Option<String>,
    #[serde(rename = "uid")]
    uid: Option<String>,
    #[serde(rename = "expires_in")]
    expires_in: Option<i64>,
    #[serde(rename = "error")]
    error: Option<String>,
    #[serde(rename = "error_code")]
    error_code: Option<i32>,
}

/// 微博扫码登录处理器
pub struct WeiboQrLogin {
    http: Client,
    /// 应用 App Key（需要在微博开放平台申请）
    app_key: String,
    /// 回调地址
    redirect_uri: String,
}

impl WeiboQrLogin {
    pub fn new(http: Client) -> Self {
        // 默认值，实际使用时应从配置读取
        Self {
            http,
            app_key: std::env::var("WEIBO_APP_KEY")
                .unwrap_or_else(|_| "YOUR_WEIBO_APP_KEY".to_string()),
            redirect_uri: std::env::var("WEIBO_REDIRECT_URI")
                .unwrap_or_else(|_| "https://api.weibo.com/oauth2/default.html".to_string()),
        }
    }
    
    /// 从微博 API 获取二维码
    async fn fetch_qrcode_from_api(&self) -> Result<(String, String), PublishError> {
        let url = "https://api.weibo.com/oauth2/qrcode_authorize/qrcode";
        
        let params = [
            ("client_id", self.app_key.as_str()),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", ""),
        ];
        
        let resp = self.http
            .get(url)
            .query(&params)
            .send()
            .await?;
        
        let status = resp.status();
        let body = resp.text().await?;
        
        if !status.is_success() {
            tracing::warn!("微博二维码获取失败: {} - {}", status, body);
            return Err(PublishError::PlatformError(format!("微博二维码获取失败: {}", status)));
        }
        
        // 解析响应，格式如: qr_url=xxx&oauthKey=xxx
        let mut qr_url = String::new();
        let mut oauth_key = String::new();
        
        for pair in body.split('&') {
            let parts: Vec<&str> = pair.split('=').collect();
            if parts.len() == 2 {
                match parts[0] {
                    "qr_url" => qr_url = urlencoding_decode(parts[1]),
                    "oauthKey" | "oauth_key" => oauth_key = parts[1].to_string(),
                    _ => {}
                }
            }
        }
        
        if oauth_key.is_empty() {
            return Err(PublishError::PlatformError("微博未返回 oauthKey".to_string()));
        }
        
        Ok((qr_url, oauth_key))
    }
    
    /// 查询二维码状态
    async fn query_qr_status(&self, oauth_key: &str) -> Result<WeiboQrStatusResponse, PublishError> {
        let url = "https://api.weibo.com/oauth2/qrcode_authorize/show";
        
        let params = [
            ("client_id", self.app_key.as_str()),
            ("oauthKey", oauth_key),
        ];
        
        let resp = self.http
            .get(url)
            .query(&params)
            .send()
            .await
            .map_err(PublishError::Network)?;
        
        let body = resp.text().await?;
        
        serde_json::from_str(&body)
            .map_err(|e| {
                tracing::warn!("微博状态解析失败: {} - {}", e, body);
                PublishError::ParseError(e.to_string())
            })
    }
    
    /// 用 code 换取 access_token
    async fn exchange_token(&self, code: &str) -> Result<WeiboTokenResponse, PublishError> {
        let url = "https://api.weibo.com/oauth2/access_token";
        
        let params = [
            ("client_id", self.app_key.as_str()),
            ("client_secret", ""),  // 微博 OAuth2 可以不传 secret
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", self.redirect_uri.as_str()),
        ];
        
        let resp = self.http
            .post(url)
            .form(&params)
            .send()
            .await
            .map_err(PublishError::Network)?;
        
        let body = resp.text().await?;
        
        serde_json::from_str(&body)
            .map_err(|e| {
                tracing::warn!("微博令牌解析失败: {} - {}", e, body);
                PublishError::ParseError(e.to_string())
            })
    }
}

#[async_trait]
impl QrLoginHandler for WeiboQrLogin {
    fn platform(&self) -> Platform {
        Platform::Weibo
    }
    
    async fn generate_qrcode(&self, _http: &Client) -> Result<QrCodeInfo, PublishError> {
        let (qr_url, oauth_key) = self.fetch_qrcode_from_api().await?;
        
        let qr_id = format!("weibo_{}", oauth_key);
        
        Ok(QrCodeInfo {
            id: qr_id,
            image: qr_url.clone(),  // 二维码图片 URL
            url: qr_url,
            status: QrCodeStatus::Pending,
            created_at: chrono::Utc::now().timestamp(),
        })
    }
    
    async fn query_status(&self, _http: &Client, qr_id: &str) -> Result<QrCodeStatus, PublishError> {
        // 从 qr_id 提取 oauth_key
        let oauth_key = qr_id.strip_prefix("weibo_")
            .ok_or_else(|| PublishError::PlatformError("无效的二维码 ID".to_string()))?;
        
        let response = self.query_qr_status(oauth_key).await?;
        
        if let Some(error) = response.error {
            return Err(PublishError::PlatformError(format!("微博扫码错误: {}", error)));
        }
        
        match response.status {
            1 => Ok(QrCodeStatus::Pending),
            2 => Ok(QrCodeStatus::Scanned),
            3 => Ok(QrCodeStatus::Confirmed),
            _ => Ok(QrCodeStatus::Pending),
        }
    }
    
    async fn confirm_login(&self, _http: &Client, qr_id: &str) -> Result<PlatformCredential, PublishError> {
        let oauth_key = qr_id.strip_prefix("weibo_")
            .ok_or_else(|| PublishError::PlatformError("无效的二维码 ID".to_string()))?;
        
        // 先查询状态，确认是否已授权
        let status_resp = self.query_qr_status(oauth_key).await?;
        
        if status_resp.status != 3 {
            return Err(PublishError::PlatformError("用户尚未授权登录".to_string()));
        }
        
        let code = status_resp.code
            .ok_or_else(|| PublishError::PlatformError("微博未返回授权码".to_string()))?;
        
        // 用 code 换取 access_token
        let token_resp = self.exchange_token(&code).await?;
        
        if let Some(error) = token_resp.error {
            return Err(PublishError::PlatformError(format!("微博令牌获取失败: {}", error)));
        }
        
        let access_token = token_resp.access_token
            .ok_or_else(|| PublishError::PlatformError("微博未返回 access_token".to_string()))?;
        
        let uid = token_resp.uid
            .ok_or_else(|| PublishError::PlatformError("微博未返回 uid".to_string()))?;
        
        let mut extra = std::collections::HashMap::new();
        extra.insert("uid".to_string(), uid);
        extra.insert("access_token".to_string(), access_token.clone());
        extra.insert("expires_in".to_string(), token_resp.expires_in.unwrap_or(0).to_string());
        
        Ok(PlatformCredential {
            platform: Platform::Weibo,
            cookies: format!("access_token={}", access_token),
            extra,
        })
    }
}

/// URL decode 辅助函数
fn urlencoding_decode(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    
    result
}
