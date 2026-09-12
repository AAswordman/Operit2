use std::collections::HashSet;

use serde_json::{json, Map, Value};

use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_model::ToolPrompt::ToolPrompt;
use operit_util::ChatMarkupRegex::{attr_value, tag_ranges, ChatMarkupRegex};

const PACKAGE_PROXY_TOOL_NAME: &str = "package_proxy";
const CLI_PROXY_TOOL_NAME: &str = "proxy";
const PROXY_TARGET_TOOL_NAME_PARAM: &str = "tool_name";

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OpenToolCall {
    pub(crate) id: String,
    pub(crate) matching_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MatchedToolCall {
    pub(crate) result_index: usize,
    pub(crate) call: OpenToolCall,
}

pub struct StructuredToolCallBridge;

impl StructuredToolCallBridge {
    /// Returns the executable tool name represented by a provider tool call.
    pub(crate) fn toolCallName(toolCall: &Value) -> String {
        let function = toolCall
            .get("function")
            .and_then(Value::as_object)
            .or_else(|| toolCall.as_object());
        let Some(function) = function else {
            return String::new();
        };
        let name = function
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if name != PACKAGE_PROXY_TOOL_NAME && name != CLI_PROXY_TOOL_NAME {
            return name;
        }
        Self::proxyTargetToolName(function)
            .filter(|target| !target.trim().is_empty())
            .unwrap_or(name)
    }

    /// Reads the target tool name carried inside one proxy tool-call envelope.
    fn proxyTargetToolName(function: &Map<String, Value>) -> Option<String> {
        for key in ["arguments", "args", "input"] {
            let Some(raw) = function.get(key) else {
                continue;
            };
            let arguments = match raw {
                Value::Object(object) => object.clone(),
                Value::String(text) => serde_json::from_str::<Value>(text)
                    .ok()
                    .and_then(|value| value.as_object().cloned())?,
                _ => return None,
            };
            return arguments
                .get(PROXY_TARGET_TOOL_NAME_PARAM)
                .and_then(Value::as_str)
                .map(|value| value.trim().to_string());
        }
        None
    }

    /// Consumes open tool calls whose executable names match result names.
    pub(crate) fn consumeMatchingToolCalls(
        openToolCalls: &mut Vec<OpenToolCall>,
        resultToolNames: &[Option<String>],
    ) -> Vec<MatchedToolCall> {
        let mut matched = Vec::new();
        for (result_index, result_name) in resultToolNames.iter().enumerate() {
            let normalized = result_name.as_deref().unwrap_or("").trim();
            if normalized.is_empty() {
                continue;
            }
            let Some(call_index) = openToolCalls
                .iter()
                .position(|call| call.matching_name == normalized)
            else {
                continue;
            };
            matched.push(MatchedToolCall {
                result_index,
                call: openToolCalls.remove(call_index),
            });
        }
        matched
    }

    /// Builds a protocol-valid result body for a tool call that did not receive a result.
    pub(crate) fn unmatchedToolResultContent(reason: &str, toolName: &str) -> String {
        let toolLabel = {
            let trimmed = toolName.trim();
            if trimmed.is_empty() {
                "未知工具"
            } else {
                trimmed
            }
        };
        let detail = match reason {
            "tool_result_partial_batch" | "tool_result_without_structured_match" => {
                "没有匹配到执行结果"
            }
            "typed_tool_call_without_payload" => "调用没有可执行参数",
            "tool_call_api_disabled" => "工具调用协议已关闭",
            "user_boundary"
            | "system_boundary"
            | "assistant_boundary"
            | "assistant_tool_call_before_result"
            | "typed_tool_call_before_result"
            | "typed_function_call_before_result"
            | "typed_tool_use_before_result"
            | "assistant_function_call_before_result"
            | "assistant_tool_use_before_result"
            | "history_end" => "后续对话历史已到达，但未返回执行结果",
            _ => "调用在未返回执行结果时结束",
        };
        format!("工具结果缺失：{toolLabel} {detail}。这不是用户取消。")
    }

    pub fn buildToolsJson(toolPrompts: Option<&[ToolPrompt]>) -> Option<String> {
        let toolPrompts = toolPrompts?;
        if toolPrompts.is_empty() {
            return None;
        }
        let tools = Self::buildToolDefinitions(toolPrompts);
        tools
            .as_array()
            .filter(|items| !items.is_empty())
            .map(|_| tools.to_string())
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

    pub fn buildMessagesJsonForProvider(
        history: &[PromptTurn],
        preserveThinkInHistory: bool,
        useToolCall: bool,
    ) -> String {
        if useToolCall {
            Self::buildStructuredMessages(history, preserveThinkInHistory).to_string()
        } else {
            Self::buildXmlModeMessages(history, preserveThinkInHistory).to_string()
        }
    }

    pub fn buildMnnChatHistory(
        history: &[PromptTurn],
        preserveThinkInHistory: bool,
    ) -> Vec<(String, String)> {
        let messages = Self::buildStructuredMessages(history, preserveThinkInHistory);
        let mut compiledHistory = Vec::new();
        let Some(messages) = messages.as_array() else {
            return compiledHistory;
        };
        for message in messages {
            let Some(message) = message.as_object() else {
                continue;
            };
            let role = message
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
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
                && contentValue
                    .map(|value| value.is_null() || value.is_string())
                    .unwrap_or(true);
            if isPlainRoleContentMessage {
                compiledHistory.push((role, content));
            } else {
                compiledHistory.push((
                    "json".to_string(),
                    Value::Object(message.clone()).to_string(),
                ));
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
                flushCurrentBlock(
                    compiled,
                    currentBlockType,
                    currentContent,
                    currentMetadata,
                    useToolCall,
                );
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

    pub fn build_messages_json_for_provider(
        history: &[PromptTurn],
        preserve_think_in_history: bool,
        use_tool_call: bool,
    ) -> String {
        Self::buildMessagesJsonForProvider(history, preserve_think_in_history, use_tool_call)
    }

    pub fn compile_history_for_provider(
        history: &[PromptTurn],
        use_tool_call: bool,
    ) -> Vec<PromptTurn> {
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
        let mut queuedOpenToolCalls = Vec::new();
        let mut openToolCalls = Vec::new();
        let mut nextToolCallOrdinal = 0usize;

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
            queuedOpenToolCalls: &mut Vec<OpenToolCall>,
            nextToolCallOrdinal: &mut usize,
            textContent: &str,
            toolCalls: &[Value],
        ) {
            appendQueuedAssistantToolText(queuedAssistantToolText, textContent);
            for toolCall in toolCalls {
                let callId = StructuredToolCallBridge::generatedToolCallId(*nextToolCallOrdinal);
                *nextToolCallOrdinal += 1;
                let mut clonedToolCall = toolCall.clone();
                if let Some(object) = clonedToolCall.as_object_mut() {
                    object.insert("id".to_string(), json!(callId.clone()));
                }
                let matchingName = StructuredToolCallBridge::toolCallName(&clonedToolCall);
                queuedToolCalls.push(clonedToolCall);
                queuedOpenToolCalls.push(OpenToolCall {
                    id: callId,
                    matching_name: matchingName,
                });
            }
        }

        fn emitQueuedToolCallsIfNeeded(
            messagesArray: &mut Vec<Value>,
            queuedAssistantToolText: &mut Option<String>,
            queuedToolCalls: &mut Vec<Value>,
            queuedOpenToolCalls: &mut Vec<OpenToolCall>,
            openToolCalls: &mut Vec<OpenToolCall>,
        ) {
            if queuedToolCalls.is_empty() {
                return;
            }
            let mut message = Map::new();
            message.insert("role".to_string(), json!("assistant"));
            if let Some(value) = queuedAssistantToolText
                .as_ref()
                .filter(|value| !value.trim().is_empty())
            {
                message.insert("content".to_string(), json!(value));
            }
            message.insert(
                "tool_calls".to_string(),
                Value::Array(queuedToolCalls.clone()),
            );
            messagesArray.push(Value::Object(message));
            openToolCalls.extend(queuedOpenToolCalls.clone());
            *queuedAssistantToolText = None;
            queuedToolCalls.clear();
            queuedOpenToolCalls.clear();
        }

        fn flushOpenToolCallsAsUnmatched(
            messagesArray: &mut Vec<Value>,
            queuedAssistantToolText: &mut Option<String>,
            queuedToolCalls: &mut Vec<Value>,
            queuedOpenToolCalls: &mut Vec<OpenToolCall>,
            openToolCalls: &mut Vec<OpenToolCall>,
            reason: &str,
        ) {
            emitQueuedToolCallsIfNeeded(
                messagesArray,
                queuedAssistantToolText,
                queuedToolCalls,
                queuedOpenToolCalls,
                openToolCalls,
            );
            if openToolCalls.is_empty() {
                return;
            }
            for openToolCall in openToolCalls.iter() {
                messagesArray.push(json!({
                    "role": "tool",
                    "tool_call_id": openToolCall.id,
                    "content": StructuredToolCallBridge::unmatchedToolResultContent(
                        reason,
                        &openToolCall.matching_name,
                    ),
                }));
            }
            openToolCalls.clear();
        }

        for turn in mergedHistory {
            let content = if !preserveThinkInHistory && turn.kind == PromptTurnKind::ASSISTANT {
                removeThinkingContent(&turn.content)
            } else {
                turn.content.clone()
            };

            match turn.kind {
                PromptTurnKind::SYSTEM => {
                    flushOpenToolCallsAsUnmatched(
                        &mut messagesArray,
                        &mut queuedAssistantToolText,
                        &mut queuedToolCalls,
                        &mut queuedOpenToolCalls,
                        &mut openToolCalls,
                        "system_boundary",
                    );
                    messagesArray.push(
                        json!({"role": "system", "content": Self::nonEmptyContent(&content)}),
                    );
                }
                PromptTurnKind::USER | PromptTurnKind::SUMMARY => {
                    flushOpenToolCallsAsUnmatched(
                        &mut messagesArray,
                        &mut queuedAssistantToolText,
                        &mut queuedToolCalls,
                        &mut queuedOpenToolCalls,
                        &mut openToolCalls,
                        "user_boundary",
                    );
                    messagesArray
                        .push(json!({"role": "user", "content": Self::nonEmptyContent(&content)}));
                }
                PromptTurnKind::ASSISTANT => {
                    let (textContent, parsedToolCalls) = Self::parseXmlToolCalls(&content);
                    let toolCalls =
                        parsedToolCalls.map(|calls| Self::wrapPackageToolCallsWithProxy(&calls));
                    if let Some(toolCalls) = toolCalls {
                        if !toolCalls.is_empty() {
                            flushOpenToolCallsAsUnmatched(
                                &mut messagesArray,
                                &mut queuedAssistantToolText,
                                &mut queuedToolCalls,
                                &mut queuedOpenToolCalls,
                                &mut openToolCalls,
                                "assistant_tool_call_before_result",
                            );
                            queueToolCalls(
                                &mut queuedAssistantToolText,
                                &mut queuedToolCalls,
                                &mut queuedOpenToolCalls,
                                &mut nextToolCallOrdinal,
                                &textContent,
                                &toolCalls,
                            );
                        } else {
                            flushOpenToolCallsAsUnmatched(
                                &mut messagesArray,
                                &mut queuedAssistantToolText,
                                &mut queuedToolCalls,
                                &mut queuedOpenToolCalls,
                                &mut openToolCalls,
                                "assistant_boundary",
                            );
                            messagesArray.push(json!({"role": "assistant", "content": Self::nonEmptyContent(&content)}));
                        }
                    } else {
                        flushOpenToolCallsAsUnmatched(
                            &mut messagesArray,
                            &mut queuedAssistantToolText,
                            &mut queuedToolCalls,
                            &mut queuedOpenToolCalls,
                            &mut openToolCalls,
                            "assistant_boundary",
                        );
                        messagesArray.push(json!({"role": "assistant", "content": Self::nonEmptyContent(&content)}));
                    }
                }
                PromptTurnKind::TOOL_CALL => {
                    let (textContent, parsedToolCalls) = Self::parseXmlToolCalls(&content);
                    let toolCalls =
                        parsedToolCalls.map(|calls| Self::wrapPackageToolCallsWithProxy(&calls));
                    if let Some(toolCalls) = toolCalls {
                        if !toolCalls.is_empty() {
                            flushOpenToolCallsAsUnmatched(
                                &mut messagesArray,
                                &mut queuedAssistantToolText,
                                &mut queuedToolCalls,
                                &mut queuedOpenToolCalls,
                                &mut openToolCalls,
                                "typed_tool_call_before_result",
                            );
                            queueToolCalls(
                                &mut queuedAssistantToolText,
                                &mut queuedToolCalls,
                                &mut queuedOpenToolCalls,
                                &mut nextToolCallOrdinal,
                                &textContent,
                                &toolCalls,
                            );
                        } else {
                            flushOpenToolCallsAsUnmatched(
                                &mut messagesArray,
                                &mut queuedAssistantToolText,
                                &mut queuedToolCalls,
                                &mut queuedOpenToolCalls,
                                &mut openToolCalls,
                                "typed_tool_call_without_payload",
                            );
                            messagesArray.push(json!({"role": "assistant", "content": Self::nonEmptyContent(&content)}));
                        }
                    } else {
                        flushOpenToolCallsAsUnmatched(
                            &mut messagesArray,
                            &mut queuedAssistantToolText,
                            &mut queuedToolCalls,
                            &mut queuedOpenToolCalls,
                            &mut openToolCalls,
                            "typed_tool_call_without_payload",
                        );
                        messagesArray.push(json!({"role": "assistant", "content": Self::nonEmptyContent(&content)}));
                    }
                }
                PromptTurnKind::TOOL_RESULT => {
                    emitQueuedToolCallsIfNeeded(
                        &mut messagesArray,
                        &mut queuedAssistantToolText,
                        &mut queuedToolCalls,
                        &mut queuedOpenToolCalls,
                        &mut openToolCalls,
                    );
                    let (textContent, toolResults) = Self::parseXmlToolResults(&content);
                    let resultsList = toolResults.unwrap_or_default();
                    if !resultsList.is_empty() && !openToolCalls.is_empty() {
                        let resultToolNames = resultsList
                            .iter()
                            .map(|result| result.name.clone())
                            .collect::<Vec<_>>();
                        let matchedCalls =
                            Self::consumeMatchingToolCalls(&mut openToolCalls, &resultToolNames);
                        for matchedCall in matchedCalls {
                            let result = &resultsList[matchedCall.result_index];
                            let mut toolMessage = Map::new();
                            toolMessage.insert("role".to_string(), json!("tool"));
                            toolMessage
                                .insert("tool_call_id".to_string(), json!(matchedCall.call.id));
                            if let Some(name) = &result.name {
                                if !name.trim().is_empty() {
                                    toolMessage.insert("name".to_string(), json!(name));
                                }
                            }
                            toolMessage.insert(
                                "content".to_string(),
                                json!(Self::nonEmptyContent(&result.content)),
                            );
                            messagesArray.push(Value::Object(toolMessage));
                        }
                        flushOpenToolCallsAsUnmatched(
                            &mut messagesArray,
                            &mut queuedAssistantToolText,
                            &mut queuedToolCalls,
                            &mut queuedOpenToolCalls,
                            &mut openToolCalls,
                            "tool_result_partial_batch",
                        );
                        if !textContent.trim().is_empty() {
                            messagesArray.push(json!({"role": "user", "content": textContent}));
                        }
                    } else {
                        flushOpenToolCallsAsUnmatched(
                            &mut messagesArray,
                            &mut queuedAssistantToolText,
                            &mut queuedToolCalls,
                            &mut queuedOpenToolCalls,
                            &mut openToolCalls,
                            "tool_result_without_structured_match",
                        );
                        if !textContent.trim().is_empty() {
                            messagesArray.push(json!({"role": "user", "content": textContent}));
                        }
                    }
                }
            }
        }

        flushOpenToolCallsAsUnmatched(
            &mut messagesArray,
            &mut queuedAssistantToolText,
            &mut queuedToolCalls,
            &mut queuedOpenToolCalls,
            &mut openToolCalls,
            "history_end",
        );
        Value::Array(messagesArray)
    }

    fn buildXmlModeMessages(history: &[PromptTurn], preserveThinkInHistory: bool) -> Value {
        let mergedHistory = Self::compileHistoryForProvider(history, false);
        let messagesArray = mergedHistory
            .into_iter()
            .map(|turn| {
                let kind = turn.kind.clone();
                let role = match kind {
                    PromptTurnKind::SYSTEM => "system",
                    PromptTurnKind::USER
                    | PromptTurnKind::SUMMARY
                    | PromptTurnKind::TOOL_RESULT => "user",
                    PromptTurnKind::ASSISTANT | PromptTurnKind::TOOL_CALL => "assistant",
                };
                let content = if !preserveThinkInHistory && turn.kind == PromptTurnKind::ASSISTANT {
                    removeThinkingContent(&turn.content)
                } else {
                    turn.content
                };
                let effectiveContent = if role == "assistant" && content.trim().is_empty() {
                    "[Empty]".to_string()
                } else {
                    content
                };
                json!({
                    "role": role,
                    "content": effectiveContent,
                })
            })
            .collect::<Vec<_>>();
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

    fn buildSchemaFromStructured(
        params: &[operit_model::ToolPrompt::ToolParameterSchema],
    ) -> Value {
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
        schema.insert("required".to_string(), Value::Array(required));
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
            let argumentsRaw = function
                .get("arguments")
                .and_then(Value::as_str)
                .unwrap_or("");
            let paramsObj = serde_json::from_str::<Value>(argumentsRaw)
                .ok()
                .and_then(|value| value.as_object().cloned());
            let toolTagName = ChatMarkupRegex::generate_random_tool_tag_name();
            xml.push_str(&format!("\n<{toolTagName} name=\"{name}\">"));
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
        xml.trim_end().to_string()
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
            .filter_map(|(index, item)| {
                item.as_object()
                    .and_then(|object| Self::normalizeSingleToolCall(object, index))
            })
            .collect()
    }

    fn normalizeSingleToolCall(raw: &Map<String, Value>, index: usize) -> Option<Value> {
        let functionObject = raw.get("function").and_then(Value::as_object);
        let functionCallObject = raw.get("function_call").and_then(Value::as_object);
        let name = functionObject
            .and_then(|object| object.get("name").and_then(Value::as_str))
            .or_else(|| {
                raw.get("name")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
            })
            .or_else(|| {
                functionCallObject.and_then(|object| object.get("name").and_then(Value::as_str))
            })
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
            .or_else(|| {
                raw.get("call_id")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
            })
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
        let mut textContent = String::with_capacity(content.len());
        let mut cursor = 0usize;
        for (callIndex, toolMatch) in matches.iter().enumerate() {
            textContent.push_str(&content[cursor..toolMatch.start]);
            cursor = toolMatch.end;
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
            let hashPart = Self::stableIdHashPart(&format!(
                "{}:{}",
                toolMatch.name,
                Value::Object(params.clone())
            ));
            let callId = Self::sanitizeToolCallId(&format!(
                "call_{}_{}_{}",
                toolNamePart, hashPart, callIndex
            ));
            toolCalls.push(json!({
                "id": callId,
                "type": "function",
                "function": {
                    "name": toolMatch.name,
                    "arguments": Value::Object(params).to_string(),
                }
            }));
        }
        textContent.push_str(&content[cursor..]);
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
            let rawArguments = function
                .get("arguments")
                .and_then(Value::as_str)
                .unwrap_or("{}");
            let originalArguments =
                serde_json::from_str::<Value>(if rawArguments.trim().is_empty() {
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

    fn generatedToolCallId(ordinal: usize) -> String {
        Self::sanitizeShortToolCallId(&format!(
            "{}_{}",
            Self::stableIdHashPart(&format!("tool_call:{ordinal}")),
            ordinal
        ))
    }

    fn sanitizeShortToolCallId(raw: &str) -> String {
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
        let filler = Self::stableIdHashPart(raw);
        format!("{cleaned}{filler}000000000")
            .chars()
            .take(9)
            .collect()
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

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::StructuredToolCallBridge;
    use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};

    /// Verifies canonical stored tool markup becomes only a native provider tool call.
    #[test]
    fn nativeHistoryRemovesCanonicalToolMarkupWithCallId() {
        let history = vec![PromptTurn::new(
            PromptTurnKind::ASSISTANT,
            concat!(
                "Opening page\n",
                "<tool name=\"package_proxy\" call_id=\"part-1\">",
                "<param name=\"params\">{\"url\":\"https://example.com\"}</param>",
                "<param name=\"tool_name\">browser:goto</param>",
                "</tool>"
            ),
        )];

        let messages: Value = serde_json::from_str(
            &StructuredToolCallBridge::buildMessagesJsonForProvider(&history, true, true),
        )
        .expect("native provider messages must be valid JSON");
        let assistant = &messages
            .as_array()
            .expect("provider messages must be an array")[0];

        assert_eq!(
            assistant.get("content"),
            Some(&Value::String("Opening page".to_string()))
        );
        let toolCalls = assistant
            .get("tool_calls")
            .and_then(Value::as_array)
            .expect("canonical tool markup must become native tool calls");
        assert_eq!(toolCalls.len(), 1);
        assert_eq!(
            toolCalls[0]
                .pointer("/function/name")
                .and_then(Value::as_str),
            Some("package_proxy")
        );
    }

    /// Verifies package proxy results match the proxied executable tool name.
    #[test]
    fn packageProxyResultMatchesTargetToolName() {
        let history = vec![
            PromptTurn::new(
                PromptTurnKind::ASSISTANT,
                concat!(
                    r#"<tool name="package_proxy">"#,
                    r#"<param name="tool_name">daily_life:get_current_date</param>"#,
                    r#"<param name="params">{}</param>"#,
                    "</tool>"
                ),
            ),
            PromptTurn::new(
                PromptTurnKind::TOOL_RESULT,
                concat!(
                    r#"<tool_result name="daily_life:get_current_date" status="success">"#,
                    r#"<content>{"local":"09/11/2026, 10:10:34 PM"}</content>"#,
                    "</tool_result>"
                ),
            ),
        ];

        let messages: Value = serde_json::from_str(
            &StructuredToolCallBridge::buildMessagesJsonForProvider(&history, true, true),
        )
        .expect("native provider messages must be valid JSON");
        let messages = messages
            .as_array()
            .expect("provider messages must be an array");
        assert_eq!(messages.len(), 2);

        let call_id = messages[0]
            .pointer("/tool_calls/0/id")
            .and_then(Value::as_str)
            .expect("package proxy call must have an id");
        assert_eq!(
            messages[0]
                .pointer("/tool_calls/0/function/name")
                .and_then(Value::as_str),
            Some("package_proxy")
        );
        assert!(!messages[0]
            .as_object()
            .expect("assistant tool-call message must be an object")
            .contains_key("content"));
        assert_eq!(
            messages[1].get("tool_call_id").and_then(Value::as_str),
            Some(call_id)
        );
        assert_eq!(
            messages[1].get("name").and_then(Value::as_str),
            Some("daily_life:get_current_date")
        );
        assert!(!messages[1]
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("工具结果缺失"));
    }
}
