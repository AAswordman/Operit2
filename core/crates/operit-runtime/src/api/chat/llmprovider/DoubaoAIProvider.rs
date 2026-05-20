use async_trait::async_trait;
use serde_json::Value;

use super::AIService::{AIService, AiServiceError, SendMessageRequest};
use super::OpenAIProvider::OpenAIProvider;
use crate::util::stream::RevisableTextStream::RevisableTextStreamLike;

pub struct DoubaoAIProvider {
    inner: OpenAIProvider,
}

impl DoubaoAIProvider {
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
        Self {
            inner: OpenAIProvider::new(api_endpoint, api_key, model_name, provider_type, custom_headers, enable_tool_call),
        }
    }

    pub fn create_request_body(&self, request: &SendMessageRequest) -> Result<Value, AiServiceError> {
        let mut body = self.inner.create_request_body(request)?;
        if let Value::Object(object) = &mut body {
            object.insert(
                "thinking".to_string(),
                serde_json::json!({
                    "type": if request.enable_thinking { "enabled" } else { "disabled" }
                }),
            );
        }
        Ok(body)
    }
}

#[async_trait]
impl AIService for DoubaoAIProvider {
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
        self.inner.reset_token_counts();
        let request_body = self.create_request_body(&request)?;
        self.inner.send_prepared_request(request, request_body).await
    }
    async fn calculate_input_tokens(
        &self,
        chat_history: &[crate::core::chat::hooks::PromptTurn::PromptTurn],
        available_tools: &[crate::data::model::ToolPrompt::ToolPrompt],
    ) -> Result<i32, AiServiceError> {
        self.inner.calculate_input_tokens(chat_history, available_tools).await
    }
}
