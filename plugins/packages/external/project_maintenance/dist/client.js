"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.tickets = void 0;
exports.call = call;
const model_js_1 = require("./model.js");
async function call(request) { return ToolPkg.ipc.call(model_js_1.CHANNEL, request, { targetRuntime: "main" }); }
exports.tickets = {
    createFromBinding: (chatId, feedback) => call({ action: "create_from_binding", chatId, feedback }),
    chats: () => call({ action: "available_chats" }),
    bindings: () => call({ action: "list_bindings" }),
    bind: (chatId, enabled) => call({ action: "bind_chat", chatId, enabled }),
    list: () => call({ action: "list" }),
    inspect: (id) => call({ action: "inspect", id }),
    get: (id) => call({ action: "get", id }),
    start: (id) => call({ action: "start", id }),
    note: (id, text) => call({ action: "add_note", id, text }),
};
