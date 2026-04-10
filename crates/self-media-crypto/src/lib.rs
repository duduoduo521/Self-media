pub mod system_key;
pub mod user_key;
pub mod hash;
pub mod error;

pub use error::CryptoError;
pub use system_key::SystemKey;
pub use user_key::UserKey;
pub use hash::{generate_salt, hash_password, verify_password};

pub const BCRYPT_COST: u32 = 10;
pub const BCRYPT_HASH_LENGTH: usize = 24;