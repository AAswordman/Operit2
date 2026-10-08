import type { SnapshotRecord, ConversationGroupRecord } from "./api";
import { record } from "./domain";
import { decodeSelection, encodeSelection, parseDomainMessage } from "./ui-contributions";
import { decodeChatMarker } from "./chat-markers";
import { requireChatSelection } from "./chat-bindings";
import { getService } from "./service-runtime";
import type { ComposeDslContext, ComposeNode, ComposeThemeSnapshot } from "../../../../types/compose-dsl";

/** Selects one plugin-owned embedded sidebar view without creating a modal presentation. */
export interface SidebarInput { view: "characters" | "groups" }
/** Contains only the neutral chat metadata supplied by the existing native history source. */
export interface SidebarChatSummary {
  id: string; title: string; updatedAt: string; displayOrder: number;
  workspaceId: string | null; workspaceName: string | null; locked: boolean; pinned: boolean;
}
/** Describes the exact live host context of an embedded chat sidebar. */
export interface ChatSidebarContext { chats: SidebarChatSummary[]; currentChatId: string | null; activeStreamingChatIds: string[] }
/** Carries opaque plugin route input separately from generic host history state. */
export interface CurrentSidebar { input: SidebarInput; chatSidebar: ChatSidebarContext }
/** Requests actual host navigation without exposing plugin selections or group membership. */
export interface ChatActivateAction { type: "toolpkg.chat.activate"; chatId: string }
/** Describes one persisted character or role-group category owned entirely by the plugin. */
export interface CharacterSidebarSection {
  id: string; title: string; avatarUri: string | null; kind: "card" | "group" | "unbound";
  selection: string | null; chats: SidebarChatSummary[];
}
/** Supplies explicit plugin selection state to a pure view without defining a storage or namespace protocol. */
export interface SidebarChatSelection { chatId: string; selection: string | null }
/** Supplies real directory records and one explicit selection state for each host chat to the category projection. */
export interface CharacterSidebarData { directory: SnapshotRecord; selections: SidebarChatSelection[] }

/** Projects an actual file-owned conversation group without exposing it to the generic native sidebar. */
export interface ConversationSidebarGroup { id: string; name: string; pinned: boolean; displayOrder: number; chats: SidebarChatSummary[] }
/** Describes one explicit plugin-owned group scope independently of native role or group fields. */
export interface ConversationSidebarScope { id: string; title: string; ownerSelection: string | null; groups: ConversationSidebarGroup[]; ungrouped: SidebarChatSummary[] }
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
/** Validates a neutral host history summary and explicitly rejects old character or conversation-group fields. */
export function parseSidebarChat(value: unknown): SidebarChatSummary {
  const chat = exactFields(value, ["id", "title", "updatedAt", "displayOrder", "workspaceId", "workspaceName", "locked", "pinned"], "sidebar chat");
  const id = sidebarIdentity(chat.id, "sidebar chat.id");
  if (typeof chat.title !== "string" || typeof chat.updatedAt !== "string") throw new Error("sidebar chat title and updatedAt must be strings");
  if (typeof chat.displayOrder !== "number" || !Number.isSafeInteger(chat.displayOrder)) throw new Error("sidebar chat.displayOrder must be a safe integer");
  if (chat.workspaceId !== null && typeof chat.workspaceId !== "string") throw new Error("sidebar chat.workspaceId must be a string or null");
  if (chat.workspaceName !== null && typeof chat.workspaceName !== "string") throw new Error("sidebar chat.workspaceName must be a string or null");
  if (typeof chat.locked !== "boolean" || typeof chat.pinned !== "boolean") throw new Error("sidebar chat locked and pinned must be booleans");
  return { id, title: chat.title, updatedAt: chat.updatedAt, displayOrder: chat.displayOrder, workspaceId: chat.workspaceId, workspaceName: chat.workspaceName, locked: chat.locked, pinned: chat.pinned };
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
export function characterSidebarSections(directory: SnapshotRecord, selections: readonly SidebarChatSelection[], chats: readonly SidebarChatSummary[]): CharacterSidebarSection[] {
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
export function conversationSidebarScope(section: CharacterSidebarSection, records: readonly ConversationGroupRecord[]): ConversationSidebarScope {
  const chats = new Map(section.chats.map(
    /** Associates the full actual host summary with its exact identity. */
    chat => [chat.id, chat] as const,
  ));
  const membership = new Set<string>(), ids = new Set<string>();
  const groups: ConversationSidebarScope["groups"] = [];
  for (const record of records) {
    if (record.ownerSelection !== section.selection) throw new Error("Conversation group belongs to another explicit scope: " + record.id);
    if (ids.has(record.id)) throw new Error("Duplicate persisted conversation group: " + record.id);
    ids.add(record.id);
    const members = [];
    for (const chatId of record.chatIds) {
      if (membership.has(chatId)) throw new Error("Chat belongs to multiple groups in this scope: " + chatId);
      const chat = chats.get(chatId);
      if (chat === undefined) throw new Error("Persisted group member has no chat in its declared scope: " + chatId);
      membership.add(chatId); members.push(chat);
    }
    members.sort(compareSidebarChats);
    groups.push({ id: record.id, name: record.name, pinned: record.pinned, displayOrder: record.displayOrder, chats: members });
  }
  groups.sort(
    /** Preserves pin priority and genuine persisted group order without updating metadata while reading. */
    (left, right) => Number(right.pinned) - Number(left.pinned) || left.displayOrder - right.displayOrder || left.id.localeCompare(right.id),
  );
  return { id: section.id, title: section.title, ownerSelection: section.selection, groups, ungrouped: section.chats.filter(
    /** Classifies explicitly absent membership only after inspecting the complete real scope catalog. */
    chat => !membership.has(chat.id),
  ) };
}

/** Reads the actual sidebar catalog from namespace extensions and the same file-backed service used by every other plugin surface. */
export async function readSidebarCatalog(value: CurrentSidebar): Promise<SidebarCatalog> {
  const current = parseCurrentSidebar(value), service = await getService(), directory = await service.snapshot();
  const selections: SidebarChatSelection[] = [];
  for (const chat of current.chatSidebar.chats) {
    const extension = await Tools.Chat.readExtension({ kind: "chat", chatId: chat.id });
    if (extension === null) { selections.push({ chatId: chat.id, selection: null }); continue; }
    const marker = decodeChatMarker(extension);
    requireChatSelection(marker.selection, directory.cards, directory.groups);
    selections.push({ chatId: chat.id, selection: marker.selection });
  }
  const sections = characterSidebarSections(directory, selections, current.chatSidebar.chats);
  if (current.input.view === "characters") return { view: "characters", sections };
  if (!sections.some(
    /** Ensures an explicit unbound grouping scope remains available for real empty group creation. */
    section => section.selection === null,
  )) {
    const unbound: CharacterSidebarSection = { id: "unbound", title: "未绑定角色", avatarUri: null, kind: "unbound", selection: null, chats: [] };
    sections.push(unbound);
  }
  const scopes = [];
  for (const section of sections) scopes.push(conversationSidebarScope(section, await service.dispatchDomain("conversation-group.list", { ownerSelection: section.selection })));
  return { view: "groups", scopes };
}
/** Registers the one actual sidebar catalog handler without reading native chats or opening the file repository. */
export function registerSidebarChannel(): void {
  ToolPkg.ipc.on<CurrentSidebar, SidebarCatalog>("character-sidebar.catalog", readSidebarCatalog);
}

/** Decodes only the existing nested WebView argument-array ABI. */
function argumentsFor(value: readonly unknown[], expected: number): unknown[] {
  if (value.length !== 1 || !Array.isArray(value[0]) || value[0].length !== expected) throw new Error("CharacterSidebarHost expects " + expected + " arguments");
  return value[0];
}
/** Renders the independently embedded sidebar using exact opaque input and live generic host context, never a modal session. */
export function renderSidebarScreen(ctx: ComposeDslContext): ComposeNode {
  const controller = ctx.createWebViewController("character-sidebar-web");
  const [input] = ctx.useState<SidebarInput | null>("input", null), [chatSidebar] = ctx.useState<ChatSidebarContext | null>("chatSidebar", null);
  const [path, setPath] = ctx.useState("character-sidebar-html", ""), [error, setError] = ctx.useState("character-sidebar-error", "");
  const current = ctx.useRef<CurrentSidebar>("character-sidebar-current", parseCurrentSidebar({ input, chatSidebar }));
  current.current = parseCurrentSidebar({ input, chatSidebar });
  const ready = ctx.useRef("character-sidebar-ready", false), published = ctx.useRef("character-sidebar-published", "");
  const unsubscribe = ctx.useRef<(() => void) | null>("character-sidebar-theme-subscription", null);
  const origin = "https://character-sidebar.operit.local/";
  /** Applies the current real Material palette without manufacturing local theme values. */
  async function applyTheme(theme: ComposeThemeSnapshot): Promise<void> {
    if (ready.current) await controller.evaluateJavascript("window.applyCharacterSidebarTheme(" + JSON.stringify(theme) + ");");
  }
  /** Publishes validated native context updates to the same real sidebar document. */
  async function publishContext(): Promise<void> {
    const serialized = JSON.stringify(current.current);
    if (!ready.current || published.current === serialized) return;
    try {
      await controller.evaluateJavascript("window.updateCharacterSidebar(" + serialized + ");");
      published.current = serialized;
    } catch (failure) { setError(String(failure)); ctx.reportError(failure); }
  }
  if (ready.current && published.current !== JSON.stringify(current.current)) void publishContext();
  /** Installs only the real package IPC and existing generic host actions before loading the sidebar asset. */
  async function initialize(): Promise<void> {
    try {
      controller.addJavascriptInterface("CharacterSidebarHost", {
        /** Returns the actual host palette for the independent embedded document. */
        currentTheme: () => ctx.Theme.getCurrent(),
        /** Reads exactly the validated route input and current native history context. */
        currentSidebar: (...args: unknown[]) => { argumentsFor(args, 0); ready.current = true; published.current = JSON.stringify(current.current); return current.current; },
        /** Reads the actual plugin-projected catalog through the one main runtime. */
        catalog: (...args: unknown[]) => { argumentsFor(args, 0); return ToolPkg.ipc.call<CurrentSidebar, SidebarCatalog>("character-sidebar.catalog", current.current, { targetRuntime: "main" }); },
        /** Delegates finite domain mutations to the same authoritative service, without a Compose-runtime repository. */
        domain: (...args: unknown[]) => { const [value] = argumentsFor(args, 1), message = parseDomainMessage(value); return ToolPkg.ipc.call("character-memory.domain", message, { targetRuntime: "main" }); },
        /** Returns the exact existing generic activation action after verifying the target in the supplied host context. */
        activateChat: (...args: unknown[]) => {
          const [value] = argumentsFor(args, 1), chatId = sidebarIdentity(value, "activate chatId");
          if (!current.current.chatSidebar.chats.some(
            /** Requires an exact supplied native identity, not another role's guessed conversation. */
            chat => chat.id === chatId,
          )) throw new Error("Chat activation target is absent from the actual sidebar context: " + chatId);
          return { type: "toolpkg.chat.activate", chatId };
        },
        /** Calls the actual generic native deletion chain; backend callbacks own extension and membership cleanup. */
        deleteChat: (...args: unknown[]) => { const [value] = argumentsFor(args, 1); return Tools.Chat.deleteChat(sidebarIdentity(value, "delete chatId")); },
      });
      unsubscribe.current = ctx.Theme.subscribe(applyTheme);
      setPath(await ToolPkg.readResource("character_sidebar_html", "character-sidebar.html"));
    } catch (failure) { setError(String(failure)); }
  }
  /** Releases only the actual theme subscription owned by this embedded surface. */
  function dispose(): void { ready.current = false; if (unsubscribe.current !== null) { unsubscribe.current(); unsubscribe.current = null; } }
  return ctx.UI.Box({ fillMaxSize: true, onLoad: initialize }, error !== "" ? ctx.UI.Text({ text: error }) : path === "" ? ctx.UI.Text({ text: "正在加载插件侧边栏…" }) : ctx.UI.WebView({
    key: "character-sidebar-web", controller, fillMaxSize: true, url: origin, javaScriptEnabled: true, domStorageEnabled: true, supportZoom: false, useWideViewPort: true,
    /** Restricts top-level navigation to this exact offline sidebar document. */
    onShouldOverrideUrlLoading: request => request.url === origin ? { action: "allow" } : { action: "cancel" },
    /** Serves the real registered plugin resource through the existing cross-platform WebView host. */
    onInterceptRequest: request => request.url === origin ? { action: "respond", response: { mimeType: "text/html", encoding: "utf-8", statusCode: 200, reasonPhrase: "OK", filePath: path } } : { action: "block" },
    /** Disposes references when the embedded sidebar instance is actually released. */
    onLifecycleEvent: event => { if (event.type === "Disposed") dispose(); },
  }));
}
