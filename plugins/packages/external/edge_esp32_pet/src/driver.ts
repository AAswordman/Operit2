import { PACKS } from './assets';
import { sceneCall } from './protocol';
import type { Mood } from './service';

export interface Esp32Target { nodeId: string; label: string; }
interface Node { nodeId: string; displayName: string; platform: string; model: string; reachable: boolean; relayHops?: number; }
export interface ConnectedDevice { target: Esp32Target; model: string; }

async function nodes(): Promise<Node[]> {
  const state = JSON.parse(await toolCall('list_core_nodes', {}));
  if (!state || typeof state.currentNodeId !== 'string' || !Array.isArray(state.nodes)) throw Error('设备空间列表无效');
  const seen = new Set<string>();
  return state.nodes.filter((node: Node) => {
    if (!node || typeof node.nodeId !== 'string' || typeof node.displayName !== 'string' ||
        typeof node.model !== 'string' || typeof node.platform !== 'string' || typeof node.reachable !== 'boolean') throw Error('设备信息无效');
    if (node.nodeId === state.currentNodeId || !node.reachable || !node.platform.startsWith('esp32') ||
        (node.relayHops !== undefined && node.relayHops !== 1) || seen.has(node.nodeId)) return false;
    seen.add(node.nodeId); return true;
  });
}

/** Verify the live native service before offering a device. */
async function inspect(node: Node): Promise<ConnectedDevice> {
  const caps = await sceneCall(node.nodeId, 'capabilities');
  if (caps.protocol !== 1 || caps.configuration?.transient !== true || caps.animation?.local !== true || caps.formats?.pack !== 'ESP1' || caps.formats?.compressedAsset !== 'ESR1' ||
      !(caps.limits?.packBytes >= Math.max(...PACKS.map(pack => pack.bytes)) && caps.limits?.cacheBytes >= 16384 && caps.limits?.packs >= 2 &&
        caps.animation?.maxFrames >= 15 && caps.formats?.paletteMax >= 33 && caps.limits?.assetsPerPack >= 1 &&
        caps.screen?.width >= 231 && caps.screen?.height - caps.reserved?.h >= 201)) throw Error('设备场景接口或容量不支持任务猫');
  return { target: { nodeId: node.nodeId, label: node.displayName }, model: node.model };
}

export async function discoverDevices(): Promise<{ devices: ConnectedDevice[]; issues: { label: string; message: string }[] }> {
  const candidates = await nodes(), devices: ConnectedDevice[] = [], issues: { label: string; message: string }[] = [];
  for (let i = 0; i < candidates.length; i += 4) {
    const batch = candidates.slice(i, i + 4);
    const results = await Promise.allSettled(batch.map(inspect));
    results.forEach((result, index) => result.status === 'fulfilled' ? devices.push(result.value) :
      issues.push({ label: batch[index].displayName, message: String(result.reason) }));
  }
  return { devices, issues };
}

export async function connectDevice(nodeId: unknown): Promise<ConnectedDevice> {
  const node = (await nodes()).find(node => node.nodeId === nodeId);
  if (!node) throw Error('设备已离线，请先在设备空间配对并加入同一空间');
  return inspect(node);
}

function owned(id: string): boolean {
  return PACKS.some(pack => pack.id === id) || id === 'task-cat-v1-0' || id === 'task-cat-v1-1';
}

/** Upload the next mood, swap scenes, then reclaim our previous pack. */
export async function sendMood(target: Esp32Target, mood: Mood): Promise<void> {
  const pack = PACKS.find(pack => (pack.moods as readonly string[]).includes(mood));
  if (!pack) throw Error('未知任务状态');
  const caps = await sceneCall(target.nodeId, 'capabilities');
  if (caps.formats?.compressedAsset !== 'ESR1') throw Error('请更新 ESP32 固件以支持原版任务猫动画');
  const cached = caps.cache.packs.find((entry: { id: string; sha256: string }) => entry.id === pack.id && entry.sha256 === pack.sha256);
  const removed = new Set<string>();
  if (!cached) {
    const config = await sceneCall(target.nodeId, 'config.read');
    for (const entry of caps.cache.packs) {
      if (owned(entry.id) && !config.layers?.some((layer: {asset: string}) => layer.asset.startsWith(`${entry.id}:`))) {
        await sceneCall(target.nodeId, 'pack.remove', { packId: entry.id }); removed.add(entry.id);
      }
    }
    const upload = await sceneCall(target.nodeId, 'pack.begin', { packId: pack.id, bytes: pack.bytes, sha256: pack.sha256 });
    try {
      for (const chunk of pack.chunks) await sceneCall(target.nodeId, 'pack.chunk', { upload: upload.upload, ...chunk });
      await sceneCall(target.nodeId, 'pack.commit', { upload: upload.upload });
    } catch (error) {
      try { await sceneCall(target.nodeId, 'pack.abort', { upload: upload.upload }); } catch { /* preserve upload error */ }
      throw error;
    }
  }
  const rectHeight = caps.screen.height - caps.reserved.h;
  const scale = Math.min(8, Math.floor(caps.screen.width / pack.width), Math.floor(rectHeight / pack.height));
  await sceneCall(target.nodeId, 'config.show', { enabled: true, background: 0x1083,
    layers: [{ asset: `${pack.id}:cat`, x: Math.floor(caps.screen.width / 2), y: Math.floor(rectHeight / 2),
      scale, anchor: 'center', play: true, loop: true }] });
  for (const entry of caps.cache.packs) {
    if (entry.id !== pack.id && owned(entry.id) && !removed.has(entry.id)) {
      try { await sceneCall(target.nodeId, 'pack.remove', { packId: entry.id }); }
      catch (error) { console.warn('[esp32-pet] reclaim', error); }
    }
  }
}

/** Clear only scenes owned by this plugin. */
export async function releaseScene(target: Esp32Target): Promise<void> {
  const config = await sceneCall(target.nodeId, 'config.read');
  if (config.layers?.length && config.layers.every((layer: { asset: string }) =>
    owned(layer.asset.split(':')[0]))) await sceneCall(target.nodeId, 'config.clear');
}
