use self_media_crypto::{generate_salt, hash_password, verify_password, SystemKey, UserKey};
use sqlx::SqlitePool;

use crate::error::*;
use crate::user::model::*;

pub struct UserService {
    db: SqlitePool,
    system_key: SystemKey,
}

impl UserService {
    pub fn new(db: SqlitePool, system_key: SystemKey) -> Self {
        Self { db, system_key }
    }

    /// 注册
    pub async fn register(&self, req: &RegisterRequest) -> Result<(User, Session), AppError> {
        validate_username(&req.username)?;
        validate_password(&req.password)?;
        validate_email(&req.email)?;
        if let Some(ref phone) = req.phone {
            validate_phone(phone)?;
        }

        let existing: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE username = ?"
        )
        .bind(&req.username)
        .fetch_one(&self.db)
        .await?;
        if existing > 0 {
            return Err(AppError::auth(AUTH_001, "用户名已存在"));
        }

        let salt = generate_salt();
        let password_hash = hash_password(&req.password, &salt)?;
        let phone_value = req.phone.as_deref().unwrap_or("");
        // 加密 MiniMax API Key 后存储
        let encrypted_api_key = self.system_key.encrypt(&req.minimax_api_key)?;
        let user: User = sqlx::query_as(
            "INSERT INTO users (username, password_hash, salt, email, minimax_api_key, phone) VALUES (?, ?, ?, ?, ?, ?) RETURNING *"
        )
        .bind(&req.username)
        .bind(&password_hash)
        .bind(&salt)
        .bind(&req.email)
        .bind(&encrypted_api_key)
        .bind(phone_value)
        .fetch_one(&self.db)
        .await?;

        let session = self.create_session(user.id).await?;
        Ok((user, session))
    }

    /// 登录
    pub async fn login(&self, username: &str, password: &str) -> Result<(Session, UserKey), AppError> {
        let t0 = std::time::Instant::now();
        let user: Option<User> = sqlx::query_as(
            "SELECT * FROM users WHERE username = ?"
        )
        .bind(username)
        .fetch_optional(&self.db)
        .await?;
        tracing::debug!("login: user query took {:?}", t0.elapsed());

        let user = user.ok_or(AppError::auth(AUTH_002, "用户名或密码错误"))?;

        let t1 = std::time::Instant::now();
        if !verify_password(password, &user.password_hash, &user.salt)? {
            return Err(AppError::auth(AUTH_002, "用户名或密码错误"));
        }
        tracing::debug!("login: password verify took {:?}", t1.elapsed());

        let t2 = std::time::Instant::now();
        let user_key = UserKey::derive_from_password(password, &user.salt)?;
        tracing::debug!("login: user_key derive took {:?}", t2.elapsed());

        let t3 = std::time::Instant::now();
        let session = self.create_session(user.id).await?;
        tracing::debug!("login: create_session took {:?}", t3.elapsed());
        tracing::debug!("login: total took {:?}", t0.elapsed());
        Ok((session, user_key))
    }

    /// 密码修改（含重加密）
    pub async fn change_password(
        &self,
        user_id: i64,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        validate_password(new_password)?;

        let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_one(&self.db)
            .await?;

        if !verify_password(old_password, &user.password_hash, &user.salt)? {
            return Err(AppError::auth(AUTH_002, "旧密码错误"));
        }

        let old_user_key = UserKey::derive_from_password(old_password, &user.salt)?;
        let new_salt = generate_salt();
        let new_password_hash = hash_password(new_password, &new_salt)?;
        let new_user_key = UserKey::derive_from_password(new_password, &new_salt)?;

        self.reencrypt_platform_cookies(user_id, &old_user_key, &new_user_key).await?;

        sqlx::query(
            "UPDATE users SET password_hash = ?, salt = ?, updated_at = datetime('now') WHERE id = ?"
        )
        .bind(&new_password_hash)
        .bind(&new_salt)
        .bind(user_id)
        .execute(&self.db)
        .await?;

        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// 验证会话
    pub async fn validate_session(&self, token: &str) -> Result<User, AppError> {
        let session: Option<Session> = sqlx::query_as(
            "SELECT * FROM sessions WHERE token = ? AND expires_at > datetime('now')"
        )
        .bind(token)
        .fetch_optional(&self.db)
        .await?;

        let session = session.ok_or(AppError::auth(AUTH_003, "会话已过期"))?;

        let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(session.user_id)
            .fetch_one(&self.db)
            .await?;

        self.maybe_renew_session(&session).await?;
        Ok(user)
    }

    /// 登出
    pub async fn logout(&self, token: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(token)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    pub async fn get_user_minimax_key(&self, user_id: i64) -> Result<String, AppError> {
        let encrypted_api_key: Option<String> = sqlx::query_scalar(
            "SELECT minimax_api_key FROM users WHERE id = ?"
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        let encrypted = encrypted_api_key.ok_or(AppError::config(CONFIG_001, "用户不存在或API Key未设置"))?;
        // 解密 API Key 后返回
        self.system_key.decrypt(&encrypted)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("API Key 解密失败: {}", e)))
    }

    pub async fn get_user_model_config(&self, user_id: i64) -> Result<UserModelConfig, AppError> {
        let config: Option<UserModelConfig> = sqlx::query_as(
            "SELECT text_model, image_model, video_model, speech_model, music_model FROM users WHERE id = ?"
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        config.ok_or(AppError::config(CONFIG_001, "用户不存在"))
    }

    pub async fn update_user_model_config(&self, user_id: i64, config: &UserModelConfig) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE users SET text_model = ?, image_model = ?, video_model = ?, speech_model = ?, music_model = ? WHERE id = ?"
        )
        .bind(&config.text_model)
        .bind(&config.image_model)
        .bind(&config.video_model)
        .bind(&config.speech_model)
        .bind(&config.music_model)
        .bind(user_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    // ---- 内部方法 ----

    async fn create_session(&self, user_id: i64) -> Result<Session, AppError> {
        let token = self.system_key.generate_jwt(user_id)?;
        let session: Session = sqlx::query_as(
            "INSERT INTO sessions (user_id, token, expires_at) VALUES (?, ?, datetime('now', '+7 days')) RETURNING *"
        )
        .bind(user_id)
        .bind(&token)
        .fetch_one(&self.db)
        .await?;
        Ok(session)
    }

    async fn maybe_renew_session(&self, session: &Session) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE sessions SET expires_at = datetime('now', '+7 days') \
             WHERE id = ? AND expires_at < datetime('now', '+1 day')"
        )
        .bind(session.id)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    async fn reencrypt_platform_cookies(
        &self,
        user_id: i64,
        old_key: &UserKey,
        new_key: &UserKey,
    ) -> Result<(), AppError> {
        let rows: Vec<(i64, String)> = sqlx::query_as(
            "SELECT id, cookies FROM platform_configs WHERE user_id = ? AND cookies IS NOT NULL"
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        for (id, cookies) in rows {
            let plaintext = old_key.decrypt(&cookies)?;
            let new_encrypted = new_key.encrypt(&plaintext)?;
            sqlx::query("UPDATE platform_configs SET cookies = ? WHERE id = ?")
                .bind(&new_encrypted)
                .bind(id)
                .execute(&self.db)
                .await?;
        }
        Ok(())
    }
}

// ---- 输入校验 ----

fn validate_username(username: &str) -> Result<(), AppError> {
    if username.len() < 3 || username.len() > 32 {
        return Err(AppError::validation(AUTH_004, "用户名长度需在 3-32 之间"));
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(AppError::validation(AUTH_004, "用户名仅允许字母、数字、下划线"));
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), AppError> {
    if password.len() < 8 || password.len() > 64 {
        return Err(AppError::validation(AUTH_005, "密码长度需在 8-64 之间"));
    }
    // 检查密码复杂度：必须包含字母和数字
    let has_letter = password.chars().any(|c| c.is_alphabetic());
    let has_digit = password.chars().any(|c| c.is_numeric());
    if !has_letter || !has_digit {
        return Err(AppError::validation(AUTH_005, "密码必须包含字母和数字"));
    }
    Ok(())
}

fn validate_email(email: &str) -> Result<(), AppError> {
    if email.is_empty() {
        return Err(AppError::validation(INPUT_001, "邮箱不能为空"));
    }
    // 预编译邮箱正则表达式（避免每次调用时重新编译）
    static EMAIL_REGEX: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let email_regex = EMAIL_REGEX.get_or_init(|| {
        regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
            .expect("email regex is valid")
    });
    if !email_regex.is_match(email) {
        return Err(AppError::validation(INPUT_001, "邮箱格式不正确"));
    }
    Ok(())
}

fn validate_phone(phone: &str) -> Result<(), AppError> {
    if phone.len() != 11 {
        return Err(AppError::validation(INPUT_001, "手机号码必须为11位"));
    }
    if !phone.chars().all(|c| c.is_numeric()) {
        return Err(AppError::validation(INPUT_001, "手机号码只能包含数字"));
    }
    Ok(())
}
