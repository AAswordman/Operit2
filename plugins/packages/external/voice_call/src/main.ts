import screen from "./call.ui.js";

const CONTROL_ROUTE = "toolpkg:com.operit.voice_call:ui:control";
const title = { zh: "语音通话", en: "Voice Call" };

/** Publishes one launcher, shared by the toolbox and chat attachment menu. */
export function registerToolPkg(): boolean {
  ToolPkg.registerUiRoute({ id: "control", route: CONTROL_ROUTE, runtime: "compose_dsl", screen, title });
  ToolPkg.registerNavigationEntry({ id: "input-menu", route: CONTROL_ROUTE, surface: "chat_attachments",
    title: { zh: "语音/视频通话", en: "Voice / video call" }, icon: "Call", order: 50 });
  ToolPkg.registerNavigationEntry({ id: "control", route: CONTROL_ROUTE, title, surface: "toolbox", icon: "Call" });
  return true;
}
