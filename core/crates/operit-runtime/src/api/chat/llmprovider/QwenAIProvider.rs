use async_trait::async_trait;
use serde_json::Value;

use super::AIService::{AIService, AiServiceError, SendMessageRequest};
use super::OpenAIProvider::OpenAIProvider;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::util::stream::RevisableTextStream::RevisableTextStreamLike;

pub struct QwenAIProvider {
    inner: OpenAIProvider,
}

impl QwenAIProvider {
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
        let siliconFlowBudget = if self.inner.provider_type == "SILICONFLOW" && request.enable_thinking {
            self.resolve_silicon_flow_thinking_budget(&body)?
        } else {
            None
        };
        if let Value::Object(object) = &mut body {
            if self.inner.provider_type == "SILICONFLOW" {
                object
                    .entry("enable_thinking".to_string())
                    .or_insert_with(|| serde_json::json!(request.enable_thinking));
                if request.enable_thinking && !object.contains_key("thinking_budget") {
                    if let Some(budget) = siliconFlowBudget {
                        object.insert("thinking_budget".to_string(), serde_json::json!(budget));
                    }
                }
            } else if request.enable_thinking && !object.contains_key("enable_thinking") {
                object.insert("enable_thinking".to_string(), serde_json::json!(true));
            }
        }
        Ok(body)
    }

    fn resolve_silicon_flow_thinking_budget(&self, requestJson: &Value) -> Result<Option<i32>, AiServiceError> {
        let qualityLevel = ApiPreferences::getInstance()
            .thinkingQualityLevelFlow()
            .first()
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let requestedBudget = match qualityLevel.clamp(1, 4) {
            1 => None,
            2 => Some(4_096),
            3 => Some(8_192),
            4 => Some(16_384),
            _ => None,
        };
        let Some(requestedBudget) = requestedBudget else {
            return Ok(None);
        };
        let modelMaxTokens = requestJson
            .get("max_tokens")
            .and_then(Value::as_i64)
            .map(|value| value as i32)
            .filter(|value| *value > 1);
        if let Some(maxTokens) = modelMaxTokens {
            let capped = requestedBudget.min(maxTokens - 1);
            Ok((capped > 0).then_some(capped))
        } else {
            Ok(Some(requestedBudget))
        }
    }
}

#[async_trait]
impl AIService for QwenAIProvider {
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
