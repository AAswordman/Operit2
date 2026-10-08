import type { DialogSpec, MemoryEditDialog } from "../../bridge/contracts";
import { button, escapeHtml, field } from "../../shared/ui/html";
/** Builds a confidence slider using the original memory editor's range and precision. */
function scoreField(label: string, key: string, value: number): string {
  return `<label class="score-field"><span>${escapeHtml(label)} <output>${Number(value).toFixed(2)}</output></span><input type="range" data-field="${key}" aria-label="${escapeHtml(label)}" min="0" max="1" step="0.01" value="${value}"></label>`;
}
/** Renders the memory-edit modal using its finite typed draft. */
export function memoryEditorSpec(dialog: MemoryEditDialog): DialogSpec {
  return { title: dialog.originalTitle === null ? "创建记忆" : "编辑记忆", style: "large", tabs: [], body: field("标题 *", "title", dialog.draft.title) + field("内容", "content", dialog.draft.content, 12) + field("内容类型", "contentType", dialog.draft.contentType) + field("来源", "source", dialog.draft.source) + scoreField("可信度", "credibility", dialog.draft.credibility) + scoreField("重要性", "importance", dialog.draft.importance) + field("文件夹", "folderPath", dialog.draft.folderPath) + field("标签（逗号分隔）", "tags", dialog.draft.tags), footer: button("close-dialog", "取消", "") + button("save-memory", "保存", "", "", "filled") };
}
