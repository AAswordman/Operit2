/**
 * Maps one explicitly provided host path onto the platform's documented VFS mount.
 * Returns null when the path is relative, a non-file URI, or the platform has no mount.
 */
export function hostFilePath(path: string, platform: string): string | null {
  const normalized = path.replace(/\\/g, "/");
  if (normalized.startsWith("/app/") || normalized.startsWith("/mnt/")) return normalized;
  if (platform === "windows" && /^[a-z]:\//i.test(normalized)) return `/mnt/windows/${normalized[0].toLowerCase()}/${normalized.slice(3)}`;
  if (!normalized.startsWith("/")) return null;
  if (platform === "macos" || platform === "linux") return `/mnt/${platform}${normalized}`;
  if (platform === "android") return `/mnt/android/root${normalized}`;
  return null;
}
