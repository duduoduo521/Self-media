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
    pub async fn register(&self, username: &str, password: &str) -> Result<(User, Session), AppError> {
        validate_username(username)?;
        validate_password(password)?;

        let existing: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE username = ?"
        )
        .bind(username)
        .fetch_one(&self.db)
        .await?;
        if existing > 0 {
            return Err(AppError::auth(AUTH_001, "用户名已存在"));
        }

        let salt = generate_salt();
        let password_hash = hash_password(password, &salt)?;
        let user: User = sqlx::query_as(
            "INSERT INTO users (username, password_hash, salt) VALUES (?, ?, ?) RETURNING *"
        )
        .bind(username)
        .bind(&password_hash)
        .bind(&salt)
        .fetch_one(&self.db)
        .await?;

        let session = self.create_session(user.id).await?;
        Ok((user, session))
    }

    /// 登录
    pub async fn login(&self, username: &str, password: &str) -> Result<(Session, UserKey), AppError> {
        let user: Option<User> = sqlx::query_as(
            "SELECT * FROM users WHERE username = ?"
        )
        .bind(username)
        .fetch_optional(&self.db)
        .await?;

        let user = user.ok_or(AppError::auth(AUTH_002, "用户名或密码错误"))?;

        if !verify_password(password, &user.password_hash, &user.salt)? {
            return Err(AppError::auth(AUTH_002, "用户名或密码错误"));
        }

        // 派生用户密钥用于后续加密操作
        let user_key = UserKey::derive_from_password(password, &user.salt)?;
        let session = self.create_session(user.id).await?;
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

        self.reencrypt_api_keys(user_id, &old_user_key, &new_user_key).await?;
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

    async fn reencrypt_api_keys(
        &self,
        user_id: i64,
        old_key: &UserKey,
        new_key: &UserKey,
    ) -> Result<(), AppError> {
        let rows: Vec<(i64, String)> = sqlx::query_as(
            "SELECT id, encrypted_key FROM api_keys WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        for (id, encrypted_key) in rows {
            let plaintext = old_key.decrypt(&encrypted_key)?;
            let new_encrypted = new_key.encrypt(&plaintext)?;
            sqlx::query("UPDATE api_keys SET encrypted_key = ? WHERE id = ?")
                .bind(&new_encrypted)
                .bind(id)
                .execute(&self.db)
                .await?;
        }
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
    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());
    if !(has_upper && has_lower && has_digit) {
        return Err(AppError::validation(AUTH_005, "密码必须包含大小写字母和数字"));
    }
    Ok(())
}
