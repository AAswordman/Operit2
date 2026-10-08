import type { CurrentSidebar, SidebarChatSummary, SidebarCatalog } from "../../../src/ui-sidebar";

/** Describes a projected conversation group for rendering, not its backend persistence or namespace schema. */
export interface SidebarGroupView {
  id: string; name: string; pinned: boolean; displayOrder: number; chats: SidebarChatSummary[];
}
/** Keeps same-name groups in distinct owner-defined scopes without imposing a global naming rule. */
export interface SidebarGroupScopeView {
  id: string; title: string; groups: SidebarGroupView[]; ungrouped: SidebarChatSummary[];
}
/** Requires the data projection to match the explicit host-selected sidebar tab. */
export type SidebarViewData = SidebarCatalog;
/** Records actual deletion outcomes for inspection; it never claims a transaction across native chat calls. */
export interface SidebarDeletionProgress { deletedChatIds: string[]; failedChatIds: { chatId: string; error: string }[]; metadataDeleted: boolean; metadataError: string | null }
/** Defines staged local form state independently of backend operation input fields. */
export type SidebarDialog =
  | { type: "create-group"; scopeId: string; name: string }
  | { type: "rename-group"; groupId: string; name: string }
  | { type: "delete-group"; groupId: string; name: string; memberCount: number; progress: SidebarDeletionProgress | null }
  | { type: "move-chat"; chatId: string; groupId: string | null; choices: { id: string; title: string }[] };
/** Holds only local view state alongside the exactly validated generic host context. */
export interface SidebarViewState {
  current: CurrentSidebar; data: SidebarViewData | null; query: string; collapsedSections: Set<string>;
  expandedGroups: Set<string>; dialog: SidebarDialog | null; error: string; busy: boolean;
}
/** Encodes finite UI intent without specifying Core create-chat or plugin namespace-storage parameters. */
export type SidebarIntent =
  | { type: "activate-chat"; chatId: string }
  | { type: "toggle-section"; sectionId: string }
  | { type: "toggle-group"; groupId: string }
  | { type: "create-group"; scopeId: string }
  | { type: "rename-group" | "pin-group" | "delete-group"; groupId: string }
  | { type: "reorder-group"; groupId: string; direction: "up" | "down" }
  | { type: "move-chat"; chatId: string }
  | { type: "create-chat"; sectionId: string; groupId: string | null }
  | { type: "submit-dialog" | "cancel-dialog" | "refresh" };
/** Injects actual feature actions later without manufacturing successful operations in the pure UI. */
export interface SidebarUiActions {
  /** Handles exactly one finite intent and propagates its actual failure. */
  dispatch(intent: SidebarIntent): Promise<void>;
}
