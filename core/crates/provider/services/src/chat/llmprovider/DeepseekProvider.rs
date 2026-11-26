use super::ApiKeyProvider::KeyGetter;
use super::ProviderHttp::KeyAuthentication;
use async_trait::async_trait;
use futures_util::StreamExt;
use http::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::OpenAIProvider::{OpenAIProvider, ResponsesStreamProtocol};
use super::OpenAIResponsesProvider::{
    append_web_search_tool, build_responses_web_search_chunks, responses_metadata_tag,
    strip_responses_reasoning_metadata, OpenAIResponsesPayloadAdapter, ParsedResponseOutput,
    RESPONSES_OUTPUT_ITEM_META_PROVIDER, RESPONSES_REASONING_META_PROVIDER,
};
use super::StructuredToolCallBridge::StructuredToolCallBridge;
use super::ThinkingDialect::ThinkingDialect;
use crate::chat::llmprovider::AIService::{
    response_stream_from_chunks, AIService, AiServiceError, SendMessageRequest, TokenCounts,
};
use crate::runtime_support::ProviderRuntimeContext;
use operit_model::ModelConfigData::{BuiltinToolRequestFormat, ModelBuiltinTool};
use operit_model::ModelParameter::ModelParameter;
use operit_model::ModelParameter::ParameterValueType;
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_model::ToolPrompt::ToolPrompt;
use operit_util::stream::RevisableTextStream::{
    with_ordered_event_channel_and_failure, ResponseStreamItem, RevisableTextStream,
    RevisableTextStreamLike, TerminalFailureState, TextStreamEventCarrier, TextStreamEventType,
};
use operit_util::stream::Stream::{FnStream, Stream};
use operit_util::AppLogger::AppLogger;
use operit_util::ChatUtils::ChatUtils;
use operit_util::TokenCacheManager::TokenCacheManager;

const PROVIDER_TRANSPORT_LOG_TAG: &str = "ProviderTransport";

pub(crate) struct DeepseekResponsesPayloadAdapter;

impl DeepseekResponsesPayloadAdapter {
    /// Converts chat messages with DeepSeek plaintext reasoning replay semantics.
    pub fn to_responses_request(chatStyleRequest: Value) -> Value {
        OpenAIResponsesPayloadAdapter::to_deepseek_responses_request(chatStyleRequest)
    }

    /// Parses DeepSeek Responses output while preserving stateless continuation metadata.
    pub fn parse_non_streaming_response(jsonResponse: &Value) -> ParsedResponseOutput {
        let mut textChunks = Vec::new();
        let mut reasoningChunks = Vec::new();
        let mut reasoningMetadataTags = Vec::new();
        let mut outputItemMetadataTags = Vec::new();
        let mut searchItems = Vec::new();
        let mut toolCalls = Vec::new();
        let mut reasoningObserved = false;

        if let Some(output) = jsonResponse.get("output").and_then(Value::as_array) {
            for item in output {
                match item.get("type").and_then(Value::as_str).unwrap_or_default() {
                    "message" if Self::is_commentary_message(item) => {
                        if let Some(metadataTag) = Self::create_commentary_metadata_tag(item) {
                            outputItemMetadataTags.push(metadataTag);
                            reasoningObserved = true;
                        }
                    }
                    "message" => {
                        for part in item
                            .get("content")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                        {
                            let text = part.get("text").and_then(Value::as_str).unwrap_or("");
                            if text.is_empty() {
                                continue;
                            }
                            match part.get("type").and_then(Value::as_str).unwrap_or("") {
                                "output_text" | "text" => textChunks.push(text.to_string()),
                                "reasoning_text" => {
                                    reasoningObserved = true;
                                    reasoningChunks.push(text.to_string());
                                }
                                _ => {}
                            }
                        }
                    }
                    "reasoning" => {
                        reasoningObserved = true;
                        if let Some(metadataTag) = Self::create_reasoning_metadata_tag(item) {
                            reasoningMetadataTags.push(metadataTag);
                        }
                        reasoningChunks.extend(Self::reasoning_texts(item));
                    }
                    "function_call" => {
                        if let Some(toolCall) = Self::convert_function_call_item(item) {
                            toolCalls.push(toolCall);
                        }
                    }
                    "web_search_call" => {
                        if let Some(metadataTag) =
                            OpenAIResponsesPayloadAdapter::create_output_item_metadata_tag(item)
                        {
                            outputItemMetadataTags.push(metadataTag);
                        }
                        searchItems.push(item.clone());
                    }
                    _ => {}
                }
            }
        }

        ParsedResponseOutput {
            textChunks,
            reasoningChunks,
            reasoningMetadataTags,
            outputItemMetadataTags,
            reasoningObserved,
            searchChunks: build_responses_web_search_chunks(&searchItems, jsonResponse),
            toolCalls: Value::Array(toolCalls),
            usage: OpenAIResponsesPayloadAdapter::parse_usage_counts(jsonResponse.get("usage")),
        }
    }

    /// Encodes one DeepSeek plaintext reasoning item for the next request.
    pub fn create_reasoning_metadata_tag(item: &Value) -> Option<String> {
        if item.get("type").and_then(Value::as_str) != Some("reasoning") {
            return None;
        }
        let reasoningId = item.get("id").and_then(Value::as_str)?.trim();
        let content = item.get("content")?.as_array()?;
        if reasoningId.is_empty() || !Self::contains_reasoning_text(content) {
            return None;
        }
        Some(responses_metadata_tag(
            RESPONSES_REASONING_META_PROVIDER,
            &json!({"reasoning_id": reasoningId, "content": content}),
        ))
    }

    /// Encodes one completed DeepSeek commentary message for reasoning replay.
    pub fn create_commentary_metadata_tag(item: &Value) -> Option<String> {
        if !Self::is_commentary_message(item) {
            return None;
        }
        let content = item.get("content")?.as_array()?;
        if Self::commentary_to_reasoning_content(content).is_empty() {
            return None;
        }
        let mut metadata = Map::new();
        metadata.insert("type".to_string(), json!("message"));
        metadata.insert("role".to_string(), json!("assistant"));
        if let Some(id) = item
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
        {
            metadata.insert("id".to_string(), json!(id));
        }
        metadata.insert("content".to_string(), Value::Array(content.clone()));
        Some(responses_metadata_tag(
            RESPONSES_OUTPUT_ITEM_META_PROVIDER,
            &Value::Object(metadata),
        ))
    }

    /// Encodes streamed commentary whose completed item omits its content snapshot.
    pub fn create_streaming_commentary_metadata_tag(
        item: &Value,
        commentaryText: &str,
    ) -> Option<String> {
        if !Self::is_commentary_message(item) || commentaryText.is_empty() {
            return None;
        }
        let mut metadata = Map::new();
        metadata.insert("type".to_string(), json!("message"));
        metadata.insert("role".to_string(), json!("assistant"));
        if let Some(id) = item
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
        {
            metadata.insert("id".to_string(), json!(id));
        }
        metadata.insert(
            "content".to_string(),
            json!([{"type": "output_text", "text": commentaryText}]),
        );
        Some(responses_metadata_tag(
            RESPONSES_OUTPUT_ITEM_META_PROVIDER,
            &Value::Object(metadata),
        ))
    }

    /// Returns whether one Responses message is DeepSeek commentary.
    pub fn is_commentary_message(item: &Value) -> bool {
        item.get("type").and_then(Value::as_str) == Some("message")
            && item
                .get("phase")
                .and_then(Value::as_str)
                .is_some_and(|phase| phase.trim().eq_ignore_ascii_case("commentary"))
    }

    /// Extracts plaintext reasoning parts from one completed reasoning item.
    pub fn reasoning_texts(item: &Value) -> Vec<String> {
        item.get("content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|part| part.get("type").and_then(Value::as_str) == Some("reasoning_text"))
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    }

    /// Returns reasoning text parts with source indexes for stream deduplication.
    pub fn reasoning_parts(item: &Value) -> Vec<(usize, String)> {
        item.get("content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
            .filter(|(_, part)| part.get("type").and_then(Value::as_str) == Some("reasoning_text"))
            .filter_map(|(index, part)| {
                let text = part.get("text").and_then(Value::as_str)?;
                (!text.is_empty()).then(|| (index, text.to_string()))
            })
            .collect()
    }

    /// Returns whether one content array carries DeepSeek reasoning text.
    fn contains_reasoning_text(content: &[Value]) -> bool {
        content.iter().any(|part| {
            part.get("type").and_then(Value::as_str) == Some("reasoning_text")
                && part
                    .get("text")
                    .and_then(Value::as_str)
                    .is_some_and(|text| !text.is_empty())
        })
    }

    /// Converts commentary parts into the required DeepSeek reasoning content shape.
    fn commentary_to_reasoning_content(content: &[Value]) -> Vec<Value> {
        content
            .iter()
            .filter_map(|part| {
                let partType = part.get("type").and_then(Value::as_str)?;
                if !matches!(partType, "output_text" | "text" | "reasoning_text") {
                    return None;
                }
                let text = part.get("text").and_then(Value::as_str)?;
                if text.is_empty() {
                    return None;
                }
                Some(json!({"type": "reasoning_text", "text": text}))
            })
            .collect()
    }

    /// Converts one Responses function call into the chat-compatible tool-call shape.
    fn convert_function_call_item(item: &Value) -> Option<Value> {
        let name = item.get("name").and_then(Value::as_str)?.trim();
        if name.is_empty() {
            return None;
        }
        let arguments = item
            .get("arguments")
            .and_then(Value::as_str)
            .filter(|arguments| !arguments.trim().is_empty())
            .unwrap_or("{}");
        let mut toolCall = Map::new();
        if let Some(callId) = item
            .get("call_id")
            .or_else(|| item.get("id"))
            .and_then(Value::as_str)
            .filter(|callId| !callId.is_empty())
        {
            toolCall.insert("id".to_string(), json!(callId));
        }
        toolCall.insert("type".to_string(), json!("function"));
        toolCall.insert(
            "function".to_string(),
            json!({"name": name, "arguments": arguments}),
        );
        Some(Value::Object(toolCall))
    }
}

#[derive(Clone)]
pub struct DeepseekProvider {
    pub api_endpoint: String,
    pub key_getter: KeyGetter,

    pub model_name: String,
    pub provider_type: String,
    pub supports_vision: bool,
    pub supports_audio: bool,
    pub supports_video: bool,
    pub enable_tool_call: bool,
    pub builtin_tools: Vec<ModelBuiltinTool>,
    pub custom_headers: Vec<(String, String)>,
    runtime_context: ProviderRuntimeContext,
    state: Arc<Mutex<DeepseekProviderState>>,
}

#[derive(Default)]
struct DeepseekProviderState {
    inputTokenCount: i64,
    cachedInputTokenCount: i64,
    outputTokenCount: i64,
    cancelled: bool,
    tokenCacheManager: TokenCacheManager,
    activeParent: Option<OpenAIProvider>,
    activeParentGeneration: u64,
}

impl DeepseekProvider {
    /// Creates a DeepSeek provider bound to one provider runtime context.
    pub fn new(
        api_endpoint: String,
        key_getter: KeyGetter,
        model_name: String,
        provider_type: String,
        custom_headers: Vec<(String, String)>,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        builtin_tools: Vec<ModelBuiltinTool>,
        enable_tool_call: bool,
        runtime_context: ProviderRuntimeContext,
    ) -> Self {
        Self {
            api_endpoint,
            key_getter,
            model_name,
            provider_type,
            supports_vision,
            supports_audio,
            supports_video,
            builtin_tools,
            enable_tool_call,
            custom_headers,
            runtime_context,
            state: Arc::new(Mutex::new(DeepseekProviderState::default())),
        }
    }

    /// Decodes the parent transport while retaining cancellation, usage and stream revisions.
    fn decode_parent_stream(&self, mut parent: OpenAIProvider, result: Box<dyn RevisableTextStreamLike>) -> Box<dyn RevisableTextStreamLike> {
        let mut provider = self.clone();
        let activeParentGeneration = provider.setActiveParent(parent.clone());
        if provider.isCancelled() { parent.cancel_streaming(); }
        let event_channel = result.event_channel().clone();
        let terminalFailure = TerminalFailureState::new();
        let producerTerminalFailure = terminalFailure.clone();
        let mut ownedResult = Some(result);
        let mut ownedParent = Some(parent);
        let mut ownedProvider = Some(provider);
        let cold_stream = FnStream::new(move |emit| {
            let mut result = ownedResult
                .take()
                .expect("Deepseek parent stream must only be collected once");
            let mut parent = ownedParent
                .take()
                .expect("Deepseek parent stream must only be collected once");
            let mut provider = ownedProvider
                .take()
                .expect("Deepseek parent stream must only be collected once");
            let producerTerminalFailure = producerTerminalFailure.clone();
            Box::pin(async move {
                let mut decoder = DeepseekDsmlDecoder::default();
                let mut savepoints = HashMap::new();
                let mut decodingError = None;
                result
                    .collect_ordered(&mut |item| match item {
                        ResponseStreamItem::Chunk(chunk) if decodingError.is_none() => {
                            match decoder.push(&chunk) {
                                Ok(content) if !content.is_empty() => emit(ResponseStreamItem::Chunk(content)),
                                Ok(_) => {}
                                Err(error) => {
                                    decodingError = Some(error.to_string());
                                    parent.cancel_streaming();
                                }
                            }
                        }
                        ResponseStreamItem::Revision(event) => {
                            match event.event_type {
                                TextStreamEventType::Savepoint => {
                                    savepoints.insert(event.id.clone(), decoder.clone());
                                }
                                TextStreamEventType::Rollback => {
                                    if let Some(saved) = savepoints.get(&event.id) {
                                        decoder.clone_from(saved);
                                    }
                                }
                            }
                            emit(ResponseStreamItem::Revision(event));
                        },
                        _ => {}
                    })
                    .await;
                if let Some(error) = decodingError {
                    producerTerminalFailure.set(error);
                } else if let Some(error) = result.terminal_failure() {
                    producerTerminalFailure.set(error);
                } else {
                    match decoder.finish() {
                        Ok(tail) if !tail.is_empty() => emit(ResponseStreamItem::Chunk(tail)),
                        Ok(_) => {}
                        Err(error) => producerTerminalFailure.set(error.to_string()),
                    }
                }
                AppLogger::v_with_level(
                    PROVIDER_TRANSPORT_LOG_TAG,
                    &format!(
                        "provider.deepseek.parent.stream.done providerModel={} inputTokens={} cachedInputTokens={} outputTokens={}",
                        parent.provider_model(),
                        parent.input_token_count(),
                        parent.cached_input_token_count(),
                        parent.output_token_count(),
                    ),
                    operit_util::AppLogger::VERBOSE_LEVEL_3,
                );
                provider.finishActiveParent(activeParentGeneration, TokenCounts {
                    input: (parent.input_token_count() - parent.cached_input_token_count()).max(0),
                    cached_input: parent.cached_input_token_count(),
                    output: parent.output_token_count(),
                });
            })
        });
        Box::new(with_ordered_event_channel_and_failure(cold_stream, event_channel, terminalFailure))
    }

    fn apply_token_counts(&self, token_counts: TokenCounts) {
        if let Ok(mut state) = self.state.lock() {
            if token_counts.input > 0 || token_counts.cached_input > 0 {
                state.tokenCacheManager.update_actual_tokens(
                    token_counts.input.max(0),
                    token_counts.cached_input.max(0),
                );
            }
            if token_counts.output > 0 {
                state
                    .tokenCacheManager
                    .set_output_tokens(token_counts.output.max(0));
            }
            state.inputTokenCount = state.tokenCacheManager.total_input_token_count();
            state.cachedInputTokenCount = state.tokenCacheManager.cached_input_token_count();
            state.outputTokenCount = state.tokenCacheManager.output_token_count();
        }
    }

    /// Uses the live transport's counters until its settled totals are copied into this provider.
    fn currentTokenCounts(&self) -> TokenCounts {
        let state = self.state.lock().expect("DeepseekProvider state mutex poisoned");
        match state.activeParent.as_ref() {
            Some(parent) => TokenCounts {
                input: parent.input_token_count(),
                cached_input: parent.cached_input_token_count(),
                output: parent.output_token_count(),
            },
            None => TokenCounts {
                input: state.tokenCacheManager.total_input_token_count(),
                cached_input: state.tokenCacheManager.cached_input_token_count(),
                output: state.tokenCacheManager.output_token_count(),
            },
        }
    }

    fn setActiveParent(&self, parent: OpenAIProvider) -> u64 {
        let mut state = self
            .state
            .lock()
            .expect("DeepseekProvider state mutex poisoned");
        state.activeParentGeneration = state.activeParentGeneration.wrapping_add(1);
        state.activeParent = Some(parent);
        state.activeParentGeneration
    }

    fn isCancelled(&self) -> bool {
        self.state
            .lock()
            .expect("DeepseekProvider state mutex poisoned")
            .cancelled
    }

    /// Commits only this transport's usage; a paused request cannot overwrite a newer request.
    fn finishActiveParent(&self, generation: u64, counts: TokenCounts) {
        let mut state = self
            .state
            .lock()
            .expect("DeepseekProvider state mutex poisoned");
        if state.activeParentGeneration == generation {
            state.tokenCacheManager.update_actual_tokens(counts.input, counts.cached_input);
            state.tokenCacheManager.set_output_tokens(counts.output);
            state.inputTokenCount = state.tokenCacheManager.total_input_token_count();
            state.cachedInputTokenCount = state.tokenCacheManager.cached_input_token_count();
            state.outputTokenCount = state.tokenCacheManager.output_token_count();
            state.activeParent = None;
        }
    }

    /// Reuses the same rich-content and transport behavior as Kotlin's OpenAI parent.
    fn open_ai_parent(&self, use_responses_api: bool) -> OpenAIProvider {
        OpenAIProvider::new_with_capabilities(
            self.api_endpoint.clone(),
            self.key_getter.clone(),
            self.model_name.clone(),
            self.provider_type.clone(),
            self.custom_headers.clone(),
            self.supports_vision,
            self.supports_audio,
            self.supports_video,
            self.enable_tool_call,
        )

        .with_responses_api(use_responses_api)
        .with_responses_stream_protocol(if use_responses_api {
            ResponsesStreamProtocol::Deepseek
        } else {
            ResponsesStreamProtocol::OpenAi
        })
    }

    pub fn create_request_body(
        &self,
        request: &SendMessageRequest,
    ) -> Result<Value, AiServiceError> {
        if self.uses_responses_protocol()? {
            let protocol_history = if request.thinking.is_on() {
                request.chat_history.clone()
            } else {
                request
                    .chat_history
                    .iter()
                    .map(|turn| {
                        turn.with_content(strip_responses_reasoning_metadata(&turn.content))
                    })
                    .collect()
            };
            let parent = self.open_ai_parent(true);
            let request_object = parent
                .create_request_body_without_thinking_for_history(request, &protocol_history)?;
            let mut responses_request =
                DeepseekResponsesPayloadAdapter::to_responses_request(request_object);
            ThinkingDialect::DeepSeekResponses.apply(&mut responses_request, &request.thinking)?;
            if self.web_search_enabled() {
                append_web_search_tool(&mut responses_request);
            }
            return Ok(responses_request);
        }

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
        let mut request_object = Value::Object(json_object);
        ThinkingDialect::ChatThinkingType.apply(&mut request_object, &request.thinking)?;
        let json_object = request_object
            .as_object_mut()
            .expect("thinking request remains an object");
        self.apply_model_parameters(json_object, &request.model_parameters);
        if effectiveEnableToolCall {
            let tools = StructuredToolCallBridge::buildToolsArray(Some(&request.available_tools));
            json_object.insert("tools".to_string(), tools);
            json_object.insert("tool_choice".to_string(), json!("auto"));
        }
        Ok(request_object)
    }

    /// Resolves the DeepSeek wire protocol from the configured endpoint path.
    fn uses_responses_protocol(&self) -> Result<bool, AiServiceError> {
        deepseek_uses_responses_protocol(&self.api_endpoint)
    }

    /// Returns whether DeepSeek Responses web search is enabled for this model.
    fn web_search_enabled(&self) -> bool {
        self.builtin_tools.iter().any(|tool| {
            tool.enabled && tool.requestFormat == BuiltinToolRequestFormat::OpenAiWebSearch
        })
    }

    pub fn build_messages_with_reasoning(
        &self,
        effectiveHistory: &[PromptTurn],
        useToolCall: bool,
    ) -> Result<Value, AiServiceError> {
        let structuredMessages: Value = StructuredToolCallBridge::buildMessagesValueForProvider(
            effectiveHistory,
            true,
            useToolCall,
        );

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

        let mut messages = Value::Array(messagesArray);
        self.open_ai_parent(false)
            .rewrite_message_media_content(&mut messages);
        Ok(messages)
    }

    fn calculate_and_store_input_tokens(
        &self,
        provider_ready_history: &[PromptTurn],
        tools_json: Option<&str>,
        preserve_think_in_history: bool,
        update_state: bool,
    ) -> i64 {
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
                update_state,
            );
            if update_state {
                state.inputTokenCount = state.tokenCacheManager.total_input_token_count();
                state.cachedInputTokenCount = state.tokenCacheManager.cached_input_token_count();
            }
            tokenCount
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
        for (name, value) in &self.custom_headers {
            let header_name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            let header_value = HeaderValue::from_str(value)
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            headers.insert(header_name, header_value);
        }
        Ok(headers)
    }

    /// Decodes buffered chat or Responses output with the same DSML grammar as streaming.
    fn parse_response_chunks(&self, response: &Value) -> Result<Vec<String>, AiServiceError> {
        if self.uses_responses_protocol()? {
            let parsed = DeepseekResponsesPayloadAdapter::parse_non_streaming_response(response);
            if let Some(usage) = parsed.usage {
                self.apply_token_counts(TokenCounts {
                    input: usage.actualInputTokens,
                    cached_input: usage.cachedInputTokens,
                    output: usage.outputTokens,
                });
            }
            let mut chunks = Vec::new();
            for reasoning in parsed.reasoningChunks {
                chunks.push(format!("<think>{reasoning}</think>"));
            }
            chunks.extend(parsed.reasoningMetadataTags);
            chunks.extend(parsed.outputItemMetadataTags);
            chunks.extend(parsed.searchChunks);
            chunks.push(DeepseekDsmlDecoder::decode(
                parsed.textChunks.iter().map(String::as_str),
            )?);
            if let Value::Array(tool_calls) = parsed.toolCalls {
                for tool_call in tool_calls {
                    chunks.push(StructuredToolCallBridge::convertToolCallPayloadToXml(
                        &json!([tool_call]).to_string(),
                    ));
                }
            }
            return Ok(chunks);
        }

        let token_counts = response
            .get("usage")
            .map(parse_usage_counts)
            .unwrap_or(TokenCounts {
                input: 0,
                cached_input: 0,
                output: 0,
            });
        self.apply_token_counts(token_counts);
        let mut chunks = Vec::new();
        if let Some(reasoning) = extract_reasoning_chunk(response) {
            if !reasoning.is_empty() {
                chunks.push(format!("<think>{reasoning}</think>"));
            }
        }
        if let Some(content) = extract_content_chunk(response) {
            if !content.is_empty() {
                chunks.push(StructuredToolCallBridge::convertToolCallPayloadToXml(
                    &DeepseekDsmlDecoder::decode([content.as_str()])?,
                ));
            }
        }
        chunks.extend(extract_tool_calls_xml_chunks(response));
        Ok(chunks)
    }
}

/// Resolves whether one DeepSeek endpoint selects the Responses protocol.
pub(crate) fn deepseek_uses_responses_protocol(endpoint: &str) -> Result<bool, AiServiceError> {
    let endpoint = url::Url::parse(endpoint)
        .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
    let segment = endpoint
        .path_segments()
        .and_then(|segments| segments.last());
    Ok(segment.is_some_and(|value| value.eq_ignore_ascii_case("responses")))
}

#[cfg(test)]
#[path = "../../../../tests/DeepseekProviderResponsesTests.rs"]
mod responses_tests;

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl AIService for DeepseekProvider {
    fn input_token_count(&self) -> i64 {
        self.currentTokenCounts().input
    }

    fn cached_input_token_count(&self) -> i64 {
        self.currentTokenCounts().cached_input
    }

    fn output_token_count(&self) -> i64 {
        self.currentTokenCounts().output
    }

    fn provider_model(&self) -> String {
        format!("{}:{}", self.provider_type, self.model_name)
    }

    fn reset_token_counts(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.activeParentGeneration = state.activeParentGeneration.wrapping_add(1);
            state.activeParent = None;
            state.inputTokenCount = 0;
            state.cachedInputTokenCount = 0;
            state.outputTokenCount = 0;
            state.tokenCacheManager.reset_token_counts();
        }
    }

    fn cancel_streaming(&mut self) {
        let activeParent = if let Ok(mut state) = self.state.lock() {
            state.cancelled = true;
            state.activeParent.clone()
        } else {
            None
        };
        if let Some(mut parent) = activeParent {
            parent.cancel_streaming();
        }
    }

    /// Sends a DeepSeek request while preserving parent-stream failures.
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
            AppLogger::v_with_level(
                PROVIDER_TRANSPORT_LOG_TAG,
                &format!(
                    "provider.deepseek.parent.create providerModel={} stream=true",
                    self.provider_model(),
                ),
                operit_util::AppLogger::VERBOSE_LEVEL_3,
            );
            let mut parent = self.open_ai_parent(self.uses_responses_protocol()?);
            parent.apply_token_counts(TokenCounts {
                input: (self.input_token_count() - self.cached_input_token_count()).max(0),
                cached_input: self.cached_input_token_count(),
                output: 0,
            });
            let mut result = parent.send_prepared_request(request, request_body).await?;
            AppLogger::v_with_level(
                PROVIDER_TRANSPORT_LOG_TAG,
                &format!(
                    "provider.deepseek.parent.stream.ready providerModel={}",
                    self.provider_model(),
                ),
                operit_util::AppLogger::VERBOSE_LEVEL_3,
            );
            return Ok(self.decode_parent_stream(parent, result));
        }

        let response = super::ProviderHttp::post(&self.api_endpoint)
            .key_getter(self.key_getter.clone(), KeyAuthentication::Bearer)
            .headers(self.headers()?)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let message = response
                .text()
                .await?;
            return Err(AiServiceError::RequestFailed(format!(
                "{status}: {message}"
            )));
        }

        let json_response: Value = response
            .json()
            .await?;
        Ok(response_stream_from_chunks(
            self.parse_response_chunks(&json_response)?,
        ))
    }

    async fn test_connection(&self) -> Result<String, AiServiceError> {
        let request_body = if self.uses_responses_protocol()? {
            json!({
                "model": self.model_name,
                "input": "hi",
                "stream": false,
                "max_output_tokens": 1,
                "reasoning": {"effort": "none"}
            })
        } else {
            json!({
                "model": self.model_name,
                "messages": [{"role": "user", "content": "hi"}],
                "stream": false,
                "max_tokens": 1,
                "thinking": {"type": "disabled"}
            })
        };
        let response = super::ProviderHttp::post(&self.api_endpoint)
            .key_getter(self.key_getter.clone(), KeyAuthentication::Bearer)
            .headers(self.headers()?)
            .json(&request_body)
            .send()
            .await?;
        if response.status().is_success() {
            Ok("Connection successful".to_string())
        } else {
            let status = response.status();
            let body = response
                .text()
                .await?;
            Err(AiServiceError::ConnectionFailed(format!(
                "{status}: {body}"
            )))
        }
    }

    async fn calculate_input_tokens(
        &self,
        chat_history: &[PromptTurn],
        available_tools: &[ToolPrompt],
    ) -> Result<i64, AiServiceError> {
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
                false,
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

/// Reads DeepSeek's reported prompt, cache, and completion counters.
fn parse_usage_counts(usage: &Value) -> TokenCounts {
    let prompt_tokens = usage
        .get("prompt_tokens")
        .or_else(|| usage.get("input_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i64;
    let cached_tokens = usage
        .pointer("/prompt_tokens_details/cached_tokens")
        .or_else(|| usage.pointer("/input_tokens_details/cached_tokens"))
        .or_else(|| usage.get("prompt_cache_hit_tokens"))
        .or_else(|| usage.get("cache_read_input_tokens"))
        .or_else(|| usage.get("cached_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i64;
    let completion_tokens = usage
        .get("completion_tokens")
        .or_else(|| usage.get("output_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i64;
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
            StructuredToolCallBridge::convertToolCallPayloadToXml(&json!([tool_call]).to_string())
        })
        .filter(|content| operit_util::ChatMarkupRegex::ChatMarkupRegex::contains_tool_tag(content))
        .collect()
}

#[cfg(test)]
#[path = "../../../../tests/DeepseekProviderTests.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../../tests/DeepseekProviderMediaRoleTests.rs"]
mod media_role_tests;

/// DeepSeek's current and legacy tool-call namespaces and container names.
const CONTAINERS: [(&str, &str); 8] = [
    ("｜｜DSML｜｜", "calls"),
    ("｜｜DSML｜｜", "function_calls"),
    ("||DSML||", "calls"),
    ("||DSML||", "function_calls"),
    ("｜DSML｜", "calls"),
    ("｜DSML｜", "function_calls"),
    ("|DSML|", "calls"),
    ("|DSML|", "function_calls"),
];

/// Matches a DSML tag name with optional whitespace after its namespace.
fn dsml_tag_rest<'a>(
    text: &'a str,
    namespace: &str,
    name: &str,
    closing: bool,
) -> Option<&'a str> {
    let prefix = if closing {
        format!("</{namespace}")
    } else {
        format!("<{namespace}")
    };
    text.strip_prefix(&prefix)?
        .trim_start()
        .strip_prefix(name)
        .filter(|rest| rest.starts_with('>') || rest.starts_with(char::is_whitespace))
}

/// Measures one complete attribute-free DSML tag at the start of the text.
fn dsml_tag_end(text: &str, namespace: &str, name: &str, closing: bool) -> Option<usize> {
    let tail = dsml_tag_rest(text, namespace, name, closing)?
        .trim_start()
        .strip_prefix('>')?;
    Some(text.len() - tail.len())
}

/// Finds a complete attribute-free DSML tag without altering parameter text.
fn find_dsml_tag(
    text: &str,
    namespace: &str,
    name: &str,
    closing: bool,
) -> Option<(usize, usize)> {
    text.match_indices('<').find_map(|(offset, _)| {
        dsml_tag_end(&text[offset..], namespace, name, closing).map(|length| (offset, offset + length))
    })
}

/// Retains incomplete opening tags even when whitespace spans multiple deltas.
fn is_partial_dsml_opening(text: &str, namespace: &str, name: &str) -> bool {
    let prefix = format!("<{namespace}");
    if prefix.starts_with(text) {
        return true;
    }
    let Some(rest) = text.strip_prefix(&prefix) else {
        return false;
    };
    let rest = rest.trim_start();
    name.starts_with(rest)
        || rest
            .strip_prefix(name)
            .is_some_and(|tail| tail.trim_start().is_empty())
}

/// Converts DeepSeek content into Operit markup without exposing partial DSML blocks.
#[derive(Clone, Default)]
struct DeepseekDsmlDecoder {
    pending: String,
    container: Option<usize>,
    reasoning: bool,
}

impl DeepseekDsmlDecoder {
    /// Emits ordinary text immediately and converts each complete DSML container atomically.
    fn push(&mut self, text: &str) -> Result<String, AiServiceError> {
        self.pending.push_str(text);
        let mut output = String::new();
        loop {
            if let Some(index) = self.container {
                let (namespace, name) = CONTAINERS[index];
                let Some((end, consumed)) = find_dsml_tag(&self.pending, namespace, name, true) else {
                    break;
                };
                let calls = parse_calls(&self.pending[..end], namespace)?;
                output.push_str(&StructuredToolCallBridge::convertToolCallPayloadToXml(
                    &Value::Array(calls).to_string(),
                ));
                self.pending.drain(..consumed);
                self.container = None;
                continue;
            }

            let reasoning_tag = if self.reasoning {
                "</think>"
            } else {
                "<think>"
            };
            let opening = CONTAINERS
                .iter()
                .enumerate()
                .filter(|_| !self.reasoning)
                .filter_map(|(index, (namespace, name))| {
                    find_dsml_tag(&self.pending, namespace, name, false)
                        .map(|(offset, consumed)| (offset, index, consumed))
                })
                .chain(self.pending.find(reasoning_tag).map(|offset| {
                    (offset, CONTAINERS.len(), offset + reasoning_tag.len())
                }))
                .min_by_key(|(offset, _, _)| *offset);
            if let Some((offset, index, consumed)) = opening {
                output.push_str(&self.pending[..offset]);
                if index == CONTAINERS.len() {
                    output.push_str(reasoning_tag);
                    self.reasoning = !self.reasoning;
                } else {
                    self.container = Some(index);
                }
                self.pending.drain(..consumed);
                continue;
            }

            // Keep only a suffix that can still become an opening token in the next delta.
            let retained = self
                .pending
                .char_indices()
                .rev()
                .filter(|(_, ch)| *ch == '<')
                .find_map(|(offset, _)| {
                    let suffix = &self.pending[offset..];
                    (reasoning_tag.starts_with(suffix)
                        || (!self.reasoning
                            && CONTAINERS.iter().any(|(namespace, name)| {
                                is_partial_dsml_opening(suffix, namespace, name)
                            })))
                    .then_some(suffix.len())
                })
                .unwrap_or(0);
            let emitted = self.pending.len() - retained;
            output.push_str(&self.pending[..emitted]);
            self.pending.drain(..emitted);
            break;
        }
        Ok(output)
    }

    /// Rejects truncated DSML while releasing a trailing literal opening angle bracket.
    fn finish(&mut self) -> Result<String, AiServiceError> {
        if self.container.is_some()
            || CONTAINERS
                .iter()
                .any(|(namespace, _)| self.pending.starts_with(&format!("<{namespace}")))
        {
            return Err(invalid_dsml("incomplete tool-call container"));
        }
        Ok(std::mem::take(&mut self.pending))
    }

    /// Converts buffered content through the same decoder used for streaming deltas.
    fn decode<'a>(chunks: impl IntoIterator<Item = &'a str>) -> Result<String, AiServiceError> {
        let mut decoder = Self::default();
        let mut output = String::new();
        for chunk in chunks {
            output.push_str(&decoder.push(chunk)?);
        }
        output.push_str(&decoder.finish()?);
        Ok(output)
    }
}

/// Parses all invocations before producing executable markup for one container.
fn parse_calls(mut body: &str, namespace: &str) -> Result<Vec<Value>, AiServiceError> {
    let mut calls = Vec::new();
    while !body.trim_start().is_empty() {
        let attributes = consume_opening(&mut body, namespace, "invoke")?;
        let name = required_attribute(&attributes, "name")?;
        if attributes.len() != 1 {
            return Err(invalid_dsml("unexpected invocation attribute"));
        }
        let mut arguments = Map::new();
        loop {
            body = body.trim_start();
            if let Some(consumed) = dsml_tag_end(body, namespace, "invoke", true) {
                body = &body[consumed..];
                break;
            }
            let attributes = consume_opening(&mut body, namespace, "parameter")?;
            let key = required_attribute(&attributes, "name")?;
            let is_string = required_attribute(&attributes, "string")?;
            if attributes.len() != 2 {
                return Err(invalid_dsml("unexpected parameter attribute"));
            }
            let (end, consumed) = find_dsml_tag(body, namespace, "parameter", true)
                .ok_or_else(|| invalid_dsml("missing parameter closing token"))?;
            let raw = &body[..end];
            let value = match is_string {
                "true" => Value::String(raw.to_string()),
                "false" => serde_json::from_str(raw)
                    .map_err(|error| invalid_dsml(&format!("invalid JSON parameter: {error}")))?,
                _ => {
                    return Err(invalid_dsml(
                        "parameter string attribute must be true or false",
                    ))
                }
            };
            if arguments.insert(key.to_string(), value).is_some() {
                return Err(invalid_dsml("duplicate parameter name"));
            }
            body = &body[consumed..];
        }
        calls.push(json!({
            "type": "function",
            "function": {"name": name, "arguments": Value::Object(arguments).to_string()}
        }));
    }
    if calls.is_empty() {
        return Err(invalid_dsml("empty tool-call container"));
    }
    Ok(calls)
}

/// Consumes one opening token and its quoted attributes using the DSML grammar.
fn consume_opening<'a>(
    body: &mut &'a str,
    namespace: &str,
    name: &str,
) -> Result<Map<String, Value>, AiServiceError> {
    let rest = dsml_tag_rest(body.trim_start(), namespace, name, false)
        .filter(|rest| rest.starts_with(char::is_whitespace))
        .ok_or_else(|| invalid_dsml(&format!("expected <{namespace}{name}")))?;
    let mut rest = rest.trim_start();
    let mut attributes = Map::new();
    loop {
        if let Some(tail) = rest.strip_prefix('>') {
            *body = tail;
            return Ok(attributes);
        }
        let name_end = rest
            .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
            .ok_or_else(|| invalid_dsml("incomplete attribute"))?;
        if name_end == 0 {
            return Err(invalid_dsml("invalid attribute name"));
        }
        let name = &rest[..name_end];
        rest = rest[name_end..]
            .trim_start()
            .strip_prefix('=')
            .ok_or_else(|| invalid_dsml("expected attribute assignment"))?
            .trim_start();
        let quote = rest
            .chars()
            .next()
            .filter(|quote| matches!(quote, '\'' | '"'))
            .ok_or_else(|| invalid_dsml("expected quoted attribute"))?;
        rest = &rest[quote.len_utf8()..];
        let end = rest
            .find(quote)
            .ok_or_else(|| invalid_dsml("unterminated attribute"))?;
        if attributes
            .insert(name.to_string(), Value::String(rest[..end].to_string()))
            .is_some()
        {
            return Err(invalid_dsml("duplicate attribute"));
        }
        rest = &rest[end + quote.len_utf8()..];
        if !rest.starts_with('>') && !rest.starts_with(char::is_whitespace) {
            return Err(invalid_dsml("expected whitespace between attributes"));
        }
        rest = rest.trim_start();
    }
}

/// Reads a required nonempty attribute without changing its literal value.
fn required_attribute<'a>(
    attributes: &'a Map<String, Value>,
    name: &str,
) -> Result<&'a str, AiServiceError> {
    attributes
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid_dsml(&format!("missing {name} attribute")))
}

/// Reports malformed DSML through the provider's existing terminal error path.
fn invalid_dsml(message: &str) -> AiServiceError {
    AiServiceError::RequestFailed(format!("Invalid DeepSeek DSML: {message}"))
}

#[cfg(test)]
mod dsml_tests {
    use serde_json::{json, Value};

    use super::DeepseekProvider;
    use super::{parse_calls, DeepseekDsmlDecoder, CONTAINERS};
    use crate::chat::llmprovider::ProviderMediaTestSupport::test_runtime_context;
    use crate::chat::llmprovider::StructuredToolCallBridge::StructuredToolCallBridge;
    use operit_util::ChatMarkupRegex::ChatMarkupRegex;

    const SCREENSHOT_CALL: &str = concat!(
        "<｜｜DSML｜｜calls>\n",
        "<｜｜DSML｜｜invoke name=\"use_package\">\n",
        "<｜｜DSML｜｜parameter name=\"package_name\" string=\"true\">time</｜｜DSML｜｜parameter>\n",
        "</｜｜DSML｜｜invoke>\n",
        "</｜｜DSML｜｜calls>"
    );

    /// Replays a disconnected native tool delta, its retry and a later final response through production decoding.
    #[tokio::test]
    async fn retry_rolls_back_partial_tool_markup_before_later_rounds() {
        use crate::chat::llmprovider::AIService::SharedAiResponseStream;
        use operit_model::MessagePartCodec::{AssistantMarkupStreamState, MessagePartCodec};
        use operit_util::stream::HotStream::mutable_shared_stream;
        use operit_util::stream::RevisableTextStream::{ResponseStreamItem, TextStreamEvent, TextStreamEventType};
        use operit_util::stream::TextStreamRevisionTracker::TextStreamRevisionTracker;
        let provider = DeepseekProvider::new("https://fixture.invalid/chat/completions".into(), "".into(), "deepseek-v4-flash".into(), "DEEPSEEK".into(), vec![], false, false, false, vec![], true, test_runtime_context());
        let parent = SharedAiResponseStream::new_ordered(mutable_shared_stream(usize::MAX), mutable_shared_stream(usize::MAX));
        let output = provider.decode_parent_stream(provider.open_ai_parent(false), Box::new(parent.clone()));
        let mut output = output;
        let mut tracker = TextStreamRevisionTracker::new("");
        let (begun, started) = tokio::sync::oneshot::channel();
        let mut begun = Some(begun);
        let collect = async {
            output.collect_ordered(&mut |item| match item {
                ResponseStreamItem::Chunk(chunk) => {
                    tracker.append(&chunk);
                    if let Some(begun) = begun.take() { let _ = begun.send(()); }
                }
                ResponseStreamItem::Revision(event) => match event.event_type {
                    TextStreamEventType::Savepoint => tracker.savepoint(&event.id),
                    TextStreamEventType::Rollback => { tracker.rollback(&event.id).expect("Retry savepoint must reach the message consumer"); }
                },
            }).await;
            assert_eq!(output.terminal_failure(), None);
        };
        let drive = async {
            parent.emit_revision(TextStreamEvent { event_type: TextStreamEventType::Savepoint, id: "attempt".into() });
            parent.emit_chunk("<tool_abandoned name=\"write_file\"><param name=\"content\">partial".into());
            started.await.unwrap();
            parent.emit_revision(TextStreamEvent { event_type: TextStreamEventType::Rollback, id: "attempt".into() });
            parent.emit_chunk("<tool_retry name=\"write_file\"><param name=\"content\">complete</param></tool_retry>".into());
            parent.close();
        };
        futures_util::future::join(collect, drive).await;
        let mut message = AssistantMarkupStreamState::new();
        message.push(tracker.current_content()).unwrap();
        message.push("<tool_result_retry name=\"write_file\" status=\"success\"><content>saved</content></tool_result_retry>最终结果已完成").unwrap();
        let parts = message.finish().unwrap();
        assert_eq!(MessagePartCodec::visibleText(&parts), "最终结果已完成");
        assert!(!tracker.current_content().contains("abandoned"));
    }

    /// Extracts semantic tool calls without depending on generated tag suffixes.
    fn decoded_calls(text: &str) -> (String, Vec<Value>) {
        let (visible, calls) = StructuredToolCallBridge::parseXmlToolCalls(text);
        (visible, calls.expect("converted tool calls"))
    }

    /// Verifies the exact DSML call shown in the user's screenshot becomes executable markup.
    #[test]
    fn screenshot_dsml_converts_to_operit_tool_markup() {
        let output = DeepseekDsmlDecoder::decode([SCREENSHOT_CALL]).unwrap();
        let (visible, calls) = decoded_calls(&output);
        assert!(visible.is_empty());
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["function"]["name"], "use_package");
        let arguments: Value =
            serde_json::from_str(calls[0]["function"]["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(arguments, json!({"package_name": "time"}));
    }

    /// Checks every UTF-8 split and individual character delta against the same conversion.
    #[test]
    fn arbitrary_stream_boundaries_preserve_the_call_and_surrounding_text() {
        let source = format!("开始😀{SCREENSHOT_CALL}完成");
        for (offset, _) in source.char_indices() {
            let output =
                DeepseekDsmlDecoder::decode([&source[..offset], &source[offset..]]).unwrap();
            let (visible, calls) = decoded_calls(&output);
            assert_eq!(visible, "开始😀\n完成", "split at {offset}");
            assert_eq!(calls.len(), 1, "split at {offset}");
        }
        let pieces = source
            .char_indices()
            .map(|(offset, ch)| &source[offset..offset + ch.len_utf8()]);
        let output = DeepseekDsmlDecoder::decode(pieces).unwrap();
        assert_eq!(decoded_calls(&output).1.len(), 1);
    }

    /// Ensures plain text is live while DSML emits only after its entire container validates.
    #[test]
    fn text_is_live_and_incomplete_dsml_is_never_executable() {
        let mut decoder = DeepseekDsmlDecoder::default();
        assert_eq!(
            decoder.push("普通文本 & <literal>").unwrap(),
            "普通文本 & <literal>"
        );
        assert_eq!(decoder.push("前缀<｜DS").unwrap(), "前缀");
        assert!(decoder
            .push("ML｜calls><｜DSML｜invoke name=\"use_package\">")
            .unwrap()
            .is_empty());
        assert!(decoder.push("</｜DSML｜invoke>").unwrap().is_empty());
        let completed = decoder.push("</｜DSML｜calls>后缀").unwrap();
        assert_eq!(ChatMarkupRegex::tool_call_matches(&completed).len(), 1);
        assert!(completed.ends_with("后缀"));
        assert!(decoder.finish().unwrap().is_empty());
        assert!(decoder.finish().unwrap().is_empty());
    }

    /// Covers both DSML container generations and their ASCII token spelling.
    #[test]
    fn container_spellings_and_multiple_invocations_are_supported() {
        for (namespace, name) in CONTAINERS {
            let opening = format!("<{namespace}{name}>");
            let closing = format!("</{namespace}{name}>");
            let body = format!(
                "<{namespace}invoke name='first'></{namespace}invoke>\n<{namespace}invoke name=\"second\"></{namespace}invoke>"
            );
            let source = format!("{opening}{body}{closing}{opening}{body}{closing}");
            let output = DeepseekDsmlDecoder::decode([source.as_str()]).unwrap();
            let names = ChatMarkupRegex::tool_call_matches(&output)
                .into_iter()
                .map(|call| call.name)
                .collect::<Vec<_>>();
            assert_eq!(names, ["first", "second", "first", "second"]);
        }
    }

    /// Converts spaced body tags and both file-info calls across every stream boundary.
    #[test]
    fn spaced_body_file_info_calls_are_executable() {
        let source = concat!(
            "Before<｜DSML｜ calls>\n",
            "<｜DSML｜ invoke name=\"file_info\">\n",
            "<｜DSML｜ parameter name=\"path\" string=\"true\">api_list.md</｜DSML｜ parameter>\n",
            "</｜DSML｜ invoke>\n",
            "<｜DSML｜ invoke name=\"file_info\">\n",
            "<｜DSML｜ parameter name=\"path\" string=\"true\">api_list.json</｜DSML｜ parameter>\n",
            "</｜DSML｜ invoke>\n",
            "</｜DSML｜ calls>After"
        );
        for (offset, _) in source.char_indices() {
            let output =
                DeepseekDsmlDecoder::decode([&source[..offset], &source[offset..]]).unwrap();
            let (visible, calls) = decoded_calls(&output);
            assert!(visible.starts_with("Before"), "split at {offset}");
            assert!(visible.ends_with("After"), "split at {offset}");
            assert_eq!(calls.len(), 2, "split at {offset}");
            for (call, path) in calls.iter().zip(["api_list.md", "api_list.json"]) {
                assert_eq!(call["function"]["name"], "file_info");
                let arguments: Value =
                    serde_json::from_str(call["function"]["arguments"].as_str().unwrap()).unwrap();
                assert_eq!(arguments, json!({"path": path}));
            }
        }
        let pieces = source
            .char_indices()
            .map(|(offset, ch)| &source[offset..offset + ch.len_utf8()]);
        assert_eq!(
            decoded_calls(&DeepseekDsmlDecoder::decode(pieces).unwrap())
                .1
                .len(),
            2
        );
    }

    /// Accepts independent tag whitespace while preserving literal parameter bytes.
    #[test]
    fn tag_whitespace_does_not_rewrite_parameter_values() {
        for (namespace, name) in CONTAINERS {
            let literal = format!("  <{namespace} calls>\nunchanged &amp;  ");
            let source = format!(
                "<{namespace}\n\t{name} >\
                 <{namespace}invoke name='file_info'>\
                 <{namespace}\tparameter name='path' string='true'>{literal}</{namespace}\n parameter >\
                 </{namespace} invoke >\
                 </{namespace}\t{name}\n>"
            );
            let pieces = source
                .char_indices()
                .map(|(offset, ch)| &source[offset..offset + ch.len_utf8()]);
            let output = DeepseekDsmlDecoder::decode(pieces).unwrap();
            let calls = decoded_calls(&output).1;
            assert_eq!(calls.len(), 1);
            let arguments: Value =
                serde_json::from_str(calls[0]["function"]["arguments"].as_str().unwrap()).unwrap();
            assert_eq!(arguments["path"], literal);
        }
    }

    /// Rejects truncated spaced containers without accepting tag-name prefixes.
    #[test]
    fn spaced_container_boundaries_are_strict() {
        for source in ["<｜DSML｜ calls", "<｜DSML｜ \n", "<｜DSML｜ calls>"] {
            assert!(DeepseekDsmlDecoder::decode([source]).is_err());
        }
        for source in ["<｜DSML｜ calls_extra>", "<｜DSML｜ calls invalid>"] {
            assert_eq!(DeepseekDsmlDecoder::decode([source]).unwrap(), source);
        }
        let source = "<｜DSML｜ calls><｜DSML｜ invoke name='file_info'></｜DSML｜ invoke_extra></｜DSML｜ calls>";
        assert!(DeepseekDsmlDecoder::decode([source]).is_err());
    }

    /// Preserves raw string bytes and parses explicitly nonstring values as strict JSON.
    #[test]
    fn parameter_types_unicode_and_markup_are_preserved() {
        let body = concat!(
            "<｜DSML｜invoke name=\"package_proxy\">",
            "<｜DSML｜parameter name=\"tool_name\" string=\"true\">files:write</｜DSML｜parameter>",
            "<｜DSML｜parameter name=\"text\" string=\"true\">  中文😀 &amp; <x>\"\\n\n  </｜DSML｜parameter>",
            "<｜DSML｜parameter name=\"params\" string=\"false\">{\"list\":[1,true,null],\"text\":\"a < b & c\"}</｜DSML｜parameter>",
            "<｜DSML｜parameter name=\"empty\" string=\"true\"></｜DSML｜parameter>",
            "<｜DSML｜parameter name=\"number\" string=\"false\">42</｜DSML｜parameter>",
            "<｜DSML｜parameter name=\"boolean\" string=\"false\">false</｜DSML｜parameter>",
            "</｜DSML｜invoke>"
        );
        let calls = parse_calls(body, "｜DSML｜").unwrap();
        let arguments: Value =
            serde_json::from_str(calls[0]["function"]["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(arguments["text"], "  中文😀 &amp; <x>\"\\n\n  ");
        assert_eq!(arguments["empty"], "");
        assert_eq!(arguments["number"], 42);
        assert_eq!(arguments["boolean"], false);
        assert_eq!(
            arguments["params"],
            json!({"list": [1, true, null], "text": "a < b & c"})
        );

        let source = format!("<｜DSML｜calls>{body}</｜DSML｜calls>");
        let output = DeepseekDsmlDecoder::decode([source.as_str()]).unwrap();
        assert!(output.find("&amp;amp; &lt;x&gt;&quot;").is_some());
        let replayed = decoded_calls(&output).1;
        let replayed_arguments: Value =
            serde_json::from_str(replayed[0]["function"]["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(replayed_arguments["params"], arguments["params"]);
    }

    /// Rejects invalid JSON, duplicate keys, malformed structure, and missing attributes.
    #[test]
    fn malformed_dsml_returns_a_provider_error() {
        for body in [
            "",
            "<｜DSML｜invoke></｜DSML｜invoke>",
            "<｜DSML｜invoke name=\"x\"></｜DSML｜wrong>",
            "<｜DSML｜invoke name=\"x\"><｜DSML｜parameter name=\"a\" string=\"false\">invalid</｜DSML｜parameter></｜DSML｜invoke>",
            "<｜DSML｜invoke name=\"x\"><｜DSML｜parameter name=\"a\" string=\"maybe\">x</｜DSML｜parameter></｜DSML｜invoke>",
            "<｜DSML｜invoke name=\"x\"><｜DSML｜parameter name=\"a\" string=\"true\">x</｜DSML｜parameter><｜DSML｜parameter name=\"a\" string=\"true\">y</｜DSML｜parameter></｜DSML｜invoke>",
            "<｜DSML｜invoke name=\"x\" name=\"y\"></｜DSML｜invoke>",
            "<｜DSML｜invoke name=\"x\" unexpected=\"y\"></｜DSML｜invoke>",
        ] {
            let source = format!("<｜DSML｜calls>{body}</｜DSML｜calls>");
            assert!(DeepseekDsmlDecoder::decode([source.as_str()]).is_err(), "{source}");
        }
    }

    /// Rejects truncated containers but keeps ordinary trailing token-like text literal.
    #[test]
    fn completion_rejects_truncated_dsml_without_changing_literal_text() {
        for source in [
            "<｜DSML｜calls",
            "<｜DSML｜calls>",
            "<｜DSML｜calls><｜DSML｜invoke name=\"x\">",
        ] {
            assert!(DeepseekDsmlDecoder::decode([source]).is_err());
        }
        for source in [
            "普通文本",
            "a < b",
            "ending <",
            "partial <｜DS",
            "<tool name=\"x\"></tool>",
        ] {
            assert_eq!(DeepseekDsmlDecoder::decode([source]).unwrap(), source);
        }
    }

    /// Keeps reasoning examples literal, including when thinking tags span text deltas.
    #[test]
    fn dsml_in_reasoning_is_not_executed() {
        let source = format!("<think>{SCREENSHOT_CALL}</think>{SCREENSHOT_CALL}");
        let pieces = source
            .char_indices()
            .map(|(offset, ch)| &source[offset..offset + ch.len_utf8()]);
        let output = DeepseekDsmlDecoder::decode(pieces).unwrap();
        assert!(output.starts_with(&format!("<think>{SCREENSHOT_CALL}</think>")));
        assert_eq!(ChatMarkupRegex::tool_call_matches(&output).len(), 1);
    }

    /// Restores a clean parser state when the upstream rolls back a partial attempt.
    #[test]
    fn retry_savepoint_discards_pending_dsml_and_reasoning_state() {
        let mut decoder = DeepseekDsmlDecoder::default();
        let savepoint = decoder.clone();
        assert!(decoder
            .push("<｜DSML｜calls><｜DSML｜invoke name=\"partial\">")
            .unwrap()
            .is_empty());
        decoder.clone_from(&savepoint);
        let output = decoder.push(SCREENSHOT_CALL).unwrap();
        assert_eq!(
            decoded_calls(&output).1[0]["function"]["name"],
            "use_package"
        );
        assert!(decoder.finish().unwrap().is_empty());
    }

    /// Exercises the real nonstreaming provider paths and preserves native tool calls and usage.
    #[test]
    fn buffered_chat_and_responses_convert_dsml_and_preserve_native_calls() {
        use crate::chat::llmprovider::AIService::AIService;
        for responses in [false, true] {
            let endpoint = if responses {
                "responses"
            } else {
                "chat/completions"
            };
            let provider = DeepseekProvider::new(
                format!("https://fixture.invalid/{endpoint}"),
                String::new().into(),
                "deepseek-flash".into(),
                "DEEPSEEK".into(),
                Vec::new(),
                false,
                false,
                false,
                Vec::new(),
                false,
                test_runtime_context(),
            );
            let reasoning = format!("Reasoning example: {SCREENSHOT_CALL}");
            let content = format!(
                "Before{}After",
                SCREENSHOT_CALL.replace("DSML｜｜", "DSML｜｜ ")
            );
            let response = if responses {
                json!({"output": [
                    {"type": "reasoning", "content": [{"type": "reasoning_text", "text": reasoning}]},
                    {"type": "message", "content": [{"type": "output_text", "text": content}]},
                    {"type": "function_call", "name": "native_call", "arguments": "{}"}
                ], "usage": {"input_tokens": 100, "input_tokens_details": {"cached_tokens": 20}, "output_tokens": 30}})
            } else {
                json!({"choices": [{"message": {
                    "reasoning_content": reasoning,
                    "content": content,
                    "tool_calls": [{"type": "function", "function": {"name": "native_call", "arguments": "{}"}}]
                }}], "usage": {"prompt_tokens": 100, "prompt_cache_hit_tokens": 20, "completion_tokens": 30}})
            };
            let output = provider.parse_response_chunks(&response).unwrap().concat();
            let names = ChatMarkupRegex::tool_call_matches(&output)
                .into_iter()
                .map(|call| call.name)
                .collect::<Vec<_>>();
            assert_eq!(names, ["use_package", "native_call"]);
            assert!(output.starts_with(&format!("<think>{reasoning}</think>")));
            assert_eq!(
                (
                    provider.input_token_count(),
                    provider.cached_input_token_count(),
                    provider.output_token_count()
                ),
                (100, 20, 30)
            );
        }
    }
}
