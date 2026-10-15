import { ownerFor, isActive } from "../characters/data";
import type { Snapshot } from "../../../src/model";
import { button, escapeHtml, icon, iconButton, switchRow } from "../../shared/ui/html";
/** Uses click actions rather than speculative local preference state. */
function globalSwitch(title: string, description: string, key: "userDescriptionEnabled" | "autoUpdateEnabled", checked: boolean): string {
  return switchRow(title, description, key, checked, false).replace(`data-field="${key}"`, `data-action="toggle-memory-preference" data-key="${key}"`);
}
/** Recreates the separate memory settings page: global, shared, then character libraries. */
export function memoryPageBody(snapshot: Snapshot, avatarSources: Map<string, string>) {
  let stores = "";
  for (const store of snapshot.stores) {
    let count = 0;
    for (const card of snapshot.cards) if (card.memoryBindingMode === "SHARED" && card.sharedMemoryId === store.id) count++;
    stores += `<div class="entity"><span class="avatar">${icon("hub")}</span><div class="entity-copy"><div class="entity-title">${escapeHtml(store.name)}</div><div class="badges"><span class="badge">${count} 个角色绑定</span></div></div><div class="entity-actions">${iconButton("owner-graph", "记忆图谱", "graph", `data-owner="shared:${escapeHtml(store.id)}" data-name="${escapeHtml(store.name)}"`)}${iconButton("owner-user", "用户资料", "profile", `data-owner="shared:${escapeHtml(store.id)}" data-name="${escapeHtml(store.name)}"`)}${iconButton("owner-settings", "记忆设置", "tune", `data-owner="shared:${escapeHtml(store.id)}" data-name="${escapeHtml(store.name)}"`)}${iconButton("edit-store", "编辑共享记忆库", "edit", `data-id="${escapeHtml(store.id)}"`)}${iconButton("delete-store", "删除共享记忆库", "trash", `data-id="${escapeHtml(store.id)}"`)}</div></div>`;
  }
  let characters = "";
  for (const card of snapshot.cards) {
    const owner = ownerFor(card), active = isActive(snapshot.active, "card", card.id);
    const source = avatarSources.get(card.avatarUri ?? "");
    characters += `<article class="entity ${active ? "active" : ""}" data-memory-card="${escapeHtml(card.id)}"><span class="avatar" data-avatar-uri="${escapeHtml(card.avatarUri ?? "")}">${source ? `<img src="${escapeHtml(source)}" alt="">` : icon("person")}</span><div class="entity-copy"><div class="entity-title">${escapeHtml(card.name)}</div><div class="badges"><span class="badge">${card.memoryBindingMode === "SHARED" ? "共享记忆" : "角色记忆"}</span>${active ? '<span class="active-pill">当前使用</span>' : ""}</div></div><div class="entity-actions">${iconButton("owner-graph", "记忆图谱", "graph", `data-owner="${escapeHtml(owner)}" data-name="${escapeHtml(card.name)}"`)}${iconButton("owner-user", "用户资料 · USER.md", "profile", `data-owner="${escapeHtml(owner)}" data-name="${escapeHtml(card.name)}"`)}${iconButton("owner-settings", "记忆设置", "tune", `data-owner="${escapeHtml(owner)}" data-name="${escapeHtml(card.name)}"`)}</div></article>`;
  }
  return `<section class="section"><div class="section-head">${icon("tune")}<h2>全局记忆设置</h2></div>${globalSwitch("向模型提供用户资料", "聊天执行时提供用户资料；关闭不会删除 USER.md 或改写历史记录。", "userDescriptionEnabled", snapshot.memoryPreferences.userDescriptionEnabled)}${globalSwitch("自动更新记忆库", "允许自动候选入队和定时提取；关闭不会取消手动历史重建。", "autoUpdateEnabled", snapshot.memoryPreferences.autoUpdateEnabled)}</section><section class="section"><div class="section-head">${icon("hub")}<h2>共享记忆库</h2>${button("create-store", "创建", "add", "", "small")}</div>${stores === "" ? '<div class="empty">当前还没有共享记忆库，创建后可被多个角色卡挂载共享。</div>' : stores}</section><section class="section"><div class="section-head">${icon("badge")}<h2>角色记忆库</h2></div>${characters}</section>`;
}
