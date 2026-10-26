import Screen from "./ui/panel.ui.js";
import { registerService } from "./service.js";

registerService();

/** Registers the same platform-independent composer contribution for every supported chat input style. */
export function registerToolPkg(): boolean {
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
