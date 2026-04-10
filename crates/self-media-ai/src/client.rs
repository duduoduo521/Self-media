use std::time::{Duration, Instant};

use futures::StreamExt;
use reqwest::{Client, Method, StatusCode};

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
            .await?
            .error_for_status()
            .map_err(|e| self.map_api_error(e))?
            .json::<TextResponse>()
            .await?;
        Ok(resp)
    }

    /// 流式文本生成（SSE）
    pub async fn generate_text_stream(
        &self,
        req: TextRequest,
    ) -> Result<futures::stream::BoxStream<'static, Result<TextChunk, AiError>>, AiError> {
        let mut stream_req = req;
        stream_req.stream = Some(true);

        let resp = self.authenticated_request(Method::POST, "/v1/text/chatcompletion_v2")
            .json(&stream_req)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| self.map_api_error(e))?;

        Ok(parse_sse_stream(resp.bytes_stream()))
    }

    /// 图片生成
    pub async fn generate_images(&self, req: ImageRequest) -> Result<ImageResponse, AiError> {
        let resp = self.authenticated_request(Method::POST, "/v1/image/generation")
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

    /// 查询视频任务状态
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

    /// 轮询视频任务直到完成，带超时和指数退避
    pub async fn poll_video_until_complete(
        &self,
        task_id: &str,
        max_duration: Duration,
    ) -> Result<String, AiError> {
        let start = Instant::now();
        let mut interval = Duration::from_secs(5);
        let max_interval = Duration::from_secs(30);

        loop {
            if start.elapsed() > max_duration {
                return Err(AiError::Timeout("视频生成超时，已自动取消".into()));
            }

            tokio::time::sleep(interval).await;

            let status = self.poll_video_task(task_id).await?;
            match status.status.as_str() {
                "Success" => {
                    return status.file_id.ok_or(AiError::ApiError("视频生成成功但未返回文件 ID".into()));
                }
                "Failed" => {
                    let msg = status.error.unwrap_or_default();
                    return Err(AiError::ApiError(format!("视频生成失败: {}", msg)));
                }
                _ => {
                    interval = (interval * 2).min(max_interval);
                }
            }
        }
    }

    /// 获取视频文件下载 URL
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

        resp["file"]["download_url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or(AiError::Parse("视频下载 URL 获取失败".into()))
    }

    /// 下载视频文件
    pub async fn download_video(&self, file_id: &str) -> Result<Vec<u8>, AiError> {
        let download_url = self.get_video_download_url(file_id).await?;
        let resp = self.http.get(&download_url)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| self.map_api_error(e))?;
        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
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
            .json::<SpeechResponse>()
            .await?;

        let audio_bytes = hex::decode(&resp.data.hex)
            .map_err(|e| AiError::Parse(format!("音频解码失败: {}", e)))?;
        Ok(audio_bytes)
    }

    /// 验证 API Key 是否有效
    pub async fn validate_api_key(&self) -> Result<(), AiError> {
        let resp = self.http
            .post(format!("{}/v1/text/chatcompletion_v2", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "model": "MiniMax-Text-01",
                "messages": [{"role": "user", "content": "hi"}],
                "tokens_to_generate": 1
            }))
            .send()
            .await
            .map_err(|e| AiError::Network(e.to_string()))?;

        match resp.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(AiError::InvalidApiKey("API Key 无效".into()))
            }
            _ if resp.status().is_success() => Ok(()),
            _ => Err(AiError::ApiError(format!("API 返回错误: {}", resp.status()))),
        }
    }

    fn map_api_error(&self, e: reqwest::Error) -> AiError {
        match e.status() {
            Some(StatusCode::UNAUTHORIZED) | Some(StatusCode::FORBIDDEN) => {
                AiError::InvalidApiKey("API Key 无效或权限不足".into())
            }
            Some(StatusCode::TOO_MANY_REQUESTS) => {
                AiError::QuotaExceeded("API 调用配额不足或限流".into())
            }
            Some(StatusCode::REQUEST_TIMEOUT) | Some(StatusCode::GATEWAY_TIMEOUT) => {
                AiError::Timeout("AI 生成超时".into())
            }
            _ => AiError::ApiError(format!("AI 服务错误: {}", e)),
        }
    }
}

/// SSE 流解析器：处理 TCP 分包导致的数据切分
fn parse_sse_stream(
    byte_stream: impl futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
) -> futures::stream::BoxStream<'static, Result<TextChunk, AiError>> {
    futures::stream::unfold(
        (byte_stream.boxed(), String::new()),
        |(mut byte_stream, mut buffer)| async move {
            loop {
                if let Some(pos) = buffer.find("\n\n") {
                    let block = buffer[..pos].to_string();
                    buffer.drain(..pos + 2);

                    for line in block.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                return None;
                            }
                            if let Ok(chunk) = serde_json::from_str::<TextChunk>(data) {
                                return Some((Ok(chunk), (byte_stream, buffer)));
                            }
                        }
                    }
                    continue;
                }

                let item = byte_stream.next().await;
                match item {
                    Some(Ok(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        continue;
                    }
                    Some(Err(e)) => {
                        return Some((Err(AiError::Network(format!("流读取失败: {}", e))), (byte_stream, buffer)));
                    }
                    None => return None,
                }
            }
        },
    )
    .boxed()
}
