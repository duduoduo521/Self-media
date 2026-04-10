//! 扫码登录 API
//! 
//! 提供各平台的扫码登录接口

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::AppState;
use self_media_core::types::Platform;
use self_media_publish::qr_login::{LoginResult, QrCodeInfo};

/// 生成二维码请求
#[derive(Debug, Deserialize)]
pub struct GenerateQrRequest {
    pub platform: String,
}

/// 确认登录请求
#[derive(Debug, Deserialize)]
pub struct ConfirmLoginRequest {
    pub qr_id: String,
}

/// 统一响应
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn error(msg: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg),
        }
    }
}

/// 生成二维码
/// POST /api/qr/generate
pub async fn generate_qr(
    State(state): State<AppState>,
    Json(req): Json<GenerateQrRequest>,
) -> Result<Json<ApiResponse<QrCodeInfo>>, StatusCode> {
    let platform = match req.platform.to_lowercase().as_str() {
        "weibo" => Platform::Weibo,
        "bilibili" | "b站" => Platform::Bilibili,
        _ => {
            return Ok(Json(ApiResponse::error(format!(
                "不支持的平台: {}",
                req.platform
            ))));
        }
    };
    
    let manager = state.qr_manager.read().await;
    match manager.generate_qrcode(platform).await {
        Ok(qr_info) => Ok(Json(ApiResponse::success(qr_info))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

/// 查询二维码状态
/// GET /api/qr/status/:platform/:qr_id
pub async fn query_status(
    State(state): State<AppState>,
    Path((platform, qr_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let platform = match platform.to_lowercase().as_str() {
        "weibo" => Platform::Weibo,
        "bilibili" | "b站" => Platform::Bilibili,
        _ => {
            return Ok(Json(ApiResponse::error(format!(
                "不支持的平台: {}",
                platform
            ))));
        }
    };
    
    let manager = state.qr_manager.read().await;
    match manager.query_status(platform, &qr_id).await {
        Ok(status) => {
            let status_str = match status {
                self_media_publish::qr_login::QrCodeStatus::Pending => "pending",
                self_media_publish::qr_login::QrCodeStatus::Scanned => "scanned",
                self_media_publish::qr_login::QrCodeStatus::Confirmed => "confirmed",
                self_media_publish::qr_login::QrCodeStatus::Expired => "expired",
                self_media_publish::qr_login::QrCodeStatus::Failed => "failed",
            };
            Ok(Json(ApiResponse::success(status_str.to_string())))
        }
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

/// 确认登录
/// POST /api/qr/confirm
pub async fn confirm_login(
    State(state): State<AppState>,
    Json(req): Json<ConfirmLoginRequest>,
) -> Result<Json<ApiResponse<LoginResult>>, StatusCode> {
    let manager = state.qr_manager.read().await;
    
    // 需要知道平台才能确认，这里从 qr_id 推断
    let platform = if req.qr_id.starts_with("weibo_") {
        Platform::Weibo
    } else if req.qr_id.starts_with("bilibili_") {
        Platform::Bilibili
    } else {
        return Ok(Json(ApiResponse::error("无效的二维码 ID".into())));
    };
    
    match manager.confirm_login(platform, &req.qr_id).await {
        Ok(result) => Ok(Json(ApiResponse::success(result))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

/// 轮询等待登录确认（阻塞）
/// GET /api/qr/wait/:platform/:qr_id
pub async fn wait_confirmation(
    State(state): State<AppState>,
    Path((platform, qr_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<LoginResult>>, StatusCode> {
    let platform = match platform.to_lowercase().as_str() {
        "weibo" => Platform::Weibo,
        "bilibili" | "b站" => Platform::Bilibili,
        _ => {
            return Ok(Json(ApiResponse::error(format!(
                "不支持的平台: {}",
                platform
            ))));
        }
    };
    
    let manager = state.qr_manager.read().await;
    // 等待 5 分钟超时
    match manager.wait_for_confirmation(platform, &qr_id, 300).await {
        Ok(result) => Ok(Json(ApiResponse::success(result))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

/// 扫码登录路由
pub fn qr_routes() -> Router<AppState> {
    Router::new()
        .route("/api/qr/generate", post(generate_qr))
        .route("/api/qr/status/{platform}/{qr_id}", get(query_status))
        .route("/api/qr/confirm", post(confirm_login))
        .route("/api/qr/wait/{platform}/{qr_id}", get(wait_confirmation))
}
