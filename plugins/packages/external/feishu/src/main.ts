import { request, restore, terminate } from "./service.js";
import type { ChannelRequest } from "./channel-api.js";

export async function channelRequest(event: ToolPkg.PublicApiEvent<ChannelRequest>): Promise<unknown> {
  const { payload, callerPackage } = event;
  if (payload.action === "attachment" && callerPackage !== "com.operit.connector_hub") throw Error("请在外部连接器中保存附件");
  if (payload.action === "manage" && callerPackage !== "com.operit.connector_hub") throw Error("请在外部连接器中管理飞书账户");
  if (payload.action === "subscribe" || payload.action === "peek" || payload.action === "ack") {
    return request({ ...payload, consumerId: callerPackage === "com.operit.connector_hub" ? (payload.consumerId ?? callerPackage) : callerPackage });
  }
  return request(payload);
}

export function registerToolPkg(): boolean {
  ToolPkg.registerApi({ name: "channel.v1", function: channelRequest });
  ToolPkg.registerAppLifecycleHook({ id: "feishu_startup", event: "application_on_create", function: restore });
  ToolPkg.registerAppLifecycleHook({ id: "feishu_foreground", event: "application_on_foreground", function: restore });
  ToolPkg.registerAppLifecycleHook({ id: "feishu_shutdown", event: "application_on_terminate", function: terminate });
  return true;
}
