use std::sync::Arc;

use crate::core::chat::hooks::PromptTurn::PromptTurn;
use crate::data::model::ModelParameter::ModelParameter;
use crate::data::model::OpenAIModels::ModelOption;
use crate::data::model::ToolPrompt::ToolPrompt;
use crate::util::stream::Stream::VecStream;
use crate::util::stream::RevisableTextStream::{
    empty_revisable_event_channel, with_event_channel, DelegatingRevisableSharedTextStream,
    RevisableTextStreamLike,
};
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

#[async_trait]
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
        Err(AiServiceError::ProviderNotImplemented(self.provider_model()))
    }

    async fn send_message(
        &mut self,
        _request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        Err(AiServiceError::ProviderNotImplemented(self.provider_model()))
    }

    async fn test_connection(&self) -> Result<String, AiServiceError> {
        Err(AiServiceError::ProviderNotImplemented(self.provider_model()))
    }

    async fn calculate_input_tokens(
        &self,
        _chat_history: &[PromptTurn],
        _available_tools: &[ToolPrompt],
    ) -> Result<i32, AiServiceError> {
        Err(AiServiceError::ProviderNotImplemented(self.provider_model()))
    }

    fn release(&mut self) {}
}
