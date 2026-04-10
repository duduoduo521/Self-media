use std::sync::Arc;

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{StatusCode, request::Parts},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};

use crate::csrf::csrf_protection;
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
use self_media_publish::qr_login::QrLoginManager;
use self_media_publish::PublisherRegistry;

mod auth;
mod config;
mod csrf;
mod hotspot;
mod qr_login;
mod sse;
mod storage;
mod task;

/// SSE 事件类型（复用 sse 模块定义）
pub use sse::SseEvent;
pub use sse::broadcast_event;

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
    pub qr_manager: Arc<tokio::sync::RwLock<QrLoginManager>>,
    pub sse_sender: tokio::sync::broadcast::Sender<sse::SseEvent>,
    pub http: reqwest::Client,
}

/// 已认证用户（由中间件注入到 request extensions）
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: i64,
    pub token: String,
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
            .cloned()
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
        ))?
        .to_string();

    let user_id = state
        .system_key
        .verify_jwt(&token)
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(ApiError::new(AUTH_003, "会话已过期"))))?;

    req.extensions_mut().insert(AuthUser { user_id, token });
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

    // 系统密钥持久化
    let system_key = load_or_create_system_key().await?;

    // 启动时保存一次密钥（确保文件存在）
    if let Err(e) = save_system_key(&system_key) {
        tracing::warn!("系统密钥持久化失败: {}", e);
    }
    let http = reqwest::Client::new();
    let http_for_hotspot = http.clone();
    let http_for_qr = http.clone();

    let user_service = Arc::new(UserService::new(pool.clone(), system_key.clone()));
    let hotspot_service = HotspotService::new(http_for_hotspot);
    let config_service = Arc::new(ConfigService::new(pool.clone()));
    let task_scheduler = TaskScheduler::new(pool.clone(), 5);
    let mut publisher_registry = PublisherRegistry::new();
    self_media_publish::adapters::register_all(&mut publisher_registry);
    let user_key_cache = Arc::new(UserKeyCache::new(100));

    // 初始化扫码登录管理器并注册处理器
    let mut qr_manager = QrLoginManager::new(http_for_qr);
    self_media_publish::adapters::register_qr_handlers(&mut qr_manager);
    let qr_manager = Arc::new(tokio::sync::RwLock::new(qr_manager));

    // SSE 广播通道
    let (sse_sender, _) = tokio::sync::broadcast::channel(100);

    let state = AppState {
        db: pool,
        system_key: Arc::new(system_key),
        user_service,
        hotspot_service: Arc::new(tokio::sync::Mutex::new(hotspot_service)),
        task_scheduler: Arc::new(tokio::sync::Mutex::new(task_scheduler)),
        config_service,
        publisher_registry: Arc::new(std::sync::Mutex::new(publisher_registry)),
        user_key_cache,
        qr_manager,
        sse_sender,
        http,
    };

    let protected = Router::new()
        .nest("/hotspot", hotspot::router())
        .nest("/tasks", task::router())
        .nest("/config", config::router())
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        // CSRF 防护：验证 X-CSRF-Token header 与 Cookie 匹配
        .layer(middleware::from_fn(csrf_protection));

    let app = Router::new()
        .route("/api/health", get(health_check))
        .merge(qr_login::qr_routes())
        .merge(sse::router())
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

/// 系统密钥文件路径
const SYSTEM_KEY_FILE: &str = "./data/system_key.enc";

/// 从文件加载或创建系统密钥
async fn load_or_create_system_key() -> anyhow::Result<SystemKey> {
    let machine_key = std::env::var("SELF_MEDIA_MACHINE_KEY")
        .unwrap_or_else(|_| "dev-machine-key-change-in-production".to_string());
    let machine_key_bytes = machine_key.as_bytes();

    // 尝试从文件加载
    if let Ok(encrypted) = tokio::fs::read_to_string(SYSTEM_KEY_FILE).await {
        match SystemKey::load(&encrypted, machine_key_bytes) {
            Ok(key) => {
                tracing::info!("系统密钥已从文件加载");
                return Ok(key);
            }
            Err(e) => {
                tracing::warn!("系统密钥加载失败，将重新生成: {}", e);
            }
        }
    }

    // 生成新密钥
    let key = SystemKey::generate();
    tracing::info!("新系统密钥已生成");
    Ok(key)
}

/// 保存系统密钥到文件
fn save_system_key(key: &SystemKey) -> anyhow::Result<()> {
    let machine_key = std::env::var("SELF_MEDIA_MACHINE_KEY")
        .unwrap_or_else(|_| "dev-machine-key-change-in-production".to_string());
    let machine_key_bytes = machine_key.as_bytes();

    let encrypted = key.save(machine_key_bytes)?;
    
    // 确保目录存在
    if let Some(parent) = std::path::Path::new(SYSTEM_KEY_FILE).parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    std::fs::write(SYSTEM_KEY_FILE, encrypted)?;
    tracing::info!("系统密钥已保存到文件");
    Ok(())
}
