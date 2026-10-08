import type { ActivePrompt, Group, Snapshot } from "../../../src/model";
import { escapeHtml, icon, iconButton } from "../../shared/ui/html";
import { isActive } from "../characters/data";
import { groupMemberNames, selectedEntity } from "../characters/selection";

/** Renders a real group option and its canonical member names without activating it. */
export function groupSelectionTile(group: Group, selection: ActivePrompt | null, snapshot: Snapshot): string {
  const selected = selectedEntity(selection, "group", group.id), names = groupMemberNames(group, snapshot).join("、");
  return `<article class="entity ${selected ? "selected" : ""}" data-selection-entity="group"><button type="button" class="entity-main" role="radio" aria-checked="${selected}" aria-label="选择群组 ${escapeHtml(group.name)}" data-action="select-group" data-id="${escapeHtml(group.id)}"><span class="avatar">${icon("groups")}</span><span class="entity-copy"><span class="entity-title">${escapeHtml(group.name)}</span><span class="entity-subtitle">${escapeHtml(group.description)}</span><span class="badges"><span class="badge">${group.members.length} 个角色</span><span class="badge">${escapeHtml(names)}</span>${isActive(snapshot.active, "group", group.id) ? '<span class="badge">当前使用</span>' : ""}</span></span><span class="selection-check">${selected ? icon("check") : ""}</span></button><div class="entity-actions">${iconButton("preview-group", `预览群组 ${group.name}`, "info", `data-id="${escapeHtml(group.id)}"`)}</div></article>`;
}
