use async_trait::async_trait;
use serde_json::Value;

use super::AIService::{AIService, AiServiceError, SendMessageRequest};
use super::KimiProvider::KimiProvider;
use crate::util::stream::RevisableTextStream::RevisableTextStreamLike;

pub struct MimoProvider {
    inner: KimiProvider,
}

impl MimoProvider {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_endpoint: String,
        api_key: String,
        model_name: String,
        provider_type: String,
        custom_headers: Vec<(String, String)>,
        _supports_vision: bool,
        _supports_audio: bool,
        _supports_video: bool,
        enable_tool_call: bool,
    ) -> Self {
        let mut mimo_headers = custom_headers;
        if !api_key.trim().is_empty()
            && !mimo_headers
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case("api-key"))
        {
            mimo_headers.push(("api-key".to_string(), api_key.clone()));
        }
        Self {
            inner: KimiProvider::new(
                api_endpoint,
                api_key,
                model_name,
                provider_type,
                mimo_headers,
                _supports_vision,
                _supports_audio,
                _supports_video,
                enable_tool_call,
            ),
        }
    }

    pub fn create_request_body(&self, request: &SendMessageRequest) -> Result<Value, AiServiceError> {
        self.inner.create_request_body(request)
    }
}

#[async_trait]
impl AIService for MimoProvider {
    fn input_token_count(&self) -> i32 { self.inner.input_token_count() }
    fn cached_input_token_count(&self) -> i32 { self.inner.cached_input_token_count() }
    fn output_token_count(&self) -> i32 { self.inner.output_token_count() }
    fn provider_model(&self) -> String { self.inner.provider_model() }
    fn reset_token_counts(&mut self) { self.inner.reset_token_counts(); }
    fn cancel_streaming(&mut self) { self.inner.cancel_streaming(); }
    async fn send_message(
        &mut self,
        request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        self.inner.send_message(request).await
    }
    async fn calculate_input_tokens(
        &self,
        chat_history: &[crate::core::chat::hooks::PromptTurn::PromptTurn],
        available_tools: &[crate::data::model::ToolPrompt::ToolPrompt],
    ) -> Result<i32, AiServiceError> {
        self.inner.calculate_input_tokens(chat_history, available_tools).await
    }
}
