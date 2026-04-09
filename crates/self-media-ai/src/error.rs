use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("API Key 无效: {0}")]
    InvalidApiKey(String),

    #[error("生成超时: {0}")]
    Timeout(String),

    #[error("配额不足: {0}")]
    QuotaExceeded(String),

    #[error("API 错误: {0}")]
    ApiError(String),

    #[error("网络错误: {0}")]
    Network(String),

    #[error("解析错误: {0}")]
    Parse(String),
}

impl From<reqwest::Error> for AiError {
    fn from(e: reqwest::Error) -> Self {
        AiError::Network(e.to_string())
    }
}
