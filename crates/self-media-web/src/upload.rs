//! 文件上传 API
//! 
//! 提供文件上传、下载、删除接口

use axum::{
    extract::{Extension, State},
    http::header,
    routing::{get, post, delete},
    Json, Router, Response,
};
use bytes::Bytes;
use tokio::fs;
use tower_http::services::ServeDir;

use crate::{AppState, ApiOk, WebError};
use crate::storage::{StorageService, FileInfo};
use self_media_core::error::AppError;

/// 存储服务状态
#[derive(Clone)]
pub struct StorageState {
    pub storage: StorageService,
}

impl StorageState {
    pub fn new() -> Self {
        Self {
            storage: StorageService::from_env(),
        }
    }
}

impl Default for StorageState {
    fn default() -> Self {
        Self::new()
    }
}

/// 创建存储路由
pub fn router() -> Router {
    Router::new()
        .route("/upload", post(upload_file))
        .route("/files/{*path}", get(serve_file))
        .route("/files/{*path}", delete(delete_file))
        .nest("/static", ServeDir::new("./data/uploads"))
}

/// 上传文件
/// POST /api/upload
async fn upload_file(
    State(state): State<StorageState>,
    mut multipart: axum::extract::Multipart,
) -> Result<ApiOk<FileInfo>, WebError> {
    let mut file_data: Option<(Vec<u8>, String, String)> = None;
    
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        WebError(AppError::Internal(format!("读取上传文件失败: {}", e)))
    })? {
        let name = field.name().unwrap_or("file").to_string();
        let filename = field.file_name().unwrap_or("unnamed").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
        
        let data = field.bytes().await.map_err(|e| {
            WebError(AppError::Internal(format!("读取文件内容失败: {}", e)))
        })?;
        
        file_data = Some((data.to_vec(), filename, content_type));
        break; // 只处理第一个文件
    }
    
    let (data, filename, content_type) = file_data.ok_or_else(|| {
        WebError(AppError::Validation("没有找到上传文件".to_string()))
    })?;
    
    let file_info = state.storage.upload(&data, &filename, &content_type).await?;
    
    Ok(ApiOk(file_info))
}

/// 服务文件
/// GET /api/files/{path}
async fn serve_file(
    State(state): State<StorageState>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Result<Response, WebError> {
    let file_path = format!("./data/uploads/{}", path);
    
    let data = fs::read(&file_path).await.map_err(|e| {
        WebError(AppError::Internal(format!("文件不存在: {}", e)))
    })?;
    
    let filename = std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    
    let ext = std::path::Path::new(filename)
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
    
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_DISPOSITION, format!("inline; filename=\"{}\"", filename))
        .body(data.into())
        .unwrap())
}

/// 删除文件
/// DELETE /api/files/{path}
async fn delete_file(
    State(state): State<StorageState>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Result<ApiOk<()>, WebError> {
    let url = format!("/uploads/{}", path);
    state.storage.delete(&url).await?;
    Ok(ApiOk(()))
}
