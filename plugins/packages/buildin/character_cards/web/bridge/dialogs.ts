import { exportSpec, importSpec } from "../features/characters/import-export-view";
import { setGraphViewport } from "../features/graph/canvas";
import { cancelGraphCameraAnimation } from "../features/graph/camera";
import { graphHeader, graphSpec } from "../features/graph/view";
import { ownerSettingsBody, ownerSettingsFooter } from "../features/memory/controls-view";
import { memoryEditorSpec } from "../features/memory/item-view";
import { linkEditSpec, linkSpec } from "../features/memory/links-view";
import { requireElement } from "../shared/dom";
import { button, escapeHtml, field, iconButton, notice } from "../shared/ui/html";
import type { EditorContext } from "./context";
import type { Dialog, DialogSpec, GraphDialog } from "./contracts";
/** Builds native dialog surfaces while feature renderers retain all business-specific markup. */
export function createDialogRenderer(context: EditorContext) {
  const { state, dialogsRoot } = context;
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


  /** Renders finite dialog types with their real independent theme choices. */
  function dialogSpec(dialog: Dialog): DialogSpec {
    switch (dialog.type) {
      case "owner-settings": return { title: "记忆设置", style: "large", tabs: ["自动提取", "检索", "历史重建"], body: ownerSettingsBody(dialog), footer: ownerSettingsFooter(dialog) };
      case "store": return { title: dialog.create ? "创建共享记忆库" : "编辑共享记忆库", style: "compact", tabs: [], body: field("名称 *", "name", dialog.store.name), footer: button("close-dialog", "取消", "") + button("save-store", "保存", "", "", "filled") };
      case "user": return { title: `${dialog.name} 的用户资料`, style: "large", tabs: [], body: dialog.loading ? '<div class="loading">正在读取 USER.md…</div>' : `<div class="muted">${escapeHtml(dialog.ownerKey)} · USER.md</div>${field("用户资料", "content", dialog.content, 18)}${notice("保存会更新当前绑定记忆库的 USER.md。")}`, footer: button("close-dialog", "取消", "") + button("save-user", "保存", "", !dialog.loaded ? "disabled" : "", "filled") };
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

  // Keep one native modal / backdrop in the WebView top layer. Parent drafts
  // remain in the stack without reopening every surface on each repaint.
  const surfaces = new Map<Dialog, HTMLDialogElement>();
  const markup = new WeakMap<HTMLDialogElement, string>();

  /** Retains parent drafts and scroll while showing only the top dialog as a native modal. */
  function renderDialogs() {
    const active = state.dialogs[state.dialogs.length - 1];
    for (const [dialog, element] of surfaces) {
      if (dialog !== active && dialog.type === "graph") cancelGraphCameraAnimation(dialog);
      if (element.open) {
        const body = element.querySelector<HTMLElement>(".dialog-body");
        if (body !== null) dialog.scroll = body.scrollTop;
      }
      if (dialog !== active && element.open) element.close();
      if (!state.dialogs.includes(dialog)) { dialog.dispose?.(); element.remove(); surfaces.delete(dialog); }
    }
    graphResizeObserver.disconnect();
    if (active === undefined) return;
    const dialog = active, index = state.dialogs.length - 1, spec = dialogSpec(dialog);
    let element = surfaces.get(dialog);
    if (element === undefined) {
      element = document.createElement("dialog");
      surfaces.set(dialog, element); dialogsRoot.append(element);
    }
    element.className = `dialog ${spec.style} ${dialog.busy ? "busy" : ""}`;
    element.setAttribute("aria-label", spec.title);
    let tabs = "";
    for (let i = 0; i < spec.tabs.length; i++) tabs += `<button type="button" class="tab" role="tab" aria-selected="${"tab" in dialog && dialog.tab === i}" data-action="select-tab" data-tab="${i}">${escapeHtml(spec.tabs[i])}</button>`;
    const content = `<div class="dialog-layout"><header class="dialog-header"><h2>${escapeHtml(spec.title)}</h2>${dialog.type === "graph" ? graphHeader(dialog) : ""}${iconButton("close-dialog", "关闭", "close", dialog.busy ? "disabled" : "")}</header>${tabs === "" ? "" : `<div class="tabs" role="tablist">${tabs}</div>`}${dialog.busy ? '<div class="busy-bar"></div>' : ""}<div class="dialog-error" role="alert" ${dialog.error === "" ? "hidden" : ""}>${escapeHtml(dialog.error)}</div>${dialog.type === "graph" ? spec.body : `<div class="dialog-body" data-index="${index}">${spec.body}</div>`}${spec.footer === "" ? "" : `<footer class="dialog-footer">${spec.footer}</footer>`}</div>`;
    // Do not replace unchanged controls after a click: native focus and the
    // modal's top-layer membership must survive routine final repainting.
    if (markup.get(element) !== content) { element.innerHTML = content; markup.set(element, content); }
    for (const control of element.querySelectorAll<HTMLButtonElement | HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>("button,input,textarea,select")) {
      if (dialog.busy) control.disabled = true;
    }
    if (dialog.busy) element.setAttribute("aria-busy", "true");
    else element.removeAttribute("aria-busy");
    if (!element.open) element.showModal();
    const surface = element.querySelector<SVGSVGElement>("[data-graph-surface]");
    if (surface !== null && dialog.type === "graph") { surface.dataset.owner = dialog.ownerKey; setGraphViewport(dialog, surface); graphResizeObserver.observe(surface); }
    const body = element.querySelector(".dialog-body");
    if (body !== null) body.scrollTop = dialog.scroll;
  }
  return { renderDialogs };
}
