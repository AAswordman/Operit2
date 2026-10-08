import type { Snapshot } from "../../../../src/model";
import type { ActionFeature, EditorContext } from "../../../bridge/context";
import type { DialogSpec, ToolAccessDialog } from "../../../bridge/contracts";
import { dataValue } from "../../../shared/dom";
import { button, copy, escapeHtml } from "../../../shared/ui/html";

const categories = [
  { catalog: "builtinTools", selected: "allowedBuiltinTools", title: "内置工具" },
  { catalog: "packages", selected: "allowedPackages", title: "工具包" },
  { catalog: "skills", selected: "allowedSkills", title: "技能" },
  { catalog: "mcpServers", selected: "allowedMcpServers", title: "MCP 服务" },
] as const;

/** Renders staged selections from the real complete tool-source catalogue. */
export function toolPolicySpec(dialog: ToolAccessDialog, snapshot: Snapshot): DialogSpec {
  let body = "";
  for (const category of categories) {
    let options = "";
    for (const source of snapshot.toolCatalog[category.catalog]) {
      const selected = dialog.draft[category.selected].indexOf(source.name) >= 0;
      options += `<label class="checkbox-row"><input type="checkbox" data-tool-category="${category.catalog}" data-tool-source="${escapeHtml(source.name)}" ${selected ? "checked" : ""}><span class="grow"><strong>${escapeHtml(source.displayName)}</strong><br><span class="muted">${escapeHtml(source.description)}</span></span></label>`;
    }
    body += `<section class="section"><h3>${category.title}</h3>${options === "" ? '<p class="muted">当前目录没有可选来源。</p>' : options}</section>`;
  }
  return { title: "自定义允许使用的工具", style: "large", tabs: [], body, footer: button("close-dialog", "取消", "") + button("save-tool-access", "确定", "check", "", "filled") };
}

/** Updates only the declared tool category and exact catalogue identity selected by the user. */
export function changeToolSource(context: EditorContext, element: HTMLInputElement): void {
  const catalog = dataValue(element, "toolCategory"), name = dataValue(element, "toolSource");
  for (const category of categories) {
    if (category.catalog !== catalog) continue;
    if (!context.snapshot().toolCatalog[category.catalog].some(
      /** Requires the exact registered source rather than interpreting a string fragment. */
      source => source.name === name,
    )) throw new Error("选择的工具来源不在真实目录中");
    const selected = context.topDialog("tool-access").draft[category.selected], index = selected.indexOf(name);
    if (element.checked && index < 0) selected.push(name);
    if (!element.checked && index >= 0) selected.splice(index, 1);
    return;
  }
  throw new Error("工具来源分类无效");
}

/** Applies the staged complete tool policy only when the nested editor is explicitly confirmed. */
export function createToolPolicyFeature(context: EditorContext): ActionFeature {
  /** Confirms the nested draft without performing a separate persistence request. */
  async function handleAction(action: string): Promise<void> {
    if (action !== "save-tool-access") throw new Error(`Unknown tool policy action: ${action}`);
    const dialog = context.topDialog("tool-access");
    dialog.parent.card.toolAccessConfig = copy(dialog.draft); context.popDialog();
  }
  return { names: ["save-tool-access"], handleAction };
}
