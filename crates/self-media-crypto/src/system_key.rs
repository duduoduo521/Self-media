use aes_gcm::aead::Aead;
use aes_gcm::{AeadCore, Aes256Gcm, KeyInit, Nonce};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use rand::RngCore;
use serde_json::Value;

use crate::CryptoError;

/// 系统密钥：用于 JWT 签名/验证
/// 随机生成，加密持久化，应用级别生命周期
#[derive(Clone)]
pub struct SystemKey {
    key_bytes: [u8; 32],
    signing_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl SystemKey {
    /// 生成新的系统密钥
    pub fn generate() -> Self {
        let mut key_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut key_bytes);
        Self::from_bytes(&key_bytes)
    }

    /// 从原始字节构建
    fn from_bytes(key_bytes: &[u8; 32]) -> Self {
        Self {
            key_bytes: *key_bytes,
            signing_key: EncodingKey::from_secret(key_bytes),
            decoding_key: DecodingKey::from_secret(key_bytes),
        }
    }

    /// 从加密存储加载系统密钥
    pub fn load(encrypted: &str, machine_key: &[u8]) -> Result<Self, CryptoError> {
        let combined = BASE64.decode(encrypted)?;
        if combined.len() < 12 {
            return Err(CryptoError::Format("系统密钥密文格式错误".into()));
        }
        let nonce = Nonce::from_slice(&combined[..12]);
        let ciphertext = &combined[12..];
        let cipher = Aes256Gcm::new_from_slice(machine_key)
            .map_err(|e| CryptoError::Decrypt(format!("机器密钥长度错误: {}", e)))?;
        let decrypted = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::Decrypt("系统密钥解密失败".into()))?;
        if decrypted.len() != 32 {
            return Err(CryptoError::Format("系统密钥长度错误".into()));
        }
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&decrypted);
        Ok(Self::from_bytes(&key_bytes))
    }

    /// 加密持久化系统密钥
    pub fn save(&self, machine_key: &[u8]) -> Result<String, CryptoError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let cipher = Aes256Gcm::new_from_slice(machine_key)
            .map_err(|e| CryptoError::Encrypt(format!("机器密钥长度错误: {}", e)))?;
        let ciphertext = cipher.encrypt(&nonce, self.key_bytes.as_slice())?;
        let mut combined = nonce.to_vec();
        combined.extend_from_slice(&ciphertext);
        Ok(BASE64.encode(&combined))
    }

    /// 签发 JWT
    pub fn generate_jwt(&self, user_id: i64) -> Result<String, CryptoError> {
        let now = chrono::Utc::now();
        let expiration = now + chrono::Duration::days(7);
        let claims = serde_json::json!({
            "sub": user_id,
            "exp": expiration.timestamp(),
            "iat": now.timestamp(),
        });
        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.signing_key)?;
        Ok(token)
    }

    /// 验证 JWT，返回 user_id
    pub fn verify_jwt(&self, token: &str) -> Result<i64, CryptoError> {
        let data = decode::<Value>(
            token,
            &self.decoding_key,
            &Validation::new(Algorithm::HS256),
        )?;
        let user_id = data.claims["sub"]
            .as_i64()
            .ok_or(CryptoError::Jwt("无效的 JWT: 缺少 sub 字段".into()))?;
        Ok(user_id)
    }

    /// 加密敏感数据（如 API Key）
    /// 使用 AES-256-GCM 加密，返回 Base64 编码的密文
    pub fn encrypt(&self, plaintext: &str) -> Result<String, CryptoError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key_bytes)
            .map_err(|e| CryptoError::Encrypt(format!("加密失败: {}", e)))?;
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes())?;
        let mut combined = nonce.to_vec();
        combined.extend_from_slice(&ciphertext);
        Ok(BASE64.encode(&combined))
    }

    /// 解密敏感数据（如 API Key）
    /// 使用 AES-256-GCM 解密 Base64 编码的密文
    pub fn decrypt(&self, encrypted: &str) -> Result<String, CryptoError> {
        let combined = BASE64.decode(encrypted)?;
        if combined.len() < 12 {
            return Err(CryptoError::Format("密文格式错误".into()));
        }
        let nonce = Nonce::from_slice(&combined[..12]);
        let ciphertext = &combined[12..];
        let cipher = Aes256Gcm::new_from_slice(&self.key_bytes)
            .map_err(|e| CryptoError::Decrypt(format!("解密失败: {}", e)))?;
        let decrypted = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::Decrypt("解密失败".into()))?;
        String::from_utf8(decrypted).map_err(|_| CryptoError::Format("解密结果不是有效UTF8".into()))
    }
}

impl Drop for SystemKey {
    fn drop(&mut self) {
        self.key_bytes.fill(0);
    }
}
