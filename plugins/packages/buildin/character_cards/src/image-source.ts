import type { ComposePickedFile } from "../../../../types/compose-dsl";

export const MAX_AVATAR_BYTES = 8 * 1024 * 1024;

/** Validates picker-supplied bytes before the plugin stores its own avatar file. */
export function readAvatarImport(file: Pick<ComposePickedFile, "contentBase64" | "size">): string {
  if (!Number.isSafeInteger(file.size) || file.size < 0) throw new Error("头像图片大小无效");
  if (file.size > MAX_AVATAR_BYTES) throw new Error("头像图片不能超过 8 MB");
  const b = file.contentBase64;
  if (typeof b !== "string") throw new Error("选择器没有返回头像图片内容");
  const padding = b.endsWith("==") ? 2 : b.endsWith("=") ? 1 : 0;
  if (b.length % 4 !== 0 || b.length / 4 * 3 - padding !== file.size
    || !/^[A-Za-z0-9+/]*={0,2}$/.test(b)) throw new Error("头像图片内容不完整");
  if (!(b.startsWith("iVBOR") || b.startsWith("/9j/") || b.startsWith("R0lGOD")
    || b.startsWith("UklGR") || b.startsWith("Qk"))) {
    throw new Error("请选择 PNG、JPEG、GIF、WebP 或 BMP 图片");
  }
  return b;
}
