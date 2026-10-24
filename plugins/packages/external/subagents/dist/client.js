"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Agents = void 0;
const model_js_1 = require("./model.js");
const storage_js_1 = require("./storage.js");
/** Calls the retained owner instead of storing task control inside a tool or mounted screen. */
function call(request) {
    return ToolPkg.ipc.call(model_js_1.CHANNEL, request, { targetRuntime: "main" });
}
/** Provides typed operations for tools and UI over one explicit IPC protocol. */
exports.Agents = {
    /** Creates a child using an explicit source chat and execution participant. */
    async spawn(parentChatId, name, task, participantId) {
        return await call({ action: "spawn", parentChatId, name, task, participantId });
    },
    /** Returns the currently retained children of exactly one parent chat. */
    async list(parentChatId) {
        return await call({ action: "list", parentChatId });
    },
    /** Waits for any selected turn to finish without cancelling work on timeout. */
    async wait(parentChatId, agentIds, timeoutMs) {
        return await call({ action: "wait", parentChatId, agentIds, timeoutMs });
    },
    /** Sends another turn and explicitly controls interruption of an active one. */
    async send(parentChatId, agentId, message, interrupt) {
        return await call({ action: "send", parentChatId, agentId, message, interrupt });
    },
    /** Closes a task and its descendants after cancelling their actual executions. */
    async close(parentChatId, agentId) {
        return await call({ action: "close", parentChatId, agentId });
    },
    /** Initializes the owner once and subscribes to synchronized composer summaries. */
    async *watch(parentChatId, signal) {
        const initial = await exports.Agents.list(parentChatId);
        if (signal.aborted)
            return;
        yield initial;
        yield* (0, storage_js_1.watchParent)(parentChatId, signal);
    },
    /** Subscribes only to the selected agent's latest semantic execution snapshots. */
    async *watchDetail(parentChatId, agentId, signal) {
        await exports.Agents.list(parentChatId);
        yield* (0, storage_js_1.watchAgent)(parentChatId, agentId, signal);
    },
};
