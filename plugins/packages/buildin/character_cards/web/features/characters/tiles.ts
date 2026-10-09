import type { Card, Snapshot } from "../../../src/model";
import { button, escapeHtml, icon, iconButton } from "../../shared/ui/html";
import { isActive } from "./data";
/** Builds an entity tile with the original active border, badges and compact action bar. */
export function cardTile(card: Card, snapshot: Snapshot, avatarSources: Map<string, string>): string {
  let tags = "";
  for (const id of card.attachedTagIds) for (const tag of snapshot.tags) if (tag.id === id) tags += `<span class="badge">${escapeHtml(tag.name)}</span>`;
  const active = isActive(snapshot.active, "card", card.id);
  const binding = card.memoryBindingMode === "SHARED" ? "共享记忆" : "角色记忆";
  return `<article class="entity ${active ? "active" : ""}"><button type="button" class="entity-main" data-action="edit-card" data-id="${escapeHtml(card.id)}" aria-label="编辑 ${escapeHtml(card.name)}"><span class="avatar">${avatarSources.has(card.avatarUri ?? "") ? `<img src="${escapeHtml(avatarSources.get(card.avatarUri ?? ""))}" alt="">` : icon("person")}${active ? `<span class="avatar-check">${icon("check")}</span>` : ""}</span><span class="entity-copy"><span class="entity-title">${escapeHtml(card.name)}</span>${card.description === "" ? "" : `<div class="entity-subtitle">${escapeHtml(card.description)}</div>`}<span class="badges"><span class="badge">${card.chatModelBindingMode === "FIXED_MODEL" ? "固定模型配置" : "跟随全局模型"}</span><span class="badge">${binding}</span>${tags}</span></span></button><div class="entity-actions">${iconButton("card-graph", "打开记忆图谱", "graph", `data-id="${escapeHtml(card.id)}"`)}${iconButton("card-user", "编辑用户资料", "profile", `data-id="${escapeHtml(card.id)}"`)}${active ? '<span class="active-pill">当前使用</span>' : button("activate-card", "使用", "", `data-id="${escapeHtml(card.id)}"`, "small")}</div></article>`;
}
