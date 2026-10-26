"use strict";
var __runInitializers = (this && this.__runInitializers) || function (thisArg, initializers, value) {
    var useValue = arguments.length > 2;
    for (var i = 0; i < initializers.length; i++) {
        value = useValue ? initializers[i].call(thisArg, value) : initializers[i].call(thisArg);
    }
    return useValue ? value : void 0;
};
var __esDecorate = (this && this.__esDecorate) || function (ctor, descriptorIn, decorators, contextIn, initializers, extraInitializers) {
    function accept(f) { if (f !== void 0 && typeof f !== "function") throw new TypeError("Function expected"); return f; }
    var kind = contextIn.kind, key = kind === "getter" ? "get" : kind === "setter" ? "set" : "value";
    var target = !descriptorIn && ctor ? contextIn["static"] ? ctor : ctor.prototype : null;
    var descriptor = descriptorIn || (target ? Object.getOwnPropertyDescriptor(target, contextIn.name) : {});
    var _, done = false;
    for (var i = decorators.length - 1; i >= 0; i--) {
        var context = {};
        for (var p in contextIn) context[p] = p === "access" ? {} : contextIn[p];
        for (var p in contextIn.access) context.access[p] = contextIn.access[p];
        context.addInitializer = function (f) { if (done) throw new TypeError("Cannot add initializers after decoration has completed"); extraInitializers.push(accept(f || null)); };
        var result = (0, decorators[i])(kind === "accessor" ? { get: descriptor.get, set: descriptor.set } : descriptor[key], context);
        if (kind === "accessor") {
            if (result === void 0) continue;
            if (result === null || typeof result !== "object") throw new TypeError("Object expected");
            if (_ = accept(result.get)) descriptor.get = _;
            if (_ = accept(result.set)) descriptor.set = _;
            if (_ = accept(result.init)) initializers.unshift(_);
        }
        else if (_ = accept(result)) {
            if (kind === "field") initializers.unshift(_);
            else descriptor[key] = _;
        }
    }
    if (target) Object.defineProperty(target, contextIn.name, descriptor);
    done = true;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.GoalService = exports.GOAL_MODE_REMOVE_VIEW_CHANNEL = exports.GOAL_MODE_TRACK_VIEW_CHANNEL = exports.GOAL_MODE_ACTIVE_VIEW_CHANNEL = void 0;
exports.registerGoalModeIpc = registerGoalModeIpc;
exports.readSingleActiveChatViewAsync = readSingleActiveChatViewAsync;
exports.upsertTrackedChatViewAsync = upsertTrackedChatViewAsync;
exports.removeTrackedChatViewAsync = removeTrackedChatViewAsync;
const goal_mode_state_js_1 = require("./goal_mode_state.js");
const goal_mode_persistence_js_1 = require("./goal_mode_persistence.js");
exports.GOAL_MODE_ACTIVE_VIEW_CHANNEL = "goal_mode.active_view";
exports.GOAL_MODE_TRACK_VIEW_CHANNEL = "goal_mode.track_view";
exports.GOAL_MODE_REMOVE_VIEW_CHANNEL = "goal_mode.remove_view";
const GOAL_MODE_MAIN_CONTEXT_KEY = "toolpkg_main:com.operit.goal_mode_bundle";
/** Returns the main execution context used by Goal Mode state handlers. */
function goalModeIpcOptions() {
    return { targetContextKey: GOAL_MODE_MAIN_CONTEXT_KEY };
}
/** Registers the shared goal-state IPC endpoints in the owning plugin runtime. */
function registerGoalModeIpc() {
    ToolPkg.ipc.on(exports.GOAL_MODE_ACTIVE_VIEW_CHANNEL, goal_mode_state_js_1.readSingleActiveChatView);
    ToolPkg.ipc.on(exports.GOAL_MODE_TRACK_VIEW_CHANNEL, goal_mode_state_js_1.upsertTrackedChatView);
    ToolPkg.ipc.on(exports.GOAL_MODE_REMOVE_VIEW_CHANNEL, (request) => {
        if (!(0, goal_mode_state_js_1.isGoalRuntime)(request.runtime)) {
            throw new Error("Goal view runtime is invalid.");
        }
        (0, goal_mode_state_js_1.removeTrackedChatView)(request.runtime, request.viewId);
    });
}
/** Reads the active chat view through the owning runtime. */
async function readSingleActiveChatViewAsync() {
    return await ToolPkg.ipc.call(exports.GOAL_MODE_ACTIVE_VIEW_CHANNEL, undefined, goalModeIpcOptions());
}
/** Tracks a chat view through the owning runtime. */
async function upsertTrackedChatViewAsync(view) {
    await ToolPkg.ipc.call(exports.GOAL_MODE_TRACK_VIEW_CHANNEL, view, goalModeIpcOptions());
}
/** Removes a closed chat view through the owning runtime. */
async function removeTrackedChatViewAsync(runtime, viewId) {
    await ToolPkg.ipc.call(exports.GOAL_MODE_REMOVE_VIEW_CHANNEL, { runtime, viewId }, goalModeIpcOptions());
}
/** Implements routed goal operations using this package's own synchronized storage. */
let GoalService = (() => {
    var _a;
    let _staticExtraInitializers = [];
    let _static_read_decorators;
    let _static_set_decorators;
    let _static_status_decorators;
    let _static_reviewed_decorators;
    let _static_clear_decorators;
    let _static_inputEnabled_decorators;
    let _static_setInput_decorators;
    let _static_watch_decorators;
    return _a = class GoalService {
            /** Reads the current durable goal. */
            static async read(chatId) { return (0, goal_mode_persistence_js_1.readPersistedGoal)(chatId); }
            /** Stores a newly established goal. */
            static async set(chatId, objective) { return (0, goal_mode_persistence_js_1.writePersistedGoal)(chatId, (0, goal_mode_state_js_1.setGoal)(chatId, objective)); }
            /** Applies a status change to the current durable goal. */
            static async status(chatId, status) {
                await (0, goal_mode_persistence_js_1.readPersistedGoal)(chatId);
                const goal = (0, goal_mode_state_js_1.setGoalStatus)(chatId, status);
                return goal ? (0, goal_mode_persistence_js_1.writePersistedGoal)(chatId, goal) : null;
            }
            /** Saves the assistant message review checkpoint. */
            static async reviewed(chatId, timestamp) {
                await (0, goal_mode_persistence_js_1.readPersistedGoal)(chatId);
                const goal = (0, goal_mode_state_js_1.markGoalMessageReviewed)(chatId, timestamp);
                return goal ? (0, goal_mode_persistence_js_1.writePersistedGoal)(chatId, goal) : null;
            }
            /** Deletes the durable goal. */
            static async clear(chatId) { await (0, goal_mode_persistence_js_1.deletePersistedGoal)(chatId); }
            /** Reads the synchronized input flag. */
            static async inputEnabled(chatId) { return (0, goal_mode_persistence_js_1.isGoalInputSlotEnabled)(chatId); }
            /** Updates the synchronized input flag. */
            static async setInput(chatId, enabled) { await (0, goal_mode_persistence_js_1.setGoalInputSlotEnabled)(chatId, enabled); }
            /** Streams the current goal and all subsequent synchronized changes. */
            static async *watch(chatId, signal) {
                if (!signal)
                    throw new Error("Goal stream requires its collector cancellation signal");
                yield* (0, goal_mode_persistence_js_1.watchPersistedGoal)(chatId, signal);
            }
        },
        (() => {
            const _metadata = typeof Symbol === "function" && Symbol.metadata ? Object.create(null) : void 0;
            _static_read_decorators = [ToolPkg.route({ binding: 0 })];
            _static_set_decorators = [ToolPkg.route({ binding: 0 })];
            _static_status_decorators = [ToolPkg.route({ binding: 0 })];
            _static_reviewed_decorators = [ToolPkg.route({ binding: 0 })];
            _static_clear_decorators = [ToolPkg.route({ binding: 0 })];
            _static_inputEnabled_decorators = [ToolPkg.route({ binding: 0 })];
            _static_setInput_decorators = [ToolPkg.route({ binding: 0 })];
            _static_watch_decorators = [ToolPkg.route({ binding: 0 })];
            __esDecorate(_a, null, _static_read_decorators, { kind: "method", name: "read", static: true, private: false, access: { has: obj => "read" in obj, get: obj => obj.read }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_set_decorators, { kind: "method", name: "set", static: true, private: false, access: { has: obj => "set" in obj, get: obj => obj.set }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_status_decorators, { kind: "method", name: "status", static: true, private: false, access: { has: obj => "status" in obj, get: obj => obj.status }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_reviewed_decorators, { kind: "method", name: "reviewed", static: true, private: false, access: { has: obj => "reviewed" in obj, get: obj => obj.reviewed }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_clear_decorators, { kind: "method", name: "clear", static: true, private: false, access: { has: obj => "clear" in obj, get: obj => obj.clear }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_inputEnabled_decorators, { kind: "method", name: "inputEnabled", static: true, private: false, access: { has: obj => "inputEnabled" in obj, get: obj => obj.inputEnabled }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_setInput_decorators, { kind: "method", name: "setInput", static: true, private: false, access: { has: obj => "setInput" in obj, get: obj => obj.setInput }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_watch_decorators, { kind: "method", name: "watch", static: true, private: false, access: { has: obj => "watch" in obj, get: obj => obj.watch }, metadata: _metadata }, null, _staticExtraInitializers);
            if (_metadata) Object.defineProperty(_a, Symbol.metadata, { enumerable: true, configurable: true, writable: true, value: _metadata });
            __runInitializers(_a, _staticExtraInitializers);
        })(),
        _a;
})();
exports.GoalService = GoalService;
