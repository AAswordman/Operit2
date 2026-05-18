use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Map, Value};

use super::AIService::{AIService, AiResponseStream, AiServiceError, SendMessageRequest, TokenCounts};
use super::OpenAIProvider::OpenAIProvider;
use super::StructuredToolCallBridge::StructuredToolCallBridge;
use crate::core::chat::hooks::PromptTurn::{PromptTurn, PromptTurnKind};
use crate::data::model::ModelParameter::ModelParameter;
use crate::data::model::ToolPrompt::ToolPrompt;

pub struct DeepseekProvider {
    pub api_endpoint: String,
    pub api_key: String,
    pub model_name: String,
    pub provider_type: String,
    pub supports_vision: bool,
    pub supports_audio: bool,
    pub supports_video: bool,
    pub enable_tool_call: bool,
    pub custom_headers: Vec<(String, String)>,
    inputTokenCount: i32,
    cachedInputTokenCount: i32,
    outputTokenCount: i32,
    cancelled: bool,
}

impl DeepseekProvider {
    pub fn new(
        api_endpoint: String,
        api_key: String,
        model_name: String,
        provider_type: String,
        custom_headers: Vec<(String, String)>,
        enable_tool_call: bool,
    ) -> Self {
        Self {
            api_endpoint,
            api_key,
            model_name,
            provider_type,
            supports_vision: false,
            supports_audio: false,
            supports_video: false,
            enable_tool_call,
            custom_headers,
            inputTokenCount: 0,
            cachedInputTokenCount: 0,
            outputTokenCount: 0,
            cancelled: false,
        }
    }

    pub fn create_request_body(&self, request: &SendMessageRequest) -> Result<Value, AiServiceError> {
        let mut json_object = Map::new();
        json_object.insert("model".to_string(), json!(self.model_name));
        json_object.insert(
            "messages".to_string(),
            self.build_messages_with_reasoning(
                &StructuredToolCallBridge::compileHistoryForProvider(
                    &request.chat_history,
                    self.enable_tool_call && !request.available_tools.is_empty(),
                ),
                self.enable_tool_call && !request.available_tools.is_empty(),
            )?,
        );
        json_object.insert("stream".to_string(), json!(request.stream));
        json_object.insert(
            "thinking".to_string(),
            json!({
                "type": self.resolve_deepseek_thinking_effort(request.enable_thinking)
            }),
        );

        self.apply_model_parameters(&mut json_object, &request.model_parameters);

        if self.enable_tool_call && !request.available_tools.is_empty() {
            json_object.insert(
                "tools".to_string(),
                StructuredToolCallBridge::buildToolsArray(Some(&request.available_tools)),
            );
            json_object.insert("tool_choice".to_string(), json!("auto"));
        }

        Ok(Value::Object(json_object))
    }

    pub fn build_messages_with_reasoning(
        &self,
        effectiveHistory: &[PromptTurn],
        _useToolCall: bool,
    ) -> Result<Value, AiServiceError> {
        let structuredMessages: Value = serde_json::from_str(&StructuredToolCallBridge::buildMessagesJson(
            effectiveHistory,
            true,
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
            let role = messageObject.get("role").and_then(Value::as_str).unwrap_or("");
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
                        json!(if content.trim().is_empty() { "[Empty]".to_string() } else { content }),
                    );
                }
                messagesArray.push(Value::Object(message));
            } else {
                messagesArray.push(messageValue.clone());
            }
        }

        Ok(Value::Array(messagesArray))
    }

    pub fn resolve_deepseek_thinking_effort(&self, enable_thinking: bool) -> &'static str {
        match enable_thinking {
            true => "enabled",
            false => "disabled",
        }
    }

    fn apply_model_parameters(&self, json_object: &mut Map<String, Value>, parameters: &[ModelParameter<Value>]) {
        for parameter in parameters {
            if parameter.isEnabled {
                json_object.insert(parameter.apiName.clone(), parameter.currentValue.clone());
            }
        }
    }

    fn build_tools_json(&self, tools: &[ToolPrompt]) -> Result<Value, AiServiceError> {
        Ok(Value::Array(
            tools
                .iter()
                .map(|tool| {
                    Ok(json!({
                        "type": "function",
                        "function": {
                            "name": tool.name,
                            "description": tool.description,
                            "parameters": serde_json::from_str::<Value>(&tool.parameters)
                                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?
                        }
                    }))
                })
                .collect::<Result<Vec<_>, AiServiceError>>()?,
        ))
    }

    fn headers(&self) -> Result<HeaderMap, AiServiceError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if !self.api_key.trim().is_empty() {
            let value = HeaderValue::from_str(&format!("Bearer {}", self.api_key.trim()))
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            headers.insert(AUTHORIZATION, value);
        }
        for (name, value) in &self.custom_headers {
            let header_name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            let header_value =
                HeaderValue::from_str(value).map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            headers.insert(header_name, header_value);
        }
        Ok(headers)
    }

    fn apply_token_counts(&mut self, token_counts: TokenCounts) {
        self.inputTokenCount = token_counts.input;
        self.cachedInputTokenCount = token_counts.cached_input;
        self.outputTokenCount = token_counts.output;
    }
}

#[async_trait]
impl AIService for DeepseekProvider {
    fn input_token_count(&self) -> i32 {
        self.inputTokenCount
    }

    fn cached_input_token_count(&self) -> i32 {
        self.cachedInputTokenCount
    }

    fn output_token_count(&self) -> i32 {
        self.outputTokenCount
    }

    fn provider_model(&self) -> String {
        format!("{}:{}", self.provider_type, self.model_name)
    }

    fn reset_token_counts(&mut self) {
        self.inputTokenCount = 0;
        self.cachedInputTokenCount = 0;
        self.outputTokenCount = 0;
    }

    fn cancel_streaming(&mut self) {
        self.cancelled = true;
    }

    async fn send_message(&mut self, request: SendMessageRequest) -> Result<AiResponseStream, AiServiceError> {
        self.cancelled = false;
        self.reset_token_counts();

        let request_body = self.create_request_body(&request)?;
        let client = reqwest::Client::new();
        let response = client
            .post(&self.api_endpoint)
            .headers(self.headers()?)
            .json(&request_body)
            .send()
            .await
            .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let message = response
                .text()
                .await
                .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
            return Err(AiServiceError::RequestFailed(format!("{status}: {message}")));
        }

        if request.stream {
            return self.process_streaming_response(response).await;
        }

        let json_response: Value = response
            .json()
            .await
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let token_counts = json_response
            .get("usage")
            .map(parse_usage_counts)
            .unwrap_or(TokenCounts {
                input: 0,
                cached_input: 0,
                output: 0,
            });
        self.apply_token_counts(token_counts.clone());

        let mut chunks = Vec::new();
        if let Some(reasoning) = extract_reasoning_chunk(&json_response) {
            if !reasoning.is_empty() {
                chunks.push(format!("<think>{}</think>", reasoning));
            }
        }
        if let Some(content) = extract_content_chunk(&json_response) {
            if !content.is_empty() {
                chunks.push(StructuredToolCallBridge::convertToolCallPayloadToXml(&content));
            }
        }
        chunks.extend(extract_tool_calls_xml_chunks(&json_response));

        Ok(AiResponseStream {
            chunks,
            token_counts,
        })
    }

    async fn test_connection(&self) -> Result<String, AiServiceError> {
        let client = reqwest::Client::new();
        let response = client
            .post(&self.api_endpoint)
            .headers(self.headers()?)
            .json(&json!({
                "model": self.model_name,
                "messages": [{"role": "user", "content": "hi"}],
                "stream": false,
                "max_tokens": 1,
                "thinking": {"type": "disabled"}
            }))
            .send()
            .await
            .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
        if response.status().is_success() {
            Ok("Connection successful".to_string())
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
            Err(AiServiceError::ConnectionFailed(format!("{status}: {body}")))
        }
    }

    async fn calculate_input_tokens(
        &self,
        chat_history: &[PromptTurn],
        available_tools: &[ToolPrompt],
    ) -> Result<i32, AiServiceError> {
        let history_chars: usize = chat_history.iter().map(|turn| turn.content.chars().count()).sum();
        let tool_chars: usize = available_tools
            .iter()
            .map(|tool| tool.name.len() + tool.description.len() + tool.parameters.len())
            .sum();
        Ok(((history_chars + tool_chars + 3) / 4) as i32)
    }
}

impl DeepseekProvider {
    async fn process_streaming_response(
        &mut self,
        response: reqwest::Response,
    ) -> Result<AiResponseStream, AiServiceError> {
        let mut parent = OpenAIProvider::new(
            self.api_endpoint.clone(),
            self.api_key.clone(),
            self.model_name.clone(),
            self.provider_type.clone(),
            self.custom_headers.clone(),
            self.enable_tool_call,
        );
        let result = parent.process_streaming_response(response).await?;
        self.apply_token_counts(result.token_counts.clone());
        Ok(result)
    }
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

fn process_streaming_line(
    line: &str,
    chunks: &mut Vec<String>,
    token_counts: &mut TokenCounts,
) -> Result<(), AiServiceError> {
    if !line.starts_with("data:") {
        return Ok(());
    }

    let data = line.trim_start_matches("data:").trim();
    if data == "[DONE]" {
        return Ok(());
    }

    let json_response: Value =
        serde_json::from_str(data).map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
    if let Some(usage) = json_response.get("usage") {
        *token_counts = parse_usage_counts(usage);
    }
    if let Some(reasoning) = extract_reasoning_chunk(&json_response) {
        if !reasoning.is_empty() {
            chunks.push(format!("<think>{}</think>", reasoning));
        }
    }
    if let Some(content) = extract_content_chunk(&json_response) {
        if !content.is_empty() {
            chunks.push(StructuredToolCallBridge::convertToolCallPayloadToXml(&content));
        }
    }
    chunks.extend(extract_tool_calls_xml_chunks(&json_response));
    Ok(())
}

fn parse_usage_counts(usage: &Value) -> TokenCounts {
    let prompt_tokens = usage
        .get("prompt_tokens")
        .or_else(|| usage.get("input_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    let cached_tokens = usage
        .pointer("/prompt_tokens_details/cached_tokens")
        .or_else(|| usage.pointer("/input_tokens_details/cached_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    let completion_tokens = usage
        .get("completion_tokens")
        .or_else(|| usage.get("output_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;

    TokenCounts {
        input: prompt_tokens,
        cached_input: cached_tokens,
        output: completion_tokens,
    }
}

fn extract_content_chunk(value: &Value) -> Option<String> {
    value
        .pointer("/choices/0/delta/content")
        .or_else(|| value.pointer("/choices/0/message/content"))
        .or_else(|| value.pointer("/choices/0/text"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn extract_reasoning_chunk(value: &Value) -> Option<String> {
    value
        .pointer("/choices/0/delta/reasoning_content")
        .or_else(|| value.pointer("/choices/0/message/reasoning_content"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn extract_tool_calls_xml_chunks(value: &Value) -> Vec<String> {
    let Some(tool_calls) = value
        .pointer("/choices/0/message/tool_calls")
        .or_else(|| value.pointer("/choices/0/delta/tool_calls"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    tool_calls
        .iter()
        .map(|tool_call| StructuredToolCallBridge::convertToolCallPayloadToXml(&tool_call.to_string()))
        .filter(|content| crate::util::ChatMarkupRegex::ChatMarkupRegex::contains_tool_tag(content))
        .collect()
}
