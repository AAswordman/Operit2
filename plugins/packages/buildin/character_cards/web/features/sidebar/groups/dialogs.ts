import type { SidebarDialog, SidebarViewState } from "../contracts";
import { button, escapeHtml, field, selectField } from "../../../shared/ui/html";

/** Reports the exact successful and failed native deletion attempts without promising rollback or automatic retry. */
function deletionProgress(dialog: Extract<SidebarDialog, { type: "delete-group" }>): string {
  if (dialog.progress === null) return "";
  let html = '<div class="sidebar-delete-progress"><p>已实际删除 ' + dialog.progress.deletedChatIds.length + ' 条对话。</p><ul>';
  for (const chatId of dialog.progress.deletedChatIds) html += '<li data-delete-result="success">' + escapeHtml(chatId) + "：已删除</li>";
  for (const failure of dialog.progress.failedChatIds) html += '<li data-delete-result="failure">' + escapeHtml(failure.chatId) + "：" + escapeHtml(failure.error) + "</li>";
  if (dialog.progress.metadataError !== null) html += '<li data-delete-result="failure">分组记录：' + escapeHtml(dialog.progress.metadataError) + "</li>";
  html += "</ul><p>" + (dialog.progress.metadataDeleted ? "分组记录已实际删除。" : "分组记录尚未删除，可核对剩余成员。") + "</p></div>";
  return html;
}
/** Paints the original create/rename/delete dialogs with staged values and explicit member-chat deletion consent. */
export function sidebarGroupDialog(state: SidebarViewState): string {
  const dialog = state.dialog;
  if (dialog === null) return "";
  let title: string, content: string, submit: string;
  switch (dialog.type) {
    case "create-group": title = "新建分组"; submit = "创建"; content = field("群组名称", "sidebar-group-name", dialog.name); break;
    case "rename-group": title = "重命名分组"; submit = "保存"; content = field("群组名称", "sidebar-group-name", dialog.name); break;
    case "delete-group": title = "删除分组"; submit = "删除"; content = '<p>确定要删除分组“' + escapeHtml(dialog.name) + '”及其中的 ' + dialog.memberCount + ' 条对话吗？</p>' + deletionProgress(dialog); break;
    case "move-chat": title = "移动对话"; submit = "移动"; content = selectField("目标分组", "sidebar-target-group", dialog.groupId, dialog.choices.map(
      /** Presents each exact scoped choice by stable id, preserving legal duplicate names. */
      choice => ({ id: choice.id, label: choice.title }),
    ), state.busy); break;
  }
  const disabled = state.busy ? "disabled" : "";
  const deleted = dialog.type === "delete-group" && dialog.progress !== null;
  const confirm = deleted ? "" : button("sidebar-submit-dialog", submit, "", disabled, dialog.type === "delete-group" ? "danger" : "filled");
  return '<div class="dialog-backdrop"><section class="dialog sidebar-form-dialog" role="dialog" aria-modal="true" aria-label="' + escapeHtml(title) + '"><header class="dialog-header"><h2>' + escapeHtml(title) + '</h2></header><div class="dialog-body">' + content + '</div><footer class="dialog-footer">' + button("sidebar-cancel-dialog", deleted ? "关闭" : "取消", "", disabled) + confirm + "</footer></section></div>";
}
