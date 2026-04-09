use std::sync::Arc;

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{StatusCode, request::Parts},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Serialize;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use self_media_core::config::ConfigService;
use self_media_core::config::service::UserKeyCache;
use self_media_core::error::*;
use self_media_core::hotspot::HotspotService;
use self_media_core::task::TaskScheduler;
use self_media_core::types::ApiError;
use self_media_core::user::UserService;
use self_media_crypto::SystemKey;
use self_media_db::create_pool;
use self_media_publish::PublisherRegistry;

mod auth;
mod config;
mod hotspot;
mod task;

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub system_key: Arc<SystemKey>,
    pub user_service: Arc<UserService>,
    pub hotspot_service: Arc<tokio::sync::Mutex<HotspotService>>,
    pub task_scheduler: Arc<tokio::sync::Mutex<TaskScheduler>>,
    pub config_service: Arc<ConfigService>,
    pub publisher_registry: Arc<std::sync::Mutex<PublisherRegistry>>,
    pub user_key_cache: Arc<UserKeyCache>,
}

/// 已认证用户（由中间件注入到 request extensions）
#[derive(Clone, Copy, Debug)]
pub struct AuthUser {
    pub user_id: i64,
}

/// 从 request extensions 中提取 AuthUser
impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = (StatusCode, Json<ApiError>);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthUser>()
            .copied()
            .ok_or((
                StatusCode::UNAUTHORIZED,
                Json(ApiError::new(AUTH_003, "未登录")),
            ))
    }
}

/// Web 层错误包装器
pub struct WebError(pub AppError);

impl From<AppError> for WebError {
    fn from(e: AppError) -> Self {
        WebError(e)
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let (status, api_error) = match &self.0 {
            AppError::Auth { code, .. } => match *code {
                AUTH_001 => (StatusCode::CONFLICT, self.0.to_api_error()),
                AUTH_002 | AUTH_003 => (StatusCode::UNAUTHORIZED, self.0.to_api_error()),
                AUTH_004 | AUTH_005 => (StatusCode::BAD_REQUEST, self.0.to_api_error()),
                _ => (StatusCode::UNAUTHORIZED, self.0.to_api_error()),
            },
            AppError::Validation { .. } => (StatusCode::BAD_REQUEST, self.0.to_api_error()),
            AppError::Ai { code, .. } => match *code {
                AI_003 => (StatusCode::TOO_MANY_REQUESTS, self.0.to_api_error()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_api_error()),
            },
            AppError::Task { code, .. } => match *code {
                TASK_003 => (StatusCode::TOO_MANY_REQUESTS, self.0.to_api_error()),
                _ => (StatusCode::BAD_REQUEST, self.0.to_api_error()),
            },
            AppError::Platform { code, .. } => match *code {
                PLAT_003 => (StatusCode::TOO_MANY_REQUESTS, self.0.to_api_error()),
                _ => (StatusCode::BAD_REQUEST, self.0.to_api_error()),
            },
            AppError::Config { .. } => (StatusCode::NOT_FOUND, self.0.to_api_error()),
            AppError::Crypto { .. } => (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_api_error()),
            AppError::Db(_) | AppError::Internal(_) => {
                tracing::error!("Internal error: {}", self.0);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiError::new("INTERNAL_001", "内部错误"),
                )
            }
        };
        (status, Json(api_error)).into_response()
    }
}

/// 认证中间件
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<ApiError>)> {
    let token = req
        .headers()
        .get_all(axum::http::header::COOKIE)
        .iter()
        .find_map(|v| {
            let cookie = v.to_str().ok()?;
            cookie
                .split(';')
                .find_map(|c| c.trim().strip_prefix("token="))
        })
        .or_else(|| {
            req.headers()
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer "))
        })
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(ApiError::new(AUTH_003, "未登录")),
        ))?;

    let user_id = state
        .system_key
        .verify_jwt(token)
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(ApiError::new(AUTH_003, "会话已过期"))))?;

    req.extensions_mut().insert(AuthUser { user_id });
    Ok(next.run(req).await)
}

/// 统一 API 响应
pub struct ApiOk<T: Serialize>(pub T);

impl<T: Serialize> IntoResponse for ApiOk<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self.0)).into_response()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    dotenvy::dotenv().ok();

    let db_path =
        std::env::var("SELF_MEDIA_DB_PATH").unwrap_or_else(|_| "./data/self-media.db".into());
    let web_port: u16 = std::env::var("SELF_MEDIA_WEB_PORT")
        .unwrap_or_else(|_| "3000".into())
        .parse()?;

    let pool = create_pool(&db_path).await?;
    tracing::info!("数据库连接成功: {}", db_path);

    let system_key = SystemKey::generate();
    let http = reqwest::Client::new();

    let user_service = Arc::new(UserService::new(pool.clone(), system_key.clone()));
    let hotspot_service = HotspotService::new(http);
    let config_service = Arc::new(ConfigService::new(pool.clone()));
    let task_scheduler = TaskScheduler::new(pool.clone(), 5);
    let mut publisher_registry = PublisherRegistry::new();
    self_media_publish::adapters::register_all(&mut publisher_registry);
    let user_key_cache = Arc::new(UserKeyCache::new(100));

    let state = AppState {
        db: pool,
        system_key: Arc::new(system_key),
        user_service,
        hotspot_service: Arc::new(tokio::sync::Mutex::new(hotspot_service)),
        task_scheduler: Arc::new(tokio::sync::Mutex::new(task_scheduler)),
        config_service,
        publisher_registry: Arc::new(std::sync::Mutex::new(publisher_registry)),
        user_key_cache,
    };

    let protected = Router::new()
        .nest("/hotspot", hotspot::router())
        .nest("/tasks", task::router())
        .nest("/config", config::router())
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    let app = Router::new()
        .route("/api/health", get(health_check))
        .nest("/api/auth", auth::router())
        .nest("/api", protected)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", web_port);
    tracing::info!("Web 服务启动: http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}
