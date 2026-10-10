import type { JsonValue, ActivePrompt, Snapshot } from "./model";
import type { DomainInput, DomainOperation, DomainOutput, ChatBindingRecord } from "./api";
import { parseDomainPayload, record } from "./domain";
import { parseChatSelection, selectionForActive } from "./chat-bindings";
import { type EntityKind, type PresentationCancel, type PresentationComplete, type ScreenSession, type ScreenPresentation, type PresentedScreen, type SelectScreen, type ScreenResult, createScreenSession, parseScreenResult } from "./presentation";
import { getService } from "./service-runtime";
import { decodeChatMarker } from "./chat-markers";
import { chatUiRoutes } from "./definition";

/** Separates the actual native selector from the large Web editor route without exposing either mode to the host. */
export interface ChatUiRoutes { readonly editor: string; readonly selection: string; readonly execution: string }
/** Carries an explicitly global or existing-chat selection projection, not a persisted binding record. */
export interface ContextSelection { chatId: string | null; selection: string | null }

/** Uses the existing owner-qualified route identifier, never a package-specific host entry point. */
export interface UiAction { routeId: string; input: JsonValue }
/** Describes the current chat's display identity and its owner-provided preview action. */
export interface UiIdentity { title: string; avatarUri: string | null; action: UiAction }
/** Returns only generic chat context presentation fields at the host boundary. */
export interface ContextActions { identity: UiIdentity | null; backgroundUri: string | null }
/** Uses the real generic Flutter chat history summary contract without legacy domain fields. */
export interface ChatSummary {
  id: string; title: string; updatedAt: string; displayOrder: number;
  workspaceId: string | null; workspaceName: string | null; locked: boolean; pinned: boolean;
}
/** Projects chat identities into an owner-defined section with opaque selection and preview input. */
export interface ChatSection {
  id: string; title: string; avatarUri: string | null; chatIds: string[];
  selection: JsonValue; preview: UiAction;
}
/** Returns every persisted owner section, including selectable sections with no chats yet. */
export interface ListSections { sections: ChatSection[] }
/** Carries the locked generic text-attachment completion value, without domain owner information. */
export interface TextAttachment { type: "text"; name: string; content: string; mediaType: "text/plain" }

/** Carries the existing typed main-runtime domain IPC envelope used by UI and memory tools. */
export type DomainMessage<K extends DomainOperation = DomainOperation> = { [P in K]: { operation: P; input: DomainInput<P> } }[K];

/** Validates operation membership and all declared input fields using the service's actual domain parser. */
export function parseDomainMessage(value: unknown): DomainMessage {
  const envelope = record(value, "character-memory.domain"), keys = Object.keys(envelope);
  if (keys.length !== 2 || !keys.every(
    /** Rejects protocol spellings other than the established operation/input fields. */
    key => key === "operation" || key === "input",
  )) throw new Error("character-memory.domain requires exactly operation and input");
  if (typeof envelope.operation !== "string" || envelope.operation.trim() === "") throw new Error("character-memory.domain.operation must be nonblank");
  const operation = envelope.operation as DomainOperation;
  return { operation, input: parseDomainPayload(operation, envelope.input) } as DomainMessage;
}

/** Calls the same main-runtime owner rather than constructing a Compose-runtime repository. */
export function callMainDomain<K extends DomainOperation>(operation: K, input: DomainInput<K>): Promise<DomainOutput<K>> {
  const checked = parseDomainPayload(operation, input);
  return ToolPkg.ipc.call<DomainMessage<K>, DomainOutput<K>>("character-memory.domain", { operation, input: checked }, { targetRuntime: "main" });
}

/** Requires a nonblank generic identity without normalizing or inferring another chat. */
export function chatIdentity(value: unknown, label: string): string {
  if (typeof value !== "string" || value.trim() === "") throw new Error(label + " must be a nonblank string");
  return value;
}

/** Validates exact request fields at the contribution boundary, rejecting undocumented protocol fields. */
function fields(value: unknown, expected: readonly string[], label: string): Record<string, unknown> {
  const object = record(value, label), keys = Object.keys(object);
  if (keys.length !== expected.length || keys.some(
    /** Requires each key to be one of the exact documented field names. */
    key => expected.indexOf(key) < 0,
  )) throw new Error(label + " has invalid fields");
  return object;
}

/** Requires an explicit string or null without treating empty metadata as another source. */
function nullableString(value: unknown, label: string): string | null {
  if (value === null || typeof value === "string") return value;
  throw new Error(label + " must be a string or null");
}

/** Decodes the explicitly global or current-chat context request sent by Flutter. */
export function parseChatRequest(value: unknown): { chatId: string | null } {
  const input = fields(value, ["chatId"], "chat UI request");
  return { chatId: input.chatId === null ? null : chatIdentity(input.chatId, "chatId") };
}

/** Validates the actual generic history metadata emitted by chatUiHistorySummary. */
export function parseSectionsRequest(value: unknown): { chats: ChatSummary[] } {
  const input = fields(value, ["chats"], "chat.list.sections");
  if (!Array.isArray(input.chats)) throw new Error("chat.list.sections.chats must be an array");
  const chats: ChatSummary[] = [], ids = new Set<string>();
  for (const raw of input.chats) {
    const chat = fields(raw, ["id", "title", "updatedAt", "displayOrder", "workspaceId", "workspaceName", "locked", "pinned"], "chat summary");
    const id = chatIdentity(chat.id, "chat.id");
    if (ids.has(id)) throw new Error("Duplicate chat summary: " + id);
    ids.add(id);
    if (typeof chat.title !== "string" || typeof chat.updatedAt !== "string") throw new Error("Chat summary title and updatedAt must be strings");
    for (const key of ["workspaceId", "workspaceName"] as const) if (chat[key] !== null && typeof chat[key] !== "string") throw new Error("Chat summary " + key + " must be a string or null");
    if (typeof chat.displayOrder !== "number" || !Number.isSafeInteger(chat.displayOrder)) throw new Error("Chat summary displayOrder must be an integer");
    if (typeof chat.locked !== "boolean" || typeof chat.pinned !== "boolean") throw new Error("Chat summary locked and pinned must be booleans");
    const workspaceId = nullableString(chat.workspaceId, "chat.workspaceId");
    const workspaceName = nullableString(chat.workspaceName, "chat.workspaceName");
    chats.push({ id, title: chat.title, updatedAt: chat.updatedAt, displayOrder: chat.displayOrder, workspaceId, workspaceName, locked: chat.locked, pinned: chat.pinned });
  }
  return { chats };
}

/** Restricts presentation-facing APIs to the authenticated host, not arbitrary invoking packages. */
export function requireUiCaller(caller: string): void {
  if (caller !== "host") throw new Error("Chat UI contributions require an authenticated host caller");
}

/** Resolves the plugin's deliberately opaque selection token using its exact declared grammar. */
export function decodeSelection(selection: string, directory: Pick<Snapshot, "cards" | "groups">): { entity: EntityKind; id: string; prompt: ActivePrompt; title: string; avatarUri: string | null } {
  const parsed = parseChatSelection(selection), id = parsed.id;
  switch (parsed.kind) {
    case "card": {
      const cards = directory.cards.filter(
        /** Requires one exact persisted identity, not a replacement active actor. */
        card => card.id === id,
      );
      if (cards.length !== 1) throw new Error("Selection does not identify exactly one character: " + id);
      const card = cards[0];
      return { entity: "card", id, prompt: { CharacterCard: { id } }, title: card.name, avatarUri: card.avatarUri };
    }
    case "group": {
      const groups = directory.groups.filter(
        /** Requires one exact persisted group identity. */
        group => group.id === id,
      );
      if (groups.length !== 1) throw new Error("Selection does not identify exactly one group: " + id);
      return { entity: "group", id, prompt: { CharacterGroup: { id } }, title: groups[0].name, avatarUri: null };
    }
    default: throw new Error("Invalid plugin selection entity");
  }
}

/** Encodes a UI-confirmed domain identity as the same persisted opaque token used by the service. */
export function encodeSelection(selection: ActivePrompt): string {
  return selectionForActive(selection);
}

/** Publishes chat identity and background independently of the plugin-rendered input menu. */
export function contextActions(routes: ChatUiRoutes, target: ContextSelection, directory: Snapshot): ContextActions {
  const selected = target.selection === null ? null : decodeSelection(target.selection, directory);
  return {
    identity: selected === null ? null : { title: selected.title, avatarUri: selected.avatarUri, action: previewAction(routes.editor, selected.entity, selected.id) },
    backgroundUri: null,
  };
}

/** Builds an opaque entity preview action without exposing its domain input to Flutter consumers. */
function previewAction(routeId: string, entity: "card" | "group", id: string): UiAction {
  return { routeId, input: { mode: "preview", entity, id } };
}

/** Projects real bindings into every persisted character and group section, preserving caller chat order. */
export function listSections(routeId: string, directory: Snapshot, bindings: readonly ChatBindingRecord[], chatIds: readonly string[]): ListSections {
  const byChat = new Map<string, string>();
  for (const binding of bindings) {
    if (byChat.has(binding.chatId)) throw new Error("Duplicate chat binding: " + binding.chatId);
    decodeSelection(binding.selection, directory); byChat.set(binding.chatId, binding.selection);
  }
  const sections: ListSections["sections"] = [];
  for (const card of directory.cards) {
    const selection = "card:" + card.id;
    sections.push({ id: selection, title: card.name, avatarUri: card.avatarUri, selection, preview: previewAction(routeId, "card", card.id), chatIds: boundChats(selection) });
  }
  for (const group of directory.groups) {
    const selection = "group:" + group.id;
    sections.push({ id: selection, title: group.name, avatarUri: null, selection, preview: previewAction(routeId, "group", group.id), chatIds: boundChats(selection) });
  }
  /** Selects only the caller's actual chats that have this exact stored opaque selection. */
  function boundChats(selection: string): string[] {
    const result: string[] = [];
    for (const chatId of chatIds) if (byChat.get(chatId) === selection) result.push(chatId);
    return result;
  }
  return { sections };
}

/** Reads the real chat binding and persisted records through the one main-runtime service. */
export async function chatContextActionsApi(event: ToolPkg.PublicApiEvent<{ chatId: string | null }>): Promise<ContextActions> {
  requireUiCaller(event.callerPackage);
  const input = parseChatRequest(event.payload), service = await getService();
  const directory = await service.snapshot();
  if (input.chatId === null) {
    const active = await service.dispatchDomain("activePrompt.get", {});
    return contextActions(chatUiRoutes, { chatId: null, selection: encodeSelection(active) }, directory);
  }
  const extension = await Tools.Chat.readExtension({ kind: "chat", chatId: input.chatId });
  const selection = extension === null ? null : decodeChatMarker(extension).selection;
  return contextActions(chatUiRoutes, { chatId: input.chatId, selection }, directory);
}

/** Projects actual chat bindings into owner-defined sections instead of reading old Flutter character fields. */
export async function chatListSectionsApi(event: ToolPkg.PublicApiEvent<{ chats: ChatSummary[] }>): Promise<ListSections> {
  requireUiCaller(event.callerPackage);
  const input = parseSectionsRequest(event.payload), service = await getService(), directory = await service.snapshot();
  const bindings = [];
  for (const chat of input.chats) {
    const extension = await Tools.Chat.readExtension({ kind: "chat", chatId: chat.id });
    if (extension !== null) bindings.push({ chatId: chat.id, selection: decodeChatMarker(extension).selection });
  }
  return listSections(chatUiRoutes.editor, directory, bindings, input.chats.map(
    /** Preserves the host's actual chat ordering in each section. */
    chat => chat.id,
  ));
}

/** Registers the context and sidebar presentation contracts without opening storage or reading directories. */
export function registerUiContributionApis(): void {
  ToolPkg.registerApi({ name: "chat.context.actions", function: chatContextActionsApi });
  ToolPkg.registerApi({ name: "chat.list.sections", function: chatListSectionsApi });
}

/** Extends the existing plugin screen session with durable chat binding and generic completion values. */
/** Adds the explicit selector target to the typed package presentation input. */
export type UiPresentedScreen = Exclude<PresentedScreen, SelectScreen> | (SelectScreen & { readonly chatId: string | null });
/** Preserves the package input throughout the Compose presentation chain. */
export type UiPresentation = ScreenPresentation<UiPresentedScreen>;

export interface UiScreenSession extends Omit<ScreenSession, "completeScreen"> {
  /** Commits a real binding before completing, or converts a completed attachment to generic text. */
  completeScreen(value: ScreenResult): Promise<PresentationComplete>;
}

/** Adapts opaque caller input locally while keeping currentScreen's existing Web contract unchanged. */
export function createUiScreenSession(presentation: UiPresentation | null): UiScreenSession {
  let selectorTarget: { chatId: string | null } | null = null;
  let checked: ScreenPresentation | null = presentation;
  if (presentation !== null && presentation.input.mode === "select") {
    const { chatId, ...screenInput } = presentation.input;
    if (chatId !== null) chatIdentity(chatId, "presentation.input.chatId");
    selectorTarget = { chatId };
    checked = { requestId: presentation.requestId, input: screenInput };
  }
  const session = createScreenSession(checked);
  let completing = false, finished = false;
  return {
    /** Reads only the package screen parameters needed by the reused typed Web view. */
    currentScreen: () => session.currentScreen(),
    /** Persists a selector choice through main IPC before acknowledging its caller-owned request. */
    async completeScreen(value: ScreenResult): Promise<PresentationComplete> {
      if (finished) throw new Error("This presentation has already finished");
      if (completing) throw new Error("A presentation completion is already running");
      const current = session.currentScreen();
      if (current.input.mode === "manage") throw new Error("Management has no presentation completion channel");
      const result = parseScreenResult(value, current.input);
      completing = true;
      try {
        let completion: PresentationComplete;
        switch (result.mode) {
          case "select": {
            if (selectorTarget === null) throw new Error("Selector has no explicit global or chat target");
            const selection = encodeSelection(result.selection);
            if (selectorTarget.chatId === null) {
              const identity = parseChatSelection(selection);
              const active = identity.kind === "card"
                ? await callMainDomain("activePrompt.setCard", { id: identity.id })
                : await callMainDomain("activePrompt.setGroup", { id: identity.id });
              if (!active.active || active.id !== identity.id || active.type !== (identity.kind === "card" ? "character_card" : "character_group")) throw new Error("Main runtime did not confirm the selected global active prompt");
            } else {
              const chatId = selectorTarget.chatId;
              const binding = await callMainDomain("chat.configuration.binding.write", { chatId, selection });
              if (binding.chatId !== chatId || binding.selection !== selection) throw new Error("Main runtime did not confirm the selected chat binding");
            }
            const complete = session.completeScreen(result);
            completion = { ...complete, value: { selection, contextKey: selection } };
            break;
          }
          case "memory-attachment": {
            const complete = session.completeScreen(result);
            completion = { ...complete, value: { type: "text", name: "记忆附件", content: result.content, mediaType: "text/plain" } };
            break;
          }
          case "preview":
          case "edit": completion = session.completeScreen(result); break;
        }
        finished = true;
        return completion;
      } finally { completing = false; }
    },
    /** Cancels the same caller request without invoking any binding write or returning domain data. */
    cancelScreen(): PresentationCancel {
      if (finished) throw new Error("This presentation has already finished");
      if (completing) throw new Error("Cannot cancel during a presentation commit");
      const cancellation = session.cancelScreen();
      finished = true;
      return cancellation;
    },
  };
}
