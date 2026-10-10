import type { Snapshot } from "../../src/model";
import { createTtsFeature } from "../features/characters/bindings/tts";
import { createThemeFeature } from "../features/characters/bindings/theme";
import { createToolPolicyFeature } from "../features/characters/bindings/tool-policy";
import { createCharactersActionsFeature } from "../features/characters/actions";
import { createPageRenderer } from "../features/characters/page";
import { createPreviewFeature } from "../features/characters/preview-actions";
import { createCharactersTagsActionsFeature } from "../features/characters/tags-actions";
import { createCharactersTransfersFeature } from "../features/characters/transfers";
import { createGraphActionsFeature } from "../features/graph/actions";
import { updateGraphSurface as paintGraph } from "../features/graph/canvas";
import { createGraphGestures, discardGraphPointers } from "../features/graph/gestures";
import { createGroupsActionsFeature } from "../features/groups/actions";
import { createMemoryControlsFeature } from "../features/memory/controls-actions";
import { createMemoryItemActionsFeature } from "../features/memory/item-actions";
import { createMemoryLibraryActionsFeature } from "../features/memory/library-actions";
import { createMemoryLinkActionsFeature } from "../features/memory/link-actions";
import { createProfileFeature } from "../features/memory/profile";
import { requireElement } from "../shared/dom";
import { copy, escapeHtml } from "../shared/ui/html";
import { createDialogActions } from "./actions";
import type { EditorContext } from "./context";
import type { Dialog, DialogInput, DialogOf, DialogType, GraphDialog, WebAction, WebRequest, WebResponses } from "./contracts";
import { createDialogRenderer } from "./dialogs";
import { createEditorEvents } from "./events";
import { createState } from "./state";
import { createScreenController } from "./screens";
import type { PresentedScreen, ScreenReceiver } from "./screens";
import { host } from "./transport";
/** Assembles isolated feature factories around one explicit, ephemeral editor context. */
export function createEditor() {
  const state = createState();
  const app = requireElement(document.getElementById("app"), HTMLElement);
  const dialogsRoot = requireElement(document.getElementById("dialogs"), HTMLElement);
  /** Returns the current nested dialog without selecting a different owner implicitly. */
  function topDialog(): Dialog;
  /** Requires the active dialog to have the expected finite discriminator. */
  function topDialog<T extends DialogType>(type: T): DialogOf<T>;
  /** Resolves the current dialog and rejects actions routed to another editor type. */
  function topDialog(type?: DialogType): Dialog {
    if (state.dialogs.length === 0) throw new Error("当前没有打开的编辑器");
    const dialog = state.dialogs[state.dialogs.length - 1];
    if (type !== undefined && dialog.type !== type) throw new Error(`当前编辑器不是 ${type}`);
    return dialog;
  }

  /** Opens a typed draft and returns the same state object tracked by the dialog stack. */
  function pushDialog<T extends DialogInput>(draft: T): DialogOf<T["type"]>;
  /** Initializes common dialog state and the enclosing character's staged tag transaction. */
  function pushDialog(draft: DialogInput): Dialog {
    const common = { error: "", scroll: 0, busy: false };
    const dialog: Dialog = draft.type === "character"
      ? { ...draft, ...common, tags: copy(snapshot().tags), tagChanges: { created: [], updated: [], deleted: [] }, nextTagId: 0, ttsBindingEnabled: draft.card.ttsConfigId !== null }
      : { ...draft, ...common };
    state.dialogs.push(dialog); renderDialogs(); return dialog;
  }

  /** Closes only the most recent dialog and restores its owning editor. */
  function popDialog(): void {
    if (topDialog().type === "graph") discardGraphPointers(state);
    state.dialogs.pop();
    renderDialogs();
  }

  /** Requires the loaded aggregate before displaying or editing persisted records. */
  function snapshot(): Snapshot {
    if (state.snapshot === null) throw new Error("角色卡目录尚未加载");
    return state.snapshot;
  }

  /** Locates the exact graph owning a nested edit operation. */
  function owningGraph(ownerKey: string): GraphDialog {
    for (let index = state.dialogs.length - 1; index >= 0; index--) {
      const dialog = state.dialogs[index];
      if (dialog.type === "graph" && dialog.ownerKey === ownerKey) return dialog;
    }
    throw new Error("记忆编辑器的所属图谱已关闭");
  }

  /** Displays a temporary announcement using the authoritative Material palette. */
  function toast(message: string): void {
    const element = requireElement(document.getElementById("snackbar"), HTMLElement);
    element.textContent = message; element.className = "visible";
    if (state.toastTimer !== null) clearTimeout(state.toastTimer);
    /** Clears the announcement after its accessible display interval. */
    state.toastTimer = setTimeout(() => { element.className = ""; }, 3500);
  }

  /** Calls the only registered host transport and rejects malformed snapshots. */
  async function request<A extends WebAction>(operation: WebRequest<A>): Promise<WebResponses[A]> {
    const dialog = state.dialogs.length === 0 ? null : topDialog();
    if (dialog !== null) { dialog.busy = true; renderDialogs(); }
    try { return await host().request(operation); }
    finally { if (dialog !== null) dialog.busy = false; }
  }
  /** Delegates top-level painting to the role-card page composer. */
  function renderMain(): void { screens.renderMain(); }
  /** Delegates stack painting to the shared Material dialog surface. */
  function renderDialogs(): void { dialogs.renderDialogs(); }
  const context: EditorContext = {
state, app, dialogsRoot, snapshot, topDialog, pushDialog, popDialog, request, renderMain, renderDialogs, toast, owningGraph,
    /** Cancels only the presented root before removing the current local dialog. */
    dismissDialog: async () => { await screens.dismissDialog(topDialog()); popDialog(); renderMain(); },
    /** Delivers a successful save or deletion through the current entity presentation. */
    entityChanged: (entity, id, operation) => screens.entityChanged(entity, id, operation),
    /** Delegates exact screen fields to their owning feature. */
    onScreenInput: element => screens.onInput(element),
    /** Opens the owner's loaded graph through its feature. */
    openGraph: (owner, name) => graph.openGraph(owner, name),
    /** Opens the owner's profile through its feature. */
    openUser: (owner, name) => profile.openUser(owner, name),
    /** Opens a saved-record export through the transfer feature. */
    openExport: (type, value) => transfers.openExport(type, value),
    /** Paints a graph without changing the captured SVG root. */
    updateGraphSurface: dialog => paintGraph(dialog, dialogsRoot),
    /** Shares the exact click dispatcher with keyboard graph activation. */
    onClick: event => events.onClick(event),
  };
  const page = createPageRenderer(context), dialogs = createDialogRenderer(context);
  const screens = createScreenController(context, page);
  const graph = createGraphActionsFeature(context), profile = createProfileFeature(context), transfers = createCharactersTransfersFeature(context);
  const themePicker = createThemeFeature(context,
    /** Reads independent theme choices through the existing package request channel. */
    () => context.request({ action: "listThemeChoices" }),
  );
  const events = createEditorEvents(context, [...screens.features, themePicker, createTtsFeature(context), createToolPolicyFeature(context), createPreviewFeature(context), createDialogActions(context), createCharactersActionsFeature(context), createCharactersTagsActionsFeature(context), createGroupsActionsFeature(context), createMemoryLibraryActionsFeature(context), createMemoryItemActionsFeature(context), createMemoryLinkActionsFeature(context), transfers, graph, profile, createMemoryControlsFeature(context)]);
  const gestures = createGraphGestures(context);
  /** Mounts finite DOM events once, with graph-specific gesture handlers kept in their own feature. */
  function mount(): void {
    document.addEventListener("click", events.onClick);
    document.addEventListener("input", events.onInput);
    document.addEventListener("change", events.onChange);
    document.addEventListener("keydown", screens.onKeyDown);
    dialogsRoot.addEventListener("cancel", events.onCancel, true);
    dialogsRoot.addEventListener("pointerdown", gestures.onPointerDown);
    dialogsRoot.addEventListener("pointermove", gestures.onPointerMove);
    dialogsRoot.addEventListener("pointerup", gestures.onPointerUp);
    dialogsRoot.addEventListener("pointercancel", gestures.onPointerUp);
    dialogsRoot.addEventListener("wheel", gestures.onWheel, { passive: false });
    dialogsRoot.addEventListener("keydown", gestures.onKeyDown);
  }
  /** Hydrates image sources without modifying persisted character paths. */
  async function loadAvatars(): Promise<void> {
    await Promise.all(snapshot().cards.map(async card => {
      const key = card.avatarUri ?? "";
      if (!state.avatarSources.has(key)) state.avatarSources.set(key, await host().avatarImage(card.avatarUri));
    }));
  }
  /** Loads the real package aggregate before exposing editable controls. */
  async function initialize(view: "characters" | "memory" = "characters"): Promise<void> {
    state.managementView = view;
    state.busy = true;
    try { state.snapshot = await request({ action: "snapshot" }); await loadAvatars(); }
    finally { state.busy = false; renderMain(); }
  }
  /** Presents a startup failure without substituting records or transports. */
  function showStartupError(error: unknown): void { app.innerHTML = `<div class="scroll"><h1>角色卡插件未能加载</h1><p role="alert">${escapeHtml(String(error))}</p></div>`; }
  /** Initializes a typed popup with its actual bridge receiver and serializes initial repository reads. */
  async function initializeScreen(input: PresentedScreen, receiver: ScreenReceiver): Promise<void> {
    state.busy = true;
    try {
      state.snapshot = await request({ action: "snapshot" });
      await loadAvatars();
      await screens.enter(input, receiver);
    } finally { state.busy = false; renderMain(); }
  }
  return { state, mount, initialize, initializeScreen, showStartupError };
}
