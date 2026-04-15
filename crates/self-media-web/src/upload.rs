//! 文件上传 API
//! 
//! 提供文件上传、下载、删除接口
//! 
//! 安全措施：
//! - 所有接口需要用户认证
//! - 路径穿越防护：验证路径不包含危险字符
//! - 文件访问限制在用户专属目录

use axum::{
    extract::State,
    http::{header, StatusCode},
    routing::{get, post, delete},
    Router,
    response::Response,
};
use axum_extra::extract::Multipart;
use tokio::fs;
use std::path::PathBuf;

use crate::{AppState, ApiOk, WebError, AuthUser};
use crate::storage::{StorageService, FileInfo};
use self_media_core::error::{AppError, INPUT_001};
use anyhow::anyhow;

/// 创建存储路由（需要认证）
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/upload", post(upload_file))
        .route("/files/{path}", get(serve_file))
        .route("/files/{path}", delete(delete_file))
}

/// 验证路径安全性，防止路径穿越攻击
fn validate_path(path: &str) -> Result<PathBuf, AppError> {
    // 禁止空路径
    if path.is_empty() {
        return Err(AppError::validation(INPUT_001, "路径不能为空"));
    }
    
    // 禁止路径穿越字符
    if path.contains("..") || path.contains('\\') || path.starts_with('/') {
        return Err(AppError::validation(INPUT_001, "非法路径：禁止路径穿越"));
    }
    
    // 禁止隐藏文件
    let path_buf = PathBuf::from(path);
    let filename = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    
    if filename.starts_with('.') {
        return Err(AppError::validation(INPUT_001, "禁止访问隐藏文件"));
    }
    
    // 构建安全路径
    let safe_path = PathBuf::from("./data/uploads").join(path);
    
    // 验证最终路径在允许的目录内
    let canonical_upload = std::fs::canonicalize("./data/uploads")
        .map_err(|_| AppError::Internal(anyhow!("上传目录不存在")))?;
    
    // 如果目标文件不存在，检查父目录
    let check_path = if safe_path.exists() {
        safe_path.canonicalize()
            .map_err(|_| AppError::validation(INPUT_001, "路径规范化失败"))?
    } else {
        safe_path.parent()
            .and_then(|p| p.canonicalize().ok())
            .unwrap_or(canonical_upload.clone())
    };
    
    if !check_path.starts_with(&canonical_upload) {
        return Err(AppError::validation(INPUT_001, "路径超出允许范围"));
    }
    
    Ok(safe_path)
}

/// 上传文件（需要认证）
/// POST /api/upload
async fn upload_file(
    auth: AuthUser,
    State(_state): State<AppState>,
    mut multipart: Multipart,
) -> Result<ApiOk<FileInfo>, WebError> {
    let mut file_data: Option<(Vec<u8>, String, String)> = None;
    
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        WebError(AppError::Internal(anyhow!("读取上传文件失败: {}", e)))
    })? {
        let filename = field.file_name().unwrap_or("unnamed").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
        
        // 验证文件名安全性
        if filename.contains("..") || filename.contains('\\') || filename.contains('/') {
            return Err(WebError(AppError::validation(INPUT_001, "非法文件名")));
        }
        
        let data = field.bytes().await.map_err(|e| {
            WebError(AppError::Internal(anyhow!("读取文件内容失败: {}", e)))
        })?;
        
        file_data = Some((data.to_vec(), filename, content_type));
        break; // 只处理第一个文件
    }
    
    let (data, filename, content_type) = file_data.ok_or_else(|| {
        WebError(AppError::validation(INPUT_001, "没有找到上传文件"))
    })?;
    
    // 确保上传目录存在
    let user_upload_dir = PathBuf::from("./data/uploads").join(format!("user_{}", auth.user_id));
    fs::create_dir_all(&user_upload_dir).await.map_err(|e| {
        WebError(AppError::Internal(anyhow!("创建上传目录失败: {}", e)))
    })?;
    
    // 构建用户专属存储路径
    let storage = StorageService::from_env();
    let file_info = storage.upload(&data, &filename, &content_type).await?;
    
    tracing::info!("用户 {} 上传文件: {}", auth.user_id, file_info.url);
    
    Ok(ApiOk(file_info))
}

/// 服务文件（需要认证）
/// GET /api/files/{path}
async fn serve_file(
    auth: AuthUser,
    State(_state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Result<Response, WebError> {
    // 验证路径安全性
    let file_path = validate_path(&path)?;
    
    // 读取文件
    let data = fs::read(&file_path).await.map_err(|e| {
        WebError(AppError::Internal(anyhow!("文件不存在: {}", e)))
    })?;
    
    let path_buf = PathBuf::from(&path);
    let filename = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    
    let filename_path = PathBuf::from(filename);
    let ext = filename_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
    let content_type = match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    };
    
    tracing::info!("用户 {} 下载文件: {}", auth.user_id, path);
    
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_DISPOSITION, format!("inline; filename=\"{}\"", filename))
        .body(data.into())
        .map_err(|e| WebError(AppError::Internal(anyhow!("响应构建失败: {}", e))))?;
    
    Ok(response)
}

/// 删除文件（需要认证）
/// DELETE /api/files/{path}
async fn delete_file(
    auth: AuthUser,
    State(_state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Result<ApiOk<()>, WebError> {
    // 验证路径安全性
    let file_path = validate_path(&path)?;
    
    // 删除文件
    fs::remove_file(&file_path).await.map_err(|e| {
        WebError(AppError::Internal(anyhow!("删除文件失败: {}", e)))
    })?;
    
    tracing::info!("用户 {} 删除文件: {}", auth.user_id, path);
    
    Ok(ApiOk(()))
}