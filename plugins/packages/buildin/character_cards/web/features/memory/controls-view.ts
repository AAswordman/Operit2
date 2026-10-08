import type { OwnerSettingsDialog } from "../../bridge/contracts";
import { button, escapeHtml, field, notice, selectField, switchRow } from "../../shared/ui/html";
/** Reproduces the original three memory-settings tabs while marking unavailable APIs. */
export function ownerSettingsBody(dialog: OwnerSettingsDialog): string {
  const unavailable = "未接入读取接口";
  const header = `<div class="muted">所属记忆库：${escapeHtml(dialog.ownerKey)}</div>${notice("插件领域设置与后台任务尚未接线。控件暂为只读，不显示模拟状态或启动本地假任务。")}`;
  switch (dialog.tab) {
    case 0: return header + field("检查间隔（分钟）", "interval", "", 0, true, unavailable) + field("记忆提取自定义规则", "extractionRules", "", 5, true, unavailable) + switchRow("自动更新 USER.md", unavailable, "profileAuto", null, true) + switchRow("锁定 USER.md", unavailable, "profileLocked", null, true);
    case 1: return header + selectField("评分模式", "scoreMode", "", [], true) + `<div class="unwired">${field("关键词权重", "keywordWeight", "", 0, true, unavailable)}${field("标签权重", "tagWeight", "", 0, true, unavailable)}${field("向量权重", "vectorWeight", "", 0, true, unavailable)}${field("关系权重", "edgeWeight", "", 0, true, unavailable)}</div>` + switchRow("云端 Embedding", unavailable, "embedding", null, true) + field("Embedding 完整请求地址", "endpoint", "", 0, true) + field("Embedding 模型", "embeddingModel", "", 0, true) + field("API Key", "apiKey", "", 0, true) + button("unwired", "重建 Embedding", "refresh", "disabled", "tonal");
    case 2: return header + field("窗口消息数", "windowMessageCount", "", 0, true, unavailable) + `<div class="settings-grid">${field("开始时间", "from", "", 0, true)}${field("结束时间", "to", "", 0, true)}</div><div class="memory-entry"><h3>选择聊天</h3><p class="muted">关联聊天与重建进度接口尚未接入。</p></div>` + button("unwired", "开始重建", "refresh", "disabled", "filled");
    default: throw new Error("Invalid memory settings tab");
  }
}
