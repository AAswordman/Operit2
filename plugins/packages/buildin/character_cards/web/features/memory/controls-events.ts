import type { EditorContext } from "../../bridge/context";
import { memoryRebuilding, updateMemoryStatus } from "./controls-view";

/** Stages owner-scoped checkboxes without writing until an explicit settings save/rebuild. */
export function changeMemoryControls(context: EditorContext, element: HTMLInputElement): boolean {
  const dialog = context.state.dialogs[context.state.dialogs.length - 1];
  if (dialog?.type !== "owner-settings") return false;
  if (element.hasAttribute("data-memory-chat")) {
    if (memoryRebuilding(dialog)) throw new Error("重建期间不能改变已提交的聊天");
    const id = element.getAttribute("data-memory-chat");
    if (id === null || dialog.chats === null || !dialog.chats.some(chat => chat.id === id)) throw new Error("聊天不属于此记忆库");
    dialog.selectedChatIds = dialog.selectedChatIds.filter(value => value !== id);
    if (element.checked) dialog.selectedChatIds.push(id);
    updateMemoryStatus(dialog, context.dialogsRoot); return true;
  }
  if (element.type !== "checkbox" || dialog.settings === null) return false;
  switch (element.dataset.field) {
    case "profileAuto": dialog.settings.profileAutoUpdateEnabled = element.checked; break;
    case "profileLocked": dialog.settings.profileAutoUpdateLocked = element.checked; break;
    case "embedding": dialog.settings.cloudEmbeddingEnabled = element.checked; break;
    default: return false;
  }
  context.renderDialogs(); return true;
}
