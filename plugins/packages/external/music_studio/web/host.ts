import { createPreviewStorage } from "./preview-storage";
import type { LiveControl } from "./live-control";
import { StudioService } from "../src/service";
import type { Request, Snapshot, Project } from "../src/shared/model";
declare global {
  interface Window {
    musicStudioControl?: LiveControl;
    MusicHost?: { getTheme(): Promise<{ brightness: string }>; request(request: Request): Promise<unknown>; saveFile(name: string, base64: string): Promise<unknown> };
    __musicTest?: { audition(): unknown; navigation(): unknown; editor(): unknown; project(): Project; snapshot(): Promise<Snapshot>; request(request: Request): Promise<unknown>; render(includeAudio?: boolean): Promise<{ peak: number; bytes: number; dropped: number; audio?: string }>; play(): Promise<void>; stop(): void; state(): unknown };
  }
}
/** Browser preview uses the same transaction service, but never masquerades as a host connection. */
const preview = new StudioService(createPreviewStorage());
export const hostMode = (): boolean => Boolean(window.MusicHost);
export async function request<T = Snapshot>(value: Request): Promise<T> {
  if (window.MusicHost) return await window.MusicHost.request(value) as T;
  // A packaged studio must not fork into browser storage while the bridge is attaching.
  if (!/^(localhost|127\.0\.0\.1|\[::1\])$/.test(location.hostname)) throw new Error("MusicHost 桥接尚未连接，请重新打开插件页面；工程未写入浏览器临时存储。");
  return await preview.request(value) as T;
}
export function base64(bytes: Uint8Array): string { let result = ""; for (let i = 0; i < bytes.length; i += 8192) result += String.fromCharCode(...bytes.subarray(i, i + 8192)); return btoa(result); }
export async function saveFile(name: string, bytes: Uint8Array): Promise<string> {
  if (window.MusicHost) {
    const result = await window.MusicHost.saveFile(name, base64(bytes)) as { cancelled?: unknown; path?: unknown } | null;
    if (!result || typeof result.cancelled !== "boolean") throw new Error("宿主未返回有效保存结果，请更新软件并重新打开工作台");
    if (result.cancelled) throw new Error("已取消导出，未保存文件");
    if (typeof result.path !== "string" || !result.path.trim()) throw new Error("宿主未返回保存路径，文件导出未确认");
    return result.path;
  }
  const url = URL.createObjectURL(new Blob([bytes as Uint8Array<ArrayBuffer>], { type: name.endsWith(".wav") ? "audio/wav" : "application/json" }));
  const a = document.createElement("a"); a.href = url; a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(url), 30000); return `浏览器下载：${name}`;
}
