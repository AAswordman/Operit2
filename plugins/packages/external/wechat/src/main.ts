import settings from "./ui/settings.ui.js";
import { registerService, restore, terminate } from "./service.js";

registerService();

/** Registers WeChat settings and the application-owned Bot lifecycle. */
export function registerToolPkg(): boolean {
  ToolPkg.registerToolboxUiModule({
    id: "wechat_settings",
    runtime: "compose_dsl",
    screen: settings,
    params: {},
    title: { zh: "微信 Bot 设置", en: "WeChat Bot Settings" },
  });
  ToolPkg.registerAppLifecycleHook({
    id: "wechat_startup",
    event: "application_on_create",
    function: restore,
  });
  ToolPkg.registerAppLifecycleHook({
    id: "wechat_foreground",
    event: "application_on_foreground",
    function: restore,
  });
  ToolPkg.registerAppLifecycleHook({
    id: "wechat_shutdown",
    event: "application_on_terminate",
    function: terminate,
  });
  return true;
}
