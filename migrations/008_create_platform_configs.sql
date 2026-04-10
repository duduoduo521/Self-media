-- 平台配置表
CREATE TABLE IF NOT EXISTS platform_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    platform TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    image_count INTEGER NOT NULL DEFAULT 1,
    cookies TEXT,
    extra TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, platform)
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_platform_configs_user ON platform_configs(user_id);
