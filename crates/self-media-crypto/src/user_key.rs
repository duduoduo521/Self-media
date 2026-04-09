use aes_gcm::aead::Aead;
use aes_gcm::{AeadCore, Aes256Gcm, KeyInit, Nonce};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::rngs::OsRng;

use crate::{ARGON2_HASH_LENGTH, ARGON2_LANES, ARGON2_MEM_COST, ARGON2_TIME_COST, CryptoError};

/// 用户密钥：用于加密/解密用户敏感数据
/// 由用户密码 + Salt 派生，仅存在于内存
#[derive(Clone)]
pub struct UserKey {
    key: [u8; 32],
}

impl UserKey {
    /// 从用户密码和 Salt 派生密钥
    pub fn derive_from_password(password: &str, salt_b64: &str) -> Result<Self, CryptoError> {
        let salt = BASE64.decode(salt_b64)?;

        let params = argon2::Params::new(
            ARGON2_MEM_COST,
            ARGON2_TIME_COST,
            ARGON2_LANES,
            Some(ARGON2_HASH_LENGTH),
        )?;

        let argon2 = argon2::Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            params,
        );

        let mut key = [0u8; 32];
        argon2
            .hash_password_into(password.as_bytes(), &salt, &mut key)
            .map_err(|e| CryptoError::KeyDerivation(format!("密钥派生失败: {}", e)))?;

        Ok(Self { key })
    }

    /// AES-256-GCM 加密
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

    /// AES-256-GCM 解密
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
            .map_err(|_| CryptoError::Decrypt("解密失败，密钥可能已变更".into()))?;
        Ok(String::from_utf8(plaintext)
            .map_err(|_| CryptoError::Encoding("解密结果非有效 UTF-8".into()))?)
    }

    /// 零化密钥内存
    pub fn zeroize(&mut self) {
        self.key.fill(0);
    }
}

impl Drop for UserKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}
