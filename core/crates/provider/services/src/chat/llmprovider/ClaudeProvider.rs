use async_trait::async_trait;
use futures_util::StreamExt;
use http::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use serde_json::{json, Map, Value};
use std::sync::{Arc, Mutex};

use super::OpenAIProvider::{StreamingJsonXmlConverter, StreamingJsonXmlEvent};
use super::OpenAIResponsesProvider::strip_responses_protocol_markup;
use super::StreamingResponseLines::SseDecoder;
use super::StructuredToolCallBridge::{OpenToolCall, StructuredToolCallBridge};
use super::ThinkingDialect::ThinkingDialect;
use crate::chat::llmprovider::AIService::{
    response_stream_from_chunks, retry_error_text, retry_message, AIService, AiServiceError,
    SendMessageRequest, SharedAiResponseStream, TokenCounts,
};
use crate::chat::llmprovider::LlmRetryPolicy::delay_retry_ms;
use crate::chat::llmprovider::MediaLinkParser::MediaLinkParser;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_model::ToolPrompt::ToolPrompt;
use operit_util::stream::RevisableTextStream::{
    empty_revisable_event_channel, with_event_channel_and_failure, RevisableTextStream,
    RevisableTextStreamLike, TerminalFailureState, TextStreamEvent, TextStreamEventType,
};
use operit_util::stream::Stream::FnStream;
use operit_util::ChatMarkupRegex::ChatMarkupRegex;
use operit_util::TokenCacheManager::TokenCacheManager;

#[derive(Clone)]
pub struct ClaudeProvider {
    pub api_endpoint: String,
    pub api_key: String,
    pub model_name: String,
    pub provider_type: String,
    pub enable_tool_call: bool,
    pub custom_headers: Vec<(String, String)>,
    enable_claude_1h_prompt_cache: bool,
    state: Arc<Mutex<ClaudeProviderState>>,
    cancel_signal: tokio::sync::watch::Sender<bool>,
}

#[derive(Debug, Default)]
struct ClaudeProviderState {
    inputTokenCount: i64,
    cachedInputTokenCount: i64,
    outputTokenCount: i64,
    cancelled: bool,
    tokenCacheManager: TokenCacheManager,
}

impl ClaudeProvider {
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
            enable_tool_call,
            custom_headers,
            enable_claude_1h_prompt_cache: false,
            state: Arc::new(Mutex::new(ClaudeProviderState::default())),
            cancel_signal: tokio::sync::watch::channel(false).0,
        }
    }

    /// Matches Kotlin enableClaude1hPromptCache; existing callers keep the default TTL.
    pub fn with_claude_1h_prompt_cache(mut self, enabled: bool) -> Self {
        self.enable_claude_1h_prompt_cache = enabled;
        self
    }

    fn set_cancelled(&self, cancelled: bool) {
        self.state
            .lock()
            .expect("ClaudeProvider state mutex poisoned")
            .cancelled = cancelled;
        self.cancel_signal.send_replace(cancelled);
    }

    fn is_cancelled(&self) -> bool {
        self.state
            .lock()
            .expect("ClaudeProvider state mutex poisoned")
            .cancelled
    }

    fn set_token_counts(&self, token_counts: TokenCounts) {
        let mut state = self
            .state
            .lock()
            .expect("ClaudeProvider state mutex poisoned");
        state
            .tokenCacheManager
            .update_actual_tokens(token_counts.input, token_counts.cached_input);
        state
            .tokenCacheManager
            .set_output_tokens(token_counts.output);
        state.inputTokenCount = state.tokenCacheManager.total_input_token_count();
        state.cachedInputTokenCount = state.tokenCacheManager.cached_input_token_count();
        state.outputTokenCount = state.tokenCacheManager.output_token_count();
    }

    /// Wakes blocked header and body reads immediately when cancellation is requested.
    async fn waitUntilCancelled(self) {
        let mut receiver = self.cancel_signal.subscribe();
        while !*receiver.borrow_and_update() {
            if receiver.changed().await.is_err() {
                break;
            }
        }
    }

    async fn sendHttpRequest(
        &self,
        request: super::ProviderHttp::RequestBuilder,
    ) -> Result<super::ProviderHttp::Response, AiServiceError> {
        if self.is_cancelled() {
            return Err(AiServiceError::RequestCancelled);
        }
        tokio::select! {
            response = request.send() => response,
            _ = self.clone().waitUntilCancelled() => Err(AiServiceError::RequestCancelled),
        }
    }

    async fn readResponseText(
        &self,
        response: super::ProviderHttp::Response,
    ) -> Result<String, AiServiceError> {
        if self.is_cancelled() {
            return Err(AiServiceError::RequestCancelled);
        }
        tokio::select! {
            text = response.text() => text,
            _ = self.clone().waitUntilCancelled() => Err(AiServiceError::RequestCancelled),
        }
    }

    async fn readResponseJson(
        &self,
        response: super::ProviderHttp::Response,
    ) -> Result<Value, AiServiceError> {
        if self.is_cancelled() {
            return Err(AiServiceError::RequestCancelled);
        }
        tokio::select! {
            json = response.json() => json,
            _ = self.clone().waitUntilCancelled() => Err(AiServiceError::RequestCancelled),
        }
    }

    async fn delayRetryOrCancel(&self, retryAttempt: i32) -> Result<(), AiServiceError> {
        if self.is_cancelled() {
            return Err(AiServiceError::RequestCancelled);
        }
        let delayMs = super::LlmRetryPolicy::LlmRetryPolicy::nextDelayMs(retryAttempt);
        tokio::select! {
            _ = defaultHostRuntimeTaskSchedulerHost().waitForHostRuntimeDelay(delayMs as u64) => Ok(()),
            _ = self.clone().waitUntilCancelled() => Err(AiServiceError::RequestCancelled),
        }
    }

    pub fn create_request_body(
        &self,
        request: &SendMessageRequest,
    ) -> Result<Value, AiServiceError> {
        let (system, messages) = self.build_serialized_history(&request.chat_history)?;
        let mut object = Map::new();
        object.insert("model".to_string(), json!(self.model_name));
        object.insert("messages".to_string(), Value::Array(messages));
        object.insert("stream".to_string(), json!(request.stream));
        if !system.is_null() {
            object.insert("system".to_string(), system);
        }
        if !object.contains_key("max_tokens") {
            object.insert("max_tokens".to_string(), json!(DEFAULT_MAX_TOKENS));
        }
        if self.enable_tool_call && !request.available_tools.is_empty() {
            object.insert(
                "tools".to_string(),
                self.build_tool_definitions_for_claude(&request.available_tools)?,
            );
        }
        let mut request_object = Value::Object(object);
        ThinkingDialect::Anthropic.apply(&mut request_object, &request.thinking)?;
        let object = request_object
            .as_object_mut()
            .expect("thinking request remains an object");
        self.add_parameters(object, &request.model_parameters);
        fit_max_tokens_to_thinking_budget(object, &request.model_parameters)?;
        self.apply_stable_cache_breakpoints(object);
        self.calculate_and_store_input_tokens(object, true);
        Ok(request_object)
    }

    pub fn build_messages_and_count_tokens(
        &self,
        chat_history: &[PromptTurn],
    ) -> Result<(Value, Vec<Value>), AiServiceError> {
        let mut object = self.build_cache_request_parts(chat_history, &[])?;
        self.calculate_and_store_input_tokens(&object, true);
        let system = object.remove("system").unwrap_or(Value::Null);
        let messages = object
            .remove("messages")
            .and_then(|value| match value {
                Value::Array(messages) => Some(messages),
                _ => None,
            })
            .unwrap_or_default();
        Ok((system, messages))
    }

    /// Serializes the history before adding any cache policy or counting tokens.
    fn build_serialized_history(
        &self,
        chat_history: &[PromptTurn],
    ) -> Result<(Value, Vec<Value>), AiServiceError> {
        let mut system_parts = Vec::new();
        let mut messages = Vec::new();
        let provider_ready_history = StructuredToolCallBridge::compileHistoryForProvider(
            chat_history,
            self.enable_tool_call,
        );
        let mut next_tool_use_ordinal = 0usize;
        let mut open_tool_calls: Vec<OpenToolCall> = Vec::new();
        for turn in provider_ready_history {
            if turn.kind != PromptTurnKind::TOOL_RESULT && !open_tool_calls.is_empty() {
                let content =
                    Self::missing_tool_result_blocks(&mut open_tool_calls, "assistant_boundary");
                messages.push(json!({"role": "user", "content": content}));
            }
            match turn.kind {
                PromptTurnKind::SYSTEM => {
                    system_parts.push(strip_responses_protocol_markup(&turn.content))
                }
                PromptTurnKind::SUMMARY => messages.push(json!({
                    "role": "user",
                    "content": self.build_content_array(&turn.content),
                })),
                PromptTurnKind::USER => messages.push(
                    json!({"role": "user", "content": self.build_content_array(&turn.content)}),
                ),
                PromptTurnKind::ASSISTANT | PromptTurnKind::TOOL_CALL => {
                    let content = if self.enable_tool_call {
                        self.build_assistant_content_blocks(
                            &turn.content,
                            &mut next_tool_use_ordinal,
                            &mut open_tool_calls,
                        )?
                    } else {
                        self.build_content_array(&turn.content)
                    };
                    messages.push(json!({"role": "assistant", "content": content}))
                }
                PromptTurnKind::TOOL_RESULT => {
                    let content = if self.enable_tool_call {
                        self.build_tool_result_blocks(&turn.content, &mut open_tool_calls)
                    } else {
                        self.build_content_array(&turn.content)
                    };
                    messages.push(json!({"role": "user", "content": content}))
                }
            }
        }
        if !open_tool_calls.is_empty() {
            let content = Self::missing_tool_result_blocks(&mut open_tool_calls, "history_end");
            messages.push(json!({"role": "user", "content": content}));
        }
        let system_prompt = system_parts.join("\n\n");
        let system = if system_prompt.trim().is_empty() {
            Value::Null
        } else {
            json!([{"type": "text", "text": system_prompt}])
        };
        Ok((system, messages))
    }

    fn cache_control_object(&self) -> Value {
        if self.enable_claude_1h_prompt_cache {
            json!({"type": "ephemeral", "ttl": "1h"})
        } else {
            json!({"type": "ephemeral"})
        }
    }

    /// Adds the same three stable breakpoints as Kotlin, without replacing caller policies.
    pub fn apply_stable_cache_breakpoints(&self, request_object: &mut Map<String, Value>) {
        for field in ["tools", "system"] {
            if let Some(block) = request_object
                .get_mut(field)
                .and_then(Value::as_array_mut)
                .and_then(|blocks| blocks.last_mut())
                .and_then(Value::as_object_mut)
            {
                block
                    .entry("cache_control")
                    .or_insert_with(|| self.cache_control_object());
            }
        }
        if let Some(messages) = request_object
            .get_mut("messages")
            .and_then(Value::as_array_mut)
        {
            for message in messages.iter_mut().rev() {
                let Some(content) = message.get_mut("content").and_then(Value::as_array_mut) else {
                    continue;
                };
                if let Some(block) = content.iter_mut().rev().find_map(Value::as_object_mut) {
                    block
                        .entry("cache_control")
                        .or_insert_with(|| self.cache_control_object());
                    break;
                }
            }
        }
    }

    /// Uses identical serialized content for send-time estimates and read-only preflight.
    fn build_cache_request_parts(
        &self,
        history: &[PromptTurn],
        tools: &[ToolPrompt],
    ) -> Result<Map<String, Value>, AiServiceError> {
        let (system, messages) = self.build_serialized_history(history)?;
        let mut object = Map::new();
        if !system.is_null() {
            object.insert("system".to_string(), system);
        }
        object.insert("messages".to_string(), Value::Array(messages));
        if self.enable_tool_call && !tools.is_empty() {
            object.insert(
                "tools".to_string(),
                self.build_tool_definitions_for_claude(tools)?,
            );
        }
        self.apply_stable_cache_breakpoints(&mut object);
        Ok(object)
    }

    fn calculate_and_store_input_tokens(
        &self,
        object: &Map<String, Value>,
        update_state: bool,
    ) -> i64 {
        let mut history = Vec::new();
        if let Some(system) = object
            .get("system")
            .filter(|value| value.as_array().is_some_and(|array| !array.is_empty()))
        {
            history.push(("system".to_string(), stable_json_value(system)));
        }
        if let Some(messages) = object.get("messages").and_then(Value::as_array) {
            for message in messages {
                let Some(role) = message
                    .get("role")
                    .and_then(Value::as_str)
                    .filter(|role| !role.is_empty())
                else {
                    continue;
                };
                let content = message
                    .get("content")
                    .filter(|value| value.is_array())
                    .cloned()
                    .unwrap_or_else(|| json!([]));
                history.push((role.to_string(), stable_json_value(&content)));
            }
        }
        let tools = object
            .get("tools")
            .filter(|value| value.as_array().is_some_and(|array| !array.is_empty()))
            .map(stable_json_value);
        let mut state = self
            .state
            .lock()
            .expect("ClaudeProvider state mutex poisoned");
        let count = state.tokenCacheManager.calculate_input_tokens(
            &history,
            tools.as_deref(),
            update_state,
        );
        if update_state {
            state.inputTokenCount = state.tokenCacheManager.total_input_token_count();
            state.cachedInputTokenCount = state.tokenCacheManager.cached_input_token_count();
        }
        count
    }

    fn build_tool_definitions_for_claude(
        &self,
        tool_prompts: &[ToolPrompt],
    ) -> Result<Value, AiServiceError> {
        let tools = tool_prompts
            .iter()
            .map(|tool| {
                let schema = serde_json::from_str::<Value>(&tool.parameters)
                    .unwrap_or_else(|_| json!({"type": "object", "properties": {}}));
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": schema,
                })
            })
            .collect();
        Ok(Value::Array(tools))
    }

    /// Builds Claude content blocks from text and registered image links.
    fn build_content_array(&self, text: &str) -> Value {
        let text_after_media_removal = if MediaLinkParser::has_media_links(text) {
            MediaLinkParser::remove_media_links(text).trim().to_string()
        } else {
            text.to_string()
        };

        if !MediaLinkParser::has_image_links(&text_after_media_removal) {
            return json!([{"type": "text", "text": text_after_media_removal}]);
        }

        let image_links = MediaLinkParser::extract_image_links(&text_after_media_removal);
        let text_without_links = MediaLinkParser::remove_image_links(&text_after_media_removal)
            .trim()
            .to_string();
        let mut content_blocks = Vec::new();
        for link in image_links {
            content_blocks.push(json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": link.mime_type,
                    "data": link.base64_data,
                },
            }));
        }
        if !text_without_links.is_empty() {
            content_blocks.push(json!({"type": "text", "text": text_without_links}));
        }
        if content_blocks.is_empty() {
            content_blocks.push(json!({"type": "text", "text": "[Empty]"}));
        }
        Value::Array(content_blocks)
    }

    fn build_assistant_content_blocks(
        &self,
        content: &str,
        next_tool_use_ordinal: &mut usize,
        open_tool_calls: &mut Vec<OpenToolCall>,
    ) -> Result<Value, AiServiceError> {
        let (text_content, calls) = StructuredToolCallBridge::parseXmlToolCalls(content);
        let Some(calls) = calls.filter(|calls| !calls.is_empty()) else {
            return Ok(self.build_content_array(content));
        };
        let mut blocks = Vec::new();
        if !text_content.trim().is_empty() {
            blocks.extend(
                self.build_content_array(&text_content)
                    .as_array()
                    .unwrap()
                    .clone(),
            );
        }
        for call in calls {
            let function = &call["function"];
            let input: Value = serde_json::from_str(function["arguments"].as_str().unwrap_or("{}"))
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            let call_id = generated_tool_use_id(*next_tool_use_ordinal);
            *next_tool_use_ordinal += 1;
            open_tool_calls.push(OpenToolCall {
                id: call_id.clone(),
                function_name: function["name"].as_str().unwrap_or_default().to_string(),
                matching_name: StructuredToolCallBridge::toolCallName(&call),
            });
            blocks.push(json!({
                "type": "tool_use",
                "id": call_id,
                "name": function["name"],
                "input": input,
            }));
        }
        Ok(Value::Array(blocks))
    }

    /// Keeps every missing tool response attached to its original invocation ID.
    fn missing_tool_result_blocks(
        open_tool_calls: &mut Vec<OpenToolCall>,
        reason: &str,
    ) -> Vec<Value> {
        open_tool_calls.drain(..).map(|call| json!({
            "type": "tool_result",
            "tool_use_id": call.id,
            "is_error": true,
            "content": StructuredToolCallBridge::unmatchedToolResultContent(reason, &call.matching_name),
        })).collect()
    }

    fn build_tool_result_blocks(
        &self,
        content: &str,
        open_tool_calls: &mut Vec<OpenToolCall>,
    ) -> Value {
        let blocks = ChatMarkupRegex::tool_result_blocks(content);
        let result_names = blocks
            .iter()
            .map(|block| operit_util::ChatMarkupRegex::attr_value(&block.raw, "name"))
            .collect::<Vec<_>>();
        let matched =
            StructuredToolCallBridge::consumeMatchingToolCalls(open_tool_calls, &result_names);
        let mut text_content = content.to_string();
        let mut result_blocks = Vec::new();
        for matched_call in matched {
            let block = &blocks[matched_call.result_index];
            let result_content = operit_util::ChatMarkupRegex::tag_ranges(&block.body, "content")
                .into_iter()
                .next()
                .and_then(|(start, end)| {
                    let raw = &block.body[start..end];
                    raw.split_once('>').and_then(|(_, tail)| {
                        tail.rsplit_once("</")
                            .map(|(body, _)| body.trim().to_string())
                    })
                })
                .unwrap_or_else(|| block.body.trim().to_string());
            result_blocks.push(json!({
                "type": "tool_result",
                "tool_use_id": matched_call.call.id,
                "content": self.build_content_array(&result_content),
            }));
            // Unmatched markup remains text rather than becoming an orphan tool_result.
            text_content = text_content.replacen(&block.raw, "", 1);
        }
        result_blocks.extend(Self::missing_tool_result_blocks(
            open_tool_calls,
            "tool_result_partial_batch",
        ));
        if !text_content.trim().is_empty() {
            result_blocks.extend(
                self.build_content_array(text_content.trim())
                    .as_array()
                    .unwrap()
                    .clone(),
            );
        }
        if result_blocks.is_empty() {
            return self.build_content_array(content);
        }
        Value::Array(result_blocks)
    }

    fn add_parameters(
        &self,
        json_object: &mut Map<String, Value>,
        parameters: &[operit_model::ModelParameter::ModelParameter<Value>],
    ) {
        for parameter in parameters {
            if !parameter.isEnabled {
                continue;
            }
            let api_name = if parameter.apiName == "max_tokens" {
                "max_tokens"
            } else {
                parameter.apiName.as_str()
            };
            json_object.insert(api_name.to_string(), parameter.currentValue.clone());
        }
    }

    fn headers(&self) -> Result<HeaderMap, AiServiceError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        if !self.api_key.trim().is_empty() {
            headers.insert(
                "x-api-key",
                HeaderValue::from_str(&self.api_key)
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?,
            );
        }
        for (name, value) in &self.custom_headers {
            headers.insert(
                HeaderName::from_bytes(name.as_bytes())
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?,
                HeaderValue::from_str(value)
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?,
            );
        }
        Ok(headers)
    }

    /// Streaming usage is a partial update: omitted fields retain prior values,
    /// while explicitly reported zeroes replace them (KT ProviderUsageNormalizer).
    fn apply_streaming_usage(&mut self, usage: &Value, accumulated: &mut Map<String, Value>) {
        self.apply_streaming_usage_with_format(usage, accumulated, false);
    }

    fn apply_streaming_usage_with_format(
        &mut self,
        usage: &Value,
        accumulated: &mut Map<String, Value>,
        open_ai_compatible: bool,
    ) {
        if let Some(fields) = usage.as_object() {
            accumulated.extend(fields.clone());
            self.apply_usage_with_format(
                Some(&Value::Object(accumulated.clone())),
                open_ai_compatible,
            );
        }
    }

    fn apply_usage(&mut self, usage: Option<&Value>) -> TokenCounts {
        self.apply_usage_with_format(usage, false)
    }

    fn apply_usage_with_format(
        &mut self,
        usage: Option<&Value>,
        open_ai_compatible: bool,
    ) -> TokenCounts {
        let has_usage = usage.is_some_and(|value| {
            [
                "input_tokens",
                "prompt_tokens",
                "cache_read_input_tokens",
                "cached_tokens",
                "cache_creation_input_tokens",
                "cache_creation",
                "output_tokens",
                "completion_tokens",
            ]
            .iter()
            .any(|field| value.get(*field).is_some())
        });
        if !has_usage {
            let state = self
                .state
                .lock()
                .expect("ClaudeProvider state mutex poisoned");
            return TokenCounts {
                input: state
                    .inputTokenCount
                    .saturating_sub(state.cachedInputTokenCount),
                cached_input: state.cachedInputTokenCount,
                output: state.outputTokenCount,
            };
        }
        let cached_input = usage
            .and_then(|value| {
                value
                    .get("cache_read_input_tokens")
                    .or_else(|| value.pointer("/prompt_tokens_details/cached_tokens"))
                    .or_else(|| value.pointer("/input_tokens_details/cached_tokens"))
                    .or_else(|| value.get("cached_tokens"))
            })
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0);
        let cache_creation = usage
            .and_then(|value| {
                value
                    .get("cache_creation_input_tokens")
                    .and_then(Value::as_i64)
                    .or_else(|| {
                        value
                            .get("cache_creation")
                            .and_then(Value::as_object)
                            .map(|fields| {
                                fields
                                    .values()
                                    .filter_map(Value::as_i64)
                                    .fold(0i64, i64::saturating_add)
                            })
                    })
            })
            .unwrap_or(0)
            .max(0);
        // Anthropic input_tokens excludes cache reads/creation; OpenAI's
        // prompt_tokens includes cached tokens. Match KT's two usage parsers.
        let input_base = usage
            .and_then(|value| {
                if open_ai_compatible {
                    return value
                        .get("prompt_tokens")
                        .or_else(|| value.get("input_tokens"))
                        .and_then(Value::as_i64)
                        .map(|total| total.saturating_sub(cached_input));
                }
                value
                    .get("input_tokens")
                    .and_then(Value::as_i64)
                    .or_else(|| {
                        value
                            .get("prompt_tokens")
                            .and_then(Value::as_i64)
                            .map(|total| total.saturating_sub(cached_input))
                    })
            })
            .unwrap_or(0)
            .max(0);
        let output = usage
            .and_then(|value| {
                value
                    .get("output_tokens")
                    .or_else(|| value.get("completion_tokens"))
            })
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0);
        let token_counts = TokenCounts {
            input: input_base.saturating_add(if open_ai_compatible {
                0
            } else {
                cache_creation
            }),
            cached_input,
            output,
        };
        self.set_token_counts(token_counts.clone());
        token_counts
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl AIService for ClaudeProvider {
    fn input_token_count(&self) -> i64 {
        self.state
            .lock()
            .expect("ClaudeProvider state mutex poisoned")
            .inputTokenCount
    }
    fn cached_input_token_count(&self) -> i64 {
        self.state
            .lock()
            .expect("ClaudeProvider state mutex poisoned")
            .cachedInputTokenCount
    }
    fn output_token_count(&self) -> i64 {
        self.state
            .lock()
            .expect("ClaudeProvider state mutex poisoned")
            .outputTokenCount
    }
    fn provider_model(&self) -> String {
        format!("{}:{}", self.provider_type, self.model_name)
    }
    fn reset_token_counts(&mut self) {
        let mut state = self
            .state
            .lock()
            .expect("ClaudeProvider state mutex poisoned");
        state.tokenCacheManager.reset_token_counts();
        state.inputTokenCount = 0;
        state.cachedInputTokenCount = 0;
        state.outputTokenCount = 0;
    }
    fn cancel_streaming(&mut self) {
        self.set_cancelled(true);
    }

    /// Sends a Claude request while preserving producer failures through the cold wrapper.
    async fn send_message(
        &mut self,
        request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        self.set_cancelled(false);
        self.reset_token_counts();
        if request.stream {
            let output = SharedAiResponseStream::new_ordered(
                operit_util::stream::HotStream::mutable_shared_stream(usize::MAX),
                operit_util::stream::HotStream::mutable_shared_stream(usize::MAX),
            );
            let producer = output.clone();
            let mut provider = self.clone();
            defaultHostRuntimeTaskSchedulerHost()
                .scheduleHostRuntimeAsyncTask(
                    "claude-response-stream",
                    Box::new(move || {
                        Box::pin(async move {
                            match provider.send_streaming_message(request, &producer).await {
                                Ok(()) | Err(AiServiceError::RequestCancelled) => {}
                                Err(error) => producer.set_terminal_failure(error.to_string()),
                            }
                            producer.close();
                        })
                    }),
                )
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            return Ok(Box::new(output));
        }
        let event_channel = empty_revisable_event_channel();
        let streamEventChannel = event_channel.clone();
        let terminalFailure = TerminalFailureState::new();
        let producerTerminalFailure = terminalFailure.clone();
        let mut ownedProvider = Some(self.clone());
        let mut ownedRequest = Some(request);
        let coldStream = FnStream::new(move |emit| {
            let request = ownedRequest
                .take()
                .expect("ClaudeProvider non-stream request must only be collected once");
            let mut provider = ownedProvider
                .take()
                .expect("ClaudeProvider non-stream request must only be collected once");
            let eventChannel = streamEventChannel.clone();
            let producerTerminalFailure = producerTerminalFailure.clone();
            Box::pin(async move {
                match provider.send_non_streaming_message(request).await {
                    Ok(mut responseStream) => {
                        responseStream.collect(emit).await;
                        if let Some(error) = responseStream.terminal_failure() {
                            producerTerminalFailure.set(error);
                        }
                    }
                    Err(AiServiceError::RequestCancelled) => {}
                    Err(error) => producerTerminalFailure.set(error.to_string()),
                }
                eventChannel.close();
            })
        });
        Ok(Box::new(with_event_channel_and_failure(
            coldStream,
            event_channel,
            terminalFailure,
        )))
    }

    async fn calculate_input_tokens(
        &self,
        chat_history: &[PromptTurn],
        available_tools: &[ToolPrompt],
    ) -> Result<i64, AiServiceError> {
        let object = self.build_cache_request_parts(chat_history, available_tools)?;
        Ok(self.calculate_and_store_input_tokens(&object, false))
    }
}

impl ClaudeProvider {
    /// Publishes each event immediately and rolls back only when another attempt is accepted.
    async fn send_streaming_message(
        &mut self,
        request: SendMessageRequest,
        output: &SharedAiResponseStream,
    ) -> Result<(), AiServiceError> {
        let savepoint = uuid::Uuid::new_v4().to_string();
        output.emit_revision(TextStreamEvent {
            event_type: TextStreamEventType::Savepoint,
            id: savepoint.clone(),
        });
        let mut retry_count = 0;
        loop {
            let request_body = self.create_request_body(&request)?;
            let result = async {
                let response = self
                    .sendHttpRequest(
                        super::ProviderHttp::post(&self.api_endpoint)
                            .headers(self.headers()?)
                            .json(&request_body),
                    )
                    .await?;
                if !response.status().is_success() {
                    let status = response.status();
                    return Err(AiServiceError::RequestFailed(format!(
                        "{status}: {}",
                        self.readResponseText(response).await?
                    )));
                }
                self.process_streaming_response(response, output).await
            }
            .await;
            match result {
                Ok(()) => return Ok(()),
                Err(AiServiceError::RequestCancelled) => {
                    return Err(AiServiceError::RequestCancelled)
                }
                Err(error) => {
                    if !request.enable_retry
                        || retry_count >= super::LlmRetryPolicy::LlmRetryPolicy::MAX_RETRY_ATTEMPTS
                    {
                        return Err(error);
                    }
                    retry_count += 1;
                    output.emit_revision(TextStreamEvent {
                        event_type: TextStreamEventType::Rollback,
                        id: savepoint.clone(),
                    });
                    self.reset_token_counts();
                    if let Some(callback) = &request.on_non_fatal_error {
                        callback(retry_message(&retry_error_text(&error), retry_count));
                    }
                    self.delayRetryOrCancel(retry_count).await?;
                }
            }
        }
    }

    async fn send_non_streaming_message(
        &mut self,
        request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        let maxRetries = super::LlmRetryPolicy::LlmRetryPolicy::MAX_RETRY_ATTEMPTS;
        let mut retryCount = 0;
        loop {
            let request_body = self.create_request_body(&request)?;
            let response = match self
                .sendHttpRequest(
                    super::ProviderHttp::post(&self.api_endpoint)
                        .headers(self.headers()?)
                        .json(&request_body),
                )
                .await
            {
                Ok(response) => response,
                Err(AiServiceError::RequestCancelled) => {
                    return Err(AiServiceError::RequestCancelled);
                }
                Err(error) => {
                    let errorText = retry_error_text(&error);
                    if !request.enable_retry {
                        return Err(error);
                    }
                    let newRetryCount = retryCount + 1;
                    if newRetryCount > maxRetries {
                        return Err(error);
                    }
                    if let Some(on_non_fatal_error) = request.on_non_fatal_error.as_ref() {
                        on_non_fatal_error(retry_message(&errorText, newRetryCount));
                    }
                    self.delayRetryOrCancel(newRetryCount).await?;
                    retryCount = newRetryCount;
                    continue;
                }
            };
            let status = response.status();
            if !status.is_success() {
                let text = self.readResponseText(response).await?;
                let error = AiServiceError::RequestFailed(format!("{status}: {text}"));
                let errorText = retry_error_text(&error);
                if !request.enable_retry {
                    return Err(error);
                }
                let newRetryCount = retryCount + 1;
                if newRetryCount > maxRetries {
                    return Err(error);
                }
                if let Some(on_non_fatal_error) = request.on_non_fatal_error.as_ref() {
                    on_non_fatal_error(retry_message(&errorText, newRetryCount));
                }
                self.delayRetryOrCancel(newRetryCount).await?;
                retryCount = newRetryCount;
                continue;
            }

            let json_response = self.readResponseJson(response).await?;
            let token_counts = if json_response.get("choices").is_some() {
                self.apply_usage_with_format(json_response.get("usage"), true)
            } else {
                self.apply_usage(json_response.get("usage"))
            };
            let mut chunks = Vec::new();
            if let Some(content) = json_response.get("content").and_then(Value::as_array) {
                for part in content {
                    match part.get("type").and_then(Value::as_str).unwrap_or_default() {
                        "text" => {
                            if let Some(text) = part.get("text").and_then(Value::as_str) {
                                chunks.push(StructuredToolCallBridge::convertToolCallPayloadToXml(
                                    text,
                                ));
                            }
                        }
                        "tool_use" => {
                            chunks.push(StructuredToolCallBridge::convertToolCallPayloadToXml(
                                &part.to_string(),
                            ))
                        }
                        _ => {}
                    }
                }
            }
            let _ = token_counts;
            return Ok(response_stream_from_chunks(chunks));
        }
    }

    /// Emits complete SSE events while preserving an interrupted response as a failure.
    pub(super) async fn process_streaming_response(
        &mut self,
        response: super::ProviderHttp::Response,
        output: &SharedAiResponseStream,
    ) -> Result<(), AiServiceError> {
        if response
            .media_type()
            .eq_ignore_ascii_case("application/json")
        {
            let json: Value = self.readResponseJson(response).await?;
            if let Some(error) = json.get("error") {
                return Err(AiServiceError::RequestFailed(error.to_string()));
            }
            self.apply_usage_with_format(json.get("usage"), json.get("choices").is_some());
            output.emit_chunk(self.parse_anthropic_non_streaming(&json));
            return Ok(());
        }
        let mut chunks = Vec::new();
        let mut accumulated_usage = Map::new();
        let mut decoder = SseDecoder::default();
        let mut bytes_stream = response.bytes_stream();
        let mut current_tool_parser = None;
        let mut current_tool_tag_name = None;
        let mut is_in_tool_call = false;
        let mut is_in_thinking_block = false;
        let mut completed = false;
        let cancelled = self.clone().waitUntilCancelled();
        tokio::pin!(cancelled);
        loop {
            if self.is_cancelled() {
                return Err(AiServiceError::RequestCancelled);
            }
            let item = tokio::select! {
                item = bytes_stream.next() => item,
                _ = &mut cancelled => return Err(AiServiceError::RequestCancelled),
            };
            let Some(item) = item else {
                break;
            };
            for data in decoder.push(&item?)? {
                self.process_streaming_event(
                    &data,
                    &mut chunks,
                    &mut accumulated_usage,
                    &mut current_tool_parser,
                    &mut current_tool_tag_name,
                    &mut is_in_tool_call,
                    &mut is_in_thinking_block,
                    &mut completed,
                )?;
                for chunk in chunks.drain(..) {
                    output.emit_chunk(chunk);
                }
                if completed {
                    if let Some(mut parser) = current_tool_parser.take() {
                        append_converter_events(&mut chunks, parser.flush());
                    }
                    if let Some(tag) = current_tool_tag_name.take() {
                        chunks.push(format!("\n</{tag}>\n"));
                    }
                    if is_in_thinking_block {
                        chunks.push("</think>\n".to_string());
                    }
                    for chunk in chunks.drain(..) {
                        output.emit_chunk(chunk);
                    }
                    return Ok(());
                }
            }
        }
        decoder.finish()?;
        Err(AiServiceError::ConnectionFailed(
            "Claude response stream ended before completion".to_string(),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    /// Interprets one fully framed event and reports protocol completion explicitly.
    fn process_streaming_event(
        &mut self,
        data: &str,
        chunks: &mut Vec<String>,
        accumulated_usage: &mut Map<String, Value>,
        current_tool_parser: &mut Option<StreamingJsonXmlConverter>,
        current_tool_tag_name: &mut Option<String>,
        is_in_tool_call: &mut bool,
        is_in_thinking_block: &mut bool,
        completed: &mut bool,
    ) -> Result<(), AiServiceError> {
        if data.trim() == "[DONE]" {
            *completed = true;
            return Ok(());
        }
        if data.trim().is_empty() {
            return Ok(());
        }
        let json_response: Value = serde_json::from_str(data)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        if let Some(error) = json_response.get("error") {
            return Err(AiServiceError::RequestFailed(error.to_string()));
        }
        let event_type = json_response
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("");
        if event_type.is_empty() {
            if let Some(usage) = json_response.get("usage") {
                self.apply_streaming_usage_with_format(usage, accumulated_usage, true);
            }
            if let Some(content) = json_response
                .pointer("/choices/0/delta/content")
                .and_then(Value::as_str)
            {
                if !content.is_empty() {
                    chunks.push(content.to_string());
                }
            }
            return Ok(());
        }
        match event_type {
            "message_start" => {
                if let Some(usage) = json_response.pointer("/message/usage") {
                    self.apply_streaming_usage(usage, accumulated_usage);
                }
            }
            "content_block_start" => {
                let content_block = json_response.get("content_block").unwrap_or(&Value::Null);
                match content_block
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                {
                    "tool_use" if self.enable_tool_call => {
                        let tool_name = content_block
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        if !tool_name.is_empty() {
                            let tag = ChatMarkupRegex::generate_random_tool_tag_name();
                            *current_tool_tag_name = Some(tag.clone());
                            chunks.push(format!("\n<{tag} name=\"{tool_name}\">"));
                            *current_tool_parser = Some(StreamingJsonXmlConverter::new());
                            *is_in_tool_call = true;
                            if let Some(input) = content_block.get("input") {
                                if let Some(parser) = current_tool_parser.as_mut() {
                                    append_converter_events(
                                        chunks,
                                        parser.feed(&input.to_string()),
                                    );
                                }
                            }
                        }
                    }
                    "thinking" => {
                        chunks.push("\n<think>".to_string());
                        *is_in_thinking_block = true;
                        if let Some(thinking) =
                            content_block.get("thinking").and_then(Value::as_str)
                        {
                            if !thinking.is_empty() {
                                chunks.push(thinking.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            "content_block_delta" => {
                let delta = json_response.get("delta").unwrap_or(&Value::Null);
                let delta_type = delta.get("type").and_then(Value::as_str).unwrap_or("");
                if delta_type == "text_delta" || delta.get("text").is_some() {
                    if let Some(content) = delta.get("text").and_then(Value::as_str) {
                        if !content.is_empty() {
                            chunks.push(content.to_string());
                        }
                    }
                } else if *is_in_thinking_block
                    && (delta_type == "thinking_delta" || delta.get("thinking").is_some())
                {
                    if let Some(thinking) = delta.get("thinking").and_then(Value::as_str) {
                        if !thinking.is_empty() {
                            chunks.push(thinking.to_string());
                        }
                    }
                } else if self.enable_tool_call
                    && *is_in_tool_call
                    && delta_type == "input_json_delta"
                {
                    if let Some(partial_json) = delta.get("partial_json").and_then(Value::as_str) {
                        if let Some(parser) = current_tool_parser.as_mut() {
                            append_converter_events(chunks, parser.feed(partial_json));
                        }
                    }
                }
            }
            "content_block_stop" => {
                if *is_in_tool_call {
                    if let Some(parser) = current_tool_parser.as_mut() {
                        append_converter_events(chunks, parser.flush());
                    }
                    if let Some(tag) = current_tool_tag_name.take() {
                        chunks.push(format!("\n</{tag}>\n"));
                    }
                    *is_in_tool_call = false;
                    *current_tool_parser = None;
                } else if *is_in_thinking_block {
                    chunks.push("</think>\n".to_string());
                    *is_in_thinking_block = false;
                }
            }
            "message_delta" => {
                if let Some(usage) = json_response.get("usage") {
                    self.apply_streaming_usage(usage, accumulated_usage);
                }
            }
            "message_stop" => {
                *completed = true;
            }
            _ => {}
        }
        Ok(())
    }

    fn parse_anthropic_non_streaming(&self, json_response: &Value) -> String {
        let mut full_text = String::new();
        let Some(content) = json_response.get("content").and_then(Value::as_array) else {
            return json_response
                .pointer("/choices/0/message/content")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
        };
        for block in content {
            match block.get("type").and_then(Value::as_str).unwrap_or("") {
                "text" => {
                    if let Some(text) = block.get("text").and_then(Value::as_str) {
                        full_text.push_str(text);
                    }
                }
                "thinking" => {
                    if let Some(thinking) = block.get("thinking").and_then(Value::as_str) {
                        if !thinking.is_empty() {
                            full_text.push_str("\n<think>");
                            full_text.push_str(thinking);
                            full_text.push_str("</think>\n");
                        }
                    }
                }
                "tool_use" if self.enable_tool_call => {
                    let tool_name = block.get("name").and_then(Value::as_str).unwrap_or("");
                    if !tool_name.is_empty() {
                        let tag = ChatMarkupRegex::generate_random_tool_tag_name();
                        full_text.push_str(&format!("\n<{tag} name=\"{tool_name}\">"));
                        if let Some(input) = block.get("input") {
                            let mut parser = StreamingJsonXmlConverter::new();
                            for event in parser
                                .feed(&input.to_string())
                                .into_iter()
                                .chain(parser.flush())
                            {
                                match event {
                                    StreamingJsonXmlEvent::Tag(text)
                                    | StreamingJsonXmlEvent::Content(text) => {
                                        full_text.push_str(&text)
                                    }
                                }
                            }
                        }
                        full_text.push_str(&format!("\n</{tag}>\n"));
                    }
                }
                _ => {}
            }
        }
        full_text
    }
}

/// `max_tokens` a request gets when the user sets none; a thinking budget comes on top.
const DEFAULT_MAX_TOKENS: i64 = 4096;

/// Makes room for a thinking budget, which Anthropic counts inside `max_tokens` and which must
/// stay below it. The default grows to hold the budget; a `max_tokens` the user set does not.
fn fit_max_tokens_to_thinking_budget(
    object: &mut Map<String, Value>,
    parameters: &[operit_model::ModelParameter::ModelParameter<Value>],
) -> Result<(), AiServiceError> {
    let Some(budget) = object
        .get("thinking")
        .and_then(|thinking| thinking.get("budget_tokens"))
        .and_then(Value::as_i64)
    else {
        return Ok(());
    };
    let Some(max_tokens) = object.get("max_tokens").and_then(Value::as_i64) else {
        return Ok(());
    };
    if max_tokens > budget {
        return Ok(());
    }
    let user_set = parameters
        .iter()
        .any(|parameter| parameter.isEnabled && parameter.apiName == "max_tokens");
    if user_set {
        return Err(AiServiceError::RequestFailed(format!(
            "max_tokens ({max_tokens}) must be larger than the thinking budget ({budget}); \
             lower the thinking level or raise max_tokens"
        )));
    }
    object.insert("max_tokens".to_string(), json!(budget + DEFAULT_MAX_TOKENS));
    Ok(())
}

fn append_converter_events(chunks: &mut Vec<String>, events: Vec<StreamingJsonXmlEvent>) {
    for event in events {
        match event {
            StreamingJsonXmlEvent::Tag(text) | StreamingJsonXmlEvent::Content(text) => {
                chunks.push(text)
            }
        }
    }
}

fn sanitize_tool_call_id(raw: &str) -> String {
    let mut output = String::new();
    let mut previous_underscore = false;
    for ch in raw.chars() {
        let next = if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            ch
        } else {
            '_'
        };
        if next == '_' {
            if !previous_underscore {
                output.push(next);
            }
            previous_underscore = true;
        } else {
            output.push(next);
            previous_underscore = false;
        }
    }
    let output = output.trim_matches('_').to_string();
    if output.is_empty() {
        "toolu".to_string()
    } else {
        output
    }
}

fn sanitize_short_tool_call_id(raw: &str) -> String {
    let cleaned = raw
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    if cleaned.is_empty() {
        return "call00000".to_string();
    }
    if cleaned.len() == 9 {
        return cleaned;
    }
    if cleaned.len() > 9 {
        return cleaned[cleaned.len() - 9..].to_string();
    }
    let filler = stable_id_hash_part(raw);
    format!("{cleaned}{filler}000000000")
        .chars()
        .take(9)
        .collect()
}

fn generated_tool_use_id(ordinal: usize) -> String {
    let suffix = sanitize_short_tool_call_id(&format!(
        "{}_{}",
        stable_id_hash_part(&format!("tool_use:{ordinal}")),
        ordinal
    ));
    format!("toolu_{suffix}")
}

fn stable_id_hash_part(raw: &str) -> String {
    let mut hash: i32 = 0;
    for unit in raw.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(unit as i32);
    }
    let positive = if hash == i32::MIN {
        0
    } else {
        hash.abs() as u32
    };
    let mut value = positive;
    let mut chars = Vec::new();
    if value == 0 {
        chars.push('0');
    }
    while value > 0 {
        chars.push(std::char::from_digit(value % 36, 36).unwrap_or('0'));
        value /= 36;
    }
    chars.iter().rev().collect()
}

#[cfg(test)]
#[path = "../../../../tests/ClaudeProviderTests.rs"]
mod tests;

/// Canonicalizes object keys while preserving array/content block order.
fn stable_json_value(value: &Value) -> String {
    match value {
        Value::Object(object) => {
            let mut keys: Vec<_> = object.keys().collect();
            keys.sort();
            let fields: Vec<_> = keys
                .into_iter()
                .map(|key| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).expect("JSON key"),
                        stable_json_value(&object[key])
                    )
                })
                .collect();
            format!("{{{}}}", fields.join(","))
        }
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(stable_json_value)
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => value.to_string(),
    }
}

#[cfg(test)]
#[path = "../../../../tests/ClaudeProviderCacheTests.rs"]
mod cache_tests;
