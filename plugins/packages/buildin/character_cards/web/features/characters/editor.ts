import type { Snapshot } from "../../../src/model";
import type { CharacterDialog } from "../../bridge/contracts";
import { button, escapeHtml, field, icon, notice } from "../../shared/ui/html";
import { ownerFor } from "./data";
import { sharedStore } from "./bindings/data";
import { bindingsBody } from "./bindings/view";
/** Builds the same basic/content/binding tabs plus owner-local memory management. */
export function characterBody(dialog: CharacterDialog, snapshot: Snapshot): string {
  const card = dialog.card;
  switch (dialog.tab) {
    case 0: {
      let tags = "";
      for (const tag of dialog.tags) tags += `<button type="button" class="chip ${card.attachedTagIds.indexOf(tag.id) >= 0 ? "selected" : ""}" data-action="toggle-tag" data-id="${escapeHtml(tag.id)}">${card.attachedTagIds.indexOf(tag.id) >= 0 ? icon("check") : ""}${escapeHtml(tag.name)}</button>`;
      return field("角色名称 *", "name", card.name) + field("描述", "description", card.description) + `<div class="avatar-editor"><div class="avatar" title="头像预览尚未接入资源接口">${icon("person")}</div><div class="grow">${field("头像 URI", "avatarUri", card.avatarUri)}<div class="actions">${button("choose-avatar", "选择图片", "", "disabled title=\"图片选择接口待接入\"", "small")}${button("clear-avatar", "清除", "", "", "small")}</div></div></div>` + field("角色设定", "characterSetting", card.characterSetting, 6) + field("开场白", "openingStatement", card.openingStatement, 3) + `<div class="tag-head"><span>标签</span>${button("show-tags", "管理标签", "", "", "small")}</div><div class="chips">${tags === "" ? '<span class="muted">暂无提示词标签</span>' : tags}</div>`;
    }
    case 1: return field("聊天附加内容", "otherContentChat", card.otherContentChat, 4) + field("语音附加内容", "otherContentVoice", card.otherContentVoice, 4) + field("高级自定义 Prompt", "advancedCustomPrompt", card.advancedCustomPrompt, 4) + field("备注", "marks", card.marks, 3);
    case 2: return bindingsBody(dialog, snapshot);
    case 3: {
      if (dialog.create) return notice("保存角色卡后，即可在这里管理这张角色卡绑定的记忆库。");
      const owner = ownerFor(card);
      let mounts = "";
      for (const mount of card.sharedMemoryMounts) {
        const name = sharedStore(snapshot, mount.sharedMemoryId).name;
        mounts += `<div class="memory-entry"><h3>${escapeHtml(name)}</h3><div class="badges"><span class="badge">${mount.readable ? "可读" : "不可读"}</span><span class="badge">${mount.writable ? "可写" : "不可写"}</span></div><div class="memory-buttons">${button("owner-graph", "记忆图谱", "graph", `data-owner="shared:${escapeHtml(mount.sharedMemoryId)}" data-name="${escapeHtml(name)}"`, "tonal")}${button("owner-user", "用户资料", "profile", `data-owner="shared:${escapeHtml(mount.sharedMemoryId)}" data-name="${escapeHtml(name)}"`)}</div></div>`;
      }
      return `<div class="memory-entry"><h3>${escapeHtml(card.name)} 的记忆库</h3><div class="muted">${escapeHtml(owner)}</div><div class="badges"><span class="badge">${card.memoryBindingMode === "SHARED" ? "共享记忆" : "角色记忆"}</span></div><div class="memory-buttons">${button("owner-graph", "记忆图谱", "graph", `data-owner="${escapeHtml(owner)}" data-name="${escapeHtml(card.name)}"`, "tonal")}${button("owner-user", "用户资料 · USER.md", "profile", `data-owner="${escapeHtml(owner)}" data-name="${escapeHtml(card.name)}"`)}${button("owner-settings", "记忆设置", "tune", `data-owner="${escapeHtml(owner)}" data-name="${escapeHtml(card.name)}"`)}</div></div>${mounts === "" ? "" : `<div class="tag-head">挂载的共享记忆库</div>${mounts}`}${notice("记忆内容读写这张角色卡当前绑定的插件记忆库，角色与记忆数据独立存储在插件目录文件中。")}`;
    }
    default: throw new Error("Invalid character editor tab");
  }
}
