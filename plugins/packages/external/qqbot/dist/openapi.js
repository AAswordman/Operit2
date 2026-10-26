"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.QQOpenApi = void 0;
const model_js_1 = require("./model.js");
/** Implements QQ token acquisition and exact OpenAPI message endpoints. */
class QQOpenApi {
    /** Binds credentials to one service generation and one networking port. */
    constructor(config, http) {
        this.config = config;
        this.http = http;
        this.token = "";
        this.expiresAt = 0;
        this.pendingToken = null;
        this.sequences = new Map();
    }
    /** Parses one real API response and rejects the original unsuccessful HTTP status. */
    async json(options) {
        const response = await this.http.request({ connect_timeout: 20, read_timeout: 20, responseType: "text", follow_redirects: false, ...options });
        if (response.statusCode < 200 || response.statusCode >= 300)
            throw new Error(`QQ HTTP ${response.statusCode}: ${response.content}`);
        return (0, model_js_1.object)(JSON.parse(response.content), "QQ API response");
    }
    /** Shares an in-flight token request without repeating credential submissions. */
    async accessToken() {
        if (this.token !== "" && Date.now() < this.expiresAt - 60000)
            return this.token;
        if (this.pendingToken !== null)
            return this.pendingToken;
        this.pendingToken = this.fetchToken();
        try {
            return await this.pendingToken;
        }
        finally {
            this.pendingToken = null;
        }
    }
    /** Obtains and validates the official application access token. */
    async fetchToken() {
        const response = await this.json({ url: "https://bots.qq.com/app/getAppAccessToken", method: "POST", headers: { "Content-Type": "application/json" }, body: { appId: (0, model_js_1.nonblank)(this.config.appId, "AppID"), clientSecret: (0, model_js_1.nonblank)(this.config.appSecret, "AppSecret") } });
        const token = (0, model_js_1.nonblank)(response.access_token, "access_token");
        const expiry = (0, model_js_1.nonblank)(response.expires_in, "expires_in");
        if (!/^[1-9]\d*$/.test(expiry))
            throw new Error("expires_in must contain positive decimal seconds");
        const expiresIn = (0, model_js_1.integer)(Number(expiry), "expires_in", 1, Math.floor((Number.MAX_SAFE_INTEGER - Date.now()) / 1000));
        this.token = token;
        this.expiresAt = Date.now() + expiresIn * 1000;
        return token;
    }
    /** Sends one authenticated OpenAPI request to the explicitly selected environment. */
    async request(method, path, body) {
        const token = await this.accessToken();
        const base = this.config.useSandbox ? "https://sandbox.api.sgroup.qq.com" : "https://api.sgroup.qq.com";
        const options = { url: base + path, method, headers: { Authorization: `QQBot ${token}`, "X-Union-Appid": this.config.appId, "Content-Type": "application/json" } };
        if (body !== undefined)
            options.body = body;
        return this.json(options);
    }
    /** Reads the gateway returned for the configured QQ application. */
    async gateway() {
        const response = await this.request("GET", "/gateway/bot");
        return (0, model_js_1.nonblank)(response.url, "gateway.url");
    }
    /** Validates credentials and reports the real profile and gateway response. */
    async test() {
        return { bot: await this.request("GET", "/users/@me"), gateway: await this.request("GET", "/gateway/bot") };
    }
    /** Uploads an actual image through the official scene-specific QQ media endpoint. */
    async uploadImage(scene, target, source) {
        if (scene !== "c2c" && scene !== "group")
            throw new Error("Image upload requires a C2C or group scene");
        (0, model_js_1.nonblank)(target, "QQ target openid");
        (0, model_js_1.nonblank)(source, "image source");
        const body = { file_type: 1, srv_send_msg: false };
        if (/^https?:\/\//i.test(source))
            body.url = source;
        else {
            const path = /^file:\/\//i.test(source) ? decodeURI(source.slice(7)) : source;
            const image = await Tools.Files.readBinary(path);
            (0, model_js_1.integer)(image.size, "QQ image size", 1, 5 * 1024 * 1024);
            body.file_data = (0, model_js_1.nonblank)(image.contentBase64, "image binary data");
        }
        const path = scene === "c2c" ? `/v2/users/${encodeURIComponent(target)}/files` : `/v2/groups/${encodeURIComponent(target)}/files`;
        const uploaded = await this.request("POST", path, body);
        (0, model_js_1.nonblank)(uploaded.file_info, "uploaded image file_info");
        return uploaded;
    }
    /** Sends one uploaded image using the same passive reply sequence as adjacent text segments. */
    async sendImage(scene, target, source, messageId) {
        const uploaded = await this.uploadImage(scene, target, source);
        const fileInfo = (0, model_js_1.nonblank)(uploaded.file_info, "uploaded image file_info");
        return this.send(scene, target, { msg_type: 7, msg_id: messageId, payload: { media: { file_info: fileInfo } } });
    }
    /** Builds one authoritative message body and keeps reply sequences monotonic per source message. */
    async send(scene, target, params) {
        (0, model_js_1.nonblank)(target, "QQ target openid");
        const body = params.payload === undefined ? {} : { ...(0, model_js_1.object)(params.payload, "payload") };
        if (params.content !== undefined)
            body.content = (0, model_js_1.nonblank)(params.content, "content");
        if (params.msg_type !== undefined)
            body.msg_type = (0, model_js_1.integer)(params.msg_type, "msg_type", 0, 7);
        if (body.msg_type === undefined)
            body.msg_type = 0;
        if (params.msg_id !== undefined)
            body.msg_id = (0, model_js_1.nonblank)(params.msg_id, "msg_id");
        if (params.event_id !== undefined)
            body.event_id = (0, model_js_1.nonblank)(params.event_id, "event_id");
        if (body.msg_type === 0)
            (0, model_js_1.nonblank)(body.content, "text message content");
        const source = body.msg_id === undefined ? "" : (0, model_js_1.nonblank)(body.msg_id, "msg_id");
        const key = `${scene}:${target}:${source}`;
        const previous = this.sequences.get(key);
        const seq = params.msg_seq === undefined ? (previous === undefined ? 1 : previous + 1) : (0, model_js_1.integer)(params.msg_seq, "msg_seq", 1, Number.MAX_SAFE_INTEGER);
        body.msg_seq = seq;
        this.sequences.set(key, seq);
        if (this.sequences.size > 512)
            this.sequences.delete(this.sequences.keys().next().value);
        const path = scene === "c2c" ? `/v2/users/${encodeURIComponent(target)}/messages` : `/v2/groups/${encodeURIComponent(target)}/messages`;
        return this.request("POST", path, body);
    }
}
exports.QQOpenApi = QQOpenApi;
