use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use serde_json::{json, Map, Value};

use super::AIService::{
    response_stream_from_chunks, AIService, AiServiceError, SendMessageRequest,
    TokenCounts,
};
use super::OpenAIProvider::{StreamingJsonXmlConverter, StreamingJsonXmlEvent};
use super::StructuredToolCallBridge::StructuredToolCallBridge;
use crate::core::chat::hooks::PromptTurn::{PromptTurn, PromptTurnKind};
use crate::data::model::ToolPrompt::ToolPrompt;
use crate::util::stream::RevisableTextStream::RevisableTextStreamLike;
use crate::util::ChatMarkupRegex::ChatMarkupRegex;

pub struct ClaudeProvider {
    pub api_endpoint: String,
    pub api_key: String,
    pub model_name: String,
    pub provider_type: String,
    pub enable_tool_call: bool,
    pub custom_headers: Vec<(String, String)>,
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
            inputTokenCount: 0,
            cachedInputTokenCount: 0,
            outputTokenCount: 0,
            cancelled: false,
        }
    }

    pub fn create_request_body(&self, request: &SendMessageRequest) -> Result<Value, AiServiceError> {
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
            object.insert("tools".to_string(), self.build_tool_definitions_for_claude(&request.available_tools)?);
        }
        self.add_parameters(&mut object, &request.model_parameters);
        self.apply_stable_cache_breakpoints(&mut object);
        Ok(Value::Object(object))
    }

    pub fn build_messages_and_count_tokens(&self, chat_history: &[PromptTurn]) -> Result<(Value, Vec<Value>), AiServiceError> {
        let mut system_parts = Vec::new();
        let mut messages = Vec::new();
        let provider_ready_history = StructuredToolCallBridge::compileHistoryForProvider(
            chat_history,
            self.enable_tool_call,
        );
        for turn in provider_ready_history {
            match turn.kind {
                PromptTurnKind::SYSTEM | PromptTurnKind::SUMMARY => system_parts.push(turn.content.clone()),
                PromptTurnKind::USER => messages.push(json!({"role": "user", "content": self.build_content_array(&turn.content)})),
                PromptTurnKind::ASSISTANT | PromptTurnKind::TOOL_CALL => {
                    let content = if self.enable_tool_call {
                        self.build_assistant_content_blocks(&turn.content)?
                    } else {
                        self.build_content_array(&turn.content)
                    };
                    messages.push(json!({"role": "assistant", "content": content}))
                }
                PromptTurnKind::TOOL_RESULT => {
                    let content = if self.enable_tool_call {
                        self.build_tool_result_blocks(&turn.content)
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
            Value::Array(system_parts.into_iter().map(|text| {
                json!({
                    "type": "text",
                    "text": text,
                    "cache_control": {"type": "ephemeral"}
                })
            }).collect())
        };
        Ok((system, messages))
    }

    pub fn apply_stable_cache_breakpoints(&self, _request_object: &mut Map<String, Value>) {}

    fn build_tool_definitions_for_claude(&self, tool_prompts: &[ToolPrompt]) -> Result<Value, AiServiceError> {
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

    fn build_assistant_content_blocks(&self, content: &str) -> Result<Value, AiServiceError> {
        let matches = ChatMarkupRegex::tool_call_matches(content);
        if matches.is_empty() {
            return Ok(self.build_content_array(content));
        }
        let mut text_content = content.to_string();
        let mut blocks = Vec::new();
        for (call_index, tool_match) in matches.iter().enumerate() {
            text_content = text_content.replace(&format!(
                "<{} name=\"{}\">{}</{}>",
                tool_match.tag_name, tool_match.name, tool_match.body, tool_match.tag_name
            ), "");
            let mut input = Map::new();
            for (start, end) in crate::util::ChatMarkupRegex::tag_ranges(&tool_match.body, "param") {
                let raw = &tool_match.body[start..end];
                let name = crate::util::ChatMarkupRegex::attr_value(raw, "name").unwrap_or_default();
                let value = raw
                    .split_once('>')
                    .and_then(|(_, tail)| tail.rsplit_once("</").map(|(body, _)| xml_unescape(body)))
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                input.insert(name, json!(value));
            }
            let tool_name_part = sanitize_tool_call_id(&tool_match.name);
            let hash_part = stable_id_hash_part(&format!("{}:{}", tool_match.name, Value::Object(input.clone())));
            let call_id = sanitize_tool_call_id(&format!("toolu_{}_{}_{}", tool_name_part, hash_part, call_index));
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

    fn build_tool_result_blocks(&self, content: &str) -> Value {
        let blocks = ChatMarkupRegex::tool_result_blocks(content);
        if blocks.is_empty() {
            return self.build_content_array(content);
        }
        let mut result_blocks = Vec::new();
        for (index, block) in blocks.iter().enumerate() {
            let result_content = crate::util::ChatMarkupRegex::tag_ranges(&block.body, "content")
                .into_iter()
                .next()
                .and_then(|(start, end)| {
                    let raw = &block.body[start..end];
                    raw.split_once('>')
                        .and_then(|(_, tail)| tail.rsplit_once("</").map(|(body, _)| body.trim().to_string()))
                })
                .unwrap_or_else(|| block.body.trim().to_string());
            result_blocks.push(json!({
                "type": "tool_result",
                "tool_use_id": format!("toolu_result_{}", index),
                "content": result_content,
            }));
        }
        Value::Array(result_blocks)
    }

    fn add_parameters(&self, json_object: &mut Map<String, Value>, parameters: &[crate::data::model::ModelParameter::ModelParameter<Value>]) {
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

    fn apply_usage(&mut self, usage: Option<&Value>) -> TokenCounts {
        let cached_input = usage
            .and_then(|value| {
                value.get("cache_read_input_tokens")
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
        let input_base = usage.and_then(|value| value.get("input_tokens")).and_then(Value::as_i64).unwrap_or(0).max(0) as i32;
        let input = input_base + cache_creation;
        let output = usage.and_then(|value| value.get("output_tokens")).and_then(Value::as_i64).unwrap_or(0) as i32;
        self.inputTokenCount = input;
        self.cachedInputTokenCount = cached_input;
        self.outputTokenCount = output;
        TokenCounts { input, cached_input, output }
    }
}

#[async_trait]
impl AIService for ClaudeProvider {
    fn input_token_count(&self) -> i32 { self.inputTokenCount }
    fn cached_input_token_count(&self) -> i32 { self.cachedInputTokenCount }
    fn output_token_count(&self) -> i32 { self.outputTokenCount }
    fn provider_model(&self) -> String { format!("{}:{}", self.provider_type, self.model_name) }
    fn reset_token_counts(&mut self) {
        self.inputTokenCount = 0;
        self.cachedInputTokenCount = 0;
        self.outputTokenCount = 0;
    }
    fn cancel_streaming(&mut self) { self.cancelled = true; }

    async fn send_message(
        &mut self,
        request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        self.cancelled = false;
        self.reset_token_counts();
        let stream = request.stream;
        let request_body = self.create_request_body(&request)?;
        let response = reqwest::Client::new()
            .post(&self.api_endpoint)
            .headers(self.headers()?)
            .json(&request_body)
            .send()
            .await
            .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
            return Err(AiServiceError::RequestFailed(format!("{status}: {text}")));
        }
        if stream {
            return self.process_streaming_response(response).await;
        }
        let json_response: Value = response.json().await.map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let token_counts = self.apply_usage(json_response.get("usage"));
        let mut chunks = Vec::new();
        if let Some(content) = json_response.get("content").and_then(Value::as_array) {
            for part in content {
                match part.get("type").and_then(Value::as_str).unwrap_or_default() {
                    "text" => {
                        if let Some(text) = part.get("text").and_then(Value::as_str) {
                            chunks.push(StructuredToolCallBridge::convertToolCallPayloadToXml(text));
                        }
                    }
                    "tool_use" => chunks.push(StructuredToolCallBridge::convertToolCallPayloadToXml(&part.to_string())),
                    _ => {}
                }
            }
        }
        let _ = token_counts;
        Ok(response_stream_from_chunks(chunks))
    }

    async fn calculate_input_tokens(&self, chat_history: &[PromptTurn], available_tools: &[ToolPrompt]) -> Result<i32, AiServiceError> {
        let history_chars: usize = chat_history.iter().map(|turn| turn.content.len()).sum();
        let tool_chars: usize = available_tools.iter().map(|tool| tool.name.len() + tool.description.len()).sum();
        Ok(((history_chars + tool_chars + 3) / 4) as i32)
    }
}

impl ClaudeProvider {
    async fn process_streaming_response(
        &mut self,
        response: reqwest::Response,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        let mut chunks = Vec::new();
        let mut token_counts = TokenCounts { input: 0, cached_input: 0, output: 0 };
        let mut pending_line = String::new();
        let mut bytes_stream = response.bytes_stream();
        let mut current_tool_parser: Option<StreamingJsonXmlConverter> = None;
        let mut current_tool_tag_name: Option<String> = None;
        let mut is_in_tool_call = false;
        let mut is_in_thinking_block = false;
        let mut non_sse_json_lines_buffer = String::new();
        let mut emitted_any = false;

        while let Some(item) = bytes_stream.next().await {
            if self.cancelled {
                break;
            }
            let bytes = item.map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
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
            if let Ok(json_response) = serde_json::from_str::<Value>(non_sse_json_lines_buffer.trim()) {
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
        self.inputTokenCount = token_counts.input;
        self.cachedInputTokenCount = token_counts.cached_input;
        self.outputTokenCount = token_counts.output;
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
        let event_type = json_response.get("type").and_then(Value::as_str).unwrap_or("");
        if event_type.is_empty() {
            if let Some(content) = json_response.pointer("/choices/0/delta/content").and_then(Value::as_str) {
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
                match content_block.get("type").and_then(Value::as_str).unwrap_or("") {
                    "tool_use" if self.enable_tool_call => {
                        let tool_name = content_block.get("name").and_then(Value::as_str).unwrap_or("");
                        if !tool_name.is_empty() {
                            let tag = ChatMarkupRegex::generate_random_tool_tag_name();
                            *current_tool_tag_name = Some(tag.clone());
                            chunks.push(format!("\n<{tag} name=\"{tool_name}\">"));
                            *current_tool_parser = Some(StreamingJsonXmlConverter::new());
                            *is_in_tool_call = true;
                            *emitted_any = true;
                            if let Some(input) = content_block.get("input") {
                                if let Some(parser) = current_tool_parser.as_mut() {
                                    append_converter_events(chunks, parser.feed(&input.to_string()));
                                }
                            }
                        }
                    }
                    "thinking" => {
                        chunks.push("\n<think>".to_string());
                        *is_in_thinking_block = true;
                        *emitted_any = true;
                        if let Some(thinking) = content_block.get("thinking").and_then(Value::as_str) {
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
                } else if *is_in_thinking_block && (delta_type == "thinking_delta" || delta.get("thinking").is_some()) {
                    if let Some(thinking) = delta.get("thinking").and_then(Value::as_str) {
                        if !thinking.is_empty() {
                            chunks.push(thinking.to_string());
                            *emitted_any = true;
                        }
                    }
                } else if self.enable_tool_call && *is_in_tool_call && delta_type == "input_json_delta" {
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
                            for event in parser.feed(&input.to_string()).into_iter().chain(parser.flush()) {
                                match event {
                                    StreamingJsonXmlEvent::Tag(text) | StreamingJsonXmlEvent::Content(text) => full_text.push_str(&text),
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

fn append_converter_events(chunks: &mut Vec<String>, events: Vec<StreamingJsonXmlEvent>) {
    for event in events {
        match event {
            StreamingJsonXmlEvent::Tag(text) | StreamingJsonXmlEvent::Content(text) => chunks.push(text),
        }
    }
}

fn sanitize_tool_call_id(raw: &str) -> String {
    let mut output = String::new();
    let mut previous_underscore = false;
    for ch in raw.chars() {
        let next = if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' { ch } else { '_' };
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
    if output.is_empty() { "toolu".to_string() } else { output }
}

fn stable_id_hash_part(raw: &str) -> String {
    let mut hash: i32 = 0;
    for unit in raw.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(unit as i32);
    }
    let positive = if hash == i32::MIN { 0 } else { hash.abs() as u32 };
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
