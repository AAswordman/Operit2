import type { ToolPkg as ToolPkgTypes } from "../../../../types/toolpkg";
import { parseChatSelection, requireChatSelection, selectionForActive } from "./chat-bindings";
import { markerObject, decodeChatMarker, encodeChatMarker } from "./chat-markers";
import { record } from "./domain";
import { dispatchDomain } from "./service-runtime";

/** Describes the sole plugin-owned opaque creation input and initial namespace marker. */
export interface ChatSelectionMarker { version: 1; selection: string }
/** Declares the generic new-chat payload supplied before native persistence. */
export interface ChatInitializationRequest extends Omit<ToolPkgTypes.ChatLifecycleEventPayload, "input"> {
  input: ChatSelectionMarker | null;
}
/** Returns only this plugin's own namespace extension, never a caller-selected owner id. */
export interface ChatInitializationResult { extension: ToolPkgTypes.JsonObject | null }
/** Keeps initialization tied to the authoritative repository helper without defining backend storage fields. */
export interface ChatInitializationReader {
  /** Reads the actual persisted active selection for an explicitly source-free first creation. */
  readActiveSelection(): Promise<string>;
  /** Requires the chosen marker to reference an actual valid character or participant group. */
  requireSelection(selection: string): Promise<void>;
}

export const CHARACTER_CARDS_NAMESPACE = "com.operit.character_cards";

/** Requires the exact declared lifecycle fields, including explicitly nullable values. */
function fields(value: unknown, keys: readonly string[], label: string): Record<string, unknown> {
  const input = record(value, label), actual = Object.keys(input);
  if (actual.length !== keys.length || actual.some(
    /** Rejects undocumented fields rather than guessing their meaning or owner. */
    key => keys.indexOf(key) === -1,
  )) throw new Error(label + " has invalid fields");
  return input;
}
/** Requires an unchanged actual chat identity without assigning another source. */
function identity(value: unknown, label: string): string {
  if (typeof value !== "string" || value.trim() === "") throw new Error(label + " must be a nonblank string");
  return value;
}
/** Validates only the plugin-owned versioned selection marker, excluding owner and legacy role fields. */
export function parseChatSelectionMarker(value: unknown, label: string): ChatSelectionMarker {
  const marker = fields(value, ["version", "selection"], label);
  if (marker.version !== 1) throw new Error(label + ".version must be 1");
  const selection = identity(marker.selection, label + ".selection");
  parseChatSelection(selection);
  return { version: 1, selection };
}
/** Builds explicitly scoped input so other registered packages never interpret this plugin selection marker. */
export function createChatInput(selection: string): ToolPkgTypes.JsonObject {
  const marker = parseChatSelectionMarker({ version: 1, selection }, "chat creation selection");
  return { [CHARACTER_CARDS_NAMESPACE]: { version: 1, selection: marker.selection } };
}
/** Reads only this package's explicit key while leaving sibling namespaces uninterpreted. */
function scopedInput(value: unknown): ChatSelectionMarker | null {
  if (value === null) return null;
  const input = markerObject(value, "chat lifecycle.input");
  if (!Object.prototype.hasOwnProperty.call(input, CHARACTER_CARDS_NAMESPACE)) return null;
  return parseChatSelectionMarker(input[CHARACTER_CARDS_NAMESPACE], "chat lifecycle.input." + CHARACTER_CARDS_NAMESPACE);
}
/** Validates the typed SDK before-create payload and decodes only plugin-owned opaque JSON fields. */
export function parseChatInitialization(value: ToolPkgTypes.ChatLifecycleEventPayload): ChatInitializationRequest {
  const keys = Object.keys(value);
  if (keys.length !== 7 || keys.some(key => ["eventName", "creationKind", "chat", "sourceChatId", "sourceMessageTimestamp", "input", "sourceExtension"].indexOf(key) < 0)) throw new Error("chat lifecycle has invalid fields");
  const chatKeys = Object.keys(value.chat);
  if (chatKeys.length !== 4 || chatKeys.some(key => ["id", "title", "workspaceId", "parentChatId"].indexOf(key) < 0)) throw new Error("chat lifecycle.chat has invalid fields");
  if (value.eventName !== "before_create") throw new Error("chat lifecycle.eventName must be before_create");
  if (value.creationKind !== "new" && value.creationKind !== "branch") throw new Error("chat lifecycle.creationKind must be new or branch");
  const workspaceId = value.chat.workspaceId === null ? null : identity(value.chat.workspaceId, "chat lifecycle.chat.workspaceId");
  const parentChatId = value.chat.parentChatId === null ? null : identity(value.chat.parentChatId, "chat lifecycle.chat.parentChatId");
  const sourceChatId = value.sourceChatId === null ? null : identity(value.sourceChatId, "chat lifecycle.sourceChatId");
  const sourceMessageTimestamp = value.sourceMessageTimestamp;
  if (sourceMessageTimestamp !== null && (!Number.isSafeInteger(sourceMessageTimestamp) || sourceMessageTimestamp < 0)) throw new Error("chat lifecycle.sourceMessageTimestamp must be a nonnegative safe integer or null");
  if (sourceChatId === null && (value.sourceExtension !== null || sourceMessageTimestamp !== null)) throw new Error("Source namespace metadata and message time require an explicit sourceChatId");
  return {
    eventName: "before_create", creationKind: value.creationKind,
    chat: { id: identity(value.chat.id, "chat lifecycle.chat.id"), title: value.chat.title, workspaceId, parentChatId },
    sourceChatId, sourceMessageTimestamp,
    input: scopedInput(value.input),
    sourceExtension: value.sourceExtension === null ? null : markerObject(value.sourceExtension, "chat lifecycle.sourceExtension"),
  };
}

/** Initializes a requested chat solely in the plugin's isolated namespace before native creation commits. */
export async function initializeChatExtension(value: ToolPkgTypes.ChatLifecycleEventPayload, reader: ChatInitializationReader): Promise<ChatInitializationResult> {
  const event = parseChatInitialization(value);
  if (event.input !== null) {
    await reader.requireSelection(event.input.selection);
    return { extension: { version: 1, selection: event.input.selection } };
  }
  if (event.sourceChatId !== null) {
    if (event.sourceExtension === null) throw new Error("Explicit sourceChatId has no character namespace extension");
    const marker = decodeChatMarker(event.sourceExtension);
    await reader.requireSelection(marker.selection);
    return { extension: encodeChatMarker(marker.selection, event.sourceExtension) };
  }
  const selection = await reader.readActiveSelection();
  await reader.requireSelection(selection);
  return { extension: { version: 1, selection } };
}

/** Reads the actual persisted active prompt through the one main-runtime service without introducing chat-binding storage. */
async function readActiveSelection(): Promise<string> {
  return selectionForActive(await dispatchDomain("activePrompt.get", {}));
}
/** Delegates existence and participant validation to the authoritative backend helper using genuine persisted records. */
async function requireSelection(selection: string): Promise<void> {
  const [cards, groups] = await Promise.all([dispatchDomain("character.list", {}), dispatchDomain("group.list", {})]);
  requireChatSelection(selection, cards, groups);
}
/** Handles the SDK's existing hook envelope and returns only this package's initialized namespace before native commit. */
export async function beforeChatCreate(event: ToolPkgTypes.ChatLifecycleHookEvent): Promise<ChatInitializationResult> {
  if (event.eventName !== "before_create") throw new Error("Character chat lifecycle requires before_create");
  return initializeChatExtension(event.eventPayload, { readActiveSelection, requireSelection });
}

/** Registers this package's sole pre-create hook by genuine exported function reference for durable SDK capture. */
export function registerChatInitialization(): void {
  ToolPkg.registerChatLifecycleHook({ id: "chat-initialization", function: beforeChatCreate });
}
