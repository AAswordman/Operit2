use operit_host_api::TimeUtils::currentTimeMillis;
use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use crate::runtime_support::{
    RuntimeChatCallRequest, RuntimeChatSendRequest, RuntimeChatSlot, ToolRuntimeSupport,
};
use crate::tools::ToolResultDataClasses::{
    stringResultData, AgentStatusResultData, ChatCallResultData, ChatCallTurnData,
    ChatCreationResultData, ChatDeleteResultData, ChatFindResultData, ChatInfo, ChatListResultData,
    ChatMessageInfo, ChatMessagesResultData, ChatServiceStartResultData, ChatSwitchResultData,
    ChatTitleUpdateResultData, JsNullable, JsOptional, MessageSendResultData, ToolResultData,
};
use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::{
    AITool, AsyncToolExecutor, ToolAccessSpec, ToolBoundary, ToolEffect, ToolInvocationFuture,
    ToolValidationResult,
};
use operit_model::ChatHistory::ChatHistory;
use operit_model::ChatTurnOptions::{ChatTurnContinuation, ChatTurnOptions};
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_model::FunctionType::FunctionType;
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_store::repository::ChatHistoryManager::ChatHistoryManager;
use serde_json::{json, Value};

#[derive(Clone)]
/// Defines built-in chat management tool names and runtime holder state.
pub struct StandardChatManagerTool {
    runtimeSupport: Arc<dyn ToolRuntimeSupport>,
}

#[derive(Clone, Copy)]
/// Operations supported by the standard chat manager tool.
pub enum ChatManagerToolOperation {
    StartChatService,
    StopChatService,
    CreateNewChat,
    ListChats,
    FindChat,
    AgentStatus,
    SwitchChat,
    UpdateChatTitle,
    UpdateChatGroup,
    UpdateChatPinned,
    UpdateChatLocked,
    ReorderChats,
    DeleteChat,
    SendMessageToAi,
    SendMessageToAiStreaming,
    CallChatModel,
    GetChatMessages,
    GetChatMessagesRange,
}

#[derive(Clone)]
/// Dispatches chat-management tool calls to runtime chat services.
pub struct ChatManagerToolExecutor {
    pub tools: StandardChatManagerTool,
    pub operation: ChatManagerToolOperation,
}

impl StandardChatManagerTool {
    /// Creates a chat manager tool set bound to one tool runtime.
    pub fn new(runtimeSupport: Arc<dyn ToolRuntimeSupport>) -> Self {
        Self { runtimeSupport }
    }

    #[allow(non_snake_case)]
    /// Starts chat service processing for the main and floating runtime slots.
    pub fn startChatService(&self, tool: &AITool) -> ToolResult {
        match self.runtimeSupport.startChatServices() {
            Ok(()) => successData(
                tool,
                ToolResultData::ChatServiceStartResultData(ChatServiceStartResultData {
                    isConnected: true,
                    connectionTime: currentTimeMillis(),
                }),
            ),
            Err(error) => toolError(tool, error),
        }
    }

    #[allow(non_snake_case)]
    /// Stops chat service processing by clearing active runtime cores.
    pub fn stopChatService(&self, tool: &AITool) -> ToolResult {
        match self.runtimeSupport.stopChatServices() {
            Ok(()) => successData(
                tool,
                ToolResultData::ChatServiceStartResultData(ChatServiceStartResultData {
                    isConnected: false,
                    connectionTime: currentTimeMillis(),
                }),
            ),
            Err(error) => toolError(tool, error),
        }
    }

    /// Creates a workspace conversation using an explicit source and opaque creation input.
    #[allow(non_snake_case)]
    pub async fn createNewChat(&self, tool: &AITool) -> ToolResult {
        if let Err(error) =
            validateChatParameterNames(tool, &["set_as_current_chat", "source_chat_id", "input"])
        {
            return toolError(tool, error);
        }
        let sourceChatId = match optionalParameterValue(tool, "source_chat_id") {
            None => None,
            Some(value) if value == "null" => None,
            Some(value) if value.trim().is_empty() => {
                return toolError(tool, "source_chat_id must not be blank".to_string())
            }
            Some(value) => Some(value),
        };
        let input = match optionalParameterValue(tool, "input") {
            None => None,
            Some(value) => match serde_json::from_str::<Value>(&value) {
                Ok(Value::Null) => None,
                Ok(value @ Value::Object(_)) => Some(value),
                Ok(_) => return toolError(tool, "input must be a JSON object or null".to_string()),
                Err(error) => return toolError(tool, error.to_string()),
            },
        };
        let setAsCurrentChat = match parseOptionalBoolean(tool, "set_as_current_chat") {
            Ok(Some(value)) => value,
            Ok(None) => true,
            Err(error) => return toolError(tool, error),
        };
        match self
            .runtimeSupport
            .createChatRuntime(setAsCurrentChat, sourceChatId, input)
            .await
        {
            Ok(chatId) => successData(
                tool,
                ToolResultData::ChatCreationResultData(ChatCreationResultData {
                    chatId,
                    createdAt: currentTimeMillis(),
                }),
            ),
            Err(error) => toolError(tool, error),
        }
    }

    #[allow(non_snake_case)]
    /// Lists stored chat histories with the requested filters.
    pub fn listChats(&self, tool: &AITool) -> ToolResult {
        match buildFilteredChatList(tool) {
            Ok((totalCount, currentChatId, chats)) => successData(
                tool,
                ToolResultData::ChatListResultData(ChatListResultData {
                    totalCount,
                    currentChatId: JsNullable::from_option(currentChatId),
                    chats,
                }),
            ),
            Err(error) => toolError(tool, error),
        }
    }

    #[allow(non_snake_case)]
    /// Finds a chat by matching the requested query and match mode.
    pub fn findChat(&self, tool: &AITool) -> ToolResult {
        let query = parameterValue(tool, "query");
        if query.trim().is_empty() {
            return toolError(tool, "Invalid parameter: missing query".to_string());
        }
        let matchMode = match parseMatchMode(tool) {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };
        let targetIndex = match optionalParameterValue(tool, "index") {
            Some(value) if !value.trim().is_empty() => match value.parse::<usize>() {
                Ok(index) => index,
                Err(_) => {
                    return toolError(
                        tool,
                        "Invalid parameter: index must be an integer".to_string(),
                    )
                }
            },
            _ => 0,
        };
        let manager = match ChatHistoryManager::default() {
            Ok(manager) => manager,
            Err(error) => return toolError(tool, format!("Error opening chat history: {error}")),
        };
        let histories = match manager.loadChatHistories() {
            Ok(value) => value,
            Err(error) => return toolError(tool, format!("Error loading chats: {error}")),
        };
        let currentChatId = match manager.currentChatIdFlow() {
            Ok(value) => value,
            Err(error) => return toolError(tool, format!("Error loading current chat: {error}")),
        };
        let messageCounts = match manager.getMessageCountsByChatId() {
            Ok(value) => value,
            Err(error) => return toolError(tool, format!("Error loading message counts: {error}")),
        };
        let idMatches = histories
            .iter()
            .filter(|chat| chat.id == query)
            .cloned()
            .collect::<Vec<_>>();
        let matched = if !idMatches.is_empty() {
            idMatches
        } else {
            match filterByTitle(histories, &query, &matchMode) {
                Ok(value) => value,
                Err(error) => return toolError(tool, error),
            }
        };
        if matched.is_empty() {
            return toolError(tool, format!("Chat not found by query: {query}"));
        }
        if targetIndex >= matched.len() {
            return toolError(
                tool,
                format!(
                    "Chat index out of range: index={targetIndex}, matched={}",
                    matched.len()
                ),
            );
        }
        successData(
            tool,
            ToolResultData::ChatFindResultData(ChatFindResultData {
                matchedCount: matched.len(),
                chat: JsNullable::Value(buildChatInfo(
                    &matched[targetIndex],
                    &messageCounts,
                    currentChatId.as_deref(),
                )),
            }),
        )
    }

    #[allow(non_snake_case)]
    /// Reports whether the selected chat is processing or idle.
    pub fn agentStatus(&self, tool: &AITool) -> ToolResult {
        let chatId = parameterValue(tool, "chat_id");
        if chatId.trim().is_empty() {
            return toolError(tool, "Invalid parameter: missing chat_id".to_string());
        }
        let isProcessing = match self.runtimeSupport.isChatProcessing(&chatId) {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };
        successData(
            tool,
            ToolResultData::AgentStatusResultData(AgentStatusResultData {
                chatId,
                state: if isProcessing { "processing" } else { "idle" }.to_string(),
                message: JsOptional::Null,
                isIdle: !isProcessing,
                isProcessing,
            }),
        )
    }

    #[allow(non_snake_case)]
    /// Switches the main runtime slot to a persisted chat.
    pub async fn switchChat(&self, tool: &AITool) -> ToolResult {
        let chatId = parameterValue(tool, "chat_id");
        if chatId.trim().is_empty() {
            return toolError(tool, "Invalid parameter: missing chat_id".to_string());
        }
        let manager = match ChatHistoryManager::default() {
            Ok(manager) => manager,
            Err(error) => return toolError(tool, format!("Error opening chat history: {error}")),
        };
        let title = match manager.getChatTitle(chatId.clone()) {
            Ok(Some(title)) => title,
            Ok(None) => return toolError(tool, format!("Chat does not exist: {chatId}")),
            Err(error) => return toolError(tool, format!("Error loading chat: {error}")),
        };
        if let Err(error) = self.runtimeSupport.switchMainChat(&chatId).await {
            return toolError(tool, error);
        }
        successData(
            tool,
            ToolResultData::ChatSwitchResultData(ChatSwitchResultData {
                chatId,
                chatTitle: title,
                switchedAt: currentTimeMillis(),
            }),
        )
    }

    #[allow(non_snake_case)]
    /// Updates the title for a persisted chat.
    pub fn updateChatTitle(&self, tool: &AITool) -> ToolResult {
        let chatId = parameterValue(tool, "chat_id");
        if chatId.trim().is_empty() {
            return toolError(tool, "Invalid parameter: missing chat_id".to_string());
        }
        let title = parameterValue(tool, "title");
        if title.trim().is_empty() {
            return toolError(tool, "Invalid parameter: missing title".to_string());
        }
        let manager = match ChatHistoryManager::default() {
            Ok(manager) => manager,
            Err(error) => return toolError(tool, format!("Error opening chat history: {error}")),
        };
        match manager.getChatTitle(chatId.clone()) {
            Ok(Some(_)) => {}
            Ok(None) => return toolError(tool, format!("Chat does not exist: {chatId}")),
            Err(error) => return toolError(tool, format!("Error loading chat: {error}")),
        }
        match manager.updateChatTitle(chatId.clone(), title.clone()) {
            Ok(()) => successData(
                tool,
                ToolResultData::ChatTitleUpdateResultData(ChatTitleUpdateResultData {
                    chatId,
                    title,
                    updatedAt: currentTimeMillis(),
                }),
            ),
            Err(error) => toolError(tool, format!("Error updating chat title: {error}")),
        }
    }

    /// Mutates only neutral chat metadata through the existing store operations.
    #[allow(non_snake_case)]
    pub fn updateChatFlag(&self, tool: &AITool, pinned: bool) -> ToolResult {
        let flag = if pinned { "pinned" } else { "locked" };
        if let Err(error) = validateChatParameterNames(tool, &["chat_id", flag]) {
            return toolError(tool, error);
        }
        let chatId = parameterValue(tool, "chat_id");
        if chatId.trim().is_empty() { return toolError(tool, "chat_id is required".to_string()); }
        let value = match parameterValue(tool, flag).as_str() {
            "true" => true, "false" => false,
            _ => return toolError(tool, format!("{flag} must be a boolean")),
        };
        let manager = match ChatHistoryManager::default() {
            Ok(manager) => manager,
            Err(error) => return toolError(tool, error.to_string()),
        };
        match manager.chatExists(chatId.clone()) {
            Ok(true) => {},
            Ok(false) => return toolError(tool, format!("Chat does not exist: {chatId}")),
            Err(error) => return toolError(tool, error.to_string()),
        }
        let result = if pinned { manager.updateChatPinned(chatId, value) }
                     else { manager.updateChatLocked(chatId, value) };
        match result {
            Ok(()) => successData(tool, stringResultData("")),
            Err(error) => toolError(tool, error.to_string()),
        }
    }

    /// Mutates native folder membership, never character-card repository data.
    #[allow(non_snake_case)]
    pub fn updateChatGroup(&self, tool: &AITool) -> ToolResult {
        if let Err(error) = validateChatParameterNames(tool, &["chat_ids", "group_name"]) {
            return toolError(tool, error);
        }
        let ids = match serde_json::from_str::<Vec<String>>(&parameterValue(tool, "chat_ids")) {
            Ok(ids) if !ids.is_empty() && ids.iter().all(|id| !id.trim().is_empty()) => ids,
            _ => return toolError(tool, "chat_ids must be a nonempty array of chat IDs".to_string()),
        };
        let name = match optionalParameterValue(tool, "group_name") {
            None => return toolError(tool, "group_name is required (string or null)".to_string()),
            Some(value) if value == "null" => None,
            Some(value) => Some(value),
        };
        let manager = match ChatHistoryManager::default() {
            Ok(manager) => manager,
            Err(error) => return toolError(tool, error.to_string()),
        };
        match manager.updateChatGroups(ids, name) {
            Ok(()) => successData(tool, stringResultData("")),
            Err(error) => toolError(tool, error.to_string()),
        }
    }

    /// Persists the same canonical ordering used by the native drawer, with no group fields.
    #[allow(non_snake_case)]
    pub fn reorderChats(&self, tool: &AITool) -> ToolResult {
        if let Err(error) = validateChatParameterNames(tool, &["chat_ids"]) {
            return toolError(tool, error);
        }
        let ids = match serde_json::from_str::<Vec<String>>(&parameterValue(tool, "chat_ids")) {
            Ok(ids) if !ids.is_empty() && ids.iter().all(|id| !id.trim().is_empty()) => ids,
            _ => return toolError(tool, "chat_ids must be a nonempty array of chat IDs".to_string()),
        };
        let manager = match ChatHistoryManager::default() {
            Ok(manager) => manager,
            Err(error) => return toolError(tool, error.to_string()),
        };
        match manager.updateChatOrder(ids) {
            Ok(()) => successData(tool, stringResultData("")),
            Err(error) => toolError(tool, error.to_string()),
        }
    }

    #[allow(non_snake_case)]
    /// Deletes a chat through the chat history manager.
    pub fn deleteChat(&self, tool: &AITool) -> ToolResult {
        let chatId = parameterValue(tool, "chat_id");
        if chatId.trim().is_empty() {
            return toolError(tool, "Invalid parameter: missing chat_id".to_string());
        }
        let manager = match ChatHistoryManager::default() {
            Ok(manager) => manager,
            Err(error) => return toolError(tool, format!("Error opening chat history: {error}")),
        };
        match manager.deleteChatHistory(chatId.clone()) {
            Ok(true) => successData(
                tool,
                ToolResultData::ChatDeleteResultData(ChatDeleteResultData {
                    chatId,
                    deletedAt: currentTimeMillis(),
                }),
            ),
            Ok(false) => toolError(tool, format!("Chat does not exist or is locked: {chatId}")),
            Err(error) => toolError(tool, format!("Error deleting chat: {error}")),
        }
    }

    #[allow(non_snake_case)]
    /// Sends a user message to the selected chat runtime and waits for a response.
    pub async fn sendMessageToAi(&self, tool: &AITool) -> ToolResult {
        let message = parameterValue(tool, "message");
        if message.trim().is_empty() && optionalParameterValue(tool, "continuation").is_none() {
            return toolError(tool, "Invalid parameter: missing message".to_string());
        }
        let runtimeSlot = match parseRuntimeSlot(optionalParameterValue(tool, "runtime").as_deref())
        {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };
        if let Err(error) = validateChatParameterNames(
            tool,
            &[
                "message",
                "runtime",
                "chat_id",
                "participant_id",
                "sender_name",
                "persist_turn",
                "notify_reply",
                "hide_user_message",
                "disable_warning",
                "timeout_ms",
                "attachments",
                "reply_to_message_timestamp",
                "continuation",
            ],
        ) {
            return toolError(tool, error);
        }
        let participantId = optionalParameterValue(tool, "participant_id");
        if participantId
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return toolError(tool, "participant_id must not be blank".to_string());
        }
        let chatId =
            optionalParameterValue(tool, "chat_id").filter(|value| !value.trim().is_empty());
        if let Some(chatId) = chatId.as_deref() {
            match ChatHistoryManager::default()
                .and_then(|manager| manager.chatExists(chatId.to_string()))
            {
                Ok(true) => {}
                Ok(false) => {
                    return toolError(tool, format!("Specified chat does not exist: {chatId}"))
                }
                Err(error) => return toolError(tool, format!("Error loading chat: {error}")),
            }
        }
        let proxySenderName =
            optionalParameterValue(tool, "sender_name").filter(|value| !value.trim().is_empty());
        let turnOptions = match parseTurnOptions(tool) {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };
        let attachments = match parseSendAttachments(tool) {
            Ok(value) => value, Err(error) => return toolError(tool, error),
        };
        let replyToMessageTimestamp = match parseReplyTimestamp(tool) {
            Ok(value) => value, Err(error) => return toolError(tool, error),
        };
        if turnOptions.continuation.is_some() && (!message.is_empty() || !attachments.is_empty() || replyToMessageTimestamp.is_some() || proxySenderName.is_some()) {
            return toolError(tool, "Continuation cannot resubmit text, attachments, reply target or sender name".to_string());
        }
        let request = RuntimeChatSendRequest {
            slot: runtimeSlot,
            participantId,
            chatId: chatId.clone(),
            message: message.clone(),
            proxySenderName,
            attachments,
            replyToMessageTimestamp,
            turnOptions,
        };
        match self.runtimeSupport.sendChatMessage(request).await {
            Ok(result) => successData(tool, ToolResultData::MessageSendResultData(result)),
            Err(error) => toolError(tool, error),
        }
    }

    /// Calls a configured functional model without persisting a chat turn.
    #[allow(non_snake_case)]
    pub async fn callChatModel(&self, tool: &AITool) -> ToolResult {
        let functionType = match parseFunctionType(parameterValue(tool, "function_type")) {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };
        let turns = match parsePromptTurns(parameterValue(tool, "turns")) {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };
        let recordTokenUsage = match parseChatCallBoolean(tool, "record_token_usage", true) {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };
        let enableThinking = match parseChatCallBoolean(tool, "enable_thinking", false) {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };
        let request = RuntimeChatCallRequest {
            functionType,
            turns,
            recordTokenUsage,
            enableThinking,
        };
        let output = match self.runtimeSupport.callChatModel(request).await {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };
        successData(
            tool,
            ToolResultData::ChatCallResultData(parseChatCallOutput(&output)),
        )
    }

    #[allow(non_snake_case)]
    /// Loads stored messages for a chat.
    pub fn getChatMessages(&self, tool: &AITool) -> ToolResult {
        let chatId = parameterValue(tool, "chat_id");
        if chatId.trim().is_empty() {
            return toolError(tool, "Invalid parameter: missing chat_id".to_string());
        }
        let order = match optionalParameterValue(tool, "order") {
            Some(value) if value.trim().is_empty() => "desc".to_string(),
            Some(value)
                if value.eq_ignore_ascii_case("asc") || value.eq_ignore_ascii_case("desc") =>
            {
                value.to_ascii_lowercase()
            }
            Some(_) => {
                return toolError(
                    tool,
                    "Invalid parameter: order must be asc/desc".to_string(),
                )
            }
            None => "desc".to_string(),
        };
        let limit = match optionalParameterValue(tool, "limit") {
            Some(value) if !value.trim().is_empty() => match value.parse::<i32>() {
                Ok(value) => value.clamp(1, 200),
                Err(_) => {
                    return toolError(
                        tool,
                        "Invalid parameter: limit must be an integer".to_string(),
                    )
                }
            },
            _ => 20,
        };
        let manager = match ChatHistoryManager::default() {
            Ok(manager) => manager,
            Err(error) => return toolError(tool, format!("Error opening chat history: {error}")),
        };
        match manager.getChatTitle(chatId.clone()) {
            Ok(Some(_)) => {}
            Ok(None) => return toolError(tool, format!("Chat does not exist: {chatId}")),
            Err(error) => return toolError(tool, format!("Error loading chat: {error}")),
        }
        match manager.loadChatMessagesWithOptions(chatId.clone(), Some(order.clone()), Some(limit))
        {
            Ok(messages) => successData(
                tool,
                ToolResultData::ChatMessagesResultData(ChatMessagesResultData {
                    chatId,
                    order,
                    limit,
                    start: None,
                    end: None,
                    messages: messages
                        .into_iter()
                        .filter(|message| message.sender != "summary")
                        .map(|message| ChatMessageInfo {
                            content: message.displayText(),
                            sender: message.sender,
                            timestamp: message.timestamp,
                            variantIndex: message.selectedVariantIndex,
                            variantCount: message.variantCount,
                            provider: message.provider,
                            modelName: message.modelName,
                        })
                        .collect(),
                }),
            ),
            Err(error) => toolError(tool, format!("Error getting chat messages: {error}")),
        }
    }

    #[allow(non_snake_case)]
    /// Loads an inclusive zero-based message range from a chat.
    pub fn getChatMessagesRange(&self, tool: &AITool) -> ToolResult {
        let chatId = parameterValue(tool, "chat_id");
        if chatId.trim().is_empty() {
            return toolError(tool, "Invalid parameter: missing chat_id".to_string());
        }
        let order = match optionalParameterValue(tool, "order") {
            Some(value) if value.trim().is_empty() => "asc".to_string(),
            Some(value)
                if value.eq_ignore_ascii_case("asc") || value.eq_ignore_ascii_case("desc") =>
            {
                value.to_ascii_lowercase()
            }
            Some(_) => {
                return toolError(
                    tool,
                    "Invalid parameter: order must be asc/desc".to_string(),
                )
            }
            None => "asc".to_string(),
        };
        let start = match optionalParameterValue(tool, "start") {
            Some(value) if !value.trim().is_empty() => match value.parse::<i32>() {
                Ok(value) => value,
                Err(_) => {
                    return toolError(
                        tool,
                        "Invalid parameter: start must be an integer".to_string(),
                    )
                }
            },
            _ => {
                return toolError(
                    tool,
                    "Invalid parameter: start and end are required".to_string(),
                )
            }
        };
        let end = match optionalParameterValue(tool, "end") {
            Some(value) if !value.trim().is_empty() => match value.parse::<i32>() {
                Ok(value) => value,
                Err(_) => {
                    return toolError(
                        tool,
                        "Invalid parameter: end must be an integer".to_string(),
                    )
                }
            },
            _ => {
                return toolError(
                    tool,
                    "Invalid parameter: start and end are required".to_string(),
                )
            }
        };
        let limit = match end
            .checked_sub(start)
            .and_then(|value| value.checked_add(1))
        {
            Some(value) if start >= 0 && end >= start => value,
            _ => {
                return toolError(
                    tool,
                    "Invalid parameter: range requires 0 <= start <= end".to_string(),
                )
            }
        };
        let manager = match ChatHistoryManager::default() {
            Ok(manager) => manager,
            Err(error) => return toolError(tool, format!("Error opening chat history: {error}")),
        };
        match manager.getChatTitle(chatId.clone()) {
            Ok(Some(_)) => {}
            Ok(None) => return toolError(tool, format!("Chat does not exist: {chatId}")),
            Err(error) => return toolError(tool, format!("Error loading chat: {error}")),
        }
        match manager.loadChatMessagesRange(chatId.clone(), order.clone(), start, end) {
            Ok(messages) => successData(
                tool,
                ToolResultData::ChatMessagesResultData(ChatMessagesResultData {
                    chatId,
                    order,
                    limit,
                    start: Some(start),
                    end: Some(end),
                    messages: messages
                        .into_iter()
                        .filter(|message| message.sender != "summary")
                        .map(|message| ChatMessageInfo {
                            content: message.displayText(),
                            sender: message.sender,
                            timestamp: message.timestamp,
                            variantIndex: message.selectedVariantIndex,
                            variantCount: message.variantCount,
                            provider: message.provider,
                            modelName: message.modelName,
                        })
                        .collect(),
                }),
            ),
            Err(error) => toolError(tool, format!("Error getting chat messages range: {error}")),
        }
    }
}

impl AsyncToolExecutor for ChatManagerToolExecutor {
    /// Validates the exact registered chat-tool request before asynchronous execution.
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        validateChatTool(self.operation, tool)
    }

    /// Declares the generic chat operation effect without consulting plugin domain storage.
    fn accessSpec(&self, _tool: &AITool) -> Result<ToolAccessSpec, String> {
        let effect = match self.operation {
            ChatManagerToolOperation::ListChats
            | ChatManagerToolOperation::FindChat
            | ChatManagerToolOperation::AgentStatus
            | ChatManagerToolOperation::GetChatMessages
            | ChatManagerToolOperation::GetChatMessagesRange => ToolEffect::READ,
            ChatManagerToolOperation::StartChatService
            | ChatManagerToolOperation::StopChatService
            | ChatManagerToolOperation::CreateNewChat
            | ChatManagerToolOperation::SwitchChat
            | ChatManagerToolOperation::UpdateChatTitle
            | ChatManagerToolOperation::UpdateChatGroup
            | ChatManagerToolOperation::UpdateChatPinned
            | ChatManagerToolOperation::UpdateChatLocked
            | ChatManagerToolOperation::ReorderChats
            | ChatManagerToolOperation::DeleteChat
            | ChatManagerToolOperation::SendMessageToAi
            | ChatManagerToolOperation::SendMessageToAiStreaming => ToolEffect::WRITE,
            ChatManagerToolOperation::CallChatModel => ToolEffect::READ,
        };
        Ok(ToolAccessSpec {
            effect,
            boundary: ToolBoundary::None,
        })
    }

    /// Creates local runtime futures on their owning scheduler and awaits only a Send receipt.
    fn invokeAndStreamAsync<'a>(&'a mut self, tool: &'a AITool) -> ToolInvocationFuture<'a> {
        let tools = self.tools.clone();
        let operation = self.operation;
        let request = tool.clone();
        let context = crate::ToolExecutionManager::ToolExecutionManager::currentToolRuntimeContext();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let scheduled = operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask("chat-tool-invocation", Box::new(move || {
                Box::pin(async move {
                    let invocation = async move {
                        let tool = request;
                        let result = match operation {
                            ChatManagerToolOperation::StartChatService => tools.startChatService(&tool),
                            ChatManagerToolOperation::StopChatService => tools.stopChatService(&tool),
                            ChatManagerToolOperation::CreateNewChat => tools.createNewChat(&tool).await,
                            ChatManagerToolOperation::ListChats => tools.listChats(&tool),
                            ChatManagerToolOperation::FindChat => tools.findChat(&tool),
                            ChatManagerToolOperation::AgentStatus => tools.agentStatus(&tool),
                            ChatManagerToolOperation::SwitchChat => tools.switchChat(&tool).await,
                            ChatManagerToolOperation::UpdateChatTitle => tools.updateChatTitle(&tool),
                            ChatManagerToolOperation::UpdateChatGroup => tools.updateChatGroup(&tool),
                            ChatManagerToolOperation::UpdateChatPinned => tools.updateChatFlag(&tool, true),
                            ChatManagerToolOperation::UpdateChatLocked => tools.updateChatFlag(&tool, false),
                            ChatManagerToolOperation::ReorderChats => tools.reorderChats(&tool),
                            ChatManagerToolOperation::DeleteChat => tools.deleteChat(&tool),
                            ChatManagerToolOperation::SendMessageToAi => tools.sendMessageToAi(&tool).await,
                            ChatManagerToolOperation::SendMessageToAiStreaming => {
                                tools.sendMessageToAi(&tool).await
                            }
                            ChatManagerToolOperation::CallChatModel => tools.callChatModel(&tool).await,
                            ChatManagerToolOperation::GetChatMessages => tools.getChatMessages(&tool),
                            ChatManagerToolOperation::GetChatMessagesRange => {
                                tools.getChatMessagesRange(&tool)
                            }
                        };
                        result
                    };
                    let result = match context {
                        Some(context) => crate::ToolExecutionManager::ToolExecutionManager::scopeToolRuntimeContext(context, invocation).await,
                        None => invocation.await,
                    };
                    let _ = sender.send(result);
                })
            }));
        Box::pin(async move {
            let result = match scheduled {
                Err(error) => toolError(tool, format!("Chat tool scheduling failed: {error}")),
                Ok(()) => match receiver.await {
                    Ok(result) => result,
                    Err(error) => toolError(tool, format!("Chat tool execution ended without a result: {error}")),
                },
            };
            vec![result]
        })
    }
}

#[allow(non_snake_case)]
fn validateChatTool(operation: ChatManagerToolOperation, tool: &AITool) -> ToolValidationResult {
    let invalid = |message: &str| ToolValidationResult {
        valid: false,
        errorMessage: message.to_string(),
    };
    match operation {
        ChatManagerToolOperation::FindChat => {
            if parameterValue(tool, "query").trim().is_empty() {
                return invalid("query is required.");
            }
        }
        ChatManagerToolOperation::AgentStatus
        | ChatManagerToolOperation::SwitchChat
        | ChatManagerToolOperation::UpdateChatTitle
        | ChatManagerToolOperation::UpdateChatPinned
        | ChatManagerToolOperation::UpdateChatLocked
        | ChatManagerToolOperation::DeleteChat
        | ChatManagerToolOperation::GetChatMessages => {
            if parameterValue(tool, "chat_id").trim().is_empty() {
                return invalid("chat_id is required.");
            }
        }
        ChatManagerToolOperation::UpdateChatGroup | ChatManagerToolOperation::ReorderChats => {
            if parameterValue(tool, "chat_ids").trim().is_empty() {
                return invalid("chat_ids is required.");
            }
        }
        ChatManagerToolOperation::GetChatMessagesRange => {
            if parameterValue(tool, "chat_id").trim().is_empty() {
                return invalid("chat_id is required.");
            }
            if let Some(order) = optionalParameterValue(tool, "order") {
                if !order.trim().is_empty()
                    && !order.eq_ignore_ascii_case("asc")
                    && !order.eq_ignore_ascii_case("desc")
                {
                    return invalid("order must be asc/desc.");
                }
            }
            let start = match optionalParameterValue(tool, "start") {
                Some(value) if !value.trim().is_empty() => match value.parse::<i32>() {
                    Ok(value) => value,
                    Err(_) => return invalid("start must be an integer."),
                },
                _ => return invalid("start is required."),
            };
            let end = match optionalParameterValue(tool, "end") {
                Some(value) if !value.trim().is_empty() => match value.parse::<i32>() {
                    Ok(value) => value,
                    Err(_) => return invalid("end must be an integer."),
                },
                _ => return invalid("end is required."),
            };
            if start < 0
                || end < start
                || end
                    .checked_sub(start)
                    .and_then(|value| value.checked_add(1))
                    .is_none()
            {
                return invalid("range requires 0 <= start <= end.");
            }
        }
        ChatManagerToolOperation::SendMessageToAi
        | ChatManagerToolOperation::SendMessageToAiStreaming => {
            if parameterValue(tool, "message").trim().is_empty() && optionalParameterValue(tool, "continuation").is_none() {
                return invalid("message is required for an initial submit.");
            }
        }
        ChatManagerToolOperation::CallChatModel => {
            if parameterValue(tool, "function_type").trim().is_empty() {
                return invalid("function_type is required.");
            }
            if parameterValue(tool, "turns").trim().is_empty() {
                return invalid("turns is required.");
            }
        }
        ChatManagerToolOperation::StartChatService
        | ChatManagerToolOperation::StopChatService
        | ChatManagerToolOperation::CreateNewChat
        | ChatManagerToolOperation::ListChats => {}
    }
    ToolValidationResult {
        valid: true,
        errorMessage: String::new(),
    }
}

/// Parses a functional model enum using the public uppercase wire names.
fn parseFunctionType(value: String) -> Result<FunctionType, String> {
    match value.trim().to_ascii_uppercase().as_str() {
        "CHAT" => Ok(FunctionType::CHAT),
        "SUMMARY" => Ok(FunctionType::SUMMARY),
        "TITLE_GENERATION" => Ok(FunctionType::TITLE_GENERATION),
        "MEMORY" => Ok(FunctionType::MEMORY),
        "UI_CONTROLLER" => Ok(FunctionType::UI_CONTROLLER),
        "TRANSLATION" => Ok(FunctionType::TRANSLATION),
        "GREP" => Ok(FunctionType::GREP),
        "ROLE_RESPONSE_PLANNER" => Ok(FunctionType::ROLE_RESPONSE_PLANNER),
        "IMAGE_RECOGNITION" => Ok(FunctionType::IMAGE_RECOGNITION),
        "AUDIO_RECOGNITION" => Ok(FunctionType::AUDIO_RECOGNITION),
        "VIDEO_RECOGNITION" => Ok(FunctionType::VIDEO_RECOGNITION),
        other => Err(format!("Invalid functionType: {other}")),
    }
}

/// Parses the Kotlin-compatible boolean spellings used by call_chat_model.
fn parseChatCallBoolean(tool: &AITool, name: &str, defaultValue: bool) -> Result<bool, String> {
    let value = tool
        .parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.trim().to_ascii_lowercase());
    match value.as_deref() {
        None | Some("") => Ok(defaultValue),
        Some("true") | Some("1") | Some("yes") => Ok(true),
        Some("false") | Some("0") | Some("no") => Ok(false),
        Some(_) => Err(format!(
            "{} must be true/false",
            if name == "record_token_usage" {
                "recordTokenUsage"
            } else {
                "enableThinking"
            }
        )),
    }
}

/// Parses the Kotlin-compatible PromptTurn JSON contract used by functional calls.
fn parsePromptTurns(value: String) -> Result<Vec<PromptTurn>, String> {
    let decoded = serde_json::from_str::<Value>(value.trim())
        .map_err(|_| "turns must be a JSON array".to_string())?;
    let items = decoded
        .as_array()
        .ok_or_else(|| "turns must be a JSON array".to_string())?;
    if items.is_empty() {
        return Err("turns must contain at least one PromptTurn".to_string());
    }
    items
        .iter()
        .enumerate()
        .map(|(index, item)| parsePromptTurn(index, item))
        .collect()
}

/// Parses one PromptTurn object with the same validation rules as the Kotlin implementation.
fn parsePromptTurn(index: usize, value: &Value) -> Result<PromptTurn, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("turns[{index}] must be an object"))?;
    let kindValue = object
        .get("kind")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("turns[{index}].kind is required"))?;
    let kind = match kindValue.to_ascii_uppercase().as_str() {
        "SYSTEM" => PromptTurnKind::SYSTEM,
        "USER" => PromptTurnKind::USER,
        "ASSISTANT" => PromptTurnKind::ASSISTANT,
        "TOOL_CALL" => PromptTurnKind::TOOL_CALL,
        "TOOL_RESULT" => PromptTurnKind::TOOL_RESULT,
        "SUMMARY" => PromptTurnKind::SUMMARY,
        _ => return Err(format!("Invalid turns[{index}].kind: {kindValue}")),
    };
    let content = object
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("turns[{index}].content must be a string"))?;
    let toolName = match object.get("toolName") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_string())
        }
        Some(_) => return Err(format!("turns[{index}].toolName must be a string")),
    };
    let metadata = match object.get("metadata") {
        None | Some(Value::Null) => HashMap::new(),
        Some(Value::Object(value)) => value
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        Some(_) => return Err(format!("turns[{index}].metadata must be an object")),
    };
    Ok(PromptTurn {
        kind,
        content: content.to_string(),
        tool_name: toolName,
        metadata,
    })
}

#[derive(Clone)]
/// Represents one tool markup range and its optional public tool name.
struct ChatCallToolMatch {
    start: usize,
    end: usize,
    content: String,
    toolName: Option<String>,
}

/// Collects paired and self-closing tool calls in the order used by the Kotlin parser.
fn collectChatCallToolMatches(content: &str) -> Vec<ChatCallToolMatch> {
    let mut matches = operit_util::ChatMarkupRegex::ChatMarkupRegex::tool_call_matches(content)
        .into_iter()
        .map(|matched| ChatCallToolMatch {
            start: matched.start,
            end: matched.end,
            content: content[matched.start..matched.end].trim().to_string(),
            toolName: Some(matched.name),
        })
        .collect::<Vec<_>>();
    let mut cursor = 0;
    while let Some(relativeStart) = content[cursor..].find('<') {
        let start = cursor + relativeStart;
        let Some(tagName) = operit_util::ChatMarkupRegex::ChatMarkupRegex::extract_opening_tag_name(
            &content[start..],
        ) else {
            cursor = start + 1;
            continue;
        };
        if !operit_util::ChatMarkupRegex::ChatMarkupRegex::is_tool_tag_name(Some(&tagName)) {
            cursor = start + 1;
            continue;
        }
        let Some(relativeEnd) = content[start..].find('>') else {
            break;
        };
        let end = start + relativeEnd + 1;
        let raw = &content[start..end];
        if raw.trim_end().ends_with("/>") {
            matches.push(ChatCallToolMatch {
                start,
                end,
                content: raw.trim().to_string(),
                toolName: operit_util::ChatMarkupRegex::attr_value(raw, "name")
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
            });
        }
        cursor = end;
    }
    matches.sort_by_key(|matched| matched.start);
    matches
}

/// Collects all complete and self-closing tool markup ranges for response text cleanup.
fn collectToolMarkupRanges(content: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut cursor = 0;
    while let Some(relativeStart) = content[cursor..].find('<') {
        let start = cursor + relativeStart;
        let Some(tagName) = operit_util::ChatMarkupRegex::ChatMarkupRegex::extract_opening_tag_name(
            &content[start..],
        ) else {
            cursor = start + 1;
            continue;
        };
        if !operit_util::ChatMarkupRegex::ChatMarkupRegex::is_tool_tag_name(Some(&tagName)) {
            cursor = start + 1;
            continue;
        }
        let Some(relativeOpenEnd) = content[start..].find('>') else {
            break;
        };
        let openEnd = start + relativeOpenEnd + 1;
        let opening = &content[start..openEnd];
        if opening.trim_end().ends_with("/>") {
            ranges.push((start, openEnd));
            cursor = openEnd;
            continue;
        }
        let close = format!("</{}>", tagName.to_ascii_lowercase());
        let lowerTail = content[start..].to_ascii_lowercase();
        let Some(relativeClose) = lowerTail.find(&close) else {
            cursor = start + 1;
            continue;
        };
        let end = start + relativeClose + close.len();
        ranges.push((start, end));
        cursor = end;
    }
    ranges
}

/// Removes response markup ranges while preserving all ordinary model text.
fn removeRanges(content: &str, ranges: &[(usize, usize)]) -> String {
    let mut output = content.to_string();
    for (start, end) in ranges.iter().rev() {
        output.replace_range(*start..*end, "");
    }
    output
}

/// Converts one raw model response into the legacy ChatCallResultData protocol.
fn parseChatCallOutput(rawContent: &str) -> ChatCallResultData {
    let mut metadata = BTreeMap::new();
    for (start, end) in operit_util::ChatMarkupRegex::tag_ranges(rawContent, "meta") {
        let tag = &rawContent[start..end];
        if let Some(provider) = operit_util::ChatMarkupRegex::attr_value(tag, "provider") {
            if let Some(body) = operit_util::ChatMarkupRegex::tag_body(tag, "meta") {
                let entry = json!({ "provider": provider, "payload": body.trim() });
                let values = metadata
                    .entry("protocolMeta".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(values) = values {
                    values.push(entry);
                }
            }
        }
    }
    let content = removeProtocolMetadata(rawContent);
    let matches = collectChatCallToolMatches(&content);
    let mut turns = Vec::new();
    let mut cursor = 0;
    for matched in matches {
        if matched.start > cursor {
            appendAssistantTurn(&mut turns, &content[cursor..matched.start]);
        }
        turns.push(ChatCallTurnData {
            kind: "TOOL_CALL".to_string(),
            content: matched.content,
            toolName: matched.toolName,
            metadata: BTreeMap::new(),
        });
        cursor = matched.end;
    }
    if cursor < content.len() {
        appendAssistantTurn(&mut turns, &content[cursor..]);
    }
    let text = removeRanges(&content, &collectToolMarkupRanges(&content));
    let finishReason = if turns.iter().any(|turn| turn.kind == "TOOL_CALL") {
        "tool_call"
    } else {
        "stop"
    };
    ChatCallResultData {
        text: text.trim().to_string(),
        turns,
        finishReason: finishReason.to_string(),
        metadata,
        receivedAt: currentTimeMillis(),
    }
}

/// Removes provider metadata tags while preserving all other response content.
fn removeProtocolMetadata(rawContent: &str) -> String {
    let mut ranges = Vec::new();
    for (start, end) in operit_util::ChatMarkupRegex::tag_ranges(rawContent, "meta") {
        let tag = &rawContent[start..end];
        if operit_util::ChatMarkupRegex::attr_value(tag, "provider").is_some() {
            ranges.push((start, end));
        }
    }
    let mut output = rawContent.to_string();
    for (start, end) in ranges.into_iter().rev() {
        output.replace_range(start..end, "");
    }
    output.trim().to_string()
}

/// Adds one non-empty assistant response segment to the result turn list.
fn appendAssistantTurn(turns: &mut Vec<ChatCallTurnData>, segment: &str) {
    let text = segment.trim();
    if !text.is_empty() {
        turns.push(ChatCallTurnData {
            kind: "ASSISTANT".to_string(),
            content: text.to_string(),
            toolName: None,
            metadata: BTreeMap::new(),
        });
    }
}

/// Reads and trims one required-style tool parameter.
fn parameterValue(tool: &AITool, name: &str) -> String {
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.trim().to_string())
        .unwrap_or_default()
}

fn optionalParameterValue(tool: &AITool, name: &str) -> Option<String> {
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.trim().to_string())
}

/// Validates exact neutral chat parameter identities and rejects duplicate or undeclared inputs.
fn validateChatParameterNames(tool: &AITool, allowed: &[&str]) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for parameter in &tool.parameters {
        if !allowed.iter().any(|name| *name == parameter.name) {
            return Err(format!("Unknown chat parameter: {}", parameter.name));
        }
        if !seen.insert(&parameter.name) {
            return Err(format!("Duplicate chat parameter: {}", parameter.name));
        }
    }
    Ok(())
}

/// Parses explicitly supplied booleans and never converts an invalid value into parameter absence.
fn parseOptionalBoolean(tool: &AITool, name: &str) -> Result<Option<bool>, String> {
    match optionalParameterValue(tool, name) {
        Some(value) if value.eq_ignore_ascii_case("true") => Ok(Some(true)),
        Some(value) if value.eq_ignore_ascii_case("false") => Ok(Some(false)),
        Some(_) => Err(format!("Invalid parameter: {name} must be true/false")),
        None => Ok(None),
    }
}

/// Resolves a chat surface identifier to its runtime slot.
fn parseRuntimeSlot(value: Option<&str>) -> Result<RuntimeChatSlot, String> {
    match value.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if value == "main" => Ok(RuntimeChatSlot::MAIN),
        Some(value) if value == "floating" || value.is_empty() => Ok(RuntimeChatSlot::FLOATING),
        Some(_) => Err("Invalid parameter: runtime must be main/floating".to_string()),
        None => Ok(RuntimeChatSlot::FLOATING),
    }
}

/// Decodes exact original attachment records instead of fabricating objects from count or filenames.
fn parseSendAttachments(tool: &AITool) -> Result<Vec<AttachmentInfo>, String> {
    match optionalParameterValue(tool, "attachments") {
        None => Ok(Vec::new()),
        Some(value) => {
            let records: Vec<operit_plugin_sdk::js_sdk::chat::ChatSendAttachment> = serde_json::from_str(&value)
                .map_err(|error| format!("attachments must be a complete attachment array: {error}"))?;
            Ok(records.into_iter().map(|record| AttachmentInfo {
                filePath: record.filePath, nodeId: record.nodeId.as_value().cloned(), fileName: record.fileName,
                mimeType: record.mimeType, fileSize: record.fileSize, content: record.content,
            }).collect())
        }
    }
}

/// Decodes an explicitly supplied canonical reply target timestamp without treating invalid input as absence.
fn parseReplyTimestamp(tool: &AITool) -> Result<Option<i64>, String> {
    optionalParameterValue(tool, "reply_to_message_timestamp").map(|value| value.parse::<i64>()
        .map_err(|error| format!("replyToMessageTimestamp must be an integer: {error}"))).transpose()
}

/// Decodes a concrete existing user-turn locator and rejects forged namespace or extra continuation fields.
fn parseContinuation(tool: &AITool) -> Result<Option<ChatTurnContinuation>, String> {
    optionalParameterValue(tool, "continuation").map(|value| {
        let continuation: ChatTurnContinuation = serde_json::from_str(&value)
            .map_err(|error| format!("Invalid continuation: {error}"))?;
        if continuation.userMessageTimestamp <= 0 {
            return Err("Continuation userMessageTimestamp must be positive".to_string());
        }
        Ok(continuation)
    }).transpose()
}

/// Applies declared omitted-option semantics while preserving explicit malformed values as errors.
fn parseTurnOptions(tool: &AITool) -> Result<ChatTurnOptions, String> {
    Ok(ChatTurnOptions {
        persistTurn: match parseOptionalBoolean(tool, "persist_turn")? { Some(value) => value, None => true },
        notifyReply: parseOptionalBoolean(tool, "notify_reply")?,
        hideUserMessage: match parseOptionalBoolean(tool, "hide_user_message")? { Some(value) => value, None => false },
        disableWarning: match parseOptionalBoolean(tool, "disable_warning")? { Some(value) => value, None => false },
        continuation: parseContinuation(tool)?,
        chatInputSubmitRequestedHandled: false,
        nativeExecutionId: None,
        outputObserver: None,
        deferSequenceCompletion: false,
    })
}

fn parseMatchMode(tool: &AITool) -> Result<String, String> {
    match optionalParameterValue(tool, "match").map(|value| value.to_ascii_lowercase()) {
        Some(value) if value == "exact" || value == "regex" || value == "contains" => Ok(value),
        Some(value) if value.trim().is_empty() => Ok("contains".to_string()),
        Some(_) => Err("Invalid parameter: match must be contains/exact/regex".to_string()),
        None => Ok("contains".to_string()),
    }
}

fn filterByTitle(
    histories: Vec<ChatHistory>,
    query: &str,
    matchMode: &str,
) -> Result<Vec<ChatHistory>, String> {
    if query.trim().is_empty() {
        return Ok(histories);
    }
    match matchMode {
        "exact" => Ok(histories
            .into_iter()
            .filter(|chat| chat.title == query)
            .collect()),
        "regex" => {
            let regex = Regex::new(query).map_err(|_| "Invalid regex query".to_string())?;
            Ok(histories
                .into_iter()
                .filter(|chat| regex.is_match(&chat.title))
                .collect())
        }
        _ => Ok(histories
            .into_iter()
            .filter(|chat| chat.title.contains(query))
            .collect()),
    }
}

fn buildFilteredChatList(tool: &AITool) -> Result<(usize, Option<String>, Vec<ChatInfo>), String> {
    let manager = ChatHistoryManager::default()
        .map_err(|error| format!("Error opening chat history: {error}"))?;
    let histories = manager
        .loadChatHistories()
        .map_err(|error| format!("Error loading chats: {error}"))?;
    let currentChatId = manager
        .currentChatIdFlow()
        .map_err(|error| format!("Error loading current chat: {error}"))?;
    let messageCounts = manager
        .getMessageCountsByChatId()
        .map_err(|error| format!("Error loading message counts: {error}"))?;
    let query = optionalParameterValue(tool, "query").unwrap_or_default();
    let matchMode = parseMatchMode(tool)?;
    let limit = match optionalParameterValue(tool, "limit") {
        Some(value) if !value.trim().is_empty() => value
            .parse::<usize>()
            .map_err(|_| "Invalid parameter: limit must be an integer".to_string())?
            .clamp(1, 200),
        _ => 50,
    };
    let sortBy = match optionalParameterValue(tool, "sort_by") {
        Some(value) if value == "createdAt" || value == "updatedAt" || value == "messageCount" => {
            value
        }
        Some(value) if value.trim().is_empty() => "updatedAt".to_string(),
        Some(_) => {
            return Err(
                "Invalid parameter: sort_by must be updatedAt/createdAt/messageCount".to_string(),
            )
        }
        None => "updatedAt".to_string(),
    };
    let sortOrder =
        match optionalParameterValue(tool, "sort_order").map(|value| value.to_ascii_lowercase()) {
            Some(value) if value == "asc" || value == "desc" => value,
            Some(value) if value.trim().is_empty() => "desc".to_string(),
            Some(_) => return Err("Invalid parameter: sort_order must be asc/desc".to_string()),
            None => "desc".to_string(),
        };
    let mut matched = filterByTitle(histories, &query, &matchMode)?;
    matched.sort_by(|left, right| {
        let leftValue = sortableChatValue(left, &messageCounts, &sortBy);
        let rightValue = sortableChatValue(right, &messageCounts, &sortBy);
        if sortOrder == "asc" {
            leftValue.cmp(&rightValue)
        } else {
            rightValue.cmp(&leftValue)
        }
    });
    let totalCount = matched.len();
    let chats = matched
        .into_iter()
        .take(limit)
        .map(|chat| buildChatInfo(&chat, &messageCounts, currentChatId.as_deref()))
        .collect();
    Ok((totalCount, currentChatId, chats))
}

fn sortableChatValue(
    chat: &ChatHistory,
    messageCounts: &std::collections::HashMap<String, i32>,
    sortBy: &str,
) -> i64 {
    match sortBy {
        "messageCount" => messageCounts.get(&chat.id).copied().unwrap_or(0) as i64,
        "createdAt" => chat.createdAt.parse::<i64>().unwrap_or(0),
        _ => chat.updatedAt.parse::<i64>().unwrap_or(0),
    }
}

/// Projects only workspace conversation metadata without resolving plugin-owned membership.
fn buildChatInfo(
    chat: &ChatHistory,
    messageCounts: &std::collections::HashMap<String, i32>,
    currentChatId: Option<&str>,
) -> ChatInfo {
    ChatInfo {
        id: chat.id.clone(),
        title: chat.title.clone(),
        messageCount: messageCounts.get(&chat.id).copied().unwrap_or(0),
        createdAt: chat.createdAt.clone(),
        updatedAt: chat.updatedAt.clone(),
        isCurrent: currentChatId == Some(chat.id.as_str()),
        inputTokens: chat.inputTokens,
        outputTokens: chat.outputTokens,
    }
}



fn successData(tool: &AITool, value: ToolResultData) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: value,
        error: None,
    }
}

fn toolError(tool: &AITool, message: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: stringResultData(""),
        error: Some(message),
    }
}

#[cfg(test)]
mod sidebar_chat_tool_contract_tests {
    use super::*;
    use crate::ToolExecutionManager::ToolParameter;

    fn request(parameters: &[(&str, &str)]) -> AITool {
        AITool {
            name: "contract-test".to_string(),
            parameters: parameters.iter().map(|(name, value)| ToolParameter {
                name: (*name).to_string(), value: (*value).to_string(),
            }).collect(),
        }
    }

    #[test]
    fn generic_metadata_tools_require_an_existing_chat_identity() {
        for operation in [ChatManagerToolOperation::UpdateChatPinned, ChatManagerToolOperation::UpdateChatLocked] {
            assert!(!validateChatTool(operation, &request(&[])).valid);
            assert!(!validateChatTool(operation, &request(&[("chat_id", " ")])).valid);
            assert!(validateChatTool(operation, &request(&[("chat_id", "explicit-chat")])).valid);
        }
    }

    #[test]
    fn canonical_order_tool_requires_an_explicit_id_list() {
        assert!(!validateChatTool(ChatManagerToolOperation::ReorderChats, &request(&[])).valid);
        assert!(validateChatTool(ChatManagerToolOperation::ReorderChats,
            &request(&[("chat_ids", "[\"first\",\"second\"]")])).valid);
        assert!(validateChatParameterNames(&request(&[("chat_ids", "[]"), ("group_id", "plugin-business")]), &["chat_ids"]).is_err());
    }
}
