import { getService } from "./service-runtime";
import { requireId } from "./validation";

/** Activates the opened conversation's bound actor and configuration through the shared selection service. */
export async function onChatViewTheme(event: ToolPkg.ChatViewHookEvent): Promise<void> {
  if (event.eventName !== "view_opened") return;
  const chatId = requireId(event.eventPayload.chatId, "opened chat id");
  await (await getService()).activateChatSelection(chatId);
}

/** Registers the existing generic navigation event without reading settings or opening plugin storage. */
export function registerChatViewTheme(): void {
  ToolPkg.registerChatViewHook({ id: "character-bound-theme", function: onChatViewTheme });
}
