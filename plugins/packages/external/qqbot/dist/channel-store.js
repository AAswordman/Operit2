"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ChannelStore = void 0;
exports.conversationKey = conversationKey;
exports.consumer = consumer;
function conversationKey(channel, accountId, scene, conversationId) {
    return JSON.stringify([channel, accountId, scene, conversationId]);
}
const value = (data) => data;
const consumerKey = (consumerId, key) => JSON.stringify([consumerId, key]);
function consumer(value) {
    if (typeof value !== "string" || !/^[a-zA-Z0-9_.-]{1,100}$/.test(value))
        throw Error("Invalid channel consumer ID");
    return value;
}
/** Bounded readable history and independent durable deliveries; no raw payloads or credentials leave this store. */
class ChannelStore {
    constructor() {
        this.database = null;
        this.queue = Promise.resolve();
    }
    open() {
        return this.database ?? (this.database = Tools.Storage.objects.open({ path: ToolPkg.getLocalDataDir() + "/channel_sessions.sqlite" }).catch(error => { this.database = null; throw error; }));
    }
    serial(work) {
        const next = this.queue.catch(() => { }).then(work);
        this.queue = next;
        return next;
    }
    async get(key) {
        const found = await (await this.open()).collection("conversations").get(key);
        if (!found)
            throw Error("Bot 尚未收到这个会话的消息");
        return found.value;
    }
    async conversations(channel, accountId) {
        const collection = (await this.open()).collection("conversations"), result = [];
        let after = null;
        do {
            const rows = await collection.list(after, 100);
            result.push(...rows.map(row => row.value).filter(row => row.channel === channel && row.accountId === accountId));
            if (rows.length < 100)
                break;
            after = rows[rows.length - 1].key;
        } while (true);
        return result.sort((a, b) => b.lastMessageAt - a.lastMessageAt);
    }
    remember(conversation) {
        return this.serial(async () => {
            const db = await this.open(), old = await db.collection("conversations").get(conversation.key);
            if (old?.value.chatId === conversation.chatId)
                return;
            await db.commit([{ op: "put", collection: "conversations", key: conversation.key, value: value(old ? { ...old.value, chatId: conversation.chatId } : conversation), expectedVersion: old?.version ?? null }]);
        });
    }
    observe(message, scene, title, chatId) {
        return this.serial(async () => {
            const db = await this.open(), key = conversationKey(message.channel, message.accountId, scene, message.conversationId);
            const row = await db.collection("conversations").get(key);
            const history = await db.collection("history").get(key);
            const messages = history?.value ?? [];
            if (await db.collection("reply_routes").get(message.key))
                return;
            const at = Date.now();
            const readable = { messageId: message.messageId, authorId: message.authorId,
                authorName: message.channel === "qq" && "author" in message ? String(message.author?.name ?? "").slice(0, 100) : "",
                text: message.text.slice(0, 6000), timestamp: message.timestamp, receivedAt: at };
            for (const field of ["referenceIndex", "referencedMessageIndex"]) {
                const reference = message[field];
                if (typeof reference === "string")
                    readable[field] = reference;
            }
            const conversation = { key, channel: message.channel, accountId: message.accountId, scene,
                conversationId: message.conversationId, title: title.slice(0, 150), chatId: chatId ?? row?.value.chatId ?? null, lastMessageAt: at };
            const mutations = [
                { op: "put", collection: "reply_routes", key: message.key, value: value({ key: message.key, channel: message.channel, accountId: message.accountId, messageId: message.messageId,
                        conversationId: message.conversationId, authorId: "", text: "", timestamp: "", context: { scene }, ...("referenceIndex" in readable ? { referenceIndex: readable.referenceIndex } : {}) }) },
                { op: "put", collection: "conversations", key, value: value(conversation), expectedVersion: row?.version ?? null },
                { op: "put", collection: "history", key, value: value([...messages, readable].slice(-80)), expectedVersion: history?.version ?? null },
            ];
            const subscribers = [];
            let after = null;
            do {
                const page = await db.collection("subscribers").list(after, 100);
                subscribers.push(...page);
                if (page.length < 100)
                    break;
                after = page[page.length - 1].key;
            } while (true);
            for (const subscriber of subscribers) {
                if (!subscriber.value.routes.some(route => route.conversationKey === key && at >= route.since))
                    continue;
                mutations.push({ op: "put", collection: "deliveries", key: consumerKey(subscriber.key, message.key), value: value({ conversationKey: key, receivedAt: at, message }), expectedVersion: null });
            }
            await db.commit(mutations);
        });
    }
    bind(key, chatId) {
        return this.serial(async () => {
            const db = await this.open(), row = await db.collection("conversations").get(key);
            if (!row)
                throw Error("Bot 会话不存在");
            const next = { ...row.value, chatId };
            await db.commit([{ op: "put", collection: "conversations", key, value: value(next), expectedVersion: row.version }]);
            return next;
        });
    }
    async history(key, limit = 20, before, authorId) {
        if (!Number.isSafeInteger(limit) || limit < 1 || limit > 50 || (before !== undefined && (!Number.isSafeInteger(before) || before < 0)))
            throw Error("Invalid history bounds");
        const conversation = await this.get(key);
        const all = (await (await this.open()).collection("history").get(key))?.value ?? [];
        const filtered = all.filter(item => (before === undefined || item.receivedAt < before) && (authorId === undefined || item.authorId === authorId));
        const messages = filtered.slice(-limit).map(item => ({ ...item }));
        let remaining = 12000;
        for (let index = messages.length - 1; index >= 0; index--) {
            const item = messages[index];
            const text = item.text.slice(0, remaining);
            remaining -= text.length;
            messages[index] = { ...item, text };
        }
        return { conversation, messages: messages.filter(item => item.text.length > 0), truncated: all.length === 80 || filtered.length > messages.length || remaining === 0 };
    }
    subscribe(consumerId, routes, channel, accountId) {
        consumer(consumerId);
        if (!Array.isArray(routes) || routes.length > 200 || new Set(routes.map(route => route.conversationKey)).size !== routes.length)
            throw Error("Invalid channel subscriptions");
        return this.serial(async () => {
            for (const route of routes) {
                if (!Number.isSafeInteger(route.since) || route.since < 0)
                    throw Error("Invalid subscription boundary");
                const conversation = await this.get(route.conversationKey);
                if (conversation.channel !== channel || conversation.accountId !== accountId)
                    throw Error("Subscription belongs to a different Bot account");
            }
            const db = await this.open(), row = await db.collection("subscribers").get(consumerId);
            await db.commit([{ op: "put", collection: "subscribers", key: consumerId, value: value({ routes }), expectedVersion: row?.version ?? null }]);
            let after = null;
            const prefix = JSON.stringify([consumerId]).slice(0, -1) + ",";
            do {
                const rows = await db.collection("deliveries").list(after, 100);
                for (const entry of rows)
                    if (entry.key.startsWith(prefix) && !routes.some(route => route.conversationKey === entry.value.conversationKey && entry.value.receivedAt >= route.since))
                        await db.collection("deliveries").remove(entry.key, { expectedVersion: entry.version });
                if (rows.length < 100)
                    break;
                after = rows[rows.length - 1].key;
            } while (true);
            return { subscribed: routes.length };
        });
    }
    async peek(consumerId, limit = 20, key) {
        consumer(consumerId);
        if (!Number.isSafeInteger(limit) || limit < 1 || limit > 100)
            throw Error("Invalid inbox limit");
        const db = await this.open(), result = [], prefix = JSON.stringify([consumerId]).slice(0, -1) + ",";
        const subscription = (await db.collection("subscribers").get(consumerId))?.value;
        let after = null;
        do {
            const rows = await db.collection("deliveries").list(after, 100);
            for (const row of rows)
                if (row.key.startsWith(prefix) && (key === undefined || row.value.conversationKey === key) && subscription?.routes.some(route => route.conversationKey === row.value.conversationKey && row.value.receivedAt >= route.since))
                    result.push({ message: row.value.message, receivedAt: row.value.receivedAt });
            if (rows.length < 100)
                break;
            after = rows[rows.length - 1].key;
        } while (true);
        return result.sort((a, b) => a.receivedAt - b.receivedAt).slice(0, limit).map(item => item.message);
    }
    async replyRoute(key) {
        return (await (await this.open()).collection("reply_routes").get(key))?.value ?? null;
    }
    async ack(consumerId, key) {
        consumer(consumerId);
        const collection = (await this.open()).collection("deliveries"), id = consumerKey(consumerId, key), row = await collection.get(id);
        if (!row)
            return false;
        await collection.remove(id, { expectedVersion: row.version });
        return true;
    }
}
exports.ChannelStore = ChannelStore;
