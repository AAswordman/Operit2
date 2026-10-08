import type { Card, Snapshot } from "../../../src/model";
import type { DialogSpec, PreviewDialog } from "../../bridge/contracts";
import { button, escapeHtml, icon, notice } from "../../shared/ui/html";
import { getGroup } from "../groups/data";
import { groupMemberNames } from "./selection";
import { getCard, ownerFor } from "./data";

/** Presents persisted text as selectable read-only content without manufacturing empty records. */
function textSection(title: string, value: string): string {
  return `<section class="preview-section"><h3>${escapeHtml(title)}</h3><pre>${escapeHtml(value)}</pre></section>`;
}

/** Resolves only the two declared model bindings and rejects incomplete fixed-model records. */
function modelBinding(card: Card): string {
  switch (card.chatModelBindingMode) {
    case "FOLLOW_GLOBAL": return "跟随全局模型配置";
    case "FIXED_MODEL":
      if (card.chatModelId === null || card.chatModelId.trim() === "") throw new Error("角色卡固定模型绑定不完整");
      return card.chatModelId;
    default: throw new Error("角色卡聊天模型绑定类型无效");
  }
}

/** Renders complete saved character fields, not a new editable draft. */
function characterPreview(dialog: PreviewDialog, snapshot: Snapshot): string {
  const card = getCard(snapshot, dialog.id);
  switch (dialog.tab) {
    case 0: {
      let tags = "";
      for (const id of card.attachedTagIds) for (const tag of snapshot.tags) if (tag.id === id) tags += `<span class="badge">${escapeHtml(tag.name)}</span>`;
      return `<div class="row"><span class="avatar">${icon("person")}</span><div class="grow"><h3>${escapeHtml(card.name)}</h3><div class="badges">${tags}</div></div></div>` + textSection("描述", card.description) + textSection("角色设定", card.characterSetting) + textSection("开场白", card.openingStatement) + (card.avatarUri === null ? "" : textSection("头像 URI", card.avatarUri) + notice("头像资源预览接口尚未接入；此处仅显示已保存的 URI。"));
    }
    case 1: return textSection("聊天附加内容", card.otherContentChat) + textSection("语音附加内容", card.otherContentVoice) + textSection("高级自定义 Prompt", card.advancedCustomPrompt) + textSection("备注", card.marks);
    case 2: {
      const model = modelBinding(card);
      const tts = card.ttsConfigId === null ? "跟随全局 TTS 配置" : card.ttsConfigId;
      const theme = card.themeConfigId === null ? "未绑定主题（不改变当前独立主题）" : card.themeConfigId;
      return textSection("聊天模型", model) + textSection("主题配置", theme) + textSection("TTS 配置", tts) + textSection("记忆库", ownerFor(card)) + textSection("共享记忆挂载", JSON.stringify(card.sharedMemoryMounts, null, 2)) + textSection("工具访问配置", JSON.stringify(card.toolAccessConfig, null, 2));
    }
    default: throw new Error("角色预览页签无效");
  }
}

/** Reuses the Material dialog shell for both local and presented entity previews. */
export function previewSpec(dialog: PreviewDialog, snapshot: Snapshot): DialogSpec {
  const footer = button("close-dialog", "关闭", "") + button("preview-edit", "编辑", "edit", "", "filled");
  switch (dialog.entity) {
    case "card": return { title: "角色卡预览", style: "large", tabs: ["基础", "内容", "绑定"], body: characterPreview(dialog, snapshot), footer };
    case "group": {
      const group = getGroup(snapshot, dialog.id);
      return { title: "群组预览", style: "large", tabs: [], body: textSection("群组名称", group.name) + textSection("描述", group.description) + textSection("主题配置", group.themeConfigId === null ? "未绑定主题（不改变当前独立主题）" : group.themeConfigId) + textSection("组内角色", groupMemberNames(group, snapshot).join("、")), footer };
    }
  }
}
