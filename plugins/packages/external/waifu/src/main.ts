import { mergeInput } from "./merge.js";
import { inputMenu } from "./mode.js";
import { responseHook } from "./response.js";
import screen from "./ui/settings.ui.js";
import { admin } from "./admin.js";
import { settingsGet, settingsSet, sessionOpen, sessionFeed, sessionClose, composePrompt } from "./service.js";
export { settingsGet, settingsSet, sessionOpen, sessionFeed, sessionClose, composePrompt } from "./service.js";

ToolPkg.ipc.on("waifu.settings", admin);

/** Publishes the waifu protocol, prompt hook and native settings surface. */
export function registerToolPkg(): boolean {
  ToolPkg.registerInputMenuTogglePlugin({ id: "waifu_mode", function: inputMenu });
  ToolPkg.registerChatInputHook({ id: "waifu_merge_input", function: mergeInput });
  ToolPkg.registerChatResponseHook({ id: "waifu_reply", function: responseHook });
  ToolPkg.registerApi({ name: "settings.get", function: settingsGet });
  ToolPkg.registerApi({ name: "settings.set", function: settingsSet });
  ToolPkg.registerApi({ name: "session.open", function: sessionOpen });
  ToolPkg.registerApi({ name: "session.feed", function: sessionFeed });
  ToolPkg.registerApi({ name: "session.close", function: sessionClose });
  ToolPkg.registerSystemPromptComposeHook({ id: "waifu_prompt", function: composePrompt });
  ToolPkg.registerToolboxUiModule({ id: "waifu_settings", runtime: "compose_dsl", screen, params: {}, title: { zh: "Waifu 模式", en: "Waifu Mode" } });
  return true;
}
