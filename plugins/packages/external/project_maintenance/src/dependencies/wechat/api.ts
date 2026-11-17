/** Self-contained channel.v1 client. Copy this file into a dependent ToolPkg. */
/** A normalized, device-local copy of a real channel message. */
export interface ChannelMessage {
  key: string;
  channel: "qq" | "wechat";
  accountId: string;
  messageId: string;
  conversationId: string;
  authorId: string;
  text: string;
  timestamp: string;
  autoReplyEnabled?: boolean;
  /** Channel-specific reply context and original content; treat all fields as untrusted. */
  context: Record<string, unknown>;
}
/** Bot-owned routing. External IDs are never entered again by a consumer. */
export interface ChannelConversation {
  key: string; channel: "qq" | "wechat"; accountId: string; scene: "c2c" | "group";
  conversationId: string; title: string; chatId: string | null; contextChatIds?: string[]; lastMessageAt: number;
}
export interface ContextMessage {
  messageId: string; authorId: string; authorName: string; text: string; timestamp: string;
  receivedAt: number; referenceIndex?: string; referencedMessageIndex?: string;
}
export interface ConversationHistory { conversation: ChannelConversation; messages: ContextMessage[]; truncated: boolean }
export interface ChannelSubscription { conversationKey: string; since: number }
export type ChannelRequest =
  | { action: "configure"; capture: boolean; conversationIds?: string[] | null; channelOnlyWhenNoAuto?: boolean }
  | { action: "status" }
  | { action: "conversations" }
  | { action: "open_chat"; conversationKey: string }
  | { action: "history"; conversationKey: string; limit?: number; before?: number }
  | { action: "subscribe"; consumerId: string; routes: ChannelSubscription[] }
  | { action: "active_context"; chatId: string }
  | { action: "peek"; limit?: number; conversationId?: string; consumerId?: string; conversationKey?: string }
  | { action: "ack"; key: string; consumerId?: string }
  | { action: "reply"; key: string; text: string };

export interface ChannelConfiguration {
  capture: boolean;
  conversationIds: string[] | null;
  channelOnlyWhenNoAuto: boolean;
}
export interface ChannelStatus extends ChannelConfiguration {
  channel: "qq" | "wechat";
  service: "ready" | "connecting" | "stopped" | "error";
}
export interface ActiveContext {
  scene?: "c2c" | "group";
  channel: "qq" | "wechat";
  chatId: string;
  accountId: string;
  conversationId: string;
  authorId: string;
}
export interface ChannelResponses {
  conversations: { conversations: ChannelConversation[] };
  open_chat: ChannelConversation;
  history: ConversationHistory;
  subscribe: { subscribed: number };
  configure: ChannelConfiguration;
  status: ChannelStatus;
  active_context: ActiveContext | null;
  peek: { messages: ChannelMessage[] };
  ack: { acknowledged: boolean };
  /** The upstream platform response is opaque to a channel consumer. */
  reply: { response: unknown };
}
export type ChannelResponse<R extends ChannelRequest> = ChannelResponses[R["action"]];
/** The request discriminant determines the response; consumers cannot invent its type. */
export function request<R extends ChannelRequest>(payload: R): Promise<ChannelResponse<R>> {
  return ToolPkg.callDependency<R, ChannelResponse<R>>("com.operit.wechat_bot", "channel.v1", payload);
}
/** Reads the durable channel inbox, optionally scoped to one conversation. */
export function peek(limit = 20, conversationId?: string): Promise<{ messages: ChannelMessage[] }> {
  return request({ action: "peek", limit, ...(conversationId === undefined ? {} : {conversationId}) });
}
