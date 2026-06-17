use std::sync::{Arc, Mutex};

use operit_host_api::TimeUtils::currentTimeMillis;
use regex::Regex;

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ConversationService::ConversationService;
use crate::api::chat::enhance::ToolExecutionManager::{AITool, ToolExecutor, ToolValidationResult};
use crate::api::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use crate::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use crate::api::chat::EnhancedAIService::EnhancedAIService;
use crate::core::tools::AIToolHandler::AIToolHandler;
use crate::core::tools::ToolResultDataClasses::{
    stringResultData, AgentStatusResultData, CharacterCardInfo, CharacterCardListResultData,
    ChatCreationResultData, ChatDeleteResultData, ChatFindResultData, ChatInfo, ChatListResultData,
    ChatMessageInfo, ChatMessagesResultData, ChatServiceStartResultData, ChatSwitchResultData,
    ChatTitleUpdateResultData, MessageSendResultData, ToolResultData,
};
use crate::data::model::AttachmentInfo::AttachmentInfo;
use crate::data::model::ChatHistory::ChatHistory;
use crate::data::model::ChatMessage::ChatMessage;
use crate::data::model::ChatTurnOptions::ChatTurnOptions;
use crate::data::model::PromptFunctionType::PromptFunctionType;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::repository::ChatHistoryManager::ChatHistoryManager;

#[derive(Clone)]
pub struct StandardChatManagerTool {
    pub holder: Arc<Mutex<ChatRuntimeHolder>>,
}

#[derive(Clone, Copy)]
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
    ListCharacterCards,
    GetChatMessages,
}

#[derive(Clone)]
pub struct ChatManagerToolExecutor {
    pub tools: StandardChatManagerTool,
    pub operation: ChatManagerToolOperation,
}

impl StandardChatManagerTool {
    pub fn new() -> Self {
        Self {
            holder: Arc::new(Mutex::new(ChatRuntimeHolder::new())),
        }
    }

    #[allow(non_snake_case)]
    pub fn startChatService(&self, tool: &AITool) -> ToolResult {
        match self.holder.lock() {
            Ok(mut holder) => {
                holder.getCore(ChatRuntimeSlot::MAIN);
                holder.getCore(ChatRuntimeSlot::FLOATING);
                successData(
                    tool,
                    ToolResultData::ChatServiceStartResultData(ChatServiceStartResultData {
                        isConnected: true,
                        connectionTime: currentTimeMillis(),
                    }),
                )
            }
            Err(error) => toolError(tool, format!("ChatRuntimeHolder lock failed: {error}")),
        }
    }

    #[allow(non_snake_case)]
    pub fn stopChatService(&self, tool: &AITool) -> ToolResult {
        match self.holder.lock() {
            Ok(mut holder) => {
                holder.cores.clear();
                successData(
                    tool,
                    ToolResultData::ChatServiceStartResultData(ChatServiceStartResultData {
                        isConnected: false,
                        connectionTime: currentTimeMillis(),
                    }),
                )
            }
            Err(error) => toolError(tool, format!("ChatRuntimeHolder lock failed: {error}")),
        }
    }

    #[allow(non_snake_case)]
    pub fn createNewChat(&self, tool: &AITool) -> ToolResult {
        let group = optionalParameterValue(tool, "group").filter(|value| !value.trim().is_empty());
        let setAsCurrentChat = match parseOptionalBoolean(tool, "set_as_current_chat") {
            Ok(value) => value.unwrap_or(true),
            Err(error) => return toolError(tool, error),
        };
        let characterCardId = optionalParameterValue(tool, "character_card_id")
            .filter(|value| !value.trim().is_empty());
        let characterCardName = match resolveCharacterCardName(characterCardId.as_deref()) {
            Ok(value) => value,
            Err(error) => return toolError(tool, error),
        };

        let previousChatIds = match ChatHistoryManager::default() {
            Ok(manager) => match manager.chatHistoriesFlow() {
                Ok(histories) => histories
                    .into_iter()
                    .map(|chat| chat.id)
                    .collect::<Vec<_>>(),
                Err(error) => return toolError(tool, format!("Error loading chats: {error}")),
            },
            Err(error) => return toolError(tool, format!("Error opening chat history: {error}")),
        };

        match self.holder.lock() {
            Ok(mut holder) => {
                let core = holder.getCore(ChatRuntimeSlot::MAIN);
                core.createNewChat(characterCardName, group, false, setAsCurrentChat, None);
            }
            Err(error) => {
                return toolError(tool, format!("ChatRuntimeHolder lock failed: {error}"))
            }
        }

        match ChatHistoryManager::default().and_then(|manager| manager.chatHistoriesFlow()) {
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
    pub fn listChats(&self, tool: &AITool) -> ToolResult {
        match buildFilteredChatList(tool) {
            Ok((totalCount, currentChatId, chats)) => successData(
                tool,
                ToolResultData::ChatListResultData(ChatListResultData {
                    totalCount,
                    currentChatId,
                    chats,
                }),
            ),
            Err(error) => toolError(tool, error),
        }
    }

    #[allow(non_snake_case)]
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
        let histories = match manager.chatHistoriesFlow() {
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
                chat: Some(buildChatInfo(
                    &matched[targetIndex],
                    &messageCounts,
                    currentChatId.as_deref(),
                )),
            }),
        )
    }

    #[allow(non_snake_case)]
    pub fn agentStatus(&self, tool: &AITool) -> ToolResult {
        let chatId = parameterValue(tool, "chat_id");
        if chatId.trim().is_empty() {
            return toolError(tool, "Invalid parameter: missing chat_id".to_string());
        }
        let isProcessing = match self.holder.lock() {
            Ok(mut holder) => holder
                .cores
                .values_mut()
                .any(|core| core.activeStreamingChatIds().iter().any(|id| id == &chatId)),
            Err(error) => {
                return toolError(tool, format!("ChatRuntimeHolder lock failed: {error}"))
            }
        };
        successData(
            tool,
            ToolResultData::AgentStatusResultData(AgentStatusResultData {
                chatId,
                state: if isProcessing { "processing" } else { "idle" }.to_string(),
                message: None,
                isIdle: !isProcessing,
                isProcessing,
            }),
        )
    }

    #[allow(non_snake_case)]
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
        match self.holder.lock() {
            Ok(mut holder) => holder
                .getCore(ChatRuntimeSlot::MAIN)
                .switchChatLocal(chatId.clone()),
            Err(error) => {
                return toolError(tool, format!("ChatRuntimeHolder lock failed: {error}"))
            }
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
            if let Err(error) = resolveCharacterCardName(Some(roleCardId)) {
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
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())
            .and_then(|runtime| {
                runtime.block_on(async {
                    let mut holder = self
                        .holder
                        .lock()
                        .map_err(|error| format!("ChatRuntimeHolder lock failed: {error}"))?;
                    let core = holder.getCore(runtimeSlot);
                    core.enhancedAiService = Some(EnhancedAIService::new(ConversationService));
                    core.sendUserMessage(
                        PromptFunctionType::CHAT,
                        roleCardId,
                        chatId.clone(),
                        Some(message.clone()),
                        proxySenderName,
                        None,
                        None,
                        Vec::<AttachmentInfo>::new(),
                        None::<ChatMessage>,
                        turnOptions,
                    )
                    .await;
                    Ok::<(), String>(())
                })
            });
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
                aiResponse,
                receivedAt: Some(currentTimeMillis()),
                sentAt,
            }),
        )
    }

    #[allow(non_snake_case)]
    pub fn listCharacterCards(&self, tool: &AITool) -> ToolResult {
        match CharacterCardManager::getInstance().getAllCharacterCards() {
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
                    messages: messages
                        .into_iter()
                        .filter(|message| message.sender != "summary")
                        .map(|message| ChatMessageInfo {
                            sender: message.sender,
                            content: message.content,
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
}

impl ToolExecutor for ChatManagerToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        validateChatTool(self.operation, tool)
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
            ChatManagerToolOperation::ListCharacterCards => self.tools.listCharacterCards(tool),
            ChatManagerToolOperation::GetChatMessages => self.tools.getChatMessages(tool),
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
        ChatManagerToolOperation::SendMessageToAi
        | ChatManagerToolOperation::SendMessageToAiStreaming => {
            if parameterValue(tool, "message").trim().is_empty() {
                return invalid("message is required.");
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

fn parseRuntimeSlot(value: Option<&str>) -> Result<ChatRuntimeSlot, String> {
    match value.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if value == "main" => Ok(ChatRuntimeSlot::MAIN),
        Some(value) if value == "floating" || value.is_empty() => Ok(ChatRuntimeSlot::FLOATING),
        Some(_) => Err("Invalid parameter: runtime must be main/floating".to_string()),
        None => Ok(ChatRuntimeSlot::FLOATING),
    }
}

fn parseTurnOptions(tool: &AITool) -> Result<ChatTurnOptions, String> {
    Ok(ChatTurnOptions {
        persistTurn: parseOptionalBoolean(tool, "persist_turn")?.unwrap_or(true),
        notifyReply: parseOptionalBoolean(tool, "notify_reply")?,
        hideUserMessage: parseOptionalBoolean(tool, "hide_user_message")?.unwrap_or(false),
        disableWarning: parseOptionalBoolean(tool, "disable_warning")?.unwrap_or(false),
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
        .chatHistoriesFlow()
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
        characterCardName: chat.characterCardName.clone(),
    }
}

fn resolveCharacterCardName(cardId: Option<&str>) -> Result<Option<String>, String> {
    match cardId {
        Some(cardId) => CharacterCardManager::getInstance()
            .getCharacterCard(cardId)
            .map(|card| Some(card.name))
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
                .map(|message| message.content)
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
