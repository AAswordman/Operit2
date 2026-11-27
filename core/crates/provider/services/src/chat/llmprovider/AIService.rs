use std::sync::Arc;

use async_trait::async_trait;
use operit_model::ModelParameter::ModelParameter;
use operit_model::OpenAIModels::ModelOption;
use operit_model::PromptTurn::PromptTurn;
use operit_model::ToolPrompt::ToolPrompt;
use operit_util::stream::RevisableTextStream::{
    collect_revised_text, collect_revised_text_observed, empty_revisable_event_channel,
    with_event_channel, DelegatingRevisableSharedTextStream, RevisableTextStreamLike,
};
use operit_util::stream::Stream::VecStream;

use super::ThinkingDialect::RequestThinking;

use serde_json::Value;
use thiserror::Error;

/// Shared provider response stream used by runtime generation coordination.
pub type SharedAiResponseStream = DelegatingRevisableSharedTextStream;

/// Complete request payload passed to an AI provider service.
pub struct SendMessageRequest {
    pub chat_history: Vec<PromptTurn>,
    pub model_parameters: Vec<ModelParameter<Value>>,
    /// The thinking the request asks for.
    pub thinking: RequestThinking,
    pub stream: bool,
    pub available_tools: Vec<ToolPrompt>,
    pub preserve_think_in_history: bool,
    pub enable_retry: bool,
    pub on_non_fatal_error: Option<Arc<dyn Fn(String) + Send + Sync>>,
    pub on_tool_invocation: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

/// Token usage counters accumulated by a provider service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenCounts {
    pub input: i64,
    pub cached_input: i64,
    pub output: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExtendedTokenCounts {
    pub cache_write_input: i64,
    pub reasoning: i64,
}

/// Extracts optional cache-write and reasoning token counts from OpenAI-style usage payloads.
pub fn parse_extended_usage_counts(usage: &serde_json::Value) -> ExtendedTokenCounts {
    let cache_write_input = usage
        .get("cache_creation_input_tokens")
        .and_then(serde_json::Value::as_i64)
        .or_else(|| {
            usage
                .pointer("/prompt_tokens_details/cache_write_tokens")
                .and_then(serde_json::Value::as_i64)
        })
        .or_else(|| {
            usage
                .pointer("/input_tokens_details/cache_write_tokens")
                .and_then(serde_json::Value::as_i64)
        })
        .or_else(|| {
            let creation = usage.get("cache_creation")?;
            let m5 = creation
                .get("ephemeral_5m_input_tokens")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            let h1 = creation
                .get("ephemeral_1h_input_tokens")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            Some((m5 + h1).max(0))
        })
        .unwrap_or(0)
        .max(0);
    let reasoning = usage
        .pointer("/completion_tokens_details/reasoning_tokens")
        .and_then(serde_json::Value::as_i64)
        .or_else(|| {
            usage
                .pointer("/output_tokens_details/reasoning_tokens")
                .and_then(serde_json::Value::as_i64)
        })
        .or_else(|| {
            usage
                .get("reasoning_tokens")
                .and_then(serde_json::Value::as_i64)
        })
        .unwrap_or(0)
        .max(0);
    ExtendedTokenCounts {
        cache_write_input,
        reasoning,
    }
}

/// Error surface shared by all AI provider services.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum AiServiceError {
    #[error("provider is not implemented: {0}")]
    ProviderNotImplemented(String),
    #[error("request cancelled")]
    RequestCancelled,
    #[error("AI first output timed out after 120 seconds")]
    FirstOutputTimeout,
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("request failed: {0}")]
    RequestFailed(String),
    #[error("token calculation failed: {0}")]
    TokenCalculationFailed(String),
}

/// Creates a revisable response stream from already collected chunks.
pub fn response_stream_from_chunks(chunks: Vec<String>) -> Box<dyn RevisableTextStreamLike> {
    let event_channel = empty_revisable_event_channel();
    event_channel.close();
    Box::new(with_event_channel(VecStream::new(chunks), event_channel))
}

/// Creates an empty closed response stream.
pub fn empty_response_stream() -> Box<dyn RevisableTextStreamLike> {
    response_stream_from_chunks(Vec::new())
}

/// Collects the accepted response, applying retries before exposing its final text.
pub async fn collect_response_text(mut stream: Box<dyn RevisableTextStreamLike>) -> Result<String, AiServiceError> {
    collect_revised_text(stream.as_mut()).await.map_err(AiServiceError::RequestFailed)
}

/// Collects the accepted response, reporting the first accepted chunk through `on_first_text`.
pub async fn collect_response_text_observed(
    mut stream: Box<dyn RevisableTextStreamLike>,
    on_first_text: impl FnMut(),
) -> Result<String, AiServiceError> {
    collect_revised_text_observed(stream.as_mut(), on_first_text)
        .await
        .map_err(AiServiceError::RequestFailed)
}

/// Rejects a failed producer before interpreting any received text as a completed model round.
pub fn ensure_response_completed(stream: &dyn RevisableTextStreamLike) -> Result<(), AiServiceError> {
    match stream.terminal_failure() {
        Some(error) => Err(AiServiceError::RequestFailed(error)),
        None => Ok(()),
    }
}

/// Converts provider errors into user-facing retry status text.
pub fn retry_error_text(error: &AiServiceError) -> String {
    if matches!(error, AiServiceError::FirstOutputTimeout) {
        return "AI 首字等待超过 120 秒".to_string();
    }
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

/// Formats a retry progress message for the given retry attempt number.
pub fn retry_message(error_text: &str, retry_number: i32) -> String {
    format!("{error_text}，正在进行第 {retry_number} 次重试...")
}

/// Common async interface implemented by every model provider.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait AIService: Send + Sync {
    /// Returns the accumulated uncached input token count.
    fn input_token_count(&self) -> i64 {
        0
    }

    /// Returns the accumulated cached input token count.
    fn cached_input_token_count(&self) -> i64 {
        0
    }

    /// Returns the accumulated output token count.
    fn output_token_count(&self) -> i64 {
        0
    }

    /// Returns cache-write input tokens reported by the provider when available.
    fn cache_write_input_token_count(&self) -> i64 {
        0
    }

    /// Returns reasoning tokens reported by the provider when available.
    fn reasoning_token_count(&self) -> i64 {
        0
    }

    /// Returns the provider/model identifier used in diagnostics.
    fn provider_model(&self) -> String {
        "UNKNOWN:unknown".to_string()
    }

    /// Resets token counters maintained by the service.
    fn reset_token_counts(&mut self) {}

    /// Cancels active streaming work owned by the service.
    fn cancel_streaming(&mut self) {}

    /// Lists models available from the provider.
    async fn get_models_list(&self) -> Result<Vec<ModelOption>, AiServiceError> {
        Err(AiServiceError::ProviderNotImplemented(
            self.provider_model(),
        ))
    }

    /// Sends a chat request and returns a revisable text stream.
    async fn send_message(
        &mut self,
        _request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        Err(AiServiceError::ProviderNotImplemented(
            self.provider_model(),
        ))
    }

    /// Checks whether the provider can be reached with the current configuration.
    async fn test_connection(&self) -> Result<String, AiServiceError> {
        Err(AiServiceError::ProviderNotImplemented(
            self.provider_model(),
        ))
    }

    /// Calculates input tokens for a prompt and visible tool list.
    async fn calculate_input_tokens(
        &self,
        _chat_history: &[PromptTurn],
        _available_tools: &[ToolPrompt],
    ) -> Result<i64, AiServiceError> {
        Err(AiServiceError::ProviderNotImplemented(
            self.provider_model(),
        ))
    }

    /// Releases provider resources held by the service.
    fn release(&mut self) {}
}
