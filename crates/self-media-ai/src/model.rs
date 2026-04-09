use serde::{Deserialize, Serialize};

// ---- 文本生成 ----

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextResponse {
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    pub total_tokens: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextChunk {
    pub choices: Vec<ChunkChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChunkChoice {
    pub delta: ChunkDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChunkDelta {
    #[serde(default)]
    pub content: String,
}

// ---- 图片生成 ----

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageResponse {
    pub data: Vec<ImageData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageData {
    pub url: String,
}

// ---- 视频生成 ----

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoTaskResponse {
    pub task_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoTaskStatus {
    pub status: String,
    pub file_id: Option<String>,
    pub error: Option<String>,
}

// ---- 语音合成 ----

#[derive(Debug, Serialize, Deserialize)]
pub struct SpeechRequest {
    pub model: String,
    pub text: String,
    pub stream: bool,
    pub voice_setting: VoiceSetting,
    pub audio_setting: AudioSetting,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoiceSetting {
    pub voice_id: String,
    pub speed: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioSetting {
    pub sample_rate: u32,
    pub format: String,
    pub channel: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpeechResponse {
    pub data: SpeechData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpeechData {
    pub hex: String,
}
