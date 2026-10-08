import type { CurrentSidebar } from "../../../src/ui-sidebar";
import type { SidebarViewState } from "./contracts";

/** Starts an explicit embedded tab with unloaded data, never an inferred management view or empty business catalog. */
export function createSidebarViewState(current: CurrentSidebar): SidebarViewState {
  return { current, data: null, query: "", collapsedSections: new Set(), expandedGroups: new Set(), dialog: null, error: "", busy: false };
}
