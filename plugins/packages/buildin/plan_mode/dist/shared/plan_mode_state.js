"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.readPlanModeStateSnapshot = readPlanModeStateSnapshot;
exports.readPlanModeStateAsync = readPlanModeStateAsync;
exports.readActiveChatViewForRuntime = readActiveChatViewForRuntime;
exports.readTrackedChatViewByChatId = readTrackedChatViewByChatId;
exports.readSingleActiveChatView = readSingleActiveChatView;
exports.upsertTrackedChatViewAsync = upsertTrackedChatViewAsync;
exports.removeTrackedChatViewAsync = removeTrackedChatViewAsync;
const state = {
    trackedViewsByChatId: {},
};
/** Clones a tracked view before it crosses the state boundary. */
function cloneTrackedChatView(view) {
    return {
        viewId: view.viewId,
        runtime: view.runtime,
        chatId: view.chatId,
        title: view.title,
        updatedAt: view.updatedAt,
    };
}
/** Clones every tracked chat view into a new lookup map. */
function cloneTrackedViewsByChatId() {
    const next = {};
    Object.entries(state.trackedViewsByChatId).forEach(([chatId, view]) => {
        next[chatId] = cloneTrackedChatView(view);
    });
    return next;
}
/** Returns an isolated snapshot of all in-memory plan state. */
function readPlanModeStateSnapshot() {
    return {
        trackedViewsByChatId: cloneTrackedViewsByChatId(),
    };
}
/** Reads plan state through the async shared-state contract. */
async function readPlanModeStateAsync() {
    return readPlanModeStateSnapshot();
}
/** Returns the tracked chat view active in one runtime. */
function readActiveChatViewForRuntime(runtime) {
    const view = Object.values(state.trackedViewsByChatId).find((item) => item.runtime === runtime);
    return view ? cloneTrackedChatView(view) : null;
}
/** Returns the tracked view associated with one chat. */
function readTrackedChatViewByChatId(chatId) {
    const view = state.trackedViewsByChatId[chatId];
    return view ? cloneTrackedChatView(view) : null;
}
/** Returns the chat view most recently reported by the host runtime. */
function readSingleActiveChatView() {
    const views = Object.values(state.trackedViewsByChatId);
    if (views.length === 0) {
        return null;
    }
    const activeView = views.reduce((latest, view) => view.updatedAt > latest.updatedAt ? view : latest);
    return cloneTrackedChatView(activeView);
}
/** Stores the latest chat view and replaces the view in its runtime. */
async function upsertTrackedChatViewAsync(view) {
    state.trackedViewsByChatId[view.chatId] = cloneTrackedChatView(view);
}
/** Removes a tracked view when its matching runtime view closes. */
async function removeTrackedChatViewAsync(runtime, viewId) {
    Object.entries(state.trackedViewsByChatId).forEach(([chatId, trackedView]) => {
        if (trackedView.runtime === runtime && trackedView.viewId === viewId) {
            delete state.trackedViewsByChatId[chatId];
        }
    });
}
