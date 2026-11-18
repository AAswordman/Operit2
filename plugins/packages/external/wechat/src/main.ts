import settings from "./ui/settings.ui.js";
import { channel, registerService, restore, terminate } from "./service.js";
import type { ChannelRequest } from "./channel-api.js";

registerService();

/** Public, versioned channel contract for declared dependent plugins. */
export async function channelRequest({payload, callerPackage}: ToolPkg.PublicApiEvent<ChannelRequest>): Promise<unknown> {
  if (payload.action === "attachment" && callerPackage !== "com.operit.connector_hub") throw Error("请在外部连接器中保存附件");
  if (payload.action === "manage" && callerPackage !== "com.operit.connector_hub") throw Error("请在外部连接器中管理微信账户");
  if ((payload.action === "subscribe" || payload.action === "peek" || payload.action === "ack") && callerPackage !== "com.operit.connector_hub" && payload.consumerId !== undefined) {
    return channel({ ...payload, consumerId: callerPackage });
  }
  return channel(payload);
}

/** Registers WeChat settings and the application-owned Bot lifecycle. */
export function registerToolPkg(): boolean {
  ToolPkg.registerApi({ name: "channel.v1", function: channelRequest });
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
