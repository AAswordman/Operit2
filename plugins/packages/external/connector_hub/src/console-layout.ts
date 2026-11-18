import type { ComposeColor, ComposeDslContext, ComposeNode, TextProps } from "../../../../types/compose-dsl";
import type { Channel, Connector } from "./api.js";

// Same mark as resources/logo.svg.
const connectorMark = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="16" fill="#17264D"/><path d="M18 21h11a4 4 0 0 1 4 4v14a4 4 0 0 0 4 4h9M18 43h9m6-10h8" fill="none" stroke="#B4C8FF" stroke-width="3" stroke-linecap="round"/><circle cx="18" cy="21" r="5" fill="#C9D7FF"/><circle cx="18" cy="43" r="5" fill="#C9D7FF"/><circle cx="46" cy="43" r="5" fill="#77E7C3"/><circle cx="46" cy="33" r="5" fill="#C9D7FF"/></svg>';

export const platforms = {
  qq: { short: "QQ", name: "QQ Bot", icon: "Forum", tint: "#1558B5", tile: "#E4EFFF",
    settings: "toolpkg:com.operit.qqbot_bundle:ui:qqbot_settings" },
  wechat: { short: "微信", name: "微信 Bot", icon: "ChatBubbleOutline", tint: "#146443", tile: "#E2F4EB",
    settings: "toolpkg:com.operit.wechat_bot:ui:wechat_settings" },
  feishu: { short: "飞书", name: "飞书 Bot", icon: "Send", tint: "#6242A6", tile: "#EEE6FA", settings: null },
} satisfies Record<Channel, { short: string; name: string; icon: string; tint: string; tile: string; settings: string | null }>;

export function consoleLayout(ctx: ComposeDslContext, busy: boolean) {
  const ui = ctx.UI;
  const colors = ctx.MaterialTheme.colorScheme;
  const shape = { cornerRadius: 14 };
  const text = (value: string, style: TextProps["style"] = "bodyMedium", options: Partial<TextProps> = {}) =>
    ui.Text({ text: value, style, letterSpacing: 0, ...options });
  const logo = (size: number) => ui.Svg({ key: "connector-logo", source: connectorMark, width: size, height: size });
  const muted = (value: string) => text(value, "bodySmall", { color: colors.onSurfaceVariant });
  const icon = (name: string, size = 20, tint?: ComposeColor) => ui.Icon({ name, size, tint });
  const row = (key: string, children: ComposeNode[]) => ui.Row({ key, fillMaxWidth: true, verticalAlignment: "center", spacing: 10 }, children);
  const flow = (key: string, children: ComposeNode[]) => ui.FlowRow({ key, spacing: 6, runSpacing: 4, itemVerticalAlignment: "center" }, children);
  const stack = (key: string, children: ComposeNode[], spacing = 8) => ui.Column({ key, fillMaxWidth: true, spacing }, children);
  const badge = (label: string, color?: ComposeColor, background?: ComposeColor) => ui.Box({ padding: { horizontal: 8, vertical: 4 },
    background: background ?? colors.surfaceContainerHighest, backgroundShape: { cornerRadius: 99 } }, text(label, "labelSmall", { color: color ?? colors.onSurfaceVariant }));
  const tile = (channel: Channel, size = 40) => {
    const platform = platforms[channel];
    return ui.Box({ width: size, height: size, contentAlignment: "center", background: platform.tile, backgroundShape: shape },
      icon(platform.icon, size === 32 ? 16 : 20, platform.tint));
  };
  const status = (connector: Connector | undefined, loaded: boolean) => {
    if (!loaded) return badge("读取中");
    if (!connector?.available) return badge("不可用");
    const current = connector.status;
    if (current?.service === "error") return badge("连接异常", colors.onErrorContainer, colors.errorContainer);
    if (!current || current.configured === false) return badge("未连接");
    if (current.service === "ready") return badge("在线", "#155A42", "#D6F4E7");
    return badge(current.service === "connecting" ? "连接中" : "已停止");
  };
  const button = (key: string, label: string, onClick: () => void | Promise<void>, variant: "primary" | "tonal" | "text" = "text",
    enabled = true, glyph?: string) => ui[variant === "primary" ? "Button" : variant === "tonal" ? "FilledTonalButton" : "TextButton"]({
      key, ...(glyph ? { content: ui.Row({ spacing: 6, verticalAlignment: "center" }, [
        icon(glyph, 16, busy || !enabled ? colors.onSurfaceVariant : variant === "primary" ? colors.onPrimary : variant === "tonal" ? colors.onSecondaryContainer : colors.primary),
        ui.Text({ text: label, inheritStyle: true }),
      ]) } : { text: label }),
      onClick, enabled: !busy && enabled, shape, contentPadding: { horizontal: 12, vertical: 8 },
    });
  const card = (key: string, children: ComposeNode[]) => ui.Card({ key, fillMaxWidth: true, shape, elevation: 0,
    containerColor: colors.surfaceContainerLow, border: { width: 1, color: colors.outlineVariant } },
    ui.Column({ fillMaxWidth: true, padding: 16, spacing: 12 }, children));
  const empty = (key: string, glyph: string, title: string, detail?: string) => ui.Column({ key, fillMaxWidth: true,
    horizontalAlignment: "center", padding: { horizontal: 16, vertical: 28 }, spacing: 8 }, [icon(glyph, 24, colors.onSurfaceVariant),
    text(title, "titleSmall"), ...(detail ? [text(detail, "bodySmall", { color: colors.onSurfaceVariant, textAlign: "center" })] : [])]);
  return { colors, shape, logo, text, muted, icon, row, flow, stack, badge, tile, status, button, card, empty };
}
