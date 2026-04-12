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
use crate::user::model::UserModelConfig;
use crate::user::service::UserService;
use crate::task::model::Task;

/// 执行上下文
pub struct ExecutionContext {
    pub task: Task,
    pub user_id: i64,
    pub user_key: UserKey,
    pub config_service: Arc<ConfigService>,
    pub user_service: Arc<UserService>,
    pub db: SqlitePool,
}

/// 执行结果
#[derive(Debug)]
pub struct ExecutionResult {
    pub success: bool,
    pub generated_text: Option<String>,
    pub generated_images: Vec<String>,
    pub adapted_contents: Vec<(Platform, String)>,
    pub results: Vec<PublishResult>,
    pub error_message: Option<String>,
}

/// 仅生成结果（未适配平台）
#[derive(Debug)]
pub struct GenerationResult {
    pub success: bool,
    pub generated_text: Option<String>,
    pub generated_images: Vec<String>,
    pub error_message: Option<String>,
}

/// 执行引擎
pub struct TaskExecutor;

impl TaskExecutor {
    /// 仅生成内容（不进行平台适配和发布）
    pub async fn generate_content(ctx: &ExecutionContext) -> Result<GenerationResult, AppError> {
        let (api_key, region) = ctx.config_service
            .get_api_key(ctx.user_id, "minimax", &ctx.user_key)
            .await?;

        let base_url = match region {
            MiniMaxRegion::CN => "https://api.minimax.chat".to_string(),
            MiniMaxRegion::Global => "https://api.minimaxi.chat".to_string(),
        };

        let client = MiniMaxClient::new(api_key, base_url);
        let model_config = Self::get_model_config(ctx).await?;

        let generated_text = match ctx.task.get_mode() {
            TaskMode::Text => {
                ctx.update_step("generate_text", 1, 2).await?;
                let text_result = Self::generate_text(&client, &ctx.task.topic, ctx.task.get_event_date(), &model_config.text_model).await
                    .map_err(|e| AppError::ai(AI_001, e.to_string()))?;
                Some(text_result.choices.first().map(|c| c.message.content.clone()).unwrap_or_default())
            }
            TaskMode::Video => {
                ctx.update_step("generate_script", 1, 2).await?;
                let _script_result = Self::generate_script(&client, &ctx.task.topic, ctx.task.get_event_date(), &model_config.text_model).await
                    .map_err(|e| AppError::ai(AI_001, e.to_string()))?;
                None
            }
        };

        let generated_images = if ctx.task.get_mode() == TaskMode::Text {
            ctx.update_step("generate_images", 2, 2).await?;
            let image_count = 3;
            let image_result = Self::generate_images(&client, generated_text.as_deref().unwrap_or(""), image_count, &model_config.image_model).await
                .map_err(|e| AppError::ai(AI_001, e.to_string()))?;
            image_result.data.image_urls.clone()
        } else {
            vec![]
        };

        Ok(GenerationResult {
            success: true,
            generated_text,
            generated_images,
            error_message: None,
        })
    }

    /// 执行图文模式任务（生成 + 适配内容，不发布）
    pub async fn execute_text_mode(ctx: &ExecutionContext) -> Result<ExecutionResult, AppError> {
        let (api_key, region) = ctx.config_service
            .get_api_key(ctx.user_id, "minimax", &ctx.user_key)
            .await?;

        let base_url = match region {
            MiniMaxRegion::CN => "https://api.minimax.chat".to_string(),
            MiniMaxRegion::Global => "https://api.minimaxi.chat".to_string(),
        };

        let client = MiniMaxClient::new(api_key, base_url);
        let model_config = Self::get_model_config(ctx).await?;

        // 发布结果（暂不发布，仅返回占位）
        let publish_results: Vec<PublishResult> = vec![];

        // ===== 步骤 1: AI 文本生成 =====
        ctx.update_step("generate_text", 1, 4).await?;

        let text_result = Self::generate_text(&client, &ctx.task.topic, ctx.task.get_event_date(), &model_config.text_model).await
            .map_err(|e| AppError::ai(AI_001, e.to_string()))?;

        let generated_text = text_result.choices
            .first().map(|c| c.message.content.clone())
            .unwrap_or_default();

        // ===== 步骤 2: AI 图片生成 =====
        ctx.update_step("generate_images", 2, 4).await?;

        let platforms: Vec<Platform> = serde_json::from_str(&ctx.task.platforms)?;
        let image_count = 3; // 默认生成 3 张图

        let image_result = Self::generate_images(&client, &generated_text, image_count, &model_config.image_model).await
            .map_err(|e| AppError::ai(AI_001, e.to_string()))?;

        let image_urls: Vec<String> = image_result.data.image_urls.clone();

        // ===== 步骤 3: 内容适配 =====
        ctx.update_step("adapt_content", 3, 4).await?;

        let adapted_contents = Self::adapt_content(&generated_text, &platforms);

        // ===== 步骤 4: 逐平台发布（仅返回内容和平台，不在此执行）====
        ctx.update_step("publish", 4, 4).await?;

        Ok(ExecutionResult {
            success: true,
            generated_text: Some(generated_text),
            generated_images: image_urls,
            adapted_contents,
            results: publish_results,
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

        // 获取用户模型配置
        let model_config = Self::get_model_config(ctx).await?;

        // ===== 步骤 1: 脚本生成 =====
        ctx.update_step("generate_script", 1, 4).await?;

        let _script_result = Self::generate_script(&client, &ctx.task.topic, ctx.task.get_event_date(), &model_config.text_model).await
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
            generated_text: None,
            generated_images: vec![],
            adapted_contents: vec![],
            results: vec![],
            error_message: None,
        })
    }
    
    // ===== AI 调用辅助方法 =====

    async fn get_model_config(ctx: &ExecutionContext) -> Result<UserModelConfig, AppError> {
        ctx.user_service.get_user_model_config(ctx.user_id).await
    }

    async fn generate_text(client: &MiniMaxClient, topic: &str, event_date: Option<chrono::NaiveDate>, model: &str) -> Result<TextResponse, AiError> {
        let date_constraint = match event_date {
            Some(date) => format!("请务必使用 {} 当天的最新信息。", date.format("%Y年%m月%d日")),
            None => "请务必使用今天的最新信息。".to_string(),
        };

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: r#"你是一位资深新媒体编辑，擅长撰写贴近生活、文笔流畅的社交媒体文章。

写作要求：
1. **真实可信**：基于搜索到的真实信息创作，内容详实有据
2. **时效性强**：严格使用指定日期范围内的最新信息
3. **语言自然**：
   - 避免AI味的套话，如"首先"、"其次"、"综上所述"、"值得注意的是"等
   - 避免机械的连接词，多用短句和自然的过渡
   - 多用口语化表达，让人感觉是真实的人在说话
4. **吸引读者**：开头要抓人，用具体的场景、数字或争议性话题引入
5. **结构清晰**：重点突出，段落短小，一般不超过3-4行

文章长度：800-1500字"#.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: format!(
                    r#"请搜索关于"{}"的最新信息，然后基于真实搜索结果创作一篇社交媒体文章。

要求：
1. 搜索并引用真实的事件内容、数据、评论
2. {}
3. 写作风格要像真实的人类编辑写的，避免任何AI味
4. 直接开始写作，不需要自我介绍或结尾总结"#,
                    topic,
                    date_constraint
                ),
            },
        ];

        client.generate_text_v2(model, messages, Some(0.7)).await
    }

    async fn generate_images(client: &MiniMaxClient, text: &str, count: u32, model: &str) -> Result<ImageResponse, AiError> {
        let request = ImageRequest {
            model: model.to_string(),
            prompt: format!("用于社交媒体的配图，主题：{}", text),
            n: Some(count),
            aspect_ratio: Some("1:1".to_string()),
        };

        client.generate_images(request).await
    }

    async fn generate_script(client: &MiniMaxClient, topic: &str, event_date: Option<chrono::NaiveDate>, model: &str) -> Result<TextResponse, AiError> {
        let date_constraint = match event_date {
            Some(date) => format!("请务必使用 {} 当天的最新信息。", date.format("%Y年%m月%d日")),
            None => "请务必使用今天的最新信息。".to_string(),
        };

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: r#"你是一位资深短视频策划，擅长创作接地气、有创意的短视频脚本。

写作要求：
1. **真实可信**：基于搜索到的真实信息创作，内容详实有据
2. **语言自然**：
   - 避免AI味的开场白，如"大家好我是xxx"等套话
   - 口语化表达，像朋友聊天一样自然
   - 避免机械的连接词和总结性话语
3. **节奏明快**：开头3秒要有吸引力，能留住观众
4. **结构清晰**：场景描述简洁，配音文字短小有力

脚本格式：
- 场景/画面描述：[画面内容]
- 配音：[配音文字]
- 时长提示：（X秒）"#.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: format!(
                    r#"请搜索关于"{}"的最新信息，然后基于真实搜索结果创作一个短视频脚本。

要求：
1. 搜索并引用真实的事件内容、数据
2. {}
3. 脚本要像真实的人类创作者写的，避免AI味
4. 时长控制在60-90秒
5. 开头要有吸引力，能在3秒内抓住观众"#, topic, date_constraint
                ),
            },
        ];

        client.generate_text_v2(model, messages, Some(0.7)).await
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
