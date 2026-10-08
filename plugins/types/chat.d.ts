// Generated from operit-plugin-sdk Rust declarations.

import type { JsonObject } from "./core";
import type { AgentStatusResultData, ChatCallResultData, ChatCreationResultData, ChatDeleteResultData, ChatFindResultData, ChatListResultData, ChatMessagesResultData, ChatServiceStartResultData, ChatSwitchResultData, ChatTitleUpdateResultData, MessageSendResultData } from "./results";

/**
 * Starts the chat service and manages generic conversations and execution participants.
 */
export namespace Chat {
  /**
   * Identifies an existing conversation or an exact persisted message revision.
   */
  export type ExtensionTarget = { kind: "chat"; chatId: string; } | { kind: "message"; chatId: string; messageTimestamp: number; variantIndex: number; };

  /**
   * Configures workspace conversation creation without exposing plugin associations.
   */
  export interface CreateOptions {
    /**
     * Controls whether the newly persisted conversation becomes current.
     */
    setAsCurrentChat?: boolean;
    /**
     * Identifies an explicitly selected existing source conversation for creation hooks.
     */
    sourceChatId?: string | null;
    /**
     * Supplies opaque creation input interpreted solely by registered plugins.
     */
    input?: JsonObject | null;
  }

  /**
   * Describes one prompt turn supplied to a non-persistent functional model call.
   */
  export interface PromptTurn {
    /**
     * Identifies the prompt role.
     */
    kind: string;
    /**
     * Contains the prompt content.
     */
    content: string;
    /**
     * Identifies the tool associated with the turn.
     */
    toolName?: string;
    /**
     * Carries caller-defined prompt metadata.
     */
    metadata?: Record<string, unknown>;
  }

  /**
   * Configures one non-persistent functional model call.
   */
  export interface CallOptions {
    /**
     * Selects the configured functional model.
     */
    functionType: string;
    /**
     * Supplies the prompt turns sent to the functional model.
     */
    turns: PromptTurn[];
    /**
     * Controls whether provider token usage is recorded.
     */
    recordTokenUsage?: boolean;
    /**
     * Controls model thinking for this request.
     */
    enableThinking?: boolean;
  }

  /**
   * Selects how a chat-list query is matched against conversation metadata.
   */
  export type HostListChatsParamsMatch = "contains" | "exact" | "regex";

  /**
   * Selects the conversation attribute used to order chat-list results.
   */
  export type HostListChatsParamsSortBy = "updatedAt" | "createdAt" | "messageCount";

  /**
   * Controls whether chat-list results are returned in ascending or descending order.
   */
  export type HostListChatsParamsSortOrder = "asc" | "desc";

  /**
   * Configures filtering, ordering, and result limits when listing conversations.
   */
  export interface HostListChatsParams {
    /**
     * Contains the text or pattern used to filter conversations.
     */
    query?: string;
    /**
     * Selects how the query is compared with chat titles and identifiers.
     */
    match?: HostListChatsParamsMatch;
    /**
     * Limits the maximum number of conversations returned.
     */
    limit?: number;
    /**
     * Selects the conversation attribute used for sorting.
     */
    sort_by?: HostListChatsParamsSortBy;
    /**
     * Selects the direction in which the chosen attribute is sorted.
     */
    sort_order?: HostListChatsParamsSortOrder;
  }

  /**
   * Selects how a chat lookup query is matched against titles or identifiers.
   */
  export type HostFindChatParamsMatch = "contains" | "exact" | "regex";

  /**
   * Identifies one conversation by query, matching strategy, and occurrence index.
   */
  export interface HostFindChatParams {
    /**
     * Contains the title, identifier, or pattern to locate.
     */
    query: string;
    /**
     * Selects how the query is compared with candidate conversations.
     */
    match?: HostFindChatParamsMatch;
    /**
     * Selects one result when the query matches multiple conversations.
     */
    index?: number;
  }

  /**
   * Controls the chronological order of messages returned from a conversation.
   */
  export type HostGetMessagesOptionsOrder = "asc" | "desc";

  /**
   * Configures ordering and pagination when reading messages from a conversation.
   */
  export interface HostGetMessagesOptions {
    /**
     * Selects chronological or reverse-chronological message order.
     */
    order?: HostGetMessagesOptionsOrder;
    /**
     * Limits the maximum number of messages returned.
     */
    limit?: number;
  }

  /**
   * Configures ordering and inclusive index bounds when reading a message range.
   */
  export interface HostGetMessagesRangeOptions {
    /**
     * Selects chronological or reverse-chronological message order.
     */
    order?: HostGetMessagesOptionsOrder;
    /**
     * Selects the zero-based first message index.
     */
    start?: number;
    /**
     * Selects the zero-based last message index.
     */
    end?: number;
  }

  /**
   * Selects the initial presentation mode used when the chat service opens.
   */
  export type StartServiceOptionsInitialMode = "WINDOW" | "BALL" | "VOICE_BALL" | "FULLSCREEN" | "RESULT_DISPLAY" | "SCREEN_OCR";

  /**
   * Preserves the original submitted text, attachments and reply target for a native sequence.
   */
  export interface TurnInput {
    text: string;
    attachments: SendAttachment[];
    replyToMessageTimestamp: number | null;
  }

  /**
   * Selects real user-only persistence or one explicitly selected execution participant.
   */
  export type InitialTurn = { kind: "record_only"; } | { kind: "execute"; participantId?: string; };

  /**
   * Sends original input once or generates a reply to an explicitly identified persisted user message.
   */
  export type SendRequest = { kind: "submit"; chatId: string; runtime: Runtime; input: TurnInput; turn: InitialTurn; notifyReply: boolean; } | { kind: "continue"; chatId: string; runtime: Runtime; userMessageTimestamp: number; participantId: string; notifyReply: boolean; };

  /**
   * Describes one already parsed semantic message block, including its structured tool fields.
   */
  export interface MessagePart {
    partId: string;
    sequence: number;
    kind: MessagePartKind;
    content: string;
    toolCallId: string | null;
    toolName: string | null;
    attributes: Record<string, string>;
  }

  /**
   * Identifies semantic content without requiring plugins to parse provider markup.
   */
  export type MessagePartKind = "markdown" | "thinking" | "tool_call" | "tool_result" | "status";

  /**
   * Publishes atomic semantic snapshots followed by the actual finalized receipt.
   * A part event replaces the entire ordered part list, including removed or revised blocks.
   * Slow consumers observe the latest snapshot; snapshots are not an append-only chunk log.
   */
  export type SendEvent = { type: "part"; chatId: string; messageTimestamp: number; revision: number; parts: MessagePart[]; } | { type: "completed"; result: MessageSendResultData; };

  /**
   * Reports cancellation of this authenticated plugin's execution in the named chat.
   */
  export interface CancelResult {
    chatId: string;
    cancelRequested: boolean;
  }

  /**
   * Check chat input processing status
   */
  function agentStatus(chatId: string): Promise<AgentStatusResultData>;
  /**
   * Calls a configured functional model without adding a turn to chat history.
   * @since ToolPkg API 2.0.0
   */
  function call(options: CallOptions): Promise<ChatCallResultData>;
  /**
   * Cancels only the calling plugin's execution captured in the specified conversation.
   */
  function cancel(chatId: string): Promise<CancelResult>;
  /**
   * Creates a workspace conversation through the registered creation hooks.
   * @param options - Current-chat control, an explicit source chat, and opaque plugin input.
   * @returns The identity and timestamp of the actually persisted conversation.
   */
  function createNew(options?: CreateOptions): Promise<ChatCreationResultData>;
  /**
   * Delete a chat conversation by id
   */
  function deleteChat(chatId: string): Promise<ChatDeleteResultData>;
  /**
   * Deletes only the authenticated executing package's extension on an existing target.
   */
  function deleteExtension(target: ExtensionTarget): Promise<boolean>;
  /**
   * Find a chat by title or id
   */
  function findChat(params: HostFindChatParams): Promise<ChatFindResultData>;
  /**
   * Get messages from a specific chat
   * @param chatId - The ID of the chat to read
   * @param options - Optional order/limit
   */
  function getMessages(chatId: string, options?: HostGetMessagesOptions): Promise<ChatMessagesResultData>;
  /**
   * Gets an inclusive index range of messages from a specific chat.
   * @param chatId - The ID of the chat to read
   * @param options - The order and inclusive start/end indexes
   */
  function getMessagesRange(chatId: string, options?: HostGetMessagesRangeOptions): Promise<ChatMessagesResultData>;
  /**
   * List all chat conversations
   * @returns Promise resolving to the list of all chats
   */
  function listAll(): Promise<ChatListResultData>;
  /**
   * List chat conversations with filters
   */
  function listChats(params?: HostListChatsParams): Promise<ChatListResultData>;
  /**
   * Reads only the authenticated executing package's extension on an existing target.
   */
  function readExtension(target: ExtensionTarget): Promise<JsonObject | null>;
  /**
   * Sends one request and resolves only after its real receipt and native cleanup are complete.
   */
  function sendMessage(request: SendRequest): Promise<MessageSendResultData>;
  /**
   * Streams authoritative parsed part snapshots and the real terminal receipt through one async iterator.
   */
  function sendMessageStreaming(request: SendRequest): AsyncIterable<SendEvent>;
  /**
   * Start the chat service (floating window)
   * @param options - Optional service startup options
   * @returns Promise resolving to service start result
   */
  function startService(options?: StartServiceOptions): Promise<ChatServiceStartResultData>;
  /**
   * Stop the chat service runtime holder
   */
  function stopService(): Promise<ChatServiceStartResultData>;
  /**
   * Switch to a specific chat conversation
   * @param chatId - The ID of the chat to switch to
   * @returns Promise resolving to the chat switch result
   */
  function switchTo(chatId: string): Promise<ChatSwitchResultData>;
  /**
   * Update chat title
   */
  function updateTitle(chatId: string, title: string): Promise<ChatTitleUpdateResultData>;
  /**
   * Replaces only the authenticated executing package's object extension on an existing target.
   */
  function writeExtension(target: ExtensionTarget, value: JsonObject): Promise<JsonObject>;
  /**
   * Selects the application surface that owns a chat turn.
   */
  export type Runtime = "main" | "floating";

  /**
   * Configures how the chat service is launched and reused.
   */
  export interface StartServiceOptions {
    /**
     * Selects the UI mode shown when the service starts.
     */
    initial_mode?: StartServiceOptionsInitialMode;
    /**
     * Requests immediate entry into voice chat after startup.
     */
    auto_enter_voice_chat?: boolean;
    /**
     * Records that the service launch was initiated by a wake action.
     */
    wake_launched?: boolean;
    /**
     * Sets the maximum startup wait in milliseconds.
     */
    timeout_ms?: number;
    /**
     * Keeps an existing service instance instead of replacing it.
     */
    keep_if_exists?: boolean;
  }

  /**
   * Carries every attachment field that the host passes to submit hooks and the real input processor.
   */
  export interface SendAttachment {
    /**
     * Contains the original logical file path, not a reconstructed placeholder.
     */
    filePath: string;
    /**
     * Identifies the actual originating node or explicitly contains null.
     */
    nodeId?: string | null;
    /**
     * Contains the original attachment filename.
     */
    fileName: string;
    /**
     * Contains the actual MIME type.
     */
    mimeType: string;
    /**
     * Contains the attachment size in bytes.
     */
    fileSize: number;
    /**
     * Contains the original inline attachment payload.
     */
    content: string;
  }

  /**
   * Continues a real committed user turn without inserting or modifying the original submitted message.
   */
  export interface SendContinuation {
    /**
     * Identifies the existing user-message row in the explicitly selected conversation.
     */
    userMessageTimestamp: number;
  }

  /**
   * Controls persistence, presentation, and timing for one AI chat turn.
   */
  export interface SendMessageOptions {
    /**
     * Selects the runtime surface that processes the turn.
     */
    runtime?: Runtime;
    /**
     * Controls whether the user and assistant messages are saved to chat history.
     */
    persist_turn?: boolean;
    /**
     * Requests a user notification when the assistant reply is ready.
     */
    notify_reply?: boolean;
    /**
     * Prevents the submitted user message from being displayed in the conversation UI.
     */
    hide_user_message?: boolean;
    /**
     * Suppresses warning presentation for this turn.
     */
    disable_warning?: boolean;
    /**
     * Sets the maximum turn-processing time in milliseconds.
     */
    timeout_ms?: number;
    /**
     * Supplies complete original attachments only for an initial submit.
     */
    attachments?: SendAttachment[];
    /**
     * Identifies the real replied-to message on an initial submit.
     */
    replyToMessageTimestamp?: number;
    /**
     * Continues the identified committed user message; attachments, reply and replacement text are prohibited.
     */
    continuation?: SendContinuation;
  }

}
