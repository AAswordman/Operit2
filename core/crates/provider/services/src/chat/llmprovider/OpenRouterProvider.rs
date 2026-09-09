use async_trait::async_trait;
use serde_json::Value;

use super::OpenAIProvider::OpenAIProvider;
use super::ThinkingConfiguration::ThinkingConfigurationApplier;
use crate::chat::llmprovider::AIService::{AIService, AiServiceError, SendMessageRequest};
use crate::runtime_support::ProviderRuntimeContext;
use operit_util::stream::RevisableTextStream::RevisableTextStreamLike;

pub struct OpenRouterProvider {
    inner: OpenAIProvider,
    runtime_context: ProviderRuntimeContext,
}

impl OpenRouterProvider {
    /// Creates an OpenRouter provider bound to one provider runtime context.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_endpoint: String,
        api_key: String,
        model_name: String,
        provider_type: String,
        custom_headers: Vec<(String, String)>,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        enable_tool_call: bool,
        runtime_context: ProviderRuntimeContext,
    ) -> Self {
        let has_referer = custom_headers
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case("HTTP-Referer"));
        let has_title = custom_headers
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case("X-Title"));
        let mut merged_headers = Vec::new();
        if !has_referer {
            merged_headers.push((
                "HTTP-Referer".to_string(),
                "ai.assistance.operit".to_string(),
            ));
        }
        if !has_title {
            merged_headers.push(("X-Title".to_string(), "Assistance App".to_string()));
        }
        merged_headers.extend(custom_headers);
        Self {
            inner: OpenAIProvider::new_with_capabilities(
                api_endpoint,
                api_key,
                model_name,
                "OPENROUTER_PARENT".to_string(),
                merged_headers,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ),
            runtime_context,
        }
    }

    pub fn create_request_body(
        &self,
        request: &SendMessageRequest,
    ) -> Result<Value, AiServiceError> {
        let mut body = self.inner.create_request_body(request)?;
        ThinkingConfigurationApplier::apply(
            &mut body,
            "OPENROUTER",
            &self.inner.model_name,
            &self.inner.api_endpoint,
            request.enable_thinking,
            request.thinking_quality_level,
            &request.thinking_configurations,
            &request.thinking_option_id,
        )?;
        Ok(body)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl AIService for OpenRouterProvider {
    fn input_token_count(&self) -> i64 {
        self.inner.input_token_count()
    }
    fn cached_input_token_count(&self) -> i64 {
        self.inner.cached_input_token_count()
    }
    fn output_token_count(&self) -> i64 {
        self.inner.output_token_count()
    }
    fn provider_model(&self) -> String {
        self.inner.provider_model()
    }
    fn reset_token_counts(&mut self) {
        self.inner.reset_token_counts();
    }
    fn cancel_streaming(&mut self) {
        self.inner.cancel_streaming();
    }
    async fn send_message(
        &mut self,
        request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        self.inner.reset_token_counts();
        let request_body = self.create_request_body(&request)?;
        self.inner
            .send_prepared_request(request, request_body)
            .await
    }
    async fn calculate_input_tokens(
        &self,
        chat_history: &[operit_model::PromptTurn::PromptTurn],
        available_tools: &[operit_model::ToolPrompt::ToolPrompt],
    ) -> Result<i64, AiServiceError> {
        self.inner
            .calculate_input_tokens(chat_history, available_tools)
            .await
    }
}
