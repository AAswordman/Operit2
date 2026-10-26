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
exports.PlanModeShared = void 0;
const plan_mode_handoff_js_1 = require("./plan_mode_handoff.js");
const plan_mode_mode_js_1 = require("./plan_mode_mode.js");
const plan_mode_plan_file_js_1 = require("./plan_mode_plan_file.js");
const plan_mode_state_js_1 = require("./plan_mode_state.js");
/** Shares device UI view bookkeeping through the existing main-context IPC boundary. */
function Shared(channel) {
    /** Registers the implemented method through a standard TypeScript initializer. */
    return function (implementation, context) {
        context.addInitializer(/** Registers local view bookkeeping on its owning service. */ function () {
            const receiver = this;
            ToolPkg.ipc.on(channel, async (args) => implementation.apply(receiver, args));
        });
        /** Invokes the view method in the owning main context. */
        return async function (...args) { return ToolPkg.ipc.call(channel, args, { targetRuntime: "main" }); };
    };
}
let PlanModeShared = (() => {
    var _a;
    let _staticExtraInitializers = [];
    let _static_getSingleActiveChatView_decorators;
    let _static_getTrackedChatViewByChatId_decorators;
    let _static_isEnabled_decorators;
    let _static_enable_decorators;
    let _static_disable_decorators;
    let _static_hasPlanFile_decorators;
    let _static_writePlanFile_decorators;
    let _static_isPlanStarted_decorators;
    let _static_claimPlanStart_decorators;
    let _static_forgetPlanStart_decorators;
    let _static_upsertTrackedChatView_decorators;
    let _static_removeTrackedChatView_decorators;
    let _static_watchStarted_decorators;
    let _static_watchPlan_decorators;
    return _a = class PlanModeShared {
            /** Reads the most recently active chat view from the owning runtime. */
            static async getSingleActiveChatView() {
                return (0, plan_mode_state_js_1.readSingleActiveChatView)();
            }
            /** Reads the tracked chat view identified by its chat id. */
            static async getTrackedChatViewByChatId(chatId) {
                return (0, plan_mode_state_js_1.readTrackedChatViewByChatId)(chatId);
            }
            /** Reads the planning flag for a chat from the owning runtime. */
            static async isEnabled(chatId) {
                return (0, plan_mode_mode_js_1.isPlanModeEnabledForChat)(chatId);
            }
            /** Enables planning for a chat in the owning runtime. */
            static async enable(chatId) {
                await (0, plan_mode_mode_js_1.enablePlanModeForChat)(chatId);
            }
            /** Disables planning for a chat in the owning runtime. */
            static async disable(chatId) {
                await (0, plan_mode_mode_js_1.disablePlanMode)(chatId);
            }
            /** Reports whether a chat has a persisted plugin-owned plan. */
            static async hasPlanFile(chatId) {
                return await (0, plan_mode_plan_file_js_1.hasPlanFile)(chatId);
            }
            /** Persists a chat plan in synchronized plugin storage. */
            static async writePlanFile(chatId, content) {
                return await (0, plan_mode_plan_file_js_1.writePlanFile)(chatId, content);
            }
            /** Reports whether this plan was already handed off for implementation. */
            static async isPlanStarted(planContent, chatId) {
                return await (0, plan_mode_handoff_js_1.isPlanStarted)(chatId, planContent);
            }
            /** Claims implementation handoff for a plan exactly once. */
            static async claimPlanStart(planContent, chatId) {
                return await (0, plan_mode_handoff_js_1.claimPlanStart)(chatId, planContent);
            }
            /** Clears the implementation-handoff record for a plan. */
            static async forgetPlanStart(planContent, chatId) {
                await (0, plan_mode_handoff_js_1.forgetPlanStart)(chatId, planContent);
            }
            /** Stores the latest tracked chat view in the shared runtime. */
            static async upsertTrackedChatView(view) {
                await (0, plan_mode_state_js_1.upsertTrackedChatViewAsync)(view);
            }
            /** Removes a closed chat view from the shared runtime. */
            static async removeTrackedChatView(runtime, viewId) {
                await (0, plan_mode_state_js_1.removeTrackedChatViewAsync)(runtime, viewId);
            }
            /** Streams the durable handoff status for the exact normalized plan. */
            static async *watchStarted(chatId, content, signal) {
                if (!signal)
                    throw new Error("Plan stream requires its collector cancellation signal");
                yield* (0, plan_mode_handoff_js_1.watchPlanStarted)(chatId, content, signal);
            }
            /** Streams the durable plan record for one chat. */
            static async *watchPlan(chatId, signal) {
                if (!signal)
                    throw new Error("Plan stream requires its collector cancellation signal");
                yield* (0, plan_mode_plan_file_js_1.watchPlanFile)(chatId, signal);
            }
        },
        (() => {
            const _metadata = typeof Symbol === "function" && Symbol.metadata ? Object.create(null) : void 0;
            _static_getSingleActiveChatView_decorators = [Shared("plan_mode.get_single_active_chat_view")];
            _static_getTrackedChatViewByChatId_decorators = [Shared("plan_mode.get_tracked_chat_view_by_chat_id")];
            _static_isEnabled_decorators = [ToolPkg.route({ binding: 0 })];
            _static_enable_decorators = [ToolPkg.route({ binding: 0 })];
            _static_disable_decorators = [ToolPkg.route({ binding: 0 })];
            _static_hasPlanFile_decorators = [ToolPkg.route({ binding: 0 })];
            _static_writePlanFile_decorators = [ToolPkg.route({ binding: 0 })];
            _static_isPlanStarted_decorators = [ToolPkg.route({ binding: 1 })];
            _static_claimPlanStart_decorators = [ToolPkg.route({ binding: 1 })];
            _static_forgetPlanStart_decorators = [ToolPkg.route({ binding: 1 })];
            _static_upsertTrackedChatView_decorators = [Shared("plan_mode.upsert_tracked_chat_view")];
            _static_removeTrackedChatView_decorators = [Shared("plan_mode.remove_tracked_chat_view")];
            _static_watchStarted_decorators = [ToolPkg.route({ binding: 0 })];
            _static_watchPlan_decorators = [ToolPkg.route({ binding: 0 })];
            __esDecorate(_a, null, _static_getSingleActiveChatView_decorators, { kind: "method", name: "getSingleActiveChatView", static: true, private: false, access: { has: obj => "getSingleActiveChatView" in obj, get: obj => obj.getSingleActiveChatView }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_getTrackedChatViewByChatId_decorators, { kind: "method", name: "getTrackedChatViewByChatId", static: true, private: false, access: { has: obj => "getTrackedChatViewByChatId" in obj, get: obj => obj.getTrackedChatViewByChatId }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_isEnabled_decorators, { kind: "method", name: "isEnabled", static: true, private: false, access: { has: obj => "isEnabled" in obj, get: obj => obj.isEnabled }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_enable_decorators, { kind: "method", name: "enable", static: true, private: false, access: { has: obj => "enable" in obj, get: obj => obj.enable }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_disable_decorators, { kind: "method", name: "disable", static: true, private: false, access: { has: obj => "disable" in obj, get: obj => obj.disable }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_hasPlanFile_decorators, { kind: "method", name: "hasPlanFile", static: true, private: false, access: { has: obj => "hasPlanFile" in obj, get: obj => obj.hasPlanFile }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_writePlanFile_decorators, { kind: "method", name: "writePlanFile", static: true, private: false, access: { has: obj => "writePlanFile" in obj, get: obj => obj.writePlanFile }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_isPlanStarted_decorators, { kind: "method", name: "isPlanStarted", static: true, private: false, access: { has: obj => "isPlanStarted" in obj, get: obj => obj.isPlanStarted }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_claimPlanStart_decorators, { kind: "method", name: "claimPlanStart", static: true, private: false, access: { has: obj => "claimPlanStart" in obj, get: obj => obj.claimPlanStart }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_forgetPlanStart_decorators, { kind: "method", name: "forgetPlanStart", static: true, private: false, access: { has: obj => "forgetPlanStart" in obj, get: obj => obj.forgetPlanStart }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_upsertTrackedChatView_decorators, { kind: "method", name: "upsertTrackedChatView", static: true, private: false, access: { has: obj => "upsertTrackedChatView" in obj, get: obj => obj.upsertTrackedChatView }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_removeTrackedChatView_decorators, { kind: "method", name: "removeTrackedChatView", static: true, private: false, access: { has: obj => "removeTrackedChatView" in obj, get: obj => obj.removeTrackedChatView }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_watchStarted_decorators, { kind: "method", name: "watchStarted", static: true, private: false, access: { has: obj => "watchStarted" in obj, get: obj => obj.watchStarted }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_watchPlan_decorators, { kind: "method", name: "watchPlan", static: true, private: false, access: { has: obj => "watchPlan" in obj, get: obj => obj.watchPlan }, metadata: _metadata }, null, _staticExtraInitializers);
            if (_metadata) Object.defineProperty(_a, Symbol.metadata, { enumerable: true, configurable: true, writable: true, value: _metadata });
            __runInitializers(_a, _staticExtraInitializers);
        })(),
        _a;
})();
exports.PlanModeShared = PlanModeShared;
