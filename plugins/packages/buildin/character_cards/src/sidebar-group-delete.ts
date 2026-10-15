/** Native deletions are not atomic; retain every real partial outcome for the confirmation dialog. */
export interface SidebarDeletionProgress { deletedChatIds: string[]; failedChatIds: { chatId: string; error: string }[] }
export async function deleteConversationGroupMembers(chatIds: string[], deleteChat: (id: string) => Promise<{chatId: string; deletedAt: number}>): Promise<SidebarDeletionProgress> {
  if (new Set(chatIds).size !== chatIds.length) throw new Error("Duplicate confirmed folder members");
  const progress: SidebarDeletionProgress = { deletedChatIds: [], failedChatIds: [] };
  for (const chatId of chatIds) {
    try {
      const result = await deleteChat(chatId);
      if (result.chatId !== chatId || !Number.isFinite(result.deletedAt)) throw new Error("Native deletion did not confirm requested chat: " + chatId);
      progress.deletedChatIds.push(chatId);
    } catch (error) { progress.failedChatIds.push({ chatId, error: String(error) }); }
  }
  return progress;
}
