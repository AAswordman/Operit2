"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.MAX_EVENTS = exports.CHANNEL = void 0;
exports.defaultConfig = defaultConfig;
exports.object = object;
exports.text = text;
exports.nonblank = nonblank;
exports.integer = integer;
exports.errorText = errorText;
exports.inbound = inbound;
exports.CHANNEL = "qqbot.service";
exports.MAX_EVENTS = 200;
/** Creates the authored initial persisted configuration. */
function defaultConfig() {
    return { waifuProfile: "default", appId: "", appSecret: "", useSandbox: false, listenerEnabled: false,
        autoReply: { enabled: false, c2cEnabled: true, groupEnabled: true, aiTimeoutMs: 120000, chatGroup: "QQ Bot", characterSelection: "", assistantInstruction: "请直接回复 QQ 用户，不要输出内部思考或工具执行过程。" },
        bindings: {}, processed: [] };
}
/** Requires an object received at a JSON protocol boundary. */
function object(value, field) {
    if (typeof value !== "object" || value === null || Array.isArray(value))
        throw new Error(`${field} must be an object`);
    return value;
}
/** Requires a string without converting invalid protocol data. */
function text(value, field) {
    if (typeof value !== "string")
        throw new Error(`${field} must be a string`);
    return value;
}
/** Requires a nonblank credential, identifier or submitted message. */
function nonblank(value, field) {
    const result = text(value, field).trim();
    if (result === "")
        throw new Error(`${field} must not be blank`);
    return result;
}
/** Checks bounded integers before they reach timers, queues or remote APIs. */
function integer(value, field, minimum, maximum) {
    if (typeof value !== "number" || !Number.isSafeInteger(value) || value < minimum || value > maximum)
        throw new Error(`${field} must be an integer between ${minimum} and ${maximum}`);
    return value;
}
/** Formats actual thrown values for persisted runtime diagnostics. */
function errorText(error) { return error instanceof Error ? error.message : String(error); }
/** Resolves the exact QQ event schema rather than guessing identifier aliases. */
function inbound(payload) {
    const eventType = nonblank(payload.t, "dispatch.t");
    const body = object(payload.d, "dispatch.d");
    const seq = integer(payload.s, "dispatch.s", 0, Number.MAX_SAFE_INTEGER);
    let scene = "unknown", userOpenId = "", groupOpenId = "", messageId = "", content = "", timestamp = "";
    if (eventType === "C2C_MESSAGE_CREATE" || eventType === "GROUP_AT_MESSAGE_CREATE" || eventType === "GROUP_MESSAGE_CREATE") {
        scene = eventType === "C2C_MESSAGE_CREATE" ? "c2c" : "group";
        const author = object(body.author, "message.author");
        userOpenId = nonblank(scene === "c2c" ? author.user_openid : author.member_openid, "message.author openid");
        if (scene === "group")
            groupOpenId = nonblank(body.group_openid, "message.group_openid");
        messageId = nonblank(body.id, "message.id");
        if (body.content !== undefined)
            content = text(body.content, "message.content");
        timestamp = text(body.timestamp, "message.timestamp");
    }
    const eventId = payload.id === undefined ? "" : text(payload.id, "dispatch.id");
    return { scene, eventType, eventId, seq, messageId, content, timestamp, receivedAt: new Date().toISOString(), userOpenId, groupOpenId, rawPayload: payload,
        replyHint: { scene, msg_id: messageId, event_id: eventId, openid: userOpenId, group_openid: groupOpenId } };
}
