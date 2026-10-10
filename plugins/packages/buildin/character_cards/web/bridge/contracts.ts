import type {
  Card, Group, GroupValues, Memory, MemoryGraphResult, Snapshot,
  Request, RequestResults, StoreValues, TagChanges, TagEdit, Theme, ToolAccessConfig,
} from "../../src/model";
import type { CurrentScreen, EntityKind, PresentationCancel, PresentationComplete, ScreenResult } from "./screens";
import type { GraphCamera, NodeSize, Point } from "../features/graph/layout";

/** Describes actual service results used by this finite editor bridge. */
export type WebResponses = Pick<RequestResults, "snapshot" | "listThemeChoices" | "saveCharacter" | "deleteCharacter" | "activate" | "saveGroup" | "deleteGroup" | "saveStore" | "deleteStore" | "readUser" | "writeUser" | "graph" | "searchMemory" | "listMemories" | "listMemoryFolders" | "saveMemory" | "deleteMemory" | "createLink" | "deleteLink" | "updateLink" | "readMemorySettings" | "readMemorySearchConfig" | "writeMemorySettings" | "writeMemorySearchConfig" | "importCharacter" | "importGroup" | "exportCharacter" | "exportGroup">;
export type WebAction = keyof WebResponses;
export type WebRequest<A extends WebAction> = Extract<Request, { action: A }>;

/** Declares the existing WebView interface registered by the package's Compose host. */
export interface CharacterMemoryBridge {
  /** Reads the host Material palette without substituting a local theme. */
  currentTheme(): Promise<Theme>;
  /** Reads the exact input and request owned by this existing UiRoute instance. */
  currentScreen(): Promise<CurrentScreen>;
  /** Returns an explicit completion event through the host's existing Compose action channel. */
  completeScreen(result: ScreenResult): Promise<PresentationComplete>;
  /** Returns an explicit cancellation event without modifying plugin records. */
  cancelScreen(): Promise<PresentationCancel>;
  /** Invokes the package provider using a typed domain request and result. */
  request<A extends WebAction>(operation: WebRequest<A>): Promise<WebResponses[A]>;
  /** Returns a readable avatar image, including the real built-in default. */
  avatarImage(uri: string | null): Promise<string>;
  /** Imports one real picker selection; cancellation leaves the existing draft intact. */
  chooseAvatar(): Promise<{ uri: string; source: string } | null>;
  /** Writes an explicitly chosen VFS export path through the host filesystem. */
  exportFile(path: string, content: string): Promise<boolean>;
}
declare global {
  interface Window {
    CharacterMemoryHost?: CharacterMemoryBridge;
    /** Applies a later host Material theme update without losing editor drafts. */
    applyCharacterMemoryTheme(theme: Theme): void;
  }
}

export interface DialogState { error: string; scroll: number; busy: boolean }
export interface PreviewDialog extends DialogState { type: "preview"; entity: EntityKind; id: string; tab: number }
export interface CharacterDialog extends DialogState {
  type: "character"; card: Card; create: boolean; tab: number;
  tags: TagEdit[]; tagChanges: TagChanges; nextTagId: number; ttsBindingEnabled: boolean;
}
export interface ThemeDialog extends DialogState { type: "theme"; parent: CharacterDialog | GroupDialog; query: string; choices: readonly import("../features/characters/bindings/theme").ThemeChoice[] }
export interface TtsDialog extends DialogState { type: "tts"; parent: CharacterDialog; query: string }
export interface ToolAccessDialog extends DialogState { type: "tool-access"; parent: CharacterDialog; draft: ToolAccessConfig }
export interface GroupDialog extends DialogState { type: "group"; group: GroupValues; create: boolean }
export interface StoreDialog extends DialogState { type: "store"; store: StoreValues; create: boolean }
export interface OwnerSettingsDialog extends DialogState {
  type: "owner-settings"; ownerKey: string; name: string; tab: number;
}
export interface UserDialog extends DialogState {
  type: "user"; ownerKey: string; name: string; content: string; loading: boolean; loaded: boolean;
}
export interface TagsDialog extends DialogState { type: "tags"; parent: CharacterDialog }
export interface TagEditDialog extends DialogState { type: "tag-edit"; parent: CharacterDialog; create: boolean; draft: TagEdit }
export interface TagDeleteDialog extends DialogState { type: "tag-delete"; parent: CharacterDialog; tagId: string; name: string }
export type MemoryEditorDraft = Omit<WebRequest<"saveMemory">, "action" | "ownerKey" | "originalTitle">;
export interface MemoryEditDialog extends DialogState {
  type: "memory-edit"; ownerKey: string; originalTitle: string | null; draft: MemoryEditorDraft;
}
export type LinkDraft = Omit<WebRequest<"createLink">, "action" | "ownerKey" | "weight"> & { weight: number | string };
export interface LinkDialog extends DialogState { type: "link"; ownerKey: string; items: Memory[]; draft: LinkDraft }
export interface LinkEditDialog extends DialogState { type: "link-edit"; ownerKey: string; linkId: WebRequest<"updateLink">["linkId"]; draft: LinkDraft }
export type DeleteRequest = WebRequest<"deleteCharacter" | "deleteGroup" | "deleteStore" | "deleteMemory" | "deleteLink">;
export interface ConfirmDialog extends DialogState { type: "confirm"; title: string; message: string; operation: DeleteRequest; closeParent: boolean }
export interface ExportDialog extends DialogState { type: "export"; path: string; content: string; typeName: "character" | "group" }
export interface ImportDialog extends DialogState { type: "import"; target: "card" | "group"; content: string }
export interface InfoDialog extends DialogState { type: "info"; title: string; content: string }
export type EditableDialog = CharacterDialog | GroupDialog | StoreDialog | MemoryEditDialog | LinkDialog | LinkEditDialog | TagEditDialog | UserDialog | ImportDialog | ExportDialog;
export interface TextDialog extends DialogState { type: "text"; key: string; value: string; label: string; parent: EditableDialog }
export interface GraphDialog extends DialogState {
  type: "graph"; ownerKey: string; name: string; loading: boolean; data: MemoryGraphResult | null;
  selected: string | null; selectedEdge: MemoryGraphResult["graph"]["edges"][number]["id"] | null; linkMode: boolean; linkSource: string | null;
  viewport: { width: number; height: number } | null; layoutSignature: string | null;
  sizes: Map<string, NodeSize>; positions: Map<string, Point>; camera: GraphCamera;
  query: string; appliedQuery: string; searchIds: Set<string> | null; folder: string;
}
export type Dialog = PreviewDialog | CharacterDialog | ThemeDialog | TtsDialog | ToolAccessDialog | GroupDialog | StoreDialog | OwnerSettingsDialog | UserDialog | TagsDialog | TagEditDialog | TagDeleteDialog | MemoryEditDialog | LinkDialog | LinkEditDialog | ConfirmDialog | ExportDialog | ImportDialog | TextDialog | InfoDialog | GraphDialog;
export type DialogType = Dialog["type"];
export type DialogOf<T extends DialogType> = Extract<Dialog, { type: T }>;
export type DialogInput = {
  [T in DialogType]: Omit<DialogOf<T>, keyof DialogState | (T extends "character" ? "tags" | "tagChanges" | "nextTagId" | "ttsBindingEnabled" : never)>
}[DialogType];
export interface DialogSpec { title: string; style: string; tabs: string[]; body: string; footer: string }
export interface GraphPointer {
  surface: SVGSVGElement; startX: number; startY: number; x: number; y: number;
  node: { id: string; start: Point } | null;
}
export interface UIState {
  managementView: "characters" | "memory"; themeChoices: readonly import("../../src/selection-application").ThemeChoice[] | null; avatarSources: Map<string, string>; snapshot: Snapshot | null; dialogs: Dialog[]; busy: boolean; error: string;
  toastTimer: ReturnType<typeof setTimeout> | null; pointers: Map<number, GraphPointer>; gestureMoved: boolean;
}
export type NativeRecord = Card | Group;
