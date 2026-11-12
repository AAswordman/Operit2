import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import type { Request, RequestOutput } from "./model";

/** Calls the sole main-runtime service; UI runtimes never own persistence. */
export function uiRequest<R extends Request>(input: R): Promise<RequestOutput<R>> {
  return ToolPkg.ipc.call<R, RequestOutput<R>>("character-memory.request", input, { targetRuntime: "main" });
}

/** Keeps asynchronous callbacks on the current state instead of a captured render. */
export function uiState<T extends object>(ctx: ComposeDslContext, key: string, initial: T) {
  const [state, setState] = ctx.useState<T>(key, initial);
  const live = ctx.useRef(key + ":live", state); live.current = state;
  /** Publishes one immutable patch while preserving changes made by other callbacks. */
  function update(patch: Partial<T>): void { live.current = { ...live.current, ...patch }; setState(live.current); }
  return { state, live, update };
}

/** Reads one explicitly selected JSON document through the host VFS boundary. */
export async function readUiJson(ctx: ComposeDslContext): Promise<string | null> {
  const result = await ctx.openFilePicker({ picker: "document", allowMultiple: false, mimeTypes: ["application/json"] });
  if (result.cancelled) return null;
  if (result.files.length !== 1) throw new Error("请选择一个 JSON 文件");
  const file = await Tools.Files.read(result.files[0].path);
  const content = file.content.replace(/^\uFEFF/, "");
  if (content.trim() === "") throw new Error("所选 JSON 文件为空");
  return content;
}

/** Validates numeric drafts only on submission so incomplete input remains editable. */
export function uiNumber(value: string, label: string, min: number, max = Infinity, integer = false): number {
  const number = Number(value);
  if (value.trim() === "" || !Number.isFinite(number) || number < min || number > max || integer && !Number.isInteger(number)) throw new Error(label + "的范围或格式无效");
  return number;
}

/** Converts a date-only draft into an inclusive local-day boundary. */
export function uiDate(value: string, end: boolean): number | null {
  if (value === "") return null;
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) throw new Error("日期格式应为 YYYY-MM-DD");
  const [year, month, day] = value.split("-").map(Number), date = new Date(year, month - 1, day);
  if (date.getFullYear() !== year || date.getMonth() !== month - 1 || date.getDate() !== day || date.getTime() < 0) throw new Error("日期无效");
  if (end) { date.setDate(date.getDate() + 1); return date.getTime() - 1; }
  return date.getTime();
}

/** Builds shared Material controls with wrapping actions and bounded lazy bodies. */
export function nativeUi(ctx: ComposeDslContext, enabled = true) {
  const colors = ctx.MaterialTheme.colorScheme;
  const [size, setSize] = ctx.useState("character-ui-size", { width: 0, height: 0 });
  const measure = ctx.Modifier.onSizeChanged(value => { if (value.width !== size.width || value.height !== size.height) setSize(value); });
  /** Creates readable text in the current Material palette. */
  function text(value: string, title = false, key?: string): ComposeNode { return ctx.UI.Text({ key, text: value, style: title ? "titleMedium" : "bodyMedium", color: title ? colors.onSurface : colors.onSurfaceVariant }); }
  /** Creates a stable keyed action with an explicit busy state. */
  function button(key: string, label: string, action: () => unknown, active = true): ComposeNode {
    return ctx.UI.TextButton({ key, height: 38, text: label, enabled: enabled && active, onClick: action as () => Promise<void> });
  }
  /** Matches the original filled submit control in modal footers. */
  function primary(key: string, label: string, action: () => unknown, active = true): ComposeNode {
    return ctx.UI.Button({ key, height: 38, text: label, enabled: enabled && active, onClick: action as () => Promise<void> });
  }
  /** Preserves the original compact icon-only entry and its accessible tooltip. */
  function iconButton(key: string, label: string, name: string, action: () => unknown, active = true): ComposeNode {
    return ctx.UI.IconButton({ key, tooltip: label, width: 32, height: 32, enabled: enabled && active, contentPadding: { horizontal: 6, vertical: 6 }, onClick: action as () => Promise<void> }, ctx.UI.Icon({ name, size: 20, tint: colors.onSurfaceVariant }));
  }
  /** Keeps the original section action labels, icons and compact typography. */
  function sectionAction(key: string, label: string, name: string, action: () => unknown, active = true): ComposeNode {
    return ctx.UI.TextButton({ key, height: 32, enabled: enabled && active, contentPadding: { horizontal: 8, vertical: 4 }, onClick: action as () => Promise<void> }, ctx.UI.Row({ spacing: 7, verticalAlignment: "center" }, [...(name === "" ? [] : [ctx.UI.Icon({ name, size: 20, tint: colors.primary })]), ctx.UI.Text({ text: label, fontSize: 13, fontWeight: "600", color: colors.primary })]));
  }
  /** Recreates the original outlined settings section without introducing navigation. */
  function section(key: string, title: string, name: string, controls: ComposeNode[], children: ComposeNode[]): ComposeNode {
    return ctx.UI.Box({ key, fillMaxWidth: true, paddingBottom: 14 }, ctx.UI.Surface({ fillMaxWidth: true, color: colors.surfaceContainer.copy({ alpha: 0.65 }), shape: { cornerRadius: 16 }, modifier: ctx.Modifier.border(1, colors.outlineVariant.copy({ alpha: 0.36 }), { cornerRadius: 16 }) }, ctx.UI.Column({ fillMaxWidth: true, padding: size.width <= 600 ? 12 : 14, spacing: 0 }, [
      ctx.UI.Row({ fillMaxWidth: true, height: 32, spacing: 10, verticalAlignment: "center" }, [ctx.UI.Icon({ name, size: 20, tint: colors.primary }), ctx.UI.Text({ text: title, fontSize: 16, fontWeight: "700", weight: 1, maxLines: 1 }), ctx.UI.FlowRow({ spacing: 2, runSpacing: 4, itemVerticalAlignment: "center" }, controls)]), ctx.UI.Spacer({ height: 12 }), ...children.flatMap((child, index) => index === 0 ? [child] : [ctx.UI.Spacer({ height: 8 }), child]),
    ])));
  }
  /** Recreates entity portraits, badges, active styling and the original action row. */
  function tile(key: string, title: string, subtitle: string, badges: string[], controls: ComposeNode[], active = false, onClick?: () => void, avatar?: string | null, name = "PersonOutline"): ComposeNode {
    const portraitImage = ctx.UI.Box({ width: 32, height: 32, contentAlignment: "center", modifier: ctx.Modifier.clip({ type: "circle" }).background(active ? colors.primaryContainer : colors.surfaceContainerHighest) }, avatar ? ctx.UI.Image({ uri: avatar, width: 32, height: 32, contentScale: "crop" }) : ctx.UI.Icon({ name, size: 24 }));
    const portrait = ctx.UI.Box({ width: 32, height: 32 }, [portraitImage, ...(active && onClick ? [ctx.UI.Box({ modifier: ctx.Modifier.align("bottomEnd").clip({ type: "circle" }).background(colors.primary), width: 14, height: 14, contentAlignment: "center" }, ctx.UI.Icon({ name: "Check", size: 9, tint: colors.onPrimary }))] : [])]);
    const content = ctx.UI.Row({ key: key + "-open", ...(size.width > 440 ? { weight: 1 } : { fillMaxWidth: true }), verticalAlignment: "center", spacing: 10, ...(enabled && onClick ? { onClick } : {}) }, [portrait, ctx.UI.Column({ weight: 1, spacing: 4 }, [ctx.UI.Text({ text: title, fontSize: 14, fontWeight: "700", maxLines: 1, overflow: "ellipsis" }), ...(subtitle === "" ? [] : [ctx.UI.Text({ text: subtitle, fontSize: 12, color: colors.onSurfaceVariant, maxLines: 1, overflow: "ellipsis" })]), ctx.UI.FlowRow({ spacing: 6, runSpacing: 5, itemVerticalAlignment: "center" }, badges.map(badge))])]);
    const actionRow = ctx.UI.FlowRow({ ...(size.width > 440 ? {} : { fillMaxWidth: true }), horizontalArrangement: "end", spacing: 4, runSpacing: 4, itemVerticalAlignment: "center" }, controls);
    const body = size.width > 440
      ? ctx.UI.Row({ fillMaxWidth: true, paddingHorizontal: 12, paddingVertical: 10, spacing: 8, verticalAlignment: "center" }, [content, actionRow])
      : ctx.UI.Column({ fillMaxWidth: true, paddingHorizontal: 12, paddingVertical: 10, spacing: 14 }, [content, actionRow]);
    return ctx.UI.Surface({ key, fillMaxWidth: true, color: active ? colors.primaryContainer.copy({ alpha: 0.16 }) : colors.surfaceContainerHighest.copy({ alpha: 0.28 }), shape: { cornerRadius: 12 }, modifier: ctx.Modifier.border(1, active ? colors.primary.copy({ alpha: 0.45 }) : colors.outlineVariant.copy({ alpha: 0.28 }), { cornerRadius: 12 }) }, body);
  }
  /** Gives metadata and current-selection badges matching dimensions and padding. */
  function badge(label: string): ComposeNode {
    const active = label === "当前使用", shape = { cornerRadius: 6 };
    const modifier = ctx.Modifier.clip(shape).background((active ? colors.primary : colors.onSurfaceVariant).copy({ alpha: 0.08 }));
    return ctx.UI.Box({ height: 20, paddingHorizontal: 7, contentAlignment: "center", modifier: active ? modifier.border(1, colors.primary.copy({ alpha: 0.32 }), shape) : modifier }, ctx.UI.Text({ text: label, fontSize: 11, fontWeight: active ? "600" : "400", color: active ? colors.primary : colors.onSurfaceVariant, maxLines: 1 }));
  }
  /** Marks the current selection with the same compact badge used in metadata rows. */
  function activePill(): ComposeNode { return badge("当前使用"); }
  /** Wraps action controls at narrow phone widths instead of shrinking labels. */
  function actions(children: ComposeNode[]): ComposeNode { return ctx.UI.FlowRow({ fillMaxWidth: true, spacing: 4, runSpacing: 4 }, children); }
  /** Creates a native retained text field with room for its floating label. */
  function field(key: string, label: string, value: string, change: (value: string) => void, lines = 1): ComposeNode {
    return ctx.UI.Box({ key: key + "-box", fillMaxWidth: true, paddingTop: 8, paddingBottom: 8 }, ctx.UI.TextField({ key, label, value, readOnly: !enabled, decoration: { filled: false, isDense: true, cornerRadius: 8, borderColor: colors.outline, focusedBorderColor: colors.primary, contentPadding: { horizontal: 12, vertical: 14 } }, style: { fontSize: 14 }, singleLine: lines === 1, minLines: lines, maxLines: lines === 1 ? 1 : Math.max(lines, 12), onValueChange: value => { if (enabled) change(value); }, fillMaxWidth: true }));
  }
  /** Puts a descriptive label beside the native switch. */
  function toggle(key: string, label: string, checked: boolean, change: (checked: boolean) => unknown): ComposeNode {
    return ctx.UI.Row({ key: key + "-row", fillMaxWidth: true, verticalAlignment: "center", spacing: 12, paddingVertical: 8 }, [ctx.UI.Text({ text: label, weight: 1 }), ctx.UI.Switch({ key, checked, enabled, onCheckedChange: change as (checked: boolean) => Promise<void> })]);
  }
  /** Gives every lazy row a stable identity and keeps outlined fields inside the clip. */
  function list(key: string, children: ComposeNode[]): ComposeNode {
    return ctx.UI.LazyColumn({ key, weight: 1, fillMaxWidth: true }, children.map((child, index) => ctx.UI.Box({ key: key + ":" + String(child.props?.key ?? index), fillMaxWidth: true, paddingHorizontal: 16, paddingVertical: 6 }, child)));
  }
  /** Displays one selectable configuration without implementing a second dropdown host. */
  function choices(key: string, label: string, selected: string, options: readonly { id: string; label: string }[], change: (id: string) => void): ComposeNode {
    return ctx.UI.Column({ key, fillMaxWidth: true, spacing: 4 }, [...(label === "" ? [] : [text(label)]), ctx.UI.FlowRow({ spacing: 6, runSpacing: 6 }, options.map(option => ctx.UI.FilterChip({ key: key + ":" + option.id, selected: selected === option.id, enabled, label: ctx.UI.Text({ text: option.label, fontSize: 13 }), onClick: () => change(option.id) })))]);
  }
  /** Groups a title, descriptive content and wrapping controls in one native surface. */
  function card(key: string, title: string, description: string, controls: ComposeNode[], onClick?: () => void, avatar?: string | null): ComposeNode {
    return ctx.UI.Surface({ key, fillMaxWidth: true, color: colors.surfaceContainer, shape: { cornerRadius: 16 } }, ctx.UI.Column({ fillMaxWidth: true, padding: 16, spacing: 8 }, [
      ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center", spacing: 12, ...(enabled && onClick ? { onClick } : {}) }, [
        ...(avatar ? [ctx.UI.Image({ uri: avatar, width: 44, height: 44, contentScale: "crop", modifier: ctx.Modifier.clip({ cornerRadius: 12 }) })] : []),
        ctx.UI.Column({ weight: 1, spacing: 4 }, [text(title, true), ...(description === "" ? [] : [text(description)])]),
      ]), actions(controls),
    ]));
  }
  /** Uses the existing native popup host for compact toolbar menus. */
  function menu(key: string, label: string, name: string, entries: readonly { key: string; label: string; name: string; action: () => unknown; enabled?: boolean }[], iconOnly = false): ComposeNode {
    return ctx.UI.MenuAnchor({ key, tooltip: label, itemHeight: 40, items: entries.map(entry => ({ enabled: enabled && entry.enabled !== false })), anchor: ctx.UI.Row({ height: 32, paddingHorizontal: iconOnly ? 6 : 8, spacing: 7, verticalAlignment: "center" }, [ctx.UI.Icon({ name, size: 20, tint: colors.primary }), ...(iconOnly ? [] : [ctx.UI.Text({ text: label, fontSize: 13, fontWeight: "600", color: colors.primary }), ctx.UI.Icon({ name: "ArrowDropDown", size: 16, tint: colors.primary })])]), onSelect: async index => { const entry = entries[index]; if (enabled && entry !== undefined && entry.enabled !== false) await entry.action(); } }, entries.map(entry => ctx.UI.Row({ key: entry.key, spacing: 10, verticalAlignment: "center" }, [ctx.UI.Icon({ name: entry.name, size: 20 }), ctx.UI.Text({ text: entry.label, fontSize: 13 })])));
  }
  /** Preserves select controls as dropdowns, including their original option order. */
  function choose(key: string, label: string, selected: string, options: readonly { id: string; label: string }[], change: (id: string) => void, compact = false): ComposeNode {
    if (compact) return ctx.UI.MenuAnchor({ key, tooltip: label, items: options.map(() => ({ enabled })), anchor: ctx.UI.Row({ height: 32, paddingHorizontal: 10, spacing: 8, verticalAlignment: "center", modifier: ctx.Modifier.border(1, colors.outlineVariant, { cornerRadius: 8 }) }, [ctx.UI.Icon({ name: "FolderOutlined", size: 16 }), ctx.UI.Text({ text: options.find(option => option.id === selected)!.label, fontSize: 13 }), ctx.UI.Icon({ name: "KeyboardArrowDown", size: 16 })]), onSelect: index => { if (enabled && options[index] !== undefined) change(options[index].id); } }, options.map(option => ctx.UI.Text({ text: option.label, fontSize: 13 })) );
    return ctx.UI.Box({ key: key + "-box", fillMaxWidth: true, paddingTop: 8, paddingBottom: 8 }, ctx.UI.DropdownMenu({ key, label, text: options.find(option => option.id === selected)?.label ?? selected, enabled, fillMaxWidth: true, onClick: index => { if (enabled && options[index] !== undefined) change(options[index].id); } }, options.map(option => ctx.UI.Text({ text: option.label, paddingVertical: 8 }))));
  }
  /** Preserves the memory editor's 0.01 precision score slider. */
  function score(key: string, label: string, value: string, change: (value: string) => void): ComposeNode {
    return ctx.UI.Column({ fillMaxWidth: true, spacing: 4 }, [text(label + " " + Number(value).toFixed(2)), ctx.UI.Slider({ key, fillMaxWidth: true, enabled, min: 0, max: 1, divisions: 100, value: Number(value), label: Number(value).toFixed(2), onValueChange: number => change(number.toFixed(2)) })]);
  }
  /** Keeps the web modal's bounds, fixed heading and fixed footer in the native host. */
  function dialog(key: string, title: string, body: ComposeNode, footer: ComposeNode[], close: () => void, variant: "compact" | "normal" | "large" | "graph" = "normal", onLoad?: () => Promise<void>, header?: ComposeNode, desiredHeight?: number): ComposeNode {
    const phone = size.width <= 600, margin = phone && variant === "graph" ? 0 : phone ? 24 : 48;
    const width = Math.max(0, Math.min(variant === "compact" ? 500 : variant === "graph" ? 1400 : 760, size.width - margin));
    const height = Math.max(0, Math.min(desiredHeight ?? (variant === "compact" ? 240 : variant === "normal" ? 620 : variant === "large" ? 820 : size.height), size.height - margin));
    return ctx.UI.Dialog({ key, onLoad, closeOnDismissRequest: false, onDismissRequest: () => { if (enabled) close(); }, containerColor: colors.surface.copy({ alpha: 0 }), properties: { fullscreen: true, usePlatformDefaultWidth: false, dismissOnClickOutside: false } }, ctx.UI.Surface({ width, height, color: variant === "graph" ? colors.surface : colors.surfaceContainerHigh, shape: { cornerRadius: phone && variant === "graph" ? 0 : phone ? 20 : 24 } }, ctx.UI.Column({ fillMaxSize: true }, [
      header ?? ctx.UI.Row({ fillMaxWidth: true, paddingStart: phone ? 18 : 22, paddingTop: phone ? 16 : 18, paddingEnd: 12, paddingBottom: 10, verticalAlignment: "center" }, [ctx.UI.Text({ text: title, fontSize: phone ? 18 : 20, fontWeight: "700", weight: 1 }), iconButton(key + "-close", "关闭", "Close", close)]),
      ...(enabled ? [] : [ctx.UI.LinearProgressIndicator({ fillMaxWidth: true })]),
      ctx.UI.Box({ weight: 1, fillMaxWidth: true }, body),
      ...(footer.length === 0 ? [] : [ctx.UI.FlowRow({ fillMaxWidth: true, paddingHorizontal: 18, paddingTop: 12, paddingBottom: 18, horizontalArrangement: "end", spacing: 8, runSpacing: 8 }, footer)]),
    ])));
  }
  return { primary, menu, size, measure, choose, score, dialog, text, button, iconButton, sectionAction, section, tile, activePill, actions, field, toggle, list, choices, card };
}
