import OpenUiScreen from "./ui/index.ui.js";
import systemPrompt from "openui:prompt";

/** Adds the exact upstream library instructions only at the system-prompt composition boundary. */
export function onSystemPrompt(event: ToolPkg.SystemPromptComposeHookEvent): ToolPkg.PromptHookObjectResult | void {
  if (event.eventName !== "after_compose_system_prompt") return;
  const current = event.eventPayload.systemPrompt;
  if (typeof current !== "string") throw new Error("OpenUI requires the composed system prompt");
  return { systemPrompt: current + "\n\n" + systemPrompt };
}

/** Routes the plugin-owned XML tag into one persistent Compose DSL screen. */
export function onXmlRender(event: ToolPkg.XmlRenderHookEvent): ToolPkg.XmlRenderHookObjectResult {
  const payload = event.eventPayload;
  if (payload.tagName !== "openui") return { handled: false };
  if (typeof payload.xmlContent !== "string" || typeof payload.chatId !== "string") {
    throw new Error("OpenUI rendering requires XML content and its owning chat identity");
  }
  return {
    handled: true,
    composeDsl: {
      screen: OpenUiScreen,
      state: { xmlContent: payload.xmlContent, chatId: payload.chatId },
      memo: {},
    },
  };
}

/** Registers all OpenUI behavior through the public ToolPkg extension points. */
export function registerToolPkg(): boolean {
  ToolPkg.registerSystemPromptComposeHook({ id: "openui.prompt", function: onSystemPrompt });
  ToolPkg.registerXmlRenderPlugin({ id: "openui.render", tag: "openui", function: onXmlRender });
  return true;
}
