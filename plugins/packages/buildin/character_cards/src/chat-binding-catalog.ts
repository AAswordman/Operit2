import type { BindableChat } from "./model";
import { bindingsForChats } from "./chat-storage";
import { assertBoolean, assertInteger, assertString, requireId } from "./validation";

/** Joins complete host history metadata with indexed plugin bindings for the batch picker. */
export async function listBindableChats(): Promise<BindableChat[]> {
  const result = await Tools.Chat.listAll();
  assertInteger(result.totalCount, "conversation count", 0);
  if (!Array.isArray(result.chats) || result.chats.length !== result.totalCount)
    throw new Error("对话列表不完整，请刷新后重试");
  const ids = new Set<string>();
  for (const chat of result.chats) {
    requireId(chat.id, "conversation id");
    if (ids.has(chat.id)) throw new Error("对话列表包含重复记录");
    ids.add(chat.id);
    assertString(chat.title, "conversation title");
    assertString(chat.updatedAt, "conversation update time");
    assertInteger(chat.messageCount, "message count", 0);
    assertBoolean(chat.isCurrent, "current conversation");
  }
  const bindings = await bindingsForChats([...ids]);
  return result.chats.map(chat => ({
    id: chat.id, title: chat.title, updatedAt: chat.updatedAt,
    messageCount: chat.messageCount, isCurrent: chat.isCurrent,
    selection: bindings.get(chat.id) ?? null,
  })).sort((left, right) => timestamp(right.updatedAt) - timestamp(left.updatedAt) || left.id.localeCompare(right.id));
}

/** Accepts both ISO dates and millisecond strings supplied by generic history APIs. */
function timestamp(value: string): number {
  const parsed = /^\d+$/.test(value) ? Number(value) : Date.parse(value);
  return Number.isFinite(parsed) ? parsed : 0;
}
