import type { Memory, Snapshot } from "../../../src/model";
import type { AttachmentScreen, ScreenFeatureContext } from "../../bridge/screens";
import type { ActionFeature } from "../../bridge/context";
import { dataValue } from "../../shared/dom";
import { button, escapeHtml, icon, iconButton } from "../../shared/ui/html";

export interface AttachmentOwner { key: string; name: string; kind: "CHARACTER" | "SHARED" }
export interface AttachmentState {
  mode: "memory-attachment";
  input: AttachmentScreen;
  step: "owners" | "folders" | "content";
  owner: AttachmentOwner | null;
  folders: string[] | null;
  folder: string | null;
  items: Memory[] | null;
  loading: boolean;
}

/** Starts owner selection with no synthetic folder list or memory records. */
export function createAttachmentState(input: AttachmentScreen): AttachmentState {
  return { mode: "memory-attachment", input, step: "owners", owner: null, folders: null, folder: null, items: null, loading: false };
}

/** Keeps picker navigation inside the immutable owner and folder requested by its actual caller. */
export function canAttachmentGoBack(state: AttachmentState): boolean {
  switch (state.step) {
    case "owners": return false;
    case "folders": return state.input.ownerKey === null;
    case "content": return state.input.folderPath === null;
  }
}

/** Lists private character stores and shared stores exactly as the original attachment dialog does. */
export function attachmentOwners(snapshot: Snapshot): AttachmentOwner[] {
  const owners: AttachmentOwner[] = [];
  for (const card of snapshot.cards) owners.push({ key: `character:${card.id}`, name: card.name, kind: "CHARACTER" });
  for (const store of snapshot.stores) owners.push({ key: `shared:${store.id}`, name: store.name, kind: "SHARED" });
  return owners;
}

/** Requires the exact owner selected by the user or the supplied presentation context. */
export function attachmentOwner(snapshot: Snapshot, ownerKey: string): AttachmentOwner {
  for (const owner of attachmentOwners(snapshot)) if (owner.key === ownerKey) return owner;
  throw new Error(`记忆库不存在：${ownerKey}`);
}

/** Includes the root directory as a real selectable folder alongside the repository's folder paths. */
export function attachmentFolders(folders: readonly string[]): string[] {
  const paths = new Set<string>([""]);
  for (const folder of folders) {
    if (typeof folder !== "string") throw new Error("记忆库返回了无效的文件夹路径");
    paths.add(folder);
  }
  return [...paths].sort();
}

/** Maps the repository's explicit null folder to its domain root, not to a replacement record. */
export function memoryFolder(memory: Memory): string {
  return memory.folderPath === null ? "" : memory.folderPath;
}

/** Keeps exact-folder semantics instead of including descendants implicitly. */
export function attachmentItems(items: readonly Memory[], folderPath: string): Memory[] {
  const selected: Memory[] = [];
  for (const memory of items) if (memoryFolder(memory) === folderPath) selected.push(memory);
  return selected;
}

/** Reproduces the original owner/folder attachment Markdown with unmodified titles and content. */
export function attachmentContent(owner: AttachmentOwner, folderPath: string, items: readonly Memory[]): string {
  if (items.length === 0) return "";
  const sections: string[] = [];
  for (const item of items) sections.push(`## ${item.title}\n\n${item.content}`);
  return `# ${owner.name} / ${folderPath === "" ? "/" : folderPath}\n\n${sections.join("\n\n")}`;
}

/** Drives the original owner/folder/content flow using the plugin's real finite Request operations. */
export function createAttachmentFeature(context: ScreenFeatureContext): ActionFeature & {
  /** Opens explicitly supplied picker context after canonical records are loaded. */
  initialize(): Promise<void>;
} {
  const { editor, screen, render, complete } = context;
  /** Loads a real folder catalogue for exactly this owner. */
  async function loadFolders(owner: AttachmentOwner): Promise<void> {
    const state = screen("memory-attachment");
    if (state.input.ownerKey !== null && state.input.ownerKey !== owner.key) throw new Error("不能更改调用方指定的记忆库");
    state.owner = owner; state.step = "folders"; state.folders = null; state.folder = null; state.items = null;
    state.loading = true; editor.state.error = ""; render();
    try { state.folders = attachmentFolders(await editor.request({ action: "listMemoryFolders", ownerKey: owner.key })); }
    catch (error) { editor.state.error = String(error); throw error; }
    finally { state.loading = false; render(); }
  }
  /** Loads complete memories and restricts the attachment to the exact selected folder. */
  async function loadContent(folder: string): Promise<void> {
    const state = screen("memory-attachment");
    if (state.owner === null || state.folders === null) throw new Error("请先选择记忆库");
    if (state.input.folderPath !== null && state.input.folderPath !== folder) throw new Error("不能更改调用方指定的记忆文件夹");
    if (state.folders.indexOf(folder) < 0) throw new Error("选择的记忆文件夹不存在");
    state.folder = folder; state.items = null; state.step = "content";
    state.loading = true; editor.state.error = ""; render();
    try { state.items = attachmentItems(await editor.request({ action: "listMemories", ownerKey: state.owner.key }), folder); }
    catch (error) { editor.state.error = String(error); throw error; }
    finally { state.loading = false; render(); }
  }
  /** Applies route-supplied owner/folder values only through actual repository reads. */
  async function initialize(): Promise<void> {
    const state = screen("memory-attachment"), input = state.input;
    if (input.ownerKey === null) return;
    try {
      await loadFolders(attachmentOwner(editor.snapshot(), input.ownerKey));
      if (input.folderPath !== null) await loadContent(input.folderPath);
    } catch (error) { editor.state.error = String(error); render(); }
  }
  /** Handles exact picker actions without interpreting owner strings as host capability names. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    const state = screen("memory-attachment");
    switch (action) {
      case "attachment-owner":
        if (state.step !== "owners") throw new Error("当前不是记忆库选择步骤");
        await loadFolders(attachmentOwner(editor.snapshot(), dataValue(element, "owner"))); return;
      case "attachment-folder":
        if (state.step !== "folders") throw new Error("当前不是记忆文件夹选择步骤");
        await loadContent(dataValue(element, "folder")); return;
      case "attachment-back":
        if (!canAttachmentGoBack(state)) throw new Error("不能退出调用方指定的记忆附件范围");
        editor.state.error = "";
        if (state.step === "content") { state.step = "folders"; state.folder = null; state.items = null; }
        else { state.step = "owners"; state.owner = null; state.folders = null; }
        render(); return;
      case "attachment-retry":
        if (state.input.ownerKey !== null && (state.folders === null || state.step === "folders" && state.input.folderPath !== null)) { await initialize(); return; }
        if (state.step === "content" && state.folder !== null) { await loadContent(state.folder); return; }
        if (state.step === "folders" && state.owner !== null) { await loadFolders(state.owner); return; }
        throw new Error("当前没有可重试的记忆读取");
      case "confirm-attachment": {
        if (state.owner === null || state.folder === null || state.items === null) throw new Error("记忆附件正文尚未读取");
        if (state.loading || editor.state.error !== "") throw new Error("记忆附件尚未成功读取");
        const content = attachmentContent(state.owner, state.folder, state.items);
        if (content === "") throw new Error("当前文件夹没有可添加的记忆");
        await complete({ mode: "memory-attachment", ownerKey: state.owner.key, folderPath: state.folder, content }); return;
      }
      default: throw new Error(`Unknown memory attachment action: ${action}`);
    }
  }
  return { names: ["attachment-owner", "attachment-folder", "attachment-back", "attachment-retry", "confirm-attachment"], handleAction, initialize };
}

/** Renders one owner/folder step or the original selectable attachment-content preview. */
export function attachmentPage(state: AttachmentState, snapshot: Snapshot, error: string, disabled: boolean): string {
  const title = state.owner === null ? "记忆附件" : state.owner.name;
  let body = "", content = "";
  if (state.loading) body = '<div class="loading" role="status">正在读取记忆库…</div>';
  else if (error !== "") body = `<div class="error-banner" role="alert">${escapeHtml(error)}</div>${button("attachment-retry", "重试", "refresh", disabled ? "disabled" : "")}`;
  else switch (state.step) {
    case "owners":
      for (const owner of attachmentOwners(snapshot)) body += `<button type="button" class="picker-row" data-action="attachment-owner" data-owner="${escapeHtml(owner.key)}" ${disabled ? "disabled" : ""}>${icon(owner.kind === "SHARED" ? "groups" : "person")}<span>${escapeHtml(owner.name)}</span></button>`;
      if (body === "") body = '<div class="empty">没有可选的记忆库。</div>';
      break;
    case "folders":
      if (state.folders === null) throw new Error("记忆文件夹尚未读取");
      for (const folder of state.folders) body += `<button type="button" class="picker-row" data-action="attachment-folder" data-folder="${escapeHtml(folder)}" ${disabled ? "disabled" : ""}>${icon("folder")}<span>${escapeHtml(folder === "" ? "/" : folder)}</span></button>`;
      break;
    case "content":
      if (state.owner === null || state.folder === null || state.items === null) throw new Error("记忆附件正文尚未读取");
      content = attachmentContent(state.owner, state.folder, state.items);
      body = content === "" ? '<div class="empty">当前文件夹没有记忆。</div>' : `<pre class="attachment-content" tabindex="0" aria-label="记忆附件正文">${escapeHtml(content)}</pre>`;
      break;
  }
  const back = canAttachmentGoBack(state) ? iconButton("attachment-back", "返回", "back", state.loading || disabled ? "disabled" : "") : "";
  return `<div class="page presentation-page attachment-page"><header class="appbar">${back}<h1>${escapeHtml(title)}</h1></header><div class="scroll attachment-body">${body}</div><footer class="presentation-footer">${button("screen-cancel", "取消", "", disabled || state.loading ? "disabled" : "")}${content === "" ? "" : button("confirm-attachment", "添加记忆附件", "add", disabled || state.loading ? "disabled" : "", "filled")}</footer></div>`;
}
