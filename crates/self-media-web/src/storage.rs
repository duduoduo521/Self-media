//! 文件存储服务
//! 
//! 支持本地文件存储和云存储（可扩展）

use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use anyhow::Result;
use self_media_core::error::AppError;

/// 存储配置
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StorageConfig {
    /// 存储类型：local, oss, s3
    pub storage_type: StorageType,
    /// 本地存储路径
    pub local_path: PathBuf,
    /// 基础URL（用于生成访问URL）
    pub base_url: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            storage_type: StorageType::Local,
            local_path: PathBuf::from("./data/uploads"),
            base_url: "/uploads".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum StorageType {
    Local,
    Oss,
    S3,
}

/// 文件存储服务
#[derive(Clone)]
#[allow(dead_code)]
pub struct StorageService {
    config: StorageConfig,
}

#[allow(dead_code)]
impl StorageService {
    /// 创建存储服务
    pub fn new(config: StorageConfig) -> Self {
        Self { config }
    }

    /// 从环境变量创建
    pub fn from_env() -> Self {
        let storage_type = std::env::var("STORAGE_TYPE")
            .unwrap_or_else(|_| "local".to_string());
        
        let storage_type = match storage_type.as_str() {
            "oss" => StorageType::Oss,
            "s3" => StorageType::S3,
            _ => StorageType::Local,
        };

        Self {
            config: StorageConfig {
                storage_type,
                local_path: std::env::var("STORAGE_LOCAL_PATH")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("./data/uploads")),
                base_url: std::env::var("STORAGE_BASE_URL")
                    .unwrap_or_else(|_| "/uploads".to_string()),
            },
        }
    }

    /// 上传文件
    pub async fn upload(&self, data: &[u8], filename: &str, content_type: &str) -> Result<FileInfo, AppError> {
        // 生成唯一文件名
        let hash = self.compute_hash(data);
        let extension = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        
        let unique_filename = if extension.is_empty() {
            format!("{}_{}", 
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis(),
                &hash[..8]
            )
        } else {
            format!("{}_{}.{}", 
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis(),
                &hash[..8],
                extension
            )
        };

        // 按日期分目录
        let now = chrono::Local::now();
        let sub_dir = format!("{}/{:02}", now.format("%Y-%m-%d"), now.format("%d"));
        let file_path = PathBuf::from(&self.config.local_path)
            .join(&sub_dir)
            .join(&unique_filename);

        // 确保目录存在
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                AppError::Internal(anyhow::anyhow!("创建存储目录失败: {}", e))
            })?;
        }

        // 写入文件
        let mut file = fs::File::create(&file_path).await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("创建文件失败: {}", e))
        })?;
        
        file.write_all(data).await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("写入文件失败: {}", e))
        })?;
        
        file.sync_all().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("同步文件失败: {}", e))
        })?;

        // 生成访问URL
        let url = format!("{}/{}/{}", self.config.base_url, sub_dir, unique_filename);

        Ok(FileInfo {
            filename: unique_filename,
            original_filename: filename.to_string(),
            url,
            size: data.len() as i64,
            content_type: content_type.to_string(),
            hash,
        })
    }

    /// 删除文件
    pub async fn delete(&self, url: &str) -> Result<(), AppError> {
        let path = url.trim_start_matches(&self.config.base_url);
        let file_path = PathBuf::from(&self.config.local_path).join(path.trim_start_matches('/'));
        
        fs::remove_file(&file_path).await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("删除文件失败: {}", e))
        })?;
        
        Ok(())
    }

    /// 获取文件
    pub async fn get(&self, url: &str) -> Result<Vec<u8>, AppError> {
        let path = url.trim_start_matches(&self.config.base_url);
        let file_path = PathBuf::from(&self.config.local_path).join(path.trim_start_matches('/'));
        
        fs::read(&file_path).await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("读取文件失败: {}", e))
        })
    }

    /// 计算文件哈希
    fn compute_hash(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }
}

/// 文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FileInfo {
    /// 生成的文件名
    pub filename: String,
    /// 原始文件名
    pub original_filename: String,
    /// 访问URL
    pub url: String,
    /// 文件大小（字节）
    pub size: i64,
    /// MIME类型
    pub content_type: String,
    /// SHA256哈希
    pub hash: String,
}

/// 文件上传请求
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UploadRequest {
    pub filename: String,
    pub content_type: String,
    #[serde(default)]
    pub category: Option<String>,
}

/// 文件上传响应
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct UploadResponse {
    pub file: FileInfo,
}
