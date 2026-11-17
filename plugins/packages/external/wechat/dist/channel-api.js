"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.request = request;
exports.peek = peek;
/** The request discriminant determines the response; consumers cannot invent its type. */
function request(payload) {
    return ToolPkg.callDependency("com.operit.wechat_bot", "channel.v1", payload);
}
/** Reads the durable channel inbox, optionally scoped to one conversation. */
function peek(limit = 20, conversationId) {
    return request({ action: "peek", limit, ...(conversationId === undefined ? {} : { conversationId }) });
}
