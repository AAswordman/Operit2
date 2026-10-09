/** Converts VFS images into sources supported by native Image and offline WebView. */
const sources = new Map<string, Promise<string>>();
export async function imageSource(path: string): Promise<string> {
  if (/^(data:image\/|https?:\/\/)/i.test(path)) return path;
  let pending = sources.get(path);
  if (pending === undefined) {
    pending = read(); sources.set(path, pending);
    pending.catch(() => sources.delete(path));
  }
  return pending;
  async function read(): Promise<string> {
    const file = await Tools.Files.readBinary(path);
    if (file.size > 8 * 1024 * 1024) throw new Error("头像图片不能超过 8 MB");
    const b = file.contentBase64;
    const mime = b.startsWith("iVBOR") ? "image/png" : b.startsWith("/9j/") ? "image/jpeg"
      : b.startsWith("R0lGOD") ? "image/gif" : b.startsWith("UklGR") ? "image/webp"
      : b.startsWith("Qk") ? "image/bmp" : null;
    if (mime === null) throw new Error("请选择 PNG、JPEG、GIF、WebP 或 BMP 图片");
    return `data:${mime};base64,${b}`;
  }
}
/** Maps only an explicitly picked host file using its actual platform's documented VFS mount. */
export function pickedImagePath(path: string, platform: string): string {
  const normalized = path.replace(/\\/g, "/");
  if (normalized.startsWith("/app/") || normalized.startsWith("/mnt/")) return normalized;
  if (platform === "windows" && /^[a-z]:\//i.test(normalized)) return `/mnt/windows/${normalized[0].toLowerCase()}/${normalized.slice(3)}`;
  if (!normalized.startsWith("/")) throw new Error("选择器没有返回可读取的文件路径");
  if (platform === "macos" || platform === "linux") return `/mnt/${platform}${normalized}`;
  if (platform === "android") return `/mnt/android/root${normalized}`;
  throw new Error("当前平台不支持导入此图片路径");
}
