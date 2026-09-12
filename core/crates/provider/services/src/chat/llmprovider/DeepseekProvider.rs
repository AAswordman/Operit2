use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Map, Value};
use std::sync::{Arc, Mutex};

use super::OpenAIProvider::{OpenAIProvider, ResponsesStreamProtocol};
use super::OpenAIResponsesProvider::{
    append_web_search_tool, build_responses_web_search_chunks, responses_metadata_tag,
    strip_responses_reasoning_metadata, OpenAIResponsesPayloadAdapter, ParsedResponseOutput,
    RESPONSES_OUTPUT_ITEM_META_PROVIDER, RESPONSES_REASONING_META_PROVIDER,
};
use super::StructuredToolCallBridge::StructuredToolCallBridge;
use super::ThinkingConfiguration::ThinkingConfigurationApplier;
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
    with_event_channel, RevisableTextStreamLike, TextStreamEventCarrier,
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
    pub api_key: String,
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
        api_key: String,
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
            api_key,
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

    fn clearActiveParent(&self, generation: u64) {
        let mut state = self
            .state
            .lock()
            .expect("DeepseekProvider state mutex poisoned");
        if state.activeParentGeneration == generation {
            state.activeParent = None;
        }
    }

    pub fn create_request_body(
        &self,
        request: &SendMessageRequest,
    ) -> Result<Value, AiServiceError> {
        if self.uses_responses_protocol()? {
            let protocol_history = if request.enable_thinking {
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
            let parent = OpenAIProvider::new_with_capabilities(
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
            let request_object = parent
                .create_request_body_without_thinking_for_history(request, &protocol_history)?;
            let mut responses_request =
                DeepseekResponsesPayloadAdapter::to_responses_request(request_object);
            ThinkingConfigurationApplier::apply(
                &mut responses_request,
                &self.provider_type,
                &self.model_name,
                &self.api_endpoint,
                request.enable_thinking,
                request.thinking_quality_level,
                &request.thinking_configurations,
                &request.thinking_option_id,
            )?;
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
        ThinkingConfigurationApplier::apply(
            &mut request_object,
            &self.provider_type,
            &self.model_name,
            &self.api_endpoint,
            request.enable_thinking,
            request.thinking_quality_level,
            &request.thinking_configurations,
            &request.thinking_option_id,
        )?;
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

    pub fn resolve_deepseek_thinking_effort(&self) -> Result<Option<&'static str>, AiServiceError> {
        let qualityLevel = self
            .runtime_context
            .support()
            .thinkingQualityLevel()
            .map_err(AiServiceError::RequestFailed)?;
        Ok(match qualityLevel.clamp(1, 4) {
            1 | 2 => Some("high"),
            3 | 4 => Some("max"),
            _ => None,
        })
    }

    fn calculate_and_store_input_tokens(
        &self,
        provider_ready_history: &[PromptTurn],
        tools_json: Option<&str>,
        preserve_think_in_history: bool,
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
                true,
            );
            state.inputTokenCount = state.tokenCacheManager.total_input_token_count();
            state.cachedInputTokenCount = state.tokenCacheManager.cached_input_token_count();
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

/// Resolves whether one DeepSeek endpoint selects the Responses protocol.
fn deepseek_uses_responses_protocol(endpoint: &str) -> Result<bool, AiServiceError> {
    let endpoint = url::Url::parse(endpoint)
        .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
    let segment = endpoint
        .path_segments()
        .and_then(|segments| segments.last());
    Ok(segment.is_some_and(|value| value.eq_ignore_ascii_case("responses")))
}

#[cfg(test)]
mod responses_tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use serde_json::{json, Value};

    use super::DeepseekProvider;
    use super::DeepseekResponsesPayloadAdapter;
    use crate::chat::llmprovider::AIService::SendMessageRequest;
    use crate::chat::llmprovider::OpenAIResponsesProvider::OpenAIResponsesPayloadAdapter;
    use crate::runtime_support::{
        ProviderCharacterPromptContext, ProviderFunctionModelBinding, ProviderMessageTiming,
        ProviderPackageInfo, ProviderRuntimeContext, ProviderRuntimeSupport,
        ProviderToolPkgAiProviderRegistration,
    };
    use operit_model::FunctionType::FunctionType;
    use operit_model::MemorySearchConfig::MemorySearchConfig;
    use operit_model::ModelConfigData::{ProviderProfile, ResolvedModelConfig};
    use operit_model::PromptFunctionType::PromptFunctionType;
    use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
    use operit_model::ToolPrompt::ToolPrompt;

    /// Supplies deterministic runtime capabilities for request-shape tests.
    struct TestRuntimeSupport;

    impl ProviderRuntimeSupport for TestRuntimeSupport {
        /// Returns no filesystem root in the isolated test runtime.
        fn dataDir(&self) -> Result<PathBuf, String> {
            Err("test runtime does not expose a data directory".to_string())
        }

        /// Returns a stable thinking quality level for the test runtime.
        fn thinkingQualityLevel(&self) -> Result<i32, String> {
            Ok(2)
        }

        /// Accepts token updates without persisting them.
        fn updateTokensForProviderModel(
            &self,
            _providerModel: &str,
            _inputTokens: i64,
            _outputTokens: i64,
            _cachedInputTokens: i64,
        ) -> Result<(), String> {
            Ok(())
        }

        /// Returns no memory configuration in the isolated test runtime.
        fn memorySearchConfig(&self, _ownerKey: &str) -> Result<MemorySearchConfig, String> {
            Err("test runtime does not expose memory configuration".to_string())
        }

        /// Returns no character prompt in the isolated test runtime.
        fn characterPromptContext(
            &self,
            _roleCardId: &str,
            _promptFunctionType: PromptFunctionType,
        ) -> Result<ProviderCharacterPromptContext, String> {
            Err("test runtime does not expose character prompts".to_string())
        }

        /// Returns no visible skill packages in the isolated test runtime.
        fn aiVisibleSkillPackages(&self) -> Result<Vec<ProviderPackageInfo>, String> {
            Ok(Vec::new())
        }

        /// Returns no function binding in the isolated test runtime.
        fn modelBindingForFunction(
            &self,
            _rootDir: PathBuf,
            _functionType: FunctionType,
        ) -> Result<ProviderFunctionModelBinding, String> {
            Err("test runtime does not expose model bindings".to_string())
        }

        /// Returns no model configuration in the isolated test runtime.
        fn resolvedModelConfig(
            &self,
            _rootDir: PathBuf,
            _providerId: &str,
            _modelId: &str,
        ) -> Result<ResolvedModelConfig, String> {
            Err("test runtime does not expose model configuration".to_string())
        }

        /// Returns no provider profile in the isolated test runtime.
        fn providerProfile(
            &self,
            _rootDir: PathBuf,
            _providerId: &str,
        ) -> Result<ProviderProfile, String> {
            Err("test runtime does not expose provider profiles".to_string())
        }

        /// Reports that no package AI provider is registered in the test runtime.
        fn hasToolPkgAiProvider(&self, _providerId: &str) -> bool {
            false
        }

        /// Returns no package AI provider registration in the test runtime.
        fn toolPkgAiProvider(
            &self,
            _providerId: &str,
        ) -> Option<ProviderToolPkgAiProviderRegistration> {
            None
        }

        /// Rejects package hook execution in the isolated test runtime.
        fn runToolPkgAiProviderHook(
            &self,
            _containerPackageName: &str,
            _functionName: &str,
            _functionSource: Option<&str>,
            _event: &str,
            _tag: Option<String>,
            _sourceKey: Option<String>,
            _eventPayload: Value,
            _runtimeContextKey: Option<String>,
            _executionKind: Option<String>,
            _onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
        ) -> Result<Option<String>, String> {
            Err("test runtime does not execute package hooks".to_string())
        }

        /// Does not decode package hook output in the test runtime.
        fn decodeToolPkgHookResult(&self, _raw: Option<String>) -> Option<Value> {
            None
        }

        /// Returns a zero timestamp for deterministic timing assertions.
        fn messageTimingNow(&self) -> ProviderMessageTiming {
            ProviderMessageTiming { startedAtMs: 0 }
        }

        /// Ignores timing records in the isolated test runtime.
        fn logMessageTiming(
            &self,
            _stage: &str,
            _startTimeMs: ProviderMessageTiming,
            _details: Option<String>,
        ) {
        }
    }

    /// Builds the isolated runtime context used by provider request tests.
    fn test_runtime_context() -> ProviderRuntimeContext {
        ProviderRuntimeContext::new(Arc::new(TestRuntimeSupport))
    }

    /// Builds a request carrying one assistant tool call and its result.
    fn tool_continuation_request(reasoning_metadata: &str) -> SendMessageRequest {
        let assistant_content = format!(
            "<think>Inspect the workspace first.</think>visible\n{reasoning_metadata}\n<tool name=\"list_files\" call_id=\"call_1\"><param name=\"path\">/workspace</param></tool>"
        );
        SendMessageRequest {
            chat_history: vec![
                PromptTurn::new(PromptTurnKind::USER, "List the workspace files."),
                PromptTurn::new(PromptTurnKind::ASSISTANT, assistant_content),
                PromptTurn::new(
                    PromptTurnKind::TOOL_RESULT,
                    "<tool_result name=\"list_files\"><content>workspace result</content></tool_result>",
                ),
                PromptTurn::new(PromptTurnKind::USER, "Continue."),
            ],
            model_parameters: Vec::new(),
            enable_thinking: true,
            thinking_quality_level: 2,
            thinking_configurations: "[]".to_string(),
            thinking_option_id: String::new(),
            stream: false,
            available_tools: vec![ToolPrompt::new(
                "list_files".to_string(),
                "Lists workspace files".to_string(),
            )],
            preserve_think_in_history: true,
            enable_retry: false,
            on_non_fatal_error: None,
            on_tool_invocation: None,
        }
    }

    /// Builds the assistant/tool continuation used by Responses replay tests.
    fn continuation_request(assistant_content: String, call_id: &str) -> Value {
        json!({
            "messages": [
                {
                    "role": "assistant",
                    "content": assistant_content,
                    "tool_calls": [{
                        "id": call_id,
                        "type": "function",
                        "function": {
                            "name": "list_files",
                            "arguments": "{\"path\":\"/workspace\"}"
                        }
                    }]
                },
                {
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": "workspace result"
                }
            ]
        })
    }

    /// Verifies plaintext reasoning is replayed before the function call and result.
    #[test]
    fn plaintext_reasoning_replays_before_function_call() {
        let reasoning_item = json!({
            "type": "reasoning",
            "id": "rs_plain_1",
            "content": [{
                "type": "reasoning_text",
                "text": "Inspect the workspace first."
            }]
        });
        let metadata =
            DeepseekResponsesPayloadAdapter::create_reasoning_metadata_tag(&reasoning_item)
                .expect("reasoning metadata");
        let request = continuation_request(
            format!("<think>Inspect the workspace first.</think>visible{metadata}"),
            "call_plain_1",
        );

        let input = DeepseekResponsesPayloadAdapter::to_responses_request(request)["input"]
            .as_array()
            .expect("Responses input array")
            .clone();
        assert_eq!(input[0]["type"], "reasoning");
        assert_eq!(input[0]["id"], "rs_plain_1");
        assert_eq!(input[0]["content"][0]["type"], "reasoning_text");
        assert_eq!(input[1]["type"], "message");
        assert_eq!(input[1]["content"], "visible");
        assert_eq!(input[2]["type"], "function_call");
        assert_eq!(input[2]["call_id"], "call_plain_1");
        assert_eq!(input[3]["type"], "function_call_output");
        assert_eq!(input[3]["call_id"], "call_plain_1");
    }

    /// Verifies encrypted reasoning is not sent through DeepSeek plaintext replay.
    #[test]
    fn encrypted_reasoning_is_not_replayed_as_plaintext() {
        let parsed = DeepseekResponsesPayloadAdapter::parse_non_streaming_response(&json!({
            "output": [{
                "type": "reasoning",
                "id": "rs_encrypted_1",
                "encrypted_content": "encrypted"
            }]
        }));
        assert!(parsed.reasoningMetadataTags.is_empty());
    }

    /// Verifies commentary continuation is encoded as DeepSeek reasoning text.
    #[test]
    fn commentary_replays_as_reasoning_text() {
        let commentary_item = json!({
            "type": "message",
            "id": "msg_commentary_1",
            "role": "assistant",
            "phase": "commentary",
            "content": [{
                "type": "output_text",
                "text": "Activate the package before calling its tool."
            }]
        });
        let parsed = DeepseekResponsesPayloadAdapter::parse_non_streaming_response(&json!({
            "output": [commentary_item]
        }));
        assert!(parsed.reasoningChunks.is_empty());
        let metadata = parsed
            .outputItemMetadataTags
            .first()
            .expect("commentary metadata")
            .clone();
        let input = DeepseekResponsesPayloadAdapter::to_responses_request(continuation_request(
            metadata,
            "call_commentary_1",
        ))["input"]
            .as_array()
            .expect("Responses input array")
            .clone();
        assert_eq!(input[0]["type"], "reasoning");
        assert_eq!(input[0]["content"][0]["type"], "reasoning_text");
        assert_eq!(
            input[0]["content"][0]["text"],
            "Activate the package before calling its tool."
        );
        assert_eq!(input[1]["type"], "function_call");
        assert_eq!(input[2]["type"], "function_call_output");
    }

    /// Verifies web-search metadata does not remove an unrelated thinking block.
    #[test]
    fn web_search_metadata_does_not_remove_thinking_content() {
        let search_metadata = OpenAIResponsesPayloadAdapter::create_output_item_metadata_tag(
            &json!({"type": "web_search_call", "id": "search_1"}),
        )
        .expect("search metadata");
        let request = continuation_request(
            format!("<think>raw thinking</think>visible{search_metadata}"),
            "call_search_1",
        );
        let input = DeepseekResponsesPayloadAdapter::to_responses_request(request)["input"]
            .as_array()
            .expect("Responses input array")
            .clone();
        let message = input
            .iter()
            .find(|item| item["type"] == "message")
            .expect("assistant message");
        assert_eq!(message["content"], "<think>raw thinking</think>visible");
        assert!(input.iter().any(|item| item["type"] == "function_call"));
        assert!(input
            .iter()
            .any(|item| item["type"] == "function_call_output"));
    }

    /// Verifies the full DeepSeek request builder preserves replay order after tool bridging.
    #[test]
    fn request_builder_preserves_reasoning_tool_and_result_order() {
        let reasoning_item = json!({
            "type": "reasoning",
            "id": "rs_request_1",
            "content": [{
                "type": "reasoning_text",
                "text": "Inspect the workspace first."
            }]
        });
        let reasoning_metadata =
            DeepseekResponsesPayloadAdapter::create_reasoning_metadata_tag(&reasoning_item)
                .expect("reasoning metadata");
        let provider = DeepseekProvider::new(
            "https://api.deepseek.com/v1/responses".to_string(),
            String::new(),
            "deepseek-reasoner".to_string(),
            "DEEPSEEK".to_string(),
            Vec::new(),
            false,
            false,
            false,
            Vec::new(),
            true,
            test_runtime_context(),
        );

        let request = tool_continuation_request(&reasoning_metadata);
        let body = provider
            .create_request_body(&request)
            .expect("DeepSeek Responses request must be buildable");
        let input = body["input"].as_array().expect("Responses input array");

        let reasoning_index = input
            .iter()
            .position(|item| item["type"] == "reasoning")
            .expect("reasoning item must be replayed");
        let function_call_index = input
            .iter()
            .position(|item| item["type"] == "function_call")
            .expect("function call must be bridged");
        let function_output_index = input
            .iter()
            .position(|item| item["type"] == "function_call_output")
            .expect("function result must be bridged");
        assert!(reasoning_index < function_call_index);
        assert!(function_call_index < function_output_index);
        assert_eq!(input[reasoning_index]["id"], "rs_request_1");
        assert_eq!(input[function_call_index]["name"], "list_files");
        assert_eq!(input[function_output_index]["output"], "workspace result");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl AIService for DeepseekProvider {
    fn input_token_count(&self) -> i64 {
        self.state
            .lock()
            .map(|state| state.tokenCacheManager.total_input_token_count())
            .unwrap_or(0)
    }

    fn cached_input_token_count(&self) -> i64 {
        self.state
            .lock()
            .map(|state| state.tokenCacheManager.cached_input_token_count())
            .unwrap_or(0)
    }

    fn output_token_count(&self) -> i64 {
        self.state
            .lock()
            .map(|state| state.tokenCacheManager.output_token_count())
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
            AppLogger::i(
                PROVIDER_TRANSPORT_LOG_TAG,
                &format!(
                    "provider.deepseek.parent.create providerModel={} stream=true",
                    self.provider_model(),
                ),
            );
            let protocol = if self.uses_responses_protocol()? {
                ResponsesStreamProtocol::Deepseek
            } else {
                ResponsesStreamProtocol::OpenAi
            };
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
            )
            .with_responses_stream_protocol(protocol);
            let mut result = parent.send_prepared_request(request, request_body).await?;
            AppLogger::i(
                PROVIDER_TRANSPORT_LOG_TAG,
                &format!(
                    "provider.deepseek.parent.stream.ready providerModel={}",
                    self.provider_model(),
                ),
            );
            let event_channel = result.event_channel().clone();
            let mut provider = self.clone();
            let activeParentGeneration = provider.setActiveParent(parent.clone());
            if provider.isCancelled() {
                parent.cancel_streaming();
            }
            let mut ownedResult = Some(result);
            let mut ownedParent = Some(parent);
            let mut ownedProvider = Some(provider);
            let cold_stream = FnStream::new(move |emit| {
                let mut result = ownedResult
                    .take()
                    .expect("Deepseek parent stream must only be collected once");
                let parent = ownedParent
                    .take()
                    .expect("Deepseek parent stream must only be collected once");
                let mut provider = ownedProvider
                    .take()
                    .expect("Deepseek parent stream must only be collected once");
                Box::pin(async move {
                    result.collect(emit).await;
                    AppLogger::i(
                        PROVIDER_TRANSPORT_LOG_TAG,
                        &format!(
                            "provider.deepseek.parent.stream.done providerModel={} inputTokens={} cachedInputTokens={} outputTokens={}",
                            parent.provider_model(),
                            parent.input_token_count(),
                            parent.cached_input_token_count(),
                            parent.output_token_count(),
                        ),
                    );
                    provider.apply_token_counts(TokenCounts {
                        input: parent.input_token_count(),
                        cached_input: parent.cached_input_token_count(),
                        output: parent.output_token_count(),
                    });
                    provider.clearActiveParent(activeParentGeneration);
                })
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
        if self.uses_responses_protocol()? {
            let parsed =
                DeepseekResponsesPayloadAdapter::parse_non_streaming_response(&json_response);
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
            chunks.extend(parsed.textChunks);
            if let Value::Array(tool_calls) = parsed.toolCalls {
                for tool_call in tool_calls {
                    chunks.push(StructuredToolCallBridge::convertToolCallPayloadToXml(
                        &tool_call.to_string(),
                    ));
                }
            }
            return Ok(response_stream_from_chunks(chunks));
        }

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
        let response = client
            .post(&self.api_endpoint)
            .headers(self.headers()?)
            .json(&request_body)
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
        .unwrap_or(0) as i64;
    let cached_tokens = usage
        .pointer("/prompt_tokens_details/cached_tokens")
        .or_else(|| usage.pointer("/input_tokens_details/cached_tokens"))
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
            StructuredToolCallBridge::convertToolCallPayloadToXml(&tool_call.to_string())
        })
        .filter(|content| operit_util::ChatMarkupRegex::ChatMarkupRegex::contains_tool_tag(content))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::deepseek_uses_responses_protocol;

    /// Selects Responses only from the final endpoint path segment.
    #[test]
    fn resolves_deepseek_endpoint_protocol() {
        assert!(
            deepseek_uses_responses_protocol("https://api.deepseek.com/v1/responses?trace=1")
                .unwrap()
        );
        assert!(
            !deepseek_uses_responses_protocol("https://api.deepseek.com/v1/chat/completions")
                .unwrap()
        );
    }
}
