use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, post, put},
    Json, Router,
};

use self_media_core::error::AUTH_006;
use self_media_core::user::model::{RegisterRequest as CoreRegisterRequest, RegisterResponse, Session, UserInfo};
use crate::AppError;
use serde::Deserialize;

use crate::{ApiOk, AppState, AuthUser, WebError};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", delete(logout))
        .route("/password", put(change_password))
}

struct LoginResponse {
    session: Session,
    token: String,
}

impl IntoResponse for LoginResponse {
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
                    "session": {
                        "id": self.session.id,
                        "user_id": self.session.user_id,
                        "token": self.token,
                        "expires_at": self.session.expires_at
                    }
                }
            })),
        )
            .into_response()
    }
}

struct RegisterResponseWrapper {
    user: UserInfo,
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
                "data": RegisterResponse {
                    user: self.user,
                    session: self.session,
                }
            })),
        )
            .into_response()
    }
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<CoreRegisterRequest>,
) -> Result<RegisterResponseWrapper, WebError> {
    let client = self_media_ai::MiniMaxClient::new(
        body.minimax_api_key.clone(),
        "https://api.minimax.chat".to_string(),
    );
    client.validate_api_key().await
        .map_err(|e| WebError(AppError::auth(AUTH_006, format!("MiniMax API Key 无效: {}", e))))?;

    let core_req = CoreRegisterRequest {
        username: body.username,
        password: body.password,
        email: body.email,
        minimax_api_key: body.minimax_api_key,
        phone: body.phone,
    };
    let (user, session) = state.user_service.register(&core_req).await?;
    let token = session.token.clone();
    let user_info = UserInfo {
        id: user.id,
        username: user.username,
        email: user.email,
        phone: user.phone,
        created_at: user.created_at,
    };
    Ok(RegisterResponseWrapper {
        user: user_info,
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
