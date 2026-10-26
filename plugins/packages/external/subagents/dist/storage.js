"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Repository = void 0;
exports.watchParent = watchParent;
exports.watchAgent = watchAgent;
const model_js_1 = require("./model.js");
let database = null;
/** Opens the same host-backed package database in tools, the retained owner, and UI contexts. */
function openDatabase() {
    if (database === null)
        database = Tools.Storage.objects.open({ path: ToolPkg.getSpaceDataDir() + "/subagents.sqlite" });
    return database;
}
/** Owns a serialized commit stream so concurrent child updates cannot overwrite sibling summaries. */
class Repository {
    constructor() {
        this.committed = new Map();
        this.writes = Promise.resolve();
    }
    /** Reads every bounded storage page and rejects corrupt or duplicated identities. */
    async load() {
        const collection = (await openDatabase()).collection("agents");
        const agents = [];
        let after = null;
        while (true) {
            const page = await collection.list(after, 200);
            if (page.length === 0)
                return agents;
            for (const entry of page) {
                const agent = (0, model_js_1.parseAgent)(entry.value);
                if (agent.id !== entry.key || this.committed.has(agent.id))
                    throw new Error("Subagent storage identity mismatch");
                this.committed.set(agent.id, (0, model_js_1.summary)(agent));
                agents.push(agent);
            }
            after = page[page.length - 1].key;
        }
    }
    /** Commits one immutable snapshot and the parent's list in the same host transaction. */
    save(agent) {
        const pending = this.writes.then(async () => {
            const next = new Map(this.committed);
            next.set(agent.id, (0, model_js_1.summary)(agent));
            const parent = {
                schemaVersion: 1,
                agents: [...next.values()].filter(item => item.parentChatId === agent.parentChatId),
            };
            await (await openDatabase()).commit([
                { op: "put", collection: "agents", key: agent.id, value: agent },
                { op: "put", collection: "parents", key: agent.parentChatId, value: parent },
            ]);
            this.committed.set(agent.id, (0, model_js_1.summary)(agent));
        });
        this.writes = pending;
        return pending;
    }
}
exports.Repository = Repository;
/** Observes exact parent snapshots across engines without timer-based UI polling. */
async function* watchParent(parentChatId, signal) {
    (0, model_js_1.requireText)(parentChatId, "parentChatId");
    for await (const entry of (await openDatabase()).collection("parents").watch(parentChatId, { signal })) {
        if (entry === null) {
            yield { schemaVersion: 1, agents: [] };
            continue;
        }
        const value = entry.value;
        if (value === null || typeof value !== "object" || value.schemaVersion !== 1 || !Array.isArray(value.agents))
            throw new Error("Invalid parent agent list");
        for (const item of value.agents) {
            (0, model_js_1.parseAgent)({ ...item, messages: [] });
            if (item.parentChatId !== parentChatId)
                throw new Error("Subagent parent identity mismatch");
        }
        yield value;
    }
}
/** Streams the selected agent's current semantic snapshots and final native receipt. */
async function* watchAgent(parentChatId, agentId, signal) {
    for await (const entry of (await openDatabase()).collection("agents").watch(agentId, { signal })) {
        if (entry === null)
            throw new Error("Subagent does not exist: " + agentId);
        const agent = (0, model_js_1.parseAgent)(entry.value);
        if (agent.id !== agentId || agent.parentChatId !== parentChatId)
            throw new Error("Subagent detail identity mismatch");
        yield agent;
    }
}
