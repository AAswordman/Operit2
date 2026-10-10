import type { ThemeChoice } from "../../../src/selection-application";
import type { GroupValues, Snapshot } from "../../../src/model";
import type { GroupDialog } from "../../bridge/contracts";
import { button, escapeHtml, field, icon } from "../../shared/ui/html";
import { getCard, isActive } from "../characters/data";
import { themeReferenceSection } from "../characters/bindings/theme";
/** Builds a group tile using the existing member order and identities. */
export function groupTile(group: GroupValues, snapshot: Snapshot): string {
  let names = "";
  for (const member of group.members) names += `${names === "" ? "" : "、"}${getCard(snapshot, member.characterCardId).name}`;
  const active = isActive(snapshot.active, "group", group.id);
  return `<article class="entity ${active ? "active" : ""}"><button type="button" class="entity-main" data-action="edit-group" data-id="${escapeHtml(group.id)}" aria-label="编辑群组 ${escapeHtml(group.name)}"><span class="avatar">${icon("groups")}</span><span class="entity-copy"><span class="entity-title">${escapeHtml(group.name)}</span>${names === "" ? "" : `<div class="entity-subtitle">${escapeHtml(names)}</div>`}<span class="badges"><span class="badge">${group.members.length} 个角色</span></span></span></button><div class="entity-actions">${active ? '<span class="active-pill">当前使用</span>' : button("activate-group", "使用", "", `data-id="${escapeHtml(group.id)}"`, "small")}</div></article>`;
}

/** Builds group membership editing without changing Core member identities. */
export function groupBody(dialog: GroupDialog, snapshot: Snapshot, themes: readonly ThemeChoice[]): string {
  let members = "";
  for (const card of snapshot.cards) {
    let index = -1;
    for (let i = 0; i < dialog.group.members.length; i++) if (dialog.group.members[i].characterCardId === card.id) index = i;
    members += `<label class="checkbox-row"><input type="checkbox" data-member="${escapeHtml(card.id)}" ${index >= 0 ? "checked" : ""}><span class="avatar">${icon("person")}</span><span class="grow">${escapeHtml(card.name)}</span></label>`;
  }
  return field("群组名称 *", "name", dialog.group.name) + field("描述", "description", dialog.group.description, 3) + themeReferenceSection(dialog.group.themeConfigId, themes) + `<div class="tag-head">组内角色</div><div class="members-list">${members}</div>`;
}
