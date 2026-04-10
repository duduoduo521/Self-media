use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, post, put},
    Json, Router,
};
use serde::Deserialize;

use self_media_core::user::model::{Session, User};

use crate::{ApiOk, AppState, AuthUser, WebError};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", delete(logout))
        .route("/password", put(change_password))
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应：设置 HttpOnly Cookie
struct LoginResponse {
    session: Session,
    token: String,
}

impl IntoResponse for LoginResponse {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        // 设置 HttpOnly Cookie（7天过期）
        let cookie = format!(
            "token={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=604800",
            self.token
        );
        headers.insert(
            axum::http::header::SET_COOKIE,
            HeaderValue::from_str(&cookie).unwrap(),
        );
        (
            StatusCode::OK,
            headers,
            Json(serde_json::json!({
                "code": "0",
                "message": "success",
                "data": { "session": self.session }
            })),
        )
            .into_response()
    }
}

/// 注册响应：设置 HttpOnly Cookie
struct RegisterResponseWrapper {
    user: User,
    session: Session,
    token: String,
}

impl IntoResponse for RegisterResponseWrapper {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        let cookie = format!(
            "token={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=604800",
            self.token
        );
        headers.insert(
            axum::http::header::SET_COOKIE,
            HeaderValue::from_str(&cookie).unwrap(),
        );
        (
            StatusCode::OK,
            headers,
            Json(serde_json::json!({
                "code": "0",
                "message": "success",
                "data": {
                    "user": self.user,
                    "session": self.session
                }
            })),
        )
            .into_response()
    }
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<RegisterResponseWrapper, WebError> {
    let (user, session) = state
        .user_service
        .register(&body.username, &body.password)
        .await?;
    // 注册后首次登录需要重新输入密码派生密钥
    let token = session.token.clone();
    Ok(RegisterResponseWrapper {
        user,
        session,
        token,
    })
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<LoginResponse, WebError> {
    let (session, user_key) = state.user_service.login(&body.username, &body.password).await?;
    // 缓存用户密钥，供后续 API Key 加密使用
    state.user_key_cache.insert(session.user_id, user_key).await;
    let token = session.token.clone();
    Ok(LoginResponse {
        session,
        token,
    })
}

/// 登出响应：清除 Cookie
struct LogoutResponse;

impl IntoResponse for LogoutResponse {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        // 清除 Cookie（设置过期时间为 0）
        let cookie = "token=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0";
        headers.insert(
            axum::http::header::SET_COOKIE,
            HeaderValue::from_str(cookie).unwrap(),
        );
        (StatusCode::OK, headers, Json(serde_json::json!({"code": "0", "message": "success"})))
            .into_response()
    }
}

async fn logout(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<LogoutResponse, WebError> {
    state.user_service.logout(&auth.token).await?;
    Ok(LogoutResponse)
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

async fn change_password(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<ApiOk<()>, WebError> {
    state
        .user_service
        .change_password(auth.user_id, &body.old_password, &body.new_password)
        .await?;
    state.user_key_cache.invalidate(auth.user_id).await;
    Ok(ApiOk(()))
}
