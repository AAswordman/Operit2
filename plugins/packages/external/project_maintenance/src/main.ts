import { channelConversation } from "./channel-conversation.js";
import screen from "./ui/screen.js";
import { receive, registerService } from "./service.js";
import type { Request } from "./model.js";
import { maintenanceConversation } from "./conversation.js";
import { call } from "./client.js";
const ROUTE = "toolpkg:com.operit.project_maintenance:ui:dashboard";
registerService();
export async function maintenanceRequest(event: ToolPkg.PublicApiEvent<Request>): Promise<unknown> { return receive(event.payload); }
/** Only persisted user messages in an explicitly bound chat become feedback. */
export async function onFeedbackMessage(event: ToolPkg.ChatMessageHookEvent): Promise<void> {
  const p = event.eventPayload;
  if (event.eventName !== "message_persisted" || p.sender !== "user" || typeof p.chatId !== "string" || typeof p.timestamp !== "number" || typeof p.content !== "string") return;
  try { await call({action: "app_message", chatId: p.chatId, timestamp: p.timestamp, text: p.content}); }
  catch (error) { console.error("[project_maintenance.app_capture]", error instanceof Error ? error.message : String(error)); }
}
export function registerToolPkg(): boolean {
  ToolPkg.registerPromptFinalizeHook({id:"maintenance_channel_conversation", function:channelConversation});
  ToolPkg.registerChatMessageHook({ id: "maintenance_app_feedback", function: onFeedbackMessage });
  ToolPkg.registerPromptFinalizeHook({ id: "maintenance_conversation", function: maintenanceConversation });
  ToolPkg.registerApi({ name: "maintenance.request", function: maintenanceRequest });
  ToolPkg.registerUiRoute({ id: "dashboard", route: ROUTE, runtime: "compose_dsl", screen, keepAlive: true, title: { zh: "维护中控", en: "Maintenance control" } });
  ToolPkg.registerNavigationEntry({ id: "maintenance_sidebar", route: ROUTE, surface: "main_sidebar_plugins", title: { zh: "维护中控", en: "Maintenance control" }, icon: "Build", order: 145 });
  return true;
}
