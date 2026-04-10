use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("加密失败: {0}")]
    Encrypt(String),

    #[error("解密失败: {0}")]
    Decrypt(String),

    #[error("密钥派生失败: {0}")]
    KeyDerivation(String),

    #[error("密码哈希失败: {0}")]
    PasswordHash(String),

    #[error("JWT 错误: {0}")]
    Jwt(String),

    #[error("编码错误: {0}")]
    Encoding(String),

    #[error("格式错误: {0}")]
    Format(String),
}

impl From<aes_gcm::Error> for CryptoError {
    fn from(e: aes_gcm::Error) -> Self {
        CryptoError::Encrypt(e.to_string())
    }
}

impl From<aes_gcm::aes::cipher::InvalidLength> for CryptoError {
    fn from(e: aes_gcm::aes::cipher::InvalidLength) -> Self {
        CryptoError::Encrypt(format!("密钥长度无效: {}", e))
    }
}

impl From<base64::DecodeError> for CryptoError {
    fn from(e: base64::DecodeError) -> Self {
        CryptoError::Encoding(e.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for CryptoError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        CryptoError::Jwt(e.to_string())
    }
}

impl From<bcrypt::BcryptError> for CryptoError {
    fn from(e: bcrypt::BcryptError) -> Self {
        CryptoError::PasswordHash(e.to_string())
    }
}

impl From<std::string::FromUtf8Error> for CryptoError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        CryptoError::Encoding(format!("解密结果非有效 UTF-8: {}", e))
    }
}
