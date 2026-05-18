use crate::core::chat::hooks::PromptTurn::PromptTurn;
use crate::data::model::ModelParameter::ModelParameter;
use crate::data::model::OpenAIModels::ModelOption;
use crate::data::model::ToolPrompt::ToolPrompt;
use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct SendMessageRequest {
    pub chat_history: Vec<PromptTurn>,
    pub model_parameters: Vec<ModelParameter<Value>>,
    pub enable_thinking: bool,
    pub stream: bool,
    pub available_tools: Vec<ToolPrompt>,
    pub preserve_think_in_history: bool,
    pub enable_retry: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenCounts {
    pub input: i32,
    pub cached_input: i32,
    pub output: i32,
}

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

pub struct AiResponseStream {
    pub chunks: Vec<String>,
    pub token_counts: TokenCounts,
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

    async fn send_message(&mut self, _request: SendMessageRequest) -> Result<AiResponseStream, AiServiceError> {
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
