import { PACKAGE } from "./dependencies/connector_hub/api.js";

/** Reads the loaded registry instead of probing an absent optional provider through IPC. */
export async function connectorIssue(): Promise<string | null> {
  const plugins: unknown = JSON.parse(await Tools.SoftwareSettings.exec(["--json", "plugin", "list"]));
  if (!Array.isArray(plugins) || !plugins.every(plugin => plugin !== null && typeof plugin === "object" &&
      typeof plugin.name === "string" && typeof plugin.enabled === "boolean")) throw Error("软件返回了无效的插件列表");
  const connector = plugins.find(plugin => plugin.name === PACKAGE);
  if (!connector) return "请先安装并启用「外部连接器」。";
  if (!connector.enabled) return "请先启用「外部连接器」。";
  return null;
}
