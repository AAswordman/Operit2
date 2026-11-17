import type { Storage } from "../../../../types/storage.js";

import type { ChannelMessage } from "./channel-api.js";

/** The key includes the account and conversation so identifiers from different sources do not collide. */
export function messageKey(parts: string[]): string {
  if (parts.some(part => typeof part !== "string" || part.length === 0)) throw new Error("Channel message identity is incomplete");
  const key = parts.map(part => encodeURIComponent(part)).join(":");
  if (key.length > 1000) throw new Error("Channel message identity is too long");
  return key;
}

/** Independent durable inbox. Acknowledging this copy never consumes the Bot's reply queue. */
export class ChannelInbox {
  private database: Promise<Storage.ObjectDatabase> | null = null;
  private open(): Promise<Storage.ObjectDatabase> {
    if (this.database === null) {
      const opening = Tools.Storage.objects.open({ path: ToolPkg.getLocalDataDir() + "/channel_inbox.sqlite" });
      this.database = opening.catch(error => { this.database = null; throw error; });
    }
    return this.database;
  }
  async record(message: ChannelMessage): Promise<void> {
    const db = await this.open(), seen = db.collection<{ key: string; message?: ChannelMessage }>("seen");
    if (await seen.get(message.key) !== null) return;
    // Only retain the routing information needed after acknowledgement, not
    // user content, raw payloads or WeChat context tokens.
    const route: ChannelMessage = { key: message.key, channel: message.channel, accountId: message.accountId,
      messageId: message.messageId, conversationId: message.conversationId, authorId: "", text: "", timestamp: "",
      ...(message.referenceIndex === undefined ? {} : { referenceIndex: message.referenceIndex }),
      context: message.channel === "qq" ? { scene: message.context.scene } : {} };
    try {
      await db.commit([
        { op: "put", collection: "seen", key: message.key, value: { key: message.key, message: route } as unknown as Storage.Value, expectedVersion: null },
        { op: "put", collection: "pending", key: message.key, value: message as unknown as Storage.Value, expectedVersion: null },
      ]);
    } catch (error) {
      // A racing duplicate is harmless; other storage failures must stop the listener.
      if (await seen.get(message.key) === null) throw error;
    }
  }
  async peek(limit = 20, conversationId?: string): Promise<ChannelMessage[]> {
    if (!Number.isSafeInteger(limit) || limit < 1 || limit > 100) throw new Error("limit must be an integer from 1 to 100");
    if (conversationId !== undefined && (typeof conversationId !== "string" || conversationId.trim() === "")) throw new Error("conversationId must be non-empty text");
    const pending = (await this.open()).collection<ChannelMessage>("pending"), matches: ChannelMessage[] = [];
    let after: string | null = null;
    do {
      const page = await pending.list(after, 100);
      for (const entry of page) {
        if (conversationId === undefined || entry.value.conversationId === conversationId) matches.push(entry.value);
        if (matches.length === limit) return matches;
      }
      if (page.length < 100) break;
      after = page[page.length - 1]!.key;
    } while (true);
    return matches;
  }
  async get(key: string): Promise<ChannelMessage | null> {
    if (typeof key !== "string" || key.length === 0) throw new Error("key must be non-empty text");
    const db = await this.open();
    const pending = await db.collection<ChannelMessage>("pending").get(key);
    if (pending !== null) return pending.value;
    // New records retain their authenticated source for deferred replies after ack.
    // Legacy seen-only records have no reply context and remain unavailable.
    return (await db.collection<{key: string; message?: ChannelMessage}>("seen").get(key))?.value.message ?? null;
  }
  async ack(key: string): Promise<boolean> {
    if (typeof key !== "string" || key.length === 0) throw new Error("key must be non-empty text");
    const pending = (await this.open()).collection<ChannelMessage>("pending"), entry = await pending.get(key);
    if (entry === null) return false;
    await pending.remove(key, { expectedVersion: entry.version });
    return true;
  }
}
