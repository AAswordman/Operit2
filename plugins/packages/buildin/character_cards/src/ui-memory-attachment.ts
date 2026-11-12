import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import type { Memory, Snapshot } from "./model";
import type { UiPresentation, UiScreenSession } from "./ui-contributions";
import { createUiScreenSession } from "./ui-contributions";
import { nativeUi, uiRequest, uiState } from "./ui-controls";

export interface AttachmentOwner { key: string; name: string }
interface AttachmentState { snapshot: Snapshot | null; step: "owners" | "folders" | "content"; owner: AttachmentOwner | null; folders: string[] | null; folder: string | null; items: Memory[] | null; busy: boolean; error: string; finished: boolean }

/** Lists private and shared owner identities from the existing attachment contract. */
export function memoryAttachmentOwners(snapshot: Snapshot): AttachmentOwner[] { return [...snapshot.cards.map(card => ({ key: "character:" + card.id, name: card.name })), ...snapshot.stores.map(store => ({ key: "shared:" + store.id, name: store.name }))]; }

/** Formats exact-folder attachments without changing user-authored content. */
export function memoryAttachmentContent(owner: AttachmentOwner, folder: string, items: readonly Memory[]): string {
  if (items.length === 0) return "";
  return `# ${owner.name} / ${folder === "" ? "/" : folder}\n\n` + items.map(item => `## ${item.title}\n\n${item.content}`).join("\n\n");
}

/** Implements owner, folder and attachment confirmation entirely in native DSL. */
export function renderMemoryAttachment(ctx: ComposeDslContext): ComposeNode {
  const [presentation] = ctx.useState<UiPresentation | null>("presentation", null);
  const session = ctx.useRef<UiScreenSession | null>("attachment-session", null);
  if (session.current === null) session.current = createUiScreenSession(presentation);
  const current = session.current.currentScreen(), screenSession = session.current;
  if (current.input.mode !== "memory-attachment") throw new Error("The attachment route requires an explicit attachment presentation");
  const input = current.input;
  const ownerPresentation = ctx.useRef("attachment-presentation", JSON.stringify(presentation));
  const { state, live, update } = uiState<AttachmentState>(ctx, "memory-attachment", { snapshot: null, step: "owners", owner: null, folders: null, folder: null, items: null, busy: false, error: "", finished: false });
  const ui = nativeUi(ctx, !state.busy && !state.finished);
  /** Rejects callbacks retained from a different host presentation. */
  function assertOwner(): void { const [actual] = ctx.useState<UiPresentation | null>("presentation", null); if (JSON.stringify(actual) !== ownerPresentation.current) throw new Error("Attachment presentation owner changed"); }
  assertOwner();
  /** Serializes reads and retains failures without fabricating attachment content. */
  async function act(operation: () => Promise<void>): Promise<void> {
    assertOwner(); if (live.current.busy || live.current.finished) return;
    update({ busy: true, error: "" });
    try { await operation(); assertOwner(); } catch (error) { assertOwner(); update({ error: String(error) }); }
    finally { update({ busy: false }); }
  }
  /** Reads only an existing folder and rejects a changed immutable scope. */
  async function content(folder: string): Promise<void> {
    const value = live.current;
    if (value.owner === null || value.folders === null || !value.folders.includes(folder)) throw new Error("请选择存在的记忆文件夹");
    if (input.folderPath !== null && input.folderPath !== folder) throw new Error("不能更改调用方指定的文件夹");
    update({ step: "content", folder, items: null });
    const items = await uiRequest({ action: "listMemories", ownerKey: value.owner.key }); assertOwner();
    update({ items: items.filter(item => (item.folderPath ?? "") === folder) });
  }
  /** Reads real folders and applies the explicitly locked folder when present. */
  async function folders(owner: AttachmentOwner): Promise<void> {
    if (input.ownerKey !== null && input.ownerKey !== owner.key) throw new Error("不能更改调用方指定的记忆库");
    update({ step: "folders", owner, folders: null, folder: null, items: null });
    const paths = await uiRequest({ action: "listMemoryFolders", ownerKey: owner.key }); assertOwner();
    update({ folders: [...new Set(["", ...paths])].sort() });
    if (input.folderPath !== null) await content(input.folderPath);
  }
  /** Initializes the caller-supplied owner without substituting the active card. */
  async function load(): Promise<void> {
    await act(async () => { const snapshot = await uiRequest({ action: "snapshot" }); assertOwner(); update({ snapshot });
      if (input.ownerKey !== null) { const owner = memoryAttachmentOwners(snapshot).find(owner => owner.key === input.ownerKey); if (owner === undefined) throw new Error("调用方指定的记忆库不存在"); await folders(owner); }
    });
  }
  /** Retries a failed read in its existing scope. */
  async function retry(): Promise<void> {
    if (live.current.snapshot === null || live.current.owner === null) { await load(); return; }
    await act(async () => { const value = live.current; if (value.step === "content" && value.folder !== null && value.folders !== null) await content(value.folder); else await folders(value.owner!); });
  }
  /** Completes successfully read, nonempty content through the presentation session. */
  async function confirm(): Promise<ToolPkg.JsonValue> {
    assertOwner(); const value = live.current;
    if (value.busy || value.finished || value.error !== "" || value.owner === null || value.folder === null || value.items === null) return null;
    const content = memoryAttachmentContent(value.owner, value.folder, value.items);
    if (content === "") return null;
    const result = await screenSession.completeScreen({ mode: "memory-attachment", ownerKey: value.owner.key, folderPath: value.folder, content });
    update({ finished: true }); return JSON.parse(JSON.stringify(result)) as ToolPkg.JsonValue;
  }
  /** Cancels once without writing selected memory or chat state. */
  function cancel(): ToolPkg.JsonValue { assertOwner(); if (live.current.busy || live.current.finished) return null; update({ finished: true }); return JSON.parse(JSON.stringify(screenSession.cancelScreen())) as ToolPkg.JsonValue; }
  const children: ComposeNode[] = [];
  if (state.error !== "") children.push(ctx.UI.Text({ key: "attachment-error", text: state.error, color: ctx.MaterialTheme.colorScheme.error }), ui.button("attachment-retry", "重试", retry));
  else if (state.busy || state.snapshot === null) children.push(ui.text("正在读取记忆库…"));
  else if (state.step === "owners") { const owners = memoryAttachmentOwners(state.snapshot); children.push(...owners.map(owner => ctx.UI.Row({ key: "attachment-owner:" + owner.key, fillMaxWidth: true, paddingVertical: 12, spacing: 12, verticalAlignment: "center", onClick: () => act(() => folders(owner)) }, [ctx.UI.Icon({ name: owner.key.startsWith("shared:") ? "GroupsOutlined" : "PersonOutline", size: 22 }), ui.text(owner.name)]))); if (owners.length === 0) children.push(ui.text("没有可选的记忆库")); }
  else if (state.step === "folders") children.push(...(state.folders ?? []).map(folder => ctx.UI.Row({ key: "attachment-folder:" + folder, fillMaxWidth: true, paddingVertical: 12, spacing: 12, verticalAlignment: "center", onClick: () => act(() => content(folder)) }, [ctx.UI.Icon({ name: "FolderOutlined", size: 22 }), ui.text(folder === "" ? "/" : folder)])));
  else if (state.owner !== null && state.folder !== null && state.items !== null) children.push(ui.text(memoryAttachmentContent(state.owner, state.folder, state.items) || "当前文件夹没有记忆"));
  const canBack = state.step === "folders" ? input.ownerKey === null : state.step === "content" && input.folderPath === null;
  return ctx.UI.Column({ key: "memory-attachment-root", fillMaxSize: true, onLoad: load }, [ctx.UI.Row({ fillMaxWidth: true, height: 58, paddingHorizontal: 18, spacing: 12, verticalAlignment: "center" }, [...(canBack ? [ui.iconButton("attachment-back", "返回", "ArrowBack", () => { if (state.step === "content") update({ step: "folders", folder: null, items: null, error: "" }); else update({ step: "owners", owner: null, folders: null, error: "" }); })] : []), ctx.UI.Text({ text: state.owner === null ? "记忆附件" : state.owner.name, fontSize: 20, fontWeight: "500" })]), ui.list("attachment-body", children), ctx.UI.Box({ fillMaxWidth: true, padding: 12 }, ui.actions([ui.button("attachment-cancel", "取消", cancel), ...(state.step === "content" && state.items !== null && state.items.length > 0 && state.error === "" ? [ui.button("attachment-confirm", "添加记忆附件", confirm)] : [])]))]);
}
