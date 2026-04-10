//! 小红书扫码登录处理器
//! 
//! 小红书使用 OAuth2 扫码登录流程
//! API 端点: xhs.cn

use async_trait::async_trait;
use reqwest::Client;

use self_media_core::types::{Platform, PlatformCredential};
use crate::publisher::PublishError;
use crate::qr_login::{QrCodeInfo, QrCodeStatus, QrLoginHandler};

/// 小红书扫码登录处理器
pub struct XiaohongshuQrLogin {
    http: Client,
    /// 客户端 ID
    client_id: String,
}

impl XiaohongshuQrLogin {
    pub fn new(http: Client) -> Self {
        Self {
            http,
            client_id: std::env::var("XIAOHONGSHU_CLIENT_ID")
                .unwrap_or_else(|_| "YOUR_XIAOHONGSHU_CLIENT_ID".to_string()),
        }
    }
    
    /// 生成小红书登录二维码 URL
    async fn generate_qrcode_from_api(&self) -> Result<(String, String), PublishError> {
        let state = uuid::Uuid::new_v4().to_string();
        let redirect_uri = std::env::var("XIAOHONGSHU_REDIRECT_URI")
            .unwrap_or_else(|_| "https://www.xiaohongshu.com/oauth/callback".to_string());
        
        // 小红书 OAuth2 URL
        let auth_url = format!(
            "https://www.xiaohongshu.com/oauth2/login?client_id={}&redirect_uri={}&response_type=code&state={}&scope=login",
            self.client_id,
            urlencoding_encode(&redirect_uri),
            state
        );
        
        Ok((auth_url, state))
    }
}

#[async_trait]
impl QrLoginHandler for XiaohongshuQrLogin {
    fn platform(&self) -> Platform {
        Platform::Xiaohongshu
    }
    
    async fn generate_qrcode(&self, _http: &Client) -> Result<QrCodeInfo, PublishError> {
        let (url, state) = self.generate_qrcode_from_api().await?;
        
        let qr_id = format!("xiaohongshu_{}", state);
        
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
        let _ = qr_id;
        Ok(QrCodeStatus::Pending)
    }
    
    async fn confirm_login(&self, _http: &Client, qr_id: &str) -> Result<PlatformCredential, PublishError> {
        let _ = qr_id;
        Err(PublishError::PlatformError(
            "小红书扫码登录需要通过 OAuth 授权页面获取 code".to_string()
        ))
    }
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
