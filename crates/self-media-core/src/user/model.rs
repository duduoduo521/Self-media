use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub salt: String,
    pub email: Option<String>,
    pub minimax_api_key: Option<String>,
    pub phone: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default = "default_text_model")]
    pub text_model: String,
    #[serde(default = "default_image_model")]
    pub image_model: String,
    #[serde(default = "default_video_model")]
    pub video_model: String,
    #[serde(default = "default_speech_model")]
    pub speech_model: String,
    #[serde(default = "default_music_model")]
    pub music_model: String,
}

fn default_text_model() -> String { "MiniMax-M2.7".to_string() }
fn default_image_model() -> String { "image-01".to_string() }
fn default_video_model() -> String { "video-01".to_string() }
fn default_speech_model() -> String { "speech-02-hd".to_string() }
fn default_music_model() -> String { "music-01".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: i64,
    pub user_id: i64,
    pub token: String,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: String,
    pub minimax_api_key: String,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub user: UserInfo,
    pub session: Session,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserModelConfig {
    pub text_model: String,
    pub image_model: String,
    pub video_model: String,
    pub speech_model: String,
    pub music_model: String,
}
