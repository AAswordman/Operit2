import settings from "./ui/settings.ui.js";
import { channel, registerService, restore, terminate } from "./service.js";
import type { ChannelRequest } from "./channel-api.js";

registerService();

/** Public, versioned channel contract for declared dependent plugins. */
export async function channelRequest(event: ToolPkg.PublicApiEvent<ChannelRequest>): Promise<unknown> { return channel(event.payload); }

/** Registers QQ Bot controls and the listener's application lifecycle. */
export function registerToolPkg(): boolean {
  ToolPkg.registerApi({ name: "channel.v1", function: channelRequest });
  ToolPkg.registerToolboxUiModule({ id: "qqbot_settings", runtime: "compose_dsl", screen: settings, params: {}, title: { zh: "QQ Bot 设置", en: "QQ Bot Settings" } });
  ToolPkg.registerAppLifecycleHook({ id: "qqbot_startup", event: "application_on_create", function: restore });
  ToolPkg.registerAppLifecycleHook({ id: "qqbot_foreground", event: "application_on_foreground", function: restore });
  ToolPkg.registerAppLifecycleHook({ id: "qqbot_shutdown", event: "application_on_terminate", function: terminate });
  return true;
}
