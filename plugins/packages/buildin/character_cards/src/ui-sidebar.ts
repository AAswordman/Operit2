import type { SnapshotRecord } from "./api";
import { record } from "./domain";
import { decodeSelection, encodeSelection } from "./ui-contributions";
import { decodeChatMarker } from "./chat-markers";
import { requireChatSelection } from "./chat-bindings";
import { getService } from "./service-runtime";

/** Selects one plugin-owned embedded sidebar view without creating a modal presentation. */
export interface SidebarInput { view: "characters" | "groups" }
/** Contains only the neutral chat metadata supplied by the existing native history source. */
export interface SidebarChatSummary {
  id: string; title: string; updatedAt: string; displayOrder: number;
  workspaceId: string | null; workspaceName: string | null; locked: boolean; pinned: boolean; group: string | null;
}
/** Describes the exact live host context of an embedded chat sidebar. */
export interface ChatSidebarContext { chats: SidebarChatSummary[]; currentChatId: string | null; activeStreamingChatIds: string[] }
/** Carries role route input separately from canonical native history state. */
export interface CurrentSidebar { input: SidebarInput; chatSidebar: ChatSidebarContext }
/** Requests actual host navigation without exposing plugin selections or group membership. */
export interface ChatActivateAction { type: "toolpkg.chat.activate"; chatId: string }
/** Describes one persisted character or role-group category owned entirely by the plugin. */
export interface CharacterSidebarSection {
  id: string; title: string; avatarUri: string | null; kind: "card" | "group" | "unbound";
  selection: string | null; chats: SidebarChatSummary[];
  conversationGroups?: ConversationSidebarGroup[]; ungroupedChats?: SidebarChatSummary[];
}
/** Supplies explicit plugin selection state to a pure view without defining a storage or namespace protocol. */
export interface SidebarChatSelection { chatId: string; selection: string | null }
/** Supplies real directory records and one explicit selection state for each host chat to the category projection. */
export interface CharacterSidebarData { directory: SnapshotRecord; selections: SidebarChatSelection[] }

/** Projects canonical native folder membership for role presentation only. */
export interface ConversationSidebarGroup { id: string; name: string; pinned: boolean; displayOrder: number; chats: SidebarChatSummary[] }
/** Describes a role category containing native folders; no folder storage belongs to the plugin. */
export interface ConversationSidebarScope { id: string; title: string; ownerSelection: string | null; avatarUri?: string | null; kind?: CharacterSidebarSection["kind"]; groups: ConversationSidebarGroup[]; ungrouped: SidebarChatSummary[] }
/** Returns only the actual catalog of the explicitly registered sidebar tab. */
export type SidebarCatalog = { view: "characters"; sections: CharacterSidebarSection[] } | { view: "groups"; scopes: ConversationSidebarScope[] };

/** Requires exactly the documented keys instead of interpreting unknown host business fields. */
function exactFields(value: unknown, expected: readonly string[], label: string): Record<string, unknown> {
  const object = record(value, label), keys = Object.keys(object);
  if (keys.length !== expected.length || keys.some(
    /** Rejects every field not explicitly declared by this boundary. */
    key => expected.indexOf(key) === -1,
  )) throw new Error(label + " has invalid fields");
  return object;
}
/** Requires an actual generic identity without selecting another chat. */
export function sidebarIdentity(value: unknown, label: string): string {
  if (typeof value !== "string" || value.trim() === "") throw new Error(label + " must be a nonblank string");
  return value;
}
/** Validates the sole plugin-owned route-input discriminator. */
export function parseSidebarInput(value: unknown): SidebarInput {
  const input = exactFields(value, ["view"], "sidebar input");
  if (input.view !== "characters" && input.view !== "groups") throw new Error("sidebar input.view must be characters or groups");
  return { view: input.view };
}
/** Validates a neutral host history summary and rejects plugin role-binding fields. */
export function parseSidebarChat(value: unknown): SidebarChatSummary {
  const chat = exactFields(value, ["id", "title", "updatedAt", "displayOrder", "workspaceId", "workspaceName", "locked", "pinned", "group"], "sidebar chat");
  if (chat.group !== null && typeof chat.group !== "string") throw new Error("sidebar chat.group must be a string or null");
  const id = sidebarIdentity(chat.id, "sidebar chat.id");
  if (typeof chat.title !== "string" || typeof chat.updatedAt !== "string") throw new Error("sidebar chat title and updatedAt must be strings");
  if (typeof chat.displayOrder !== "number" || !Number.isSafeInteger(chat.displayOrder)) throw new Error("sidebar chat.displayOrder must be a safe integer");
  if (chat.workspaceId !== null && typeof chat.workspaceId !== "string") throw new Error("sidebar chat.workspaceId must be a string or null");
  if (chat.workspaceName !== null && typeof chat.workspaceName !== "string") throw new Error("sidebar chat.workspaceName must be a string or null");
  if (typeof chat.locked !== "boolean" || typeof chat.pinned !== "boolean") throw new Error("sidebar chat locked and pinned must be booleans");
  return { id, title: chat.title, updatedAt: chat.updatedAt, displayOrder: chat.displayOrder, workspaceId: chat.workspaceId, workspaceName: chat.workspaceName, locked: chat.locked, pinned: chat.pinned, group: chat.group };
}
/** Validates the live generic host state without defaulting missing state to a management screen. */
export function parseChatSidebarContext(value: unknown): ChatSidebarContext {
  const context = exactFields(value, ["chats", "currentChatId", "activeStreamingChatIds"], "chatSidebar");
  if (!Array.isArray(context.chats)) throw new Error("chatSidebar.chats must be an array");
  if (!Array.isArray(context.activeStreamingChatIds)) throw new Error("chatSidebar.activeStreamingChatIds must be an array");
  const chats: SidebarChatSummary[] = [], ids = new Set<string>(), streaming = new Set<string>();
  for (const raw of context.chats) {
    const chat = parseSidebarChat(raw);
    if (ids.has(chat.id)) throw new Error("Duplicate sidebar chat: " + chat.id);
    ids.add(chat.id); chats.push(chat);
  }
  for (const raw of context.activeStreamingChatIds) {
    const id = sidebarIdentity(raw, "activeStreamingChatIds entry");
    if (streaming.has(id)) throw new Error("Duplicate active streaming chat: " + id);
    streaming.add(id);
  }
  const currentChatId = context.currentChatId === null ? null : sidebarIdentity(context.currentChatId, "chatSidebar.currentChatId");
  return { chats, currentChatId, activeStreamingChatIds: [...streaming] };
}
/** Parses both explicit inputs of the embedded route, without presentation or mode inference. */
export function parseCurrentSidebar(value: unknown): CurrentSidebar {
  const current = exactFields(value, ["input", "chatSidebar"], "current sidebar");
  return { input: parseSidebarInput(current.input), chatSidebar: parseChatSidebarContext(current.chatSidebar) };
}
/** Accepts only the exact generic navigation action understood by the owning sidebar host. */
export function parseChatActivateAction(value: unknown): ChatActivateAction {
  const action = exactFields(value, ["type", "chatId"], "chat activate action");
  if (action.type !== "toolpkg.chat.activate") throw new Error("Invalid chat activation discriminator");
  return { type: action.type, chatId: sidebarIdentity(action.chatId, "chat activate action.chatId") };
}

/** Orders actual chat summaries by persisted pin and display order, retaining stable identity ties. */
export function compareSidebarChats(left: SidebarChatSummary, right: SidebarChatSummary): number {
  return Number(right.pinned) - Number(left.pinned) || left.displayOrder - right.displayOrder || left.id.localeCompare(right.id);
}
/** Projects explicit namespace-derived selection state into plugin categories without defining its storage protocol. */
export function characterSidebarSections(directory: Pick<SnapshotRecord, "cards" | "groups">, selections: readonly SidebarChatSelection[], chats: readonly SidebarChatSummary[]): CharacterSidebarSection[] {
  const sections: CharacterSidebarSection[] = [], bySelection = new Map<string, CharacterSidebarSection>();
  for (const card of directory.cards) {
    const selection = encodeSelection({ CharacterCard: { id: card.id } });
    if (bySelection.has(selection)) throw new Error("Duplicate persisted character category: " + selection);
    const section: CharacterSidebarSection = { id: selection, title: card.name, avatarUri: card.avatarUri, kind: "card", selection, chats: [] };
    sections.push(section); bySelection.set(selection, section);
  }
  for (const group of directory.groups) {
    const selection = encodeSelection({ CharacterGroup: { id: group.id } });
    if (bySelection.has(selection)) throw new Error("Duplicate persisted role-group category: " + selection);
    const section: CharacterSidebarSection = { id: selection, title: group.name, avatarUri: null, kind: "group", selection, chats: [] };
    sections.push(section); bySelection.set(selection, section);
  }
  const selectionByChat = new Map<string, SidebarChatSelection>();
  for (const state of selections) {
    if (selectionByChat.has(state.chatId)) throw new Error("Duplicate sidebar selection state: " + state.chatId);
    if (state.selection !== null) decodeSelection(state.selection, directory);
    selectionByChat.set(state.chatId, state);
  }
  const unbound: CharacterSidebarSection = { id: "unbound", title: "未绑定角色", avatarUri: null, kind: "unbound", selection: null, chats: [] };
  for (const chat of chats) {
    const state = selectionByChat.get(chat.id);
    if (state === undefined) throw new Error("Sidebar has no explicit namespace selection state for chat: " + chat.id);
    if (state.selection === null) { unbound.chats.push(chat); continue; }
    const section = bySelection.get(state.selection);
    if (section === undefined) throw new Error("Plugin selection has no real category: " + state.selection);
    section.chats.push(chat);
  }
  if (unbound.chats.length !== 0) sections.push(unbound);
  for (const section of sections) section.chats.sort(compareSidebarChats);
  return sections;
}

/** Projects only the actual members of one explicit role or unbound scope and preserves persisted empty groups. */
export function conversationSidebarScope(section: CharacterSidebarSection): ConversationSidebarScope {
  const byName = new Map<string, SidebarChatSummary[]>(), ungrouped: SidebarChatSummary[] = [];
  for (const chat of section.chats) {
    const name = chat.group?.trim();
    if (!name) { ungrouped.push(chat); continue; }
    const members = byName.get(name) ?? []; members.push(chat); byName.set(name, members);
  }
  const groups: ConversationSidebarGroup[] = [...byName].map(([name, chats]) => ({
    id: section.id + ":folder:" + JSON.stringify(name), name,
    pinned: chats.length > 0 && chats.every(chat => chat.pinned),
    displayOrder: Math.min(...chats.map(chat => chat.displayOrder)), chats: chats.sort(compareSidebarChats),
  }));
  groups.sort((a,b) => Number(b.pinned)-Number(a.pinned) || a.displayOrder-b.displayOrder || a.name.localeCompare(b.name));
  return { id: section.id, title: section.title, ownerSelection: section.selection, avatarUri: section.avatarUri,
    kind: section.kind, groups, ungrouped: ungrouped.sort(compareSidebarChats) };
}

/** Reuses known role identities while applying live native metadata; unknown chats await authoritative classification. */
export function reconcileSidebarCatalog(catalog: SidebarCatalog, current: CurrentSidebar): SidebarCatalog {
  const live = new Map(current.chatSidebar.chats.map(chat => [chat.id, chat] as const));
  const sections: CharacterSidebarSection[] = catalog.view === "characters" ? catalog.sections : catalog.scopes.map(scope => ({
    id: scope.id, title: scope.title, kind: scope.kind ?? "unbound", selection: scope.ownerSelection, avatarUri: scope.avatarUri ?? null,
    chats: [...scope.ungrouped, ...scope.groups.flatMap(group => group.chats)], conversationGroups: scope.groups,
  }));
  const projected = sections.map(section => {
    const chats = section.chats.flatMap(chat => { const updated = live.get(chat.id); return updated === undefined ? [] : [updated]; }).sort(compareSidebarChats);
    const scope = conversationSidebarScope({...section, chats});
    for (const group of scope.groups) group.id = section.conversationGroups?.find(previous => previous.name === group.name)?.id ?? group.id;
    return {section: {...section, chats, conversationGroups: scope.groups, ungroupedChats: scope.ungrouped}, scope};
  });
  return catalog.view === "characters" ? {view: "characters", sections: projected.map(value => value.section)}
    : {view: "groups", scopes: projected.map(value => value.scope)};
}

/** Reads the actual sidebar catalog from namespace extensions and the same database-backed service used by every other plugin surface. */
export async function readSidebarCatalog(value: CurrentSidebar): Promise<SidebarCatalog> {
  const current = parseCurrentSidebar(value), service = await getService();
  const directory = await service.sidebarDirectory();
  const selections: SidebarChatSelection[] = [];
  // Bound outstanding bridge requests instead of paying one round trip per chat in series.
  // Read every extension afresh so rebinding the same IDs cannot leave a stale classification.
  const chats = current.chatSidebar.chats;
  for (let offset = 0; offset < chats.length; offset += 8) {
    selections.push(...await Promise.all(chats.slice(offset, offset + 8).map(async chat => {
      const extension = await Tools.Chat.readExtension({ kind: "chat", chatId: chat.id });
      if (extension === null) return { chatId: chat.id, selection: null };
      const marker = decodeChatMarker(extension);
      requireChatSelection(marker.selection, directory.cards, directory.groups);
      return { chatId: chat.id, selection: marker.selection };
    })));
  }
  const sections = characterSidebarSections(directory, selections, current.chatSidebar.chats);
  if (!sections.some(
    /** Ensures an explicit unbound grouping scope remains available for real empty group creation. */
    section => section.selection === null,
  )) {
    const unbound: CharacterSidebarSection = { id: "unbound", title: "未绑定角色", avatarUri: null, kind: "unbound", selection: null, chats: [] };
    sections.push(unbound);
  }
  if (current.input.view === "characters") {
    for (const section of sections) {
      const scope = conversationSidebarScope(section);
      section.conversationGroups = scope.groups;
      section.ungroupedChats = scope.ungrouped;
    }
    return { view: "characters", sections };
  }
  const scopes = [];
  for (const section of sections) scopes.push(conversationSidebarScope(section));
  return { view: "groups", scopes };
}
/** Registers the one actual sidebar catalog handler without reading native chats or opening the file repository. */
export function registerSidebarChannel(): void {
  ToolPkg.ipc.on<CurrentSidebar, SidebarCatalog>("character-sidebar.catalog", readSidebarCatalog);
}

/** Sidebar content is native Compose DSL; only the separate management editor uses a WebView. */
export { renderSidebarScreen } from "./ui-sidebar-native";
