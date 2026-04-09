use axum::{
    extract::State,
    routing::{delete, post, put},
    Json, Router,
};
use serde::Deserialize;

use self_media_core::user::model::{Session, User};
use self_media_core::user::UserService;

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

#[derive(serde::Serialize)]
pub struct RegisterResponse {
    pub user: User,
    pub session: Session,
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<ApiOk<RegisterResponse>, WebError> {
    let (user, session) = state
        .user_service
        .register(&body.username, &body.password)
        .await?;
    Ok(ApiOk(RegisterResponse { user, session }))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<ApiOk<Session>, WebError> {
    let session = state.user_service.login(&body.username, &body.password).await?;
    Ok(ApiOk(session))
}

async fn logout(
    auth: AuthUser,
    State(_state): State<AppState>,
) -> Result<ApiOk<()>, WebError> {
    Ok(ApiOk(()))
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
