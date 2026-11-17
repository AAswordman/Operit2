"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.NotificationOutbox = void 0;
/** A durable at-most-once reservation. Ambiguous network outcomes are never replayed. */
class NotificationOutbox {
    constructor() {
        this.database = null;
    }
    open() {
        if (this.database === null) {
            const opening = Tools.Storage.objects.open({ path: ToolPkg.getLocalDataDir() + "/notifications.sqlite" });
            this.database = opening.catch(error => { this.database = null; throw error; });
        }
        return this.database;
    }
    async send(notice, active, deliver) {
        if (!notice || typeof notice.eventId !== "string" || !notice.eventId || notice.eventId.length > 1000 || typeof notice.messageKey !== "string" || !notice.messageKey || typeof notice.text !== "string" || !notice.text)
            throw new Error("Invalid notification");
        if (!await active())
            return;
        const db = await this.open(), collection = db.collection("deliveries");
        if (await collection.get(notice.eventId) !== null)
            return;
        if (!await active())
            return;
        const value = { eventId: notice.eventId, state: "sending", startedAt: Date.now() };
        try {
            await db.commit([{ op: "put", collection: "deliveries", key: notice.eventId, value, expectedVersion: null }]);
        }
        catch (error) {
            if (await collection.get(notice.eventId) !== null)
                return;
            throw error;
        }
        const entry = await collection.get(notice.eventId);
        if (entry === null)
            throw new Error("Notification reservation is missing");
        if (!await active()) {
            await collection.remove(notice.eventId, { expectedVersion: entry.version });
            return;
        }
        try {
            await deliver();
        }
        catch (error) {
            // No remote idempotency contract exists: a timeout might follow delivery.
            await db.commit([{ op: "put", collection: "deliveries", key: notice.eventId, value: { ...value, state: "uncertain" }, expectedVersion: entry.version }]);
            throw error;
        }
        await db.commit([{ op: "put", collection: "deliveries", key: notice.eventId, value: { ...value, state: "sent" }, expectedVersion: entry.version }]);
    }
    async status() {
        const collection = (await this.open()).collection("deliveries");
        let after = null, sent = 0, uncertain = 0;
        while (true) {
            const page = await collection.list(after, 200);
            for (const entry of page) {
                if (entry.value.state === "sent")
                    sent++;
                else
                    uncertain++;
            }
            if (page.length < 200)
                return { sent, uncertain };
            after = page[page.length - 1].key;
        }
    }
}
exports.NotificationOutbox = NotificationOutbox;
