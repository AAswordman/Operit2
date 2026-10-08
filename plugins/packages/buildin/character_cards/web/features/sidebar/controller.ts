import type { CurrentSidebar } from "../../../src/ui-sidebar";
import type { SidebarUiActions, SidebarViewData } from "./contracts";
import { installSidebarEvents } from "./events";
import { createSidebarViewState } from "./state";
import { sidebarView } from "./view";

/** Creates a real DOM sidebar renderer while requiring an explicit actual action adapter from its caller. */
export function createSidebarController(root: HTMLElement, current: CurrentSidebar, actions: SidebarUiActions) {
  const state = createSidebarViewState(current);
  /** Repaints the currently selected tab without manufacturing an unloaded catalog. */
  function render(): void { root.innerHTML = sidebarView(state); }
  const dispose = installSidebarEvents(root, state, actions, render);
  render();
  return {
    state, render, dispose,
    /** Publishes one actual projected catalog only for the explicit current tab. */
    setData(data: SidebarViewData): void {
      if (data.view !== state.current.input.view) throw new Error("Sidebar data belongs to another registered tab");
      state.data = data; render();
    },
    /** Updates generic host context; business state is reloaded separately by its actual adapter. */
    setContext(value: CurrentSidebar): void {
      if (value.input.view !== state.current.input.view) throw new Error("An embedded sidebar instance cannot switch its opaque registered tab input");
      state.current = value; render();
    },
  };
}
