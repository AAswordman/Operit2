use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Map, Value};
use std::sync::{Arc, Mutex};

use super::AIService::{
    response_stream_from_chunks, AIService, AiServiceError, SendMessageRequest, TokenCounts,
};
use super::OpenAIProvider::OpenAIProvider;
use super::StructuredToolCallBridge::StructuredToolCallBridge;
use crate::core::chat::hooks::PromptTurn::{PromptTurn, PromptTurnKind};
use crate::data::model::ModelParameter::ModelParameter;
use crate::data::model::ModelParameter::ParameterValueType;
use crate::data::model::ToolPrompt::ToolPrompt;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::util::stream::RevisableTextStream::{
    with_event_channel, RevisableTextStreamLike, TextStreamEventCarrier,
};
use crate::util::stream::Stream::FnStream;
use crate::util::ChatUtils::ChatUtils;
use crate::util::TokenCacheManager::TokenCacheManager;

#[derive(Clone)]
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
    state: Arc<Mutex<DeepseekProviderState>>,
}

#[derive(Debug, Default)]
struct DeepseekProviderState {
    inputTokenCount: i32,
    cachedInputTokenCount: i32,
    outputTokenCount: i32,
    cancelled: bool,
    tokenCacheManager: TokenCacheManager,
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
            state: Arc::new(Mutex::new(DeepseekProviderState::default())),
        }
    }

    fn apply_token_counts(&self, token_counts: TokenCounts) {
        if let Ok(mut state) = self.state.lock() {
            if token_counts.input > 0 || token_counts.cached_input > 0 {
                state.tokenCacheManager.update_actual_tokens(
                    token_counts.input.max(0) as usize,
                    token_counts.cached_input.max(0) as usize,
                );
            }
            if token_counts.output > 0 {
                state
                    .tokenCacheManager
                    .set_output_tokens(token_counts.output.max(0) as usize);
            }
            state.inputTokenCount = state.tokenCacheManager.total_input_token_count() as i32;
            state.cachedInputTokenCount = state.tokenCacheManager.cached_input_token_count() as i32;
            state.outputTokenCount = state.tokenCacheManager.output_token_count() as i32;
        }
    }

    pub fn create_request_body(
        &self,
        request: &SendMessageRequest,
    ) -> Result<Value, AiServiceError> {
        let mut json_object = Map::new();
        let effectiveEnableToolCall = self.enable_tool_call && !request.available_tools.is_empty();
        json_object.insert("model".to_string(), json!(self.model_name));
        json_object.insert(
            "messages".to_string(),
            self.build_messages_with_reasoning(
                &StructuredToolCallBridge::compileHistoryForProvider(
                    &request.chat_history,
                    effectiveEnableToolCall,
                ),
                effectiveEnableToolCall,
            )?,
        );
        json_object.insert("stream".to_string(), json!(request.stream));
        json_object.insert(
            "thinking".to_string(),
            json!({
                "type": if request.enable_thinking { "enabled" } else { "disabled" }
            }),
        );
        if request.enable_thinking && !json_object.contains_key("reasoning_effort") {
            if let Some(effort) = self.resolve_deepseek_thinking_effort() {
                json_object.insert("reasoning_effort".to_string(), json!(effort));
            }
        }

        self.apply_model_parameters(&mut json_object, &request.model_parameters);

        if effectiveEnableToolCall {
            let tools = StructuredToolCallBridge::buildToolsArray(Some(&request.available_tools));
            let toolsJson = tools.to_string();
            self.calculate_and_store_input_tokens(
                &StructuredToolCallBridge::compileHistoryForProvider(
                    &request.chat_history,
                    effectiveEnableToolCall,
                ),
                Some(&toolsJson),
                true,
            );
            json_object.insert("tools".to_string(), tools);
            json_object.insert("tool_choice".to_string(), json!("auto"));
        } else {
            self.calculate_and_store_input_tokens(
                &StructuredToolCallBridge::compileHistoryForProvider(
                    &request.chat_history,
                    effectiveEnableToolCall,
                ),
                None,
                true,
            );
        }

        Ok(Value::Object(json_object))
    }

    pub fn build_messages_with_reasoning(
        &self,
        effectiveHistory: &[PromptTurn],
        useToolCall: bool,
    ) -> Result<Value, AiServiceError> {
        let structuredMessages: Value =
            serde_json::from_str(&StructuredToolCallBridge::buildMessagesJsonForProvider(
                effectiveHistory,
                true,
                useToolCall,
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

    pub fn resolve_deepseek_thinking_effort(&self) -> Option<&'static str> {
        let qualityLevel = ApiPreferences::getInstance()
            .thinkingQualityLevelFlow()
            .first()
            .ok()?;
        match qualityLevel.clamp(1, 4) {
            1 | 2 => Some("high"),
            3 | 4 => Some("max"),
            _ => None,
        }
    }

    fn calculate_and_store_input_tokens(
        &self,
        provider_ready_history: &[PromptTurn],
        tools_json: Option<&str>,
        preserve_think_in_history: bool,
    ) -> i32 {
        let comparableHistory = provider_ready_history
            .iter()
            .map(|turn| {
                let role = match turn.kind {
                    PromptTurnKind::SYSTEM => "system",
                    PromptTurnKind::USER => "user",
                    PromptTurnKind::ASSISTANT => "assistant",
                    PromptTurnKind::TOOL_CALL => "tool_call",
                    PromptTurnKind::TOOL_RESULT => "tool_result",
                    PromptTurnKind::SUMMARY => "summary",
                }
                .to_string();
                let content =
                    if !preserve_think_in_history && turn.kind == PromptTurnKind::ASSISTANT {
                        ChatUtils::remove_thinking_content(&turn.content)
                    } else {
                        turn.content.clone()
                    };
                (role, content)
            })
            .collect::<Vec<_>>();
        if let Ok(mut state) = self.state.lock() {
            let tokenCount = state.tokenCacheManager.calculate_input_tokens(
                &comparableHistory,
                tools_json,
                true,
            );
            state.inputTokenCount = state.tokenCacheManager.total_input_token_count() as i32;
            state.cachedInputTokenCount = state.tokenCacheManager.cached_input_token_count() as i32;
            tokenCount as i32
        } else {
            0
        }
    }

    fn apply_model_parameters(
        &self,
        json_object: &mut Map<String, Value>,
        parameters: &[ModelParameter<Value>],
    ) {
        for parameter in parameters {
            if parameter.isEnabled {
                let value = match parameter.valueType {
                    ParameterValueType::INT => {
                        let Some(number) = parameter.currentValue.as_i64() else {
                            continue;
                        };
                        json!(number)
                    }
                    ParameterValueType::FLOAT => {
                        let Some(number) = parameter.currentValue.as_f64() else {
                            continue;
                        };
                        json!(number)
                    }
                    ParameterValueType::STRING => {
                        let Some(text) = parameter.currentValue.as_str() else {
                            continue;
                        };
                        json!(text)
                    }
                    ParameterValueType::BOOLEAN => {
                        let Some(value) = parameter.currentValue.as_bool() else {
                            continue;
                        };
                        json!(value)
                    }
                    ParameterValueType::OBJECT => {
                        if parameter.currentValue.is_object() || parameter.currentValue.is_array() {
                            parameter.currentValue.clone()
                        } else if let Some(raw) = parameter.currentValue.as_str() {
                            let trimmed = raw.trim();
                            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                                serde_json::from_str(trimmed).unwrap_or_else(|_| json!(trimmed))
                            } else {
                                json!(trimmed)
                            }
                        } else {
                            parameter.currentValue.clone()
                        }
                    }
                };
                json_object.insert(parameter.apiName.clone(), value);
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
            let header_value = HeaderValue::from_str(value)
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            headers.insert(header_name, header_value);
        }
        Ok(headers)
    }
}

#[async_trait]
impl AIService for DeepseekProvider {
    fn input_token_count(&self) -> i32 {
        self.state
            .lock()
            .map(|state| state.tokenCacheManager.total_input_token_count() as i32)
            .unwrap_or(0)
    }

    fn cached_input_token_count(&self) -> i32 {
        self.state
            .lock()
            .map(|state| state.tokenCacheManager.cached_input_token_count() as i32)
            .unwrap_or(0)
    }

    fn output_token_count(&self) -> i32 {
        self.state
            .lock()
            .map(|state| state.tokenCacheManager.output_token_count() as i32)
            .unwrap_or(0)
    }

    fn provider_model(&self) -> String {
        format!("{}:{}", self.provider_type, self.model_name)
    }

    fn reset_token_counts(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.inputTokenCount = 0;
            state.cachedInputTokenCount = 0;
            state.outputTokenCount = 0;
            state.tokenCacheManager.reset_token_counts();
        }
    }

    fn cancel_streaming(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.cancelled = true;
        }
    }

    async fn send_message(
        &mut self,
        request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        if let Ok(mut state) = self.state.lock() {
            state.cancelled = false;
        }
        self.reset_token_counts();

        let request_body = self.create_request_body(&request)?;
        if request.stream {
            let mut parent = OpenAIProvider::new_with_capabilities(
                self.api_endpoint.clone(),
                self.api_key.clone(),
                self.model_name.clone(),
                self.provider_type.clone(),
                self.custom_headers.clone(),
                self.supports_vision,
                self.supports_audio,
                self.supports_video,
                self.enable_tool_call,
            );
            let mut result = parent.send_prepared_request(request, request_body).await?;
            let event_channel = result.event_channel().clone();
            let mut provider = self.clone();
            let cold_stream = FnStream::new(move |emit| {
                result.collect(&mut |content| {
                    emit(content);
                });
                provider.apply_token_counts(TokenCounts {
                    input: parent.input_token_count(),
                    cached_input: parent.cached_input_token_count(),
                    output: parent.output_token_count(),
                });
            });
            return Ok(Box::new(with_event_channel(cold_stream, event_channel)));
        }

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
            return Err(AiServiceError::RequestFailed(format!(
                "{status}: {message}"
            )));
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
                chunks.push(StructuredToolCallBridge::convertToolCallPayloadToXml(
                    &content,
                ));
            }
        }
        chunks.extend(extract_tool_calls_xml_chunks(&json_response));

        Ok(response_stream_from_chunks(chunks))
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
            Err(AiServiceError::ConnectionFailed(format!(
                "{status}: {body}"
            )))
        }
    }

    async fn calculate_input_tokens(
        &self,
        chat_history: &[PromptTurn],
        available_tools: &[ToolPrompt],
    ) -> Result<i32, AiServiceError> {
        let useToolCall = self.enable_tool_call && !available_tools.is_empty();
        let providerReadyHistory =
            StructuredToolCallBridge::compileHistoryForProvider(chat_history, useToolCall);
        let toolsJson = if available_tools.is_empty() {
            None
        } else {
            Some(StructuredToolCallBridge::buildToolsArray(Some(available_tools)).to_string())
        };
        Ok(
            self.calculate_and_store_input_tokens(
                &providerReadyHistory,
                toolsJson.as_deref(),
                true,
            ),
        )
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

    let json_response: Value = serde_json::from_str(data)
        .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
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
            chunks.push(StructuredToolCallBridge::convertToolCallPayloadToXml(
                &content,
            ));
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
    let actual_input_tokens = (prompt_tokens - cached_tokens).max(0);

    TokenCounts {
        input: actual_input_tokens,
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
        .map(|tool_call| {
            StructuredToolCallBridge::convertToolCallPayloadToXml(&tool_call.to_string())
        })
        .filter(|content| crate::util::ChatMarkupRegex::ChatMarkupRegex::contains_tool_tag(content))
        .collect()
}
