import type { Storage } from "../../../../types/storage.js";
export interface Notice { eventId: string; messageKey: string; text: string }
type Delivery = { eventId: string; state: "sending" | "sent" | "uncertain"; startedAt: number };
/** A durable at-most-once reservation. Ambiguous network outcomes are never replayed. */
export class NotificationOutbox {
  private database: Promise<Storage.ObjectDatabase> | null = null;
  private open(): Promise<Storage.ObjectDatabase> {
    if (this.database === null) {
      const opening = Tools.Storage.objects.open({path: ToolPkg.getLocalDataDir() + "/notifications.sqlite"});
      this.database = opening.catch(error => { this.database = null; throw error; });
    }
    return this.database;
  }
  async send(notice: Notice, active: () => boolean | Promise<boolean>, deliver: () => Promise<unknown>): Promise<void> {
    if (!notice || typeof notice.eventId !== "string" || !notice.eventId || notice.eventId.length > 1000 || typeof notice.messageKey !== "string" || !notice.messageKey || typeof notice.text !== "string" || !notice.text) throw new Error("Invalid notification");
    if (!await active()) return;
    const db = await this.open(), collection = db.collection<Delivery>("deliveries");
    if (await collection.get(notice.eventId) !== null) return;
    if (!await active()) return;
    const value: Delivery = {eventId: notice.eventId, state: "sending", startedAt: Date.now()};
    try { await db.commit([{op: "put", collection: "deliveries", key: notice.eventId, value, expectedVersion: null}]); }
    catch (error) { if (await collection.get(notice.eventId) !== null) return; throw error; }
    const entry = await collection.get(notice.eventId);
    if (entry === null) throw new Error("Notification reservation is missing");
    if (!await active()) { await collection.remove(notice.eventId, {expectedVersion: entry.version}); return; }
    try {
      await deliver();
    } catch (error) {
      // No remote idempotency contract exists: a timeout might follow delivery.
      await db.commit([{op:"put", collection:"deliveries", key:notice.eventId, value:{...value,state:"uncertain"}, expectedVersion:entry.version}]);
      throw error;
    }
    await db.commit([{op:"put", collection:"deliveries", key:notice.eventId, value:{...value,state:"sent"}, expectedVersion:entry.version}]);
  }
  async status(): Promise<{sent: number; uncertain: number}> {
    const collection = (await this.open()).collection<Delivery>("deliveries");
    let after: string | null = null, sent = 0, uncertain = 0;
    while (true) {
      const page = await collection.list(after, 200);
      for (const entry of page) { if (entry.value.state === "sent") sent++; else uncertain++; }
      if (page.length < 200) return {sent, uncertain};
      after = page[page.length - 1].key;
    }
  }
}
