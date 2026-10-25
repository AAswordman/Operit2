import type { ChatBinding } from "./model";
import { markerObject } from "./chat-markers";
import { assertInteger, requireId } from "./validation";
import { getRepository } from "./repository-runtime";

// Main-runtime entry adapters share the repository's completed initialization.
export async function findChatBinding(chatId: string): Promise<ChatBinding | null> {
  return (await getRepository()).chatBindings.findChatBinding(chatId);
}

export async function bindingsForChats(chatIds: readonly string[]): Promise<Map<string, string>> {
  return (await getRepository()).chatBindings.bindingsForChats(chatIds);
}

export async function prepareChatBinding(binding: ChatBinding): Promise<void> {
  return (await getRepository()).chatBindings.prepareChatBinding(binding);
}

export async function readDraftBinding(chatId: string): Promise<ChatBinding> {
  return (await getRepository()).chatBindings.readDraftBinding(chatId);
}

export async function commitDraftBinding(chatId: string): Promise<void> {
  return (await getRepository()).chatBindings.commitDraftBinding(chatId);
}

export async function abortDraftBinding(chatId: string): Promise<void> {
  return (await getRepository()).chatBindings.abortDraftBinding(chatId);
}

/** Reads only the actual author ID attached by Core; execution settings are never stored here. */
export async function readMessageAuthor(chatId: string, messageTimestamp: number, variantIndex: number): Promise<string | null> {
  requireId(chatId, "chat id");
  assertInteger(messageTimestamp, "message timestamp", 0);
  assertInteger(variantIndex, "message variant index", 0);
  const { extension } = await Tools.Chat.readExtension({ kind: "message", chatId, messageTimestamp, variantIndex });
  if (extension === null) return null;
  const identity = markerObject(extension, "message author identity");
  if (Object.keys(identity).length !== 1) throw new Error("Invalid message author identity");
  return requireId(identity.participantId, "message author participant id");
}
