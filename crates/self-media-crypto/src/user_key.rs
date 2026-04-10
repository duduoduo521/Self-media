use aes_gcm::aead::Aead;
use aes_gcm::{AeadCore, Aes256Gcm, KeyInit, Nonce};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::rngs::OsRng;
use sha2::{Sha256, Digest};

use crate::CryptoError;

#[derive(Clone)]
pub struct UserKey {
    key: [u8; 32],
}

impl UserKey {
    pub fn derive_from_password(password: &str, salt_b64: &str) -> Result<Self, CryptoError> {
        let salt = BASE64.decode(salt_b64)?;

        let mut key = [0u8; 32];
        pbkdf2_simple(password.as_bytes(), &salt, &mut key);

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
}

fn pbkdf2_simple(password: &[u8], salt: &[u8], output: &mut [u8]) {
    let mut hasher = Sha256::new();
    hasher.update(password);
    hasher.update(salt);
    let mut result = hasher.finalize();
    output[..32].copy_from_slice(&result);

    for i in 1..1000 {
        hasher = Sha256::new();
        hasher.update(&result);
        hasher.update(&(i as u32).to_le_bytes());
        result = hasher.finalize();
        for (j, byte) in result.iter().enumerate() {
            output[j] ^= byte;
        }
    }
}