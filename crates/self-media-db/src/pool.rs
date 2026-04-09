use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteJournalMode, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;

use crate::DbError;

/// 创建 SQLite 连接池
pub async fn create_pool(db_path: &str) -> Result<SqlitePool, DbError> {
    // 确保数据库目录存在
    if let Some(parent) = Path::new(db_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DbError::Connection(format!("创建数据库目录失败: {}", e)))?;
        }
    }

    let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))
        .map_err(|e| DbError::Connection(format!("数据库连接配置错误: {}", e)))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| DbError::Connection(format!("数据库连接失败: {}", e)))?;

    // 执行迁移
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .map_err(|e| DbError::Migration(format!("数据库迁移失败: {}", e)))?;

    Ok(pool)
}
