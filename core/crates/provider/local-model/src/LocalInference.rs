use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum LocalInferenceError {
    #[error("local model is not loaded: {0}")]
    ModelNotLoaded(String),
    #[error("local inference request failed: {0}")]
    RequestFailed(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelSelection {
    pub modelId: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalSttRequest {
    pub model: LocalModelSelection,
    pub audioPath: String,
    pub language: Option<String>,
    pub options: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalSttSegment {
    pub text: String,
    pub startMs: i64,
    pub endMs: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalSttResponse {
    pub text: String,
    pub segments: Vec<LocalSttSegment>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalTtsRequest {
    pub model: LocalModelSelection,
    pub text: String,
    pub voice: String,
    pub outputFormat: String,
    pub options: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalTtsResponse {
    pub audioBytes: Vec<u8>,
    pub outputFormat: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalChatRequest {
    pub model: LocalModelSelection,
    pub prompt: String,
    pub options: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalChatResponse {
    pub text: String,
    pub metadata: Value,
}

pub trait LocalSpeechToTextSession: Send {
    /// Transcribes one audio request with a loaded local STT model.
    fn transcribe(
        &mut self,
        request: LocalSttRequest,
    ) -> Result<LocalSttResponse, LocalInferenceError>;

    /// Releases resources held by this STT session.
    fn release(&mut self);
}

pub trait LocalTextToSpeechSession: Send {
    /// Synthesizes one text request with a loaded local TTS model.
    fn synthesize(
        &mut self,
        request: LocalTtsRequest,
    ) -> Result<LocalTtsResponse, LocalInferenceError>;

    /// Releases resources held by this TTS session.
    fn release(&mut self);
}

pub trait LocalChatSession: Send {
    /// Generates one chat response with a loaded local chat model.
    fn generate(
        &mut self,
        request: LocalChatRequest,
    ) -> Result<LocalChatResponse, LocalInferenceError>;

    /// Releases resources held by this chat session.
    fn release(&mut self);
}
