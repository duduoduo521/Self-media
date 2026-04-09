pub mod system_key;
pub mod user_key;
pub mod hash;
pub mod error;

pub use error::CryptoError;
pub use system_key::SystemKey;
pub use user_key::UserKey;
pub use hash::{generate_salt, hash_password, verify_password};

/// Argon2id 统一配置参数
pub const ARGON2_MEM_COST: u32 = 65536;   // 64 MB
pub const ARGON2_TIME_COST: u32 = 3;
pub const ARGON2_LANES: u32 = 4;
pub const ARGON2_HASH_LENGTH: usize = 32;
