use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("数据库连接失败: {0}")]
    Connection(String),

    #[error("查询失败: {0}")]
    Query(String),

    #[error("迁移失败: {0}")]
    Migration(String),
}

impl From<sqlx::Error> for DbError {
    fn from(e: sqlx::Error) -> Self {
        DbError::Query(e.to_string())
    }
}
