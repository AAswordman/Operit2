import settings from "./ui/settings.ui.js";
import { registerService, restore, terminate } from "./service.js";

registerService();

/** Registers QQ Bot controls and the listener's application lifecycle. */
export function registerToolPkg(): boolean {
  ToolPkg.registerToolboxUiModule({ id: "qqbot_settings", runtime: "compose_dsl", screen: settings, params: {}, title: { zh: "QQ Bot 设置", en: "QQ Bot Settings" } });
  ToolPkg.registerAppLifecycleHook({ id: "qqbot_startup", event: "application_on_create", function: restore });
  ToolPkg.registerAppLifecycleHook({ id: "qqbot_foreground", event: "application_on_foreground", function: restore });
  ToolPkg.registerAppLifecycleHook({ id: "qqbot_shutdown", event: "application_on_terminate", function: terminate });
  return true;
}
