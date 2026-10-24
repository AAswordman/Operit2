import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import type { GlobalMemoryPreferences, MemoryAutoSaveStatus, Request, RequestOutput } from "./model";
import { chatIdentity } from "./ui-contributions";
import { definition } from "./definition";
import { findChatBinding } from "./chat-storage";
import { parseChatSelection } from "./chat-bindings";

/** Calls the same serialized main-runtime service used by the memory editor and scheduler. */
function request<R extends Request>(input: R): Promise<RequestOutput<R>> {
  return ToolPkg.ipc.call<R, RequestOutput<R>>("character-memory.request", input, { targetRuntime: "main" });
}

/** Preserves the main branch's four exact summaries. */
export function memorySummary(preferences: GlobalMemoryPreferences): string {
  if (preferences.userDescriptionEnabled) return preferences.autoUpdateEnabled ? "用户资料 · 记忆库更新" : "用户资料";
  return preferences.autoUpdateEnabled ? "记忆库更新" : "关";
}

/** Preserves the main branch's queue text, including its threshold and error suffix. */
export function memoryQueueDescription(queue: MemoryAutoSaveStatus | null): string {
  if (queue === null) return "暂无队列状态。自动提取会定期检查当前记忆库。";
  return `待处理 ${queue.pendingCandidates} 条 · ${queue.pendingChats} 个聊天\n`
    + `处理中 ${queue.processingCandidates} · 失败 ${queue.failedCandidates}\n`
    + `下次检查：约 ${queue.minutesUntilNextRun} 分钟后\n\n`
    + "自动检查需要至少 5 个候选；不足时继续等待。手动更新不受此门槛限制。"
    + (queue.lastError === "" ? "" : "\n\n" + queue.lastError);
}

/** Reproduces the original memory group while keeping all state, rendering and operations in the plugin. */
export function renderMemoryMenu(ctx: ComposeDslContext): ComposeNode {
  const [chatId] = ctx.useState<string | null>("chatId", null);
  if (chatId !== null) chatIdentity(chatId, "memory menu chatId");
  const owner = ctx.useRef("memory-menu-chat", chatId);
  const [available, setAvailable] = ctx.useState<boolean | null>("memory-menu-available", null);
  const [preferences, setPreferences] = ctx.useState<GlobalMemoryPreferences | null>("memory-menu-preferences", null);
  const [queue, setQueue] = ctx.useState<MemoryAutoSaveStatus | null>("memory-menu-queue", null);
  const [expanded, setExpanded] = ctx.useState("memory-menu-expanded", false);
  const [busy, setBusy] = ctx.useState("memory-menu-busy", false);
  const liveBusy = ctx.useRef("memory-menu-busy-live", false);
  const started = ctx.useRef("memory-menu-started", false);
  const colors = ctx.MaterialTheme.colorScheme;
  /** Rejects actions retained after the host changes the chat owner. */
  function assertOwner(): void {
    const [current] = ctx.useState<string | null>("chatId", null);
    if (current !== owner.current) throw new Error("Memory menu chat owner changed");
  }
  assertOwner();
  /** Reads current queue status only when the menu or a user action requests it. */
  async function refreshQueue(): Promise<void> {
    assertOwner();
    if (chatId === null) return;
    const value = await request({ action: "readChatMemoryAutoSaveStatus", chatId });
    assertOwner(); setQueue(value);
  }
  /** Loads memory controls only for conversations with an explicitly selected character. */
  async function load(): Promise<void> {
    assertOwner(); if (started.current) return; started.current = true;
    if (chatId !== null) {
      const binding = await findChatBinding(chatId); assertOwner();
      const hasCharacter = binding !== null && parseChatSelection(binding.selection).kind === "card";
      setAvailable(hasCharacter);
      if (!hasCharacter) return;
    }
    const value = await request({ action: "readGlobalMemoryPreferences" }); assertOwner(); setPreferences(value);
    await refreshQueue();
  }
  /** Reads current status before presenting the user-requested explanation. */
  async function showInfo(): Promise<ToolPkg.JsonValue> {
    await refreshQueue(); assertOwner();
    const [current] = ctx.useState<MemoryAutoSaveStatus | null>("memory-menu-queue", null);
    return { type: "toolpkg.ui.present", routeId: `toolpkg:${definition.id}:ui:memory-status`, input: { description: memoryQueueDescription(current) } };
  }
  /** Writes the same global preference consumed by prompt composition or the automatic job scheduler. */
  async function toggle(key: keyof GlobalMemoryPreferences): Promise<void> {
    assertOwner();
    const [current] = ctx.useState<GlobalMemoryPreferences | null>("memory-menu-preferences", null);
    if (current === null) throw new Error("Memory preferences are not loaded");
    const value = await request({ action: "writeGlobalMemoryPreference", key, value: !current[key] });
    assertOwner(); setPreferences(value);
  }
  /** Runs real extraction once and sends the original completion or failure text to the generic snackbar host. */
  async function manual(): Promise<ToolPkg.JsonValue> {
    assertOwner(); if (liveBusy.current || chatId === null) return null;
    liveBusy.current = true; setBusy(true);
    try {
      await request({ action: "updateCurrentChatMemory", chatId }); assertOwner(); await refreshQueue();
      return { type: "toolpkg.ui.message", message: "记忆提取完成" };
    } catch (error) {
      assertOwner(); return { type: "toolpkg.ui.message", message: "记忆提取失败：" + String(error) };
    } finally { liveBusy.current = false; assertOwner(); setBusy(false); }
  }
  /** Returns every failed record to the pending queue and reports the actual requeued count. */
  async function retryFailed(): Promise<ToolPkg.JsonValue> {
    assertOwner(); if (liveBusy.current || chatId === null) return null;
    liveBusy.current = true; setBusy(true);
    try {
      const result = await request({ action: "retryFailedCandidates", chatId }); assertOwner(); await refreshQueue();
      return { type: "toolpkg.ui.message", message: "已重新排队 " + result.retried + " 条失败候选" };
    } catch (error) {
      assertOwner(); return { type: "toolpkg.ui.message", message: "重试失败候选失败：" + String(error) };
    } finally { liveBusy.current = false; assertOwner(); setBusy(false); }
  }
  /** Deletes every terminal failed record and reports the actual cleared count. */
  async function clearFailed(): Promise<ToolPkg.JsonValue> {
    assertOwner(); if (liveBusy.current || chatId === null) return null;
    liveBusy.current = true; setBusy(true);
    try {
      const result = await request({ action: "clearFailedCandidates", chatId }); assertOwner(); await refreshQueue();
      return { type: "toolpkg.ui.message", message: "已清除 " + result.cleared + " 条失败记录" };
    } catch (error) {
      assertOwner(); return { type: "toolpkg.ui.message", message: "清除失败记录失败：" + String(error) };
    } finally { liveBusy.current = false; assertOwner(); setBusy(false); }
  }
  /** Creates the compact icon shared by memory menu rows. */
  function icon(name: string, color = colors.onSurfaceVariant): ComposeNode { return ctx.UI.Icon({ name, size: 16, tint: color }); }
  /** Reproduces the 32-pixel rows and 40-by-28 compact switches without a separate card. */
  function switchRow(key: keyof GlobalMemoryPreferences, name: string, title: string): ComposeNode {
    if (preferences === null) throw new Error("Memory preferences are not loaded");
    const checked = preferences[key];
    return ctx.UI.Row({ key: "memory-" + key, fillMaxWidth: true, paddingHorizontal: 12, verticalAlignment: "center",
      modifier: ctx.Modifier.heightIn({ minHeight: 32 }), onClick: () => toggle(key) }, [
      icon(name, checked ? colors.primary : colors.onSurfaceVariant), ctx.UI.Spacer({ width: 12 }),
      ctx.UI.Text({ text: title, style: "bodySmall", color: colors.onSurface, maxLines: 1, overflow: "ellipsis", weight: 1 }),
      ctx.UI.Text({ text: checked ? "开" : "关", style: "bodySmall", color: colors.primary, fontWeight: "600" }),
      ctx.UI.Spacer({ width: 8 }), ctx.UI.Switch({ checked, width: 40, height: 28, onCheckedChange: () => toggle(key) }),
    ]);
  }
  if (available === false) return ctx.UI.Box({ key: "memory-menu-unavailable" });
  if (preferences === null) return ctx.UI.Box({ key: "memory-menu-loading", onLoad: load });
  const children: ComposeNode[] = [ctx.UI.Row({ key: "memory-section-header", fillMaxWidth: true, paddingHorizontal: 12, verticalAlignment: "center",
    modifier: ctx.Modifier.heightIn({ minHeight: 36 }), onClick: () => { assertOwner(); setExpanded(!expanded); } }, [
    icon("DataObjectOutlined", colors.onSurfaceVariant.copy({ alpha: 0.7 })), ctx.UI.Spacer({ width: 12 }),
    ctx.UI.Text({ text: "记忆", style: "bodySmall" }), ctx.UI.Spacer({ width: 8 }),
    ctx.UI.Text({ text: memorySummary(preferences), style: "bodySmall", color: colors.primary, fontWeight: "600", textAlign: "end", maxLines: 1, overflow: "ellipsis", weight: 1 }),
    ctx.UI.IconButton({ key: "memory-info", tooltip: "记忆状态说明", width: 24, height: 28, contentPadding: { horizontal: 0, vertical: 0 },
      onClick: showInfo,
      content: ctx.UI.Icon({ name: "InfoOutline", size: 16, tint: colors.onSurfaceVariant.copy({ alpha: 0.7 }) }) }),
    ctx.UI.Spacer({ width: 6 }), ctx.UI.Icon({ name: expanded ? "KeyboardArrowUp" : "KeyboardArrowDown", size: 20, tint: colors.onSurfaceVariant }),
  ])];
  if (expanded) {
    const sectionChildren: ComposeNode[] = [
      switchRow("userDescriptionEnabled", "AssignmentIndOutlined", "提供用户资料"),
      switchRow("autoUpdateEnabled", preferences.autoUpdateEnabled ? "Save" : "SaveOutlined", "自动更新记忆库"),
      ctx.UI.Row({ key: "memory-manual", fillMaxWidth: true, paddingHorizontal: 12, verticalAlignment: "center",
        modifier: ctx.Modifier.heightIn({ minHeight: 32 }), onClick: !busy && chatId !== null ? manual : undefined }, [
        icon("SaveOutlined"), ctx.UI.Spacer({ width: 12 }),
        ctx.UI.Text({ text: busy ? "正在更新记忆…" : "手动更新记忆", style: "bodySmall", maxLines: 2, overflow: "ellipsis", weight: 1,
          color: !busy && chatId !== null ? colors.onSurface : colors.onSurfaceVariant.copy({ alpha: 0.45 }) }),
      ]),
    ];
    /** Exposes recovery only while real failed records remain in the owner queue. */
    if (queue !== null && queue.failedCandidates > 0) {
      const enabled = !busy && chatId !== null;
      sectionChildren.push(
        ctx.UI.Row({ key: "memory-retry-failed", fillMaxWidth: true, paddingHorizontal: 12, verticalAlignment: "center",
          modifier: ctx.Modifier.heightIn({ minHeight: 32 }), onClick: enabled ? retryFailed : undefined }, [
          icon("Refresh"), ctx.UI.Spacer({ width: 12 }),
          ctx.UI.Text({ text: "重试失败候选（" + queue.failedCandidates + "）", style: "bodySmall", maxLines: 1, overflow: "ellipsis", weight: 1,
            color: enabled ? colors.onSurface : colors.onSurfaceVariant.copy({ alpha: 0.45 }) }),
        ]),
        ctx.UI.Row({ key: "memory-clear-failed", fillMaxWidth: true, paddingHorizontal: 12, verticalAlignment: "center",
          modifier: ctx.Modifier.heightIn({ minHeight: 32 }), onClick: enabled ? clearFailed : undefined }, [
          icon("DeleteOutline"), ctx.UI.Spacer({ width: 12 }),
          ctx.UI.Text({ text: "清除失败记录（" + queue.failedCandidates + "）", style: "bodySmall", maxLines: 1, overflow: "ellipsis", weight: 1,
            color: enabled ? colors.onSurface : colors.onSurfaceVariant.copy({ alpha: 0.45 }) }),
        ]),
      );
    }
    children.push(ctx.UI.Column({ key: "memory-section-children", modifier: ctx.Modifier.padding(12, 0, 8, 4) }, sectionChildren));
  }
  return ctx.UI.Column({ key: "memory-section", fillMaxWidth: true }, children);
}
