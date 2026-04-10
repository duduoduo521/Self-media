//! B站扫码登录处理器
//! 
//! B站使用 OAuth 扫码登录流程
//! API 端点: passport.bilibili.com

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use self_media_core::types::{Platform, PlatformCredential};
use crate::publisher::PublishError;
use crate::qr_login::{QrCodeInfo, QrCodeStatus, QrLoginHandler};

/// B站二维码状态响应
#[derive(Debug, Deserialize)]
struct BilibiliQrStatusResponse {
    /// 状态: 0=未扫码, 86101=等待扫码, 86090=已扫码待确认, 86038=已确认, -4=已过期
    #[serde(rename = "status")]
    status: i32,
    /// 登录成功时的用户信息
    #[serde(rename = "url")]
    url: Option<String>,
    /// 错误信息
    #[serde(rename = "message")]
    message: Option<String>,
    /// 跳转 URL（包含登录信息）
    #[serde(rename = "redirect_url")]
    redirect_url: Option<String>,
}

/// B站扫码登录处理器
pub struct BilibiliQrLogin {
    http: Client,
}

impl BilibiliQrLogin {
    pub fn new(http: Client) -> Self {
        Self { http }
    }
    
    /// 生成二维码
    async fn generate_qrcode_from_api(&self) -> Result<(String, String), PublishError> {
        let url = "https://passport.bilibili.com/qrcode/web/generate";
        
        let resp = self.http
            .get(url)
            .header("Referer", "https://passport.bilibili.com/")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await?;
        
        let body = resp.text().await?;
        
        // B站返回的是 JavaScript 格式，需要解析
        // 格式: urlENcode({...})
        let json_str = extract_json_from_js_callback(&body)?;
        
        #[derive(Deserialize)]
        struct QrGenerateResponse {
            #[serde(rename = "oauthKey")]
            oauth_key: Option<String>,
            url: Option<String>,
            #[serde(rename = "qrcode_key")]
            qrcode_key: Option<String>,
        }
        
        let resp_data: QrGenerateResponse = serde_json::from_str(&json_str)
            .map_err(|e| PublishError::ParseError(format!("B站二维码生成响应解析失败: {} - {}", e, body)))?;
        
        let oauth_key = resp_data.oauth_key
            .or(resp_data.qrcode_key)
            .ok_or_else(|| PublishError::PlatformError("B站未返回 oauthKey".to_string()))?;
        
        let url = resp_data.url
            .ok_or_else(|| PublishError::PlatformError("B站未返回二维码 URL".to_string()))?;
        
        Ok((url, oauth_key))
    }
    
    /// 查询二维码状态
    async fn query_qr_status(&self, oauth_key: &str) -> Result<BilibiliQrStatusResponse, PublishError> {
        let url = "https://passport.bilibili.com/qrcode/web/query";
        
        let resp = self.http
            .post(url)
            .form(&[("oauthKey", oauth_key)])
            .header("Referer", "https://passport.bilibili.com/")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await?;
        
        let body = resp.text().await?;
        
        serde_json::from_str(&body)
            .map_err(|e| PublishError::ParseError(format!("B站状态查询响应解析失败: {} - {}", e, body)))
    }
    
    /// 从跳转 URL 中提取登录信息并构建凭证
    async fn extract_credentials_from_url(&self, url: &str) -> Result<PlatformCredential, PublishError> {
        let resp = self.http
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Referer", "https://passport.bilibili.com/")
            .send()
            .await?;
        
        // 提取 Set-Cookie
        let mut cookies = String::new();
        for header in resp.headers().get_all("set-cookie") {
            if let Ok(cookie_str) = header.to_str() {
                if let Some(pos) = cookie_str.find(';') {
                    let cookie_part = &cookie_str[..pos];
                    if !cookies.is_empty() {
                        cookies.push_str("; ");
                    }
                    cookies.push_str(cookie_part);
                }
            }
        }
        
        if cookies.is_empty() {
            return Err(PublishError::PlatformError("B站登录后未返回 Cookie".to_string()));
        }
        
        // 提取关键 cookie 值用于存储
        let buvid3 = extract_cookie_value(&cookies, "buvid3");
        let sessdata = extract_cookie_value(&cookies, "SESSDATA");
        let bili_jct = extract_cookie_value(&cookies, "bili_jct");
        
        let mut extra = std::collections::HashMap::new();
        if let Some(v) = buvid3 {
            extra.insert("buvid3".to_string(), v);
        }
        if let Some(v) = sessdata {
            extra.insert("SESSDATA".to_string(), v);
        }
        if let Some(v) = bili_jct {
            extra.insert("bili_jct".to_string(), v);
        }
        extra.insert("platform".to_string(), "bilibili".to_string());
        
        Ok(PlatformCredential {
            platform: Platform::Bilibili,
            cookies,
            extra,
        })
    }
}

#[async_trait]
impl QrLoginHandler for BilibiliQrLogin {
    fn platform(&self) -> Platform {
        Platform::Bilibili
    }
    
    async fn generate_qrcode(&self, _http: &Client) -> Result<QrCodeInfo, PublishError> {
        let (url, oauth_key) = self.generate_qrcode_from_api().await?;
        
        let qr_id = format!("bilibili_{}", oauth_key);
        
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
        let oauth_key = qr_id.strip_prefix("bilibili_")
            .ok_or_else(|| PublishError::PlatformError("无效的二维码 ID".to_string()))?;
        
        let response = self.query_qr_status(oauth_key).await?;
        
        match response.status {
            0 | 86101 => Ok(QrCodeStatus::Pending),  // 等待扫码
            86090 => Ok(QrCodeStatus::Scanned),      // 已扫码待确认
            86038 => Ok(QrCodeStatus::Confirmed),    // 已确认登录成功
            -4 => Ok(QrCodeStatus::Expired),        // 已过期
            _ => Ok(QrCodeStatus::Failed),
        }
    }
    
    async fn confirm_login(&self, _http: &Client, qr_id: &str) -> Result<PlatformCredential, PublishError> {
        let oauth_key = qr_id.strip_prefix("bilibili_")
            .ok_or_else(|| PublishError::PlatformError("无效的二维码 ID".to_string()))?;
        
        let response = self.query_qr_status(oauth_key).await?;
        
        if response.status != 86038 {
            return Err(PublishError::PlatformError("用户尚未确认登录".to_string()));
        }
        
        let redirect_url = response.redirect_url
            .or(response.url)
            .ok_or_else(|| PublishError::PlatformError("B站未返回跳转 URL".to_string()))?;
        
        self.extract_credentials_from_url(&redirect_url).await
    }
}

/// 从 JavaScript 回调格式中提取 JSON
fn extract_json_from_js_callback(body: &str) -> Result<String, PublishError> {
    if let Some(start) = body.find('(') {
        if let Some(end) = body.rfind(')') {
            return Ok(body[start + 1..end].to_string());
        }
    }
    if body.starts_with('{') {
        return Ok(body.to_string());
    }
    Err(PublishError::ParseError(format!("无法解析响应: {}", body)))
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

/// 从 cookie 字符串中提取指定名称的值
fn extract_cookie_value(cookies: &str, name: &str) -> Option<String> {
    for cookie in cookies.split("; ") {
        if let Some(pos) = cookie.find('=') {
            let key = &cookie[..pos];
            let value = &cookie[pos + 1..];
            if key == name {
                return Some(value.to_string());
            }
        }
    }
    None
}
