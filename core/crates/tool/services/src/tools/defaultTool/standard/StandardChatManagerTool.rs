use operit_host_api::TimeUtils::currentTimeMillis;
use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use crate::runtime_support::{
    RuntimeChatCallRequest, RuntimeChatSendRequest, RuntimeChatSlot, ToolRuntimeSupport,
};
use crate::tools::ToolResultDataClasses::{
    stringResultData, AgentStatusResultData, CharacterCardInfo, CharacterCardListResultData,
    ChatCallResultData, ChatCallTurnData, ChatCreationResultData, ChatDeleteResultData,
    ChatFindResultData, ChatInfo, ChatListResultData, ChatMessageInfo, ChatMessagesResultData,
    ChatServiceStartResultData, ChatSwitchResultData, ChatTitleUpdateResultData, JsNullable,
    JsOptional, MessageSendResultData, ToolResultData,
};
use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::{
    AITool, ToolAccessSpec, ToolBoundary, ToolEffect, ToolExecutor, ToolValidationResult,
};
use operit_model::ChatHistory::ChatHistory;
use operit_model::ChatTurnOptions::ChatTurnOptions;
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
    DeleteChat,
    SendMessageToAi,
    SendMessageToAiStreaming,
    CallChatModel,
    ListCharacterCards,
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

    #[allow(non_snake_case)]
    /// Creates a new chat and returns its persisted metadata.
    pub fn createNewChat(&self, tool: &AITool) -> ToolResult {
        let group = optionalParameterValue(tool, "group").filter(|value| !value.trim().is_empty());
        let setAsCurrentChat = match parseOptionalBoolean(tool, "set_as_current_chat") {
            Ok(value) => value.unwrap_or(true),
            Err(error) => return toolError(tool, error),
        };
        let characterCardId = optionalParameterValue(tool, "character_card_id")
            .filter(|value| !value.trim().is_empty());
        let characterCardName = match resolveCharacterCardName(
            self.runtimeSupport.as_ref(),
            characterCardId.as_deref(),
        ) {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };

        let previousChatIds = match ChatHistoryManager::default() {
            Ok(manager) => match manager.loadChatHistories() {
                Ok(histories) => histories
                    .into_iter()
                    .map(|chat| chat.id)
                    .collect::<Vec<_>>(),
                Err(error) => return toolError(tool, format!("Error loading chats: {error}")),
            },
            Err(error) => return toolError(tool, format!("Error opening chat history: {error}")),
        };

        if let Err(error) =
            self.runtimeSupport
                .createChatRuntime(characterCardName, group, setAsCurrentChat)
        {
            return toolError(tool, error);
        }

        match ChatHistoryManager::default().and_then(|manager| manager.loadChatHistories()) {
            Ok(histories) => {
                let created = histories
                    .into_iter()
                    .find(|chat| !previousChatIds.iter().any(|id| id == &chat.id));
                match created {
                    Some(chat) => successData(
                        tool,
                        ToolResultData::ChatCreationResultData(ChatCreationResultData {
                            chatId: chat.id,
                            createdAt: currentTimeMillis(),
                        }),
                    ),
                    None => toolError(
                        tool,
                        "Failed to create chat, unable to get new chat ID".to_string(),
                    ),
                }
            }
            Err(error) => toolError(tool, format!("Error creating chat: {error}")),
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
    pub fn switchChat(&self, tool: &AITool) -> ToolResult {
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
        if let Err(error) = self.runtimeSupport.switchMainChat(&chatId) {
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
    pub fn sendMessageToAi(&self, tool: &AITool) -> ToolResult {
        let message = parameterValue(tool, "message");
        if message.trim().is_empty() {
            return toolError(tool, "Invalid parameter: missing message".to_string());
        }
        let runtimeSlot = match parseRuntimeSlot(optionalParameterValue(tool, "runtime").as_deref())
        {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };
        let roleCardId =
            optionalParameterValue(tool, "role_card_id").filter(|value| !value.trim().is_empty());
        if let Some(roleCardId) = roleCardId.as_deref() {
            if let Err(error) =
                resolveCharacterCardName(self.runtimeSupport.as_ref(), Some(roleCardId))
            {
                return toolError(tool, error);
            }
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
        let sentAt = currentTimeMillis();
        let request = RuntimeChatSendRequest {
            slot: runtimeSlot,
            roleCardId,
            chatId: chatId.clone(),
            message: message.clone(),
            proxySenderName,
            turnOptions,
        };
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())
            .and_then(|runtime| runtime.block_on(self.runtimeSupport.sendChatMessage(request)));
        if let Err(error) = result {
            return toolError(tool, format!("Error sending message: {error}"));
        }
        let resolvedChatId = match chatId {
            Some(chatId) => chatId,
            None => match ChatHistoryManager::default()
                .and_then(|manager| manager.currentChatIdFlow())
            {
                Ok(Some(chatId)) => chatId,
                Ok(None) => return toolError(tool, "Unable to get current chat ID".to_string()),
                Err(error) => {
                    return toolError(tool, format!("Error loading current chat: {error}"))
                }
            },
        };
        let aiResponse = latestAssistantMessage(&resolvedChatId);
        successData(
            tool,
            ToolResultData::MessageSendResultData(MessageSendResultData {
                chatId: resolvedChatId,
                message,
                aiResponse: JsOptional::from_nullable_option(aiResponse),
                receivedAt: JsOptional::Value(currentTimeMillis()),
                sentAt,
            }),
        )
    }

    /// Calls a configured functional model without persisting a chat turn.
    #[allow(non_snake_case)]
    pub fn callChatModel(&self, tool: &AITool) -> ToolResult {
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
        let output = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())
            .and_then(|runtime| runtime.block_on(self.runtimeSupport.callChatModel(request)))
        {
            Ok(value) => value,
            Err(error) => return toolError(tool, format!("Error calling chat model: {error}")),
        };
        successData(
            tool,
            ToolResultData::ChatCallResultData(parseChatCallOutput(&output)),
        )
    }

    #[allow(non_snake_case)]
    /// Lists character cards available to chat sessions.
    pub fn listCharacterCards(&self, tool: &AITool) -> ToolResult {
        match self.runtimeSupport.listCharacterCards() {
            Ok(cards) => successData(
                tool,
                ToolResultData::CharacterCardListResultData(CharacterCardListResultData {
                    totalCount: cards.len(),
                    cards: cards
                        .into_iter()
                        .map(|card| CharacterCardInfo {
                            id: card.id,
                            name: card.name,
                            description: card.description,
                            isDefault: card.isDefault,
                            createdAt: card.createdAt,
                            updatedAt: card.updatedAt,
                        })
                        .collect(),
                }),
            ),
            Err(error) => toolError(tool, format!("Error listing character cards: {error}")),
        }
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
                            roleName: message.roleName,
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
                            roleName: message.roleName,
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

impl ToolExecutor for ChatManagerToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        validateChatTool(self.operation, tool)
    }

    fn accessSpec(&self, _tool: &AITool) -> Result<ToolAccessSpec, String> {
        let effect = match self.operation {
            ChatManagerToolOperation::ListChats
            | ChatManagerToolOperation::FindChat
            | ChatManagerToolOperation::AgentStatus
            | ChatManagerToolOperation::ListCharacterCards
            | ChatManagerToolOperation::GetChatMessages
            | ChatManagerToolOperation::GetChatMessagesRange => ToolEffect::READ,
            ChatManagerToolOperation::StartChatService
            | ChatManagerToolOperation::StopChatService
            | ChatManagerToolOperation::CreateNewChat
            | ChatManagerToolOperation::SwitchChat
            | ChatManagerToolOperation::UpdateChatTitle
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

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        let result = match self.operation {
            ChatManagerToolOperation::StartChatService => self.tools.startChatService(tool),
            ChatManagerToolOperation::StopChatService => self.tools.stopChatService(tool),
            ChatManagerToolOperation::CreateNewChat => self.tools.createNewChat(tool),
            ChatManagerToolOperation::ListChats => self.tools.listChats(tool),
            ChatManagerToolOperation::FindChat => self.tools.findChat(tool),
            ChatManagerToolOperation::AgentStatus => self.tools.agentStatus(tool),
            ChatManagerToolOperation::SwitchChat => self.tools.switchChat(tool),
            ChatManagerToolOperation::UpdateChatTitle => self.tools.updateChatTitle(tool),
            ChatManagerToolOperation::DeleteChat => self.tools.deleteChat(tool),
            ChatManagerToolOperation::SendMessageToAi => self.tools.sendMessageToAi(tool),
            ChatManagerToolOperation::SendMessageToAiStreaming => self.tools.sendMessageToAi(tool),
            ChatManagerToolOperation::CallChatModel => self.tools.callChatModel(tool),
            ChatManagerToolOperation::ListCharacterCards => self.tools.listCharacterCards(tool),
            ChatManagerToolOperation::GetChatMessages => self.tools.getChatMessages(tool),
            ChatManagerToolOperation::GetChatMessagesRange => self.tools.getChatMessagesRange(tool),
        };
        vec![result]
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
        | ChatManagerToolOperation::DeleteChat
        | ChatManagerToolOperation::GetChatMessages => {
            if parameterValue(tool, "chat_id").trim().is_empty() {
                return invalid("chat_id is required.");
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
            if parameterValue(tool, "message").trim().is_empty() {
                return invalid("message is required.");
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
        | ChatManagerToolOperation::ListChats
        | ChatManagerToolOperation::ListCharacterCards => {}
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

fn parseOptionalBoolean(tool: &AITool, name: &str) -> Result<Option<bool>, String> {
    match optionalParameterValue(tool, name) {
        Some(value) if value.eq_ignore_ascii_case("true") => Ok(Some(true)),
        Some(value) if value.eq_ignore_ascii_case("false") => Ok(Some(false)),
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(_) => Err(format!("Invalid parameter: {name} must be true/false")),
        None => Ok(None),
    }
}

fn parseRuntimeSlot(value: Option<&str>) -> Result<RuntimeChatSlot, String> {
    match value.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if value == "main" => Ok(RuntimeChatSlot::MAIN),
        Some(value) if value == "floating" || value.is_empty() => Ok(RuntimeChatSlot::FLOATING),
        Some(_) => Err("Invalid parameter: runtime must be main/floating".to_string()),
        None => Ok(RuntimeChatSlot::FLOATING),
    }
}

fn parseTurnOptions(tool: &AITool) -> Result<ChatTurnOptions, String> {
    Ok(ChatTurnOptions {
        persistTurn: parseOptionalBoolean(tool, "persist_turn")?.unwrap_or(true),
        notifyReply: parseOptionalBoolean(tool, "notify_reply")?,
        hideUserMessage: parseOptionalBoolean(tool, "hide_user_message")?.unwrap_or(false),
        disableWarning: parseOptionalBoolean(tool, "disable_warning")?.unwrap_or(false),
        chatInputSubmitRequestedHandled: false,
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
        characterCardName: JsOptional::from_nullable_option(chat.characterCardName.clone()),
    }
}

fn resolveCharacterCardName(
    runtimeSupport: &dyn ToolRuntimeSupport,
    cardId: Option<&str>,
) -> Result<Option<String>, String> {
    match cardId {
        Some(cardId) => runtimeSupport
            .characterCardName(cardId)
            .map(Some)
            .map_err(|_| "Invalid parameter: character_card_id not found".to_string()),
        None => Ok(None),
    }
}

fn latestAssistantMessage(chatId: &str) -> Option<String> {
    ChatHistoryManager::default()
        .and_then(|manager| {
            manager.loadChatMessagesWithOptions(
                chatId.to_string(),
                Some("desc".to_string()),
                Some(20),
            )
        })
        .ok()
        .and_then(|messages| {
            messages
                .into_iter()
                .find(|message| message.sender != "user" && message.sender != "summary")
                .map(|message| message.displayText())
        })
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
