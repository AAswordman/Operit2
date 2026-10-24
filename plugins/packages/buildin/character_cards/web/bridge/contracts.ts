import type {
  Memory, MemoryGraphResult, MemorySettings, MemorySearchConfig, MemoryAutoSaveStatus, MemoryRebuildProgress, MemoryChat, Snapshot,
  Request, RequestResults, StoreValues, Theme, BindableChat,
} from "../../src/model";
import type { EntityKind, PresentationCancel, PresentationComplete, ScreenResult } from "./screens";
import type { ScreenBootstrap } from "../../src/presentation";
import type { GraphCamera, NodeSize, Point } from "../features/graph/layout";
import type { ComposeFileSaveResult } from "../../../../../types/compose-dsl";

/** Describes actual service results used by this finite editor bridge. */
export type WebResponses = Pick<RequestResults, "listBindableChats" | "writeChatBinding" | "snapshot" | "writeGlobalMemoryPreference" | "searchMemoryDebug" | "readMemoryLink" | "exportMemory" | "importMemory" | "activate" | "saveStore" | "deleteStore" | "readUser" | "writeUser" | "graph" | "searchMemory" | "listMemories" | "listMemoryFolders" | "saveMemory" | "deleteMemory" | "createLink" | "deleteLink" | "updateLink" | "readMemorySettings" | "readMemorySearchConfig" | "writeMemorySettings" | "writeMemorySearchConfig" | "writeMemoryOwnerSettings" | "readMemoryAutoSaveStatus" | "listMemoryChats" | "pruneMissingChatBindings" | "startMemoryRebuild" | "readMemoryRebuildProgress" | "cancelMemoryRebuild" | "rebuildMemoryEmbeddings" | "updateChatMemory" | "autoCategorizeMemory" | "retryFailedCandidates" | "clearFailedCandidates" | "importCharacter" | "importGroup">;
export type WebAction = keyof WebResponses;
export type WebRequest<A extends WebAction> = Extract<Request, { action: A }>;

/** Declares the existing WebView interface registered by the package's Compose host. */
export interface CharacterMemoryBridge {
  /** Reads the actual palette, route input and fresh aggregate in one startup response. */
  loadScreen(): Promise<ScreenBootstrap>;
  /** Returns an explicit completion event through the host's existing Compose action channel. */
  completeScreen(result: ScreenResult): Promise<PresentationComplete>;
  /** Returns an explicit cancellation event without modifying plugin records. */
  cancelScreen(): Promise<PresentationCancel>;
  /** Invokes the package provider using a typed domain request and result. */
  request<A extends WebAction>(operation: WebRequest<A>): Promise<WebResponses[A]>;
  /** Returns a readable avatar image, including the real built-in default. */
  avatarImage(uri: string | null): Promise<string>;
  /** Opens the sole plugin-authored native entity surface. */
  openEntity(mode: "create" | "edit" | "preview", entity: EntityKind, id: string): Promise<void>;
  /** Saves JSON using the host file selector and returns its explicit cancellation result. */
  exportFile(name: string, content: string): Promise<ComposeFileSaveResult>;
}
declare global {
  interface Window {
    CharacterMemoryHost?: CharacterMemoryBridge;
    /** Applies a later host Material theme update without losing editor drafts. */
    applyCharacterMemoryTheme(theme: Theme): void;
  }
}

export interface DialogState { dispose?: () => void; error: string; scroll: number; busy: boolean }
export interface StoreDialog extends DialogState { type: "store"; store: StoreValues; create: boolean }
export interface OwnerSettingsDialog extends DialogState {
  type: "owner-settings"; ownerKey: string; name: string; tab: number;
  loading: boolean; settings: MemorySettings | null; config: MemorySearchConfig | null;
  chats: MemoryChat[] | null; missingChatIds: string[]; queue: MemoryAutoSaveStatus | null; progress: MemoryRebuildProgress | null;
  searchQuery: string; searchThreshold: number; searchResult: import("../../src/model").MemorySearchDebug | null;
  statusError: string; selectedChatIds: string[]; windowMessageCount: number; from: string; to: string; pendingClearFailed: boolean;
}
export interface UserDialog extends DialogState {
  type: "user"; ownerKey: string; name: string; content: string; loading: boolean; loaded: boolean;
}
export type MemoryEditorDraft = Omit<WebRequest<"saveMemory">, "action" | "ownerKey" | "originalTitle">;
export interface MemoryEditDialog extends DialogState {
  type: "memory-edit"; ownerKey: string; originalTitle: string | null; draft: MemoryEditorDraft;
}
export type LinkDraft = Omit<WebRequest<"createLink">, "action" | "ownerKey" | "weight"> & { weight: number | string };
export interface LinkDialog extends DialogState { type: "link"; ownerKey: string; items: Memory[]; draft: LinkDraft }
export interface LinkEditDialog extends DialogState { type: "link-edit"; ownerKey: string; linkId: WebRequest<"updateLink">["linkId"]; draft: LinkDraft }
export type DeleteRequest = WebRequest<"deleteStore" | "deleteMemory" | "deleteLink">;
export interface ConfirmDialog extends DialogState { type: "confirm"; title: string; message: string; operation: DeleteRequest; closeParent: boolean }
export interface ExportDialog extends DialogState { type: "export"; name: string; content: string; typeName: "character" | "group" | "memory" }
export interface ImportDialog extends DialogState { type: "import"; target: "card" | "group" | "memory"; ownerKey: string | null; format: "operit" | "tavern"; strategy: "SKIP" | "UPDATE" | "CREATE_NEW"; content: string }
export interface InfoDialog extends DialogState { type: "info"; title: string; content: string }
export type EditableDialog = StoreDialog | MemoryEditDialog | LinkDialog | LinkEditDialog | UserDialog | ImportDialog | ExportDialog | OwnerSettingsDialog;
export interface TextDialog extends DialogState { type: "text"; key: string; value: string; label: string; parent: EditableDialog }
export interface GraphDialog extends DialogState {
  type: "graph"; ownerKey: string; name: string; loading: boolean; data: MemoryGraphResult | null;
  selected: string | null; selectedEdge: MemoryGraphResult["graph"]["edges"][number]["id"] | null; linkMode: boolean; linkSource: string | null;
  viewport: { width: number; height: number } | null; layoutSignature: string | null;
  sizes: Map<string, NodeSize>; positions: Map<string, Point>; camera: GraphCamera;
  query: string; appliedQuery: string; searchIds: Set<string> | null; folder: string;
}
export interface ChatBindingDialog extends DialogState {
  type: "chat-binding"; cardId: string; query: string; filter: "all" | "unbound";
  loading: boolean; chats: BindableChat[]; selectedIds: string[];
  failures: Record<string, string>; completed: number; total: number;
}
export type Dialog = ChatBindingDialog | StoreDialog | OwnerSettingsDialog | UserDialog | MemoryEditDialog | LinkDialog | LinkEditDialog | ConfirmDialog | ExportDialog | ImportDialog | TextDialog | InfoDialog | GraphDialog;
export type DialogType = Dialog["type"];
export type DialogOf<T extends DialogType> = Extract<Dialog, { type: T }>;
export type DialogInput = {
  [T in DialogType]: Omit<DialogOf<T>, keyof DialogState>
}[DialogType];
export interface DialogSpec { title: string; style: string; tabs: string[]; body: string; footer: string }
export interface GraphPointer {
  surface: SVGSVGElement; startX: number; startY: number; x: number; y: number;
  node: { id: string; start: Point } | null;
}
export interface UIState {
  managementView: "characters" | "memory"; avatarSources: Map<string, string>; snapshot: Snapshot | null; dialogs: Dialog[]; busy: boolean; error: string;
  toastTimer: ReturnType<typeof setTimeout> | null; pointers: Map<number, GraphPointer>; gestureMoved: boolean;
}
