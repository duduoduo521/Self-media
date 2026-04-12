use std::time::{Duration, Instant};

use futures::StreamExt;
use reqwest::{Client, Method};

use crate::error::AiError;
use crate::model::*;

pub struct MiniMaxClient {
    http: Client,
    api_key: String,
    base_url: String,
}

impl MiniMaxClient {
    pub fn new(api_key: String, base_url: String) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("HTTP client build failed");
        Self { http, api_key, base_url }
    }

    fn authenticated_request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.api_key))
    }

    /// 非流式文本生成
    pub async fn generate_text(&self, req: TextRequest) -> Result<TextResponse, AiError> {
        let resp = self.authenticated_request(Method::POST, "/v1/text/chatcompletion_v2")
            .json(&req)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        tracing::debug!("MiniMax API response status: {}, body: {}", status, body);

        if !status.is_success() {
            return Err(AiError::Network(format!("API returned status {}: {}", status, body)));
        }

        let text_resp: TextResponse = serde_json::from_str(&body)
            .map_err(|e| AiError::Network(format!("Failed to parse response: {} - body: {}", e, body)))?;

        Ok(text_resp)
    }

    /// 流式文本生成（SSE）- 暂时禁用
    #[allow(dead_code)]
    pub async fn generate_text_stream(
        &self,
        _req: TextRequest,
    ) -> Result<futures::stream::BoxStream<'static, Result<TextChunk, AiError>>, AiError> {
        Err(AiError::Network("Streaming not implemented".into()))
    }

    /// 新的聊天完成 API (支持联网搜索)
    pub async fn chat_completions<R: serde::Serialize, T: serde::de::DeserializeOwned>(&self, req: &R) -> Result<T, AiError> {
        let resp = self.authenticated_request(Method::POST, "/v1/chat/completions")
            .json(req)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        tracing::debug!("MiniMax chat/completions response status: {}, body: {}", status, body);

        if !status.is_success() {
            return Err(AiError::Network(format!("API returned status {}: {}", status, body)));
        }

        let parsed: T = serde_json::from_str(&body)
            .map_err(|e| AiError::Network(format!("Failed to parse response: {} - body: {}", e, body)))?;

        Ok(parsed)
    }

    /// 使用新端点的文本生成 (MiniMax-M2.7等新模型)
    pub async fn generate_text_v2(&self, model: &str, messages: Vec<Message>, temperature: Option<f64>) -> Result<TextResponse, AiError> {
        let request = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "temperature": temperature
        });

        let resp: serde_json::Value = self.chat_completions(&request).await?;

        let content = resp["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        Ok(TextResponse {
            choices: vec![Choice {
                message: Message {
                    role: "assistant".to_string(),
                    content: content.clone(),
                },
                finish_reason: None,
            }],
            usage: Usage { total_tokens: 0 },
        })
    }

    /// 图片生成
    pub async fn generate_images(&self, req: ImageRequest) -> Result<ImageResponse, AiError> {
        let resp = self.authenticated_request(Method::POST, "/v1/image_generation")
            .json(&req)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| self.map_api_error(e))?
            .json::<ImageResponse>()
            .await?;
        Ok(resp)
    }

    /// 提交视频生成任务
    pub async fn submit_video_task(&self, req: VideoRequest) -> Result<VideoTaskResponse, AiError> {
        let resp = self.authenticated_request(Method::POST, "/v1/video_generation")
            .json(&req)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| self.map_api_error(e))?
            .json::<VideoTaskResponse>()
            .await?;
        Ok(resp)
    }

    /// 查询视频生成状态
    pub async fn poll_video_task(&self, task_id: &str) -> Result<VideoTaskStatus, AiError> {
        let resp = self.authenticated_request(
            Method::GET,
            &format!("/v1/query/video_generation?task_id={}", task_id),
        )
        .send()
        .await?
        .error_for_status()
        .map_err(|e| self.map_api_error(e))?
        .json::<VideoTaskStatus>()
        .await?;
        Ok(resp)
    }

    /// 轮询视频任务直到完成
    pub async fn poll_video_until_complete(
        &self,
        task_id: &str,
        max_duration: Duration,
    ) -> Result<String, AiError> {
        let start = Instant::now();
        loop {
            if start.elapsed() > max_duration {
                return Err(AiError::Timeout("Video generation timeout".into()));
            }

            let status = self.poll_video_task(task_id).await?;

            match status.status.as_str() {
                "success" => {
                    let file_id = status.file_id
                        .ok_or_else(|| AiError::Network("No file_id in response".into()))?;
                    return Ok(file_id);
                }
                "fail" => {
                    return Err(AiError::Network(format!("Video generation failed: {:?}", status)));
                }
                _ => {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// 获取视频下载链接
    pub async fn get_video_download_url(&self, file_id: &str) -> Result<String, AiError> {
        let resp = self.authenticated_request(
            Method::GET,
            &format!("/v1/files/retrieval?file_id={}", file_id),
        )
        .send()
        .await?
        .error_for_status()
        .map_err(|e| self.map_api_error(e))?
        .json::<serde_json::Value>()
        .await?;

        resp["data"]["download_url"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| AiError::Network("No download_url in response".into()))
    }

    /// 下载视频数据
    pub async fn download_video(&self, file_id: &str) -> Result<Vec<u8>, AiError> {
        let download_url = self.get_video_download_url(file_id).await?;
        let resp = self.http.get(&download_url)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| self.map_api_error(e))?
            .bytes()
            .await?
            .to_vec();
        Ok(resp)
    }

    /// 语音合成
    pub async fn synthesize_speech(&self, text: &str, voice_id: &str) -> Result<Vec<u8>, AiError> {
        let req_body = SpeechRequest {
            model: "speech-02-hd".into(),
            text: text.into(),
            stream: false,
            voice_setting: VoiceSetting {
                voice_id: voice_id.into(),
                speed: 1.0,
            },
            audio_setting: AudioSetting {
                sample_rate: 32000,
                format: "mp3".into(),
                channel: 1,
            },
        };

        let resp = self.authenticated_request(Method::POST, "/v1/t2a_v2")
            .json(&req_body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| self.map_api_error(e))?
            .bytes()
            .await?
            .to_vec();

        Ok(resp)
    }

    /// 验证 API Key 是否有效
    pub async fn validate_api_key(&self) -> Result<(), AiError> {
        let req = TextRequest {
            model: "MiniMax-Text-01".into(),
            messages: vec![
                Message {
                    role: "user".into(),
                    content: "hi".into(),
                }
            ],
            temperature: None,
            stream: Some(false),
        };

        let resp = self.authenticated_request(Method::POST, "/v1/text/chatcompletion_v2")
            .json(&req)
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AiError::InvalidApiKey(format!("Invalid API key, status: {}", resp.status())))
        }
    }

    fn map_api_error(&self, e: reqwest::Error) -> AiError {
        if e.is_timeout() {
            AiError::Timeout("Request timeout".into())
        } else if e.is_connect() {
            AiError::Network("Connection failed".into())
        } else {
            AiError::Network(e.to_string())
        }
    }
}
