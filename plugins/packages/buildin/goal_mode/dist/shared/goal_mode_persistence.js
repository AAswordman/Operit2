"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.isGoalInputSlotEnabled = isGoalInputSlotEnabled;
exports.setGoalInputSlotEnabled = setGoalInputSlotEnabled;
exports.readPersistedGoal = readPersistedGoal;
exports.writePersistedGoal = writePersistedGoal;
exports.deletePersistedGoal = deletePersistedGoal;
exports.watchPersistedGoal = watchPersistedGoal;
const goal_mode_state_js_1 = require("./goal_mode_state.js");
let database = null;
/** Opens the goal package's synchronized database without consulting chat workspaces. */
function openGoalDatabase() {
    if (database === null) {
        database = Tools.Storage.objects.open({ path: ToolPkg.getSpaceDataDir() + "/goals.sqlite" });
    }
    return database;
}
/** Reads the synchronized one-message goal input flag for a chat. */
async function isGoalInputSlotEnabled(chatId) {
    const entry = await (await openGoalDatabase()).collection("inputSlots").get(chatId);
    if (entry === null)
        return false;
    if (entry.value !== true)
        throw new Error("Persisted goal input flag is invalid.");
    return true;
}
/** Persists the chat's goal input flag in the plugin-owned database. */
async function setGoalInputSlotEnabled(chatId, enabled) {
    const slots = (await openGoalDatabase()).collection("inputSlots");
    if (enabled)
        await slots.put(chatId, true);
    else
        await slots.remove(chatId);
}
/** Validates a structured goal record read from the owning plugin database. */
function parsePersistedGoal(parsed) {
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
        throw new Error("Persisted goal must be an object.");
    }
    const value = parsed;
    if (typeof value.objective !== "string" ||
        (value.status !== "active" && value.status !== "paused" && value.status !== "completed") ||
        typeof value.updatedAt !== "number" || typeof value.startedAt !== "number" ||
        typeof value.pausedDurationMs !== "number")
        throw new Error("Persisted goal has invalid fields.");
    for (const field of ["pausedAt", "completedAt", "lastReviewedMessageTimestamp"]) {
        if (value[field] !== undefined && typeof value[field] !== "number") {
            throw new Error("Persisted goal has an invalid timestamp: " + field);
        }
    }
    return value;
}
/** Reads the current chat goal and clears memory when its durable record was deleted. */
async function readPersistedGoal(chatId) {
    const entry = await (await openGoalDatabase()).collection("goals").get(chatId);
    if (entry === null) {
        (0, goal_mode_state_js_1.clearGoal)(chatId);
        return null;
    }
    return (0, goal_mode_state_js_1.restoreGoal)(chatId, parsePersistedGoal(entry.value));
}
/** Saves a goal as plain structured data with absent optional properties omitted. */
async function writePersistedGoal(chatId, goal) {
    const value = {
        objective: goal.objective, status: goal.status, updatedAt: goal.updatedAt,
        startedAt: goal.startedAt, pausedDurationMs: goal.pausedDurationMs,
    };
    if (goal.pausedAt !== undefined)
        value.pausedAt = goal.pausedAt;
    if (goal.completedAt !== undefined)
        value.completedAt = goal.completedAt;
    if (goal.lastReviewedMessageTimestamp !== undefined)
        value.lastReviewedMessageTimestamp = goal.lastReviewedMessageTimestamp;
    await (await openGoalDatabase()).collection("goals").put(chatId, value);
    return (0, goal_mode_state_js_1.restoreGoal)(chatId, goal);
}
/** Removes the chat goal from plugin storage and runtime memory. */
async function deletePersistedGoal(chatId) {
    await (await openGoalDatabase()).collection("goals").remove(chatId);
    (0, goal_mode_state_js_1.clearGoal)(chatId);
}
/** Emits current chat state after local or remote synchronized goal transactions. */
async function* watchPersistedGoal(chatId, signal) {
    const database = await openGoalDatabase();
    for await (const entry of database.collection("goals").watch(chatId, { signal })) {
        if (entry === null) {
            (0, goal_mode_state_js_1.clearGoal)(chatId);
            yield null;
        }
        else {
            yield (0, goal_mode_state_js_1.restoreGoal)(chatId, parsePersistedGoal(entry.value));
        }
    }
}
