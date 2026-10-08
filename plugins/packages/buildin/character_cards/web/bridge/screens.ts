import type { CurrentScreen, EditScreen, EntityKind, PresentedScreen, PreviewScreen, ScreenReceiver, ScreenResult } from "../../src/presentation";
import type { EditorContext, ActionFeature } from "./context";
import type { Dialog, CharacterMemoryBridge } from "./contracts";
import { type AttachmentState, createAttachmentFeature, createAttachmentState, attachmentPage } from "../features/memory/attachments";
import { type SelectionState, createSelectionFeature, validateSelection, createSelectionState, selectionPage } from "../features/characters/selection";
import { parseScreenInput, parseScreenResult } from "../../src/presentation";
import { host } from "./transport";
import { getCard } from "../features/characters/data";
import { getGroup } from "../features/groups/data";
import { copy, escapeHtml } from "../shared/ui/html";

/** Keeps presentation domain contracts in this plugin, shared by its service and Web entry. */
export type {
  AttachmentScreen, CurrentScreen, EditScreen, EntityKind, ManageScreen,
  PresentationCancel, PresentationComplete, PresentedScreen, PreviewScreen,
  ScreenInput, ScreenReceiver, ScreenResult, SelectScreen, SelectionKind,
} from "../../src/presentation";

/** Injects presentation state and result delivery without declaring a host implementation. */
export interface ScreenFeatureContext {
  editor: EditorContext;
  /** Requires this feature's exact current screen discriminator. */
  screen<T extends ScreenState["mode"]>(mode: T): ScreenStateOf<T>;
  /** Repaints only the current presentation page. */
  render(): void;
  /** Delivers a typed result through the explicitly supplied real receiver. */
  complete(result: ScreenResult): Promise<void>;
  /** Cancels the explicitly supplied presentation receiver. */
  cancel(): Promise<void>;
}

export interface PreviewState { mode: "preview"; input: PreviewScreen; root: Dialog | null }
export interface EditState { mode: "edit"; input: EditScreen; root: Dialog | null }
export type ScreenState = { mode: "manage" } | SelectionState | AttachmentState | PreviewState | EditState;
export type ScreenStateOf<T extends ScreenState["mode"]> = Extract<ScreenState, { mode: T }>;

export { parseScreenInput } from "../../src/presentation";

/** Validates the existing package bridge envelope without inferring a screen or request identifier. */
export function parseCurrentScreen(value: unknown): CurrentScreen {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("currentScreen 必须返回对象");
  const record = value as Record<string, unknown>, keys = Object.keys(record);
  if (keys.length !== 2 || !keys.every(
    /** Accepts only the two fields declared by the plugin's actual host bridge. */
    key => key === "requestId" || key === "input",
  )) throw new Error("currentScreen 必须只包含 requestId 和 input");
  const input = parseScreenInput(record.input);
  if (input.mode === "manage") {
    if (record.requestId !== null) throw new Error("管理视图不能包含 presentation requestId");
    return { requestId: null, input };
  }
  if (typeof record.requestId !== "string" || record.requestId.trim() === "") throw new Error("弹窗视图缺少真实 presentation requestId");
  return { requestId: record.requestId, input };
}

/** Delivers one plugin result through the already registered WebView action-result channel. */
export function createHostScreenReceiver(bridge: CharacterMemoryBridge, requestId: string, input: PresentedScreen): ScreenReceiver {
  return {
    /** Completes this exact host request after validating its package-owned result fields. */
    async complete(result: ScreenResult): Promise<void> {
      const value = parseScreenResult(result, input);
      const acknowledgement = await bridge.completeScreen(value);
      if (acknowledgement.type !== "toolpkg.presentation.complete" || acknowledgement.requestId !== requestId) throw new Error("宿主未确认当前 presentation 的完成结果");
    },
    /** Cancels the real caller-owned request without changing the active actor or returning a selection. */
    async cancel(): Promise<void> {
      const acknowledgement = await bridge.cancelScreen();
      if (acknowledgement.type !== "toolpkg.presentation.cancel" || acknowledgement.requestId !== requestId) throw new Error("宿主未确认当前 presentation 的取消结果");
    },
  };
}

/** Describes the same editor reused by the management route and caller-owned plugin popups. */
export interface PresentedEditor {
  /** Loads the management screen without creating a presentation result channel. */
  initialize(): Promise<void>;
  /** Loads one real route input with its existing WebView result receiver. */
  initializeScreen(input: PresentedScreen, receiver: ScreenReceiver): Promise<void>;
}

/** Starts the exact mode supplied by UiRoute initialState through the existing package host bridge. */
export async function initializeHostScreen(editor: PresentedEditor): Promise<void> {
  const bridge = host(), current = parseCurrentScreen(await bridge.currentScreen());
  if (current.input.mode === "manage") { await editor.initialize(); return; }
  if (current.requestId === null) throw new Error("弹窗视图没有真实 presentation requestId");
  await editor.initializeScreen(current.input, createHostScreenReceiver(bridge, current.requestId, current.input));
}

/** Assembles package-owned views; actual presentation transport must be injected by its caller. */
export function createScreenController(editor: EditorContext, management: { /** Paints the existing manager. */ renderMain(): void }) {
  let active: ScreenState = { mode: "manage" };
  let receiver: ScreenReceiver | null = null;
  let finished = false;

  /** Requires the exact current view rather than selecting another mode implicitly. */
  function screen<T extends ScreenState["mode"]>(mode: T): ScreenStateOf<T> {
    if (active.mode !== mode) throw new Error(`当前插件视图不是 ${mode}`);
    return active as ScreenStateOf<T>;
  }

  /** Paints a presentation page or its original management view without duplicating editors. */
  function renderMain(): void {
    if (editor.state.snapshot === null) return;
    if (finished) { editor.app.innerHTML = '<div class="loading" role="status">当前视图已结束。</div>'; return; }
    switch (active.mode) {
      case "manage": management.renderMain(); return;
      case "select": editor.app.innerHTML = selectionPage(active, editor.snapshot(), editor.state.error, editor.state.busy); return;
      case "memory-attachment": editor.app.innerHTML = attachmentPage(active, editor.snapshot(), editor.state.error, editor.state.busy); return;
      case "preview":
      case "edit": editor.app.innerHTML = `<div class="presentation-backdrop" aria-label="${active.mode === "preview" ? "实体预览" : "实体编辑"}">${editor.state.error === "" ? "" : `<div class="error-banner" role="alert">${escapeHtml(editor.state.error)}</div>`}</div>`; return;
    }
  }

  /** Returns a result only to an explicitly supplied receiver for this exact active screen. */
  async function complete(result: ScreenResult): Promise<void> {
    if (receiver === null || active.mode === "manage" || finished) throw new Error("当前视图没有活动的 presentation 接收方");
    if (result.mode !== active.mode) throw new Error("视图结果类型与当前视图不匹配");
    await receiver.complete(result); finished = true; renderMain();
  }

  /** Cancels only an explicitly presented view; management has no invented completion channel. */
  async function cancel(): Promise<void> {
    if (receiver === null || active.mode === "manage" || finished) throw new Error("当前视图没有活动的 presentation 接收方");
    await receiver.cancel(); finished = true; renderMain();
  }

  const context: ScreenFeatureContext = { editor, screen, render: renderMain, complete, cancel };
  const selection = createSelectionFeature(context), attachment = createAttachmentFeature(context);
  /** Dispatches only the shared presentation cancellation action. */
  async function handleAction(action: string): Promise<void> {
    if (action !== "screen-cancel") throw new Error(`Unknown presentation action: ${action}`);
    await cancel();
  }
  const common: ActionFeature = { names: ["screen-cancel"], handleAction };

  /** Starts one typed view only after real package records and an actual receiver are available. */
  async function enter(input: PresentedScreen, target: ScreenReceiver): Promise<void> {
    if (active.mode !== "manage" || editor.state.dialogs.length !== 0) throw new Error("当前插件实例已有活动视图");
    receiver = target; editor.state.error = "";
    switch (input.mode) {
      case "select":
        validateSelection(input.selected, input.kind, editor.snapshot()); active = createSelectionState(input); break;
      case "memory-attachment": active = createAttachmentState(input); break;
      case "preview": active = { mode: "preview", input, root: null }; break;
      case "edit": active = { mode: "edit", input, root: null }; break;
    }
    renderMain();
    switch (active.mode) {
      case "select": return;
      case "memory-attachment": await attachment.initialize(); return;
      case "preview": {
        if (input.mode !== "preview") throw new Error("预览上下文不匹配");
        if (input.entity === "card") getCard(editor.snapshot(), input.id); else getGroup(editor.snapshot(), input.id);
        active.root = editor.pushDialog({ type: "preview", entity: input.entity, id: input.id, tab: 0 }); return;
      }
      case "edit": {
        if (input.mode !== "edit") throw new Error("编辑上下文不匹配");
        active.root = input.entity === "card"
          ? editor.pushDialog({ type: "character", card: copy(getCard(editor.snapshot(), input.id)), create: false, tab: 0 })
          : editor.pushDialog({ type: "group", group: copy(getGroup(editor.snapshot(), input.id)), create: false }); return;
      }
      default: { const unexpected: never = active; throw new Error(`Unexpected presented view: ${String(unexpected)}`); }
    }
  }

  /** Observes explicit close actions on the presented root, not on nested editors. */
  async function dismissDialog(dialog: Dialog): Promise<void> {
    if ((active.mode !== "preview" && active.mode !== "edit") || active.root !== dialog) return;
    if (active.mode === "preview") await complete({ mode: "preview", entity: active.input.entity, id: active.input.id });
    else await cancel();
  }

  /** Completes an edit presentation only after its corresponding real save or delete succeeds. */
  async function entityChanged(entity: EntityKind, id: string, operation: "saved" | "deleted"): Promise<void> {
    if (active.mode !== "edit") return;
    if (active.input.entity !== entity || active.input.id !== id) return;
    if (active.root === null || editor.state.dialogs.indexOf(active.root) < 0) throw new Error("编辑 presentation 的根编辑器已关闭");
    await complete({ mode: "edit", entity, id, operation });
  }

  /** Handles searchable screen fields without moving them into the generic editor draft contract. */
  function onInput(element: HTMLInputElement): boolean {
    if (!element.hasAttribute("data-selection-query")) return false;
    selection.onInput(element); return true;
  }

  /** Makes Escape cancel a standalone selector or attachment picker, while nested dialogs own their Escape. */
  async function onKeyDown(event: KeyboardEvent): Promise<void> {
    if (event.key !== "Escape" || active.mode === "manage" || editor.state.dialogs.length !== 0 || editor.state.busy || finished) return;
    event.preventDefault(); editor.state.busy = true;
    try { await cancel(); }
    catch (error) { editor.state.error = String(error); }
    finally { editor.state.busy = false; renderMain(); }
  }
  return { renderMain, enter, dismissDialog, entityChanged, onInput, onKeyDown, features: [common, selection, attachment] };
}
