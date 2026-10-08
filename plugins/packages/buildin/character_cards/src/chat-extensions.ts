import type { ChatBinding } from "./model";
import type { MessageMarker } from "./chat-markers";
import { markerObject, decodeChatMarker, decodeMessageMarker, encodeChatMarker } from "./chat-markers";
import { assertBoolean, assertInteger, requireId } from "./validation";

/** Reads the executing package's actual record namespace and requires an explicit valid selection marker. */
export async function readChatExtensionBinding(chatId: string): Promise<ChatBinding> {
  requireId(chatId, "chat id");
  const extension = await Tools.Chat.readExtension({ kind: "chat", chatId });
  if (extension === null) throw new Error("Chat has no character selection in this plugin's namespace: " + chatId);
  const marker = decodeChatMarker(extension);
  return { chatId, selection: marker.selection };
}
/** Requires a complete snapshot from one exact persisted message revision without reading current chat identity. */
export async function readMessageExtensionMarker(chatId: string, messageTimestamp: number, variantIndex: number): Promise<MessageMarker> {
  requireId(chatId, "chat id"); assertInteger(messageTimestamp, "message timestamp", 0); assertInteger(variantIndex, "message variant index", 0);
  if (variantIndex > 2147483647) throw new Error("Message variant index exceeds the host's i32 range");
  const extension = await Tools.Chat.readExtension({ kind: "message", chatId, messageTimestamp, variantIndex });
  if (extension === null) throw new Error("Message revision has no saved character identity in this plugin's namespace: " + chatId + "/" + messageTimestamp + "/" + variantIndex);
  return decodeMessageMarker(extension);
}
/** Enumerates real host conversations and projects only actual markers from their authenticated record namespaces. */
export async function listChatExtensionBindings(): Promise<ChatBinding[]> {
  const result = await Tools.Chat.listAll(); assertInteger(result.totalCount, "all chat count", 0);
  if (!Array.isArray(result.chats) || result.chats.length !== result.totalCount) throw new Error("Chat.listAll did not return its complete declared records");
  const bindings: ChatBinding[] = [], ids = new Set<string>();
  for (const chat of result.chats) {
    const chatId = requireId(chat.id, "host chat id");
    if (ids.has(chatId)) throw new Error("Duplicate host chat identity: " + chatId); ids.add(chatId);
    const extension = await Tools.Chat.readExtension({ kind: "chat", chatId });
    if (extension === null) continue;
    const marker = decodeChatMarker(extension); bindings.push({ chatId, selection: marker.selection });
  }
  return bindings;
}
/** Writes an explicit selection while retaining every other validated field in the same actual record namespace. */
export async function writeChatExtensionBinding(binding: ChatBinding): Promise<ChatBinding> {
  requireId(binding.chatId, "chat id");
  const target = { kind: "chat" as const, chatId: binding.chatId };
  const existing = await Tools.Chat.readExtension(target), value = encodeChatMarker(binding.selection, existing === null ? null : markerObject(existing, "existing chat extension"));
  const stored = await Tools.Chat.writeExtension(target, value), marker = decodeChatMarker(stored);
  if (marker.selection !== binding.selection) throw new Error("Stored chat extension selection differs from the submitted value");
  return { chatId: binding.chatId, selection: marker.selection };
}
/** Removes this plugin's namespace from an existing record without touching file-owned group membership or deleting chats. */
export async function deleteChatExtensionBinding(chatId: string): Promise<{ chatId: string; deleted: boolean }> {
  requireId(chatId, "chat id");
  const deleted = await Tools.Chat.deleteExtension({ kind: "chat", chatId }); assertBoolean(deleted, "chat extension deleted");
  return { chatId, deleted };
}
