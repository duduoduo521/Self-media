use aes_gcm::aead::Aead;
use aes_gcm::{AeadCore, Aes256Gcm, KeyInit, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::rngs::OsRng;

use crate::CryptoError;

#[derive(Clone)]
pub struct UserKey {
    key: [u8; 32],
}

impl UserKey {
    pub fn derive_from_password(password: &str, salt_b64: &str) -> Result<Self, CryptoError> {
        let salt = BASE64.decode(salt_b64)?;
        if salt.len() < 8 {
            return Err(CryptoError::Format("盐值长度不足，至少需要8字节".into()));
        }

        let params = Params::new(
            65536,
            3,
            4,
            Some(32)
        ).map_err(|e| CryptoError::Format(format!("Argon2参数错误: {}", e)))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        let mut key = [0u8; 32];
        argon2
            .hash_password_into(password.as_bytes(), &salt, &mut key)
            .map_err(|e| CryptoError::Format(format!("密钥派生失败: {}", e)))?;

        Ok(Self { key })
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, CryptoError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let cipher = Aes256Gcm::new_from_slice(&self.key)?;
        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| CryptoError::Encrypt(format!("加密失败: {}", e)))?;
        let mut combined = nonce.to_vec();
        combined.extend_from_slice(&ciphertext);
        Ok(BASE64.encode(&combined))
    }

    pub fn decrypt(&self, encrypted: &str) -> Result<String, CryptoError> {
        let combined = BASE64.decode(encrypted)?;
        if combined.len() < 12 {
            return Err(CryptoError::Format("密文格式错误".into()));
        }
        let nonce = Nonce::from_slice(&combined[..12]);
        let ciphertext = &combined[12..];
        let cipher = Aes256Gcm::new_from_slice(&self.key)?;
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| CryptoError::Decrypt(format!("解密失败: {}", e)))?;
        String::from_utf8(plaintext)
            .map_err(|e| CryptoError::Format(format!("解密后数据不是有效字符串: {}", e)))
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.key
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { key: bytes }
    }
}