-- 添加加密用户密钥字段
-- 用于在服务重启后恢复用户密钥缓存
ALTER TABLE users ADD COLUMN encrypted_user_key TEXT;