use async_trait::async_trait;
use serde_json::{json, Map, Value};

use super::AIService::{AIService, AiServiceError, SendMessageRequest};
use super::OpenAIProvider::OpenAIProvider;
use crate::util::ChatMarkupRegex::{attr_value, tag_ranges, ChatMarkupRegex};
use crate::util::stream::RevisableTextStream::RevisableTextStreamLike;

pub struct MistralProvider {
    inner: OpenAIProvider,
}

impl MistralProvider {
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
        let mut request_body = self.inner.create_request_body(request)?;
        if let Some(messages) = request_body.get_mut("messages").and_then(Value::as_array_mut) {
            for message in messages {
                if message.get("role").and_then(Value::as_str) != Some("assistant") {
                    continue;
                }
                if let Some(tool_calls) = message.get_mut("tool_calls").and_then(Value::as_array_mut) {
                    for (index, tool_call) in tool_calls.iter_mut().enumerate() {
                        let name = tool_call
                            .pointer("/function/name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let arguments = tool_call
                            .pointer("/function/arguments")
                            .and_then(Value::as_str)
                            .unwrap_or("{}");
                        let params = serde_json::from_str::<Value>(arguments).unwrap_or_else(|_| json!({}));
                        if let Some(object) = tool_call.as_object_mut() {
                            object.insert("id".to_string(), json!(generate_mistral_tool_call_id(&name, &params, index)));
                        }
                    }
                }
            }
        }
        Ok(request_body)
    }
}

#[async_trait]
impl AIService for MistralProvider {
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

fn generate_mistral_tool_call_id(tool_name: &str, params: &Value, index: usize) -> String {
    let raw = format!("{tool_name}:{params}:{index}");
    let mut hash: i32 = 0;
    for unit in raw.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(unit as i32);
    }
    let positive = if hash == i32::MIN { 0 } else { hash.abs() as u32 };
    let mut base = to_base36(positive)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    if base.is_empty() {
        base = "0".to_string();
    }
    let padded = format!("{base:0>9}");
    if padded.len() > 9 {
        padded[padded.len() - 9..].to_string()
    } else {
        padded
    }
}

fn to_base36(mut value: u32) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let mut chars = Vec::new();
    while value > 0 {
        chars.push(std::char::from_digit(value % 36, 36).unwrap_or('0'));
        value /= 36;
    }
    chars.iter().rev().collect()
}
