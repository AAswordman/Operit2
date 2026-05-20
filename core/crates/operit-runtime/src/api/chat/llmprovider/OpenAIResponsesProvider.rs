use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

use super::AIService::{
    response_stream_from_chunks, AIService, AiServiceError, SendMessageRequest,
    TokenCounts,
};
use super::OpenAIProvider::OpenAIProvider;
use super::StructuredToolCallBridge::StructuredToolCallBridge;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::util::stream::RevisableTextStream::{
    with_event_channel, RevisableTextStreamLike, TextStreamEventCarrier,
};
use crate::util::stream::Stream::FnStream;

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
    pub customHeaders: Vec<(String, String)>,
    state: Arc<Mutex<OpenAIResponsesProviderState>>,
}

#[derive(Debug, Default)]
struct OpenAIResponsesProviderState {
    inputTokenCount: i32,
    cachedInputTokenCount: i32,
    outputTokenCount: i32,
    cancelled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageCounts {
    pub totalInputTokens: i32,
    pub actualInputTokens: i32,
    pub cachedInputTokens: i32,
    pub outputTokens: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedResponseOutput {
    pub textChunks: Vec<String>,
    pub reasoningChunks: Vec<String>,
    pub toolCalls: Value,
    pub usage: Option<UsageCounts>,
}

pub struct OpenAIResponsesPayloadAdapter;

impl OpenAIResponsesProvider {
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
        enableToolCall: bool,
    ) -> Self {
        Self {
            responsesApiEndpoint,
            api_key,
            modelName,
            responsesProviderType,
            supportsVision,
            supportsAudio,
            supportsVideo,
            enableToolCall,
            customHeaders,
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

    pub fn create_request_body(
        &self,
        request: &SendMessageRequest,
    ) -> Result<Value, AiServiceError> {
        let parent = OpenAIProvider::new(
            self.responsesApiEndpoint.clone(),
            self.api_key.clone(),
            self.modelName.clone(),
            self.responsesProviderType.clone(),
            self.customHeaders.clone(),
            self.enableToolCall,
        );
        let mut requestObject = OpenAIResponsesPayloadAdapter::to_responses_request(
            parent.create_request_body(request)?,
        );
        self.apply_responses_reasoning_effort(&mut requestObject, request.enable_thinking);

        let messagesArray = requestObject
            .get("input")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let toolsJson = requestObject.get("tools").map(Value::to_string);
        self.customize_final_request_object(&mut requestObject, &messagesArray, toolsJson.as_deref());
        Ok(requestObject)
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
    ) {
        if !enableThinking {
            return;
        }
        let Some(object) = requestJson.as_object_mut() else {
            return;
        };
        match object.get("reasoning") {
            Some(Value::Object(reasoning)) if reasoning.get("effort").and_then(Value::as_str).is_some_and(|value| !value.trim().is_empty()) => {
                return;
            }
            Some(Value::Object(_)) | None => {}
            Some(_) => return,
        }

        let Some(effort) = self.resolve_responses_reasoning_effort() else {
            return;
        };
        let reasoningObject = object
            .entry("reasoning".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(reasoning) = reasoningObject {
            reasoning.insert("effort".to_string(), json!(effort));
        }
    }

    fn resolve_responses_reasoning_effort(&self) -> Option<&'static str> {
        let qualityLevel = ApiPreferences::getInstance()
            .thinkingQualityLevelFlow()
            .first()
            .ok()?;
        match qualityLevel.clamp(1, 4) {
            1 => Some("low"),
            2 => Some("medium"),
            3 => Some("high"),
            4 => Some("xhigh"),
            _ => None,
        }
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
                    messageObject.get("content").map(Value::to_string).unwrap_or_default()
                ));
                continue;
            }

            if role == "user" {
                anchorParts.push(format!(
                    "{}:{}",
                    role,
                    messageObject.get("content").map(Value::to_string).unwrap_or_default()
                ));
                break;
            }
        }

        if anchorParts.is_empty() && assistantOrToolSeen {
            if let Some(firstMessage) = messagesArray.first().and_then(Value::as_object) {
                anchorParts.push(format!(
                    "{}:{}",
                    firstMessage.get("role").and_then(Value::as_str).unwrap_or("unknown"),
                    firstMessage.get("content").map(Value::to_string).unwrap_or_default()
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
        let hex = digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
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
        let totalInputTokens = opt_i32(usage, "prompt_tokens")
            .unwrap_or_else(|| opt_i32(usage, "input_tokens").unwrap_or(0));
        let outputTokens = opt_i32(usage, "completion_tokens")
            .unwrap_or_else(|| opt_i32(usage, "output_tokens").unwrap_or(0));
        let cachedDetails = usage
            .get("prompt_tokens_details")
            .or_else(|| usage.get("input_tokens_details"));
        let cachedInputTokens = cachedDetails
            .and_then(|details| opt_i32(details, "cached_tokens"))
            .unwrap_or_else(|| opt_i32(usage, "cached_tokens").unwrap_or(0));
        let actualInputTokens = (totalInputTokens - cachedInputTokens).max(0);

        if totalInputTokens > 0 || outputTokens > 0 || cachedInputTokens > 0 {
            Some(UsageCounts {
                totalInputTokens,
                actualInputTokens,
                cachedInputTokens,
                outputTokens,
            })
        } else {
            None
        }
    }

    pub fn to_responses_request(chatStyleRequest: Value) -> Value {
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

            if let Some(Value::Array(tools)) = object.get("tools") {
                object.insert(
                    "tools".to_string(),
                    Value::Array(Self::convert_tools_to_responses_format(tools)),
                );
            }

            if let Some(Value::Array(messages)) = object.remove("messages") {
                object.insert(
                    "input".to_string(),
                    Value::Array(Self::convert_messages_to_responses_input(&messages)),
                );
            }
        }
        converted
    }

    pub fn parse_non_streaming_response(jsonResponse: &Value) -> ParsedResponseOutput {
        let mut textChunks = Vec::new();
        let mut reasoningChunks = Vec::new();
        let mut toolCalls = Vec::new();

        if let Some(output) = jsonResponse.get("output").and_then(Value::as_array) {
            for item in output {
                let itemType = item.get("type").and_then(Value::as_str).unwrap_or_default();
                match itemType {
                    "message" => {
                        if let Some(contentArray) = item.get("content").and_then(Value::as_array) {
                            for part in contentArray {
                                match part.get("type").and_then(Value::as_str).unwrap_or_default() {
                                    "output_text" | "text" => {
                                        if let Some(text) = part.get("text").and_then(Value::as_str) {
                                            if !text.is_empty() {
                                                textChunks.push(text.to_string());
                                            }
                                        }
                                    }
                                    "reasoning_text" => {
                                        if let Some(text) = part.get("text").and_then(Value::as_str) {
                                            if !text.is_empty() {
                                                reasoningChunks.push(text.to_string());
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    "reasoning" => {
                        if let Some(summaryArray) = item.get("summary").and_then(Value::as_array) {
                            for summaryPart in summaryArray {
                                if let Some(text) = summaryPart.get("text").and_then(Value::as_str) {
                                    if !text.is_empty() {
                                        reasoningChunks.push(text.to_string());
                                    }
                                }
                            }
                        }
                    }
                    "function_call" => {
                        if let Some(toolCall) = Self::convert_function_call_item_to_chat_tool_call(item) {
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
                json!(function.get("name").and_then(Value::as_str).unwrap_or_default()),
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

    fn convert_messages_to_responses_input(messages: &[Value]) -> Vec<Value> {
        let mut input = Vec::new();
        for message in messages {
            let Some(messageObject) = message.as_object() else {
                continue;
            };
            let role = messageObject.get("role").and_then(Value::as_str).unwrap_or_default();
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
                        "output": Self::extract_tool_output_text(messageObject.get("content")),
                    }));
                    continue;
                }
            }

            if role == "assistant" {
                if let Some(toolCalls) = messageObject.get("tool_calls").and_then(Value::as_array) {
                    for call in toolCalls {
                        let Some(function) = call.get("function").and_then(Value::as_object) else {
                            continue;
                        };
                        let name = function.get("name").and_then(Value::as_str).unwrap_or_default();
                        if name.is_empty() {
                            continue;
                        }
                        let mut callItem = Map::new();
                        callItem.insert("type".to_string(), json!("function_call"));
                        callItem.insert("name".to_string(), json!(name));
                        callItem.insert(
                            "arguments".to_string(),
                            json!(function.get("arguments").and_then(Value::as_str).unwrap_or("{}")),
                        );
                        if let Some(callId) = call.get("id").and_then(Value::as_str) {
                            if !callId.is_empty() {
                                callItem.insert("call_id".to_string(), json!(callId));
                            }
                        }
                        input.push(Value::Object(callItem));
                    }
                }
            }

            let convertedContent = Self::convert_message_content_for_responses(messageObject.get("content"));
            let hasContent = match &convertedContent {
                Value::String(value) => !value.trim().is_empty(),
                Value::Array(value) => !value.is_empty(),
                _ => false,
            };
            if hasContent {
                input.push(json!({
                    "type": "message",
                    "role": if role == "system" { "developer" } else { role },
                    "content": convertedContent,
                }));
            }
        }
        input
    }

    fn convert_message_content_for_responses(content: Option<&Value>) -> Value {
        match content {
            None | Some(Value::Null) => json!(""),
            Some(Value::String(value)) => json!(value),
            Some(Value::Array(parts)) => {
                let mut convertedParts = Vec::new();
                for part in parts {
                    let partType = part.get("type").and_then(Value::as_str).unwrap_or_default();
                    match partType {
                        "text" | "output_text" | "input_text" => {
                            if let Some(text) = part.get("text").and_then(Value::as_str) {
                                if !text.is_empty() {
                                    convertedParts.push(json!({"type": "input_text", "text": text}));
                                }
                            }
                        }
                        "image_url" | "input_image" => {
                            let imageUrl = if partType == "input_image" {
                                part.get("image_url").and_then(Value::as_str).unwrap_or_default()
                            } else {
                                part.pointer("/image_url/url")
                                    .and_then(Value::as_str)
                                    .or_else(|| part.get("image_url").and_then(Value::as_str))
                                    .unwrap_or_default()
                            };
                            if !imageUrl.is_empty() {
                                convertedParts.push(json!({"type": "input_image", "image_url": imageUrl}));
                            }
                        }
                        "input_audio" => {
                            if let Some(audioObject) = part.get("input_audio") {
                                convertedParts.push(json!({"type": "input_audio", "input_audio": audioObject}));
                            }
                        }
                        _ => {
                            if let Some(text) = part.get("text").and_then(Value::as_str) {
                                if !text.is_empty() {
                                    convertedParts.push(json!({"type": "input_text", "text": text}));
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
        root.insert("function".to_string(), json!({
            "name": name,
            "arguments": arguments,
        }));
        Some(Value::Object(root))
    }
}

#[async_trait]
impl AIService for OpenAIResponsesProvider {
    fn input_token_count(&self) -> i32 {
        self.state
            .lock()
            .map(|state| state.inputTokenCount)
            .unwrap_or(0)
    }

    fn cached_input_token_count(&self) -> i32 {
        self.state
            .lock()
            .map(|state| state.cachedInputTokenCount)
            .unwrap_or(0)
    }

    fn output_token_count(&self) -> i32 {
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
        let requestBody = self.create_request_body(&request)?;
        if request.stream {
            let mut parent = OpenAIProvider::new(
                self.responsesApiEndpoint.clone(),
                self.api_key.clone(),
                self.modelName.clone(),
                self.responsesProviderType.clone(),
                self.customHeaders.clone(),
                self.enableToolCall,
            );
            let mut parent_stream = parent
                .send_prepared_request(request, requestBody)
                .await?;
            let event_channel = parent_stream.event_channel().clone();
            let mut provider = self.clone();
            let cold_stream = FnStream::new(move |emit| {
                parent_stream.collect(&mut |content| {
                    emit(content);
                });
                provider.apply_usage_counts(&UsageCounts {
                    totalInputTokens: parent.input_token_count() + parent.cached_input_token_count(),
                    actualInputTokens: parent.input_token_count(),
                    cachedInputTokens: parent.cached_input_token_count(),
                    outputTokens: parent.output_token_count(),
                });
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
            return Err(AiServiceError::RequestFailed(format!("{status}: {message}")));
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
        chunks.extend(parsed.textChunks);
        if let Value::Array(toolCalls) = parsed.toolCalls {
            for toolCall in toolCalls {
                chunks.push(StructuredToolCallBridge::convertToolCallPayloadToXml(&toolCall.to_string()));
            }
        }

        Ok(response_stream_from_chunks(chunks))
    }
}

fn opt_i32(value: &Value, key: &str) -> Option<i32> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .and_then(|number| i32::try_from(number).ok())
}
