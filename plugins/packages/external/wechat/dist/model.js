"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.VERSION = exports.MAX_MEDIA_BYTES = exports.MAX_EVENTS = exports.CDN_BASE = exports.API_BASE = exports.CHANNEL = void 0;
exports.defaultConfig = defaultConfig;
exports.object = object;
exports.text = text;
exports.nonblank = nonblank;
exports.integer = integer;
exports.parseJson = parseJson;
exports.identifier = identifier;
exports.wechatUrl = wechatUrl;
exports.itemText = itemText;
exports.inbound = inbound;
exports.errorText = errorText;
exports.CHANNEL = "wechat.service";
exports.API_BASE = "https://ilinkai.weixin.qq.com";
exports.CDN_BASE = "https://novac2c.cdn.weixin.qq.com/c2c";
exports.MAX_EVENTS = 200;
exports.MAX_MEDIA_BYTES = 25 * 1024 * 1024;
exports.VERSION = "1.0.0";
/** Creates one device-local account and its durable receive queue. */
function defaultConfig() {
    return {
        account: null,
        listenerEnabled: false,
        autoReply: {
            enabled: true,
            aiTimeoutMs: 120000,
            chatGroup: "微信 Bot",
            characterSelection: "",
            assistantInstruction: "请直接回复微信用户，不要输出内部思考或工具执行过程。",
            waifuProfile: "default",
        },
        cursor: "",
        queue: [],
        seen: [],
        contexts: {},
        bindings: {},
        history: [],
        delivery: null,
    };
}
/** Rejects non-object data at protocol boundaries. */
function object(value, field) {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
        throw new Error(`${field} must be an object`);
    }
    return value;
}
/** Reads a string without silently coercing malformed protocol data. */
function text(value, field) {
    if (typeof value !== "string")
        throw new Error(`${field} must be a string`);
    return value;
}
/** Requires a nonempty identifier, credential or content field. */
function nonblank(value, field) {
    const result = text(value, field).trim();
    if (result === "")
        throw new Error(`${field} must not be blank`);
    return result;
}
/** Validates integer sizes, timer bounds and protocol enum values. */
function integer(value, field, min, max) {
    if (typeof value !== "number" ||
        !Number.isSafeInteger(value) ||
        value < min ||
        value > max) {
        throw new Error(`${field} must be an integer between ${min} and ${max}`);
    }
    return value;
}
/** Preserves decimal int64 identifiers when parsing JSON responses. */
function parseJson(source) {
    const exact = source.replace(/"(?:\\.|[^"\\])*"|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/g, (token) => {
        if (/^-?\d+$/.test(token) && !Number.isSafeInteger(Number(token)))
            return JSON.stringify(token);
        return token;
    });
    return object(JSON.parse(exact), "WeChat response");
}
/** Accepts exact remote identifiers without rounding unsafe JSON integers. */
function identifier(value, field) {
    if (typeof value === "number")
        return String(integer(value, field, 0, Number.MAX_SAFE_INTEGER));
    return nonblank(value, field);
}
/** Restricts credential-bearing API and media requests to WeChat-owned HTTPS hosts. */
function wechatUrl(value, field) {
    const url = nonblank(value, field);
    if (!/^https:\/\/(?:[a-z0-9-]+\.)*weixin\.qq\.com(?::443)?(?:[/?][^\s#]*)?$/i.test(url)) {
        throw new Error(`${field} must use a WeChat HTTPS host`);
    }
    return url.replace(/\/$/, "");
}
/** Extracts visible text and explicit quoted context from one message item. */
function itemText(item) {
    if (item.type === 1)
        return text(object(item.text_item, "text_item").text, "text_item.text");
    if (item.type === 3) {
        const voice = object(item.voice_item, "voice_item");
        return voice.text === undefined ? "" : text(voice.text, "voice_item.text");
    }
    return "";
}
/** Validates a complete incoming user message before committing its receive cursor. */
function inbound(raw) {
    if (raw.group_id !== undefined && raw.group_id !== "")
        throw new Error("Unsupported group message");
    if (!Array.isArray(raw.item_list) || raw.item_list.length > 32)
        throw new Error("Invalid WeChat item_list");
    const items = raw.item_list.map((item) => object(item, "message item"));
    return {
        messageId: identifier(raw.message_id, "message_id"),
        userId: nonblank(raw.from_user_id, "from_user_id"),
        text: items
            .map(itemText)
            .filter((value) => value !== "")
            .join("\n"),
        timestamp: integer(raw.create_time_ms, "create_time_ms", 0, Number.MAX_SAFE_INTEGER),
        items,
        contextToken: nonblank(raw.context_token, "context_token"),
        raw,
    };
}
/** Removes known credentials and request URLs from operational diagnostics. */
function errorText(error, secrets = []) {
    let result = error instanceof Error ? error.message : String(error);
    result = result.replace(/https:\/\/[^\s"<>]+/g, "[request URL]");
    for (const secret of secrets) {
        if (secret !== "")
            result = result.split(secret).join("[redacted]");
    }
    return result.slice(0, 1000);
}
