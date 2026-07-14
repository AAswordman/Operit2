// Generated from operit-plugin-sdk Rust declarations.

import type { AgentStatusResultData, CharacterCardListResultData, ChatCreationResultData, ChatDeleteResultData, ChatFindResultData, ChatListResultData, ChatMessagesResultData, ChatServiceStartResultData, ChatSwitchResultData, ChatTitleUpdateResultData, MessageSendResultData, MessageSendStreamEventData } from "./results";

/**
 * Starts the chat service and manages conversations, messages, and character cards.
 */
export namespace Chat {
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
   * Selects the initial presentation mode used when the chat service opens.
   */
  export type StartServiceOptionsInitialMode = "WINDOW" | "BALL" | "VOICE_BALL" | "FULLSCREEN" | "RESULT_DISPLAY" | "SCREEN_OCR";

  /**
   * Check chat input processing status
   */
  function agentStatus(chatId: string): Promise<AgentStatusResultData>;
  /**
   * Create a new chat conversation
   * @param group - Optional group name for the new chat
   * @param setAsCurrentChat - Optional, whether to switch to the new chat (default true)
   * @param characterCardId - Optional character card id to bind for the new chat
   * @returns Promise resolving to the new chat creation result
   */
  function createNew(group?: string, setAsCurrentChat?: boolean, characterCardId?: string): Promise<ChatCreationResultData>;
  /**
   * Delete a chat conversation by id
   */
  function deleteChat(chatId: string): Promise<ChatDeleteResultData>;
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
   * List all chat conversations
   * @returns Promise resolving to the list of all chats
   */
  function listAll(): Promise<ChatListResultData>;
  /**
   * List all character cards
   */
  function listCharacterCards(): Promise<CharacterCardListResultData>;
  /**
   * List chat conversations with filters
   */
  function listChats(params?: HostListChatsParams): Promise<ChatListResultData>;
  /**
   * Send a message to the AI
   * @param message - The message content to send
   * @param chatId - Optional chat ID to send the message to (defaults to current chat)
   * @param roleCardId - Optional role card ID to use for this send
   * @param senderName - Optional display name when AI sends as user
   * @param options - Optional per-turn controls for persistence, notification, hidden user-message display, and timeout
   * @returns Promise resolving to the message send result
   */
  function sendMessage(message: string, chatId?: string, roleCardId?: string, senderName?: string, options?: SendMessageOptions): Promise<MessageSendResultData>;
  /**
   * Send a message to the AI and receive incremental reply chunks.
   * @param message - The message content to send
   * @param chatId - Optional chat ID to send the message to (defaults to current chat)
   * @param roleCardId - Optional role card ID to use for this send
   * @param senderName - Optional display name when AI sends as user
   * @param options - Optional per-turn controls, plus streaming callback and waifu-style chunk aggregation
   * @returns Promise resolving to the final message send result
   */
  function sendMessageStreaming(message: string, chatId?: string, roleCardId?: string, senderName?: string, options?: SendMessageStreamingOptions): Promise<MessageSendResultData>;
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
  }

  /**
   * Extends chat-turn controls with incremental reply delivery.
   */
  export interface SendMessageStreamingOptions {
    /**
     * Contains the persistence, presentation, and timeout controls for the turn.
     */
    base_send_message_options: SendMessageOptions;
    /**
     * Enables waifu-style aggregation of streamed reply chunks.
     */
    waifu?: boolean;
    /**
     * Receives each intermediate event emitted while the assistant reply is generated.
     */
    on_intermediate_result?: (arg0: MessageSendStreamEventData) => void;
  }

}
