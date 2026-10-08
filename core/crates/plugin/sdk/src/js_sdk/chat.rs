//! Chat service controls and conversation-management types exposed to plugins.
use super::core::JsonObject;
use super::results::*;
use super::{JsAsyncIterable, JsDate, JsFuture, JsNullable, JsOptional};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Identifies an existing conversation or an exact persisted message revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ChatExtensionTarget {
    /// Selects the executing owner's namespace on one existing conversation.
    #[serde(rename = "chat")]
    Chat { chatId: String },
    /// Selects revision zero or one explicitly identified persisted variant.
    #[serde(rename = "message")]
    Message {
        chatId: String,
        messageTimestamp: i64,
        variantIndex: i32,
    },
}

impl ChatExtensionTarget {
    /// Rejects empty identities and negative variant indexes before native storage access.
    pub fn validate(&self) -> Result<(), String> {
        let chatId = match self {
            Self::Chat { chatId } | Self::Message { chatId, .. } => chatId,
        };
        if chatId.trim().is_empty() {
            return Err("Chat extension target requires a nonempty chatId".to_string());
        }
        if let Self::Message { variantIndex, .. } = self {
            if *variantIndex < 0 {
                return Err("Chat extension variantIndex must be nonnegative".to_string());
            }
        }
        Ok(())
    }
}

/// Configures workspace conversation creation without exposing plugin associations.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatCreateOptions {
    /// Controls whether the newly persisted conversation becomes current.
    pub setAsCurrentChat: Option<bool>,
    /// Identifies an explicitly selected existing source conversation for creation hooks.
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    pub sourceChatId: JsOptional<String>,
    /// Supplies opaque creation input interpreted solely by registered plugins.
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    pub input: JsOptional<JsonObject>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Describes one prompt turn supplied to a non-persistent functional model call.
pub struct ChatPromptTurn {
    /// Identifies the prompt role.
    pub kind: String,
    /// Contains the prompt content.
    pub content: String,
    /// Identifies the tool associated with the turn.
    #[serde(rename = "toolName", skip_serializing_if = "Option::is_none")]
    pub toolName: Option<String>,
    /// Carries caller-defined prompt metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, serde_json::Value>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures one non-persistent functional model call.
pub struct ChatCallOptions {
    /// Selects the configured functional model.
    pub functionType: String,
    /// Supplies the prompt turns sent to the functional model.
    pub turns: Vec<ChatPromptTurn>,
    /// Controls whether provider token usage is recorded.
    pub recordTokenUsage: Option<bool>,
    /// Controls model thinking for this request.
    pub enableThinking: Option<bool>,
}
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
/// Configures ordering and inclusive index bounds when reading a message range.
pub struct ChatHostGetMessagesRangeOptions {
    /// Selects chronological or reverse-chronological message order.
    pub order: Option<ChatHostGetMessagesOptionsOrder>,
    /// Selects the zero-based first message index.
    pub start: Option<f64>,
    /// Selects the zero-based last message index.
    pub end: Option<f64>,
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
/// Preserves the original submitted text, attachments and reply target for a native sequence.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatTurnInput {
    pub text: String,
    pub attachments: Vec<ChatSendAttachment>,
    pub replyToMessageTimestamp: JsNullable<i64>,
}

/// Selects real user-only persistence or one explicitly selected execution participant.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChatInitialTurn {
    #[serde(rename = "record_only")]
    RecordOnly,
    #[serde(rename = "execute")]
    Execute { participantId: Option<String> },
}

/// Sends original input once or generates a reply to an explicitly identified persisted user message.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChatSendRequest {
    #[serde(rename = "submit")]
    Submit {
        chatId: String,
        runtime: ChatRuntime,
        input: ChatTurnInput,
        turn: ChatInitialTurn,
        notifyReply: bool,
    },
    #[serde(rename = "continue")]
    Continue {
        chatId: String,
        runtime: ChatRuntime,
        userMessageTimestamp: i64,
        participantId: String,
        notifyReply: bool,
    },
}

impl ChatSendRequest {
    /// Rejects malformed identities and ambiguous input before acquiring native execution ownership.
    pub fn validate(&self) -> Result<(), String> {
        let chatId = match self { Self::Submit { chatId, .. } | Self::Continue { chatId, .. } => chatId };
        if chatId.trim().is_empty() { return Err("Chat send requires a nonempty chatId".to_string()); }
        match self {
            Self::Submit { input, turn, .. } => {
                if input.text.trim().is_empty() && input.attachments.is_empty() {
                    return Err("Initial submission requires text or complete attachments".to_string());
                }
                if input.replyToMessageTimestamp.as_value().is_some_and(|timestamp| *timestamp <= 0) {
                    return Err("Reply target requires a positive committed timestamp".to_string());
                }
                for attachment in &input.attachments {
                    if attachment.fileName.trim().is_empty() || attachment.mimeType.trim().is_empty()
                        || attachment.fileSize < 0 || attachment.nodeId.as_value().is_some_and(|id| id.trim().is_empty()) {
                        return Err("Chat attachment requires its complete original identity and metadata".to_string());
                    }
                }
                if let ChatInitialTurn::Execute { participantId: Some(id) } = turn {
                    if id.trim().is_empty() { return Err("Execution participantId cannot be blank".to_string()); }
                }
            }
            Self::Continue { userMessageTimestamp, participantId, .. } => {
                if *userMessageTimestamp <= 0 || participantId.trim().is_empty() {
                    return Err("Continuation requires a committed user timestamp and participantId".to_string());
                }
            }
        }
        Ok(())
    }
}

/// Describes one already parsed semantic message block, including its structured tool fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ChatMessagePart {
    pub partId: String,
    pub sequence: i32,
    pub kind: ChatMessagePartKind,
    pub content: String,
    pub toolCallId: JsNullable<String>,
    pub toolName: JsNullable<String>,
    pub attributes: BTreeMap<String, String>,
}

/// Identifies semantic content without requiring plugins to parse provider markup.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatMessagePartKind {
    #[serde(rename = "markdown")]
    Markdown,
    #[serde(rename = "thinking")]
    Thinking,
    #[serde(rename = "tool_call")]
    ToolCall,
    #[serde(rename = "tool_result")]
    ToolResult,
    #[serde(rename = "status")]
    Status,
}

/// Publishes atomic semantic snapshots followed by the actual finalized receipt.
/// A part event replaces the entire ordered part list, including removed or revised blocks.
/// Slow consumers observe the latest snapshot; snapshots are not an append-only chunk log.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatSendEvent {
    #[serde(rename = "part")]
    Part { chatId: String, messageTimestamp: i64, revision: u64, parts: Vec<ChatMessagePart> },
    #[serde(rename = "completed")]
    Completed { result: MessageSendResultData },
}

/// Reports cancellation of this authenticated plugin's execution in the named chat.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ChatCancelResult { pub chatId: String, pub cancelRequested: bool }

/// Starts the chat service and manages generic conversations and execution participants.
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
    /// Creates a workspace conversation through the registered creation hooks.
    /// @param options - Current-chat control, an explicit source chat, and opaque plugin input.
    /// @returns The identity and timestamp of the actually persisted conversation.
    fn createNew(&self, options: Option<ChatCreateOptions>) -> JsFuture<ChatCreationResultData>;
    /// Reads only the authenticated executing package's extension on an existing target.
    fn readExtension(&self, target: ChatExtensionTarget) -> JsFuture<JsNullable<JsonObject>>;
    /// Replaces only the authenticated executing package's object extension on an existing target.
    fn writeExtension(
        &self,
        target: ChatExtensionTarget,
        value: JsonObject,
    ) -> JsFuture<JsonObject>;
    /// Deletes only the authenticated executing package's extension on an existing target.
    fn deleteExtension(&self, target: ChatExtensionTarget) -> JsFuture<bool>;
    ///
    ///List all chat conversations
    ///@returns Promise resolving to the list of all chats
    ///
    fn listAll(&self) -> JsFuture<ChatListResultData>;
    ///
    ///List chat conversations with filters
    ///
    fn listChats(&self, params: Option<ChatHostListChatsParams>) -> JsFuture<ChatListResultData>;
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
    fn updateTitle(&self, chatId: String, title: String) -> JsFuture<ChatTitleUpdateResultData>;
    ///
    ///Delete a chat conversation by id
    ///
    fn deleteChat(&self, chatId: String) -> JsFuture<ChatDeleteResultData>;
    /// Sends one request and resolves only after its real receipt and native cleanup are complete.
    fn sendMessage(&self, request: ChatSendRequest) -> JsFuture<MessageSendResultData>;
    /// Streams authoritative parsed part snapshots and the real terminal receipt through one async iterator.
    fn sendMessageStreaming(&self, request: ChatSendRequest) -> JsAsyncIterable<ChatSendEvent>;
    /// Cancels only the calling plugin's execution captured in the specified conversation.
    fn cancel(&self, chatId: String) -> JsFuture<ChatCancelResult>;
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
    /// Gets an inclusive index range of messages from a specific chat.
    /// @param chatId - The ID of the chat to read
    /// @param options - The order and inclusive start/end indexes
    ///
    fn getMessagesRange(
        &self,
        chatId: String,
        options: Option<ChatHostGetMessagesRangeOptions>,
    ) -> JsFuture<ChatMessagesResultData>;
    /// Calls a configured functional model without adding a turn to chat history.
    /// @since ToolPkg API 2.0.0
    fn call(&self, options: ChatCallOptions) -> JsFuture<ChatCallResultData>;
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
/// Carries every attachment field that the host passes to submit hooks and the real input processor.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatSendAttachment {
    /// Contains the original logical file path, not a reconstructed placeholder.
    pub filePath: String,
    /// Identifies the actual originating node or explicitly contains null.
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    pub nodeId: JsOptional<String>,
    /// Contains the original attachment filename.
    pub fileName: String,
    /// Contains the actual MIME type.
    pub mimeType: String,
    /// Contains the attachment size in bytes.
    pub fileSize: i64,
    /// Contains the original inline attachment payload.
    pub content: String,
}

/// Continues a real committed user turn without inserting or modifying the original submitted message.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatSendContinuation {
    /// Identifies the existing user-message row in the explicitly selected conversation.
    pub userMessageTimestamp: i64,
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
    /// Supplies complete original attachments only for an initial submit.
    pub attachments: Option<Vec<ChatSendAttachment>>,
    /// Identifies the real replied-to message on an initial submit.
    pub replyToMessageTimestamp: Option<i64>,
    /// Continues the identified committed user message; attachments, reply and replacement text are prohibited.
    pub continuation: Option<ChatSendContinuation>,

}
#[cfg(test)]
mod chat_extension_contract_tests {
    use super::{ChatCreateOptions, ChatExtensionTarget, JsOptional, JsonObject};
    use serde_json::json;

    /// Accepts only existing-record targets with an explicit zero-based message revision identity.
    #[test]
    fn accepts_exact_tagged_targets() {
        let chat: ChatExtensionTarget =
            serde_json::from_value(json!({"kind":"chat", "chatId":"chat-a"})).unwrap();
        chat.validate().unwrap();
        for variantIndex in [0, 1, 7] {
            let raw = json!({"kind":"message", "chatId":"chat-a", "messageTimestamp":123, "variantIndex":variantIndex});
            let target: ChatExtensionTarget = serde_json::from_value(raw.clone()).unwrap();
            target.validate().unwrap();
            assert_eq!(serde_json::to_value(target).unwrap(), raw);
        }
    }

    /// Rejects missing revisions, forged namespaces, unknown discriminators, and malformed target values.
    #[test]
    fn rejects_untyped_or_forged_targets() {
        for raw in [
            json!({"kind":"message", "chatId":"chat-a", "messageTimestamp":123}),
            json!({"kind":"chat", "chatId":"chat-a", "owner":"other"}),
            json!({"kind":"chat", "chatId":"chat-a", "namespace":"other"}),
            json!({"kind":"chat", "chatId":"chat-a", "pluginId":"other"}),
            json!({"kind":"variant", "chatId":"chat-a"}),
            json!({"kind":"message", "chatId":"chat-a", "messageTimestamp":123, "variantIndex":0.5}),
        ] {
            assert!(serde_json::from_value::<ChatExtensionTarget>(raw).is_err());
        }
        for raw in [
            json!({"kind":"chat", "chatId":" "}),
            json!({"kind":"message", "chatId":"chat-a", "messageTimestamp":123, "variantIndex":-1}),
        ] {
            assert!(serde_json::from_value::<ChatExtensionTarget>(raw)
                .unwrap()
                .validate()
                .is_err());
        }
    }

    /// Preserves omitted and explicit-null creation inputs without exposing role or group association parameters.
    #[test]
    fn preserves_creation_options_without_legacy_associations() {
        let omitted: ChatCreateOptions = serde_json::from_value(json!({})).unwrap();
        assert!(matches!(omitted.sourceChatId, JsOptional::Undefined));
        assert!(matches!(omitted.input, JsOptional::Undefined));
        let explicit: ChatCreateOptions =
            serde_json::from_value(json!({"sourceChatId":null, "input":null})).unwrap();
        assert!(matches!(explicit.sourceChatId, JsOptional::Null));
        assert!(matches!(explicit.input, JsOptional::Null));
        for key in ["group", "selection", "roleCardId", "characterCardId"] {
            let mut raw = json!({});
            raw[key] = json!("old-association");
            assert!(serde_json::from_value::<ChatCreateOptions>(raw).is_err());
        }
    }

    /// Keeps the SDK object representation lossless while rejecting non-object stored values.
    #[test]
    fn round_trips_complete_json_objects() {
        let raw = json!({"actor":{"id":"opaque"}, "nullable":null, "values":[1, false, "text"]});
        let value: JsonObject = serde_json::from_value(raw.clone()).unwrap();
        let canonical = serde_json::Value::Object(value.into_iter().collect());
        assert_eq!(canonical, raw);
        for value in [json!(null), json!([]), json!("not-an-object")] {
            assert!(serde_json::from_value::<JsonObject>(value).is_err());
        }
    }
}
