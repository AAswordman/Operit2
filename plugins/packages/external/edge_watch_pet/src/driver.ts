export interface WatchConfig {
  version: 1;
  accent: "mint" | "peach" | "lilac";
  haptics: boolean;
  keepAwake: boolean;
}

export interface WatchTarget {
  nodeId: string;
  label: string;
}

const SERVICE = "edge.watchPet";

/** Enforces the version and settings vocabulary shared with the watch. */
export function parseConfig(data: unknown): WatchConfig {
  if (!data || typeof data !== "object") throw Error("手表设置格式无效");
  const { version, accent, haptics, keepAwake } = data as Partial<WatchConfig>;
  if (Object.keys(data).length !== 4 || version !== 1 || typeof haptics !== "boolean" ||
      typeof keepAwake !== "boolean" || (accent !== "mint" && accent !== "peach" && accent !== "lilac")) {
    throw Error("手表设置版本或字段不受支持");
  }
  return { version, accent, haptics, keepAwake };
}

/** Resolves an existing Space name or node reference through the standard Edge tool. */
export async function connectWatch(reference: unknown): Promise<{ target: WatchTarget; config: WatchConfig }> {
  if (typeof reference !== "string" || !/^[^\s\x00-\x1f\x7f]{1,128}$/u.test(reference.trim())) {
    throw Error("请输入不含空格的设备名称或节点 ID");
  }
  const label = reference.trim();
  const result = await toolCall("edge_execute", {
    node_id: label,
    interface_info: { target: SERVICE, method: "getConfig" },
    args: {},
  });
  if (typeof result.nodeId !== "string" || !result.nodeId) throw Error("设备空间未返回手表身份");
  return { target: { nodeId: result.nodeId, label }, config: parseConfig(result.data) };
}

/** Reads the live settings from the canonical device saved by the plugin. */
export async function readConfig(target: WatchTarget): Promise<WatchConfig> {
  const { data } = await Tools.device(target.nodeId, { timeoutMs: 12_000 }).Edge.execute({ target: SERVICE, method: "getConfig" }, {});
  return parseConfig(data);
}

/** Returns the watch's committed snapshot after a validated settings write. */
export async function writeConfig(target: WatchTarget, input: unknown): Promise<WatchConfig> {
  const config = parseConfig(input);
  const { data } = await Tools.device(target.nodeId, { timeoutMs: 12_000 }).Edge.execute({ target: SERVICE, method: "setConfig" }, config);
  return parseConfig(data);
}
