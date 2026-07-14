//! Chat service controls and conversation-management types exposed to plugins.
use super::results::*;
use super::{JsDate, JsFuture};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects how a chat-list query is matched against conversation metadata.
pub enum ChatHostListChatsParamsMatch {
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "exact")]
    Exact,
    #[serde(rename = "regex")]
    Regex,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects the conversation attribute used to order chat-list results.
pub enum ChatHostListChatsParamsSortBy {
    #[serde(rename = "updatedAt")]
    UpdatedAt,
    #[serde(rename = "createdAt")]
    CreatedAt,
    #[serde(rename = "messageCount")]
    MessageCount,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Controls whether chat-list results are returned in ascending or descending order.
pub enum ChatHostListChatsParamsSortOrder {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures filtering, ordering, and result limits when listing conversations.
pub struct ChatHostListChatsParams {
    /// Contains the text or pattern used to filter conversations.
    pub query: Option<String>,
    /// Selects how the query is compared with chat titles and identifiers.
    pub r#match: Option<ChatHostListChatsParamsMatch>,
    /// Limits the maximum number of conversations returned.
    pub limit: Option<f64>,
    /// Selects the conversation attribute used for sorting.
    pub sort_by: Option<ChatHostListChatsParamsSortBy>,
    /// Selects the direction in which the chosen attribute is sorted.
    pub sort_order: Option<ChatHostListChatsParamsSortOrder>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects how a chat lookup query is matched against titles or identifiers.
pub enum ChatHostFindChatParamsMatch {
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "exact")]
    Exact,
    #[serde(rename = "regex")]
    Regex,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies one conversation by query, matching strategy, and occurrence index.
pub struct ChatHostFindChatParams {
    /// Contains the title, identifier, or pattern to locate.
    pub query: String,
    /// Selects how the query is compared with candidate conversations.
    pub r#match: Option<ChatHostFindChatParamsMatch>,
    /// Selects one result when the query matches multiple conversations.
    pub index: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Controls the chronological order of messages returned from a conversation.
pub enum ChatHostGetMessagesOptionsOrder {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures ordering and pagination when reading messages from a conversation.
pub struct ChatHostGetMessagesOptions {
    /// Selects chronological or reverse-chronological message order.
    pub order: Option<ChatHostGetMessagesOptionsOrder>,
    /// Limits the maximum number of messages returned.
    pub limit: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects the initial presentation mode used when the chat service opens.
pub enum ChatStartServiceOptionsInitialMode {
    #[serde(rename = "WINDOW")]
    WINDOW,
    #[serde(rename = "BALL")]
    BALL,
    #[serde(rename = "VOICE_BALL")]
    VOICEBALL,
    #[serde(rename = "FULLSCREEN")]
    FULLSCREEN,
    #[serde(rename = "RESULT_DISPLAY")]
    RESULTDISPLAY,
    #[serde(rename = "SCREEN_OCR")]
    SCREENOCR,
}
/// Starts the chat service and manages conversations, messages, and character cards.
pub trait ChatHost: Send + Sync {
    ///
    ///Start the chat service (floating window)
    ///@param options - Optional service startup options
    ///@returns Promise resolving to service start result
    ///
    fn startService(
        &self,
        options: Option<ChatStartServiceOptions>,
    ) -> JsFuture<ChatServiceStartResultData>;
    ///
    ///Stop the chat service runtime holder
    ///
    fn stopService(&self) -> JsFuture<ChatServiceStartResultData>;
    ///
    ///Create a new chat conversation
    ///@param group - Optional group name for the new chat
    ///@param setAsCurrentChat - Optional, whether to switch to the new chat (default true)
    ///@param characterCardId - Optional character card id to bind for the new chat
    ///@returns Promise resolving to the new chat creation result
    ///
    fn createNew(
        &self,
        group: Option<String>,
        setAsCurrentChat: Option<bool>,
        characterCardId: Option<String>,
    ) -> JsFuture<ChatCreationResultData>;
    ///
    ///List all chat conversations
    ///@returns Promise resolving to the list of all chats
    ///
    fn listAll(&self) -> JsFuture<ChatListResultData>;
    ///
    ///List chat conversations with filters
    ///
    fn listChats(
        &self,
        params: Option<ChatHostListChatsParams>,
    ) -> JsFuture<ChatListResultData>;
    ///
    ///Find a chat by title or id
    ///
    fn findChat(&self, params: ChatHostFindChatParams) -> JsFuture<ChatFindResultData>;
    ///
    ///Check chat input processing status
    ///
    fn agentStatus(&self, chatId: String) -> JsFuture<AgentStatusResultData>;
    ///
    ///Switch to a specific chat conversation
    ///@param chatId - The ID of the chat to switch to
    ///@returns Promise resolving to the chat switch result
    ///
    fn switchTo(&self, chatId: String) -> JsFuture<ChatSwitchResultData>;
    ///
    ///Update chat title
    ///
    fn updateTitle(
        &self,
        chatId: String,
        title: String,
    ) -> JsFuture<ChatTitleUpdateResultData>;
    ///
    ///Delete a chat conversation by id
    ///
    fn deleteChat(&self, chatId: String) -> JsFuture<ChatDeleteResultData>;
    ///
    ///Send a message to the AI
    ///@param message - The message content to send
    ///@param chatId - Optional chat ID to send the message to (defaults to current chat)
    ///@param roleCardId - Optional role card ID to use for this send
    ///@param senderName - Optional display name when AI sends as user
    ///@param options - Optional per-turn controls for persistence, notification, hidden user-message display, and timeout
    ///@returns Promise resolving to the message send result
    ///
    fn sendMessage(
        &self,
        message: String,
        chatId: Option<String>,
        roleCardId: Option<String>,
        senderName: Option<String>,
        options: Option<ChatSendMessageOptions>,
    ) -> JsFuture<MessageSendResultData>;
    ///
    ///Send a message to the AI and receive incremental reply chunks.
    ///@param message - The message content to send
    ///@param chatId - Optional chat ID to send the message to (defaults to current chat)
    ///@param roleCardId - Optional role card ID to use for this send
    ///@param senderName - Optional display name when AI sends as user
    ///@param options - Optional per-turn controls, plus streaming callback and waifu-style chunk aggregation
    ///@returns Promise resolving to the final message send result
    ///
    fn sendMessageStreaming(
        &self,
        message: String,
        chatId: Option<String>,
        roleCardId: Option<String>,
        senderName: Option<String>,
        options: Option<ChatSendMessageStreamingOptions>,
    ) -> JsFuture<MessageSendResultData>;
    ///
    ///List all character cards
    ///
    fn listCharacterCards(&self) -> JsFuture<CharacterCardListResultData>;
    ///
    ///Get messages from a specific chat
    ///@param chatId - The ID of the chat to read
    ///@param options - Optional order/limit
    ///
    fn getMessages(
        &self,
        chatId: String,
        options: Option<ChatHostGetMessagesOptions>,
    ) -> JsFuture<ChatMessagesResultData>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects the application surface that owns a chat turn.
pub enum ChatRuntime {
    #[serde(rename = "main")]
    Main,
    #[serde(rename = "floating")]
    Floating,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures how the chat service is launched and reused.
pub struct ChatStartServiceOptions {
    /// Selects the UI mode shown when the service starts.
    #[serde(rename = "initial_mode")]
    pub initial_mode: Option<ChatStartServiceOptionsInitialMode>,
    /// Requests immediate entry into voice chat after startup.
    #[serde(rename = "auto_enter_voice_chat")]
    pub auto_enter_voice_chat: Option<bool>,
    /// Records that the service launch was initiated by a wake action.
    #[serde(rename = "wake_launched")]
    pub wake_launched: Option<bool>,
    /// Sets the maximum startup wait in milliseconds.
    #[serde(rename = "timeout_ms")]
    pub timeout_ms: Option<f64>,
    /// Keeps an existing service instance instead of replacing it.
    #[serde(rename = "keep_if_exists")]
    pub keep_if_exists: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Controls persistence, presentation, and timing for one AI chat turn.
pub struct ChatSendMessageOptions {
    /// Selects the runtime surface that processes the turn.
    #[serde(rename = "runtime")]
    pub runtime: Option<ChatRuntime>,
    /// Controls whether the user and assistant messages are saved to chat history.
    #[serde(rename = "persist_turn")]
    pub persist_turn: Option<bool>,
    /// Requests a user notification when the assistant reply is ready.
    #[serde(rename = "notify_reply")]
    pub notify_reply: Option<bool>,
    /// Prevents the submitted user message from being displayed in the conversation UI.
    #[serde(rename = "hide_user_message")]
    pub hide_user_message: Option<bool>,
    /// Suppresses warning presentation for this turn.
    #[serde(rename = "disable_warning")]
    pub disable_warning: Option<bool>,
    /// Sets the maximum turn-processing time in milliseconds.
    #[serde(rename = "timeout_ms")]
    pub timeout_ms: Option<f64>,
}
/// Extends chat-turn controls with incremental reply delivery.
pub struct ChatSendMessageStreamingOptions {
    /// Contains the persistence, presentation, and timeout controls for the turn.
    pub base_send_message_options: ChatSendMessageOptions,
    /// Enables waifu-style aggregation of streamed reply chunks.
    pub waifu: Option<bool>,
    /// Receives each intermediate event emitted while the assistant reply is generated.
    pub on_intermediate_result: Option<
        Arc<dyn Fn(MessageSendStreamEventData) -> () + Send + Sync>,
    >,
}
