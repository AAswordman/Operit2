use async_trait::async_trait;
use serde_json::Value;

use super::AIService::{AIService, AiServiceError, SendMessageRequest};
use super::OpenAIProvider::OpenAIProvider;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::util::stream::RevisableTextStream::RevisableTextStreamLike;

pub struct OpenRouterProvider {
    inner: OpenAIProvider,
}

impl OpenRouterProvider {
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
        let has_referer = custom_headers
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case("HTTP-Referer"));
        let has_title = custom_headers
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case("X-Title"));
        let mut merged_headers = Vec::new();
        if !has_referer {
            merged_headers.push(("HTTP-Referer".to_string(), "ai.assistance.operit".to_string()));
        }
        if !has_title {
            merged_headers.push(("X-Title".to_string(), "Assistance App".to_string()));
        }
        merged_headers.extend(custom_headers);
        Self {
            inner: OpenAIProvider::new(
                api_endpoint,
                api_key,
                model_name,
                provider_type,
                merged_headers,
                enable_tool_call,
            ),
        }
    }

    pub fn create_request_body(&self, request: &SendMessageRequest) -> Result<Value, AiServiceError> {
        let mut body = self.inner.create_request_body(request)?;
        let reasoning = if request.enable_thinking {
            let budget = self.resolve_reasoning_budget(&body)?;
            if let Some(budget) = budget {
                serde_json::json!({ "max_tokens": budget })
            } else {
                serde_json::json!({})
            }
        } else {
            serde_json::json!({ "enabled": false, "max_tokens": 0 })
        };
        if let Value::Object(object) = &mut body {
            object.insert("reasoning".to_string(), reasoning);
        }
        Ok(body)
    }

    fn resolve_reasoning_budget(&self, requestJson: &Value) -> Result<Option<i32>, AiServiceError> {
        let qualityLevel = ApiPreferences::getInstance()
            .thinkingQualityLevelFlow()
            .first()
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let requestedBudget = match qualityLevel.clamp(1, 4) {
            1 => None,
            2 => Some(1024),
            3 => Some(16_000),
            4 => Some(32_000),
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
            let capped = (requestedBudget).min(maxTokens - 1);
            Ok((capped > 0).then_some(capped))
        } else {
            Ok(Some(requestedBudget))
        }
    }
}

#[async_trait]
impl AIService for OpenRouterProvider {
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
