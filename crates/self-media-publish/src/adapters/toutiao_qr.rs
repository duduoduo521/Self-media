//! 头条扫码登录处理器
//! 
//! 头条使用 OAuth2 扫码登录流程
//! API 端点: oembed.toutiao.com

use async_trait::async_trait;
use reqwest::Client;

use self_media_core::types::{Platform, PlatformCredential};
use crate::publisher::PublishError;
use crate::qr_login::{QrCodeInfo, QrCodeStatus, QrLoginHandler};

/// 头条扫码登录处理器
pub struct ToutiaoQrLogin {
    #[allow(dead_code)]
    http: Client,
    /// 客户端 ID
    client_key: String,
}

impl ToutiaoQrLogin {
    pub fn new(http: Client) -> Self {
        Self {
            http,
            client_key: std::env::var("TOUTIAO_CLIENT_KEY")
                .unwrap_or_else(|_| "YOUR_TOUTIAO_CLIENT_KEY".to_string()),
        }
    }
    
    /// 生成头条登录二维码
    async fn generate_qrcode_from_api(&self) -> Result<(String, String), PublishError> {
        // 头条 OAuth2 扫码登录 URL
        let state = uuid::Uuid::new_v4().to_string();
        let redirect_uri = std::env::var("TOUTIAO_REDIRECT_URI")
            .unwrap_or_else(|_| "https://api.toutiao.com/oauth/callback".to_string());
        
        let auth_url = format!(
            "https://open.toutiao.com/oauth/authorize?client_key={}&response_type=code&redirect_uri={}&state={}",
            self.client_key,
            urlencoding_encode(&redirect_uri),
            state
        );
        
        Ok((auth_url, state))
    }
}

#[async_trait]
impl QrLoginHandler for ToutiaoQrLogin {
    fn platform(&self) -> Platform {
        Platform::Toutiao
    }
    
    async fn generate_qrcode(&self, _http: &Client) -> Result<QrCodeInfo, PublishError> {
        let (url, state) = self.generate_qrcode_from_api().await?;
        
        let qr_id = format!("toutiao_{}", state);
        
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
        // 头条 OAuth 需要回调，无法本地查询状态
        // 返回 Pending，等待用户授权
        let _ = qr_id;
        Ok(QrCodeStatus::Pending)
    }
    
    async fn confirm_login(&self, _http: &Client, _qr_id: &str) -> Result<PlatformCredential, PublishError> {
        // 头条扫码登录实际上是通过 OAuth 回调 URL 携带 code 参数
        // 前端需要打开授权页面，让用户授权后获取 callback URL 中的 code
        // 然后调用后端 /api/qr/confirm 接口，传入 code
        
        // 这里简化处理：qr_id 格式 toutiao_{state}，但实际 code 需要前端传递
        // 建议前端使用 OAuth 授权页面直接获取 code
        
        Err(PublishError::PlatformError(
            "头条扫码登录需要通过 OAuth 授权页面获取 code，请使用 /api/qr/authorize 接口".to_string()
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
