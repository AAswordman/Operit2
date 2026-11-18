/** Self-contained connectors.v1 contract for dependent ToolPkgs. */
export const CHANNELS = ["qq", "wechat", "feishu"] as const;
export type Channel = typeof CHANNELS[number];
export const PACKAGE = "com.operit.connector_hub";
export interface Conversation {
  key: string; channel: Channel; accountId: string; conversationId: string;
  scene: "c2c" | "group"; title: string; chatId: string | null; contextChatIds?: string[]; lastMessageAt: number;
}
export interface ChannelStatus {
  channel: Channel; service: "ready" | "connecting" | "stopped" | "error";
  configured?: boolean; listenerEnabled?: boolean; autoReplyEnabled?: boolean; error?: string;
  groupAutoReplyEnabled?: boolean; groupAutoReplyIssue?: string;
  capture: boolean; conversationIds: string[] | null; channelOnlyWhenNoAuto: boolean;
}
export interface ActiveContext {
  channel: Channel; chatId: string; scene?: Conversation["scene"];
  accountId: string; conversationId: string; authorId: string;
}
export interface Message {
  key: string; channel: Channel; accountId: string; messageId: string; conversationId: string;
  authorId: string; text: string; timestamp: string; autoReplyEnabled?: boolean; context: Record<string, unknown>;
}
export interface HistoryMessage {
  messageId: string; authorName: string; text: string; timestamp: string; receivedAt: number;
  replyKey?: string; direction?: "outgoing"; media?: { kind: Media["kind"]; name?: string; downloadable?: boolean }[];
  delivery?: "accepted" | "rejected" | "unknown";
}
export interface History { conversation: Conversation; messages: HistoryMessage[]; truncated: boolean; latestReplyKey?: string }
export interface Media {
  kind: "image" | "audio" | "video" | "file"; source: string; fileName?: string;
  cover?: string; durationMs?: number; voiceFormat?: number; sampleRate?: number;
}
export interface SendFields {
  text?: string; media?: Media; markdown?: { content: string }; post?: Record<string, unknown>; card?: Record<string, unknown>;
}
export type RouteRequest = { channel: Channel } & (
  | { action: "status" | "conversations" }
  | { action: "open_chat"; conversationKey: string }
  | { action: "history"; conversationKey: string; limit?: number; before?: number }
  | { action: "active_context"; chatId: string }
  | { action: "subscribe"; routes: { conversationKey: string; since: number }[] }
  | { action: "peek"; conversationKey: string; limit?: number }
  | { action: "ack"; key: string }
  | ({ action: "reply"; key: string } & SendFields)
  | ({ action: "send"; conversationKey: string } & SendFields)
  | { action: "configure"; capture: boolean; conversationIds: string[] | null; channelOnlyWhenNoAuto: boolean }
);
export interface Connector {
  channel: Channel; available: boolean; status?: ChannelStatus; issue?: string;
}
export interface Dashboard { connectors: Connector[]; conversations: Conversation[] }
export type Request = RouteRequest | { action: "dashboard" };
export interface Responses {
  dashboard: Dashboard;
  status: ChannelStatus; conversations: { conversations: Conversation[] }; open_chat: Conversation; history: History;
  active_context: ActiveContext | null; subscribe: { subscribed: number }; peek: { messages: Message[] };
  ack: { acknowledged: boolean }; reply: { response: null }; send: { response: null };
  configure: Pick<ChannelStatus, "capture" | "conversationIds" | "channelOnlyWhenNoAuto">;
}
export function request<R extends Request>(payload: R): Promise<Responses[R["action"]]> {
  return ToolPkg.callDependency<R, Responses[R["action"]]>(PACKAGE, "connectors.v1", payload);
}
