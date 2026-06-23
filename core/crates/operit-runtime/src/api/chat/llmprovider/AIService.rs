use std::sync::Arc;
use std::time::Duration;

use crate::core::chat::hooks::PromptTurn::PromptTurn;
use crate::data::model::ModelParameter::ModelParameter;
use crate::data::model::OpenAIModels::ModelOption;
use crate::data::model::ToolPrompt::ToolPrompt;
use crate::util::stream::RevisableTextStream::{
    empty_revisable_event_channel, with_event_channel, DelegatingRevisableSharedTextStream,
    RevisableTextStreamLike,
};
use crate::util::stream::Stream::VecStream;
use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;

pub struct SendMessageRequest {
    pub chat_history: Vec<PromptTurn>,
    pub model_parameters: Vec<ModelParameter<Value>>,
    pub enable_thinking: bool,
    pub stream: bool,
    pub available_tools: Vec<ToolPrompt>,
    pub preserve_think_in_history: bool,
    pub enable_retry: bool,
    pub on_non_fatal_error: Option<Arc<dyn Fn(String) + Send + Sync>>,
    pub on_tool_invocation: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenCounts {
    pub input: i32,
    pub cached_input: i32,
    pub output: i32,
}

pub type SharedAiResponseStream = DelegatingRevisableSharedTextStream;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum AiServiceError {
    #[error("provider is not implemented: {0}")]
    ProviderNotImplemented(String),
    #[error("request cancelled")]
    RequestCancelled,
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("request failed: {0}")]
    RequestFailed(String),
    #[error("token calculation failed: {0}")]
    TokenCalculationFailed(String),
}

pub fn response_stream_from_chunks(chunks: Vec<String>) -> Box<dyn RevisableTextStreamLike> {
    let event_channel = empty_revisable_event_channel();
    event_channel.close();
    Box::new(with_event_channel(VecStream::new(chunks), event_channel))
}

pub fn empty_response_stream() -> Box<dyn RevisableTextStreamLike> {
    response_stream_from_chunks(Vec::new())
}

pub fn collect_stream_chunks(mut stream: Box<dyn RevisableTextStreamLike>) -> Vec<String> {
    let mut chunks = Vec::new();
    stream.collect(&mut |chunk| {
        chunks.push(chunk);
    });
    chunks
}

pub fn retry_error_text(error: &AiServiceError) -> String {
    let message = error.to_string();
    let lower = message.to_ascii_lowercase();
    if lower.contains("timeout") || lower.contains("timed out") {
        "连接超时".to_string()
    } else if lower.contains("dns")
        || lower.contains("resolve")
        || lower.contains("unknown host")
        || lower.contains("failed to lookup address")
    {
        "无法解析主机".to_string()
    } else {
        match error {
            AiServiceError::ConnectionFailed(value) if value.trim().is_empty() => {
                "网络中断".to_string()
            }
            AiServiceError::ConnectionFailed(value) => value.clone(),
            AiServiceError::RequestFailed(value) if value.trim().is_empty() => {
                "网络中断".to_string()
            }
            AiServiceError::RequestFailed(value) => value.clone(),
            AiServiceError::RequestCancelled => "请求已取消".to_string(),
            _ => message,
        }
    }
}

pub fn retry_message(error_text: &str, retry_number: i32) -> String {
    format!("{error_text}，正在进行第 {retry_number} 次重试...")
}

pub async fn delay_retry_ms(retry_attempt: i32) {
    let delay_ms = super::LlmRetryPolicy::LlmRetryPolicy::nextDelayMs(retry_attempt);
    tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait AIService: Send + Sync {
    fn input_token_count(&self) -> i32 {
        0
    }

    fn cached_input_token_count(&self) -> i32 {
        0
    }

    fn output_token_count(&self) -> i32 {
        0
    }

    fn provider_model(&self) -> String {
        "UNKNOWN:unknown".to_string()
    }

    fn reset_token_counts(&mut self) {}

    fn cancel_streaming(&mut self) {}

    async fn get_models_list(&self) -> Result<Vec<ModelOption>, AiServiceError> {
        Err(AiServiceError::ProviderNotImplemented(
            self.provider_model(),
        ))
    }

    async fn send_message(
        &mut self,
        _request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        Err(AiServiceError::ProviderNotImplemented(
            self.provider_model(),
        ))
    }

    async fn test_connection(&self) -> Result<String, AiServiceError> {
        Err(AiServiceError::ProviderNotImplemented(
            self.provider_model(),
        ))
    }

    async fn calculate_input_tokens(
        &self,
        _chat_history: &[PromptTurn],
        _available_tools: &[ToolPrompt],
    ) -> Result<i32, AiServiceError> {
        Err(AiServiceError::ProviderNotImplemented(
            self.provider_model(),
        ))
    }

    fn release(&mut self) {}
}
