/** Self-contained maintenance client. Only the standard ToolPkg SDK is required. */
export type Stage = "new" | "triage" | "needs_info" | "not_actionable" | "implement" | "review" | "delivery" | "completed" | "interrupted" | "failed";
export interface Note { at: number; kind: string; text: string }
export interface Ticket {
  schemaVersion: 1; id: string; parentChatId: string; projectPath: string; source: string;
  feedback: string; stage: Stage; createdAt: number; updatedAt: number; agentId: string | null;
  attempt: number; notes: Note[]; error: string | null; autoStart?: boolean; forceTriage?: boolean; autoResumeAfterInfo?: boolean;
}
export interface CapturedMessage {
  key: string; channel: ExternalChannel | "app"; accountId: string; messageId: string;
  conversationId: string; authorId: string; text: string; timestamp: string; autoReplyEnabled?: boolean;
  context: Record<string, unknown>;
}
export interface ChatBinding { chatId: string; projectPath: string | null; since: number | null }
export interface AvailableChat { id: string; title: string; workspaceId: string | null; workspacePath: string | null }
export type Request =
  | { action: "dashboard" }
  | { action: "prepare_channel_chat"; channel: ExternalChannel; conversationKey: string }
  | { action: "list_bindings" }
  | { action: "available_chats" }
  | { action: "conversation_status"; chatId: string }
  | { action: "channel_status"; channel: ExternalChannel; parentChatId: string; projectPath: string; accountId: string; conversationId: string; authorId: string }
  | { action: "bind_chat"; chatId: string; enabled: boolean }
  | { action: "app_message"; chatId: string; timestamp: number; text: string }
  | { action: "channel_notifications"; channel: ExternalChannel; parentChatId: string; projectPath: string; conversationId: string; since: number }
  | { action: "capture"; parentChatId: string; projectPath: string; message: CapturedMessage }
  | { action: "create"; parentChatId: string; projectPath: string; source: string; feedback: string }
  | { action: "create_from_binding"; chatId: string; feedback: string }
  | { action: "list" }
  | { action: "get"; id: string }
  | { action: "inspect"; id: string }
  | { action: "start"; id: string }
  | { action: "add_note"; id: string; text: string };

export interface Inspection {
  ticketId: string; observedAt: number; claim: "this_ticket" | "other_ticket" | "none" | "unknown";
  agentId: string | null; managed: string; native: "processing" | "idle" | "unknown";
}

export interface ChannelSession {
  key: string; channel: ExternalChannel; accountId: string; scene: "c2c" | "group";
  conversationId: string; title: string; chatId: string | null; contextChatIds?: string[]; lastMessageAt: number;
  workspacePath: string | null; enabled: boolean; error: string;
}
export interface ChannelReport {
  availability: "available" | "unavailable";
  channel: ExternalChannel; enabled: boolean; service: "ready" | "connecting" | "stopped" | "error" | "unknown";
  sessions: ChannelSession[]; lastError: string; observedAt: number;
}
export interface FeedbackItem { key: string; channel: string; authorId: string; conversationId: string; projectPath: string; text: string; receivedAt: number; ticketId: string | null; ignored: string | null }

export interface ProjectSummary {
  path: string; tickets: number; active: number; attention: number; completed: number;
  updatedAt: number | null; chats: string[]; channels: ExternalChannel[];
}
export type ExternalChannel = "qq" | "wechat" | "feishu";
export interface Dashboard {
  observedAt: number; workspaceChats: AvailableChat[]; tickets: Ticket[]; bindings: ChatBinding[]; channels: ChannelReport[];
  feedbackTotal: number; feedback: FeedbackItem[]; projects: ProjectSummary[];
}
export interface TicketSummary { id: string; stage: Stage; updatedAt: number }
export interface ConversationStatus {
  bound: boolean; total: number; counts: Record<string, number>;
  tickets: (TicketSummary & {gitVerified: boolean; verifiedPullRequest: string | null})[];
}
export interface ChannelTicketStatus { pending: number; total: number; counts: Record<string, number>; tickets: TicketSummary[] }
export interface ChannelNotice { eventId: string; messageKey: string; text: string }
export interface Responses {
  dashboard: Dashboard;
  prepare_channel_chat: {chatId: string};
  list_bindings: ChatBinding[];
  available_chats: AvailableChat[];
  conversation_status: ConversationStatus;
  channel_status: ChannelTicketStatus;
  bind_chat: { chatId: string; projectPath: string | null };
  app_message: { saved: boolean };
  channel_notifications: ChannelNotice[];
  capture: { saved: boolean };
  create: Ticket;
  create_from_binding: Ticket;
  list: Ticket[];
  get: Ticket;
  inspect: Inspection;
  start: Ticket;
  add_note: Ticket;
}
export type Response<R extends Request> = Responses[R["action"]];
/** Calls the provider's explicit public method with an inferred response type. */
export function request<R extends Request>(payload: R): Promise<Response<R>> {
  return ToolPkg.callDependency<R, Response<R>>("com.operit.project_maintenance", "maintenance.request", payload);
}
