use crate::CryptoError;

const BCRYPT_COST: u32 = 10;

pub fn generate_salt() -> String {
    use rand::rngs::OsRng;
    use rand::RngCore;
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    BASE64.encode(salt)
}

pub fn hash_password(password: &str, _salt_b64: &str) -> Result<String, CryptoError> {
    bcrypt::hash(password, BCRYPT_COST).map_err(|e| CryptoError::PasswordHash(e.to_string()))
}

pub fn verify_password(password: &str, hash: &str, _salt_b64: &str) -> Result<bool, CryptoError> {
    bcrypt::verify(password, hash).map_err(|e| CryptoError::PasswordHash(e.to_string()))
}