//! 扫码登录服务
//! 
//! 支持各平台的二维码登录流程

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time::sleep;

use self_media_core::types::{Platform, PlatformCredential};
use crate::publisher::PublishError;

/// 二维码状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QrCodeStatus {
    /// 等待扫码
    Pending,
    /// 已扫码，待确认
    Scanned,
    /// 已确认，登录成功
    Confirmed,
    /// 已过期
    Expired,
    /// 登录失败
    Failed,
}

/// 二维码信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrCodeInfo {
    /// 二维码 ID
    pub id: String,
    /// 二维码图片 URL 或 Base64
    pub image: String,
    /// 扫码链接
    pub url: String,
    /// 当前状态
    pub status: QrCodeStatus,
    /// 创建时间
    pub created_at: i64,
}

/// 扫码登录结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResult {
    /// 平台名称
    pub platform: Platform,
    /// 登录是否成功
    pub success: bool,
    /// 凭证信息
    pub credential: Option<PlatformCredential>,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 扫码登录处理器接口
#[async_trait]
pub trait QrLoginHandler: Send + Sync {
    /// 获取平台
    fn platform(&self) -> Platform;
    
    /// 生成二维码
    async fn generate_qrcode(&self, http: &Client) -> Result<QrCodeInfo, PublishError>;
    
    /// 查询二维码状态
    async fn query_status(&self, http: &Client, qr_id: &str) -> Result<QrCodeStatus, PublishError>;
    
    /// 确认登录并获取凭证
    async fn confirm_login(&self, http: &Client, qr_id: &str) -> Result<PlatformCredential, PublishError>;
}

/// 扫码登录管理器
pub struct QrLoginManager {
    http: Client,
    handlers: HashMap<Platform, Arc<dyn QrLoginHandler>>,
    /// 缓存的二维码状态（实际应存 Redis）
    qr_status: Arc<RwLock<HashMap<String, QrCodeInfo>>>,
}

impl QrLoginManager {
    pub fn new(http: Client) -> Self {
        Self {
            http,
            handlers: HashMap::new(),
            qr_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn register_handler<H: QrLoginHandler + 'static>(&mut self, handler: H) {
        self.handlers.insert(handler.platform(), Arc::new(handler));
    }
    
    /// 生成指定平台的二维码
    pub async fn generate_qrcode(&self, platform: Platform) -> Result<QrCodeInfo, PublishError> {
        let handler = self.handlers.get(&platform)
            .ok_or_else(|| PublishError::PlatformError(format!("平台 {:?} 不支持扫码登录", platform)))?;
        
        let qr_info = handler.generate_qrcode(&self.http).await?;
        
        // 缓存二维码状态
        {
            let mut status = self.qr_status.write().await;
            status.insert(qr_info.id.clone(), qr_info.clone());
        }
        
        Ok(qr_info)
    }
    
    /// 查询二维码状态
    pub async fn query_status(&self, platform: Platform, qr_id: &str) -> Result<QrCodeStatus, PublishError> {
        let handler = self.handlers.get(&platform)
            .ok_or_else(|| PublishError::PlatformError(format!("平台 {:?} 不支持扫码登录", platform)))?;
        
        let status = handler.query_status(&self.http, qr_id).await?;
        
        // 更新缓存
        {
            let mut cache = self.qr_status.write().await;
            if let Some(info) = cache.get_mut(qr_id) {
                info.status = status.clone();
            }
        }
        
        Ok(status)
    }
    
    /// 执行登录确认
    pub async fn confirm_login(&self, platform: Platform, qr_id: &str) -> Result<LoginResult, PublishError> {
        let handler = self.handlers.get(&platform)
            .ok_or_else(|| PublishError::PlatformError(format!("平台 {:?} 不支持扫码登录", platform)))?;
        
        match handler.confirm_login(&self.http, qr_id).await {
            Ok(credential) => {
                // 清理缓存
                {
                    let mut cache = self.qr_status.write().await;
                    cache.remove(qr_id);
                }
                Ok(LoginResult {
                    platform,
                    success: true,
                    credential: Some(credential),
                    error_message: None,
                })
            }
            Err(e) => {
                Ok(LoginResult {
                    platform,
                    success: false,
                    credential: None,
                    error_message: Some(e.to_string()),
                })
            }
        }
    }
    
    /// 轮询等待用户确认（阻塞直到登录成功或失败）
    pub async fn wait_for_confirmation(&self, platform: Platform, qr_id: &str, timeout_secs: u64) -> Result<LoginResult, PublishError> {
        let start = Instant::now();
        let poll_interval = Duration::from_secs(2);
        
        loop {
            if start.elapsed().as_secs() > timeout_secs {
                return Ok(LoginResult {
                    platform,
                    success: false,
                    credential: None,
                    error_message: Some("登录超时".to_string()),
                });
            }
            
            let status = self.query_status(platform.clone(), qr_id).await?;
            
            match status {
                QrCodeStatus::Confirmed => {
                    return self.confirm_login(platform.clone(), qr_id).await;
                }
                QrCodeStatus::Expired => {
                    return Ok(LoginResult {
                        platform: platform.clone(),
                        success: false,
                        credential: None,
                        error_message: Some("二维码已过期".to_string()),
                    });
                }
                QrCodeStatus::Failed => {
                    return Ok(LoginResult {
                        platform,
                        success: false,
                        credential: None,
                        error_message: Some("登录失败".to_string()),
                    });
                }
                _ => {
                    sleep(poll_interval).await;
                }
            }
        }
    }
}
