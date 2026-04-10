use thiserror::Error;

use crate::types::ApiError;

/// 全局错误码常量
pub const AUTH_001: &str = "AUTH_001"; // 用户名已存在
pub const AUTH_002: &str = "AUTH_002"; // 密码错误
pub const AUTH_003: &str = "AUTH_003"; // 会话过期
pub const AUTH_004: &str = "AUTH_004"; // 用户名格式不合法
pub const AUTH_005: &str = "AUTH_005"; // 密码强度不足
pub const AUTH_006: &str = "AUTH_006"; // 资源访问被拒绝
pub const AI_001: &str = "AI_001";     // API Key 无效
pub const AI_002: &str = "AI_002";     // 生成超时
pub const AI_003: &str = "AI_003";     // 配额不足
pub const PLAT_001: &str = "PLAT_001"; // Cookie 过期
pub const PLAT_002: &str = "PLAT_002"; // 上传失败
pub const PLAT_003: &str = "PLAT_003"; // 发布限流
pub const PLAT_004: &str = "PLAT_004"; // 签名验证失败
pub const TASK_001: &str = "TASK_001"; // 任务不存在
pub const TASK_002: &str = "TASK_002"; // 任务已取消
pub const TASK_003: &str = "TASK_003"; // 并发任务数超限
pub const CONFIG_001: &str = "CONFIG_001"; // 配置项不存在
pub const CRYPTO_001: &str = "CRYPTO_001"; // 加解密失败
pub const INPUT_001: &str = "INPUT_001"; // 输入参数校验失败

#[derive(Debug, Error)]
pub enum AppError {
    #[error("认证错误: {message}")]
    Auth { code: &'static str, message: String },

    #[error("AI 错误: {message}")]
    Ai { code: &'static str, message: String },

    #[error("平台错误: {message}")]
    Platform { code: &'static str, message: String },

    #[error("任务错误: {message}")]
    Task { code: &'static str, message: String },

    #[error("配置错误: {message}")]
    Config { code: &'static str, message: String },

    #[error("加密错误: {message}")]
    Crypto { code: &'static str, message: String },

    #[error("校验错误: {message}")]
    Validation { code: &'static str, message: String },

    #[error("数据库错误: {0}")]
    Db(#[from] self_media_db::DbError),

    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn auth(code: &'static str, message: impl Into<String>) -> Self {
        Self::Auth { code, message: message.into() }
    }
    pub fn ai(code: &'static str, message: impl Into<String>) -> Self {
        Self::Ai { code, message: message.into() }
    }
    pub fn platform(code: &'static str, message: impl Into<String>) -> Self {
        Self::Platform { code, message: message.into() }
    }
    pub fn task(code: &'static str, message: impl Into<String>) -> Self {
        Self::Task { code, message: message.into() }
    }
    pub fn config(code: &'static str, message: impl Into<String>) -> Self {
        Self::Config { code, message: message.into() }
    }
    pub fn crypto(code: &'static str, message: impl Into<String>) -> Self {
        Self::Crypto { code, message: message.into() }
    }
    pub fn validation(code: &'static str, message: impl Into<String>) -> Self {
        Self::Validation { code, message: message.into() }
    }

    /// 获取错误码
    pub fn code(&self) -> &str {
        match self {
            Self::Auth { code, .. } => code,
            Self::Ai { code, .. } => code,
            Self::Platform { code, .. } => code,
            Self::Task { code, .. } => code,
            Self::Config { code, .. } => code,
            Self::Crypto { code, .. } => code,
            Self::Validation { code, .. } => code,
            Self::Db(_) => "DB_001",
            Self::Internal(_) => "INTERNAL_001",
        }
    }

    /// 转换为 API 错误响应
    pub fn to_api_error(&self) -> ApiError {
        ApiError::new(self.code(), &self.to_string())
    }
}

impl From<self_media_crypto::CryptoError> for AppError {
    fn from(e: self_media_crypto::CryptoError) -> Self {
        AppError::crypto(CRYPTO_001, e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Internal(e.into())
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Db(self_media_db::DbError::from(e))
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Internal(e.into())
    }
}
