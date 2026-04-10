//! 微信公众号扫码登录处理器
//! 
//! 微信公众号使用 OAuth2 扫码登录流程
//! API 端点: mp.weixin.qq.com

use async_trait::async_trait;
use reqwest::Client;

use self_media_core::types::{Platform, PlatformCredential};
use crate::publisher::PublishError;
use crate::qr_login::{QrCodeInfo, QrCodeStatus, QrLoginHandler};

/// 微信公众号扫码登录处理器
pub struct WechatQrLogin {
    http: Client,
    /// 微信公众号 AppID
    app_id: String,
}

impl WechatQrLogin {
    pub fn new(http: Client) -> Self {
        Self {
            http,
            app_id: std::env::var("WECHAT_APP_ID")
                .unwrap_or_else(|_| "YOUR_WECHAT_APP_ID".to_string()),
        }
    }
    
    /// 生成微信公众号登录二维码 URL
    async fn generate_qrcode_from_api(&self) -> Result<(String, String), PublishError> {
        let state = uuid::Uuid::new_v4().to_string();
        
        // 微信公众平台 OAuth2 扫码登录 URL
        let redirect_uri = std::env::var("WECHAT_REDIRECT_URI")
            .unwrap_or_else(|_| "https://mp.weixin.qq.com/cgi-bin/callback".to_string());
        
        // 微信需要 URL encode 的 redirect_uri
        let auth_url = format!(
            "https://open.weixin.qq.com/connect/qrconnect?appid={}&redirect_uri={}&response_type=code&scope=snsapi_login&state={}#wechat_redirect",
            self.app_id,
            urlencoding_encode(&redirect_uri),
            state
        );
        
        Ok((auth_url, state))
    }
}

#[async_trait]
impl QrLoginHandler for WechatQrLogin {
    fn platform(&self) -> Platform {
        Platform::WeChatOfficial
    }
    
    async fn generate_qrcode(&self, _http: &Client) -> Result<QrCodeInfo, PublishError> {
        let (url, state) = self.generate_qrcode_from_api().await?;
        
        let qr_id = format!("wechat_{}", state);
        
        Ok(QrCodeInfo {
            id: qr_id,
            image: format!("https://api.qrserver.com/v1/create-qr-code/?size=200x200&data={}", 
                          urlencoding_encode(&url)),
            url,
            status: QrCodeStatus::Pending,
            created_at: chrono::Utc::now().timestamp(),
        })
    }
    
    async fn query_status(&self, _http: &Client, qr_id: &str) -> Result<QrCodeStatus, PublishError> {
        // 微信 OAuth 需要回调
        let _ = qr_id;
        Ok(QrCodeStatus::Pending)
    }
    
    async fn confirm_login(&self, _http: &Client, qr_id: &str) -> Result<PlatformCredential, PublishError> {
        // 微信公众号扫码登录需要通过 OAuth 回调获取 code
        // 然后用 code 换取 access_token
        let _ = qr_id;
        Err(PublishError::PlatformError(
            "微信公众号扫码登录需要通过 OAuth 授权页面获取 code".to_string()
        ))
    }
}

/// 用 code 换取 access_token（独立函数）
pub async fn wechat_exchange_token(app_id: &str, app_secret: &str, code: &str, http: &Client) -> Result<PlatformCredential, PublishError> {
    let url = format!(
        "https://api.weixin.qq.com/sns/oauth2/access_token?appid={}&secret={}&code={}&grant_type=authorization_code",
        app_id,
        app_secret,
        code
    );
    
    let resp = http
        .get(&url)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    
    if let Some(errcode) = resp["errcode"].as_i64() {
        if errcode != 0 {
            return Err(PublishError::PlatformError(
                format!("微信获取 access_token 失败: {}", resp["errmsg"])
            ));
        }
    }
    
    let access_token = resp["access_token"]
        .as_str()
        .ok_or_else(|| PublishError::PlatformError("微信未返回 access_token".to_string()))?;
    
    let openid = resp["openid"]
        .as_str()
        .ok_or_else(|| PublishError::PlatformError("微信未返回 openid".to_string()))?;
    
    let mut extra = std::collections::HashMap::new();
    extra.insert("openid".to_string(), openid.to_string());
    extra.insert("access_token".to_string(), access_token.to_string());
    
    Ok(PlatformCredential {
        platform: Platform::WeChatOfficial,
        cookies: format!("access_token={}", access_token),
        extra,
    })
}

/// URL encode 辅助函数
fn urlencoding_encode(input: &str) -> String {
    let mut result = String::new();
    for c in input.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                result.push(c);
            }
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}
