-- 添加缺失的数据库索引，优化查询性能

-- platform_configs 表：按 user_id 查询需要索引
CREATE INDEX IF NOT EXISTS idx_platform_configs_user_id ON platform_configs(user_id);

-- api_keys 表：按 user_id 查询需要索引
CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);

-- user_preferences 表：主键已经是 user_id，无需额外索引

-- task_steps 表：按 task_id 查询需要索引（用于查看任务步骤）
CREATE INDEX IF NOT EXISTS idx_task_steps_task_id ON task_steps(task_id);