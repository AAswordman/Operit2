use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use serde_json::{json, Map, Value};
use std::sync::{Arc, Mutex};

use super::AIService::{
    delay_retry_ms, response_stream_from_chunks, retry_error_text, retry_message, AIService,
    AiServiceError, SendMessageRequest, TokenCounts,
};
use super::OpenAIProvider::{StreamingJsonXmlConverter, StreamingJsonXmlEvent};
use super::StructuredToolCallBridge::StructuredToolCallBridge;
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_model::ToolPrompt::ToolPrompt;
use operit_util::stream::RevisableTextStream::{
    empty_revisable_event_channel, with_event_channel, RevisableTextStreamLike,
};
use operit_util::stream::Stream::FnStream;
use operit_util::ChatMarkupRegex::ChatMarkupRegex;

#[derive(Clone)]
pub struct ClaudeProvider {
    pub api_endpoint: String,
    pub api_key: String,
    pub model_name: String,
    pub provider_type: String,
    pub enable_tool_call: bool,
    pub custom_headers: Vec<(String, String)>,
    state: Arc<Mutex<ClaudeProviderState>>,
}

#[derive(Debug, Default)]
struct ClaudeProviderState {
    inputTokenCount: i32,
    cachedInputTokenCount: i32,
    outputTokenCount: i32,
    cancelled: bool,
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
            state: Arc::new(Mutex::new(ClaudeProviderState::default())),
        }
    }

    fn set_cancelled(&self, cancelled: bool) {
        self.state
            .lock()
            .expect("ClaudeProvider state mutex poisoned")
            .cancelled = cancelled;
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
        state.inputTokenCount = token_counts.input;
        state.cachedInputTokenCount = token_counts.cached_input;
        state.outputTokenCount = token_counts.output;
    }

    async fn waitUntilCancelled(self) {
        loop {
            if self.is_cancelled() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }

    async fn sendHttpRequest(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, AiServiceError> {
        if self.is_cancelled() {
            return Err(AiServiceError::RequestCancelled);
        }
        tokio::select! {
            response = request.send() => response.map_err(|error| AiServiceError::ConnectionFailed(error.to_string())),
            _ = self.clone().waitUntilCancelled() => Err(AiServiceError::RequestCancelled),
        }
    }

    async fn readResponseText(
        &self,
        response: reqwest::Response,
    ) -> Result<String, AiServiceError> {
        if self.is_cancelled() {
            return Err(AiServiceError::RequestCancelled);
        }
        tokio::select! {
            text = response.text() => text.map_err(|error| AiServiceError::ConnectionFailed(error.to_string())),
            _ = self.clone().waitUntilCancelled() => Err(AiServiceError::RequestCancelled),
        }
    }

    async fn readResponseJson(&self, response: reqwest::Response) -> Result<Value, AiServiceError> {
        if self.is_cancelled() {
            return Err(AiServiceError::RequestCancelled);
        }
        tokio::select! {
            json = response.json() => json.map_err(|error| AiServiceError::RequestFailed(error.to_string())),
            _ = self.clone().waitUntilCancelled() => Err(AiServiceError::RequestCancelled),
        }
    }

    async fn delayRetryOrCancel(&self, retryAttempt: i32) -> Result<(), AiServiceError> {
        if self.is_cancelled() {
            return Err(AiServiceError::RequestCancelled);
        }
        let delayMs = super::LlmRetryPolicy::LlmRetryPolicy::nextDelayMs(retryAttempt);
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_millis(delayMs as u64)) => Ok(()),
            _ = self.clone().waitUntilCancelled() => Err(AiServiceError::RequestCancelled),
        }
    }

    pub fn create_request_body(
        &self,
        request: &SendMessageRequest,
    ) -> Result<Value, AiServiceError> {
        let (system, messages) = self.build_messages_and_count_tokens(&request.chat_history)?;
        let mut object = Map::new();
        object.insert("model".to_string(), json!(self.model_name));
        object.insert("messages".to_string(), Value::Array(messages));
        object.insert("stream".to_string(), json!(request.stream));
        if !system.is_null() {
            object.insert("system".to_string(), system);
        }
        if !object.contains_key("max_tokens") {
            object.insert("max_tokens".to_string(), json!(4096));
        }
        if self.enable_tool_call && !request.available_tools.is_empty() {
            object.insert(
                "tools".to_string(),
                self.build_tool_definitions_for_claude(&request.available_tools)?,
            );
        }
        self.add_parameters(&mut object, &request.model_parameters);
        self.apply_thinking_format(&mut object, request);
        self.apply_stable_cache_breakpoints(&mut object);
        Ok(Value::Object(object))
    }

    pub fn build_messages_and_count_tokens(
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
        let mut open_tool_use_ids: Vec<String> = Vec::new();
        for turn in provider_ready_history {
            match turn.kind {
                PromptTurnKind::SYSTEM | PromptTurnKind::SUMMARY => {
                    system_parts.push(turn.content.clone())
                }
                PromptTurnKind::USER => messages.push(
                    json!({"role": "user", "content": self.build_content_array(&turn.content)}),
                ),
                PromptTurnKind::ASSISTANT | PromptTurnKind::TOOL_CALL => {
                    let content = if self.enable_tool_call {
                        self.build_assistant_content_blocks(
                            &turn.content,
                            &mut next_tool_use_ordinal,
                            &mut open_tool_use_ids,
                        )?
                    } else {
                        self.build_content_array(&turn.content)
                    };
                    messages.push(json!({"role": "assistant", "content": content}))
                }
                PromptTurnKind::TOOL_RESULT => {
                    let content = if self.enable_tool_call {
                        self.build_tool_result_blocks(&turn.content, &mut open_tool_use_ids)
                    } else {
                        self.build_content_array(&turn.content)
                    };
                    messages.push(json!({"role": "user", "content": content}))
                }
            }
        }
        let system = if system_parts.is_empty() {
            Value::Null
        } else {
            Value::Array(
                system_parts
                    .into_iter()
                    .map(|text| {
                        json!({
                            "type": "text",
                            "text": text,
                            "cache_control": {"type": "ephemeral"}
                        })
                    })
                    .collect(),
            )
        };
        Ok((system, messages))
    }

    pub fn apply_stable_cache_breakpoints(&self, _request_object: &mut Map<String, Value>) {}

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

    fn build_content_array(&self, text: &str) -> Value {
        json!([{"type": "text", "text": text}])
    }

    fn build_assistant_content_blocks(
        &self,
        content: &str,
        next_tool_use_ordinal: &mut usize,
        open_tool_use_ids: &mut Vec<String>,
    ) -> Result<Value, AiServiceError> {
        let matches = ChatMarkupRegex::tool_call_matches(content);
        if matches.is_empty() {
            return Ok(self.build_content_array(content));
        }
        let mut text_content = content.to_string();
        let mut blocks = Vec::new();
        for tool_match in matches.iter() {
            text_content = text_content.replace(
                &format!(
                    "<{} name=\"{}\">{}</{}>",
                    tool_match.tag_name, tool_match.name, tool_match.body, tool_match.tag_name
                ),
                "",
            );
            let mut input = Map::new();
            for (start, end) in operit_util::ChatMarkupRegex::tag_ranges(&tool_match.body, "param")
            {
                let raw = &tool_match.body[start..end];
                let name =
                    operit_util::ChatMarkupRegex::attr_value(raw, "name").unwrap_or_default();
                let value = raw
                    .split_once('>')
                    .and_then(|(_, tail)| {
                        tail.rsplit_once("</").map(|(body, _)| xml_unescape(body))
                    })
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                input.insert(name, json!(value));
            }
            let call_id = generated_tool_use_id(*next_tool_use_ordinal);
            *next_tool_use_ordinal += 1;
            open_tool_use_ids.push(call_id.clone());
            blocks.push(json!({
                "type": "tool_use",
                "id": call_id,
                "name": tool_match.name,
                "input": Value::Object(input),
            }));
        }
        let trimmed = text_content.trim();
        let mut content_blocks = Vec::new();
        if !trimmed.is_empty() {
            content_blocks.push(json!({"type": "text", "text": trimmed}));
        }
        content_blocks.extend(blocks);
        Ok(Value::Array(content_blocks))
    }

    fn build_tool_result_blocks(
        &self,
        content: &str,
        open_tool_use_ids: &mut Vec<String>,
    ) -> Value {
        let blocks = ChatMarkupRegex::tool_result_blocks(content);
        if blocks.is_empty() {
            return self.build_content_array(content);
        }
        let mut result_blocks = Vec::new();
        for (index, block) in blocks.iter().enumerate() {
            let tool_use_id = if !open_tool_use_ids.is_empty() {
                open_tool_use_ids.remove(0)
            } else {
                generated_tool_use_id(index)
            };
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
                "tool_use_id": tool_use_id,
                "content": result_content,
            }));
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

    fn apply_thinking_format(
        &self,
        json_object: &mut Map<String, Value>,
        request: &SendMessageRequest,
    ) {
        if !request.enable_thinking {
            return;
        }

        let thinking = match self.thinking_format() {
            ClaudeThinkingFormat::Adaptive => json!({
                "type": "adaptive",
                "display": "summarized"
            }),
            ClaudeThinkingFormat::Enabled => json!({
                "type": "enabled",
                "budget_tokens": self.thinking_budget_tokens(json_object)
            }),
        };
        json_object.insert("thinking".to_string(), thinking);
    }

    fn thinking_format(&self) -> ClaudeThinkingFormat {
        let model_name = normalize_claude_model_name(&self.model_name);
        if has_claude_family(&model_name, "fable")
            || has_claude_family(&model_name, "mythos")
            || has_claude_family_at_least(&model_name, "opus", 4, 6)
            || has_claude_family_at_least(&model_name, "sonnet", 4, 6)
        {
            ClaudeThinkingFormat::Adaptive
        } else {
            ClaudeThinkingFormat::Enabled
        }
    }

    fn thinking_budget_tokens(&self, json_object: &Map<String, Value>) -> i64 {
        let max_tokens = json_object
            .get("max_tokens")
            .and_then(Value::as_i64)
            .filter(|value| *value > 0);
        match max_tokens {
            Some(value) => value.min(1024),
            None => 1024,
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

    fn apply_usage(&mut self, usage: Option<&Value>) -> TokenCounts {
        let cached_input = usage
            .and_then(|value| {
                value
                    .get("cache_read_input_tokens")
                    .or_else(|| value.pointer("/input_tokens_details/cached_tokens"))
                    .or_else(|| value.get("cached_tokens"))
            })
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0) as i32;
        let cache_creation = usage
            .and_then(|value| value.get("cache_creation_input_tokens"))
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0) as i32;
        let input_base = usage
            .and_then(|value| value.get("input_tokens"))
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0) as i32;
        let input = input_base + cache_creation;
        let output = usage
            .and_then(|value| value.get("output_tokens"))
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32;
        let token_counts = TokenCounts {
            input,
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
    fn input_token_count(&self) -> i32 {
        self.state
            .lock()
            .expect("ClaudeProvider state mutex poisoned")
            .inputTokenCount
    }
    fn cached_input_token_count(&self) -> i32 {
        self.state
            .lock()
            .expect("ClaudeProvider state mutex poisoned")
            .cachedInputTokenCount
    }
    fn output_token_count(&self) -> i32 {
        self.state
            .lock()
            .expect("ClaudeProvider state mutex poisoned")
            .outputTokenCount
    }
    fn provider_model(&self) -> String {
        format!("{}:{}", self.provider_type, self.model_name)
    }
    fn reset_token_counts(&mut self) {
        self.set_token_counts(TokenCounts {
            input: 0,
            cached_input: 0,
            output: 0,
        });
    }
    fn cancel_streaming(&mut self) {
        self.set_cancelled(true);
    }

    async fn send_message(
        &mut self,
        request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        self.set_cancelled(false);
        self.reset_token_counts();
        if request.stream {
            let event_channel = empty_revisable_event_channel();
            let streamEventChannel = event_channel.clone();
            let mut provider = self.clone();
            let mut ownedRequest = Some(request);
            let coldStream = FnStream::new(move |emit| {
                let request = ownedRequest
                    .take()
                    .expect("ClaudeProvider stream must only be collected once");
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("tokio runtime must build for ClaudeProvider stream");
                if let Ok(mut responseStream) =
                    runtime.block_on(provider.send_streaming_message(request))
                {
                    responseStream.collect(emit);
                }
                streamEventChannel.close();
            });
            return Ok(Box::new(with_event_channel(coldStream, event_channel)));
        }
        let event_channel = empty_revisable_event_channel();
        let streamEventChannel = event_channel.clone();
        let mut provider = self.clone();
        let mut ownedRequest = Some(request);
        let coldStream = FnStream::new(move |emit| {
            let request = ownedRequest
                .take()
                .expect("ClaudeProvider non-stream request must only be collected once");
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio runtime must build for ClaudeProvider non-stream request");
            if let Ok(mut responseStream) =
                runtime.block_on(provider.send_non_streaming_message(request))
            {
                responseStream.collect(emit);
            }
            streamEventChannel.close();
        });
        Ok(Box::new(with_event_channel(coldStream, event_channel)))
    }

    async fn calculate_input_tokens(
        &self,
        chat_history: &[PromptTurn],
        available_tools: &[ToolPrompt],
    ) -> Result<i32, AiServiceError> {
        let history_chars: usize = chat_history.iter().map(|turn| turn.content.len()).sum();
        let tool_chars: usize = available_tools
            .iter()
            .map(|tool| tool.name.len() + tool.description.len())
            .sum();
        Ok(((history_chars + tool_chars + 3) / 4) as i32)
    }
}

impl ClaudeProvider {
    async fn send_streaming_message(
        &mut self,
        request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        let maxRetries = super::LlmRetryPolicy::LlmRetryPolicy::MAX_RETRY_ATTEMPTS;
        let mut retryCount = 0;
        loop {
            let request_body = self.create_request_body(&request)?;
            let response = match self
                .sendHttpRequest(
                    reqwest::Client::new()
                        .post(&self.api_endpoint)
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
            match self.process_streaming_response(response).await {
                Ok(responseStream) => return Ok(responseStream),
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
                    reqwest::Client::new()
                        .post(&self.api_endpoint)
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
            let token_counts = self.apply_usage(json_response.get("usage"));
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

    async fn process_streaming_response(
        &mut self,
        response: reqwest::Response,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        let mut chunks = Vec::new();
        let mut token_counts = TokenCounts {
            input: 0,
            cached_input: 0,
            output: 0,
        };
        let mut pending_line = String::new();
        let mut bytes_stream = response.bytes_stream();
        let mut current_tool_parser: Option<StreamingJsonXmlConverter> = None;
        let mut current_tool_tag_name: Option<String> = None;
        let mut is_in_tool_call = false;
        let mut is_in_thinking_block = false;
        let mut non_sse_json_lines_buffer = String::new();
        let mut emitted_any = false;

        while let Some(item) = bytes_stream.next().await {
            if self.is_cancelled() {
                break;
            }
            let bytes =
                item.map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
            pending_line.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(newline_index) = pending_line.find('\n') {
                let line = pending_line[..newline_index].trim().to_string();
                pending_line = pending_line[newline_index + 1..].to_string();
                self.process_streaming_line(
                    &line,
                    &mut chunks,
                    &mut token_counts,
                    &mut current_tool_parser,
                    &mut current_tool_tag_name,
                    &mut is_in_tool_call,
                    &mut is_in_thinking_block,
                    &mut non_sse_json_lines_buffer,
                    &mut emitted_any,
                )?;
            }
        }
        let pending = pending_line.trim().to_string();
        if !pending.is_empty() {
            self.process_streaming_line(
                &pending,
                &mut chunks,
                &mut token_counts,
                &mut current_tool_parser,
                &mut current_tool_tag_name,
                &mut is_in_tool_call,
                &mut is_in_thinking_block,
                &mut non_sse_json_lines_buffer,
                &mut emitted_any,
            )?;
        }
        if !emitted_any && !non_sse_json_lines_buffer.trim().is_empty() {
            if let Ok(json_response) =
                serde_json::from_str::<Value>(non_sse_json_lines_buffer.trim())
            {
                let text = self.parse_anthropic_non_streaming(&json_response);
                if !text.is_empty() {
                    chunks.push(text);
                }
                token_counts = self.apply_usage(json_response.get("usage"));
            }
        }
        if is_in_tool_call {
            if let Some(mut parser) = current_tool_parser {
                append_converter_events(&mut chunks, parser.flush());
            }
            if let Some(tag) = current_tool_tag_name {
                chunks.push(format!("\n</{tag}>\n"));
            }
        }
        if is_in_thinking_block {
            chunks.push("</think>\n".to_string());
        }
        self.set_token_counts(token_counts);
        Ok(response_stream_from_chunks(chunks))
    }

    #[allow(clippy::too_many_arguments)]
    fn process_streaming_line(
        &mut self,
        line: &str,
        chunks: &mut Vec<String>,
        token_counts: &mut TokenCounts,
        current_tool_parser: &mut Option<StreamingJsonXmlConverter>,
        current_tool_tag_name: &mut Option<String>,
        is_in_tool_call: &mut bool,
        is_in_thinking_block: &mut bool,
        non_sse_json_lines_buffer: &mut String,
        emitted_any: &mut bool,
    ) -> Result<(), AiServiceError> {
        if !line.starts_with("data:") {
            if line.starts_with('{') || line.starts_with('[') {
                non_sse_json_lines_buffer.push_str(line);
                non_sse_json_lines_buffer.push('\n');
            }
            return Ok(());
        }
        let data = line.trim_start_matches("data:").trim_start();
        if data == "[DONE]" || data.is_empty() {
            return Ok(());
        }
        let json_response: Value = serde_json::from_str(data)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let event_type = json_response
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("");
        if event_type.is_empty() {
            if let Some(content) = json_response
                .pointer("/choices/0/delta/content")
                .and_then(Value::as_str)
            {
                if !content.is_empty() {
                    chunks.push(content.to_string());
                    *emitted_any = true;
                }
            }
            return Ok(());
        }
        match event_type {
            "message_start" => {
                if let Some(usage) = json_response.pointer("/message/usage") {
                    *token_counts = self.apply_usage(Some(usage));
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
                            *emitted_any = true;
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
                        *emitted_any = true;
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
                            *emitted_any = true;
                        }
                    }
                } else if *is_in_thinking_block
                    && (delta_type == "thinking_delta" || delta.get("thinking").is_some())
                {
                    if let Some(thinking) = delta.get("thinking").and_then(Value::as_str) {
                        if !thinking.is_empty() {
                            chunks.push(thinking.to_string());
                            *emitted_any = true;
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
                    *token_counts = self.apply_usage(Some(usage));
                }
            }
            "message_stop" => {}
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClaudeThinkingFormat {
    Adaptive,
    Enabled,
}

fn normalize_claude_model_name(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_separator = false;
    let mut previous_kind = ModelNameCharKind::Separator;

    for ch in value.trim().chars().flat_map(char::to_lowercase) {
        let current_kind = ModelNameCharKind::from_char(ch);
        if current_kind == ModelNameCharKind::Separator {
            if !previous_was_separator && !normalized.is_empty() {
                normalized.push('-');
                previous_was_separator = true;
            }
            previous_kind = ModelNameCharKind::Separator;
            continue;
        }

        if should_split_model_name_part(previous_kind, current_kind, previous_was_separator) {
            normalized.push('-');
        }
        normalized.push(ch);
        previous_was_separator = false;
        previous_kind = current_kind;
    }

    normalized.trim_matches('-').to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelNameCharKind {
    Letter,
    Digit,
    Separator,
}

impl ModelNameCharKind {
    fn from_char(ch: char) -> Self {
        if ch.is_ascii_alphabetic() {
            Self::Letter
        } else if ch.is_ascii_digit() {
            Self::Digit
        } else {
            Self::Separator
        }
    }
}

fn should_split_model_name_part(
    previous_kind: ModelNameCharKind,
    current_kind: ModelNameCharKind,
    previous_was_separator: bool,
) -> bool {
    !previous_was_separator
        && matches!(
            (previous_kind, current_kind),
            (ModelNameCharKind::Letter, ModelNameCharKind::Digit)
                | (ModelNameCharKind::Digit, ModelNameCharKind::Letter)
        )
}

fn has_claude_family(normalized_model_name: &str, family: &str) -> bool {
    normalized_model_name.split('-').any(|part| part == family)
}

fn has_claude_family_at_least(
    normalized_model_name: &str,
    family: &str,
    min_major: i32,
    min_minor: i32,
) -> bool {
    match claude_family_version(normalized_model_name, family) {
        Some((major, minor)) => major > min_major || major == min_major && minor >= min_minor,
        None => false,
    }
}

fn claude_family_version(normalized_model_name: &str, family: &str) -> Option<(i32, i32)> {
    let parts = normalized_model_name
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let family_index = parts.iter().position(|part| *part == family)?;

    let before_family = take_last_version_parts(&parts[..family_index]);
    if let Some(version) = numeric_version(&before_family) {
        return Some(version);
    }

    let after_family = take_first_version_parts(&parts[family_index + 1..]);
    numeric_version(&after_family)
}

fn take_last_version_parts(parts: &[&str]) -> Vec<String> {
    let mut version_parts = parts
        .iter()
        .rev()
        .take_while(|part| is_version_part(part))
        .map(|part| (*part).to_string())
        .collect::<Vec<_>>();
    version_parts.reverse();
    let start = version_parts.len().saturating_sub(2);
    version_parts[start..].to_vec()
}

fn take_first_version_parts(parts: &[&str]) -> Vec<String> {
    parts
        .iter()
        .take_while(|part| is_version_part(part))
        .take(2)
        .map(|part| (*part).to_string())
        .collect()
}

fn is_version_part(value: &str) -> bool {
    value.len() < 8 && value.chars().all(|ch| ch.is_ascii_digit())
}

fn numeric_version(parts: &[String]) -> Option<(i32, i32)> {
    let major = parts.first()?.parse::<i32>().ok()?;
    let minor = match parts.get(1) {
        Some(value) => value.parse::<i32>().ok()?,
        None => 0,
    };
    Some((major, minor))
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

fn xml_unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}
