import { assertMemorySettings, assertSearchConfig } from "../../../src/validation";
import type { EditorContext } from "../../bridge/context";
import type { OwnerSettingsDialog } from "../../bridge/contracts";
import { dataValue } from "../../shared/dom";
import { memoryRebuilding, updateMemoryStatus } from "./controls-view";

/** Converts date-only controls to the same inclusive local-day range used by main's Rust/Dart UI. */
export function memoryDateBoundary(value: string, end: boolean): number | null {
  if (value === "") return null;
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) throw new Error("重建日期无效");
  const [year, month, day] = value.split("-").map(Number), date = new Date(year, month - 1, day);
  if (date.getFullYear() !== year || date.getMonth() !== month - 1 || date.getDate() !== day || date.getTime() < 0) throw new Error("重建日期无效");
  if (!end) return date.getTime();
  date.setDate(date.getDate() + 1); return date.getTime() - 1;
}

/** Connects owner settings and jobs to the same real main-runtime service as commands and chat hooks. */
export function createMemoryControlsFeature(context: EditorContext) {
  const { state, openUser, pushDialog, topDialog, request, renderDialogs, toast } = context;
  /** Ignores late responses to a closed dialog instead of reviving another owner's controls. */
  function exists(dialog: OwnerSettingsDialog): boolean { return state.dialogs.includes(dialog); }
  /** Reads genuine queue/progress values without marking drafts busy or rewriting them. */
  async function poll(dialog: OwnerSettingsDialog): Promise<void> {
    const [queue, progress] = await Promise.all([
      request({ action: "readMemoryAutoSaveStatus", ownerKey: dialog.ownerKey }, { background: true }),
      request({ action: "readMemoryRebuildProgress", ownerKey: dialog.ownerKey }, { background: true }),
    ]);
    if (!exists(dialog)) return;
    dialog.queue = queue; dialog.progress = progress; dialog.statusError = "";
    if (topDialog() === dialog) updateMemoryStatus(dialog, context.dialogsRoot);
  }
  /** Polls only while this modal is owned by the stack; closing never cancels the durable job. */
  function startPolling(dialog: OwnerSettingsDialog): void {
    dialog.dispose?.();
    let stopped = false, timer: ReturnType<typeof setTimeout>;
    dialog.dispose = () => { stopped = true; clearTimeout(timer); };
    /** Waits for the previous read and pauses during foreground or nested editing. */
    async function tick(): Promise<void> {
      if (stopped || !exists(dialog)) return;
      try {
        if (!state.busy && !dialog.busy && topDialog() === dialog) await poll(dialog);
      } catch (error) {
        if (exists(dialog)) { dialog.statusError = `状态读取失败：${String(error)}`; if (topDialog() === dialog) updateMemoryStatus(dialog, context.dialogsRoot); }
      } finally { if (!stopped && exists(dialog)) timer = setTimeout(tick, 3000); }
    }
    timer = setTimeout(tick, 3000);
  }
  /** Loads every tab from the real selected owner, never from an active-card fallback or sample defaults. */
  async function load(dialog: OwnerSettingsDialog): Promise<void> {
    dialog.loading = true; dialog.error = ""; renderDialogs();
    try {
      const [settings, config, chats, queue, progress] = await Promise.all([
        request({ action: "readMemorySettings", ownerKey: dialog.ownerKey }, { background: true }),
        request({ action: "readMemorySearchConfig", ownerKey: dialog.ownerKey }, { background: true }),
        request({ action: "listMemoryChats", ownerKey: dialog.ownerKey }, { background: true }),
        request({ action: "readMemoryAutoSaveStatus", ownerKey: dialog.ownerKey }, { background: true }),
        request({ action: "readMemoryRebuildProgress", ownerKey: dialog.ownerKey }, { background: true }),
      ]);
      assertMemorySettings(settings); assertSearchConfig(config);
      if (!exists(dialog)) return;
      dialog.settings = settings; dialog.config = config; dialog.chats = chats; dialog.queue = queue; dialog.progress = progress;
      dialog.statusError = ""; startPolling(dialog);
    } finally { if (exists(dialog)) { dialog.loading = false; renderDialogs(); } }
  }
  /** Applies a real request and refreshes only acknowledged job state. */
  async function handleAction(action: string, element: HTMLElement | SVGElement): Promise<void> {
    if (action === "owner-user") { await openUser(dataValue(element, "owner"), dataValue(element, "name")); return; }
    if (action === "owner-settings") {
      const dialog = pushDialog({ type: "owner-settings", ownerKey: dataValue(element, "owner"), name: dataValue(element, "name"), tab: 0,
        loading: true, settings: null, config: null, chats: null, queue: null, progress: null, statusError: "",
        searchQuery: "", searchThreshold: 0, searchResult: null, selectedChatIds: [], windowMessageCount: 32, from: "", to: "" });
      await load(dialog); return;
    }
    const dialog = topDialog("owner-settings"), ownerKey = dialog.ownerKey;
    switch (action) {
      case "search-memory-debug": {
        if (dialog.config === null) throw new Error("检索配置尚未读取成功");
        for (const input of context.dialogsRoot.querySelectorAll<HTMLInputElement>("input[type=number]")) if (!input.reportValidity()) throw new Error("请检查数字设置的范围与格式");
        assertSearchConfig(dialog.config);
        dialog.searchResult = null;
        dialog.searchResult = await request({ action: "searchMemoryDebug", ownerKey, query: dialog.searchQuery, config: dialog.config, relevanceThreshold: dialog.searchThreshold });
        renderDialogs();
        const results = context.dialogsRoot.querySelector<HTMLElement>("[data-search-results]");
        results?.scrollIntoView({ block: "nearest" });
        const body = results?.closest(".dialog-body");
        if (body !== null && body !== undefined) dialog.scroll = body.scrollTop;
        return;
      }
      case "reload-memory-settings": await load(dialog); return;
      case "refresh-memory-status": await poll(dialog); return;
      case "save-memory-settings": {
        if (dialog.settings === null || dialog.config === null || dialog.loading) throw new Error("记忆设置尚未读取成功");
        for (const input of context.dialogsRoot.querySelectorAll<HTMLInputElement>("input[type=number]")) if (!input.reportValidity()) throw new Error("请检查数字设置的范围与格式");
        assertMemorySettings(dialog.settings); assertSearchConfig(dialog.config);
        const saved = await request({ action: "writeMemoryOwnerSettings", ownerKey, settings: dialog.settings, config: dialog.config });
        dialog.settings = saved.settings; dialog.config = saved.config;
        toast("记忆设置已保存"); await poll(dialog); return;
      }
      case "rebuild-memory-embeddings": {
        const count = await request({ action: "rebuildMemoryEmbeddings", ownerKey });
        toast(`已重建 ${count} 条记忆的 Embedding`); return;
      }
      case "start-memory-rebuild": {
        if (memoryRebuilding(dialog)) throw new Error("记忆重建已在运行");
        const fromInclusive = memoryDateBoundary(dialog.from, false), toInclusive = memoryDateBoundary(dialog.to, true);
        if (fromInclusive !== null && toInclusive !== null && fromInclusive > toInclusive) throw new Error("开始时间不能晚于结束时间");
        dialog.progress = await request({ action: "startMemoryRebuild", ownerKey, rebuild: { chatIds: [...dialog.selectedChatIds], windowMessageCount: dialog.windowMessageCount, fromInclusive, toInclusive } });
        toast("后台重建任务已提交"); return;
      }
      case "cancel-memory-rebuild": dialog.progress = await request({ action: "cancelMemoryRebuild", ownerKey }); toast("重建已取消"); return;
      default: throw new Error(`Unknown memory controls action: ${action}`);
    }
  }
  return { names: ["search-memory-debug", "owner-user", "owner-settings", "reload-memory-settings", "refresh-memory-status", "save-memory-settings", "rebuild-memory-embeddings", "start-memory-rebuild", "cancel-memory-rebuild"], handleAction };
}
