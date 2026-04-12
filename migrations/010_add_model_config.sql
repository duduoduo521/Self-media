-- 添加用户模型配置字段（如果不存在）
ALTER TABLE users ADD COLUMN text_model TEXT NOT NULL DEFAULT 'MiniMax-M2.7';
ALTER TABLE users ADD COLUMN image_model TEXT NOT NULL DEFAULT 'image-01';
ALTER TABLE users ADD COLUMN video_model TEXT NOT NULL DEFAULT 'video-01';
ALTER TABLE users ADD COLUMN speech_model TEXT NOT NULL DEFAULT 'speech-02-hd';
ALTER TABLE users ADD COLUMN music_model TEXT NOT NULL DEFAULT 'music-01';
