use std::collections::HashSet;

use serde_json::{json, Map, Value};

use crate::core::chat::hooks::PromptTurn::{PromptTurn, PromptTurnKind};
use crate::data::model::ToolPrompt::ToolPrompt;
use crate::util::ChatMarkupRegex::{attr_value, tag_ranges, ChatMarkupRegex};

#[derive(Clone, Debug, PartialEq, Eq)]
enum ProviderHistoryBlockType {
    ASSISTANT,
    USER_INPUT,
    TOOL_RESULT,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ToolResultRecord {
    name: Option<String>,
    content: String,
}

pub struct StructuredToolCallBridge;

impl StructuredToolCallBridge {
    pub fn buildToolsJson(toolPrompts: Option<&[ToolPrompt]>) -> Option<String> {
        let toolPrompts = toolPrompts?;
        if toolPrompts.is_empty() {
            return None;
        }
        let tools = Self::buildToolDefinitions(toolPrompts);
        tools.as_array().filter(|items| !items.is_empty()).map(|_| tools.to_string())
    }

    pub fn buildToolsArray(toolPrompts: Option<&[ToolPrompt]>) -> Value {
        match toolPrompts {
            Some(toolPrompts) if !toolPrompts.is_empty() => Self::buildToolDefinitions(toolPrompts),
            _ => Value::Array(Vec::new()),
        }
    }

    pub fn buildMessagesJson(history: &[PromptTurn], preserveThinkInHistory: bool) -> String {
        Self::buildStructuredMessages(history, preserveThinkInHistory).to_string()
    }

    pub fn buildMnnChatHistory(history: &[PromptTurn], preserveThinkInHistory: bool) -> Vec<(String, String)> {
        let messages = Self::buildStructuredMessages(history, preserveThinkInHistory);
        let mut compiledHistory = Vec::new();
        let Some(messages) = messages.as_array() else {
            return compiledHistory;
        };
        for message in messages {
            let Some(message) = message.as_object() else {
                continue;
            };
            let role = message.get("role").and_then(Value::as_str).unwrap_or("").trim().to_string();
            let contentValue = message.get("content");
            let content = match contentValue {
                None | Some(Value::Null) => String::new(),
                Some(Value::String(value)) => value.clone(),
                Some(value) => value.to_string(),
            };
            let isPlainRoleContentMessage = !role.is_empty()
                && message.len() == 2
                && message.contains_key("role")
                && message.contains_key("content")
                && contentValue.map(|value| value.is_null() || value.is_string()).unwrap_or(true);
            if isPlainRoleContentMessage {
                compiledHistory.push((role, content));
            } else {
                compiledHistory.push(("json".to_string(), Value::Object(message.clone()).to_string()));
            }
        }
        compiledHistory
    }

    pub fn compileHistoryForProvider(history: &[PromptTurn], useToolCall: bool) -> Vec<PromptTurn> {
        if history.is_empty() {
            return history.to_vec();
        }

        let mut compiled = Vec::new();
        let mut currentBlockType: Option<ProviderHistoryBlockType> = None;
        let mut currentContent = String::new();
        let mut currentMetadata = Map::new();

        fn flushCurrentBlock(
            compiled: &mut Vec<PromptTurn>,
            currentBlockType: &mut Option<ProviderHistoryBlockType>,
            currentContent: &mut String,
            currentMetadata: &mut Map<String, Value>,
            useToolCall: bool,
        ) {
            let Some(blockType) = currentBlockType.take() else {
                return;
            };
            let kind = match blockType {
                ProviderHistoryBlockType::ASSISTANT => PromptTurnKind::ASSISTANT,
                ProviderHistoryBlockType::USER_INPUT => PromptTurnKind::USER,
                ProviderHistoryBlockType::TOOL_RESULT => {
                    if useToolCall {
                        PromptTurnKind::TOOL_RESULT
                    } else {
                        PromptTurnKind::USER
                    }
                }
            };
            let mut turn = PromptTurn::new(kind, currentContent.trim().to_string());
            turn.metadata = currentMetadata.clone().into_iter().collect();
            compiled.push(turn);
            currentContent.clear();
            currentMetadata.clear();
        }

        fn appendToBlock(
            compiled: &mut Vec<PromptTurn>,
            currentBlockType: &mut Option<ProviderHistoryBlockType>,
            currentContent: &mut String,
            currentMetadata: &mut Map<String, Value>,
            blockType: ProviderHistoryBlockType,
            turn: &PromptTurn,
            useToolCall: bool,
        ) {
            if currentBlockType.as_ref() != Some(&blockType) {
                flushCurrentBlock(compiled, currentBlockType, currentContent, currentMetadata, useToolCall);
                *currentBlockType = Some(blockType);
            }
            let trimmedContent = turn.content.trim();
            if !trimmedContent.is_empty() {
                if !currentContent.is_empty() {
                    currentContent.push('\n');
                }
                currentContent.push_str(trimmedContent);
            }
            if !turn.metadata.is_empty() {
                currentMetadata.extend(turn.metadata.clone());
            }
        }

        for turn in history {
            match turn.kind {
                PromptTurnKind::SYSTEM => {
                    flushCurrentBlock(
                        &mut compiled,
                        &mut currentBlockType,
                        &mut currentContent,
                        &mut currentMetadata,
                        useToolCall,
                    );
                    compiled.push(turn.clone());
                }
                PromptTurnKind::ASSISTANT | PromptTurnKind::TOOL_CALL => appendToBlock(
                    &mut compiled,
                    &mut currentBlockType,
                    &mut currentContent,
                    &mut currentMetadata,
                    ProviderHistoryBlockType::ASSISTANT,
                    turn,
                    useToolCall,
                ),
                PromptTurnKind::TOOL_RESULT => appendToBlock(
                    &mut compiled,
                    &mut currentBlockType,
                    &mut currentContent,
                    &mut currentMetadata,
                    ProviderHistoryBlockType::TOOL_RESULT,
                    turn,
                    useToolCall,
                ),
                PromptTurnKind::USER | PromptTurnKind::SUMMARY => appendToBlock(
                    &mut compiled,
                    &mut currentBlockType,
                    &mut currentContent,
                    &mut currentMetadata,
                    ProviderHistoryBlockType::USER_INPUT,
                    turn,
                    useToolCall,
                ),
            }
        }

        flushCurrentBlock(
            &mut compiled,
            &mut currentBlockType,
            &mut currentContent,
            &mut currentMetadata,
            useToolCall,
        );
        compiled
    }

    pub fn convertToolCallPayloadToXml(content: &str) -> String {
        if content.trim().is_empty() {
            return content.to_string();
        }
        if ChatMarkupRegex::contains_any_tool_like_tag(content) {
            return content.to_string();
        }
        let Some(toolCalls) = Self::parsePossibleToolCallsFromText(content) else {
            return content.to_string();
        };
        let xml = Self::convertToolCallsToXml(&toolCalls);
        if xml.trim().is_empty() {
            content.to_string()
        } else {
            xml
        }
    }

    pub fn build_tools_json(tool_prompts: &[ToolPrompt]) -> String {
        Self::buildToolsJson(Some(tool_prompts)).unwrap_or_else(|| "[]".to_string())
    }

    pub fn build_messages_json(history: &[PromptTurn], preserve_think_in_history: bool) -> String {
        Self::buildMessagesJson(history, preserve_think_in_history)
    }

    pub fn compile_history_for_provider(history: &[PromptTurn], use_tool_call: bool) -> Vec<PromptTurn> {
        Self::compileHistoryForProvider(history, use_tool_call)
    }

    pub fn convert_tool_call_payload_to_xml(content: &str) -> String {
        Self::convertToolCallPayloadToXml(content)
    }

    fn buildStructuredMessages(history: &[PromptTurn], preserveThinkInHistory: bool) -> Value {
        let mergedHistory = Self::compileHistoryForProvider(history, true);
        let mut messagesArray = Vec::new();
        let mut queuedAssistantToolText: Option<String> = None;
        let mut queuedToolCalls = Vec::new();
        let mut queuedToolCallIds = Vec::new();
        let mut openToolCallIds = Vec::new();

        fn appendQueuedAssistantToolText(queuedAssistantToolText: &mut Option<String>, text: &str) {
            if text.trim().is_empty() {
                return;
            }
            *queuedAssistantToolText = Some(match queuedAssistantToolText.take() {
                Some(existing) if !existing.trim().is_empty() => format!("{existing}\n{text}"),
                _ => text.to_string(),
            });
        }

        fn queueToolCalls(
            queuedAssistantToolText: &mut Option<String>,
            queuedToolCalls: &mut Vec<Value>,
            queuedToolCallIds: &mut Vec<String>,
            textContent: &str,
            toolCalls: &[Value],
        ) {
            appendQueuedAssistantToolText(queuedAssistantToolText, textContent);
            for toolCall in toolCalls {
                queuedToolCalls.push(toolCall.clone());
                if let Some(callId) = toolCall.get("id").and_then(Value::as_str).map(str::trim) {
                    if !callId.is_empty() {
                        queuedToolCallIds.push(callId.to_string());
                    }
                }
            }
        }

        fn emitQueuedToolCallsIfNeeded(
            messagesArray: &mut Vec<Value>,
            queuedAssistantToolText: &mut Option<String>,
            queuedToolCalls: &mut Vec<Value>,
            queuedToolCallIds: &mut Vec<String>,
            openToolCallIds: &mut Vec<String>,
        ) {
            if queuedToolCalls.is_empty() {
                return;
            }
            messagesArray.push(json!({
                "role": "assistant",
                "content": match queuedAssistantToolText {
                    Some(value) if !value.trim().is_empty() => Value::String(value.clone()),
                    _ => Value::Null,
                },
                "tool_calls": queuedToolCalls.clone(),
            }));
            openToolCallIds.extend(queuedToolCallIds.clone());
            *queuedAssistantToolText = None;
            queuedToolCalls.clear();
            queuedToolCallIds.clear();
        }

        fn flushOpenToolCallsAsCancelled(
            messagesArray: &mut Vec<Value>,
            queuedAssistantToolText: &mut Option<String>,
            queuedToolCalls: &mut Vec<Value>,
            queuedToolCallIds: &mut Vec<String>,
            openToolCallIds: &mut Vec<String>,
        ) {
            emitQueuedToolCallsIfNeeded(
                messagesArray,
                queuedAssistantToolText,
                queuedToolCalls,
                queuedToolCallIds,
                openToolCallIds,
            );
            if openToolCallIds.is_empty() {
                return;
            }
            for toolCallId in openToolCallIds.iter() {
                messagesArray.push(json!({
                    "role": "tool",
                    "tool_call_id": toolCallId,
                    "content": "User cancelled",
                }));
            }
            openToolCallIds.clear();
        }

        for turn in mergedHistory {
            let content = if !preserveThinkInHistory && turn.kind == PromptTurnKind::ASSISTANT {
                removeThinkingContent(&turn.content)
            } else {
                turn.content.clone()
            };

            match turn.kind {
                PromptTurnKind::SYSTEM => {
                    flushOpenToolCallsAsCancelled(
                        &mut messagesArray,
                        &mut queuedAssistantToolText,
                        &mut queuedToolCalls,
                        &mut queuedToolCallIds,
                        &mut openToolCallIds,
                    );
                    messagesArray.push(json!({"role": "system", "content": Self::nonEmptyContent(&content)}));
                }
                PromptTurnKind::USER | PromptTurnKind::SUMMARY => {
                    flushOpenToolCallsAsCancelled(
                        &mut messagesArray,
                        &mut queuedAssistantToolText,
                        &mut queuedToolCalls,
                        &mut queuedToolCallIds,
                        &mut openToolCallIds,
                    );
                    messagesArray.push(json!({"role": "user", "content": Self::nonEmptyContent(&content)}));
                }
                PromptTurnKind::ASSISTANT | PromptTurnKind::TOOL_CALL => {
                    let (textContent, parsedToolCalls) = Self::parseXmlToolCalls(&content);
                    let toolCalls = parsedToolCalls.map(|calls| Self::wrapPackageToolCallsWithProxy(&calls));
                    if let Some(toolCalls) = toolCalls {
                        if !toolCalls.is_empty() {
                            flushOpenToolCallsAsCancelled(
                                &mut messagesArray,
                                &mut queuedAssistantToolText,
                                &mut queuedToolCalls,
                                &mut queuedToolCallIds,
                                &mut openToolCallIds,
                            );
                            queueToolCalls(
                                &mut queuedAssistantToolText,
                                &mut queuedToolCalls,
                                &mut queuedToolCallIds,
                                &textContent,
                                &toolCalls,
                            );
                        } else {
                            flushOpenToolCallsAsCancelled(
                                &mut messagesArray,
                                &mut queuedAssistantToolText,
                                &mut queuedToolCalls,
                                &mut queuedToolCallIds,
                                &mut openToolCallIds,
                            );
                            messagesArray.push(json!({"role": "assistant", "content": Self::nonEmptyContent(&content)}));
                        }
                    } else {
                        flushOpenToolCallsAsCancelled(
                            &mut messagesArray,
                            &mut queuedAssistantToolText,
                            &mut queuedToolCalls,
                            &mut queuedToolCallIds,
                            &mut openToolCallIds,
                        );
                        messagesArray.push(json!({"role": "assistant", "content": Self::nonEmptyContent(&content)}));
                    }
                }
                PromptTurnKind::TOOL_RESULT => {
                    emitQueuedToolCallsIfNeeded(
                        &mut messagesArray,
                        &mut queuedAssistantToolText,
                        &mut queuedToolCalls,
                        &mut queuedToolCallIds,
                        &mut openToolCallIds,
                    );
                    let (textContent, toolResults) = Self::parseXmlToolResults(&content);
                    let resultsList = toolResults.unwrap_or_default();
                    if !resultsList.is_empty() && !openToolCallIds.is_empty() {
                        let validCount = resultsList.len().min(openToolCallIds.len());
                        for index in 0..validCount {
                            let result = &resultsList[index];
                            let mut toolMessage = Map::new();
                            toolMessage.insert("role".to_string(), json!("tool"));
                            toolMessage.insert("tool_call_id".to_string(), json!(openToolCallIds[index]));
                            if let Some(name) = &result.name {
                                if !name.trim().is_empty() {
                                    toolMessage.insert("name".to_string(), json!(name));
                                }
                            }
                            toolMessage.insert("content".to_string(), json!(Self::nonEmptyContent(&result.content)));
                            messagesArray.push(Value::Object(toolMessage));
                        }
                        openToolCallIds.drain(0..validCount);
                        if !textContent.trim().is_empty() {
                            messagesArray.push(json!({"role": "user", "content": textContent}));
                        }
                    } else {
                        flushOpenToolCallsAsCancelled(
                            &mut messagesArray,
                            &mut queuedAssistantToolText,
                            &mut queuedToolCalls,
                            &mut queuedToolCallIds,
                            &mut openToolCallIds,
                        );
                        messagesArray.push(json!({
                            "role": "user",
                            "content": if !textContent.trim().is_empty() {
                                textContent
                            } else {
                                Self::nonEmptyContent(&content)
                            },
                        }));
                    }
                }
            }
        }

        flushOpenToolCallsAsCancelled(
            &mut messagesArray,
            &mut queuedAssistantToolText,
            &mut queuedToolCalls,
            &mut queuedToolCallIds,
            &mut openToolCallIds,
        );
        Value::Array(messagesArray)
    }

    fn nonEmptyContent(content: &str) -> String {
        if content.trim().is_empty() {
            "[Empty]".to_string()
        } else {
            content.to_string()
        }
    }

    fn buildToolDefinitions(toolPrompts: &[ToolPrompt]) -> Value {
        Value::Array(
            toolPrompts
                .iter()
                .map(|tool| {
                    let fullDescription = if !tool.details.is_empty() {
                        format!("{}\n{}", tool.description, tool.details)
                    } else {
                        tool.description.clone()
                    };
                    json!({
                        "type": "function",
                        "function": {
                            "name": tool.name,
                            "description": fullDescription,
                            "parameters": Self::buildSchemaFromStructured(tool.parametersStructured.as_deref().unwrap_or(&[])),
                        }
                    })
                })
                .collect(),
        )
    }

    fn buildSchemaFromStructured(params: &[crate::data::model::ToolPrompt::ToolParameterSchema]) -> Value {
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

    fn convertToolCallsToXml(toolCalls: &[Value]) -> String {
        let mut xml = String::new();
        for toolCall in toolCalls {
            let Some(function) = toolCall.get("function").and_then(Value::as_object) else {
                continue;
            };
            let name = function.get("name").and_then(Value::as_str).unwrap_or("");
            if name.trim().is_empty() {
                continue;
            }
            let argumentsRaw = function.get("arguments").and_then(Value::as_str).unwrap_or("");
            let paramsObj = serde_json::from_str::<Value>(argumentsRaw)
                .ok()
                .and_then(|value| value.as_object().cloned());
            let toolTagName = ChatMarkupRegex::generate_random_tool_tag_name();
            xml.push_str(&format!(r#"<{toolTagName} name="{name}">"#));
            if let Some(paramsObj) = paramsObj {
                for (key, value) in paramsObj {
                    xml.push_str(&format!(
                        "\n<param name=\"{}\">{}</param>",
                        key,
                        Self::escapeXml(&jsonValueToText(&value))
                    ));
                }
            } else if !argumentsRaw.trim().is_empty() {
                xml.push_str(&format!(
                    "\n<param name=\"_raw_arguments\">{}</param>",
                    Self::escapeXml(argumentsRaw)
                ));
            }
            xml.push_str(&format!("\n</{toolTagName}>\n"));
        }
        xml.trim().to_string()
    }

    fn parsePossibleToolCallsFromText(content: &str) -> Option<Vec<Value>> {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return None;
        }
        let mut candidates = Vec::new();
        let mut seen = HashSet::new();
        pushCandidate(&mut candidates, &mut seen, trimmed);
        if let Some(extractedJson) = extractJson(trimmed) {
            pushCandidate(&mut candidates, &mut seen, &extractedJson);
        }
        if let Some(extractedArray) = extractJsonArray(trimmed) {
            pushCandidate(&mut candidates, &mut seen, &extractedArray);
        }
        for fenced in extractFencedJsonBlocks(trimmed) {
            pushCandidate(&mut candidates, &mut seen, &fenced);
        }
        for candidate in candidates {
            if let Ok(value) = serde_json::from_str::<Value>(&candidate) {
                match value {
                    Value::Object(object) => {
                        if let Some(calls) = Self::extractToolCallsFromObject(&object) {
                            if !calls.is_empty() {
                                return Some(calls);
                            }
                        }
                    }
                    Value::Array(array) => {
                        let normalized = Self::normalizeToolCalls(&array);
                        if !normalized.is_empty() {
                            return Some(normalized);
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    fn extractToolCallsFromObject(root: &Map<String, Value>) -> Option<Vec<Value>> {
        if let Some(Value::Array(array)) = root.get("tool_calls") {
            let normalized = Self::normalizeToolCalls(array);
            if !normalized.is_empty() {
                return Some(normalized);
            }
        }
        if let Some(Value::Object(functionCall)) = root.get("function_call") {
            if let Some(normalized) = Self::normalizeSingleToolCall(functionCall, 0) {
                return Some(vec![normalized]);
            }
        }
        if root.get("type").and_then(Value::as_str) == Some("function_call") {
            if let Some(normalized) = Self::normalizeSingleToolCall(root, 0) {
                return Some(vec![normalized]);
            }
        }
        if let Some(Value::Array(outputArray)) = root.get("output") {
            let normalized = Self::normalizeToolCalls(outputArray);
            if !normalized.is_empty() {
                return Some(normalized);
            }
        }
        None
    }

    fn normalizeToolCalls(source: &[Value]) -> Vec<Value> {
        source
            .iter()
            .enumerate()
            .filter_map(|(index, item)| item.as_object().and_then(|object| Self::normalizeSingleToolCall(object, index)))
            .collect()
    }

    fn normalizeSingleToolCall(raw: &Map<String, Value>, index: usize) -> Option<Value> {
        let functionObject = raw.get("function").and_then(Value::as_object);
        let functionCallObject = raw.get("function_call").and_then(Value::as_object);
        let name = functionObject
            .and_then(|object| object.get("name").and_then(Value::as_str))
            .or_else(|| raw.get("name").and_then(Value::as_str).filter(|value| !value.trim().is_empty()))
            .or_else(|| functionCallObject.and_then(|object| object.get("name").and_then(Value::as_str)))
            .unwrap_or("");
        if name.trim().is_empty() {
            return None;
        }
        let argumentsValue = functionObject
            .and_then(|object| object.get("arguments"))
            .or_else(|| raw.get("arguments"))
            .or_else(|| functionCallObject.and_then(|object| object.get("arguments")));
        let arguments = match argumentsValue {
            Some(Value::Object(_)) | Some(Value::Array(_)) => argumentsValue.unwrap().to_string(),
            Some(Value::String(value)) if value.trim().is_empty() => "{}".to_string(),
            Some(Value::String(value)) => value.clone(),
            None => "{}".to_string(),
            Some(value) => value.to_string(),
        };
        let rawId = raw
            .get("id")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .or_else(|| raw.get("call_id").and_then(Value::as_str).filter(|value| !value.trim().is_empty()))
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("call_{}_{}", Self::sanitizeToolCallId(name), index));
        let callId = Self::sanitizeToolCallId(&rawId);
        Some(json!({
            "id": callId,
            "type": "function",
            "function": {
                "name": name,
                "arguments": arguments,
            }
        }))
    }

    fn parseXmlToolCalls(content: &str) -> (String, Option<Vec<Value>>) {
        let matches = ChatMarkupRegex::tool_call_matches(content);
        if matches.is_empty() {
            return (content.to_string(), None);
        }
        let mut toolCalls = Vec::new();
        let mut textContent = content.to_string();
        for (callIndex, toolMatch) in matches.iter().enumerate() {
            let mut params = Map::new();
            for (start, end) in tag_ranges(&toolMatch.body, "param") {
                let raw = &toolMatch.body[start..end];
                let paramName = attr_value(raw, "name").unwrap_or_default();
                let paramValue = raw
                    .split_once('>')
                    .and_then(|(_, tail)| tail.rsplit_once("</").map(|(body, _)| body))
                    .map(Self::xmlUnescape)
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                params.insert(paramName, json!(paramValue));
            }
            let toolNamePart = Self::sanitizeToolCallId(&toolMatch.name);
            let hashPart = Self::stableIdHashPart(&format!("{}:{}", toolMatch.name, Value::Object(params.clone())));
            let callId = Self::sanitizeToolCallId(&format!("call_{}_{}_{}", toolNamePart, hashPart, callIndex));
            toolCalls.push(json!({
                "id": callId,
                "type": "function",
                "function": {
                    "name": toolMatch.name,
                    "arguments": Value::Object(params).to_string(),
                }
            }));
            textContent = textContent.replace(&toolMatch.rawText(), "");
        }
        (textContent.trim().to_string(), Some(toolCalls))
    }

    fn wrapPackageToolCallsWithProxy(toolCalls: &[Value]) -> Vec<Value> {
        let mut wrappedToolCalls = Vec::new();
        for toolCall in toolCalls {
            let Some(function) = toolCall.get("function").and_then(Value::as_object) else {
                wrappedToolCalls.push(toolCall.clone());
                continue;
            };
            let toolName = function.get("name").and_then(Value::as_str).unwrap_or("");
            if !toolName.contains(':') || toolName == "package_proxy" {
                wrappedToolCalls.push(toolCall.clone());
                continue;
            }
            let rawArguments = function.get("arguments").and_then(Value::as_str).unwrap_or("{}");
            let originalArguments = serde_json::from_str::<Value>(if rawArguments.trim().is_empty() {
                "{}"
            } else {
                rawArguments
            })
            .unwrap_or_else(|_| json!({}));
            let mut wrapped = toolCall.clone();
            if let Some(object) = wrapped.as_object_mut() {
                object.insert(
                    "function".to_string(),
                    json!({
                        "name": "package_proxy",
                        "arguments": json!({
                            "tool_name": toolName,
                            "params": originalArguments,
                        }).to_string(),
                    }),
                );
            }
            wrappedToolCalls.push(wrapped);
        }
        wrappedToolCalls
    }

    fn parseXmlToolResults(content: &str) -> (String, Option<Vec<ToolResultRecord>>) {
        let matches = ChatMarkupRegex::tool_result_blocks(content);
        if matches.is_empty() {
            return (content.to_string(), None);
        }
        let mut results = Vec::new();
        let mut textContent = content.to_string();
        for block in matches {
            let fullContent = block.body.trim().to_string();
            let resultContent = extractContentTag(&fullContent).unwrap_or(fullContent);
            let resultName = attr_value(&block.raw, "name");
            results.push(ToolResultRecord {
                name: resultName,
                content: resultContent.trim().to_string(),
            });
            textContent = textContent.replace(&block.raw, "").trim().to_string();
        }
        (textContent.trim().to_string(), Some(results))
    }

    fn escapeXml(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    fn xmlUnescape(text: &str) -> String {
        text.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&")
    }

    fn sanitizeToolCallId(raw: &str) -> String {
        let mut output = String::new();
        let mut previousUnderscore = false;
        for ch in raw.chars() {
            let next = if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            };
            if next == '_' {
                if !previousUnderscore {
                    output.push(next);
                }
                previousUnderscore = true;
            } else {
                output.push(next);
                previousUnderscore = false;
            }
        }
        let output = output.trim_matches('_').to_string();
        if output.is_empty() {
            "call".to_string()
        } else {
            output
        }
    }

    fn stableIdHashPart(raw: &str) -> String {
        let mut hash: i32 = 0;
        for unit in raw.encode_utf16() {
            hash = hash.wrapping_mul(31).wrapping_add(unit as i32);
        }
        let positive = if hash == i32::MIN { 0 } else { hash.abs() };
        let base = toBase36(positive as u32)
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        if base.is_empty() {
            "0".to_string()
        } else {
            base
        }
    }
}

trait ToolCallMatchRawText {
    fn rawText(&self) -> String;
}

impl ToolCallMatchRawText for crate::util::ChatMarkupRegex::ToolCallMatch {
    fn rawText(&self) -> String {
        format!(
            "<{} name=\"{}\">{}</{}>",
            self.tag_name, self.name, self.body, self.tag_name
        )
    }
}

fn jsonValueToText(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn removeThinkingContent(content: &str) -> String {
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

fn pushCandidate(candidates: &mut Vec<String>, seen: &mut HashSet<String>, value: &str) {
    let trimmed = value.trim();
    if !trimmed.is_empty() && seen.insert(trimmed.to_string()) {
        candidates.push(trimmed.to_string());
    }
}

fn extractJson(content: &str) -> Option<String> {
    extractBalanced(content, '{', '}')
}

fn extractJsonArray(content: &str) -> Option<String> {
    extractBalanced(content, '[', ']')
}

fn extractBalanced(content: &str, open: char, close: char) -> Option<String> {
    let start = content.find(open)?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (relative_index, ch) in content[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
        } else if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(content[start..start + relative_index + ch.len_utf8()].to_string());
            }
        }
    }
    None
}

fn extractFencedJsonBlocks(content: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut cursor = 0;
    while let Some(start_relative) = content[cursor..].find("```") {
        let start = cursor + start_relative + 3;
        let after_language = content[start..]
            .find('\n')
            .map(|index| start + index + 1)
            .unwrap_or(start);
        let Some(end_relative) = content[after_language..].find("```") else {
            break;
        };
        let end = after_language + end_relative;
        let fenced = content[after_language..end].trim();
        if !fenced.is_empty() {
            blocks.push(fenced.to_string());
        }
        cursor = end + 3;
    }
    blocks
}

fn extractContentTag(content: &str) -> Option<String> {
    let (start, end) = tag_ranges(content, "content").into_iter().next()?;
    let raw = &content[start..end];
    raw.split_once('>')
        .and_then(|(_, tail)| tail.rsplit_once("</").map(|(body, _)| body.to_string()))
}

fn toBase36(mut value: u32) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let mut chars = Vec::new();
    while value > 0 {
        let digit = value % 36;
        chars.push(std::char::from_digit(digit, 36).unwrap_or('0'));
        value /= 36;
    }
    chars.iter().rev().collect()
}
