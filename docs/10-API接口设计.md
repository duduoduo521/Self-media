# API 接口设计详细文档

---

## 1. 统一错误响应格式

所有接口（Tauri Commands 和 Axum Routes）使用统一的错误结构：

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,       // 错误码，如 "AUTH_002"
    pub message: String,    // 人类可读的错误信息
}
```

**Tauri Commands** 返回 `Result<T, ApiError>` 而非 `Result<T, String>`，保留结构化错误信息。

**Axum Routes** 返回 HTTP 状态码 + JSON body：

```json
{
  "code": "AUTH_002",
  "message": "用户名或密码错误"
}
```

| HTTP 状态码 | 含义 |
|------------|------|
| 400 | 输入校验失败 (INPUT_001, AUTH_004, AUTH_005) |
| 401 | 认证失败 (AUTH_002, AUTH_003) |
| 409 | 资源冲突 (AUTH_001) |
| 429 | 限流 (AI_003, PLAT_003, TASK_003) |
| 500 | 内部错误 |

---

## 2. Tauri Commands（桌面端 IPC）

### 2.1 用户模块

```rust
#[tauri::command]
async fn user_register(
    username: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<(User, Session), ApiError>;

#[tauri::command]
async fn user_login(
    username: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<Session, ApiError>;

#[tauri::command]
async fn user_logout(
    token: String,
    state: State<'_, AppState>,
) -> Result<(), ApiError>;

#[tauri::command]
async fn user_change_password(
    token: String,
    old_password: String,
    new_password: String,
    state: State<'_, AppState>,
) -> Result<(), ApiError>;
```

### 2.2 热点模块

```rust
#[tauri::command]
async fn hotspot_fetch_all(
    force_refresh: Option<bool>,
    state: State<'_, AppState>,
) -> Result<Vec<Hotspot>, ApiError>;

#[tauri::command]
async fn hotspot_fetch_by_source(
    source: HotspotSource,
    state: State<'_, AppState>,
) -> Result<Vec<Hotspot>, ApiError>;
```

### 2.3 任务模块

```rust
#[tauri::command]
async fn task_create(
    token: String,
    mode: TaskMode,
    topic: String,
    platforms: Vec<Platform>,
    state: State<'_, AppState>,
) -> Result<Task, ApiError>;

#[tauri::command]
async fn task_execute(
    token: String,
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), ApiError>;

#[tauri::command]
async fn task_get(
    token: String,
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Task, ApiError>;

#[tauri::command]
async fn task_list(
    token: String,
    state: State<'_, AppState>,
) -> Result<Vec<Task>, ApiError>;

#[tauri::command]
async fn task_cancel(
    token: String,
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), ApiError>;
```

### 2.4 配置模块

```rust
#[tauri::command]
async fn config_set_api_key(
    token: String,
    provider: String,
    key: String,
    region: String,
    state: State<'_, AppState>,
) -> Result<(), ApiError>;

#[tauri::command]
async fn config_set_platform(
    token: String,
    platform: Platform,
    enabled: bool,
    image_count: u32,
    cookies: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), ApiError>;

#[tauri::command]
async fn config_get_platforms(
    token: String,
    state: State<'_, AppState>,
) -> Result<Vec<PlatformConfig>, ApiError>;

#[tauri::command]
async fn config_get_preferences(
    token: String,
    state: State<'_, AppState>,
) -> Result<UserPreferences, ApiError>;

#[tauri::command]
async fn config_set_preferences(
    token: String,
    preferences: UserPreferences,
    state: State<'_, AppState>,
) -> Result<(), ApiError>;
```

### 2.5 平台登录

```rust
#[tauri::command]
async fn platform_qr_login(
    token: String,
    platform: Platform,
    state: State<'_, AppState>,
) -> Result<QRLoginResult, ApiError>;

#[tauri::command]
async fn platform_check_login(
    token: String,
    platform: Platform,
    state: State<'_, AppState>,
) -> Result<CookieStatus, ApiError>;
```

### 2.6 事件推送

```rust
// 任务进度事件
app_handle.emit("task-progress", &TaskProgressEvent)?;

// 文本流式事件
app_handle.emit("text-stream", &TextChunkEvent)?;
```

---

## 3. Axum Routes（Web 端 REST API）

### 3.1 认证接口

```
POST   /api/auth/register
  Body: { "username": "string", "password": "string" }
  Response: { "user": {...}, "session": {...} }

POST   /api/auth/login
  Body: { "username": "string", "password": "string" }
  Response: { "session": {...} }
  Set-Cookie: token=<jwt>; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=604800

DELETE /api/auth/logout
  Cookie: token=<jwt>
  Response: 200 OK

PUT    /api/auth/password
  Cookie: token=<jwt>
  Body: { "old_password": "string", "new_password": "string" }
  Response: 200 OK
```

### 3.2 热点接口

```
GET    /api/hotspot?force_refresh=false
  Cookie: token=<jwt>
  Response: { "hotspots": [...] }

GET    /api/hotspot/:source
  Cookie: token=<jwt>
  Response: { "hotspots": [...] }
```

### 3.3 任务接口

```
POST   /api/tasks
  Cookie: token=<jwt>
  Body: { "mode": "Text|Video", "topic": "string", "platforms": [...] }
  Response: { "task": {...} }

POST   /api/tasks/:id/execute
  Cookie: token=<jwt>
  Response: 202 Accepted

GET    /api/tasks
  Cookie: token=<jwt>
  Response: { "tasks": [...] }

GET    /api/tasks/:id
  Cookie: token=<jwt>
  Response: { "task": {...} }

DELETE /api/tasks/:id
  Cookie: token=<jwt>
  Response: 200 OK

GET    /api/tasks/:id/stream   (SSE)
  Cookie: token=<jwt>
  Response: text/event-stream
```

### 3.4 配置接口

```
PUT    /api/config/api-key
  Cookie: token=<jwt>
  Body: { "provider": "minimax", "key": "string", "region": "cn|global" }
  Response: 200 OK

GET    /api/config/platforms
  Cookie: token=<jwt>
  Response: { "platforms": [...] }

PUT    /api/config/platforms/:platform
  Cookie: token=<jwt>
  Body: { "enabled": true, "image_count": 3, "cookies": "string" }
  Response: 200 OK

GET    /api/config/preferences
  Cookie: token=<jwt>
  Response: { "preferences": {...} }

PUT    /api/config/preferences
  Cookie: token=<jwt>
  Body: { "default_mode": "text", "default_tags": [...], "auto_publish": false }
  Response: 200 OK
```

### 3.5 平台登录接口

```
POST   /api/platforms/:platform/qrcode
  Cookie: token=<jwt>
  Response: { "qr_url": "string", "session_id": "string" }

GET    /api/platforms/:platform/login-status
  Cookie: token=<jwt>
  Response: { "valid": true, "last_checked": "..." }
```

---

## 4. 认证中间件

### 4.1 Tauri 端

```rust
/// 从 Tauri Command 参数中提取 token，验证后注入 user_id
fn verify_token(token: &str, system_key: &SystemKey) -> Result<i64, ApiError> {
    system_key.verify_jwt(token)
        .map_err(|_| ApiError::new(AUTH_003, "会话已过期，请重新登录"))
}
```

### 4.2 Axum 端

```rust
/// 已认证用户提取器
pub struct AuthUser {
    pub user_id: i64,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    S: AsRef<AppState>,  // 通过 State 访问 AppState
{
    type Rejection = (StatusCode, Json<ApiError>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = state.as_ref();

        // 1. 从 Cookie 中提取 token
        let token = parts.headers
            .get_all(COOKIE)
            .iter()
            .find_map(|v| {
                let cookie = v.to_str().ok()?;
                cookie.split(';')
                    .find_map(|c| c.trim().strip_prefix("token="))
            })
            .ok_or((StatusCode::UNAUTHORIZED, Json(ApiError::new(AUTH_003, "未登录"))))?;

        // 2. 验证 JWT
        let user_id = app_state.system_key.verify_jwt(token)
            .map_err(|_| (StatusCode::UNAUTHORIZED, Json(ApiError::new(AUTH_003, "会话已过期"))))?;

        Ok(AuthUser { user_id })
    }
}

/// 应用状态（由 Axum State 管理）
pub struct AppState {
    pub db: SqlitePool,
    pub system_key: SystemKey,
    pub user_key_cache: UserKeyCache,
    pub user_service: UserService,
    pub hotspot_service: HotspotService,
    pub task_scheduler: TaskScheduler,
    pub config_service: ConfigService,
    pub publisher_registry: PublisherRegistry,
}

impl AsRef<AppState> for AppState {
    fn as_ref(&self) -> &Self { self }
}
```

### 4.3 CSRF 防护中间件（Axum Web 端）

```rust
use axum_csrf::CsrfLayer;

/// CSRF 防护配置
pub fn csrf_layer() -> CsrfLayer {
    CsrfLayer::new()
        .cookie_name("csrf_token")
        .cookie_secure(true)     // 仅 HTTPS
        .cookie_http_only(false) // 前端需读取
        .cookie_same_site(SameSite::Strict)
}
```

前端在发起请求时，从 Cookie 读取 CSRF Token，放在请求头 `X-CSRF-Token` 中。

---

## 5. 实时进度推送

### 5.1 统一进度事件格式

```json
{
    "task_id": "uuid",
    "step_index": 2,
    "step_name": "generate_images",
    "status": "running",
    "progress_percent": 50,
    "message": "正在生成第 2/3 张配图...",
    "timestamp": "2026-04-09T10:30:00Z"
}
```

### 5.2 推送方式

| 端 | 方式 | 实现 |
|----|------|------|
| 桌面 | Tauri Event | `app_handle.emit("task-progress", &event)` |
| Web | WebSocket | `ws.send(Message::Text(serde_json::to_string(&event)?))` |

### 5.3 文本流式事件

```json
{
    "task_id": "uuid",
    "delta": "生成的一段文案内容...",
    "finish_reason": null
}
```
