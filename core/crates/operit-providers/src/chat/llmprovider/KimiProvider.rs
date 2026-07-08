use async_trait::async_trait;
use serde_json::{json, Value};

use super::AIService::{AIService, AiServiceError, SendMessageRequest};
use super::OpenAIProvider::OpenAIProvider;
use super::StructuredToolCallBridge::StructuredToolCallBridge;
use operit_util::stream::RevisableTextStream::RevisableTextStreamLike;

pub struct KimiProvider {
    inner: OpenAIProvider,
}

impl KimiProvider {
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
    ) -> Self {
        Self {
            inner: OpenAIProvider::new_with_capabilities(
                api_endpoint,
                api_key,
                model_name,
                provider_type,
                custom_headers,
                supports_vision,
                supports_audio,
                supports_video,
                enable_tool_call,
            ),
        }
    }

    pub fn create_request_body(
        &self,
        request: &SendMessageRequest,
    ) -> Result<Value, AiServiceError> {
        let mut body = self.inner.create_request_body_internal(request)?;
        let Some(object) = body.as_object_mut() else {
            return Ok(body);
        };
        object.insert(
            "thinking".to_string(),
            json!({
                "type": if request.enable_thinking { "enabled" } else { "disabled" }
            }),
        );
        if request.enable_thinking {
            let useToolCall = self.inner.enable_tool_call && !request.available_tools.is_empty();
            let providerReadyHistory = self
                .inner
                .prepare_history_for_provider(&request.chat_history, useToolCall);
            object.insert(
                "messages".to_string(),
                build_messages_with_reasoning(
                    &providerReadyHistory,
                    request.preserve_think_in_history,
                    useToolCall,
                )?,
            );
            self.inner.calculate_and_store_input_tokens(
                &providerReadyHistory,
                object.get("tools").map(Value::to_string).as_deref(),
                true,
            );
        }
        Ok(body)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl AIService for KimiProvider {
    fn input_token_count(&self) -> i32 {
        self.inner.input_token_count()
    }

    fn cached_input_token_count(&self) -> i32 {
        self.inner.cached_input_token_count()
    }

    fn output_token_count(&self) -> i32 {
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
    ) -> Result<i32, AiServiceError> {
        self.inner
            .calculate_input_tokens(chat_history, available_tools)
            .await
    }
}

fn build_messages_with_reasoning(
    effective_history: &[operit_model::PromptTurn::PromptTurn],
    preserve_think_in_history: bool,
    use_tool_call: bool,
) -> Result<Value, AiServiceError> {
    let structuredMessages: Value =
        serde_json::from_str(&StructuredToolCallBridge::buildMessagesJsonForProvider(
            effective_history,
            preserve_think_in_history,
            use_tool_call,
        ))
        .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
    let mut messagesArray = Vec::new();
    let Some(messages) = structuredMessages.as_array() else {
        return Ok(Value::Array(messagesArray));
    };
    for messageValue in messages {
        let Some(messageObject) = messageValue.as_object() else {
            continue;
        };
        let role = messageObject
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("");
        if role == "assistant" {
            let contentValue = messageObject.get("content");
            let originalContent = match contentValue {
                Some(Value::String(value)) => value.clone(),
                Some(Value::Null) | None => String::new(),
                Some(value) => value.to_string(),
            };
            let (content, reasoningContent) = split_think_content(&originalContent);
            let mut message = messageObject.clone();
            message.insert("reasoning_content".to_string(), json!(reasoningContent));
            if message.contains_key("tool_calls") {
                if content.trim().is_empty() {
                    message.insert("content".to_string(), Value::Null);
                } else {
                    message.insert("content".to_string(), json!(content));
                }
            } else {
                message.insert(
                    "content".to_string(),
                    json!(if content.trim().is_empty() {
                        "[Empty]".to_string()
                    } else {
                        content
                    }),
                );
            }
            messagesArray.push(Value::Object(message));
        } else {
            messagesArray.push(messageValue.clone());
        }
    }
    Ok(Value::Array(messagesArray))
}

fn split_think_content(content: &str) -> (String, String) {
    let start_tag = "<think>";
    let end_tag = "</think>";
    let Some(start_index) = content.find(start_tag) else {
        return (content.to_string(), String::new());
    };
    let Some(end_relative_index) = content[start_index + start_tag.len()..].find(end_tag) else {
        return (content.to_string(), String::new());
    };
    let reasoning_start = start_index + start_tag.len();
    let reasoning_end = reasoning_start + end_relative_index;
    let reasoning_content = content[reasoning_start..reasoning_end].to_string();
    let mut visible_content = String::new();
    visible_content.push_str(&content[..start_index]);
    visible_content.push_str(&content[reasoning_end + end_tag.len()..]);
    (visible_content.trim().to_string(), reasoning_content)
}
