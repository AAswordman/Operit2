import type { ThemeChoice } from "../../../../src/selection-application";
import { assertThemeChoices } from "../../../../src/selection-application";
import type { ThemeDialog, DialogSpec } from "../../../bridge/contracts";
import { button, escapeHtml, icon } from "../../../shared/ui/html";
import type { ActionFeature, EditorContext } from "../../../bridge/context";
import { dataValue, requireElement } from "../../../shared/dom";
import { matchesSelectionQuery } from "../selection";

/** Uses the plugin's single private picker-row type, not a second SDK wire schema. */
export type { ThemeChoice } from "../../../../src/selection-application";

export { assertThemeChoices };
/** Returns the owning draft only; picking a theme never applies it or mutates an independent configuration. */
export function themeDraft(dialog: ThemeDialog): { themeConfigId: string | null } {
  switch (dialog.parent.type) {
    case "character": return dialog.parent.card;
    case "group": return dialog.parent.group;
  }
}
/** Stages one exact real directory reference, or the user's explicit request not to change the independent current theme. */
export function chooseThemeReference(dialog: ThemeDialog, id: string | null): void {
  assertThemeChoices(dialog.choices);
  if (id !== null && !dialog.choices.some(
    /** Requires the exact independent configuration ID without rewriting it into a legacy role prefix. */
    choice => choice.id === id,
  )) throw new Error("主题配置不存在：" + id);
  themeDraft(dialog).themeConfigId = id;
}

/** Displays the owner's explicit independent reference and opens the real shared picker without changing current prefs. */
export function themeReferenceSection(themeConfigId: string | null, choices: readonly ThemeChoice[]): string {
  assertThemeChoices(choices);
  const selected = choices.find(
    /** Resolves the stored opaque reference against the real named theme catalog. */
    choice => choice.id === themeConfigId,
  );
  const missing = themeConfigId !== null && selected === undefined;
  const description = themeConfigId === null ? "切换到此角色或群组时保持当前主题" : missing ? "已绑定主题配置不存在：" + themeConfigId : selected!.label;
  return '<section class="binding" aria-label="主题配置"><strong>主题配置</strong><p class="muted"' + (missing ? ' role="alert"' : '') + '>' + escapeHtml(description) + '</p><div class="row"><span class="grow"></span>' + (themeConfigId === null ? '' : button("unbind-theme", "解除绑定", "", "", "small")) + button("select-theme", "选择主题配置", "tune", "", "small") + '</div></section>';
}

/** Requires the caller's actual catalog reader; this feature neither implements nor substitutes a Theme host capability. */
export function createThemeFeature(context: EditorContext, readChoices: () => Promise<readonly ThemeChoice[]>): ActionFeature {
  /** Opens actual supplied choices or edits only the parent draft; save and automatic application remain separate operations. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    switch (action) {
      case "select-theme": {
        const parent = context.topDialog();
        if (parent.type !== "character" && parent.type !== "group") throw new Error("主题绑定必须属于角色或群组编辑器");
        const choices = await readChoices(); assertThemeChoices(choices);
        if (context.topDialog() !== parent) throw new Error("主题选择请求的所属编辑器已改变");
        context.pushDialog({ type: "theme", parent, query: "", choices }); return;
      }
      case "unbind-theme": {
        const parent = context.topDialog();
        if (parent.type === "character") parent.card.themeConfigId = null;
        else if (parent.type === "group") parent.group.themeConfigId = null;
        else throw new Error("主题绑定必须属于角色或群组编辑器");
        context.renderDialogs(); return;
      }
      case "choose-theme": chooseThemeReference(context.topDialog("theme"), dataValue(element, "id")); context.popDialog(); return;
      case "clear-theme": chooseThemeReference(context.topDialog("theme"), null); context.popDialog(); return;
      default: throw new Error("Unknown theme picker action: " + action);
    }
  }
  return { names: ["select-theme", "choose-theme", "clear-theme", "unbind-theme"], handleAction };
}
/** Preserves real input focus and selection while updating only the local picker query. */
export function updateThemeQuery(context: EditorContext, element: HTMLInputElement): void {
  const start = element.selectionStart, end = element.selectionEnd;
  context.topDialog("theme").query = element.value; context.renderDialogs();
  const input = requireElement(context.dialogsRoot.querySelector("[data-theme-query]"), HTMLInputElement);
  input.focus(); input.setSelectionRange(start, end);
}

/** Renders the same searchable compact picker pattern as independent TTS configuration selection. */
export function themeSpec(dialog: ThemeDialog): DialogSpec {
  assertThemeChoices(dialog.choices); const selected = themeDraft(dialog).themeConfigId;
  let rows = '<button type="button" class="picker-row" data-action="clear-theme" aria-label="不改变当前主题">' + icon(selected === null ? "check" : "tune") + '<span><strong>不改变当前主题</strong><br><span class="muted">角色或群组不要求切换独立主题配置</span></span></button>';
  let matches = 0;
  for (const choice of dialog.choices) {
    if (!matchesSelectionQuery(dialog.query, [choice.label])) continue;
    rows += '<button type="button" class="picker-row" data-action="choose-theme" data-id="' + escapeHtml(choice.id) + '" aria-label="选择主题 ' + escapeHtml(choice.label) + '">' + icon(selected === choice.id ? "check" : "tune") + '<span><strong>' + escapeHtml(choice.label) + '</strong></span></button>';
    matches += 1;
  }
  const search = '<label class="field"><span class="field-label">搜索主题配置</span><input data-theme-query aria-label="搜索主题配置" value="' + escapeHtml(dialog.query) + '"></label>';
  const absent = selected !== null && !dialog.choices.some(
    /** Reports a genuinely missing saved reference rather than presenting it as the unbound null choice. */
    choice => choice.id === selected,
  ) ? '<p role="alert">已绑定主题配置不存在：' + escapeHtml(selected) + '</p>' : "";
  return { title: "选择主题配置", style: "chooser", tabs: [], body: search + absent + rows + (dialog.choices.length === 0 ? '<div class="empty">还没有已保存的主题配置。请先在设置 → 外观中保存主题配置，再来绑定。</div>' : matches === 0 ? '<div class="empty">没有符合条件的主题配置。</div>' : ""), footer: button("close-dialog", "取消", "") };
}
