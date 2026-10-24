import type { OwnerSettingsDialog } from "../../bridge/contracts";
import { button, escapeHtml, field, notice, switchRow } from "../../shared/ui/html";

/** Renders actual production search scores, never browser-computed sample results. */
function searchResults(dialog: OwnerSettingsDialog): string {
  const result = dialog.searchResult;
  if (result === null) return "";
  const rows = result.candidates.map(hit => `<tr><td>${escapeHtml(hit.memory.title)}</td><td>${hit.score.toFixed(4)}</td><td>${hit.keywordScore.toFixed(4)}</td><td>${hit.tagScore.toFixed(4)}</td><td>${hit.vectorScore.toFixed(4)}</td><td>${hit.edgeScore.toFixed(4)}</td><td>${hit.passedThreshold ? "命中" : "未过阈值"}</td></tr>`).join("");
  return `<section data-search-results><h3>查询：${escapeHtml(result.query)}</h3><p class="muted">范围 ${result.memoriesInScopeCount} 条 · 命中 ${result.candidates.filter(hit => hit.passedThreshold).length} 条 · 阈值 ${result.relevanceThreshold}</p><div class="memory-search-scores" tabindex="0" role="region" aria-label="检索真实评分"><table class="memory-search-table"><thead><tr><th>记忆</th><th>总分</th><th>关键词</th><th>标签</th><th>向量</th><th>关系</th><th>结果</th></tr></thead><tbody>${rows}</tbody></table></div>${rows === "" ? '<p class="muted">当前记忆库没有候选记录</p>' : ""}</section>`;
}

/** Reads only the durable provider job status, never an optimistic local running flag. */
export function memoryRebuilding(dialog: OwnerSettingsDialog): boolean {
  return dialog.progress !== null && (dialog.progress.status === "preparing" || dialog.progress.status === "running");
}

/** Displays actual candidate counts, the persisted next-check timestamp and genuine failure recovery actions. */
export function memoryQueueStatus(dialog: OwnerSettingsDialog): string {
  const queue = dialog.queue;
  if (queue === null) return '<p class="muted">正在读取自动提取队列…</p>';
  const recovery = queue.failedCandidates === 0 ? "" : `<div class="actions">${dialog.pendingClearFailed
    ? button("confirm-clear-failed-candidates", "确认清除失败记录", "trash", "", "filled") + button("cancel-clear-failed-candidates", "取消", "close")
    : button("retry-failed-candidates", "重试失败候选", "refresh", "", "tonal") + button("clear-failed-candidates", "清除失败记录", "trash", "", "tonal")}</div>${dialog.pendingClearFailed ? '<p class="muted">清除只删除失败候选记录，不影响已写入的记忆。</p>' : ""}`;
  return `<div class="muted">待处理 ${queue.pendingCandidates} 条 / ${queue.pendingChats} 个聊天 · 处理中 ${queue.processingCandidates} · 失败 ${queue.failedCandidates}</div><div class="muted">${queue.nextRunAtMs === 0 ? "尚未安排下次检查" : `下次检查：约 ${queue.minutesUntilNextRun} 分钟后`}</div>${queue.lastError === "" ? "" : `<p class="error-banner">${escapeHtml(queue.lastError)}</p>`}${recovery}`;
}

/** Projects genuine rebuild counters and failures without advancing them in the browser. */
export function memoryRebuildStatus(dialog: OwnerSettingsDialog): string {
  const progress = dialog.progress;
  if (progress === null) return '<p class="muted">正在读取重建进度…</p>';
  const names: Record<string, string> = { idle: "未开始", preparing: "准备中", running: "重建中", completed: "已完成", cancelled: "已取消", failed: "失败" };
  return `<p>${escapeHtml(names[progress.status] ?? progress.status)} · 聊天 ${progress.completedChats}/${progress.totalChats} · 窗口 ${progress.completedWindows}/${progress.totalWindows} · 失败 ${progress.failedWindows}</p><p class="muted">已处理源消息 ${progress.processedSourceMessages}/${progress.totalSourceMessages}${progress.currentChatTitle === "" ? "" : ` · ${escapeHtml(progress.currentChatTitle)}`}</p>${memoryRebuilding(dialog) ? `<progress aria-label="重建进度" max="${Math.max(1, progress.totalWindows)}" value="${progress.completedWindows}"></progress><p class="muted">关闭此窗口不会终止后台重建；取消会在当前窗口结束后生效。</p>` : ""}${progress.lastError === "" ? "" : `<p class="error-banner">${escapeHtml(progress.lastError)}</p>`}`;
}

/** Uses native numeric validation rather than coercing an empty draft to a configured value. */
function numericField(label: string, key: string, value: number, min: number, max?: number, step: number | "any" = 1): string {
  return field(label, key, value, 0, false, "", { type: "number", min, max, step, required: true });
}

/** Renders drafts loaded from the exact character/shared owner through main-runtime IPC. */
export function ownerSettingsBody(dialog: OwnerSettingsDialog): string {
  const header = `<div class="muted">所属记忆库：${escapeHtml(dialog.ownerKey)}</div>`;
  if (dialog.loading) return header + '<div class="loading" role="status">正在读取记忆设置…</div>';
  const settings = dialog.settings, config = dialog.config;
  if (settings === null || config === null || dialog.chats === null) return header + button("reload-memory-settings", "重试", "refresh");
  const statusError = `<div data-memory-status-error class="error-banner" role="alert" ${dialog.statusError === "" ? "hidden" : ""}>${escapeHtml(dialog.statusError)}</div>`;
  const missingBanner = dialog.missingChatIds.length === 0 ? "" : `<div class="error-banner" role="alert">检测到 ${dialog.missingChatIds.length} 条已失效的会话绑定（对应聊天已被删除），自动提取与历史重建会跳过它们。${button("prune-missing-chat-bindings", "清理失效绑定", "")}</div>`;
  switch (dialog.tab) {
    case 0: return header + statusError + missingBanner + `<div data-memory-queue>${memoryQueueStatus(dialog)}</div>` + notice("至少积累 5 个候选后自动提取；提取结果写入此记忆库。")
      + numericField("检查间隔（分钟）", "interval", settings.autoSaveIntervalMinutes, 1, 30)
      + field("记忆提取自定义规则", "extractionRules", settings.memoryExtractionCustomRules, 5)
      + switchRow("自动更新 USER.md", "提取记忆时更新此记忆库的用户资料。", "profileAuto", settings.profileAutoUpdateEnabled, false)
      + switchRow("锁定 USER.md", "禁止模型覆盖用户资料。", "profileLocked", settings.profileAutoUpdateLocked, false);
    case 1: return header + statusError + missingBanner
      + numericField("关键词权重", "keywordWeight", config.keywordWeight, 0, undefined, "any")
      + numericField("标签权重", "tagWeight", config.tagWeight, 0, undefined, "any")
      + numericField("向量权重", "vectorWeight", config.vectorWeight, 0, undefined, "any")
      + numericField("关系权重", "edgeWeight", config.edgeWeight, 0, undefined, "any")
      + switchRow("云端 Embedding", "启用后，搜索文本和记忆文本会发送至指定服务。", "embedding", settings.cloudEmbeddingEnabled, false)
      + (settings.cloudEmbeddingEnabled ? field("Embedding 完整请求地址", "endpoint", settings.cloudEmbeddingEndpoint)
        + field("Embedding 模型", "embeddingModel", settings.cloudEmbeddingModel)
        + field("API Key", "apiKey", settings.cloudEmbeddingApiKey, 0, false, "", { type: "password" })
        + notice("重建使用已保存的配置。修改服务地址、模型或密钥后，请先保存设置。")
        + button("rebuild-memory-embeddings", "重建 Embedding", "refresh", "", "tonal") : "")
      + '<h3>检索调试</h3>' + field("检索查询", "searchQuery", dialog.searchQuery)
      + numericField("相关性阈值", "searchThreshold", dialog.searchThreshold, 0, 1, "any")
      + notice("使用当前权重执行真实检索，不保存权重；向量检索使用已保存的 Embedding 服务配置。各通道显示对总分的实际贡献。")
      + button("search-memory-debug", "使用当前权重检索", "search", "", "tonal") + searchResults(dialog);
    case 2: {
      const rebuilding = memoryRebuilding(dialog);
      let chats = "";
      for (const chat of dialog.chats) chats += `<label class="checkbox-row"><input type="checkbox" data-memory-chat="${escapeHtml(chat.id)}" aria-label="${escapeHtml(chat.title)}" ${dialog.selectedChatIds.includes(chat.id) ? "checked" : ""} ${rebuilding ? "disabled" : ""}><span>${escapeHtml(chat.title)}<span class="muted"> · ${chat.messageCount} 条消息</span></span></label>`;
      return header + statusError + missingBanner + notice("从聊天历史重建记忆（追加／更新，不删除现有记忆）。")
        + numericField("窗口消息数", "windowMessageCount", dialog.windowMessageCount, 1)
        + `<div class="settings-grid">${field("开始时间", "from", dialog.from, 0, false, "包含所选日期", { type: "date" })}${field("结束时间", "to", dialog.to, 0, false, "包含所选日期", { type: "date" })}</div>`
        + '<h3>选择聊天</h3>' + (chats === "" ? '<p class="muted">此记忆库暂无绑定聊天</p>' : chats)
        + `<div class="actions">${button("start-memory-rebuild", "开始重建", "refresh", rebuilding || dialog.selectedChatIds.length === 0 ? "disabled" : "", "filled")}${button("cancel-memory-rebuild", "取消重建", "", rebuilding ? "" : "disabled", "tonal")}${button("refresh-memory-status", "刷新状态", "refresh")}</div>`
        + `<div data-memory-rebuild>${memoryRebuildStatus(dialog)}</div>`;
    }
    default: throw new Error("Invalid memory settings tab");
  }
}

/** Allows saving only authoritative loaded drafts; initial/read failures cannot overwrite defaults. */
export function ownerSettingsFooter(dialog: OwnerSettingsDialog): string {
  return button("close-dialog", "关闭", "") + button("save-memory-settings", "保存", "", dialog.loading || dialog.settings === null || dialog.config === null ? "disabled" : "", "filled");
}

/** Repaints only live status regions so background reads never replace fields or steal focus. */
export function updateMemoryStatus(dialog: OwnerSettingsDialog, root: HTMLElement): void {
  const queue = root.querySelector("[data-memory-queue]");
  if (queue !== null) queue.innerHTML = memoryQueueStatus(dialog);
  const rebuild = root.querySelector("[data-memory-rebuild]");
  if (rebuild !== null) rebuild.innerHTML = memoryRebuildStatus(dialog);
  const error = root.querySelector<HTMLElement>("[data-memory-status-error]");
  if (error !== null) { error.textContent = dialog.statusError; error.hidden = dialog.statusError === ""; }
  const start = root.querySelector<HTMLButtonElement>('[data-action="start-memory-rebuild"]');
  if (start !== null) start.disabled = dialog.busy || memoryRebuilding(dialog) || dialog.selectedChatIds.length === 0;
  const cancel = root.querySelector<HTMLButtonElement>('[data-action="cancel-memory-rebuild"]');
  if (cancel !== null) cancel.disabled = dialog.busy || !memoryRebuilding(dialog);
  for (const checkbox of root.querySelectorAll<HTMLInputElement>("[data-memory-chat]")) checkbox.disabled = dialog.busy || memoryRebuilding(dialog);
}
