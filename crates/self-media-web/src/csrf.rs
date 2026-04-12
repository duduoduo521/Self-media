use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;

/// 生成 CSRF Token
pub fn generate_csrf_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// CSRF 中间件验证（Double Submit Cookie Pattern）
pub async fn csrf_protection(
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    // GET 请求跳过 CSRF 检查（CSRF 主要针对状态变更请求）
    if req.method().is_safe() {
        return Ok(next.run(req).await);
    }

    // 从请求头获取 CSRF Token
    let csrf_header = req
        .headers()
        .get("X-CSRF-Token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // 从 Cookie 获取 CSRF Token（需要通过提取器）
    let cookie_token = extract_csrf_token_from_request(&req);

    // Double Submit: 两者必须匹配
    match (csrf_header, cookie_token) {
        (Some(header), Some(cookie)) if header == cookie => {
            req.extensions_mut().insert(CsrfVerified(true));
            Ok(next.run(req).await)
        }
        _ => Err((StatusCode::FORBIDDEN, "CSRF 验证失败")),
    }
}

/// 从请求中提取 CSRF Token（手动解析 Cookie header）
fn extract_csrf_token_from_request(req: &Request) -> Option<String> {
    req.headers()
        .get("Cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .find_map(|c| c.trim().strip_prefix("csrf_token="))
        })
        .map(|s| s.to_string())
}

/// CSRF 验证标记（可注入到请求扩展中供后续使用）
#[derive(Clone, Debug)]
pub struct CsrfVerified(#[allow(dead_code)] pub bool);

/// 生成包含 CSRF Token 的 Cookie 响应
#[allow(dead_code)]
pub fn csrf_token_response(token: &str) -> (String, axum::http::header::HeaderValue) {
    let cookie = format!(
        "csrf_token={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=86400",
        token
    );
    let header_value = axum::http::header::HeaderValue::from_str(&cookie)
        .expect("Invalid CSRF cookie header");
    (token.to_string(), header_value)
}
