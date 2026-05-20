use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::AIService::{
    response_stream_from_chunks, AIService, AiServiceError, SendMessageRequest,
    TokenCounts,
};
use super::StructuredToolCallBridge::StructuredToolCallBridge;
use crate::core::chat::hooks::PromptTurn::{PromptTurn, PromptTurnKind};
use crate::data::model::ModelParameter::ModelParameter;
use crate::data::model::ToolPrompt::ToolPrompt;
use crate::util::stream::RevisableTextStream::{
    empty_revisable_event_channel, with_event_channel, RevisableTextStreamLike, TextStreamEvent,
    TextStreamEventType,
};
use crate::util::ChatMarkupRegex::ChatMarkupRegex;
use crate::util::stream::Stream::FnStream;

#[derive(Clone)]
pub struct OpenAIProvider {
    pub api_endpoint: String,
    pub api_key: String,
    pub model_name: String,
    pub provider_type: String,
    pub supports_vision: bool,
    pub supports_audio: bool,
    pub supports_video: bool,
    pub enable_tool_call: bool,
    pub custom_headers: Vec<(String, String)>,
    state: Arc<Mutex<OpenAIProviderState>>,
}

#[derive(Debug, Default)]
struct OpenAIProviderState {
    inputTokenCount: i32,
    cachedInputTokenCount: i32,
    outputTokenCount: i32,
    cancelled: bool,
}

pub struct StreamingState {
    pub chunks: Vec<String>,
    pub pending_line: String,
    pub usage: TokenCounts,
    pub chunkCount: i32,
    pub isInReasoningMode: bool,
    pub hasEmittedThinkStart: bool,
    pub hasEmittedRegularContent: bool,
    pub isFirstResponse: bool,
    pub accumulatedToolCalls: HashMap<i32, Value>,
    pub toolCallState: ToolCallState,
    pub lastProcessedToolIndex: Option<i32>,
}

pub struct StreamEmitter {
    pub received_content: String,
    pub event_channel: crate::util::stream::HotStream::MutableSharedStreamImpl<TextStreamEvent>,
    pub savepoints: HashMap<String, usize>,
}

impl StreamEmitter {
    pub fn new(
        event_channel: crate::util::stream::HotStream::MutableSharedStreamImpl<TextStreamEvent>,
    ) -> Self {
        Self {
            received_content: String::new(),
            event_channel,
            savepoints: HashMap::new(),
        }
    }

    pub fn emit_chunk(&mut self, chunk: &str) {
        if chunk.is_empty() {
            return;
        }
        self.received_content.push_str(chunk);
    }

    pub fn emit_savepoint(&mut self, id: &str) {
        self.savepoints
            .insert(id.to_string(), self.received_content.len());
        self.event_channel.emit(TextStreamEvent {
            event_type: TextStreamEventType::Savepoint,
            id: id.to_string(),
        });
    }

    pub fn emit_rollback(&mut self, id: &str) -> bool {
        let Some(savepoint_length) = self.savepoints.get(id).copied() else {
            return false;
        };
        if self.received_content.len() > savepoint_length {
            self.received_content.truncate(savepoint_length);
        }
        self.event_channel.emit(TextStreamEvent {
            event_type: TextStreamEventType::Rollback,
            id: id.to_string(),
        });
        true
    }
}

#[derive(Default)]
pub struct ToolCallState {
    pub emitted: HashMap<i32, bool>,
    pub nameEmitted: HashMap<i32, bool>,
    pub parser: HashMap<i32, StreamingJsonXmlConverter>,
    pub closed: HashMap<i32, bool>,
    pub fedLength: HashMap<i32, usize>,
    pub tagNames: HashMap<i32, String>,
}

impl ToolCallState {
    pub fn getParser(&mut self, index: i32) -> &mut StreamingJsonXmlConverter {
        self.parser.entry(index).or_insert_with(StreamingJsonXmlConverter::new)
    }

    pub fn getTagName(&mut self, index: i32) -> String {
        self.tagNames
            .entry(index)
            .or_insert_with(ChatMarkupRegex::generate_random_tool_tag_name)
            .clone()
    }

    pub fn clear(&mut self) {
        self.emitted.clear();
        self.nameEmitted.clear();
        self.parser.clear();
        self.closed.clear();
        self.fedLength.clear();
        self.tagNames.clear();
    }
}

#[derive(Clone)]
pub enum StreamingJsonXmlEvent {
    Tag(String),
    Content(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StreamingJsonXmlState {
    WAIT_BRACE,
    WAIT_KEY_QUOTE,
    READ_KEY,
    WAIT_COLON,
    WAIT_VALUE,
    READ_STRING,
    READ_PRIMITIVE,
    ESCAPE,
    UNICODE_ESCAPE,
    WAIT_COMMA,
}

pub struct StreamingJsonXmlConverter {
    state: StreamingJsonXmlState,
    buffer: String,
    unicodeCount: i32,
    primitiveNestingDepth: i32,
    primitiveInString: bool,
    primitiveEscape: bool,
    keyEscape: bool,
    readingComplexValue: bool,
    hasOpenParam: bool,
}

impl Default for StreamingJsonXmlConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingJsonXmlConverter {
    pub fn new() -> Self {
        Self {
            state: StreamingJsonXmlState::WAIT_BRACE,
            buffer: String::new(),
            unicodeCount: 0,
            primitiveNestingDepth: 0,
            primitiveInString: false,
            primitiveEscape: false,
            keyEscape: false,
            readingComplexValue: false,
            hasOpenParam: false,
        }
    }

    fn resetPrimitiveTracking(&mut self) {
        self.primitiveNestingDepth = 0;
        self.primitiveInString = false;
        self.primitiveEscape = false;
        self.readingComplexValue = false;
    }

    fn emitPrimitiveParam(&mut self, events: &mut Vec<StreamingJsonXmlEvent>) {
        events.push(StreamingJsonXmlEvent::Content(escapeXml(&self.buffer)));
        events.push(StreamingJsonXmlEvent::Tag("</param>".to_string()));
        self.hasOpenParam = false;
        self.buffer.clear();
        self.resetPrimitiveTracking();
    }

    fn canFinalizePrimitiveOnFlush(&self) -> bool {
        if self.state != StreamingJsonXmlState::READ_PRIMITIVE || self.buffer.is_empty() {
            return false;
        }
        if !self.readingComplexValue {
            return true;
        }
        self.primitiveNestingDepth == 0 && !self.primitiveInString && !self.primitiveEscape
    }

    pub fn hasUnfinishedParam(&self) -> bool {
        self.hasOpenParam
    }

    pub fn feed(&mut self, chunk: &str) -> Vec<StreamingJsonXmlEvent> {
        let mut events = Vec::new();

        for c in chunk.chars() {
            match self.state {
                StreamingJsonXmlState::WAIT_BRACE => {
                    if c == '{' {
                        self.state = StreamingJsonXmlState::WAIT_KEY_QUOTE;
                    }
                }
                StreamingJsonXmlState::WAIT_KEY_QUOTE => {
                    if c == '"' {
                        self.state = StreamingJsonXmlState::READ_KEY;
                        self.keyEscape = false;
                        self.buffer.clear();
                    } else if c == '}' {
                        self.state = StreamingJsonXmlState::WAIT_BRACE;
                    }
                }
                StreamingJsonXmlState::READ_KEY => {
                    if self.keyEscape {
                        self.buffer.push(c);
                        self.keyEscape = false;
                    } else {
                        match c {
                            '\\' => self.keyEscape = true,
                            '"' => {
                                events.push(StreamingJsonXmlEvent::Tag(format!(
                                    "\n  <param name=\"{}\">",
                                    self.buffer
                                )));
                                self.hasOpenParam = true;
                                self.state = StreamingJsonXmlState::WAIT_COLON;
                            }
                            _ => self.buffer.push(c),
                        }
                    }
                }
                StreamingJsonXmlState::WAIT_COLON => {
                    if c == ':' {
                        self.state = StreamingJsonXmlState::WAIT_VALUE;
                    }
                }
                StreamingJsonXmlState::WAIT_VALUE => {
                    if !c.is_whitespace() {
                        if c == '"' {
                            self.state = StreamingJsonXmlState::READ_STRING;
                        } else {
                            self.state = StreamingJsonXmlState::READ_PRIMITIVE;
                            self.buffer.clear();
                            self.buffer.push(c);
                            self.readingComplexValue = c == '[' || c == '{';
                            self.primitiveNestingDepth = if self.readingComplexValue { 1 } else { 0 };
                            self.primitiveInString = false;
                            self.primitiveEscape = false;
                        }
                    }
                }
                StreamingJsonXmlState::READ_STRING => {
                    if c == '"' {
                        self.state = StreamingJsonXmlState::WAIT_COMMA;
                        events.push(StreamingJsonXmlEvent::Tag("</param>".to_string()));
                        self.hasOpenParam = false;
                    } else if c == '\\' {
                        self.state = StreamingJsonXmlState::ESCAPE;
                    } else {
                        events.push(StreamingJsonXmlEvent::Content(escapeXml(&c.to_string())));
                    }
                }
                StreamingJsonXmlState::ESCAPE => {
                    if c == 'u' {
                        self.state = StreamingJsonXmlState::UNICODE_ESCAPE;
                        self.unicodeCount = 0;
                        self.buffer.clear();
                    } else {
                        let unescaped = match c {
                            'n' => "\n".to_string(),
                            'r' => "\r".to_string(),
                            't' => "\t".to_string(),
                            'b' => "\u{0008}".to_string(),
                            'f' => "\u{000c}".to_string(),
                            '"' => "\"".to_string(),
                            '\\' => "\\".to_string(),
                            '/' => "/".to_string(),
                            _ => c.to_string(),
                        };
                        events.push(StreamingJsonXmlEvent::Content(escapeXml(&unescaped)));
                        self.state = StreamingJsonXmlState::READ_STRING;
                    }
                }
                StreamingJsonXmlState::UNICODE_ESCAPE => {
                    self.buffer.push(c);
                    self.unicodeCount += 1;
                    if self.unicodeCount == 4 {
                        if let Ok(code) = u32::from_str_radix(&self.buffer, 16) {
                            if let Some(ch) = char::from_u32(code) {
                                events.push(StreamingJsonXmlEvent::Content(escapeXml(&ch.to_string())));
                            }
                        }
                        self.state = StreamingJsonXmlState::READ_STRING;
                    }
                }
                StreamingJsonXmlState::READ_PRIMITIVE => {
                    if self.readingComplexValue {
                        if self.primitiveInString {
                            self.buffer.push(c);
                            if self.primitiveEscape {
                                self.primitiveEscape = false;
                            } else if c == '\\' {
                                self.primitiveEscape = true;
                            } else if c == '"' {
                                self.primitiveInString = false;
                            }
                        } else {
                            match c {
                                '"' => {
                                    self.primitiveInString = true;
                                    self.buffer.push(c);
                                }
                                '[' | '{' => {
                                    self.primitiveNestingDepth += 1;
                                    self.buffer.push(c);
                                }
                                ']' | '}' => {
                                    self.primitiveNestingDepth -= 1;
                                    self.buffer.push(c);
                                    if self.primitiveNestingDepth == 0 {
                                        self.emitPrimitiveParam(&mut events);
                                        self.state = StreamingJsonXmlState::WAIT_COMMA;
                                    }
                                }
                                _ => self.buffer.push(c),
                            }
                        }
                    } else if c == ',' || c == '}' || c.is_whitespace() {
                        self.emitPrimitiveParam(&mut events);
                        if c == ',' {
                            self.state = StreamingJsonXmlState::WAIT_KEY_QUOTE;
                        } else if c == '}' {
                            self.state = StreamingJsonXmlState::WAIT_BRACE;
                        } else {
                            self.state = StreamingJsonXmlState::WAIT_COMMA;
                        }
                    } else {
                        self.buffer.push(c);
                    }
                }
                StreamingJsonXmlState::WAIT_COMMA => {
                    if c == ',' {
                        self.state = StreamingJsonXmlState::WAIT_KEY_QUOTE;
                    } else if c == '}' {
                        self.state = StreamingJsonXmlState::WAIT_BRACE;
                    }
                }
            }
        }
        events
    }

    pub fn flush(&mut self) -> Vec<StreamingJsonXmlEvent> {
        let mut events = Vec::new();
        if self.canFinalizePrimitiveOnFlush() {
            self.emitPrimitiveParam(&mut events);
        }
        events
    }
}

impl OpenAIProvider {
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
            state: Arc::new(Mutex::new(OpenAIProviderState::default())),
        }
    }

    fn apply_token_counts(&self, token_counts: TokenCounts) {
        if let Ok(mut state) = self.state.lock() {
            state.inputTokenCount = token_counts.input;
            state.cachedInputTokenCount = token_counts.cached_input;
            state.outputTokenCount = token_counts.output;
        }
    }

    fn is_cancelled(&self) -> bool {
        self.state.lock().map(|state| state.cancelled).unwrap_or(false)
    }

    pub fn create_request_body(&self, request: &SendMessageRequest) -> Result<Value, AiServiceError> {
        let mut json_object = Map::new();
        json_object.insert("model".to_string(), json!(self.model_name));
        json_object.insert(
            "messages".to_string(),
            serde_json::from_str(&StructuredToolCallBridge::buildMessagesJson(
                &request.chat_history,
                request.preserve_think_in_history,
            ))
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?,
        );
        json_object.insert("stream".to_string(), json!(request.stream));

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

    pub fn build_messages_json(&self, chat_history: &[PromptTurn]) -> Value {
        Value::Array(
            chat_history
                .iter()
                .map(|turn| {
                    let role = match turn.kind {
                        PromptTurnKind::SYSTEM => "system",
                        PromptTurnKind::USER => "user",
                        PromptTurnKind::ASSISTANT | PromptTurnKind::TOOL_CALL => "assistant",
                        PromptTurnKind::TOOL_RESULT => "tool",
                        PromptTurnKind::SUMMARY => "system",
                    };

                    let mut message = Map::new();
                    message.insert("role".to_string(), json!(role));
                    message.insert("content".to_string(), json!(turn.content));
                    if let Some(tool_name) = &turn.tool_name {
                        message.insert("name".to_string(), json!(tool_name));
                    }
                    Value::Object(message)
                })
                .collect(),
        )
    }

    async fn read_streaming_response(
        &self,
        response: reqwest::Response,
        on_tool_invocation: Option<&Arc<dyn Fn(String) + Send + Sync>>,
        tx: &std::sync::mpsc::Sender<String>,
        emitter: &mut StreamEmitter,
    ) -> Result<(), AiServiceError> {
        let mut state = StreamingState {
            chunks: Vec::new(),
            pending_line: String::new(),
            usage: TokenCounts {
                input: 0,
                cached_input: 0,
                output: 0,
            },
            chunkCount: 0,
            isInReasoningMode: false,
            hasEmittedThinkStart: false,
            hasEmittedRegularContent: false,
            isFirstResponse: true,
            accumulatedToolCalls: HashMap::new(),
            toolCallState: ToolCallState::default(),
            lastProcessedToolIndex: None,
        };
        let mut response_stream = response.bytes_stream();

        let result = async {
            while let Some(item) = response_stream.next().await {
                if self.is_cancelled() {
                    break;
                }
                let bytes =
                    item.map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
                state.pending_line.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(newline_index) = state.pending_line.find('\n') {
                    let line = state.pending_line[..newline_index].trim().to_string();
                    state.pending_line = state.pending_line[newline_index + 1..].to_string();
                    let emitted_before = state.chunks.len();
                    self.process_streaming_line(&line, &mut state, on_tool_invocation)?;
                    for chunk in state.chunks[emitted_before..].iter().cloned() {
                        let _ = tx.send(chunk.clone());
                        emitter.emit_chunk(&chunk);
                    }
                }
            }

            let pending = state.pending_line.trim().to_string();
            if !pending.is_empty() {
                let emitted_before = state.chunks.len();
                self.process_streaming_line(&pending, &mut state, on_tool_invocation)?;
                for chunk in state.chunks[emitted_before..].iter().cloned() {
                    let _ = tx.send(chunk.clone());
                    emitter.emit_chunk(&chunk);
                }
            }

            self.apply_token_counts(state.usage.clone());
            Ok(())
        }
        .await;

        result
    }

    fn process_streaming_line(
        &self,
        line: &str,
        state: &mut StreamingState,
        on_tool_invocation: Option<&Arc<dyn Fn(String) + Send + Sync>>,
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

        if json_response.get("type").and_then(Value::as_str).is_some() {
            return self.processResponsesStreamingEvent(&json_response, state, on_tool_invocation);
        }

        self.processResponseChunk(&json_response, state, on_tool_invocation)
    }

    fn createToolCallAccumulator(index: i32) -> Value {
        json!({
            "index": index,
            "id": "",
            "type": "function",
            "function": {
                "name": "",
                "arguments": ""
            }
        })
    }

    fn handleJsonEvents(&self, events: Vec<StreamingJsonXmlEvent>, state: &mut StreamingState) {
        for event in events {
            match event {
                StreamingJsonXmlEvent::Tag(text) | StreamingJsonXmlEvent::Content(text) => state.chunks.push(text),
            }
        }
    }

    fn handleToolSwitch(
        &self,
        prevIndex: i32,
        state: &mut StreamingState,
    ) {
        if state.toolCallState.closed.get(&prevIndex).copied() != Some(true)
            && state.toolCallState.nameEmitted.get(&prevIndex).copied() == Some(true)
        {
            self.closeToolCallIfOpen(prevIndex, state);
        }
    }

    fn processToolCallChunk(
        &self,
        index: i32,
        deltaCall: &Value,
        state: &mut StreamingState,
        on_tool_invocation: Option<&Arc<dyn Fn(String) + Send + Sync>>,
    ) {
        state
            .accumulatedToolCalls
            .entry(index)
            .or_insert_with(|| Self::createToolCallAccumulator(index));

        if let Some(id) = deltaCall.get("id").and_then(Value::as_str).filter(|value| !value.is_empty()) {
            if let Some(accumulated) = state.accumulatedToolCalls.get_mut(&index) {
                accumulated["id"] = json!(id);
            }
        }
        if let Some(call_type) = deltaCall.get("type").and_then(Value::as_str).filter(|value| !value.is_empty()) {
            if let Some(accumulated) = state.accumulatedToolCalls.get_mut(&index) {
                accumulated["type"] = json!(call_type);
            }
        }

        let Some(deltaFunction) = deltaCall.get("function").and_then(Value::as_object) else {
            return;
        };
        let name = deltaFunction.get("name").and_then(Value::as_str).unwrap_or("");
        if !name.is_empty() {
            if let Some(accumulated) = state.accumulatedToolCalls.get_mut(&index) {
                accumulated["function"]["name"] = json!(name);
            }
            if state.toolCallState.nameEmitted.get(&index).copied() != Some(true) {
                if let Some(callback) = on_tool_invocation {
                    callback(name.to_string());
                }
                let toolTagName = state.toolCallState.getTagName(index);
                let toolStartTag = if state.toolCallState.emitted.get(&index).copied() != Some(true) {
                    state.toolCallState.emitted.insert(index, true);
                    format!("\n<{toolTagName} name=\"{name}\">")
                } else {
                    String::new()
                };
                if !toolStartTag.is_empty() {
                    state.chunks.push(toolStartTag);
                }
                state.toolCallState.nameEmitted.insert(index, true);

                let canonicalArgs = state
                    .accumulatedToolCalls
                    .get(&index)
                    .and_then(|accumulated| accumulated.get("function"))
                    .and_then(|function| function.get("arguments"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if !canonicalArgs.is_empty() {
                    self.feedParserFromCanonical(index, &canonicalArgs, state);
                }
            }
        }

        let args = deltaFunction.get("arguments").and_then(Value::as_str).unwrap_or("");
        if !args.is_empty() {
            let currentArgs = state
                .accumulatedToolCalls
                .get(&index)
                .and_then(|accumulated| accumulated.get("function"))
                .and_then(|function| function.get("arguments"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let mergedArgs = self.mergeCanonicalArgs(&currentArgs, args);
            let changed = mergedArgs != currentArgs;
            if changed {
                if let Some(accumulated) = state.accumulatedToolCalls.get_mut(&index) {
                    accumulated["function"]["arguments"] = json!(mergedArgs.clone());
                }
                if state.toolCallState.nameEmitted.get(&index).copied() == Some(true) {
                    self.feedParserFromCanonical(index, &mergedArgs, state);
                }
            }
        }
    }

    fn mergeCanonicalArgs(&self, existing: &str, incoming: &str) -> String {
        if incoming.is_empty() {
            return existing.to_string();
        }
        if existing.is_empty() {
            return incoming.to_string();
        }
        if incoming.starts_with(existing) {
            incoming.to_string()
        } else {
            format!("{existing}{incoming}")
        }
    }

    fn feedParserFromCanonical(
        &self,
        index: i32,
        canonicalArgs: &str,
        state: &mut StreamingState,
    ) -> usize {
        let previousFedLength = state.toolCallState.fedLength.get(&index).copied().unwrap_or(0);
        let safeFedLength = previousFedLength.min(canonicalArgs.len());
        if safeFedLength == canonicalArgs.len() {
            state.toolCallState.fedLength.insert(index, safeFedLength);
            return 0;
        }
        let deltaToFeed = &canonicalArgs[safeFedLength..];
        let events = state.toolCallState.getParser(index).feed(deltaToFeed);
        self.handleJsonEvents(events, state);
        state.toolCallState.fedLength.insert(index, canonicalArgs.len());
        deltaToFeed.len()
    }

    fn getAccumulatedToolArguments(&self, state: &StreamingState, index: i32) -> String {
        state
            .accumulatedToolCalls
            .get(&index)
            .and_then(|call| call.get("function"))
            .and_then(|function| function.get("arguments"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    }

    fn processToolCallsDelta(
        &self,
        toolCallsDeltas: &[Value],
        state: &mut StreamingState,
        on_tool_invocation: Option<&Arc<dyn Fn(String) + Send + Sync>>,
    ) {
        if state.isInReasoningMode {
            state.isInReasoningMode = false;
            state.chunks.push("</think>".to_string());
            state.hasEmittedThinkStart = false;
        }

        for deltaCall in toolCallsDeltas {
            let index = deltaCall.get("index").and_then(Value::as_i64).unwrap_or(-1) as i32;
            if index < 0 {
                continue;
            }
            if let Some(prevIndex) = state.lastProcessedToolIndex {
                if prevIndex != index {
                    self.handleToolSwitch(prevIndex, state);
                }
            }
            state.lastProcessedToolIndex = Some(index);
            self.processToolCallChunk(index, deltaCall, state, on_tool_invocation);
        }
    }

    fn closeToolCallIfOpen(
        &self,
        index: i32,
        state: &mut StreamingState,
    ) {
        if state.toolCallState.closed.get(&index).copied() == Some(true)
            || state.toolCallState.nameEmitted.get(&index).copied() != Some(true)
        {
            return;
        }

        let accumulatedArgsBeforeFlush = self.getAccumulatedToolArguments(state, index);
        let Some(toolTagName) = state.toolCallState.tagNames.get(&index).cloned() else {
            return;
        };
        let parser = state.toolCallState.getParser(index);
        let events = parser.flush();
        let hasUnfinishedParam = parser.hasUnfinishedParam();
        self.handleJsonEvents(events, state);

        if hasUnfinishedParam {
            let parsedAsJson = serde_json::from_str::<Value>(&accumulatedArgsBeforeFlush).is_ok();
            if parsedAsJson {
                state.chunks.push("</param>".to_string());
                state.chunks.push(format!("\n</{toolTagName}>"));
                state.toolCallState.closed.insert(index, true);
            }
            return;
        }

        state.chunks.push(format!("\n</{toolTagName}>"));
        state.toolCallState.closed.insert(index, true);
    }

    fn hasOpenToolCalls(&self, state: &StreamingState) -> bool {
        state.toolCallState.nameEmitted.iter().any(|(index, emitted)| {
            *emitted && state.toolCallState.closed.get(index).copied() != Some(true)
        })
    }

    fn closeAllOpenToolCalls(&self, state: &mut StreamingState) {
        if !self.hasOpenToolCalls(state) {
            return;
        }
        let mut sortedIndices: Vec<i32> = state.accumulatedToolCalls.keys().copied().collect();
        sortedIndices.sort_unstable();
        for index in sortedIndices {
            self.closeToolCallIfOpen(index, state);
        }
    }

    fn handleFinishReason(
        &self,
        finishReason: &str,
        state: &mut StreamingState,
    ) {
        let normalizedFinishReason = finishReason.trim();
        if normalizedFinishReason.is_empty()
            || normalizedFinishReason.eq_ignore_ascii_case("null")
            || normalizedFinishReason.eq_ignore_ascii_case("none")
        {
            return;
        }

        if self.hasOpenToolCalls(state) {
            self.closeAllOpenToolCalls(state);
            state.accumulatedToolCalls.clear();
            state.lastProcessedToolIndex = None;
        }
    }

    fn processContentDelta(
        &self,
        reasoningContent: &str,
        regularContent: &str,
        state: &mut StreamingState,
    ) {
        let hasReasoning = !reasoningContent.is_empty() && reasoningContent != "null";
        let hasRegular = !regularContent.is_empty() && regularContent != "null";

        if hasReasoning && !state.hasEmittedRegularContent {
            if !state.isInReasoningMode {
                state.isInReasoningMode = true;
                if !state.hasEmittedThinkStart {
                    state.chunks.push("<think>".to_string());
                    state.hasEmittedThinkStart = true;
                }
            }
            state.chunks.push(reasoningContent.to_string());
        }

        if hasRegular {
            if state.isInReasoningMode {
                state.isInReasoningMode = false;
                state.chunks.push("</think>".to_string());
                state.hasEmittedThinkStart = false;
            }
            state.hasEmittedRegularContent = true;
            if state.isFirstResponse {
                state.isFirstResponse = false;
            }
            state.chunks.push(regularContent.to_string());
        }
    }

    fn processResponseChunk(
        &self,
        jsonResponse: &Value,
        state: &mut StreamingState,
        on_tool_invocation: Option<&Arc<dyn Fn(String) + Send + Sync>>,
    ) -> Result<(), AiServiceError> {
        let usage = jsonResponse.get("usage");
        let choices = jsonResponse.get("choices").and_then(Value::as_array);
        if choices.map(|items| items.is_empty()).unwrap_or(true) {
            if let Some(usage) = usage {
                state.usage = parse_usage_counts(usage);
            }
            return Ok(());
        }

        let choice = &choices.unwrap()[0];
        if let Some(delta) = choice.get("delta").and_then(Value::as_object) {
            let finishReason = if choice.get("finish_reason").map(|value| !value.is_null()).unwrap_or(false) {
                choice.get("finish_reason").and_then(Value::as_str).unwrap_or("").trim().to_string()
            } else {
                String::new()
            };
            if let Some(toolCallsDeltas) = delta.get("tool_calls").and_then(Value::as_array) {
                if !toolCallsDeltas.is_empty() && self.enable_tool_call {
                    self.processToolCallsDelta(toolCallsDeltas, state, on_tool_invocation);
                }
            }
            if !finishReason.is_empty() {
                self.handleFinishReason(&finishReason, state);
            }
            let reasoningContent = delta
                .get("reasoning_content")
                .or_else(|| delta.get("reasoning"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let regularContent = delta.get("content").and_then(Value::as_str).unwrap_or("");
            self.processContentDelta(reasoningContent, regularContent, state);
        } else if let Some(message) = choice.get("message").and_then(Value::as_object) {
            let reasoningContent = message
                .get("reasoning_content")
                .or_else(|| message.get("reasoning"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let regularContent = message.get("content").and_then(Value::as_str).unwrap_or("");
            if !reasoningContent.is_empty() && !state.hasEmittedRegularContent {
                state.chunks.push(format!("<think>{reasoningContent}</think>"));
            }
            if !regularContent.is_empty() {
                state.hasEmittedRegularContent = true;
                state.chunks.push(StructuredToolCallBridge::convertToolCallPayloadToXml(regularContent));
            }
            if let Some(toolCallsDeltas) = message.get("tool_calls").and_then(Value::as_array) {
                if !toolCallsDeltas.is_empty() && self.enable_tool_call {
                    for xml in convertToolCallsToXmlChunks(toolCallsDeltas) {
                        state.chunks.push(xml);
                    }
                }
            }
        }

        if let Some(usage) = usage {
            state.usage = parse_usage_counts(usage);
        }
        Ok(())
    }

    fn processResponsesStreamingEvent(
        &self,
        jsonResponse: &Value,
        state: &mut StreamingState,
        on_tool_invocation: Option<&Arc<dyn Fn(String) + Send + Sync>>,
    ) -> Result<(), AiServiceError> {
        let eventType = jsonResponse.get("type").and_then(Value::as_str).unwrap_or("");

        let normalizedStorage;
        let normalized = if eventType.starts_with("response.image_generation_call.") {
            let mut normalized = jsonResponse.clone();
            if let Some(object) = normalized.as_object_mut() {
                object.insert(
                    "type".to_string(),
                    json!(eventType
                        .trim_start_matches("response.")
                        .replace("image_generation_call.", "image_generation.")),
                );
            }
            normalizedStorage = normalized;
            &normalizedStorage
        } else {
            jsonResponse
        };

        let eventType = normalized.get("type").and_then(Value::as_str).unwrap_or("");
        match eventType {
            "response.output_text.delta" => {
                let delta = normalized.get("delta").and_then(Value::as_str).unwrap_or("");
                if !delta.is_empty() {
                    self.processContentDelta("", delta, state);
                }
            }
            "response.reasoning_text.delta" | "response.reasoning_summary_text.delta" => {
                let delta = normalized.get("delta").and_then(Value::as_str).unwrap_or("");
                if !delta.is_empty() {
                    self.processContentDelta(delta, "", state);
                }
            }
            "response.output_item.added" | "response.output_item.done" => {
                if !self.enable_tool_call {
                    return Ok(());
                }
                let outputIndex = normalized.get("output_index").and_then(Value::as_i64).unwrap_or(-1) as i32;
                let Some(item) = normalized.get("item").and_then(Value::as_object) else {
                    return Ok(());
                };
                if outputIndex < 0 || item.get("type").and_then(Value::as_str).unwrap_or("") != "function_call" {
                    return Ok(());
                }
                let mut functionObj = Map::new();
                let name = item.get("name").and_then(Value::as_str).unwrap_or("");
                if !name.is_empty() {
                    functionObj.insert("name".to_string(), json!(name));
                }
                let mut deltaCall = Map::new();
                deltaCall.insert("index".to_string(), json!(outputIndex));
                let callId = item
                    .get("call_id")
                    .or_else(|| item.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if !callId.is_empty() {
                    deltaCall.insert("id".to_string(), json!(callId));
                }
                deltaCall.insert("type".to_string(), json!("function"));
                deltaCall.insert("function".to_string(), Value::Object(functionObj));
                self.processToolCallChunk(
                    outputIndex,
                    &Value::Object(deltaCall),
                    state,
                    on_tool_invocation,
                );
                state.lastProcessedToolIndex = Some(outputIndex);
            }
            "response.function_call_arguments.delta" => {
                if !self.enable_tool_call {
                    return Ok(());
                }
                let outputIndex = normalized.get("output_index").and_then(Value::as_i64).unwrap_or(-1) as i32;
                if outputIndex < 0 {
                    return Ok(());
                }
                let mut functionObj = Map::new();
                let name = normalized.get("name").and_then(Value::as_str).unwrap_or("");
                if !name.is_empty() {
                    functionObj.insert("name".to_string(), json!(name));
                }
                let delta = normalized.get("delta").and_then(Value::as_str).unwrap_or("");
                if !delta.is_empty() {
                    functionObj.insert("arguments".to_string(), json!(delta));
                }
                let deltaCall = json!({
                    "index": outputIndex,
                    "type": "function",
                    "function": Value::Object(functionObj),
                });
                self.processToolCallChunk(outputIndex, &deltaCall, state, on_tool_invocation);
                state.lastProcessedToolIndex = Some(outputIndex);
            }
            "response.function_call_arguments.done" => {
                if !self.enable_tool_call {
                    return Ok(());
                }
                let outputIndex = normalized.get("output_index").and_then(Value::as_i64).unwrap_or(-1) as i32;
                if outputIndex >= 0 {
                    self.closeToolCallIfOpen(outputIndex, state);
                    state.lastProcessedToolIndex = Some(outputIndex);
                }
            }
            "response.completed" => {
                if state.isInReasoningMode {
                    state.isInReasoningMode = false;
                    state.chunks.push("</think>".to_string());
                    state.hasEmittedThinkStart = false;
                }
                self.closeAllOpenToolCalls(state);
                if let Some(usage) = normalized.pointer("/response/usage") {
                    state.usage = parse_usage_counts(usage);
                }
            }
            "response.failed" | "response.error" => {
                let errorMessage = normalized
                    .pointer("/error/message")
                    .or_else(|| normalized.pointer("/response/error/message"))
                    .or_else(|| normalized.pointer("/response/status"))
                    .and_then(Value::as_str)
                    .unwrap_or("Responses stream returned error");
                return Err(AiServiceError::RequestFailed(errorMessage.to_string()));
            }
            value if value.starts_with("image_generation.") => {
                self.processImageGenerationEvent(normalized, state);
            }
            _ => {}
        }

        Ok(())
    }

    fn processImageGenerationEvent(&self, jsonResponse: &Value, state: &mut StreamingState) {
        if let Some(delta) = jsonResponse.get("delta").and_then(Value::as_str) {
            if !delta.is_empty() {
                let outputIndex = jsonResponse.get("output_index").and_then(Value::as_i64).unwrap_or(0);
                state.chunks.push(format!("\n![openai_image_{outputIndex}](data:image/png;base64,{delta})\n"));
            }
        }
        if let Some(completed) = jsonResponse.get("b64_json").and_then(Value::as_str) {
            if !completed.is_empty() {
                let outputIndex = jsonResponse.get("output_index").and_then(Value::as_i64).unwrap_or(0);
                state.chunks.push(format!("\n![openai_image_{outputIndex}](data:image/png;base64,{completed})\n"));
            }
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
                            "parameters": parse_tool_parameters(&tool.parameters)?
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

}

#[async_trait]
impl AIService for OpenAIProvider {
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
        format!("{}:{}", self.provider_type, self.model_name)
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

        let request_body = self.create_request_body(&request)?;
        return self.send_prepared_request(request, request_body).await;
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
                "max_tokens": 1
            }))
            .send()
            .await
            .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;

        if response.status().is_success() {
            Ok("ok".to_string())
        } else {
            let status = response.status();
            let text = response
                .text()
                .await
                .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
            Err(AiServiceError::RequestFailed(format!("{status}: {text}")))
        }
    }

    async fn calculate_input_tokens(
        &self,
        chat_history: &[PromptTurn],
        available_tools: &[ToolPrompt],
    ) -> Result<i32, AiServiceError> {
        let history_chars: usize = chat_history.iter().map(|turn| turn.content.len()).sum();
        let tool_chars: usize = available_tools
            .iter()
            .map(|tool| tool.name.len() + tool.description.len() + tool.parameters.len())
            .sum();
        Ok(((history_chars + tool_chars + 3) / 4) as i32)
    }
}

impl OpenAIProvider {
    pub async fn send_prepared_request(
        &mut self,
        request: SendMessageRequest,
        request_body: Value,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        if request.stream {
            let mut provider = self.clone();
            let event_channel = empty_revisable_event_channel();
            let stream_event_channel = event_channel.clone();
            let mut request_parts = Some((
                self.api_endpoint.clone(),
                self.headers()?,
                request_body,
                request.on_tool_invocation.clone(),
            ));
            let cold_stream = FnStream::new(move |emit| {
                let (api_endpoint, headers, request_body, on_tool_invocation) = request_parts
                    .take()
                    .expect("OpenAIProvider stream must only be collected once");
                let (tx, rx) = channel::<String>();
                let worker_provider = provider.clone();
                let worker_event_channel = stream_event_channel.clone();
                let worker = std::thread::spawn(move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("tokio runtime must build for OpenAIProvider stream");
                    let result: Result<(), AiServiceError> = runtime.block_on(async {
                        let request_savepoint_id = format!("attempt_{}", Uuid::new_v4().simple());
                        let mut emitter = StreamEmitter::new(worker_event_channel.clone());
                        emitter.emit_savepoint(&request_savepoint_id);

                        let result = async {
                            let client = reqwest::Client::new();
                            let response = client
                                .post(&api_endpoint)
                                .headers(headers)
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

                            worker_provider
                                .read_streaming_response(
                                    response,
                                    on_tool_invocation.as_ref(),
                                    &tx,
                                    &mut emitter,
                                )
                                .await
                        }
                        .await;
                        if result.is_err() {
                            let _ = emitter.emit_rollback(&request_savepoint_id);
                        }
                        result
                    });
                    worker_event_channel.close();
                    if let Err(error) = result {
                        let _ = error;
                    }
                });
                while let Ok(chunk) = rx.recv() {
                    emit(chunk);
                }
                let _ = worker.join();
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
            return Err(AiServiceError::RequestFailed(format!("{status}: {message}")));
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

        Ok(response_stream_from_chunks(chunks))
    }

}

fn parse_tool_parameters(parameters: &str) -> Result<Value, AiServiceError> {
    serde_json::from_str(parameters).map_err(|error| AiServiceError::RequestFailed(error.to_string()))
}

fn escapeXml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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

fn convertToolCallsToXmlChunks(tool_calls: &[Value]) -> Vec<String> {
    tool_calls
        .iter()
        .map(|tool_call| StructuredToolCallBridge::convertToolCallPayloadToXml(&tool_call.to_string()))
        .filter(|content| !content.trim().is_empty())
        .collect()
}

fn emit_new_chunks(
    state: &StreamingState,
    before_len: usize,
    callback: Option<&Arc<dyn Fn(String) + Send + Sync>>,
) {
    let Some(callback) = callback else {
        return;
    };
    for chunk in state.chunks.iter().skip(before_len) {
        callback(chunk.clone());
    }
}
