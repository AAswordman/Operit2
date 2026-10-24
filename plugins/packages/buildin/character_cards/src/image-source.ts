import { hostFilePath } from "./host-path";

/** Reads a picked file once for import; display code passes paths to its image host. */
export async function readAvatarImport(path: string): Promise<string> {
  const file = await Tools.Files.readBinary(path);
  if (file.size > 8 * 1024 * 1024) throw new Error("头像图片不能超过 8 MB");
  const b = file.contentBase64;
  if (!(b.startsWith("iVBOR") || b.startsWith("/9j/") || b.startsWith("R0lGOD")
    || b.startsWith("UklGR") || b.startsWith("Qk"))) {
    throw new Error("请选择 PNG、JPEG、GIF、WebP 或 BMP 图片");
  }
  return b;
}
/** Maps only an explicitly picked host file using its actual platform's documented VFS mount. */
export function pickedImagePath(path: string, platform: string): string {
  const mapped = hostFilePath(path, platform);
  if (mapped !== null) return mapped;
  if (!path.replace(/\\/g, "/").startsWith("/")) throw new Error("选择器没有返回可读取的文件路径");
  throw new Error("当前平台不支持导入此图片路径");
}
