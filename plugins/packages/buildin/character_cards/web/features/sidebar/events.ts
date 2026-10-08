import { dataValue, requireElement } from "../../shared/dom";
import type { SidebarIntent, SidebarUiActions, SidebarViewState } from "./contracts";

/** Converts exact DOM action metadata into finite sidebar intent, including SVG-originated clicks. */
export function sidebarIntent(element: HTMLElement): SidebarIntent {
  switch (dataValue(element, "action")) {
    case "sidebar-activate-chat": return { type: "activate-chat", chatId: dataValue(element, "chatId") };
    case "sidebar-toggle-section": return { type: "toggle-section", sectionId: dataValue(element, "sectionId") };
    case "sidebar-toggle-group": return { type: "toggle-group", groupId: dataValue(element, "groupId") };
    case "sidebar-create-group": return { type: "create-group", scopeId: dataValue(element, "scopeId") };
    case "sidebar-rename-group": return { type: "rename-group", groupId: dataValue(element, "groupId") };
    case "sidebar-pin-group": return { type: "pin-group", groupId: dataValue(element, "groupId") };
    case "sidebar-delete-group": return { type: "delete-group", groupId: dataValue(element, "groupId") };
    case "sidebar-group-up": return { type: "reorder-group", groupId: dataValue(element, "groupId"), direction: "up" };
    case "sidebar-group-down": return { type: "reorder-group", groupId: dataValue(element, "groupId"), direction: "down" };
    case "sidebar-move-chat": return { type: "move-chat", chatId: dataValue(element, "chatId") };
    case "sidebar-create-chat": {
      const groupId = dataValue(element, "groupId");
      return { type: "create-chat", sectionId: dataValue(element, "sectionId"), groupId: groupId === "" ? null : groupId };
    }
    case "sidebar-submit-dialog": return { type: "submit-dialog" };
    case "sidebar-cancel-dialog": return { type: "cancel-dialog" };
    case "sidebar-refresh": return { type: "refresh" };
    default: throw new Error("Unknown sidebar action: " + dataValue(element, "action"));
  }
}
/** Installs owned finite DOM listeners; mutations are delivered only to the explicitly injected actual action owner. */
export function installSidebarEvents(root: HTMLElement, state: SidebarViewState, actions: SidebarUiActions, render: () => void): () => void {
  /** Delivers one exact intent and makes any actual rejection visible without a successful substitute result. */
  async function dispatch(intent: SidebarIntent): Promise<void> {
    try { await actions.dispatch(intent); }
    catch (failure) { state.error = String(failure); render(); }
  }
  /** Resolves SVG clicks to their button while ignoring ordinary non-action page interactions. */
  function onClick(event: Event): void {
    if (!(event.target instanceof Element)) return;
    const target = event.target.closest("[data-action]");
    if (target === null || !root.contains(target)) return;
    const element = requireElement(target, HTMLElement);
    if (element instanceof HTMLButtonElement && element.disabled) return;
    void dispatch(sidebarIntent(element));
  }
  /** Updates staged local search or dialog fields without persisting intermediate chooser state. */
  function onInput(event: Event): void {
    if (!(event.target instanceof HTMLInputElement) && !(event.target instanceof HTMLSelectElement)) return;
    const element = event.target;
    if (element.hasAttribute("data-sidebar-query")) {
      state.query = element.value; render();
      const search = requireElement(root.querySelector("[data-sidebar-query]"), HTMLInputElement);
      search.focus(); return;
    }
    const key = element.dataset.field;
    if (key === undefined) return;
    if (state.dialog === null) throw new Error("Sidebar dialog field has no actual owning dialog");
    switch (key) {
      case "sidebar-group-name":
        if (state.dialog.type !== "create-group" && state.dialog.type !== "rename-group") throw new Error("Group-name field is not owned by a name editor");
        state.dialog.name = element.value; return;
      case "sidebar-target-group":
        if (state.dialog.type !== "move-chat") throw new Error("Group-target field is not owned by the move dialog");
        state.dialog.groupId = element.value === "" ? null : element.value; return;
      default: throw new Error("Unknown sidebar dialog field: " + key);
    }
  }
  root.addEventListener("click", onClick); root.addEventListener("input", onInput); root.addEventListener("change", onInput);
  /** Releases only listeners owned by this embedded sidebar instance. */
  return function dispose(): void { root.removeEventListener("click", onClick); root.removeEventListener("input", onInput); root.removeEventListener("change", onInput); };
}
