use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
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
use crate::data::model::ModelParameter::{ParameterCategory, ModelParameter};
use crate::data::model::ToolPrompt::ToolPrompt;
use crate::util::stream::RevisableTextStream::RevisableTextStreamLike;
use crate::util::ChatMarkupRegex::{attr_value, tag_ranges, ChatMarkupRegex};

pub struct GeminiProvider {
    pub api_endpoint: String,
    pub api_key: String,
    pub model_name: String,
    pub provider_type: String,
    pub custom_headers: Vec<(String, String)>,
    pub enable_google_search: bool,
    pub enable_tool_call: bool,
    inputTokenCount: i32,
    cachedInputTokenCount: i32,
    outputTokenCount: i32,
    cancelled: bool,
    isInThinkingMode: bool,
}

struct GeminiFunctionCallPayload {
    text_content: String,
    function_calls: Vec<Value>,
    thought_signature: Option<String>,
}

impl GeminiProvider {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_endpoint: String,
        api_key: String,
        model_name: String,
        provider_type: String,
        custom_headers: Vec<(String, String)>,
        enable_google_search: bool,
        enable_tool_call: bool,
    ) -> Self {
        Self {
            api_endpoint,
            api_key,
            model_name,
            provider_type,
            custom_headers,
            enable_google_search,
            enable_tool_call,
            inputTokenCount: 0,
            cachedInputTokenCount: 0,
            outputTokenCount: 0,
            cancelled: false,
            isInThinkingMode: false,
        }
    }

    pub fn create_request_body(&mut self, request: &SendMessageRequest) -> Result<Value, AiServiceError> {
        let mut root = Map::new();
        let mut tools = Vec::new();
        if self.enable_tool_call && !request.available_tools.is_empty() {
            let declarations = self.build_tool_definitions_for_gemini(&request.available_tools);
            if !declarations.is_empty() {
                tools.push(json!({"function_declarations": declarations}));
            }
        }
        if self.enable_google_search {
            tools.push(json!({"googleSearch": {}}));
        }
        let tools_json = if tools.is_empty() {
            None
        } else {
            root.insert("tools".to_string(), Value::Array(tools.clone()));
            Some(Value::Array(tools).to_string())
        };

        let (contents, system_instruction, token_count) = self.build_contents_and_count_tokens(
            &request.chat_history,
            tools_json.as_deref(),
            request.preserve_think_in_history,
        )?;
        self.inputTokenCount = token_count;
        self.cachedInputTokenCount = 0;
        if let Some(system_instruction) = system_instruction {
            root.insert("systemInstruction".to_string(), system_instruction);
        }
        root.insert("contents".to_string(), Value::Array(contents));

        let mut generation_config = Map::new();
        if request.enable_thinking {
            generation_config.insert("thinkingConfig".to_string(), json!({"includeThoughts": true}));
        }
        self.apply_model_parameters(&mut root, &mut generation_config, &request.model_parameters)?;
        root.insert("generationConfig".to_string(), Value::Object(generation_config));
        Ok(Value::Object(root))
    }

    fn build_contents_and_count_tokens(
        &self,
        chat_history: &[PromptTurn],
        tools_json: Option<&str>,
        preserve_think_in_history: bool,
    ) -> Result<(Vec<Value>, Option<Value>, i32), AiServiceError> {
        let provider_ready_history = StructuredToolCallBridge::compileHistoryForProvider(
            chat_history,
            self.enable_tool_call,
        );
        let history_chars: usize = provider_ready_history.iter().map(|turn| turn.content.chars().count()).sum();
        let tools_chars = tools_json.map(str::len).unwrap_or(0);
        let token_count = ((history_chars + tools_chars + 3) / 4) as i32;

        let mut contents_array = Vec::new();
        let system_messages = provider_ready_history
            .iter()
            .filter(|turn| turn.kind == PromptTurnKind::SYSTEM)
            .map(|turn| turn.content.clone())
            .collect::<Vec<_>>();
        let system_instruction = if system_messages.is_empty() {
            None
        } else {
            Some(json!({"parts": [{"text": system_messages.join("\n\n")}]}))
        };

        let history_without_system = provider_ready_history
            .iter()
            .filter(|turn| turn.kind != PromptTurnKind::SYSTEM)
            .cloned()
            .collect::<Vec<_>>();

        let mut queued_assistant_tool_text: Option<String> = None;
        let mut queued_assistant_thought_signature: Option<String> = None;
        let mut queued_function_calls: Vec<Value> = Vec::new();
        let mut open_function_call_names: Vec<String> = Vec::new();

        for turn in history_without_system {
            let content = if !preserve_think_in_history && turn.kind == PromptTurnKind::ASSISTANT {
                remove_thinking_content(&turn.content)
            } else {
                turn.content.clone()
            };
            let content_without_gemini_meta = ChatMarkupRegex::remove_gemini_thought_signature_meta(&content);

            if self.enable_tool_call {
                match turn.kind {
                    PromptTurnKind::ASSISTANT => {
                        let payload = self.parse_xml_tool_calls(&content);
                        if !payload.function_calls.is_empty() {
                            if !open_function_call_names.is_empty() {
                                flush_open_function_calls_as_cancelled(&mut contents_array, &mut queued_assistant_tool_text, &mut queued_assistant_thought_signature, &mut queued_function_calls, &mut open_function_call_names);
                            }
                            queue_function_calls(
                                &mut queued_assistant_tool_text,
                                &mut queued_assistant_thought_signature,
                                &mut queued_function_calls,
                                &payload.text_content,
                                payload.function_calls,
                                payload.thought_signature,
                            );
                        } else {
                            flush_open_function_calls_as_cancelled(&mut contents_array, &mut queued_assistant_tool_text, &mut queued_assistant_thought_signature, &mut queued_function_calls, &mut open_function_call_names);
                            contents_array.push(json!({"role": "model", "parts": self.build_parts_array(&content_without_gemini_meta)}));
                        }
                    }
                    PromptTurnKind::TOOL_CALL => {
                        let payload = self.parse_xml_tool_calls(&content);
                        if !payload.function_calls.is_empty() {
                            if !open_function_call_names.is_empty() {
                                flush_open_function_calls_as_cancelled(&mut contents_array, &mut queued_assistant_tool_text, &mut queued_assistant_thought_signature, &mut queued_function_calls, &mut open_function_call_names);
                            }
                            queue_function_calls(
                                &mut queued_assistant_tool_text,
                                &mut queued_assistant_thought_signature,
                                &mut queued_function_calls,
                                &payload.text_content,
                                payload.function_calls,
                                payload.thought_signature,
                            );
                        } else {
                            flush_open_function_calls_as_cancelled(&mut contents_array, &mut queued_assistant_tool_text, &mut queued_assistant_thought_signature, &mut queued_function_calls, &mut open_function_call_names);
                            contents_array.push(json!({"role": "model", "parts": self.build_parts_array(&content_without_gemini_meta)}));
                        }
                    }
                    PromptTurnKind::USER | PromptTurnKind::SUMMARY => {
                        let mut parts = Vec::new();
                        append_cancelled_open_function_responses(
                            &mut contents_array,
                            &mut parts,
                            &mut queued_assistant_tool_text,
                            &mut queued_assistant_thought_signature,
                            &mut queued_function_calls,
                            &mut open_function_call_names,
                        );
                        parts.extend(self.build_parts_array(&content_without_gemini_meta));
                        contents_array.push(json!({"role": "user", "parts": parts}));
                    }
                    PromptTurnKind::TOOL_RESULT => {
                        emit_queued_function_calls_if_needed(&mut contents_array, &mut queued_assistant_tool_text, &mut queued_assistant_thought_signature, &mut queued_function_calls, &mut open_function_call_names);
                        let (text_content, responses_list) = self.parse_xml_tool_results(&content_without_gemini_meta);
                        if !responses_list.is_empty() && !open_function_call_names.is_empty() {
                            let valid_count = responses_list.len().min(open_function_call_names.len());
                            let mut parts = Vec::new();
                            for index in 0..valid_count {
                                let mut response = responses_list[index].clone();
                                if let Some(object) = response.as_object_mut() {
                                    let pending_name = &open_function_call_names[index];
                                    if !pending_name.trim().is_empty() {
                                        object.insert("name".to_string(), json!(pending_name));
                                    }
                                }
                                parts.push(json!({"functionResponse": response}));
                            }
                            open_function_call_names.drain(0..valid_count);
                            if !text_content.is_empty() {
                                parts.extend(self.build_parts_array(&text_content));
                            }
                            contents_array.push(json!({"role": "user", "parts": parts}));
                        } else {
                            let mut parts = Vec::new();
                            append_cancelled_open_function_responses(
                                &mut contents_array,
                                &mut parts,
                                &mut queued_assistant_tool_text,
                                &mut queued_assistant_thought_signature,
                                &mut queued_function_calls,
                                &mut open_function_call_names,
                            );
                            let content = if !text_content.is_empty() {
                                text_content
                            } else if !content_without_gemini_meta.trim().is_empty() {
                                content_without_gemini_meta
                            } else {
                                "[Empty]".to_string()
                            };
                            parts.extend(self.build_parts_array(&content));
                            contents_array.push(json!({"role": "user", "parts": parts}));
                        }
                    }
                    PromptTurnKind::SYSTEM => {}
                }
            } else {
                let role = match turn.kind {
                    PromptTurnKind::ASSISTANT | PromptTurnKind::TOOL_CALL => "model",
                    _ => "user",
                };
                contents_array.push(json!({"role": role, "parts": self.build_parts_array(&content_without_gemini_meta)}));
            }
        }

        flush_open_function_calls_as_cancelled(&mut contents_array, &mut queued_assistant_tool_text, &mut queued_assistant_thought_signature, &mut queued_function_calls, &mut open_function_call_names);

        Ok((contents_array, system_instruction, token_count))
    }

    fn build_parts_array(&self, text: &str) -> Vec<Value> {
        vec![json!({"text": text})]
    }

    fn parse_xml_tool_calls(&self, content: &str) -> GeminiFunctionCallPayload {
        if !self.enable_tool_call {
            return GeminiFunctionCallPayload {
                text_content: content.to_string(),
                function_calls: Vec::new(),
                thought_signature: None,
            };
        }
        let thought_signature = ChatMarkupRegex::extract_gemini_thought_signature(content)
            .and_then(|value| general_purpose::STANDARD.decode(value).ok())
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .filter(|value| !value.is_empty());
        let sanitized_content = ChatMarkupRegex::remove_gemini_thought_signature_meta(content);
        let matches = ChatMarkupRegex::tool_call_matches(&sanitized_content);
        if matches.is_empty() {
            return GeminiFunctionCallPayload {
                text_content: sanitized_content,
                function_calls: Vec::new(),
                thought_signature: None,
            };
        }
        let mut text_content = sanitized_content.clone();
        let mut function_calls = Vec::new();
        for tool_match in matches {
            let mut args = Map::new();
            for (start, end) in tag_ranges(&tool_match.body, "param") {
                let raw = &tool_match.body[start..end];
                let param_name = attr_value(raw, "name").unwrap_or_default();
                let param_value = raw
                    .split_once('>')
                    .and_then(|(_, tail)| tail.rsplit_once("</").map(|(body, _)| xml_unescape(body)))
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                args.insert(param_name, json!(param_value));
            }
            function_calls.push(json!({"name": tool_match.name, "args": Value::Object(args)}));
            text_content = text_content.replace(&format!(
                "<{} name=\"{}\">{}</{}>",
                tool_match.tag_name, tool_match.name, tool_match.body, tool_match.tag_name
            ), "");
        }
        GeminiFunctionCallPayload {
            text_content: text_content.trim().to_string(),
            function_calls,
            thought_signature,
        }
    }

    fn parse_xml_tool_results(&self, content: &str) -> (String, Vec<Value>) {
        if !self.enable_tool_call {
            return (content.to_string(), Vec::new());
        }
        let blocks = ChatMarkupRegex::tool_result_blocks(content);
        if blocks.is_empty() {
            return (content.to_string(), Vec::new());
        }
        let mut text_content = content.to_string();
        let mut responses = Vec::new();
        for block in blocks {
            let tool_name = attr_value(&block.raw, "name").unwrap_or_default();
            let result_content = tag_ranges(&block.body, "content")
                .into_iter()
                .next()
                .and_then(|(start, end)| {
                    let raw = &block.body[start..end];
                    raw.split_once('>')
                        .and_then(|(_, tail)| tail.rsplit_once("</").map(|(body, _)| body.trim().to_string()))
                })
                .unwrap_or_else(|| block.body.trim().to_string());
            responses.push(json!({
                "name": tool_name,
                "response": {"result": result_content},
            }));
            text_content = text_content.replace(&block.raw, "").trim().to_string();
        }
        (text_content.trim().to_string(), responses)
    }

    fn build_tool_definitions_for_gemini(&self, tool_prompts: &[ToolPrompt]) -> Vec<Value> {
        tool_prompts
            .iter()
            .map(|tool| {
                let full_description = if !tool.details.is_empty() {
                    format!("{}\n{}", tool.description, tool.details)
                } else {
                    tool.description.clone()
                };
                json!({
                    "name": tool.name,
                    "description": full_description,
                    "parameters": build_schema_from_structured(tool.parametersStructured.as_deref().unwrap_or(&[])),
                })
            })
            .collect()
    }

    fn apply_model_parameters(
        &self,
        root: &mut Map<String, Value>,
        generation_config: &mut Map<String, Value>,
        parameters: &[ModelParameter<Value>],
    ) -> Result<(), AiServiceError> {
        for parameter in parameters {
            if !parameter.isEnabled {
                continue;
            }
            match parameter.apiName.as_str() {
                "temperature" => {
                    generation_config.insert("temperature".to_string(), parameter.currentValue.clone());
                }
                "top_p" => {
                    generation_config.insert("topP".to_string(), parameter.currentValue.clone());
                }
                "top_k" => {
                    generation_config.insert("topK".to_string(), parameter.currentValue.clone());
                }
                "max_tokens" => {
                    generation_config.insert("maxOutputTokens".to_string(), parameter.currentValue.clone());
                }
                api_name => {
                    if parameter.category == ParameterCategory::OTHER {
                        root.insert(api_name.to_string(), parameter.currentValue.clone());
                    } else {
                        generation_config.insert(api_name.to_string(), parameter.currentValue.clone());
                    }
                }
            }
        }
        Ok(())
    }

    fn request_url(&self, stream: bool) -> String {
        let base_url = determine_base_url(&self.api_endpoint);
        let method = if stream { "streamGenerateContent" } else { "generateContent" };
        let url = format!("{base_url}/v1beta/models/{}:{method}", self.model_name);
        if url.contains('?') {
            format!("{url}&key={}", self.api_key)
        } else {
            format!("{url}?key={}", self.api_key)
        }
    }

    fn headers(&self) -> Result<HeaderMap, AiServiceError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
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

    async fn process_streaming_response(
        &mut self,
        response: reqwest::Response,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        let mut chunks = Vec::new();
        let mut pending = String::new();
        let mut bytes_stream = response.bytes_stream();
        while let Some(item) = bytes_stream.next().await {
            if self.cancelled {
                break;
            }
            let bytes = item.map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
            pending.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(newline_index) = pending.find('\n') {
                let line = pending[..newline_index].trim().to_string();
                pending = pending[newline_index + 1..].to_string();
                self.process_response_line(&line, &mut chunks)?;
            }
        }
        let tail = pending.trim().to_string();
        if !tail.is_empty() {
            self.process_response_line(&tail, &mut chunks)?;
        }
        if self.isInThinkingMode {
            chunks.push("</think>".to_string());
            self.isInThinkingMode = false;
        }
        if chunks.is_empty() {
            chunks.push(" ".to_string());
        }
        Ok(response_stream_from_chunks(chunks))
    }

    fn process_response_line(&mut self, line: &str, chunks: &mut Vec<String>) -> Result<(), AiServiceError> {
        if line.is_empty() || line == "[" || line == "]" {
            return Ok(());
        }
        let data = if line.starts_with("data: ") {
            line.trim_start_matches("data: ").trim()
        } else if line.starts_with("data:") {
            line.trim_start_matches("data:").trim()
        } else {
            line.trim().trim_end_matches(',')
        };
        if data == "[DONE]" || data.is_empty() || data == "[" || data == "]" {
            return Ok(());
        }
        let json: Value = serde_json::from_str(data)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        let content = self.extract_content_from_json(&json)?;
        if !content.is_empty() {
            chunks.push(content);
        }
        Ok(())
    }

    fn extract_content_from_json(&mut self, json_response: &Value) -> Result<String, AiServiceError> {
        if let Some(error) = json_response.get("error") {
            return Err(AiServiceError::RequestFailed(error.to_string()));
        }
        let mut content_builder = String::new();
        let mut search_sources_builder = String::new();
        if self.enable_google_search {
            if let Some(metadata) = json_response.pointer("/candidates/0/groundingMetadata") {
                if let Some(queries) = metadata.get("webSearchQueries").and_then(Value::as_array) {
                    if !queries.is_empty() {
                        search_sources_builder.push_str("\n<search>\n\nSearch sources:\n");
                        for query in queries {
                            if let Some(query) = query.as_str() {
                                search_sources_builder.push_str(&format!("Query: {query}\n"));
                            }
                        }
                        if let Some(chunks) = metadata.get("groundingChunks").and_then(Value::as_array) {
                            for (index, chunk) in chunks.iter().enumerate() {
                                let uri = chunk.pointer("/web/uri").and_then(Value::as_str).unwrap_or("");
                                let title = chunk.pointer("/web/title").and_then(Value::as_str).unwrap_or("");
                                if !uri.is_empty() {
                                    if title.is_empty() {
                                        search_sources_builder.push_str(&format!("{}. <{}>\n", index + 1, uri));
                                    } else {
                                        search_sources_builder.push_str(&format!("{}. [{}]({})\n", index + 1, title, uri));
                                    }
                                }
                            }
                        }
                        search_sources_builder.push_str("\n</search>\n\n");
                    }
                }
            }
        }
        let Some(parts) = json_response.pointer("/candidates/0/content/parts").and_then(Value::as_array) else {
            self.apply_usage(json_response.get("usageMetadata"));
            return Ok(String::new());
        };
        let mut pending_thought_signatures = Vec::new();
        for (index, part) in parts.iter().enumerate() {
            let is_thought = part.get("thought").and_then(Value::as_bool).unwrap_or(false);
            if let Some(function_call) = part.get("functionCall") {
                if self.enable_tool_call {
                    if self.isInThinkingMode {
                        content_builder.push_str("</think>");
                        self.isInThinkingMode = false;
                    }
                    let tool_name = function_call.get("name").and_then(Value::as_str).unwrap_or("");
                    if !tool_name.is_empty() {
                        let tag = ChatMarkupRegex::generate_random_tool_tag_name();
                        content_builder.push_str(&format!("\n<{tag} name=\"{tool_name}\">"));
                        if let Some(args) = function_call.get("args") {
                            let mut converter = StreamingJsonXmlConverter::new();
                            append_converter_events_to_string(&mut content_builder, converter.feed(&args.to_string()));
                            append_converter_events_to_string(&mut content_builder, converter.flush());
                        }
                        content_builder.push_str(&format!("\n</{tag}>\n"));
                    }
                    if let Some(signature) = opt_gemini_thought_signature(part) {
                        pending_thought_signatures.push(signature);
                    }
                }
            }
            let inline_data = part.get("inline_data").or_else(|| part.get("inlineData"));
            if let Some(inline_data) = inline_data {
                let mime_type = inline_data
                    .get("mime_type")
                    .or_else(|| inline_data.get("mimeType"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let b64 = inline_data.get("data").and_then(Value::as_str).unwrap_or("");
                if mime_type.starts_with("image/") && !b64.is_empty() {
                    if self.isInThinkingMode {
                        content_builder.push_str("</think>");
                        self.isInThinkingMode = false;
                    }
                    content_builder.push_str(&format!("\n![gemini_image_{index}](data:{mime_type};base64,{b64})\n"));
                }
            }
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                if !text.is_empty() {
                    if is_thought && !self.isInThinkingMode {
                        content_builder.push_str("<think>");
                        self.isInThinkingMode = true;
                    } else if !is_thought && self.isInThinkingMode {
                        content_builder.push_str("</think>");
                        self.isInThinkingMode = false;
                    }
                    content_builder.push_str(text);
                    self.outputTokenCount += ((text.chars().count() + 3) / 4) as i32;
                }
            }
        }
        for signature in pending_thought_signatures {
            if !signature.is_empty() {
                if !content_builder.is_empty() && !content_builder.ends_with('\n') {
                    content_builder.push('\n');
                }
                let encoded = general_purpose::STANDARD.encode(signature.as_bytes());
                content_builder.push_str(&ChatMarkupRegex::gemini_thought_signature_meta_tag(&encoded));
            }
        }
        self.apply_usage(json_response.get("usageMetadata"));
        Ok(format!("{search_sources_builder}{content_builder}"))
    }

    fn apply_usage(&mut self, usage: Option<&Value>) {
        let Some(usage) = usage else {
            return;
        };
        let prompt = usage.get("promptTokenCount").and_then(Value::as_i64).unwrap_or(0).max(0) as i32;
        let cached = usage.get("cachedContentTokenCount").and_then(Value::as_i64).unwrap_or(0).max(0) as i32;
        let candidates = usage.get("candidatesTokenCount").and_then(Value::as_i64).unwrap_or(0).max(0) as i32;
        if prompt > 0 || cached > 0 || candidates > 0 {
            self.inputTokenCount = (prompt - cached).max(0);
            self.cachedInputTokenCount = cached;
            self.outputTokenCount = candidates;
        }
    }
}

#[async_trait]
impl AIService for GeminiProvider {
    fn input_token_count(&self) -> i32 { self.inputTokenCount }
    fn cached_input_token_count(&self) -> i32 { self.cachedInputTokenCount }
    fn output_token_count(&self) -> i32 { self.outputTokenCount }
    fn provider_model(&self) -> String { format!("{}:{}", self.provider_type, self.model_name) }

    fn reset_token_counts(&mut self) {
        self.inputTokenCount = 0;
        self.cachedInputTokenCount = 0;
        self.outputTokenCount = 0;
        self.isInThinkingMode = false;
    }

    fn cancel_streaming(&mut self) {
        self.cancelled = true;
    }

    async fn send_message(
        &mut self,
        request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        self.cancelled = false;
        self.reset_token_counts();
        let stream = request.stream;
        let request_body = self.create_request_body(&request)?;
        let response = reqwest::Client::new()
            .post(self.request_url(stream))
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
        let mut chunks = Vec::new();
        let content = self.extract_content_from_json(&json_response)?;
        if content.is_empty() {
            chunks.push(" ".to_string());
        } else {
            chunks.push(content);
        }
        if self.isInThinkingMode {
            chunks.push("</think>".to_string());
            self.isInThinkingMode = false;
        }
        Ok(response_stream_from_chunks(chunks))
    }

    async fn calculate_input_tokens(&self, chat_history: &[PromptTurn], available_tools: &[ToolPrompt]) -> Result<i32, AiServiceError> {
        let tools_json = if self.enable_tool_call && !available_tools.is_empty() {
            Some(Value::Array(self.build_tool_definitions_for_gemini(available_tools)).to_string())
        } else if self.enable_google_search {
            Some(json!([{"googleSearch": {}}]).to_string())
        } else {
            None
        };
        let (_, _, token_count) = self.build_contents_and_count_tokens(chat_history, tools_json.as_deref(), false)?;
        Ok(token_count)
    }
}

fn emit_queued_function_calls_if_needed(
    contents_array: &mut Vec<Value>,
    queued_assistant_tool_text: &mut Option<String>,
    queued_assistant_thought_signature: &mut Option<String>,
    queued_function_calls: &mut Vec<Value>,
    open_function_call_names: &mut Vec<String>,
) {
    if queued_function_calls.is_empty() {
        return;
    }
    let mut parts = Vec::new();
    if let Some(text) = queued_assistant_tool_text.take() {
        if !text.trim().is_empty() {
            parts.push(json!({"text": text}));
        }
    }
    for (index, function_call) in queued_function_calls.iter().enumerate() {
        let mut part = Map::new();
        part.insert("functionCall".to_string(), function_call.clone());
        if index == 0 {
            if let Some(signature) = queued_assistant_thought_signature.take() {
                part.insert("thought_signature".to_string(), json!(signature));
            }
        }
        parts.push(Value::Object(part));
        open_function_call_names.push(
            function_call
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string(),
        );
    }
    contents_array.push(json!({"role": "model", "parts": parts}));
    queued_function_calls.clear();
}

fn append_cancelled_open_function_responses(
    contents_array: &mut Vec<Value>,
    target: &mut Vec<Value>,
    queued_assistant_tool_text: &mut Option<String>,
    queued_assistant_thought_signature: &mut Option<String>,
    queued_function_calls: &mut Vec<Value>,
    open_function_call_names: &mut Vec<String>,
) -> bool {
    emit_queued_function_calls_if_needed(
        contents_array,
        queued_assistant_tool_text,
        queued_assistant_thought_signature,
        queued_function_calls,
        open_function_call_names,
    );
    if open_function_call_names.is_empty() {
        return false;
    }
    for name in open_function_call_names.iter() {
        target.push(json!({
            "functionResponse": {
                "name": if name.trim().is_empty() { "cancelled_function" } else { name },
                "response": {"result": "User cancelled"},
            }
        }));
    }
    open_function_call_names.clear();
    true
}

fn flush_open_function_calls_as_cancelled(
    contents_array: &mut Vec<Value>,
    queued_assistant_tool_text: &mut Option<String>,
    queued_assistant_thought_signature: &mut Option<String>,
    queued_function_calls: &mut Vec<Value>,
    open_function_call_names: &mut Vec<String>,
) {
    emit_queued_function_calls_if_needed(
        contents_array,
        queued_assistant_tool_text,
        queued_assistant_thought_signature,
        queued_function_calls,
        open_function_call_names,
    );
    let mut parts = Vec::new();
    if append_cancelled_open_function_responses(
        contents_array,
        &mut parts,
        queued_assistant_tool_text,
        queued_assistant_thought_signature,
        queued_function_calls,
        open_function_call_names,
    ) {
        contents_array.push(json!({"role": "user", "parts": parts}));
    }
}

fn queue_function_calls(
    queued_assistant_tool_text: &mut Option<String>,
    queued_assistant_thought_signature: &mut Option<String>,
    queued_function_calls: &mut Vec<Value>,
    text_content: &str,
    function_calls: Vec<Value>,
    thought_signature: Option<String>,
) {
    if !text_content.trim().is_empty() {
        *queued_assistant_tool_text = Some(match queued_assistant_tool_text.take() {
            Some(existing) if !existing.trim().is_empty() => format!("{existing}\n{text_content}"),
            _ => text_content.to_string(),
        });
    }
    if queued_assistant_thought_signature.is_none() {
        *queued_assistant_thought_signature = thought_signature;
    }
    queued_function_calls.extend(function_calls);
}

fn build_schema_from_structured(params: &[crate::data::model::ToolPrompt::ToolParameterSchema]) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for param in params {
        let mut property = Map::new();
        property.insert("type".to_string(), json!(param.r#type));
        property.insert("description".to_string(), json!(param.description));
        if let Some(default) = &param.default {
            property.insert("default".to_string(), json!(default));
        }
        properties.insert(param.name.clone(), Value::Object(property));
        if param.required {
            required.push(Value::String(param.name.clone()));
        }
    }
    let mut schema = Map::new();
    schema.insert("type".to_string(), json!("object"));
    schema.insert("properties".to_string(), Value::Object(properties));
    if !required.is_empty() {
        schema.insert("required".to_string(), Value::Array(required));
    }
    Value::Object(schema)
}

fn determine_base_url(endpoint: &str) -> String {
    let trimmed = endpoint.trim_end_matches('/');
    if let Some(index) = trimmed.find("/v1beta") {
        return trimmed[..index].to_string();
    }
    trimmed.to_string()
}

fn opt_gemini_thought_signature(value: &Value) -> Option<String> {
    value
        .get("thoughtSignature")
        .or_else(|| value.get("thought_signature"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn append_converter_events_to_string(target: &mut String, events: Vec<StreamingJsonXmlEvent>) {
    for event in events {
        match event {
            StreamingJsonXmlEvent::Tag(text) | StreamingJsonXmlEvent::Content(text) => target.push_str(&text),
        }
    }
}

fn remove_thinking_content(content: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    let mut ranges = ChatMarkupRegex::think_ranges(content);
    ranges.sort_by_key(|range| range.0);
    for (start, end) in ranges {
        output.push_str(&content[cursor..start]);
        cursor = end;
    }
    output.push_str(&content[cursor..]);
    output
}

fn xml_unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}
