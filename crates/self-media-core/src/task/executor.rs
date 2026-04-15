//! 任务执行引擎
//!
//! 完整链路：AI 生成 → 内容适配 → 平台发布

use std::sync::Arc;
use std::time::Duration;

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
    pub generated_video: Option<String>,
    pub generated_audio: Option<String>,
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

/// 视频生成结果
#[derive(Debug)]
pub struct VideoGenerationResult {
    pub script: String,
    pub video_url: Option<String>,
    pub audio_url: Option<String>,
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
                let script_result = Self::generate_script(&client, &ctx.task.topic, ctx.task.get_event_date(), &model_config.text_model).await
                    .map_err(|e| AppError::ai(AI_001, e.to_string()))?;
                Some(script_result.choices.first().map(|c| c.message.content.clone()).unwrap_or_default())
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

        let publish_results: Vec<PublishResult> = vec![];

        ctx.update_step("generate_text", 1, 4).await?;

        let text_result = Self::generate_text(&client, &ctx.task.topic, ctx.task.get_event_date(), &model_config.text_model).await
            .map_err(|e| AppError::ai(AI_001, e.to_string()))?;

        let generated_text = text_result.choices
            .first().map(|c| c.message.content.clone())
            .unwrap_or_default();

        ctx.update_step("generate_images", 2, 4).await?;

        let platforms: Vec<Platform> = serde_json::from_str(&ctx.task.platforms)?;
        let image_count = 3;

        let image_result = Self::generate_images(&client, &generated_text, image_count, &model_config.image_model).await
            .map_err(|e| AppError::ai(AI_001, e.to_string()))?;

        let image_urls: Vec<String> = image_result.data.image_urls.clone();

        ctx.update_step("adapt_content", 3, 4).await?;

        let adapted_contents = Self::adapt_content(&generated_text, &platforms);

        ctx.update_step("publish", 4, 4).await?;

        Ok(ExecutionResult {
            success: true,
            generated_text: Some(generated_text),
            generated_images: image_urls,
            generated_video: None,
            generated_audio: None,
            adapted_contents,
            results: publish_results,
            error_message: None,
        })
    }
    
    /// 执行视频模式任务
    /// 流程：生成脚本 → TTS配音 → 生成视频 → 下载视频 → ffmpeg合并音视频
    pub async fn execute_video_mode(ctx: &ExecutionContext) -> Result<ExecutionResult, AppError> {
        let (api_key, region) = ctx.config_service
            .get_api_key(ctx.user_id, "minimax", &ctx.user_key)
            .await?;

        let base_url = match region {
            MiniMaxRegion::CN => "https://api.minimax.chat".to_string(),
            MiniMaxRegion::Global => "https://api.minimaxi.chat".to_string(),
        };

        let client = MiniMaxClient::new(api_key, base_url);
        let model_config = Self::get_model_config(ctx).await?;

        ctx.update_step("generate_script", 1, 6).await?;

        let script_result = Self::generate_script(&client, &ctx.task.topic, ctx.task.get_event_date(), &model_config.text_model).await
            .map_err(|e| AppError::ai(AI_001, e.to_string()))?;

        let script_content = script_result.choices
            .first().map(|c| c.message.content.clone())
            .unwrap_or_default();

        tracing::info!("视频脚本生成完成，长度: {} 字符", script_content.len());

        ctx.update_step("extract_narration", 2, 6).await?;

        let narration_text = Self::extract_narration_from_script(&script_content);

        ctx.update_step("generate_tts", 3, 6).await?;

        let audio_data = Self::generate_tts(&client, &narration_text, &model_config.speech_model).await
            .map_err(|e| {
                tracing::warn!("TTS 生成失败: {}, 继续执行视频生成", e);
                AppError::ai(AI_002, format!("TTS 生成失败: {}", e))
            })?;

        let audio_path = Self::save_audio_file(&ctx.task.id, &audio_data).await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("音频文件保存失败: {}", e)))?;

        tracing::info!("TTS 音频生成完成，保存路径: {}", audio_path);

        ctx.update_step("generate_video", 4, 6).await?;

        let video_prompt = Self::build_video_prompt(&script_content);

        let video_download_url = Self::generate_video(&client, &video_prompt, &model_config.video_model).await
            .map_err(|e| {
                tracing::warn!("视频生成失败: {}", e);
                AppError::ai(AI_003, format!("视频生成失败: {}", e))
            })?;

        tracing::info!("视频生成完成，下载URL: {}", video_download_url);

        ctx.update_step("download_and_merge", 5, 6).await?;

        let raw_video_path = Self::download_video(&ctx.task.id, &video_download_url).await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("视频下载失败: {}", e)))?;

        tracing::info!("视频下载完成，保存路径: {}", raw_video_path);

        let final_video_path = Self::merge_video_audio(&ctx.task.id, &raw_video_path, &audio_path).await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("音视频合并失败: {}", e)))?;

        tracing::info!("音视频合并完成，最终视频路径: {}", final_video_path);

        ctx.update_step("adapt_content", 6, 6).await?;

        let platforms: Vec<Platform> = serde_json::from_str(&ctx.task.platforms)
            .map_err(|e| AppError::validation(INPUT_001, format!("平台配置解析失败: {}", e)))?;

        let adapted_contents = Self::adapt_video_content(&script_content, &final_video_path, &audio_path, &platforms);

        Ok(ExecutionResult {
            success: true,
            generated_text: Some(script_content),
            generated_images: vec![],
            generated_video: Some(final_video_path),
            generated_audio: Some(audio_path),
            adapted_contents,
            results: vec![],
            error_message: None,
        })
    }

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

    fn extract_narration_from_script(script: &str) -> String {
        let mut narration_parts: Vec<String> = Vec::new();
        
        for line in script.lines() {
            if line.contains("配音：") || line.contains("配音:") {
                let text = line
                    .replace("配音：", "")
                    .replace("配音:", "")
                    .trim()
                    .to_string();
                if !text.is_empty() {
                    narration_parts.push(text);
                }
            }
        }
        
        if narration_parts.is_empty() {
            script.lines()
                .filter(|l| !l.contains("场景") && !l.contains("画面") && !l.contains("时长"))
                .take(10)
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            narration_parts.join("\n")
        }
    }

    async fn generate_tts(client: &MiniMaxClient, text: &str, voice_id: &str) -> Result<Vec<u8>, AiError> {
        let voice_id = if voice_id.is_empty() { "male-qn-qingse" } else { voice_id };
        
        let truncated_text = if text.len() > 1000 {
            tracing::warn!("TTS 文本过长，截断到 1000 字符");
            &text[..1000]
        } else {
            text
        };

        client.synthesize_speech(truncated_text, voice_id).await
    }

    async fn save_audio_file(task_id: &str, audio_data: &[u8]) -> Result<String, anyhow::Error> {
        let audio_dir = std::path::PathBuf::from("./data/audio");
        tokio::fs::create_dir_all(&audio_dir).await?;
        
        let audio_path = audio_dir.join(format!("{}.mp3", task_id));
        tokio::fs::write(&audio_path, audio_data).await?;
        
        Ok(audio_path.to_string_lossy().to_string())
    }

    fn build_video_prompt(script: &str) -> String {
        let scene_parts: Vec<String> = script.lines()
            .filter(|l| l.contains("场景") || l.contains("画面"))
            .take(5)
            .map(|l| {
                l.replace("场景/画面描述：", "")
                    .replace("场景:", "")
                    .replace("画面:", "")
                    .trim()
                    .to_string()
            })
            .collect();

        if scene_parts.is_empty() {
            format!("根据以下内容创作短视频画面：{}", 
                script.lines().take(3).collect::<Vec<_>>().join(" "))
        } else {
            scene_parts.join("，")
        }
    }

    async fn generate_video(client: &MiniMaxClient, prompt: &str, model: &str) -> Result<String, AiError> {
        let request = VideoRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            image: None,
        };

        let task_response = client.submit_video_task(request).await?;
        tracing::info!("视频任务已提交，task_id: {}", task_response.task_id);

        let file_id = client.poll_video_until_complete(
            &task_response.task_id,
            Duration::from_secs(300)
        ).await?;

        let download_url = client.get_video_download_url(&file_id).await?;
        tracing::info!("视频下载链接: {}", download_url);

        Ok(download_url)
    }

    async fn download_video(task_id: &str, download_url: &str) -> Result<String, anyhow::Error> {
        let video_dir = std::path::PathBuf::from("./data/video");
        tokio::fs::create_dir_all(&video_dir).await?;
        
        let raw_video_path = video_dir.join(format!("{}_raw.mp4", task_id));
        
        tracing::info!("开始下载视频: {} -> {}", download_url, raw_video_path.display());
        
        let response = reqwest::get(download_url).await?;
        let video_data = response.bytes().await?;
        
        tokio::fs::write(&raw_video_path, &video_data).await?;
        
        tracing::info!("视频下载完成，大小: {} bytes", video_data.len());
        
        Ok(raw_video_path.to_string_lossy().to_string())
    }

    async fn merge_video_audio(task_id: &str, video_path: &str, audio_path: &str) -> Result<String, anyhow::Error> {
        let video_dir = std::path::PathBuf::from("./data/video");
        let final_video_path = video_dir.join(format!("{}.mp4", task_id));
        
        tracing::info!("开始合并音视频: {} + {} -> {}", video_path, audio_path, final_video_path.display());
        
        let output = tokio::process::Command::new("ffmpeg")
            .args([
                "-i", video_path,
                "-i", audio_path,
                "-c:v", "copy",
                "-c:a", "aac",
                "-map", "0:v:0",
                "-map", "1:a:0",
                "-shortest",
                "-y",
                final_video_path.to_str().unwrap(),
            ])
            .output()
            .await?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("ffmpeg 合并失败: {}", stderr);
            return Err(anyhow::anyhow!("ffmpeg 合并失败: {}", stderr));
        }
        
        tracing::info!("音视频合并完成: {}", final_video_path.display());
        
        Ok(final_video_path.to_string_lossy().to_string())
    }

    fn adapt_content(text: &str, platforms: &[Platform]) -> Vec<(Platform, String)> {
        platforms.iter().map(|p| {
            let adapted = match p {
                Platform::Weibo => {
                    let truncated = if text.len() > 2000 { &text[..2000] } else { text };
                    format!("{}\n\n#新媒体# #内容创作#", truncated)
                }
                Platform::Bilibili => {
                    let lines: Vec<&str> = text.lines().take(50).collect();
                    format!("{}\n\n欢迎观看！记得一键三连哦～", lines.join("\n"))
                }
                Platform::Toutiao => {
                    format!("【{}】\n\n{}", "今日热文", text)
                }
                Platform::Xiaohongshu => {
                    format!("✨ 今日分享 ✨\n\n{}\n\n#种草 #好物推荐", text)
                }
                Platform::Douyin => {
                    let short = if text.len() > 150 { &text[..150] } else { text };
                    format!("{}\n\n#抖音 #热门", short)
                }
                Platform::WeChatOfficial => {
                    text.to_string()
                }
            };
            (p.clone(), adapted)
        }).collect()
    }

    fn adapt_video_content(script: &str, video_url: &str, audio_path: &str, platforms: &[Platform]) -> Vec<(Platform, String)> {
        platforms.iter().map(|p| {
            let adapted = match p {
                Platform::Weibo => {
                    format!(
                        "🎬 新视频发布\n\n{}\n\n视频链接：{}\n音频：{}\n\n#短视频# #创作#",
                        script.lines().take(5).collect::<Vec<_>>().join("\n"),
                        video_url,
                        audio_path
                    )
                }
                Platform::Bilibili => {
                    format!(
                        "📹 {}\n\n视频已生成，欢迎观看！\n\n视频链接：{}\n\n记得一键三连哦～",
                        script.lines().take(3).collect::<Vec<_>>().join("\n"),
                        video_url
                    )
                }
                Platform::Douyin => {
                    format!(
                        "🎬 {}\n\n视频链接：{}\n\n#抖音 #热门 #短视频",
                        script.lines().take(2).collect::<Vec<_>>().join("\n"),
                        video_url
                    )
                }
                Platform::Xiaohongshu => {
                    format!(
                        "✨ 视频创作 ✨\n\n{}\n\n视频链接：{}\n\n#视频 #创作 #分享",
                        script.lines().take(5).collect::<Vec<_>>().join("\n"),
                        video_url
                    )
                }
                Platform::Toutiao => {
                    format!(
                        "【视频】{}\n\n视频链接：{}\n\n#热门视频#",
                        script.lines().take(3).collect::<Vec<_>>().join("\n"),
                        video_url
                    )
                }
                Platform::WeChatOfficial => {
                    format!(
                        "视频内容\n\n{}\n\n视频链接：{}\n音频文件：{}",
                        script,
                        video_url,
                        audio_path
                    )
                }
            };
            (p.clone(), adapted)
        }).collect()
    }
}

impl ExecutionContext {
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
    
    pub async fn mark_running(&self) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE tasks SET status = 'Running', updated_at = datetime('now') WHERE id = ?"
        )
        .bind(&self.task.id)
        .execute(&self.db)
        .await?;
        Ok(())
    }
    
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