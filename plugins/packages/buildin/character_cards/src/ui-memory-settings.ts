import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import type { MemorySettings, MemorySearchConfig, MemoryChatListing, MemoryAutoSaveStatus, MemoryRebuildProgress, MemorySearchDebug } from "./model";
import type { MemoryTarget } from "./ui-memory-workspace";
import { assertMemorySettings, assertSearchConfig } from "./validation";
import { nativeUi, uiDate, uiNumber, uiRequest, uiState } from "./ui-controls";

interface SettingsState {
  settings: MemorySettings | null; config: MemorySearchConfig | null; listing: MemoryChatListing | null;
  queue: MemoryAutoSaveStatus | null; progress: MemoryRebuildProgress | null;
  tab: string; busy: boolean; error: string; statusError: string; notice: string;
  numbers: Record<string, string>; query: string; threshold: string; result: MemorySearchDebug | null;
  selected: string[]; window: string; from: string; to: string; clearFailed: boolean;
}

/** Edits owner-scoped settings and controls durable jobs without a browser form. */
export function renderMemorySettings(ctx: ComposeDslContext, target: MemoryTarget, close: () => void, key: string): ComposeNode {
  const { state, live, update } = uiState<SettingsState>(ctx, key, { settings: null, config: null, listing: null, queue: null, progress: null, tab: "automatic", busy: false, error: "", statusError: "", notice: "", numbers: {}, query: "", threshold: "0", result: null, selected: [], window: "32", from: "", to: "", clearFailed: false });
  const ui = nativeUi(ctx, !state.busy), ownerKey = target.ownerKey;
  const alive = ctx.useRef(key + ":alive", true);
  /** Reads status without replacing active settings drafts or stealing field focus. */
  async function status(): Promise<void> {
    const [queue, progress] = await Promise.all([uiRequest({ action: "readMemoryAutoSaveStatus", ownerKey }), uiRequest({ action: "readMemoryRebuildProgress", ownerKey })]);
    if (alive.current) update({ queue, progress, statusError: "" });
  }
  /** Serializes mutations while retaining explicit errors and all unsaved inputs. */
  async function act(operation: () => Promise<void>): Promise<void> {
    if (live.current.busy || !alive.current) return;
    update({ busy: true, error: "", notice: "" });
    try { await operation(); } catch (error) { if (alive.current) update({ error: String(error) }); }
    finally { if (alive.current) update({ busy: false }); }
  }
  /** Loads complete settings before enabling saves; read failures never create defaults. */
  async function load(): Promise<void> {
    await act(async () => {
      const [settings, config, listing, queue, progress] = await Promise.all([
        uiRequest({ action: "readMemorySettings", ownerKey }), uiRequest({ action: "readMemorySearchConfig", ownerKey }), uiRequest({ action: "listMemoryChats", ownerKey }), uiRequest({ action: "readMemoryAutoSaveStatus", ownerKey }), uiRequest({ action: "readMemoryRebuildProgress", ownerKey }),
      ]);
      assertMemorySettings(settings); assertSearchConfig(config);
      if (!alive.current) return;
      update({ settings, config, listing, queue, progress, numbers: { interval: String(settings.autoSaveIntervalMinutes), keywordWeight: String(config.keywordWeight), tagWeight: String(config.tagWeight), vectorWeight: String(config.vectorWeight), edgeWeight: String(config.edgeWeight) } });
    });
  }
  ctx.useEffect(key + ":poll", () => {
    alive.current = true;
    let stopped = false, timer: ReturnType<typeof setTimeout>;
    /** Polls only while this exact settings page remains composed. */
    async function tick(): Promise<void> {
      try { if (!stopped && !live.current.busy && live.current.settings !== null) await status(); }
      catch (error) { if (!stopped) update({ statusError: "状态读取失败：" + String(error) }); }
      finally { if (!stopped) timer = setTimeout(tick, 3000); }
    }
    timer = setTimeout(tick, 3000);
    return () => { stopped = true; alive.current = false; clearTimeout(timer); };
  }, []);
  /** Patches only an authoritative settings draft that has finished loading. */
  function settingsChange(patch: Partial<MemorySettings>): void { if (live.current.settings !== null && !live.current.busy) update({ settings: { ...live.current.settings, ...patch } }); }
  /** Preserves numeric text until a validated search or save is submitted. */
  function numberChange(name: string, value: string): void { update({ numbers: { ...live.current.numbers, [name]: value } }); }
  /** Validates all configured search weights using the existing domain contract. */
  function configDraft(): MemorySearchConfig {
    if (live.current.config === null) throw new Error("检索设置尚未读取成功");
    const draft = { ...live.current.config };
    for (const name of ["keywordWeight", "tagWeight", "vectorWeight", "edgeWeight"] as const) draft[name] = uiNumber(live.current.numbers[name], name, 0);
    assertSearchConfig(draft); return draft;
  }
  /** Saves settings and search configuration through the existing atomic owner operation. */
  async function save(): Promise<void> {
    await act(async () => {
      if (live.current.settings === null) throw new Error("记忆设置尚未读取成功");
      const settings = { ...live.current.settings, autoSaveIntervalMinutes: uiNumber(live.current.numbers.interval, "检查间隔", 1, 30, true) }, config = configDraft();
      assertMemorySettings(settings);
      const result = await uiRequest({ action: "writeMemoryOwnerSettings", ownerKey, settings, config });
      if (alive.current) update({ settings: result.settings, config: result.config, notice: "设置已保存" });
      await status();
    });
  }
  const children: ComposeNode[] = [ui.text("所属记忆库：" + ownerKey)];
  if (state.error !== "") children.push(ctx.UI.Text({ key: "memory-settings-error", text: state.error, color: ctx.MaterialTheme.colorScheme.error }));
  if (state.statusError !== "") children.push(ui.text(state.statusError));
  if (state.notice !== "") children.push(ui.text(state.notice));
  const settings = state.settings, config = state.config;
  if (settings === null || config === null || state.listing === null) children.push(ui.text(state.busy ? "正在读取记忆设置…" : "记忆设置尚未读取成功"), ui.button("memory-settings-retry", "重试", load));
  else {

    if (state.listing.missingChatIds.length > 0) children.push(ui.text(`检测到 ${state.listing.missingChatIds.length} 条已失效的会话绑定，自动提取与历史重建会跳过它们。`), ui.button("memory-prune-bindings", "清理失效绑定", () => act(async () => { const result = await uiRequest({ action: "pruneMissingChatBindings", ownerKey }); const listing = await uiRequest({ action: "listMemoryChats", ownerKey }); if (alive.current) update({ listing, notice: `已清理 ${result.removedChatIds.length} 条会话绑定和 ${result.removedCandidateIds.length} 条提取候选` }); })));
    if (state.tab === "automatic") {
      const queue = state.queue;
      if (queue !== null) {
        children.push(ui.text(`待处理 ${queue.pendingCandidates} 条 / ${queue.pendingChats} 个聊天 · 处理中 ${queue.processingCandidates} · 失败 ${queue.failedCandidates}`), ui.text(queue.nextRunAtMs === 0 ? "尚未安排下次检查" : `下次检查：约 ${queue.minutesUntilNextRun} 分钟后`));
        if (queue.lastError !== "") children.push(ui.text("提取失败：" + queue.lastError));
        if (queue.failedCandidates > 0) children.push(ui.actions([
          ui.button("memory-retry-failed", "重试失败候选", () => act(async () => { const result = await uiRequest({ action: "retryFailedCandidates", ownerKey }); if (alive.current) update({ notice: `已重新排队 ${result.retried} 条失败候选` }); await status(); })),
          ui.button("memory-clear-failed", state.clearFailed ? "确认清除失败记录" : "清除失败记录", () => state.clearFailed ? act(async () => { const result = await uiRequest({ action: "clearFailedCandidates", ownerKey }); if (alive.current) update({ clearFailed: false, notice: `已清除 ${result.cleared} 条失败记录` }); await status(); }) : update({ clearFailed: true })),
          ...(state.clearFailed ? [ui.button("memory-clear-failed-cancel", "取消清除", () => update({ clearFailed: false }))] : []),
        ]));
      }
      children.push(ui.text("至少积累 5 个候选后自动提取；清除失败记录不会删除已有记忆。"), ui.field("memory-interval", "检查间隔（1–30 分钟）", state.numbers.interval, value => numberChange("interval", value)), ui.field("memory-extraction-rules", "记忆提取自定义规则", settings.memoryExtractionCustomRules, memoryExtractionCustomRules => settingsChange({ memoryExtractionCustomRules }), 5), ui.toggle("memory-profile-auto", "自动更新 USER.md", settings.profileAutoUpdateEnabled, profileAutoUpdateEnabled => settingsChange({ profileAutoUpdateEnabled })), ui.toggle("memory-profile-locked", "锁定 USER.md，禁止模型覆盖", settings.profileAutoUpdateLocked, profileAutoUpdateLocked => settingsChange({ profileAutoUpdateLocked })));
    } else if (state.tab === "search") {

      for (const [name, label] of [["keywordWeight", "关键词权重"], ["tagWeight", "标签权重"], ["vectorWeight", "向量权重"], ["edgeWeight", "关系权重"]] as const) children.push(ui.field("memory-" + name, label, state.numbers[name], value => numberChange(name, value)));
      children.push(ui.toggle("memory-embedding", "云端 Embedding", settings.cloudEmbeddingEnabled, cloudEmbeddingEnabled => settingsChange({ cloudEmbeddingEnabled })));
      if (settings.cloudEmbeddingEnabled) children.push(ui.text("搜索文本与记忆文本会发送至指定服务。修改服务地址、模型或密钥后，请先保存再重建。"), ui.field("memory-embedding-endpoint", "Embedding 完整请求地址", settings.cloudEmbeddingEndpoint, cloudEmbeddingEndpoint => settingsChange({ cloudEmbeddingEndpoint })), ui.field("memory-embedding-model", "Embedding 模型", settings.cloudEmbeddingModel, cloudEmbeddingModel => settingsChange({ cloudEmbeddingModel })), ctx.UI.TextField({ key: "memory-embedding-key", label: "API Key", value: settings.cloudEmbeddingApiKey, enabled: !state.busy, isPassword: true, fillMaxWidth: true, onValueChange: cloudEmbeddingApiKey => settingsChange({ cloudEmbeddingApiKey }) }), ui.button("memory-embedding-rebuild", "使用已保存设置重建 Embedding", () => act(async () => { const count = await uiRequest({ action: "rebuildMemoryEmbeddings", ownerKey }); if (alive.current) update({ notice: `已重建 ${count} 条记忆的 Embedding` }); })));
      children.push(ui.text("检索调试", true), ui.field("memory-debug-query", "查询", state.query, query => update({ query })), ui.field("memory-debug-threshold", "相关性阈值（0–1）", state.threshold, threshold => update({ threshold })), ui.text("使用当前权重执行真实检索，不保存权重；向量检索使用已保存的服务配置。"), ui.button("memory-debug-search", "使用当前权重检索", () => act(async () => { const result = await uiRequest({ action: "searchMemoryDebug", ownerKey, query: live.current.query, config: configDraft(), relevanceThreshold: uiNumber(live.current.threshold, "相关性阈值", 0, 1) }); if (alive.current) update({ result }); })));
      if (state.result !== null) { children.push(ui.text(`范围 ${state.result.memoriesInScopeCount} 条 · 命中 ${state.result.candidates.filter(hit => hit.passedThreshold).length} 条`)); for (const hit of state.result.candidates) children.push(ui.card("memory-hit:" + hit.memory.uuid, hit.memory.title, `总分 ${hit.score.toFixed(4)} · ${hit.passedThreshold ? "命中" : "未过阈值"}\n关键词 ${hit.keywordScore.toFixed(4)} · 标签 ${hit.tagScore.toFixed(4)} · 向量 ${hit.vectorScore.toFixed(4)} · 关系 ${hit.edgeScore.toFixed(4)}`, [])); }
    } else {
      const progress = state.progress, running = progress !== null && (progress.status === "preparing" || progress.status === "running");
      children.push(ui.text("从聊天历史重建记忆：追加或更新，保留现有记忆。关闭此页不会终止后台任务。"), ui.field("memory-rebuild-window", "窗口消息数", state.window, window => update({ window })), ui.field("memory-rebuild-from", "开始日期（YYYY-MM-DD，可留空）", state.from, from => update({ from })), ui.field("memory-rebuild-to", "结束日期（YYYY-MM-DD，可留空）", state.to, to => update({ to })));
      for (const chat of state.listing.chats) children.push(ctx.UI.Row({ key: "memory-rebuild-chat:" + chat.id, fillMaxWidth: true, spacing: 8, verticalAlignment: "center" }, [ctx.UI.Checkbox({ key: "memory-rebuild-check:" + chat.id, checked: state.selected.includes(chat.id), enabled: !state.busy && !running, onCheckedChange: checked => update({ selected: checked ? [...new Set([...live.current.selected, chat.id])] : live.current.selected.filter(id => id !== chat.id) }) }), ctx.UI.Column({ weight: 1, spacing: 4 }, [ui.text(chat.title), ui.text(`${chat.messageCount} 条消息`)])]));
      if (state.listing.chats.length === 0) children.push(ui.text("此记忆库暂无绑定聊天"));
      children.push(ui.actions([ui.button("memory-rebuild-start", "开始重建", () => act(async () => { const value = live.current, fromInclusive = uiDate(value.from, false), toInclusive = uiDate(value.to, true); if (fromInclusive !== null && toInclusive !== null && fromInclusive > toInclusive) throw new Error("开始日期不能晚于结束日期"); const result = await uiRequest({ action: "startMemoryRebuild", ownerKey, rebuild: { chatIds: [...value.selected], windowMessageCount: uiNumber(value.window, "窗口消息数", 1, Infinity, true), fromInclusive, toInclusive } }); if (alive.current) update({ progress: result, notice: "后台重建任务已提交" }); }), !running && state.selected.length > 0), ui.button("memory-rebuild-cancel", "取消重建", () => act(async () => { const progress = await uiRequest({ action: "cancelMemoryRebuild", ownerKey }); if (alive.current) update({ progress }); }), running), ui.button("memory-status-refresh", "刷新状态", () => act(status))]));
      if (progress !== null) { const labels: Record<string, string> = { idle: "未开始", preparing: "准备中", running: "重建中", completed: "已完成", cancelled: "已取消", failed: "失败" }; children.push(ui.text(`${labels[progress.status] ?? progress.status} · 聊天 ${progress.completedChats}/${progress.totalChats} · 窗口 ${progress.completedWindows}/${progress.totalWindows} · 失败 ${progress.failedWindows}`), ui.text(`已处理源消息 ${progress.processedSourceMessages}/${progress.totalSourceMessages} · ${progress.currentChatTitle}`)); if (progress.lastError !== "") children.push(ui.text(progress.lastError)); }
    }

  }
  const tabs = [{ id: "automatic", label: "自动提取" }, { id: "search", label: "检索" }, { id: "rebuild", label: "历史重建" }];
  return ui.dialog("memory-settings-root", target.name + " · 记忆设置", ctx.UI.Column({ fillMaxSize: true }, [
    ctx.UI.PrimaryTabRow({ key: "memory-settings-tabs", fillMaxWidth: true, selectedTabIndex: tabs.findIndex(tab => tab.id === state.tab), tabs: tabs.map(tab => ctx.UI.Tab({ key: "memory-settings-tabs:" + tab.id, selected: state.tab === tab.id, enabled: !state.busy, onClick: () => update({ tab: tab.id }) }, ctx.UI.Text({ text: tab.label, style: "titleSmall" }))) }),
    ui.list("memory-settings-body", children),
  ]), [ui.button("memory-settings-back", "关闭", close), ...(settings !== null && config !== null && state.listing !== null ? [ui.primary("memory-settings-save", "保存", save)] : [])], close, "large", load);
}
