//! 任务执行引擎
//! 
//! 完整链路：AI 生成 → 内容适配 → 平台发布

use std::sync::Arc;

use self_media_ai::client::MiniMaxClient;
use self_media_ai::error::AiError;
use self_media_ai::model::*;
use self_media_crypto::UserKey;
use sqlx::SqlitePool;

use crate::config::ConfigService;
use crate::error::*;
use crate::types::*;
use crate::task::model::Task;

/// 执行上下文
pub struct ExecutionContext {
    pub task: Task,
    pub user_id: i64,
    pub user_key: UserKey,
    pub config_service: Arc<ConfigService>,
    pub db: SqlitePool,
}

/// 执行结果
#[derive(Debug)]
pub struct ExecutionResult {
    pub success: bool,
    pub results: Vec<PublishResult>,
    pub error_message: Option<String>,
}

/// 执行引擎
pub struct TaskExecutor;

impl TaskExecutor {
    /// 执行图文模式任务
    pub async fn execute_text_mode(ctx: &ExecutionContext) -> Result<ExecutionResult, AppError> {
        let results = Vec::new();
        
        // 获取 API Key
        let (api_key, region) = ctx.config_service
            .get_api_key(ctx.user_id, "minimax", &ctx.user_key)
            .await?;
        
        let base_url = match region {
            MiniMaxRegion::CN => "https://api.minimax.chat".to_string(),
            MiniMaxRegion::Global => "https://api.minimaxi.chat".to_string(),
        };
        
        let client = MiniMaxClient::new(api_key, base_url);
        
        // ===== 步骤 1: AI 文本生成 =====
        ctx.update_step("generate_text", 1, 4).await?;
        
        let text_result = Self::generate_text(&client, &ctx.task.topic).await
            .map_err(|e| AppError::ai(AI_001, e.to_string()))?;
        
        let generated_text = text_result.choices
            .first().map(|c| c.message.content.clone())
            .unwrap_or_default();
        
        // ===== 步骤 2: AI 图片生成 =====
        ctx.update_step("generate_images", 2, 4).await?;
        
        let platforms: Vec<Platform> = serde_json::from_str(&ctx.task.platforms)?;
        let image_count = 3; // 默认生成 3 张图
        
        let image_result = Self::generate_images(&client, &generated_text, image_count).await
            .map_err(|e| AppError::ai(AI_001, e.to_string()))?;
        
        let _image_urls: Vec<String> = image_result.data
            .iter()
            .map(|d| d.url.clone())
            .collect();
        
        // ===== 步骤 3: 内容适配 =====
        ctx.update_step("adapt_content", 3, 4).await?;
        
        let _adapted_contents = Self::adapt_content(&generated_text, &platforms);
        
        // ===== 步骤 4: 逐平台发布 =====
        ctx.update_step("publish", 4, 4).await?;
        
        // TODO: 获取发布器并发布
        // 需要从 AppState 获取 PublisherRegistry
        
        Ok(ExecutionResult {
            success: true,
            results,
            error_message: None,
        })
    }
    
    /// 执行视频模式任务
    pub async fn execute_video_mode(ctx: &ExecutionContext) -> Result<ExecutionResult, AppError> {
        // 获取 API Key
        let (api_key, region) = ctx.config_service
            .get_api_key(ctx.user_id, "minimax", &ctx.user_key)
            .await?;
        
        let base_url = match region {
            MiniMaxRegion::CN => "https://api.minimax.chat".to_string(),
            MiniMaxRegion::Global => "https://api.minimaxi.chat".to_string(),
        };
        
        let client = MiniMaxClient::new(api_key, base_url);
        
        // ===== 步骤 1: 脚本生成 =====
        ctx.update_step("generate_script", 1, 4).await?;
        
        let _script_result = Self::generate_script(&client, &ctx.task.topic).await
            .map_err(|e| AppError::ai(AI_001, e.to_string()))?;
        
        // TODO: 使用生成的脚本进行视频生成
        
        // ===== 步骤 2: 视频生成 =====
        ctx.update_step("generate_video", 2, 4).await?;
        
        // 注意：视频生成是异步的，需要轮询
        // 这里简化处理，实际需要更复杂的逻辑
        
        // ===== 步骤 3: TTS 语音生成 =====
        ctx.update_step("generate_tts", 3, 4).await?;
        
        // ===== 步骤 4: 发布 =====
        ctx.update_step("publish", 4, 4).await?;
        
        Ok(ExecutionResult {
            success: true,
            results: vec![],
            error_message: None,
        })
    }
    
    // ===== AI 调用辅助方法 =====
    
    async fn generate_text(client: &MiniMaxClient, topic: &str) -> Result<TextResponse, AiError> {
        let request = TextRequest {
            model: "abab6.5s-chat".to_string(),
            temperature: Some(0.7),
            stream: Some(false),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "你是一个专业的新媒体内容创作者，擅长撰写吸引人的社交媒体文章。".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: format!("请为以下主题创作一篇吸引人的社交媒体文章：{}", topic),
                },
            ],
        };
        
        client.generate_text(request).await
    }
    
    async fn generate_images(client: &MiniMaxClient, text: &str, count: u32) -> Result<ImageResponse, AiError> {
        let request = ImageRequest {
            model: "image-01".to_string(),
            prompt: format!("用于社交媒体的配图，主题：{}", text),
            n: Some(count),
            aspect_ratio: Some("1:1".to_string()),
        };
        
        client.generate_images(request).await
    }
    
    async fn generate_script(client: &MiniMaxClient, topic: &str) -> Result<TextResponse, AiError> {
        let request = TextRequest {
            model: "abab6.5s-chat".to_string(),
            temperature: Some(0.7),
            stream: Some(false),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "你是一个专业的短视频脚本作家，擅长创作吸引人的短视频内容。".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: format!("请为以下主题创作一个短视频脚本（包含画面描述和配音文字）：{}", topic),
                },
            ],
        };
        
        client.generate_text(request).await
    }
    
    /// 根据平台要求适配内容
    fn adapt_content(text: &str, platforms: &[Platform]) -> Vec<(Platform, String)> {
        platforms.iter().map(|p| {
            let adapted = match p {
                Platform::Weibo => {
                    // 微博：限制 2000 字，支持话题标签
                    let truncated = if text.len() > 2000 { &text[..2000] } else { text };
                    format!("{}\n\n#新媒体# #内容创作#", truncated)
                }
                Platform::Bilibili => {
                    // B站：标题 + 正文，分段
                    let lines: Vec<&str> = text.lines().take(50).collect();
                    format!("{}\n\n欢迎观看！记得一键三连哦～", lines.join("\n"))
                }
                Platform::Toutiao => {
                    // 头条：标题 + 正文，添加关键词
                    format!("【{}】\n\n{}", "今日热文", text)
                }
                Platform::Xiaohongshu => {
                    // 小红书：emoji + 标签
                    format!("✨ 今日分享 ✨\n\n{}\n\n#种草 #好物推荐", text)
                }
                Platform::Douyin => {
                    // 抖音：简短，配标签
                    let short = if text.len() > 150 { &text[..150] } else { text };
                    format!("{}\n\n#抖音 #热门", short)
                }
                Platform::WeChatOfficial => {
                    // 公众号：完整内容
                    text.to_string()
                }
            };
            (p.clone(), adapted)
        }).collect()
    }
}

impl ExecutionContext {
    /// 更新任务步骤
    pub async fn update_step(&self, step_name: &str, current: u32, total: u32) -> Result<(), AppError> {
        let progress = (current * 100) / total;
        
        sqlx::query(
            "UPDATE tasks SET current_step = ?, progress = ?, updated_at = datetime('now') WHERE id = ?"
        )
        .bind(step_name)
        .bind(progress as i64)
        .bind(&self.task.id)
        .execute(&self.db)
        .await?;
        
        tracing::info!("任务 {} 进度: {}% (step: {})", self.task.id, progress, step_name);
        Ok(())
    }
    
    /// 更新任务状态为运行中
    pub async fn mark_running(&self) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE tasks SET status = 'Running', updated_at = datetime('now') WHERE id = ?"
        )
        .bind(&self.task.id)
        .execute(&self.db)
        .await?;
        Ok(())
    }
    
    /// 更新任务状态为完成
    pub async fn mark_completed(&self, result: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE tasks SET status = 'Completed', progress = 100, result = ?, updated_at = datetime('now') WHERE id = ?"
        )
        .bind(result)
        .bind(&self.task.id)
        .execute(&self.db)
        .await?;
        Ok(())
    }
    
    /// 更新任务状态为失败
    pub async fn mark_failed(&self, error: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE tasks SET status = 'Failed', error = ?, updated_at = datetime('now') WHERE id = ?"
        )
        .bind(error)
        .bind(&self.task.id)
        .execute(&self.db)
        .await?;
        Ok(())
    }
}
