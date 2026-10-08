import type { ConversationGroupRecord, DomainInput, DomainOperation, DomainOutput } from "../src/api";
import { parseCurrentSidebar, type CurrentSidebar, type SidebarCatalog, type ConversationSidebarScope } from "../src/ui-sidebar";
import type { Theme } from "../src/model";
import { createSidebarController } from "./features/sidebar/controller";
import type { SidebarIntent } from "./features/sidebar/contracts";
import { deleteConversationGroupMembers } from "./features/sidebar/groups/delete";
import { applyMaterialTheme } from "./shared/ui/material";

/** Declares only actual interfaces registered by the embedded Compose sidebar. */
interface SidebarHost {
  currentTheme(): Promise<Theme>;
  currentSidebar(): Promise<CurrentSidebar>;
  catalog(): Promise<SidebarCatalog>;
  domain<K extends DomainOperation>(message: { operation: K; payload: DomainInput<K> }): Promise<DomainOutput<K>>;
  activateChat(chatId: string): Promise<unknown>;
  createChat(selection: string, groupId: string | null): Promise<unknown>;
  deleteChat(chatId: string): Promise<{ chatId: string; deletedAt: number }>;
}
declare global {
  interface Window {
    CharacterSidebarHost?: SidebarHost;
    applyCharacterSidebarTheme(theme: Theme): void;
    updateCharacterSidebar(current: CurrentSidebar): Promise<void>;
  }
}

/** Requires the registered sidebar ABI without selecting another transport. */
function host(): SidebarHost {
  const value = window.CharacterSidebarHost;
  if (value === undefined) throw new Error("CharacterSidebarHost is not registered");
  for (const name of ["currentTheme", "currentSidebar", "catalog", "domain", "activateChat", "createChat", "deleteChat"] as const) {
    if (typeof value[name] !== "function") throw new Error("Missing sidebar host method: " + name);
  }
  return value;
}

/** Mounts real catalog projections and sends finite actions to their actual owners. */
export async function initializeSidebar(): Promise<void> {
  const deadline = performance.now() + 5000;
  while (window.CharacterSidebarHost === undefined) {
    if (performance.now() >= deadline) throw new Error("CharacterSidebarHost registration timed out");
    await new Promise<void>(
      /** Waits for the one WebView ABI registration, not an alternate implementation. */
      resolve => setTimeout(resolve, 20),
    );
  }
  const bridge = host(), element = document.getElementById("app");
  if (element === null) throw new Error("The sidebar document has no app root");
  window.applyCharacterSidebarTheme = applyMaterialTheme;
  window.applyCharacterSidebarTheme(await bridge.currentTheme());
  const controller = createSidebarController(element, parseCurrentSidebar(await bridge.currentSidebar()), { dispatch });

  /** Reads the actual catalog for the exact current host context. */
  async function refresh(): Promise<void> {
    const owner = JSON.stringify(controller.state.current), data = await bridge.catalog();
    if (owner !== JSON.stringify(controller.state.current)) throw new Error("Sidebar context changed during catalog read");
    controller.setData(data);
  }
  /** Requires loaded grouping data and its exact scope identity. */
  function scope(id: string): ConversationSidebarScope {
    const data = controller.state.data;
    if (data === null || data.view !== "groups") throw new Error("Conversation-group catalog is not loaded");
    const value = data.scopes.find(
      /** Selects an exact scope rather than inferring it from a group title. */
      candidate => candidate.id === id,
    );
    if (value === undefined) throw new Error("Unknown conversation-group scope: " + id);
    return value;
  }
  /** Requires the one projected group owner in the current complete catalog. */
  function groupScope(id: string): ConversationSidebarScope {
    const data = controller.state.data;
    if (data === null || data.view !== "groups") throw new Error("Conversation-group catalog is not loaded");
    const matches = data.scopes.filter(
      /** Uses exact persisted group ids while retaining scoped duplicate names. */
      candidate => candidate.groups.some(
        /** Identifies membership in the projected scope. */
        group => group.id === id,
      ),
    );
    if (matches.length !== 1) throw new Error("Conversation group has no unique scope: " + id);
    return matches[0];
  }
  /** Reads the real persisted record through the sole main-runtime service. */
  async function readGroup(id: string, owner: ConversationSidebarScope): Promise<ConversationGroupRecord> {
    const records = await bridge.domain({ operation: "conversation-group.list", payload: { ownerSelection: owner.ownerSelection } });
    const record = records.find(
      /** Requires the explicitly selected persisted id. */
      candidate => candidate.id === id,
    );
    if (record === undefined) throw new Error("Conversation group no longer exists: " + id);
    return record;
  }
  /** Applies an explicitly staged dialog once and preserves partial native deletion evidence. */
  async function submitDialog(): Promise<void> {
    const dialog = controller.state.dialog;
    if (dialog === null) throw new Error("No sidebar dialog is open");
    switch (dialog.type) {
      case "create-group": await bridge.domain({ operation: "conversation-group.create", payload: { ownerSelection: scope(dialog.scopeId).ownerSelection, name: dialog.name, pinned: false } }); break;
      case "rename-group": await bridge.domain({ operation: "conversation-group.update", payload: { id: dialog.groupId, changes: { name: dialog.name } } }); break;
      case "move-chat": {
        const data = controller.state.data;
        if (data === null || data.view !== "groups") throw new Error("Conversation-group catalog is not loaded");
        const owners = data.scopes.filter(
          /** Finds the chat only in its actual projected native membership. */
          candidate => candidate.ungrouped.some(
            /** Matches the exact ungrouped chat. */
            chat => chat.id === dialog.chatId,
          ) || candidate.groups.some(
            /** Matches the exact grouped chat. */
            group => group.chats.some(
              /** Identifies the retained native conversation. */
              chat => chat.id === dialog.chatId,
            ),
          ),
        );
        if (owners.length !== 1) throw new Error("Chat has no unique conversation-group scope");
        await bridge.domain({ operation: "conversation-group.moveChat", payload: { chatId: dialog.chatId, groupId: dialog.groupId, ownerSelection: owners[0].ownerSelection } }); break;
      }
      case "delete-group": {
        if (dialog.progress !== null) throw new Error("Confirmed group deletion has already been attempted");
        const owner = groupScope(dialog.groupId), record = await readGroup(dialog.groupId, owner);
        dialog.progress = await deleteConversationGroupMembers(record, {
          /** Executes actual native chat deletion without simulating its receipt. */
          deleteChat: id => bridge.deleteChat(id),
          /** Reads actual remaining memberships after native deletion callbacks. */
          readGroup: id => readGroup(id, owner),
          /** Removes only the confirmed empty plugin record. */
          deleteMetadata: id => bridge.domain({ operation: "conversation-group.delete", payload: { id } }),
        });
        await refresh(); return;
      }
    }
    controller.state.dialog = null; await refresh();
  }
  /** Dispatches exact sidebar intents without command-string construction or guessed defaults. */
  async function dispatch(intent: SidebarIntent): Promise<void> {
    const state = controller.state;
    if (state.busy) throw new Error("A sidebar operation is already running");
    state.error = ""; state.busy = true; controller.render();
    try {
      switch (intent.type) {
        case "refresh": await refresh(); break;
        case "activate-chat": await bridge.activateChat(intent.chatId); break;
        case "toggle-section": if (state.collapsedSections.has(intent.sectionId)) state.collapsedSections.delete(intent.sectionId); else state.collapsedSections.add(intent.sectionId); break;
        case "toggle-group": if (state.expandedGroups.has(intent.groupId)) state.expandedGroups.delete(intent.groupId); else state.expandedGroups.add(intent.groupId); break;
        case "cancel-dialog": state.dialog = null; break;
        case "create-group": scope(intent.scopeId); state.dialog = { type: "create-group", scopeId: intent.scopeId, name: "" }; break;
        case "rename-group": { const record = await readGroup(intent.groupId, groupScope(intent.groupId)); state.dialog = { type: "rename-group", groupId: record.id, name: record.name }; break; }
        case "delete-group": { const record = await readGroup(intent.groupId, groupScope(intent.groupId)); state.dialog = { type: "delete-group", groupId: record.id, name: record.name, memberCount: record.chatIds.length, progress: null }; break; }
        case "pin-group": { const record = await readGroup(intent.groupId, groupScope(intent.groupId)); await bridge.domain({ operation: "conversation-group.update", payload: { id: record.id, changes: { pinned: !record.pinned } } }); await refresh(); break; }
        case "reorder-group": {
          const owner = groupScope(intent.groupId), records = await bridge.domain({ operation: "conversation-group.list", payload: { ownerSelection: owner.ownerSelection } });
          const ids = records.map(
            /** Retains the actual complete persisted scope order. */
            record => record.id,
          ), index = ids.indexOf(intent.groupId), target = index + (intent.direction === "up" ? -1 : 1);
          if (index < 0 || target < 0 || target >= ids.length) throw new Error("Conversation group cannot move beyond its scope order");
          [ids[index], ids[target]] = [ids[target], ids[index]];
          await bridge.domain({ operation: "conversation-group.reorder", payload: { ownerSelection: owner.ownerSelection, ids } }); await refresh(); break;
        }
        case "move-chat": {
          const data = state.data;
          if (data === null || data.view !== "groups") throw new Error("Conversation-group catalog is not loaded");
          const choices = [{ id: "", title: "未分组" }];
          for (const owner of data.scopes) for (const group of owner.groups) choices.push({ id: group.id, title: owner.title + " / " + group.name });
          state.dialog = { type: "move-chat", chatId: intent.chatId, groupId: null, choices }; break;
        }
        case "create-chat": {
          const data = state.data;
          if (data === null) throw new Error("Sidebar catalog is not loaded");
          let selection: string | null;
          if (data.view === "groups") selection = scope(intent.sectionId).ownerSelection;
          else {
            const section = data.sections.find(
              /** Requires the exact selected character category. */
              value => value.id === intent.sectionId,
            );
            if (section === undefined) throw new Error("Unknown character sidebar category");
            selection = section.selection;
          }
          if (selection === null) throw new Error("Creating a sidebar chat requires an explicit character selection");
          await bridge.createChat(selection, intent.groupId); break;
        }
        case "submit-dialog": await submitDialog(); break;
      }
    } finally { state.busy = false; controller.render(); }
  }
  /** Publishes actual native context changes before reading their matching plugin projection. */
  window.updateCharacterSidebar = async function updateCharacterSidebar(current: CurrentSidebar): Promise<void> {
    controller.setContext(parseCurrentSidebar(current));
    try { await refresh(); } catch (failure) { controller.state.error = String(failure); controller.render(); throw failure; }
  };
  await refresh();
}

/** Makes genuine startup failures visible without publishing an empty successful catalog. */
async function start(): Promise<void> {
  try { await initializeSidebar(); }
  catch (failure) {
    const element = document.getElementById("app");
    if (element === null) throw failure;
    element.textContent = String(failure); element.setAttribute("role", "alert");
  }
}
void start();
