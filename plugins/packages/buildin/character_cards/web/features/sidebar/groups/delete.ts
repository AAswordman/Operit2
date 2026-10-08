import type { ConversationGroupDeletion, ConversationGroupRecord } from "../../../../src/api";
import type { SidebarDeletionProgress } from "../contracts";

/** Requires concrete native deletion and backend metadata operations, without implementing any successful substitutes. */
export interface ConversationGroupDeleteActions {
  /** Deletes exactly one actual host conversation through the generic Chat API. */
  deleteChat(chatId: string): Promise<{ chatId: string; deletedAt: number }>;
  /** Reads the actual persisted group after native deletion callbacks have removed successful memberships. */
  readGroup(id: string): Promise<ConversationGroupRecord>;
  /** Deletes only already-empty plugin group metadata after every native member deletion has succeeded. */
  deleteMetadata(id: string): Promise<ConversationGroupDeletion>;
}
/** Performs the explicitly confirmed original group-and-member deletion UX without claiming atomicity or retrying failures. */
export async function deleteConversationGroupMembers(group: ConversationGroupRecord, actions: ConversationGroupDeleteActions): Promise<SidebarDeletionProgress> {
  const progress: SidebarDeletionProgress = { deletedChatIds: [], failedChatIds: [], metadataDeleted: false, metadataError: null };
  const members = [...group.chatIds];
  if (new Set(members).size !== members.length) throw new Error("Confirmed conversation group has duplicate members");
  for (const chatId of members) {
    try {
      const result = await actions.deleteChat(chatId);
      if (result.chatId !== chatId || !Number.isFinite(result.deletedAt)) throw new Error("Native deletion did not confirm the requested chat: " + chatId);
      progress.deletedChatIds.push(chatId);
    } catch (failure) { progress.failedChatIds.push({ chatId, error: String(failure) }); }
  }
  if (progress.failedChatIds.length !== 0) return progress;
  try {
    const persisted = await actions.readGroup(group.id);
    if (persisted.id !== group.id || persisted.ownerSelection !== group.ownerSelection) throw new Error("Confirmed conversation group identity or scope changed during deletion");
    if (persisted.chatIds.length !== 0) throw new Error("Group still has actual persisted members after native deletion; its metadata was not deleted");
    const deleted = await actions.deleteMetadata(group.id);
    if (deleted.id !== group.id || deleted.deleted !== true || deleted.releasedChatIds.length !== 0) throw new Error("The backend did not confirm empty group metadata deletion");
    progress.metadataDeleted = true;
  } catch (failure) { progress.metadataError = String(failure); }
  return progress;
}
