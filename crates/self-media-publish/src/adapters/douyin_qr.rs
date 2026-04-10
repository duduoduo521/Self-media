//! 抖音扫码登录处理器
//! 
//! 抖音使用 OAuth2 扫码登录流程
//! API 端点: open.douyin.com

use async_trait::async_trait;
use reqwest::Client;

use self_media_core::types::{Platform, PlatformCredential};
use crate::publisher::PublishError;
use crate::qr_login::{QrCodeInfo, QrCodeStatus, QrLoginHandler};

/// 抖音扫码登录处理器
pub struct DouyinQrLogin {
    http: Client,
    /// 客户端 Key
    client_key: String,
}

impl DouyinQrLogin {
    pub fn new(http: Client) -> Self {
        Self {
            http,
            client_key: std::env::var("DOUYIN_CLIENT_KEY")
                .unwrap_or_else(|_| "YOUR_DOUYIN_CLIENT_KEY".to_string()),
        }
    }
    
    /// 生成抖音登录二维码 URL
    async fn generate_qrcode_from_api(&self) -> Result<(String, String), PublishError> {
        // 抖音 OAuth2 扫码登录 URL
        let state = uuid::Uuid::new_v4().to_string();
        let redirect_uri = std::env::var("DOUYIN_REDIRECT_URI")
            .unwrap_or_else(|_| "https://open.douyin.com/connect/qrcode/auth".to_string());
        
        // 抖音需要构造特定的扫码 URL
        let auth_url = format!(
            "https://open.douyin.com/connect/qrcode/auth?client_key={}&response_type=code&redirect_uri={}&scope=user_info&state={}",
            self.client_key,
            urlencoding_encode(&redirect_uri),
            state
        );
        
        Ok((auth_url, state))
    }
}

#[async_trait]
impl QrLoginHandler for DouyinQrLogin {
    fn platform(&self) -> Platform {
        Platform::Douyin
    }
    
    async fn generate_qrcode(&self, _http: &Client) -> Result<QrCodeInfo, PublishError> {
        let (url, state) = self.generate_qrcode_from_api().await?;
        
        let qr_id = format!("douyin_{}", state);
        
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
        // 抖音 OAuth 需要回调
        let _ = qr_id;
        Ok(QrCodeStatus::Pending)
    }
    
    async fn confirm_login(&self, _http: &Client, qr_id: &str) -> Result<PlatformCredential, PublishError> {
        // 抖音扫码登录需要通过 OAuth 回调获取 code
        let _ = qr_id;
        Err(PublishError::PlatformError(
            "抖音扫码登录需要通过 OAuth 授权页面获取 code".to_string()
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
