import Screen from "./ui/panel.ui.js";
import Conversation from "./ui/conversation.ui.js";
import { CONVERSATION_ROUTE } from "./ui/shared.js";
import { receive, registerService } from "./service.js";

registerService();

/** Public dependency entry uses the same validated owner as native tools. */
export async function agentRequest(event: ToolPkg.PublicApiEvent<unknown>) { return receive(event.payload); }

/** Registers the same platform-independent composer contribution for every supported chat input style. */
export function registerToolPkg(): boolean {
  ToolPkg.registerApi({ name: "agents.request", function: agentRequest });
  ToolPkg.registerUiRoute({ id: "conversation", route: CONVERSATION_ROUTE, runtime: "compose_dsl", screen: Conversation,
    keepAlive: true, title: { zh: "子代理会话", en: "Subagent conversation" } });
  ToolPkg.registerNavigationEntry({ id: "conversation", route: CONVERSATION_ROUTE, surface: "workspace",
    title: { zh: "子代理会话", en: "Subagent conversation" }, icon: "account_tree" });
  ToolPkg.registerChatComposerSlot({ id: "subagents", slot: "above_input", screen: Screen, order: 20, keepAlive: true });
  return true;
}

/** Proves that the package main module can execute without creating conversations or tasks. */
export function test_connection(): { passed: boolean } {
  return { passed: true };
}

/** Checks the actual Chat SDK through a read-only host operation without starting model generation. */
export async function test_tool_call(): Promise<{ passed: boolean; message: string }> {
  await Tools.Chat.listChats({ limit: 1 });
  return { passed: true, message: "Chat SDK read succeeded. Model execution requires an explicit spawn_agent task." };
}
