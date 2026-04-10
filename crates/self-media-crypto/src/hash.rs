use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::rngs::OsRng;
use rand::RngCore;
use subtle::ConstantTimeEq;

use crate::{ARGON2_HASH_LENGTH, ARGON2_LANES, ARGON2_MEM_COST, ARGON2_TIME_COST, CryptoError};

/// 生成随机 Salt
pub fn generate_salt() -> String {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    BASE64.encode(salt)
}

/// 密码哈希（使用 Argon2id）
pub fn hash_password(password: &str, salt_b64: &str) -> Result<String, CryptoError> {
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

    let mut hash = [0u8; 32];
    argon2.hash_password_into(password.as_bytes(), &salt, &mut hash)?;

    Ok(BASE64.encode(hash))
}

/// 密码验证（恒定时间比较，防时序攻击）
pub fn verify_password(password: &str, hash_b64: &str, salt_b64: &str) -> Result<bool, CryptoError> {
    let expected = BASE64.decode(hash_b64)?;
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

    let mut actual = [0u8; 32];
    argon2.hash_password_into(password.as_bytes(), &salt, &mut actual)?;

    // 恒定时间比较，防止时序攻击
    Ok(expected.ct_eq(&actual).into())
}
