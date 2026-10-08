import type { Theme } from "../../../src/model";

/** Applies the complete actual host Material palette to either offline document. */
export function applyMaterialTheme(theme: Theme): void {
  if (theme.brightness !== "light" && theme.brightness !== "dark") throw new Error("宿主主题亮度无效");
  const roles = ["primary", "onPrimary", "primaryContainer", "onPrimaryContainer", "onSurface", "onSurfaceVariant", "surface", "surfaceContainer", "surfaceContainerHigh", "surfaceContainerHighest", "outline", "outlineVariant", "error"];
  for (const role of roles) {
    if (typeof theme.colors[role] !== "string" || !/^#[0-9a-f]{8}$/i.test(theme.colors[role])) throw new Error("宿主缺少主题颜色：" + role);
    document.documentElement.style.setProperty("--" + role, theme.colors[role]);
  }
  document.documentElement.style.colorScheme = theme.brightness;
  document.documentElement.dataset.brightness = theme.brightness;
}
