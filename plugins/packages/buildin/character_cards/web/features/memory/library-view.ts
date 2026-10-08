import type { Snapshot } from "../../../src/model";
import { button, escapeHtml, icon, iconButton, notice, switchRow } from "../../shared/ui/html";
/** Keeps shared library management inside the character plugin's settings dialog. */
export function globalMemoryBody(snapshot: Snapshot) {
  let stores = "";
  for (const store of snapshot.stores) {
    let count = 0;
    for (const card of snapshot.cards) if (card.memoryBindingMode === "SHARED" && card.sharedMemoryId === store.id) count++;
    stores += `<div class="entity"><span class="avatar">${icon("hub")}</span><div class="entity-copy"><div class="entity-title">${escapeHtml(store.name)}</div><div class="badges"><span class="badge">${count} 个角色绑定</span></div></div><div class="entity-actions">${iconButton("owner-graph", "记忆图谱", "graph", `data-owner="shared:${escapeHtml(store.id)}" data-name="${escapeHtml(store.name)}"`)}${iconButton("owner-user", "用户资料", "profile", `data-owner="shared:${escapeHtml(store.id)}" data-name="${escapeHtml(store.name)}"`)}${iconButton("edit-store", "编辑共享记忆库", "edit", `data-id="${escapeHtml(store.id)}"`)}${iconButton("delete-store", "删除共享记忆库", "trash", `data-id="${escapeHtml(store.id)}"`)}</div></div>`;
  }
  return `<section class="section"><div class="section-head">${icon("tune")}<h2>全局记忆设置</h2></div><div id="unwired-notice" class="muted">这两个全局开关尚未接入插件领域读写，本页不显示猜测状态。</div>${switchRow("向模型提供用户资料", "聊天时把当前用户资料写入提示词。状态未接入。", "userDescription", null, true)}${switchRow("自动更新记忆库", "允许 AI 从对话中整理信息并写入记忆库。状态未接入。", "autoMemory", null, true)}</section><section class="section"><div class="section-head">${icon("hub")}<h2>共享记忆库</h2>${button("create-store", "创建", "add", "", "small")}</div>${stores === "" ? '<div class="empty">当前还没有共享记忆库，创建后可被多个角色卡挂载共享。</div>' : stores}</section>${notice("每张角色的记忆内容位于角色编辑器的“记忆”页签。这里仅管理全局设置和共享库，不是另一个插件入口。")}`;
}
