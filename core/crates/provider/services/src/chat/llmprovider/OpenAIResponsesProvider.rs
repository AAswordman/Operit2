use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

use super::OpenAIProvider::OpenAIProvider;
use super::StructuredToolCallBridge::StructuredToolCallBridge;
use super::ThinkingConfiguration::ThinkingConfigurationApplier;
use crate::chat::llmprovider::AIService::{
    response_stream_from_chunks, AIService, AiServiceError, SendMessageRequest, TokenCounts,
};
use crate::runtime_support::ProviderRuntimeContext;
use operit_model::ModelConfigData::{BuiltinToolRequestFormat, ModelBuiltinTool};
use operit_util::stream::RevisableTextStream::{
    with_event_channel, RevisableTextStreamLike, TextStreamEventCarrier,
};
use operit_util::stream::Stream::{FnStream, Stream};
use operit_util::ChatMarkupRegex::{attr_value, tag_body, tag_ranges};
use operit_util::ChatUtils::ChatUtils;

pub(crate) const RESPONSES_REASONING_META_PROVIDER: &str = "openai:responses_reasoning";
pub(crate) const RESPONSES_OUTPUT_ITEM_META_PROVIDER: &str = "openai:responses_output_item";

#[derive(Clone)]
pub struct OpenAIResponsesProvider {
    pub responsesApiEndpoint: String,
    pub api_key: String,
    pub modelName: String,
    pub responsesProviderType: String,
    pub supportsVision: bool,
    pub supportsAudio: bool,
    pub supportsVideo: bool,
    pub enableToolCall: bool,
    pub builtinTools: Vec<ModelBuiltinTool>,
    pub customHeaders: Vec<(String, String)>,
    runtimeContext: ProviderRuntimeContext,
    state: Arc<Mutex<OpenAIResponsesProviderState>>,
}

#[derive(Default)]
struct OpenAIResponsesProviderState {
    inputTokenCount: i64,
    cachedInputTokenCount: i64,
    outputTokenCount: i64,
    cancelled: bool,
    activeParent: Option<OpenAIProvider>,
    activeParentGeneration: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageCounts {
    pub totalInputTokens: i64,
    pub actualInputTokens: i64,
    pub cachedInputTokens: i64,
    pub outputTokens: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedResponseOutput {
    pub textChunks: Vec<String>,
    pub reasoningChunks: Vec<String>,
    pub reasoningMetadataTags: Vec<String>,
    pub outputItemMetadataTags: Vec<String>,
    pub reasoningObserved: bool,
    pub searchChunks: Vec<String>,
    pub toolCalls: Value,
    pub usage: Option<UsageCounts>,
}

pub struct OpenAIResponsesPayloadAdapter;

#[derive(Clone, Copy, Eq, PartialEq)]
enum ResponsesHistoryProtocol {
    OpenAi,
    Deepseek,
}

impl OpenAIResponsesProvider {
    /// Creates a Responses API provider bound to one provider runtime context.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        responsesApiEndpoint: String,
        api_key: String,
        modelName: String,
        responsesProviderType: String,
        customHeaders: Vec<(String, String)>,
        supportsVision: bool,
        supportsAudio: bool,
        supportsVideo: bool,
        builtinTools: Vec<ModelBuiltinTool>,
        enableToolCall: bool,
        runtimeContext: ProviderRuntimeContext,
    ) -> Self {
        Self {
            responsesApiEndpoint,
            api_key,
            modelName,
            responsesProviderType,
            supportsVision,
            supportsAudio,
            supportsVideo,
            builtinTools,
            enableToolCall,
            customHeaders,
            runtimeContext,
            state: Arc::new(Mutex::new(OpenAIResponsesProviderState::default())),
        }
    }

    fn apply_usage_counts(&self, usage: &UsageCounts) {
        if let Ok(mut state) = self.state.lock() {
            state.inputTokenCount = usage.actualInputTokens;
            state.cachedInputTokenCount = usage.cachedInputTokens;
            state.outputTokenCount = usage.outputTokens;
        }
    }

    fn setActiveParent(&self, parent: OpenAIProvider) -> u64 {
        let mut state = self
            .state
            .lock()
            .expect("OpenAIResponsesProvider state mutex poisoned");
        state.activeParentGeneration = state.activeParentGeneration.wrapping_add(1);
        state.activeParent = Some(parent);
        state.activeParentGeneration
    }

    fn isCancelled(&self) -> bool {
        self.state
            .lock()
            .expect("OpenAIResponsesProvider state mutex poisoned")
            .cancelled
    }

    fn clearActiveParent(&self, generation: u64) {
        let mut state = self
            .state
            .lock()
            .expect("OpenAIResponsesProvider state mutex poisoned");
        if state.activeParentGeneration == generation {
            state.activeParent = None;
        }
    }

    pub fn create_request_body(
        &self,
        request: &SendMessageRequest,
    ) -> Result<Value, AiServiceError> {
        let parent = OpenAIProvider::new_with_capabilities(
            self.responsesApiEndpoint.clone(),
            self.api_key.clone(),
            self.modelName.clone(),
            "OPENAI_RESPONSES_PARENT".to_string(),
            self.customHeaders.clone(),
            self.supportsVision,
            self.supportsAudio,
            self.supportsVideo,
            self.enableToolCall,
        );
        let request_history = if request.enable_thinking {
            request.chat_history.clone()
        } else {
            request
                .chat_history
                .iter()
                .map(|turn| turn.with_content(strip_responses_reasoning_metadata(&turn.content)))
                .collect()
        };
        let mut requestObject = OpenAIResponsesPayloadAdapter::to_responses_request(
            parent.create_request_body_without_thinking_for_history(request, &request_history)?,
        );
        ThinkingConfigurationApplier::apply(
            &mut requestObject,
            &self.responsesProviderType,
            &self.modelName,
            &self.responsesApiEndpoint,
            request.enable_thinking,
            request.thinking_quality_level,
            &request.thinking_configurations,
            &request.thinking_option_id,
        )?;

        let messagesArray = requestObject
            .get("input")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if self.open_ai_web_search_enabled() {
            append_web_search_tool(&mut requestObject);
        }
        let toolsJson = requestObject.get("tools").map(Value::to_string);
        self.customize_final_request_object(
            &mut requestObject,
            &messagesArray,
            toolsJson.as_deref(),
        );
        Ok(requestObject)
    }

    /// Returns whether the model configuration enables Responses web search.
    fn open_ai_web_search_enabled(&self) -> bool {
        self.builtinTools.iter().any(|tool| {
            tool.enabled && tool.requestFormat == BuiltinToolRequestFormat::OpenAiWebSearch
        })
    }

    pub fn customize_final_request_object(
        &self,
        requestObject: &mut Value,
        messagesArray: &[Value],
        toolsJson: Option<&str>,
    ) {
        if !self.should_attach_prompt_cache_key() {
            return;
        }
        let Some(object) = requestObject.as_object_mut() else {
            return;
        };
        if object.contains_key("prompt_cache_key") {
            return;
        }
        let Some(promptCacheKey) = self.build_prompt_cache_key(messagesArray, toolsJson) else {
            return;
        };
        object.insert("prompt_cache_key".to_string(), json!(promptCacheKey));
    }

    pub fn apply_responses_reasoning_effort(
        &self,
        requestJson: &mut Value,
        enableThinking: bool,
    ) -> Result<(), AiServiceError> {
        if !enableThinking {
            return Ok(());
        }
        let Some(object) = requestJson.as_object_mut() else {
            return Ok(());
        };
        match object.get("reasoning") {
            Some(Value::Object(reasoning))
                if reasoning
                    .get("effort")
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty()) =>
            {
                return Ok(());
            }
            Some(Value::Object(_)) | None => {}
            Some(_) => return Ok(()),
        }

        let Some(effort) = self.resolve_responses_reasoning_effort()? else {
            return Ok(());
        };
        let reasoningObject = object
            .entry("reasoning".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(reasoning) = reasoningObject {
            reasoning.insert("effort".to_string(), json!(effort));
        }
        Ok(())
    }

    fn resolve_responses_reasoning_effort(&self) -> Result<Option<&'static str>, AiServiceError> {
        let qualityLevel = self
            .runtimeContext
            .support()
            .thinkingQualityLevel()
            .map_err(AiServiceError::RequestFailed)?;
        Ok(match qualityLevel.clamp(1, 4) {
            1 => Some("low"),
            2 => Some("medium"),
            3 => Some("high"),
            4 => Some("xhigh"),
            _ => None,
        })
    }

    fn should_attach_prompt_cache_key(&self) -> bool {
        self.responsesProviderType == "OPENAI_RESPONSES"
    }

    fn build_prompt_cache_key(
        &self,
        messagesArray: &[Value],
        toolsJson: Option<&str>,
    ) -> Option<String> {
        if messagesArray.is_empty() && toolsJson.is_none_or(str::is_empty) {
            return None;
        }

        let mut anchorParts = Vec::new();
        let mut assistantOrToolSeen = false;

        for message in messagesArray {
            let Some(messageObject) = message.as_object() else {
                continue;
            };
            let role = messageObject
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if role.is_empty() {
                continue;
            }

            if role == "assistant" || role == "tool" {
                assistantOrToolSeen = true;
                break;
            }

            if role == "system" || role == "developer" {
                anchorParts.push(format!(
                    "{}:{}",
                    role,
                    messageObject
                        .get("content")
                        .map(Value::to_string)
                        .unwrap_or_default()
                ));
                continue;
            }

            if role == "user" {
                anchorParts.push(format!(
                    "{}:{}",
                    role,
                    messageObject
                        .get("content")
                        .map(Value::to_string)
                        .unwrap_or_default()
                ));
                break;
            }
        }

        if anchorParts.is_empty() && assistantOrToolSeen {
            if let Some(firstMessage) = messagesArray.first().and_then(Value::as_object) {
                anchorParts.push(format!(
                    "{}:{}",
                    firstMessage
                        .get("role")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                    firstMessage
                        .get("content")
                        .map(Value::to_string)
                        .unwrap_or_default()
                ));
            }
        }

        let mut digestInput = String::new();
        digestInput.push_str("operit:responses_prompt_cache:v1");
        digestInput.push_str("|model=");
        digestInput.push_str(&self.modelName);
        digestInput.push_str("|toolCall=");
        digestInput.push_str(if self.enableToolCall { "true" } else { "false" });
        if let Some(toolsJson) = toolsJson {
            if !toolsJson.trim().is_empty() {
                digestInput.push_str("|tools=");
                digestInput.push_str(toolsJson);
            }
        }
        for part in anchorParts {
            digestInput.push_str("|anchor=");
            digestInput.push_str(&part);
        }

        let digest = Sha256::digest(digestInput.as_bytes());
        let hex = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Some(format!("operit_resp_{}", &hex[..48]))
    }

    fn headers(&self) -> Result<HeaderMap, AiServiceError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if !self.api_key.trim().is_empty() {
            let value = format!("Bearer {}", self.api_key);
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&value)
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?,
            );
        }
        for (name, value) in &self.customHeaders {
            headers.insert(
                HeaderName::from_bytes(name.as_bytes())
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?,
                HeaderValue::from_str(value)
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?,
            );
        }
        Ok(headers)
    }
}

impl OpenAIResponsesPayloadAdapter {
    pub fn map_parameter_name_for_responses(apiName: &str) -> String {
        match apiName {
            "max_tokens" => "max_output_tokens".to_string(),
            _ => apiName.to_string(),
        }
    }

    pub fn parse_usage_counts(usage: Option<&Value>) -> Option<UsageCounts> {
        let usage = usage?;
        let hasInput = usage.get("prompt_tokens").is_some() || usage.get("input_tokens").is_some();
        let hasOutput =
            usage.get("completion_tokens").is_some() || usage.get("output_tokens").is_some();
        let cachedDetails = usage
            .get("prompt_tokens_details")
            .or_else(|| usage.get("input_tokens_details"));
        let hasCached = usage.get("cached_tokens").is_some()
            || cachedDetails
                .and_then(|details| details.get("cached_tokens"))
                .is_some();
        if !hasInput && !hasOutput && !hasCached {
            return None;
        }
        let totalInputTokens = opt_i64(usage, "prompt_tokens")
            .or_else(|| opt_i64(usage, "input_tokens"))
            .unwrap_or(0)
            .max(0);
        let outputTokens = opt_i64(usage, "completion_tokens")
            .or_else(|| opt_i64(usage, "output_tokens"))
            .unwrap_or(0)
            .max(0);
        let cachedInputTokens = cachedDetails
            .and_then(|details| opt_i64(details, "cached_tokens"))
            .or_else(|| opt_i64(usage, "cached_tokens"))
            .unwrap_or(0)
            .max(0);
        let actualInputTokens = (totalInputTokens - cachedInputTokens).max(0);

        Some(UsageCounts {
            totalInputTokens,
            actualInputTokens,
            cachedInputTokens,
            outputTokens,
        })
    }

    pub fn to_responses_request(chatStyleRequest: Value) -> Value {
        Self::to_responses_request_for_protocol(chatStyleRequest, ResponsesHistoryProtocol::OpenAi)
    }

    /// Converts chat messages using DeepSeek's plaintext reasoning replay contract.
    pub(crate) fn to_deepseek_responses_request(chatStyleRequest: Value) -> Value {
        Self::to_responses_request_for_protocol(
            chatStyleRequest,
            ResponsesHistoryProtocol::Deepseek,
        )
    }

    /// Converts one chat-style request into the selected Responses history protocol.
    fn to_responses_request_for_protocol(
        chatStyleRequest: Value,
        protocol: ResponsesHistoryProtocol,
    ) -> Value {
        let mut converted = chatStyleRequest;
        if let Value::Object(object) = &mut converted {
            if object.contains_key("max_tokens") && !object.contains_key("max_output_tokens") {
                if let Some(maxTokens) = object.remove("max_tokens") {
                    object.insert("max_output_tokens".to_string(), maxTokens);
                }
            }

            if let Some(responseFormat) = object.remove("response_format") {
                let textConfig = object
                    .entry("text".to_string())
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Value::Object(textObject) = textConfig {
                    textObject.insert("format".to_string(), responseFormat);
                }
            }

            if let Some(reasoningEffort) = object.remove("reasoning_effort") {
                let reasoning = object
                    .entry("reasoning".to_string())
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Value::Object(reasoningObject) = reasoning {
                    if !reasoningObject.contains_key("effort") {
                        reasoningObject.insert("effort".to_string(), reasoningEffort);
                    }
                }
            }

            if let Some(Value::Array(tools)) = object.get("tools") {
                object.insert(
                    "tools".to_string(),
                    Value::Array(Self::convert_tools_to_responses_format(tools)),
                );
            }

            if let Some(Value::Array(messages)) = object.remove("messages") {
                object.insert(
                    "input".to_string(),
                    Value::Array(Self::convert_messages_to_responses_input(
                        &messages, protocol,
                    )),
                );
            }
        }
        converted
    }

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
                let itemType = item.get("type").and_then(Value::as_str).unwrap_or_default();
                match itemType {
                    "message" => {
                        let isCommentaryMessage = item
                            .get("phase")
                            .and_then(Value::as_str)
                            .is_some_and(|phase| phase.trim().eq_ignore_ascii_case("commentary"));
                        if let Some(contentArray) = item.get("content").and_then(Value::as_array) {
                            for part in contentArray {
                                match part.get("type").and_then(Value::as_str).unwrap_or_default() {
                                    "output_text" | "text" => {
                                        if let Some(text) = part.get("text").and_then(Value::as_str)
                                        {
                                            if !text.is_empty() {
                                                if isCommentaryMessage {
                                                    reasoningObserved = true;
                                                    reasoningChunks.push(text.to_string());
                                                } else {
                                                    textChunks.push(text.to_string());
                                                }
                                            }
                                        }
                                    }
                                    "reasoning_text" => {
                                        if let Some(text) = part.get("text").and_then(Value::as_str)
                                        {
                                            if !text.is_empty() {
                                                reasoningObserved = true;
                                                reasoningChunks.push(text.to_string());
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    "web_search_call" => {
                        if let Some(metadataTag) = Self::create_output_item_metadata_tag(item) {
                            outputItemMetadataTags.push(metadataTag);
                        }
                        searchItems.push(item.clone());
                    }
                    "reasoning" => {
                        reasoningObserved = true;
                        if let Some(metadataTag) = Self::create_reasoning_metadata_tag(item) {
                            reasoningMetadataTags.push(metadataTag);
                        }
                        if let Some(summaryArray) = item.get("summary").and_then(Value::as_array) {
                            for summaryPart in summaryArray {
                                if let Some(text) = summaryPart.get("text").and_then(Value::as_str)
                                {
                                    if !text.is_empty() {
                                        reasoningChunks.push(text.to_string());
                                    }
                                }
                            }
                        }
                    }
                    "function_call" => {
                        if let Some(toolCall) =
                            Self::convert_function_call_item_to_chat_tool_call(item)
                        {
                            toolCalls.push(toolCall);
                        }
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
            usage: Self::parse_usage_counts(jsonResponse.get("usage")),
        }
    }

    fn convert_tools_to_responses_format(chatTools: &[Value]) -> Vec<Value> {
        let mut converted = Vec::new();
        for tool in chatTools {
            if tool.get("type").and_then(Value::as_str) != Some("function") {
                converted.push(tool.clone());
                continue;
            }
            let Some(function) = tool.get("function").and_then(Value::as_object) else {
                converted.push(tool.clone());
                continue;
            };
            let mut convertedFunction = Map::new();
            convertedFunction.insert("type".to_string(), json!("function"));
            convertedFunction.insert(
                "name".to_string(),
                json!(function
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()),
            );
            for key in ["description", "parameters", "strict"] {
                if let Some(value) = function.get(key) {
                    convertedFunction.insert(key.to_string(), value.clone());
                }
            }
            converted.push(Value::Object(convertedFunction));
        }
        converted
    }

    fn convert_messages_to_responses_input(
        messages: &[Value],
        protocol: ResponsesHistoryProtocol,
    ) -> Vec<Value> {
        let mut input = Vec::new();
        for message in messages {
            let Some(messageObject) = message.as_object() else {
                continue;
            };
            let role = messageObject
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if role.is_empty() {
                continue;
            }

            if role == "tool" {
                let callId = messageObject
                    .get("tool_call_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !callId.is_empty() {
                    input.push(json!({
                        "type": "function_call_output",
                        "call_id": callId,
                        "output": Self::extract_tool_output_content(messageObject.get("content")),
                    }));
                    continue;
                }
            }

            if role == "assistant" {
                let reasoningItems = match protocol {
                    ResponsesHistoryProtocol::OpenAi => {
                        Self::extract_reasoning_items_from_message(messageObject)
                    }
                    ResponsesHistoryProtocol::Deepseek => {
                        Self::extract_deepseek_reasoning_items_from_message(messageObject)
                    }
                };
                let outputItems = match protocol {
                    ResponsesHistoryProtocol::OpenAi => {
                        Self::extract_output_items_from_message(messageObject)
                    }
                    ResponsesHistoryProtocol::Deepseek => {
                        Self::extract_deepseek_output_items_from_message(messageObject)
                    }
                };
                let removeThinkingContent = protocol == ResponsesHistoryProtocol::Deepseek
                    && (!reasoningItems.is_empty()
                        || Self::contains_deepseek_commentary_metadata(messageObject));
                input.extend(reasoningItems);
                input.extend(outputItems);

                if protocol == ResponsesHistoryProtocol::Deepseek {
                    let convertedContent = Self::convert_message_content_for_responses(
                        messageObject.get("content"),
                        removeThinkingContent,
                    );
                    if responses_content_is_not_empty(&convertedContent) {
                        input.push(json!({
                            "type": "message",
                            "role": "assistant",
                            "content": convertedContent,
                        }));
                    }
                }

                if let Some(toolCalls) = messageObject.get("tool_calls").and_then(Value::as_array) {
                    for call in toolCalls {
                        let Some(function) = call.get("function").and_then(Value::as_object) else {
                            continue;
                        };
                        let name = function
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        if name.is_empty() {
                            continue;
                        }
                        let mut callItem = Map::new();
                        callItem.insert("type".to_string(), json!("function_call"));
                        callItem.insert("name".to_string(), json!(name));
                        callItem.insert(
                            "arguments".to_string(),
                            json!(function
                                .get("arguments")
                                .and_then(Value::as_str)
                                .unwrap_or("{}")),
                        );
                        if let Some(callId) = call.get("id").and_then(Value::as_str) {
                            if !callId.is_empty() {
                                callItem.insert("call_id".to_string(), json!(callId));
                            }
                        }
                        input.push(Value::Object(callItem));
                    }
                }
                if protocol == ResponsesHistoryProtocol::Deepseek {
                    continue;
                }
            }

            let convertedContent =
                Self::convert_message_content_for_responses(messageObject.get("content"), false);
            if responses_content_is_not_empty(&convertedContent) {
                input.push(json!({
                    "type": "message",
                    "role": if role == "system" { "developer" } else { role },
                    "content": convertedContent,
                }));
            }
        }
        input
    }

    fn convert_message_content_for_responses(
        content: Option<&Value>,
        removeThinkingContent: bool,
    ) -> Value {
        match content {
            None | Some(Value::Null) => json!(""),
            Some(Value::String(value)) => {
                json!(sanitize_responses_message_text(
                    value,
                    removeThinkingContent
                ))
            }
            Some(Value::Array(parts)) => {
                let mut convertedParts = Vec::new();
                for part in parts {
                    let partType = part.get("type").and_then(Value::as_str).unwrap_or_default();
                    match partType {
                        "text" | "output_text" | "input_text" => {
                            if let Some(text) = part.get("text").and_then(Value::as_str) {
                                if !text.is_empty() {
                                    let text = sanitize_responses_message_text(
                                        text,
                                        removeThinkingContent,
                                    );
                                    if !text.is_empty() {
                                        convertedParts
                                            .push(json!({"type": "input_text", "text": text}));
                                    }
                                }
                            }
                        }
                        "image_url" | "input_image" => {
                            let imageUrl = if partType == "input_image" {
                                part.get("image_url")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                            } else {
                                part.pointer("/image_url/url")
                                    .and_then(Value::as_str)
                                    .or_else(|| part.get("image_url").and_then(Value::as_str))
                                    .unwrap_or_default()
                            };
                            if !imageUrl.is_empty() {
                                convertedParts
                                    .push(json!({"type": "input_image", "image_url": imageUrl}));
                            }
                        }
                        "input_audio" => {
                            if let Some(audioObject) = part.get("input_audio") {
                                convertedParts.push(
                                    json!({"type": "input_audio", "input_audio": audioObject}),
                                );
                            }
                        }
                        _ => {
                            if let Some(text) = part.get("text").and_then(Value::as_str) {
                                if !text.is_empty() {
                                    let text = sanitize_responses_message_text(
                                        text,
                                        removeThinkingContent,
                                    );
                                    if !text.is_empty() {
                                        convertedParts
                                            .push(json!({"type": "input_text", "text": text}));
                                    }
                                }
                            }
                        }
                    }
                }
                Value::Array(convertedParts)
            }
            Some(value) => json!(value.to_string()),
        }
    }

    /// Encodes one OpenAI encrypted reasoning item for stateless replay.
    pub fn create_reasoning_metadata_tag(item: &Value) -> Option<String> {
        if item.get("type").and_then(Value::as_str) != Some("reasoning") {
            return None;
        }
        let reasoningId = item.get("id").and_then(Value::as_str)?.trim();
        let encryptedContent = item
            .get("encrypted_content")
            .and_then(Value::as_str)?
            .trim();
        if reasoningId.is_empty() || encryptedContent.is_empty() {
            return None;
        }
        let summary = item
            .get("summary")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Some(responses_metadata_tag(
            RESPONSES_REASONING_META_PROVIDER,
            &json!({
                "reasoning_id": reasoningId,
                "encrypted_content": encryptedContent,
                "summary": summary,
            }),
        ))
    }

    /// Encodes one Responses web-search output item for stateless replay.
    pub fn create_output_item_metadata_tag(item: &Value) -> Option<String> {
        if item.get("type").and_then(Value::as_str) != Some("web_search_call") {
            return None;
        }
        let id = item.get("id").and_then(Value::as_str)?.trim();
        if id.is_empty() {
            return None;
        }
        Some(responses_metadata_tag(
            RESPONSES_OUTPUT_ITEM_META_PROVIDER,
            item,
        ))
    }

    /// Restores OpenAI encrypted reasoning items from assistant message metadata.
    fn extract_reasoning_items_from_message(message: &Map<String, Value>) -> Vec<Value> {
        extract_responses_metadata_from_content(
            message.get("content"),
            RESPONSES_REASONING_META_PROVIDER,
        )
        .into_iter()
        .filter_map(|metadata| {
            let reasoningId = metadata.get("reasoning_id")?.as_str()?.trim();
            let encryptedContent = metadata.get("encrypted_content")?.as_str()?.trim();
            if reasoningId.is_empty() || encryptedContent.is_empty() {
                return None;
            }
            Some(json!({
                "type": "reasoning",
                "id": reasoningId,
                "encrypted_content": encryptedContent,
                "summary": metadata
                    .get("summary")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
            }))
        })
        .collect()
    }

    /// Restores Responses web-search output items from assistant message metadata.
    fn extract_output_items_from_message(message: &Map<String, Value>) -> Vec<Value> {
        extract_responses_metadata_from_content(
            message.get("content"),
            RESPONSES_OUTPUT_ITEM_META_PROVIDER,
        )
        .into_iter()
        .filter(|metadata| {
            metadata.get("type").and_then(Value::as_str) == Some("web_search_call")
                && metadata
                    .get("id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| !id.trim().is_empty())
        })
        .collect()
    }

    /// Restores DeepSeek plaintext reasoning items from assistant metadata.
    fn extract_deepseek_reasoning_items_from_message(message: &Map<String, Value>) -> Vec<Value> {
        extract_responses_metadata_from_content(
            message.get("content"),
            RESPONSES_REASONING_META_PROVIDER,
        )
        .into_iter()
        .filter_map(|metadata| {
            let reasoningId = metadata.get("reasoning_id")?.as_str()?.trim();
            let content = metadata.get("content")?.as_array()?;
            if reasoningId.is_empty() || !contains_reasoning_text(content) {
                return None;
            }
            Some(json!({
                "type": "reasoning",
                "id": reasoningId,
                "content": content,
            }))
        })
        .collect()
    }

    /// Restores DeepSeek commentary and web-search output metadata.
    fn extract_deepseek_output_items_from_message(message: &Map<String, Value>) -> Vec<Value> {
        extract_responses_metadata_from_content(
            message.get("content"),
            RESPONSES_OUTPUT_ITEM_META_PROVIDER,
        )
        .into_iter()
        .filter_map(
            |metadata| match metadata.get("type").and_then(Value::as_str) {
                Some("web_search_call") => {
                    let valid = metadata
                        .get("id")
                        .and_then(Value::as_str)
                        .is_some_and(|id| !id.trim().is_empty());
                    valid.then_some(metadata)
                }
                Some("message") => {
                    if metadata.get("role").and_then(Value::as_str) != Some("assistant") {
                        return None;
                    }
                    let reasoningContent =
                        commentary_to_reasoning_content(metadata.get("content")?.as_array()?);
                    if reasoningContent.is_empty() {
                        return None;
                    }
                    let mut item = Map::new();
                    item.insert("type".to_string(), json!("reasoning"));
                    if let Some(id) = metadata
                        .get("id")
                        .and_then(Value::as_str)
                        .filter(|id| !id.trim().is_empty())
                    {
                        item.insert("id".to_string(), json!(id));
                    }
                    item.insert("content".to_string(), Value::Array(reasoningContent));
                    Some(Value::Object(item))
                }
                _ => None,
            },
        )
        .collect()
    }

    /// Returns whether DeepSeek continuation metadata contains a commentary message.
    fn contains_deepseek_commentary_metadata(message: &Map<String, Value>) -> bool {
        extract_responses_metadata_from_content(
            message.get("content"),
            RESPONSES_OUTPUT_ITEM_META_PROVIDER,
        )
        .into_iter()
        .any(|metadata| {
            metadata.get("type").and_then(Value::as_str) == Some("message")
                && metadata.get("role").and_then(Value::as_str) == Some("assistant")
                && metadata
                    .get("content")
                    .and_then(Value::as_array)
                    .is_some_and(|content| !commentary_to_reasoning_content(content).is_empty())
        })
    }

    fn extract_tool_output_text(content: Option<&Value>) -> String {
        match content {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(value)) => value.clone(),
            Some(Value::Array(parts)) => {
                let mut textParts = Vec::new();
                for part in parts {
                    let partType = part.get("type").and_then(Value::as_str).unwrap_or_default();
                    if matches!(partType, "text" | "output_text" | "input_text") {
                        if let Some(text) = part.get("text").and_then(Value::as_str) {
                            if !text.is_empty() {
                                textParts.push(text.to_string());
                            }
                        }
                    }
                }
                if textParts.is_empty() {
                    Value::Array(parts.clone()).to_string()
                } else {
                    textParts.join("\n")
                }
            }
            Some(value) => value.to_string(),
        }
    }

    /// Converts a tool result into a Responses-compatible string or rich content value.
    fn extract_tool_output_content(content: Option<&Value>) -> Value {
        match content {
            Some(Value::Array(_)) => {
                let converted = Self::convert_message_content_for_responses(content, false);
                if responses_content_is_not_empty(&converted) {
                    converted
                } else {
                    json!(Self::extract_tool_output_text(content))
                }
            }
            Some(Value::String(value)) => {
                json!(sanitize_responses_message_text(value, false))
            }
            _ => json!(Self::extract_tool_output_text(content)),
        }
    }

    fn convert_function_call_item_to_chat_tool_call(item: &Value) -> Option<Value> {
        let name = item.get("name").and_then(Value::as_str).unwrap_or_default();
        if name.is_empty() {
            return None;
        }
        let arguments = item
            .get("arguments")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("{}");
        let callId = item
            .get("call_id")
            .and_then(Value::as_str)
            .or_else(|| item.get("id").and_then(Value::as_str))
            .unwrap_or_default();
        let mut root = Map::new();
        if !callId.is_empty() {
            root.insert("id".to_string(), json!(callId));
        }
        root.insert("type".to_string(), json!("function"));
        root.insert(
            "function".to_string(),
            json!({
                "name": name,
                "arguments": arguments,
            }),
        );
        Some(Value::Object(root))
    }
}

/// Checks whether a Responses content value contains visible input content.
fn responses_content_is_not_empty(content: &Value) -> bool {
    match content {
        Value::String(value) => !value.trim().is_empty(),
        Value::Array(value) => !value.is_empty(),
        _ => false,
    }
}

/// Removes thinking markup before stripping Responses-only protocol metadata.
fn sanitize_responses_message_text(content: &str, removeThinkingContent: bool) -> String {
    let visibleContent = if removeThinkingContent {
        ChatUtils::remove_thinking_content(content)
    } else {
        content.to_string()
    };
    strip_responses_protocol_markup(&visibleContent)
}

/// Returns whether one Responses content array contains reasoning text.
fn contains_reasoning_text(content: &[Value]) -> bool {
    content.iter().any(|part| {
        part.get("type").and_then(Value::as_str) == Some("reasoning_text")
            && part
                .get("text")
                .and_then(Value::as_str)
                .is_some_and(|text| !text.is_empty())
    })
}

/// Converts DeepSeek commentary output text into Responses reasoning text parts.
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

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl AIService for OpenAIResponsesProvider {
    fn input_token_count(&self) -> i64 {
        self.state
            .lock()
            .map(|state| state.inputTokenCount)
            .unwrap_or(0)
    }

    fn cached_input_token_count(&self) -> i64 {
        self.state
            .lock()
            .map(|state| state.cachedInputTokenCount)
            .unwrap_or(0)
    }

    fn output_token_count(&self) -> i64 {
        self.state
            .lock()
            .map(|state| state.outputTokenCount)
            .unwrap_or(0)
    }

    fn provider_model(&self) -> String {
        format!("{}:{}", self.responsesProviderType, self.modelName)
    }

    fn reset_token_counts(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.inputTokenCount = 0;
            state.cachedInputTokenCount = 0;
            state.outputTokenCount = 0;
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
        let requestBody = self.create_request_body(&request)?;
        if request.stream {
            let mut parent = OpenAIProvider::new_with_capabilities(
                self.responsesApiEndpoint.clone(),
                self.api_key.clone(),
                self.modelName.clone(),
                self.responsesProviderType.clone(),
                self.customHeaders.clone(),
                self.supportsVision,
                self.supportsAudio,
                self.supportsVideo,
                self.enableToolCall,
            );
            let mut parent_stream = parent.send_prepared_request(request, requestBody).await?;
            let event_channel = parent_stream.event_channel().clone();
            let mut provider = self.clone();
            let activeParentGeneration = provider.setActiveParent(parent.clone());
            if provider.isCancelled() {
                parent.cancel_streaming();
            }
            let mut ownedParentStream = Some(parent_stream);
            let mut ownedParent = Some(parent);
            let mut ownedProvider = Some(provider);
            let cold_stream = FnStream::new(move |emit| {
                let mut parentStream = ownedParentStream
                    .take()
                    .expect("OpenAI Responses parent stream must only be collected once");
                let parent = ownedParent
                    .take()
                    .expect("OpenAI Responses parent stream must only be collected once");
                let mut provider = ownedProvider
                    .take()
                    .expect("OpenAI Responses parent stream must only be collected once");
                Box::pin(async move {
                    parentStream.collect(emit).await;
                    provider.apply_usage_counts(&UsageCounts {
                        totalInputTokens: parent.input_token_count()
                            + parent.cached_input_token_count(),
                        actualInputTokens: parent.input_token_count(),
                        cachedInputTokens: parent.cached_input_token_count(),
                        outputTokens: parent.output_token_count(),
                    });
                    provider.clearActiveParent(activeParentGeneration);
                })
            });
            return Ok(Box::new(with_event_channel(cold_stream, event_channel)));
        }

        let response = reqwest::Client::new()
            .post(&self.responsesApiEndpoint)
            .headers(self.headers()?)
            .json(&requestBody)
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

        let jsonResponse: Value = response
            .json()
            .await
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let parsed = OpenAIResponsesPayloadAdapter::parse_non_streaming_response(&jsonResponse);
        if let Some(usage) = parsed.usage {
            self.apply_usage_counts(&usage);
        }

        let mut chunks = Vec::new();
        for reasoning in parsed.reasoningChunks {
            chunks.push(format!("<think>{reasoning}</think>"));
        }
        chunks.extend(parsed.reasoningMetadataTags);
        chunks.extend(parsed.outputItemMetadataTags);
        chunks.extend(parsed.searchChunks);
        chunks.extend(parsed.textChunks);
        if let Value::Array(toolCalls) = parsed.toolCalls {
            for toolCall in toolCalls {
                chunks.push(StructuredToolCallBridge::convertToolCallPayloadToXml(
                    &toolCall.to_string(),
                ));
            }
        }

        Ok(response_stream_from_chunks(chunks))
    }
}

fn opt_i64(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

/// Adds the official Responses web-search tool declaration once.
pub fn append_web_search_tool(request: &mut Value) {
    let object = request
        .as_object_mut()
        .expect("Responses request must remain a JSON object");
    let tools = object
        .entry("tools".to_string())
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .expect("Responses tools must be a JSON array");
    if !tools
        .iter()
        .any(|tool| tool.get("type").and_then(Value::as_str) == Some("web_search"))
    {
        tools.push(json!({"type": "web_search"}));
    }
    object.insert("tool_choice".to_string(), json!("auto"));
}

/// Builds one structured search block from Responses output items and citations.
pub fn build_responses_web_search_chunks(items: &[Value], response: &Value) -> Vec<String> {
    if items.is_empty() {
        return Vec::new();
    }
    let mut queries = Vec::new();
    let mut sources = Vec::new();
    let mut status = String::new();
    let mut action_type = String::new();
    for item in items {
        if let Some(value) = item.get("status").and_then(Value::as_str) {
            status = value.to_string();
        }
        if let Some(action) = item.get("action") {
            if let Some(value) = action.get("type").and_then(Value::as_str) {
                action_type = value.to_string();
            }
            collect_search_queries(action, &mut queries);
            collect_search_sources(action, &mut sources);
        }
    }
    collect_response_citations(response, &mut sources);
    let mut xml = format!(
        "<search provider=\"responses\" action=\"{}\" status=\"{}\">",
        escape_xml_attribute(&action_type),
        escape_xml_attribute(&status)
    );
    for query in queries {
        xml.push_str("<query>");
        xml.push_str(&escape_xml_text(&query));
        xml.push_str("</query>");
    }
    for source in sources {
        let title = source.get("title").and_then(Value::as_str).unwrap_or("");
        let url = source.get("url").and_then(Value::as_str).unwrap_or("");
        if url.is_empty() {
            continue;
        }
        xml.push_str(&format!(
            "<source title=\"{}\" url=\"{}\" />",
            escape_xml_attribute(title),
            escape_xml_attribute(url)
        ));
    }
    xml.push_str("</search>");
    vec![xml]
}

/// Encodes one Responses protocol value into hidden chat metadata.
pub(crate) fn responses_metadata_tag(provider: &str, item: &Value) -> String {
    let payload = BASE64_STANDARD.encode(item.to_string().as_bytes());
    format!("<meta provider=\"{provider}\">{payload}</meta>")
}

/// Decodes one Responses metadata provider from string or rich message content.
pub(crate) fn extract_responses_metadata_from_content(
    content: Option<&Value>,
    provider: &str,
) -> Vec<Value> {
    match content {
        Some(Value::String(content)) => extract_responses_metadata(content, provider),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .flat_map(|text| extract_responses_metadata(text, provider))
            .collect(),
        _ => Vec::new(),
    }
}

/// Decodes one Responses metadata provider from assistant protocol text.
fn extract_responses_metadata(content: &str, provider: &str) -> Vec<Value> {
    let mut items = Vec::new();
    for (start, end) in tag_ranges(content, "meta") {
        let tag = &content[start..end];
        if !attr_value(tag, "provider").is_some_and(|value| value.eq_ignore_ascii_case(provider)) {
            continue;
        }
        let Some(payload) = tag_body(tag, "meta") else {
            continue;
        };
        let Ok(decoded) = BASE64_STANDARD.decode(payload.trim()) else {
            continue;
        };
        let Ok(item) = serde_json::from_slice::<Value>(&decoded) else {
            continue;
        };
        items.push(item);
    }
    items
}

/// Removes Responses metadata and search presentation blocks from model input text.
pub(crate) fn strip_responses_protocol_markup(content: &str) -> String {
    let mut ranges = tag_ranges(content, "search");
    for (start, end) in tag_ranges(content, "meta") {
        let tag = &content[start..end];
        if attr_value(tag, "provider").is_some_and(|provider| {
            provider.eq_ignore_ascii_case(RESPONSES_REASONING_META_PROVIDER)
                || provider.eq_ignore_ascii_case(RESPONSES_OUTPUT_ITEM_META_PROVIDER)
        }) {
            ranges.push((start, end));
        }
    }
    ranges.sort_by_key(|range| range.0);
    let mut output = String::new();
    let mut cursor = 0;
    for (start, end) in ranges {
        if start >= cursor {
            output.push_str(&content[cursor..start]);
            cursor = end;
        }
    }
    output.push_str(&content[cursor..]);
    output.trim().to_string()
}

/// Removes only Responses reasoning metadata while retaining other protocol records.
pub(crate) fn strip_responses_reasoning_metadata(content: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    let mut removed = false;
    for (start, end) in tag_ranges(content, "meta") {
        let tag = &content[start..end];
        let is_reasoning = attr_value(tag, "provider").is_some_and(|provider| {
            provider.eq_ignore_ascii_case(RESPONSES_REASONING_META_PROVIDER)
        });
        if is_reasoning {
            output.push_str(&content[cursor..start]);
            cursor = end;
            removed = true;
        }
    }
    output.push_str(&content[cursor..]);
    if removed {
        output.trim_end().to_string()
    } else {
        content.to_string()
    }
}

/// Collects search query strings from a Responses action object.
fn collect_search_queries(action: &Value, queries: &mut Vec<String>) {
    if let Some(values) = action.get("queries").and_then(Value::as_array) {
        for value in values {
            if let Some(query) = value.as_str() {
                push_unique_string(queries, query);
            }
        }
    }
    if let Some(query) = action.get("query").and_then(Value::as_str) {
        push_unique_string(queries, query);
    }
}

/// Collects structured sources from a Responses search action.
fn collect_search_sources(action: &Value, sources: &mut Vec<Value>) {
    if let Some(values) = action.get("sources").and_then(Value::as_array) {
        for value in values {
            push_unique_source(sources, value);
        }
    }
    if let Some(url) = action.get("url").and_then(Value::as_str) {
        push_unique_source(
            sources,
            &json!({"url": url, "title": action.get("title").and_then(Value::as_str).unwrap_or("")}),
        );
    }
}

/// Collects URL citations from final Responses message content.
fn collect_response_citations(response: &Value, sources: &mut Vec<Value>) {
    let Some(output) = response.get("output").and_then(Value::as_array) else {
        return;
    };
    for item in output {
        let Some(content) = item.get("content").and_then(Value::as_array) else {
            continue;
        };
        for part in content {
            let Some(annotations) = part.get("annotations").and_then(Value::as_array) else {
                continue;
            };
            for annotation in annotations {
                if annotation.get("type").and_then(Value::as_str) == Some("url_citation") {
                    push_unique_source(sources, annotation);
                }
            }
        }
    }
}

/// Appends one non-empty string while retaining response order.
fn push_unique_string(values: &mut Vec<String>, value: &str) {
    let value = value.trim();
    if !value.is_empty() && !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

/// Appends one URL source while retaining response order.
fn push_unique_source(values: &mut Vec<Value>, value: &Value) {
    let Some(url) = value.get("url").and_then(Value::as_str) else {
        return;
    };
    if url.is_empty()
        || values
            .iter()
            .any(|existing| existing.get("url").and_then(Value::as_str) == Some(url))
    {
        return;
    }
    values.push(value.clone());
}

/// Escapes text for XML element content.
fn escape_xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Escapes text for a quoted XML attribute.
fn escape_xml_attribute(value: &str) -> String {
    escape_xml_text(value)
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::{
        build_responses_web_search_chunks, extract_responses_metadata,
        strip_responses_protocol_markup, OpenAIResponsesPayloadAdapter, UsageCounts,
        RESPONSES_OUTPUT_ITEM_META_PROVIDER,
    };
    use serde_json::json;

    /// Renders search queries, citations, and replay metadata together.
    #[test]
    fn renders_responses_web_search_output() {
        let item = json!({
            "type": "web_search_call",
            "id": "search_1",
            "status": "completed",
            "action": {
                "type": "search",
                "queries": ["DeepSeek V4"],
                "sources": [{"title": "DeepSeek", "url": "https://deepseek.com"}]
            }
        });
        let chunks = build_responses_web_search_chunks(&[item.clone()], &json!({}));
        assert!(chunks[0].contains("<query>DeepSeek V4</query>"));
        assert!(chunks[0].contains("url=\"https://deepseek.com\""));
        assert_eq!(chunks.len(), 1);
        let metadata = OpenAIResponsesPayloadAdapter::create_output_item_metadata_tag(&item)
            .expect("web search item metadata");
        assert_eq!(
            extract_responses_metadata(&metadata, RESPONSES_OUTPUT_ITEM_META_PROVIDER),
            vec![item]
        );
    }

    /// Removes search presentation and hidden replay metadata from model text.
    #[test]
    fn strips_responses_search_protocol_markup() {
        let chunks = build_responses_web_search_chunks(
            &[json!({"type": "web_search_call", "id": "search_1"})],
            &json!({}),
        );
        let content = format!("before{}after", chunks.join(""));
        assert_eq!(strip_responses_protocol_markup(&content), "beforeafter");
    }

    /// Preserves an explicitly reported all-zero usage payload.
    #[test]
    fn parses_zero_usage_payload() {
        assert_eq!(
            OpenAIResponsesPayloadAdapter::parse_usage_counts(Some(&json!({
                "input_tokens": 0,
                "output_tokens": 0
            }))),
            Some(UsageCounts {
                totalInputTokens: 0,
                actualInputTokens: 0,
                cachedInputTokens: 0,
                outputTokens: 0,
            })
        );
    }
}
