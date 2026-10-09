export {
  onChatInput,
  onChatViewEvent,
  onGoalCommand,
  onInputMenuToggle,
  onPromptFinalize,
  registerToolPkg,
} from "./plugin/goal_mode_plugin.js";

/** Connectivity proves the existing plugin main runtime actually executed this function. */
export function test_connection(): { passed: boolean } {
  return { passed: true };
}

/** Never invoke state-changing hooks or fabricate a tool success for a UI-only plugin. */
export function test_tool_call(): { passed: boolean; message: string } {
  return { passed: false, message: "此插件仅提供界面或聊天钩子，没有业务工具" };
}
