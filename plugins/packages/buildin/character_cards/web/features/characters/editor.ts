import type { Snapshot } from "../../../src/model";
import type { CharacterDialog } from "../../bridge/contracts";
import { button, escapeHtml, field, icon } from "../../shared/ui/html";
import { bindingsBody } from "./bindings/view";
/** Builds the same basic/content/binding tabs without embedding memory management. */
export function characterBody(dialog: CharacterDialog, snapshot: Snapshot, avatarSources: Map<string, string>): string {
  const card = dialog.card;
  switch (dialog.tab) {
    case 0: {
      let tags = "";
      for (const tag of dialog.tags) tags += `<button type="button" class="chip ${card.attachedTagIds.indexOf(tag.id) >= 0 ? "selected" : ""}" data-action="toggle-tag" data-id="${escapeHtml(tag.id)}">${card.attachedTagIds.indexOf(tag.id) >= 0 ? icon("check") : ""}${escapeHtml(tag.name)}</button>`;
      return field("角色名称 *", "name", card.name) + field("描述", "description", card.description) + `<fieldset class="avatar-editor"><legend>角色头像</legend><div class="avatar">${avatarSources.has(card.avatarUri ?? "") ? `<img src="${escapeHtml(avatarSources.get(card.avatarUri ?? ""))}" alt="角色头像">` : icon("person")}</div><span class="grow avatar-path">${escapeHtml(card.avatarUri ?? "未设置")}</span>${button("choose-avatar", "选择", "", "", "small")}${card.avatarUri === null ? "" : button("clear-avatar", "清除", "", "", "small")}</fieldset>` + field("角色设定", "characterSetting", card.characterSetting, 6) + field("开场白", "openingStatement", card.openingStatement, 3) + `<div class="tag-head"><span>标签</span>${button("show-tags", "管理标签", "", "", "small")}</div><div class="chips">${tags === "" ? '<span class="muted">暂无提示词标签</span>' : tags}</div>`;
    }
    case 1: return field("聊天附加内容", "otherContentChat", card.otherContentChat, 4) + field("语音附加内容", "otherContentVoice", card.otherContentVoice, 4) + field("高级自定义 Prompt", "advancedCustomPrompt", card.advancedCustomPrompt, 4) + field("备注", "marks", card.marks, 3);
    case 2: return bindingsBody(dialog, snapshot);
    default: throw new Error("Invalid character editor tab");
  }
}
