"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.decodeRequest = decodeRequest;
exports.receive = receive;
exports.registerService = registerService;
const manager_js_1 = require("./manager.js");
const model_js_1 = require("./model.js");
const storage_js_1 = require("./storage.js");
let manager = null;
/** Binds platform-independent execution exclusively through the current host Chat SDK. */
const chat = {
    /** Creates a persisted child without changing the visible parent chat. */
    async create(parentChatId) {
        return (await Tools.Chat.createNew({ setAsCurrentChat: false, sourceChatId: parentChatId, parentChatId })).chatId;
    },
    /** Observes the one original send through semantic message snapshots. */
    send(request) { return Tools.Chat.sendMessageStreaming(request); },
    /** Uses authenticated package ownership for native cancellation. */
    cancel(chatId) { return Tools.Chat.cancel(chatId); },
    /** Reads the structured processing flag without using the visible chat selection. */
    async isProcessing(chatId) {
        const result = await Tools.Chat.agentStatus(chatId);
        if (result.chatId !== chatId || typeof result.isProcessing !== "boolean")
            throw new Error("Invalid native chat processing result");
        return result.isProcessing;
    },
};
const clock = {
    /** Reads the host-backed JavaScript date clock. */
    now() { return Date.now(); },
    /** Returns the exact timer cleanup used by the manager's wait subscription. */
    schedule(callback, milliseconds) {
        const timer = setTimeout(callback, milliseconds);
        return () => { clearTimeout(timer); };
    },
    /** Allows native admission and cancellation callbacks to advance independently. */
    delay(milliseconds) { return new Promise(resolve => { setTimeout(resolve, milliseconds); }); },
};
/** Constructs the retained owner lazily; registration does not open storage or start agents. */
function owner() {
    if (manager === null)
        manager = new manager_js_1.AgentManager(new storage_js_1.Repository(), chat, clock, error => { console.error("[subagents]", error); });
    return manager;
}
/** Validates the exact discriminated IPC request before entering the shared manager. */
function decodeRequest(value) {
    if (value === null || typeof value !== "object" || Array.isArray(value))
        throw new Error("Subagent request must be an object");
    const input = value;
    const parentChatId = (0, model_js_1.requireText)(input.parentChatId, "parentChatId");
    switch (input.action) {
        case "list": return { action: "list", parentChatId };
        case "spawn": {
            const participantId = input.participantId === null ? null : (0, model_js_1.requireText)(input.participantId, "participantId");
            return { action: "spawn", parentChatId, task: (0, model_js_1.requireText)(input.task, "task"), name: (0, model_js_1.requireText)(input.name, "name"), participantId };
        }
        case "wait": {
            if (!Array.isArray(input.agentIds) || !input.agentIds.every(id => typeof id === "string"))
                throw new Error("agentIds must be a string array");
            if (typeof input.timeoutMs !== "number")
                throw new Error("timeoutMs must be a number");
            return { action: "wait", parentChatId, agentIds: input.agentIds, timeoutMs: input.timeoutMs };
        }
        case "send": {
            if (typeof input.interrupt !== "boolean")
                throw new Error("interrupt must be boolean");
            return { action: "send", parentChatId, agentId: (0, model_js_1.requireText)(input.agentId, "agentId"), message: (0, model_js_1.requireText)(input.message, "message"), interrupt: input.interrupt };
        }
        case "close": return { action: "close", parentChatId, agentId: (0, model_js_1.requireText)(input.agentId, "agentId") };
        default: throw new Error("Unknown subagent operation");
    }
}
/** Receives all tools and UI actions in the same package main runtime. */
async function receive(value) {
    return owner().request(decodeRequest(value));
}
/** Declares handlers during module evaluation so targeted main-runtime IPC can initialize the service. */
function registerService() {
    ToolPkg.ipc.on(model_js_1.CHANNEL, receive);
}
