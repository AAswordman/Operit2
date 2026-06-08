"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.PROMPT_TURN_KIND = exports.INPUT_MENU_TOGGLE_ACTION = exports.TOGGLE_IDS = exports.HOOK_IDS = exports.ENV_KEYS = exports.FLOOR_OPTIONS = exports.DEFAULT_LIMITER_ENABLED = exports.DEFAULT_FLOOR_LIMIT = void 0;
exports.DEFAULT_FLOOR_LIMIT = 5;
exports.DEFAULT_LIMITER_ENABLED = true;
exports.FLOOR_OPTIONS = [3, 5, 8, 10, 15, 20, 30, 50, 100];
exports.ENV_KEYS = {
    floorLimit: "CTX_LIMITER_FLOOR_LIMIT",
    enabled: "CTX_LIMITER_ENABLED",
};
exports.HOOK_IDS = {
    finalize: "ctx_limiter_finalize",
    menu: "ctx_limiter_menu",
};
exports.TOGGLE_IDS = {
    limiter: "ctx_limiter_toggle",
    adjust: "ctx_limiter_adjust",
};
exports.INPUT_MENU_TOGGLE_ACTION = {
    create: "create",
    toggle: "toggle",
};
exports.PROMPT_TURN_KIND = {
    SYSTEM: "SYSTEM",
    USER: "USER",
    ASSISTANT: "ASSISTANT",
};
