"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.WeChatApi = exports.WeChatApiError = void 0;
exports.encodeAsciiBase64 = encodeAsciiBase64;
exports.hostHttp = hostHttp;
const model_js_1 = require("./model.js");
const [majorVersion, minorVersion, patchVersion] = model_js_1.VERSION.split(".").map(Number);
const CLIENT_VERSION = String((majorVersion << 16) | (minorVersion << 8) | patchVersion);
/** Carries exact WeChat business failures without conflating them with network errors. */
class WeChatApiError extends Error {
    /** Retains the server code for explicit expired-session handling. */
    constructor(code, message) {
        super(`微信接口 ${code}: ${message}`);
        this.code = code;
        this.name = "WeChatApiError";
    }
}
exports.WeChatApiError = WeChatApiError;
/** Implements the official QR, long-poll, message and typing HTTP endpoints. */
class WeChatApi {
    /** Binds a host HTTP port and an optional authenticated account. */
    constructor(http, account = null) {
        this.http = http;
        this.account = account;
    }
    /** Builds official application headers without reusing tokens on QR requests. */
    headers(authenticated, post) {
        const headers = {
            "iLink-App-Id": "bot",
            "iLink-App-ClientVersion": CLIENT_VERSION,
        };
        if (post) {
            headers["Content-Type"] = "application/json";
            headers.AuthorizationType = "ilink_bot_token";
            const random = parseInt(CryptoJS.binary.randomHex(4), 16);
            headers["X-WECHAT-UIN"] = encodeAsciiBase64(String(random));
        }
        if (authenticated)
            headers.Authorization = `Bearer ${this.requireAccount().token}`;
        return headers;
    }
    /** Requires a fully authenticated account before issuing a Bot request. */
    requireAccount() {
        if (this.account === null)
            throw new Error("请先扫码登录微信机器人。");
        return this.account;
    }
    /** Checks HTTP status and documented business error fields separately. */
    async json(options) {
        const response = await this.http.request({
            connect_timeout: 15,
            read_timeout: 45,
            follow_redirects: false,
            responseType: "text",
            ...options,
        });
        if (response.statusCode < 200 || response.statusCode >= 300)
            throw new Error(`微信 HTTP ${response.statusCode}`);
        const data = (0, model_js_1.parseJson)(response.content);
        for (const field of ["ret", "errcode"]) {
            if (data[field] === undefined)
                continue;
            const code = (0, model_js_1.integer)(data[field], field, -2147483648, 2147483647);
            if (code !== 0)
                throw new WeChatApiError(code, data.errmsg === undefined ? "请求失败" : (0, model_js_1.text)(data.errmsg, "errmsg"));
        }
        return data;
    }
    /** Requests a new QR code without sending existing credentials. */
    loginStart() {
        return this.json({
            url: `${model_js_1.API_BASE}/ilink/bot/get_bot_qrcode?bot_type=3`,
            method: "POST",
            headers: this.headers(false, true),
            body: { local_token_list: [] },
        });
    }
    /** Polls one QR session on its explicitly selected WeChat routing host. */
    loginPoll(base, qrcode, code) {
        let url = `${(0, model_js_1.wechatUrl)(base, "QR base URL")}/ilink/bot/get_qrcode_status?qrcode=${encodeURIComponent(qrcode)}`;
        if (code !== "")
            url += `&verify_code=${encodeURIComponent(code)}`;
        return this.json({
            url,
            method: "GET",
            headers: this.headers(false, false),
        });
    }
    /** Sends an authenticated JSON operation with Operit attribution metadata. */
    post(path, body = {}, timeoutSeconds = 20) {
        const account = this.requireAccount();
        return this.json({
            url: `${(0, model_js_1.wechatUrl)(account.baseUrl, "API base URL")}/ilink/bot/${path}`,
            method: "POST",
            headers: this.headers(true, true),
            read_timeout: timeoutSeconds,
            body: {
                ...body,
                base_info: { channel_version: model_js_1.VERSION, bot_agent: `Operit/${model_js_1.VERSION}` },
            },
        });
    }
    /** Fetches one server batch with an exact persistent get_updates_buf cursor. */
    updates(cursor, timeoutSeconds) {
        return this.post("getupdates", { get_updates_buf: cursor }, timeoutSeconds);
    }
    /** Delivers exactly one text or media item with a caller-selected conversation context. */
    send(userId, contextToken, item) {
        return this.post("sendmessage", {
            msg: {
                from_user_id: "",
                to_user_id: (0, model_js_1.nonblank)(userId, "user_id"),
                client_id: `operit-${CryptoJS.binary.randomHex(16)}`,
                message_type: 2,
                message_state: 2,
                context_token: (0, model_js_1.nonblank)(contextToken, "context_token"),
                item_list: [item],
            },
        }).then((response) => {
            if (response.ret !== 0)
                throw new Error("微信发送接口没有返回明确的成功回执。");
            return response;
        });
    }
    /** Sends a complete plain text message within the channel text limit. */
    sendText(userId, contextToken, content) {
        (0, model_js_1.nonblank)(content, "text");
        if (Array.from(content).length > 4000)
            throw new Error("微信单条文本不能超过 4000 个字符。");
        return this.send(userId, contextToken, {
            type: 1,
            text_item: { text: content },
        });
    }
    /** Obtains a typing ticket for the exact peer and reply context. */
    async typing(userId, contextToken, active) {
        const config = await this.post("getconfig", {
            ilink_user_id: (0, model_js_1.nonblank)(userId, "user_id"),
            context_token: (0, model_js_1.nonblank)(contextToken, "context_token"),
        });
        const ticket = (0, model_js_1.nonblank)(config.typing_ticket, "typing_ticket");
        await this.post("sendtyping", {
            ilink_user_id: userId,
            typing_ticket: ticket,
            status: active ? 1 : 2,
        });
    }
}
exports.WeChatApi = WeChatApi;
/** Encodes protocol ASCII fields without requiring browser or Node globals. */
function encodeAsciiBase64(value) {
    const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let output = "";
    for (let index = 0; index < value.length; index += 3) {
        const a = value.charCodeAt(index), b = value.charCodeAt(index + 1), c = value.charCodeAt(index + 2);
        if (a > 127 || b > 127 || c > 127)
            throw new Error("Expected ASCII input");
        const bits = (a << 16) | ((b || 0) << 8) | (c || 0);
        output += alphabet[(bits >>> 18) & 63] + alphabet[(bits >>> 12) & 63];
        output += index + 1 < value.length ? alphabet[(bits >>> 6) & 63] : "=";
        output += index + 2 < value.length ? alphabet[bits & 63] : "=";
    }
    return output;
}
/** Creates the ordinary cross-platform host networking port, including binary uploads. */
function hostHttp() {
    return {
        request: (options) => Tools.Net.http(options),
        async upload(url, base64) {
            const client = OkHttp.newBuilder()
                .connectTimeout(15000)
                .readTimeout(60000)
                .writeTimeout(60000)
                .followRedirects(false)
                .retryOnConnectionFailure(false)
                .build();
            const response = await client
                .newRequest()
                .url(url)
                .method("POST")
                .header("Content-Type", "application/octet-stream")
                .body(base64, "base64")
                .build()
                .execute();
            return response.raw;
        },
    };
}
