import type { GroupValues, Snapshot } from "../../../src/model";
import { button, escapeHtml, icon } from "../../shared/ui/html";
import { getCard, isActive } from "../characters/data";
/** Builds a group tile using the existing member order and identities. */
export function groupTile(group: GroupValues, snapshot: Snapshot): string {
  let names = "";
  for (const member of group.members) names += `${names === "" ? "" : "、"}${getCard(snapshot, member.characterCardId).name}`;
  const active = isActive(snapshot.active, "group", group.id);
  return `<article class="entity ${active ? "active" : ""}"><button type="button" class="entity-main" data-action="edit-group" data-id="${escapeHtml(group.id)}" aria-label="编辑群组 ${escapeHtml(group.name)}"><span class="avatar">${icon("groups")}</span><span class="entity-copy"><span class="entity-title">${escapeHtml(group.name)}</span>${names === "" ? "" : `<div class="entity-subtitle">${escapeHtml(names)}</div>`}<span class="badges"><span class="badge">${group.members.length} 个角色</span></span></span></button><div class="entity-actions">${active ? '<span class="active-pill">当前使用</span>' : button("activate-group", "使用", "", `data-id="${escapeHtml(group.id)}"`, "small")}</div></article>`;
}

