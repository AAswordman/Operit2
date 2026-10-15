import type { ChatBinding } from "./model";
import type { MessageMarker } from "./chat-markers";
import {
  decodeChatMarker,
  decodeMessageMarker,
  markerObject,
} from "./chat-markers";
import { assertInteger, requireId } from "./validation";
import { RelationalDatabase } from "./storage/relational";

let database: Promise<RelationalDatabase> | null = null;
export interface SnapshotReference {
  recordId: string;
  schemaVersion: 1;
}
/** Retains the plugin's SQL handle independently of any Core extension reads. */
export function chatDatabase(): Promise<RelationalDatabase> {
  if (database === null) database = RelationalDatabase.open();
  return database;
}
/** Reads an explicitly nullable binding from the plugin's indexed table. */
export async function findChatBinding(
  chatId: string,
): Promise<ChatBinding | null> {
  requireId(chatId, "chat id");
  const row = await (await chatDatabase()).get("chat_bindings", chatId);
  return row === null
    ? null
    : { chatId, selection: decodeChatMarker(row.value).selection };
}
/** Requires a genuine plugin-owned binding without consulting global active state. */
export async function readChatBinding(chatId: string): Promise<ChatBinding> {
  const binding = await findChatBinding(chatId);
  if (binding === null)
    throw new Error("Chat has no stored character binding: " + chatId);
  return binding;
}
/** Validates a generic persisted reference before accessing the plugin snapshot table. */
export function snapshotReference(value: unknown): SnapshotReference {
  const reference = markerObject(value, "plugin snapshot reference");
  if (Object.keys(reference).length !== 2 || reference.schemaVersion !== 1)
    throw new Error("Invalid plugin snapshot reference");
  return {
    recordId: requireId(reference.recordId, "snapshot record id"),
    schemaVersion: 1,
  };
}
/** Reads an immutable complete snapshot from this plugin's typed relation tables. */
export async function readStoredSnapshot(
  value: SnapshotReference,
): Promise<MessageMarker> {
  const reference = snapshotReference(value),
    row = await (
      await chatDatabase()
    ).get("message_snapshots", reference.recordId);
  if (row === null)
    throw new Error("Plugin snapshot is unavailable: " + reference.recordId);
  return decodeMessageMarker(row.value);
}
/** Persists the snapshot before Core commits its generic reference. */
export async function storeSnapshot(
  value: MessageMarker,
): Promise<SnapshotReference> {
  const marker = decodeMessageMarker(value),
    recordId =
      "snapshot:" +
      Array.from({ length: 4 }, () =>
        Math.floor(Math.random() * 0x100000000)
          .toString(16)
          .padStart(8, "0"),
      ).join("");
  await (
    await chatDatabase()
  ).commit([
    {
      op: "put",
      collection: "message_snapshots",
      key: recordId,
      value: markerObject(marker, "message snapshot"),
      expectedVersion: null,
    },
  ]);
  return { recordId, schemaVersion: 1 };
}
/** Reads the exact Core revision reference and resolves its plugin-owned historical snapshot. */
export async function readMessageSnapshot(
  chatId: string,
  messageTimestamp: number,
  variantIndex: number,
): Promise<MessageMarker> {
  requireId(chatId, "chat id");
  assertInteger(messageTimestamp, "message timestamp", 0);
  assertInteger(variantIndex, "message variant index", 0);
  if (variantIndex > 2147483647)
    throw new Error("Message variant index exceeds the host's i32 range");
  const reference = (
    await Tools.Chat.readExtension({
      kind: "message",
      chatId,
      messageTimestamp,
      variantIndex,
    })
  ).extension;
  if (reference === null)
    throw new Error(
      "Message revision has no saved configuration reference: " +
        chatId +
        "/" +
        messageTimestamp +
        "/" +
        variantIndex,
    );
  return readStoredSnapshot(snapshotReference(reference));
}
/** Enumerates bindings in one indexed SQL query without listing Core conversations. */
export async function listChatBindings(): Promise<ChatBinding[]> {
  const rows = await (
    await chatDatabase()
  ).sql.query(
    'SELECT "_key", "selection" FROM "chat_bindings" ORDER BY "_key"',
  );
  return rows.map((row) => ({
    chatId: requireId(row._key, "stored chat id"),
    selection: decodeChatMarker({ version: 1, selection: row.selection })
      .selection,
  }));
}
/** Reads a supplied set of bindings in bounded SQL batches for sidebar presentation. */
export async function bindingsForChats(
  chatIds: readonly string[],
): Promise<Map<string, string>> {
  const result = new Map<string, string>(),
    sql = (await chatDatabase()).sql;
  for (let offset = 0; offset < chatIds.length; offset += 500) {
    const ids = chatIds.slice(offset, offset + 500);
    ids.forEach((id) => requireId(id, "chat id"));
    const rows = await sql.query(
      'SELECT "_key", "selection" FROM "chat_bindings" WHERE "_key" IN (' +
        ids.map(() => "?").join(",") +
        ")",
      [...ids],
    );
    for (const row of rows)
      result.set(
        String(row._key),
        decodeChatMarker({ version: 1, selection: row.selection }).selection,
      );
  }
  return result;
}

/** Filters primary owners and writable mounts by joining actual plugin relation tables. */
export async function chatsForMemoryOwner(ownerKey: string): Promise<string[]> {
  requireId(ownerKey, "memory owner");
  const rows = await (
    await chatDatabase()
  ).sql.query(
    `
    SELECT DISTINCT b."_key" AS chatId FROM "chat_bindings" b
    JOIN "cards" c ON b."selection" = 'card:' || c."id"
      OR EXISTS (SELECT 1 FROM "groups_members" m WHERE b."selection" = 'group:' || m."_root" AND m."characterCardId" = c."id")
    WHERE (c."memoryBindingMode" = 'CHARACTER' AND 'character:' || c."id" = ?)
      OR (c."memoryBindingMode" = 'SHARED' AND 'shared:' || c."sharedMemoryId" = ?)
      OR EXISTS (SELECT 1 FROM "cards_sharedMemoryMounts" mount WHERE mount."_root" = c."_key" AND mount."writable" = 1 AND 'shared:' || mount."sharedMemoryId" = ?)
    ORDER BY chatId`,
    [ownerKey, ownerKey, ownerKey],
  );
  return rows.map((row) => requireId(row.chatId, "selected chat id"));
}
/** Writes a validated selection with an exact SQL revision precondition. */
export async function writeChatBinding(
  binding: ChatBinding,
): Promise<ChatBinding> {
  requireId(binding.chatId, "chat id");
  decodeChatMarker({ version: 1, selection: binding.selection });
  const sql = await chatDatabase(),
    existing = await sql.get("chat_bindings", binding.chatId);
  await sql.commit([
    {
      op: "put",
      collection: "chat_bindings",
      key: binding.chatId,
      value: { version: 1, selection: binding.selection },
      expectedVersion: existing === null ? null : existing.version,
    },
  ]);
  return binding;
}

/** Prepares a binding under an allocated draft identity without publishing it in chat lists. */
export async function prepareChatBinding(binding: ChatBinding): Promise<void> {
  requireId(binding.chatId, "draft id");
  decodeChatMarker({ version: 1, selection: binding.selection });
  await (
    await chatDatabase()
  ).commit([
    {
      op: "put",
      collection: "chat_drafts",
      key: binding.chatId,
      value: { version: 1, selection: binding.selection },
      expectedVersion: null,
    },
  ]);
}
/** Resolves only the explicitly prepared draft binding. */
export async function readDraftBinding(chatId: string): Promise<ChatBinding> {
  const row = await (
    await chatDatabase()
  ).get("chat_drafts", requireId(chatId, "draft id"));
  if (row === null)
    throw new Error("Chat draft binding is unavailable: " + chatId);
  return { chatId, selection: decodeChatMarker(row.value).selection };
}
/** Publishes a prepared binding after Core's chat commit, with both SQL changes in one transaction. */
export async function commitDraftBinding(chatId: string): Promise<void> {
  const database = await chatDatabase(),
    draft = await database.get("chat_drafts", requireId(chatId, "draft id"));
  if (draft === null)
    throw new Error("Chat draft binding is unavailable: " + chatId);
  await database.commit([
    {
      op: "put",
      collection: "chat_bindings",
      key: chatId,
      value: draft.value,
      expectedVersion: null,
    },
    {
      op: "delete",
      collection: "chat_drafts",
      key: chatId,
      expectedVersion: draft.version,
    },
  ]);
}
/** Aborts an allocated draft without changing a committed conversation binding. */
export async function abortDraftBinding(chatId: string): Promise<void> {
  const database = await chatDatabase(),
    draft = await database.get("chat_drafts", requireId(chatId, "draft id"));
  if (draft === null)
    throw new Error("Chat draft binding is unavailable: " + chatId);
  await database.commit([
    {
      op: "delete",
      collection: "chat_drafts",
      key: chatId,
      expectedVersion: draft.version,
    },
  ]);
}
/** Deletes the plugin binding while retaining Core conversations and immutable historical snapshots. */
export async function deleteChatBinding(
  chatId: string,
): Promise<{ chatId: string; deleted: boolean }> {
  requireId(chatId, "chat id");
  const sql = await chatDatabase(),
    existing = await sql.get("chat_bindings", chatId);
  if (existing === null) return { chatId, deleted: false };
  await sql.commit([
    {
      op: "delete",
      collection: "chat_bindings",
      key: chatId,
      expectedVersion: existing.version,
    },
  ]);
  return { chatId, deleted: true };
}
