import screen from "./ui";
import { cancelChat, composePrompt, inputHook, openOriginal, publish, removeRule, saveRule, snapshot } from "./service";

RuntimeContext.register({ cancelChat, openOriginal, removeRule, saveRule, snapshot });
ToolPkg.ipc.on("autonomous_reply.publish", publish);
export { inputHook, composePrompt } from "./service";

export function registerToolPkg(): boolean {
  ToolPkg.registerToolboxUiModule({ id: "proactive_messages_settings", runtime: "compose_dsl", screen,
    params: {}, title: { zh: "AI 自主回复", en: "AI Autonomous Replies" } });
  ToolPkg.registerChatInputHook({ id: "autonomous_reply_input", function: inputHook });
  ToolPkg.registerSystemPromptComposeHook({ id: "autonomous_reply_prompt", function: composePrompt });
  return true;
}
