"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.MAX_ACTIVE_AGENTS = exports.DEFAULT_WAIT_MS = exports.SCHEMA_VERSION = exports.CHANNEL = exports.PACKAGE_ID = void 0;
exports.requireText = requireText;
exports.errorText = errorText;
exports.summary = summary;
exports.isActive = isActive;
exports.parseAgent = parseAgent;
exports.PACKAGE_ID = "com.operit.subagents";
exports.CHANNEL = "subagents.control";
exports.SCHEMA_VERSION = 1;
exports.DEFAULT_WAIT_MS = 30000;
exports.MAX_ACTIVE_AGENTS = 8;
/** Requires authored nonblank text without silently converting or replacing invalid input. */
function requireText(value, label) {
    if (typeof value !== "string" || value.trim() === "")
        throw new Error(label + " must be nonblank text");
    return value;
}
/** Formats an actual thrown value for the task record and diagnostics. */
function errorText(error) {
    return error instanceof Error ? error.message : String(error);
}
/** Projects a durable agent into its small composer and tool representation. */
function summary(agent) {
    const { messages: _messages, ...value } = agent;
    return value;
}
/** Separates terminal task states from execution and cancellation still in progress. */
function isActive(status) {
    return status === "starting" || status === "running" || status === "closing";
}
/** Validates durable records before they can drive controls or be displayed as execution state. */
function parseAgent(value) {
    if (typeof value !== "object" || value === null || Array.isArray(value))
        throw new Error("Invalid subagent record");
    const agent = value;
    if (agent.schemaVersion !== exports.SCHEMA_VERSION)
        throw new Error("Unsupported subagent schema version");
    for (const field of ["id", "parentChatId", "chatId", "name", "task", "lastInput"])
        requireText(agent[field], field);
    const statuses = ["starting", "running", "completed", "cancelled", "failed", "closing", "closed", "interrupted"];
    if (statuses.indexOf(agent.status) === -1)
        throw new Error("Invalid subagent status");
    if (!Number.isSafeInteger(agent.runNumber) || agent.runNumber < 1)
        throw new Error("Invalid subagent turn number");
    if (!Number.isFinite(agent.createdAt) || !Number.isFinite(agent.updatedAt))
        throw new Error("Invalid subagent timestamps");
    if (agent.participantId !== null)
        requireText(agent.participantId, "participantId");
    if (agent.error !== null && typeof agent.error !== "string")
        throw new Error("Invalid subagent error");
    if (!Array.isArray(agent.messages))
        throw new Error("Invalid subagent message snapshots");
    for (const message of agent.messages) {
        if (!Number.isSafeInteger(message.messageTimestamp) || !Number.isSafeInteger(message.revision) || !Array.isArray(message.parts))
            throw new Error("Invalid subagent message snapshot");
    }
    if (agent.result !== null) {
        if (typeof agent.result !== "object" || agent.result.outcome === null || typeof agent.result.outcome !== "object")
            throw new Error("Invalid subagent receipt");
        if (agent.result.response !== null && typeof agent.result.response !== "string")
            throw new Error("Invalid subagent response");
        const outcome = agent.result.outcome;
        if (outcome.type !== "committed" && outcome.type !== "not_persisted" && outcome.type !== "blocked" && outcome.type !== "consumed")
            throw new Error("Invalid subagent outcome");
        if ((outcome.type === "committed" || outcome.type === "not_persisted") && outcome.status !== "completed" && outcome.status !== "cancelled")
            throw new Error("Invalid native turn status");
    }
    return agent;
}
