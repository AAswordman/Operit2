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

export interface ConnectedWatch { target: WatchTarget; config: WatchConfig; model: string; }
export interface WatchDiscovery {
  watches: ConnectedWatch[];
  issues: { label: string; message: string }[];
}
interface SpaceNode { nodeId: string; displayName: string; platform: string; model: string; reachable: boolean; relayHops?: number; }

const SERVICE = "edge.watchPet";

/** Sends one typed task mood through the watch's existing robot-face service. */
export async function sendMood(target: WatchTarget, mood: import("./service").Mood): Promise<void> {
  await Tools.device(target.nodeId, { timeoutMs: 12_000 }).Edge.execute({ target: "edge.robotFace", method: "setExpression" }, { expression: mood });
}

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

/** Reads current Space membership and limits probing to directly reachable Android Edge nodes. */
async function watchNodes(): Promise<SpaceNode[]> {
  const state = JSON.parse(await toolCall("list_core_nodes", {}));
  if (!state || typeof state.currentNodeId !== "string" || !Array.isArray(state.nodes)) throw Error("设备空间返回的设备列表无效");
  const nodes: SpaceNode[] = [];
  const seen = new Set<string>();
  for (const node of state.nodes) {
    if (!node || typeof node.nodeId !== "string" || !node.nodeId || typeof node.displayName !== "string" ||
        typeof node.platform !== "string" || typeof node.model !== "string" || typeof node.reachable !== "boolean" ||
        (node.relayHops !== undefined && (!Number.isInteger(node.relayHops) || node.relayHops < 1))) throw Error("设备空间返回的设备信息无效");
    if (node.nodeId === state.currentNodeId || !node.reachable || node.platform !== "android-edge" ||
        (node.relayHops !== undefined && node.relayHops > 1) || seen.has(node.nodeId)) continue;
    seen.add(node.nodeId);
    nodes.push(node);
  }
  return nodes;
}

/** Verifies the actual watch service instead of inferring capability from a device name. */
async function inspectWatch(node: SpaceNode): Promise<ConnectedWatch> {
  const target = { nodeId: node.nodeId, label: node.displayName };
  return { target, config: await readConfig(target), model: node.model };
}

/** Probes eligible devices in bounded batches and reports each failed capability check. */
export async function discoverWatches(): Promise<WatchDiscovery> {
  const nodes = await watchNodes();
  const watches: ConnectedWatch[] = [], issues: WatchDiscovery["issues"] = [];
  for (let offset = 0; offset < nodes.length; offset += 4) {
    const batch = nodes.slice(offset, offset + 4);
    const results = await Promise.allSettled(batch.map(inspectWatch));
    results.forEach((result, index) => {
      if (result.status === "fulfilled") watches.push(result.value);
      else issues.push({ label: batch[index].displayName, message: result.reason instanceof Error ? result.reason.message : String(result.reason) });
    });
  }
  return { watches, issues };
}

/** Accepts only an exact identity still present in the live eligible device list. */
export async function connectWatch(nodeId: unknown): Promise<ConnectedWatch> {
  if (typeof nodeId !== "string" || !nodeId) throw Error("请选择已识别的手表");
  const node = (await watchNodes()).find(node => node.nodeId === nodeId);
  if (!node) throw Error("这块手表已断开，请刷新设备列表");
  return inspectWatch(node);
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
