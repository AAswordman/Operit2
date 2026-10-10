import { ttsSpec } from "../features/characters/bindings/tts";
import { themeSpec } from "../features/characters/bindings/theme";
import { toolPolicySpec } from "../features/characters/bindings/tool-policy";
import { characterBody } from "../features/characters/editor";
import { previewSpec } from "../features/characters/preview";
import { exportSpec, importSpec } from "../features/characters/import-export-view";
import { tagManagerBody } from "../features/characters/tags-view";
import { setGraphViewport } from "../features/graph/canvas";
import { graphHeader, graphSpec } from "../features/graph/view";
import { groupBody } from "../features/groups/view";
import { ownerSettingsBody } from "../features/memory/controls-view";
import { memoryEditorSpec } from "../features/memory/item-view";
import { linkEditSpec, linkSpec } from "../features/memory/links-view";
import { requireElement } from "../shared/dom";
import { button, escapeHtml, field, iconButton, notice } from "../shared/ui/html";
import type { EditorContext } from "./context";
import type { Dialog, DialogSpec, GraphDialog } from "./contracts";
/** Builds native dialog surfaces while feature renderers retain all business-specific markup. */
export function createDialogRenderer(context: EditorContext) {
  const { state, dialogsRoot, snapshot } = context;
  const graphResizeObserver = new ResizeObserver(
    /** Applies the same center-preserving resize behavior as the native graph canvas. */
    entries => {
      for (const entry of entries) {
        const surface = requireElement(entry.target, SVGSVGElement);
        const dialog = state.dialogs.find(
          /** Matches the rendered graph to its exact owner reference. */
          (item): item is GraphDialog => item.type === "graph" && item.ownerKey === surface.dataset.owner,
        );
        if (dialog !== undefined) setGraphViewport(dialog, surface);
      }
    },
  );


  /** Requires the independent catalog loaded for this editor without substituting absent theme choices. */
  function themeChoices() {
    if (state.themeChoices === null) throw new Error("主题配置目录尚未加载");
    return state.themeChoices;
  }
  /** Renders finite dialog types with their real independent theme choices. */
  function dialogSpec(dialog: Dialog): DialogSpec {
    switch (dialog.type) {
      case "character": return { title: dialog.create ? "创建角色卡" : "编辑角色卡", style: "large", tabs: ["基础", "内容", "绑定"], body: characterBody(dialog, snapshot(), state.avatarSources, themeChoices()), footer: `${!dialog.create && !dialog.card.isDefault ? button("delete-card", "删除", "", "", "danger") : ""}${!dialog.create ? button("export-card", "导出", "") : ""}<span class="spacer"></span>${button("close-dialog", "取消", "")}${button("save-card", "保存", "", "", "filled")}` };
      case "group": return { title: dialog.create ? "创建群组" : "编辑群组", style: "large", tabs: [], body: groupBody(dialog, snapshot(), themeChoices()), footer: `${!dialog.create ? button("delete-group", "删除", "", "", "danger") + button("export-group", "导出 JSON", "") : ""}<span class="spacer"></span>${button("close-dialog", "取消", "")}${button("save-group", "保存", "", "", "filled")}` };
      case "owner-settings": return { title: "记忆设置", style: "large", tabs: ["自动提取", "检索", "历史重建"], body: ownerSettingsBody(dialog), footer: button("close-dialog", "关闭", "") + button("unwired", "保存", "", "disabled", "filled") };
      case "store": return { title: dialog.create ? "创建共享记忆库" : "编辑共享记忆库", style: "compact", tabs: [], body: field("名称 *", "name", dialog.store.name), footer: button("close-dialog", "取消", "") + button("save-store", "保存", "", "", "filled") };
      case "user": return { title: `${dialog.name} 的用户资料`, style: "large", tabs: [], body: dialog.loading ? '<div class="loading">正在读取 USER.md…</div>' : `<div class="muted">${escapeHtml(dialog.ownerKey)} · USER.md</div>${field("用户资料", "content", dialog.content, 18)}${notice("保存会更新当前绑定记忆库的 USER.md。")}`, footer: button("close-dialog", "取消", "") + button("save-user", "保存", "", !dialog.loaded ? "disabled" : "", "filled") };
      case "tags": return { title: "管理标签", style: "large", tabs: [], body: tagManagerBody(dialog), footer: button("close-dialog", "完成", "") };
      case "tag-edit": return { title: dialog.create ? "创建标签" : "编辑标签", style: "compact", tabs: [], body: field("标签名称 *", "name", dialog.draft.name) + field("描述", "description", dialog.draft.description, 3) + field("提示词内容", "promptContent", dialog.draft.promptContent, 6), footer: button("close-dialog", "取消", "") + button("save-tag", "保存", "", "", "filled") };
      case "tag-delete": return { title: "删除标签", style: "compact", tabs: [], body: `<p>确定删除“${escapeHtml(dialog.name)}”？</p>`, footer: button("close-dialog", "取消", "") + button("confirm-tag-delete", "删除", "", "", "danger") };
      case "theme": return themeSpec(dialog);
      case "tts": return ttsSpec(dialog, snapshot());
      case "tool-access": return toolPolicySpec(dialog, snapshot());
      case "preview": return previewSpec(dialog, snapshot());
      case "memory-edit": return memoryEditorSpec(dialog);
      case "link-edit": return linkEditSpec(dialog);
      case "link": return linkSpec(dialog);
      case "confirm": return { title: dialog.title, style: "compact", tabs: [], body: `<p>${escapeHtml(dialog.message)}</p>`, footer: button("close-dialog", "取消", "") + button("confirm-operation", "删除", "", "", "danger") };
      case "export": return exportSpec(dialog);
      case "import": return importSpec(dialog);
      case "text": return { title: dialog.label, style: "large", tabs: [], body: field(dialog.label, "value", dialog.value, 22), footer: button("close-dialog", "取消", "") + button("save-text", "确定", "", "", "filled") };
      case "info": return { title: dialog.title, style: "large", tabs: [], body: `<pre class="notice">${escapeHtml(dialog.content)}</pre>`, footer: button("close-dialog", "关闭", "") };
      case "graph": return graphSpec(dialog);
      default: { const unknown: never = dialog; throw new Error(`Unknown dialog: ${JSON.stringify(unknown)}`); }
    }
  }

  /** Rebuilds nested native modal surfaces while retaining their draft and scroll state. */
  function renderDialogs() {
    for (const body of dialogsRoot.querySelectorAll<HTMLElement>(".dialog-body")) {
      const index = Number(body.dataset.index);
      if (index < state.dialogs.length) state.dialogs[index].scroll = body.scrollTop;
    }
    graphResizeObserver.disconnect();
    dialogsRoot.replaceChildren();
    for (let index = 0; index < state.dialogs.length; index++) {
      const dialog = state.dialogs[index], spec = dialogSpec(dialog);
      const element = document.createElement("dialog");
      element.className = `dialog ${spec.style} ${dialog.busy ? "busy" : ""}`;
      element.setAttribute("aria-label", spec.title);
      let tabs = "";
      for (let i = 0; i < spec.tabs.length; i++) tabs += `<button type="button" class="tab" role="tab" aria-selected="${"tab" in dialog && dialog.tab === i}" data-action="select-tab" data-tab="${i}">${escapeHtml(spec.tabs[i])}</button>`;
      element.innerHTML = `<div class="dialog-layout"><header class="dialog-header"><h2>${escapeHtml(spec.title)}</h2>${dialog.type === "graph" ? graphHeader(dialog) : ""}${iconButton("close-dialog", "关闭", "close", dialog.busy ? "disabled" : "")}</header>${tabs === "" ? "" : `<div class="tabs" role="tablist">${tabs}</div>`}${dialog.busy ? '<div class="busy-bar"></div>' : ""}<div class="dialog-error" role="alert" ${dialog.error === "" ? "hidden" : ""}>${escapeHtml(dialog.error)}</div>${dialog.type === "graph" ? spec.body : `<div class="dialog-body" data-index="${index}">${spec.body}</div>`}${spec.footer === "" ? "" : `<footer class="dialog-footer">${spec.footer}</footer>`}</div>`;
      if (dialog.busy) {
        for (const control of element.querySelectorAll<HTMLButtonElement | HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>("button,input,textarea,select")) control.disabled = true;
        element.setAttribute("aria-busy", "true");
      }
      dialogsRoot.append(element); element.showModal();
      const surface = element.querySelector<SVGSVGElement>("[data-graph-surface]");
      if (surface !== null && dialog.type === "graph") { surface.dataset.owner = dialog.ownerKey; setGraphViewport(dialog, surface); graphResizeObserver.observe(surface); }
      const body = element.querySelector(".dialog-body");
      if (body !== null) body.scrollTop = dialog.scroll;
    }
  }
  return { renderDialogs };
}
