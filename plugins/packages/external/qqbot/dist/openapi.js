"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.QQOpenApi = void 0;
const messages_js_1 = require("./messages.js");
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
    /** Uploads one of QQ's four media types without consuming a message quota. */
    async uploadMedia(scene, target, media) {
        if (scene !== "c2c" && scene !== "group")
            throw new Error("Media upload requires a C2C or group scene");
        (0, model_js_1.nonblank)(target, "QQ target openid");
        const fileTypes = { image: 1, video: 2, audio: 3, file: 4 };
        if (!Object.prototype.hasOwnProperty.call(fileTypes, media.kind))
            throw new Error("Unsupported QQ media kind");
        const source = (0, model_js_1.nonblank)(media.source, "media source");
        const body = { file_type: fileTypes[media.kind], srv_send_msg: false };
        if (media.fileName !== undefined)
            body.file_name = (0, model_js_1.nonblank)(media.fileName, "file_name");
        if (/^https?:\/\//i.test(source))
            body.url = source;
        else {
            if (/^[a-z][a-z\d+.-]*:\/\//i.test(source) && !/^file:\/\//i.test(source))
                throw new Error("Media URL must use HTTP(S)");
            const path = /^file:\/\//i.test(source) ? decodeURIComponent(source.slice(7)) : source;
            const file = await Tools.Files.readBinary(path);
            (0, model_js_1.integer)(file.size, "QQ media size (direct upload maximum 5 MiB)", 1, 5 * 1024 * 1024);
            body.file_name = media.fileName === undefined ? (0, model_js_1.nonblank)(path.split(/[\\/]/).pop(), "file_name") : body.file_name;
            body.file_data = (0, model_js_1.nonblank)(file.contentBase64, "media binary data");
        }
        const path = scene === "c2c" ? `/v2/users/${encodeURIComponent(target)}/files` : `/v2/groups/${encodeURIComponent(target)}/files`;
        const uploaded = await this.request("POST", path, body);
        (0, model_js_1.nonblank)(uploaded.file_info, "uploaded media file_info");
        return uploaded;
    }
    /** Compatibility wrapper for existing image callers. */
    async uploadImage(scene, target, source) {
        return this.uploadMedia(scene, target, { kind: "image", source });
    }
    /** Uploads then sends media using the same sequencing as adjacent text messages. */
    async sendMedia(scene, target, media, params = {}) {
        // Validate the complete message before uploading bytes or contacting QQ.
        this.messageBody({ ...params, msg_type: 7, media: { file_info: "pending-upload" } });
        const uploaded = await this.uploadMedia(scene, target, media);
        return this.send(scene, target, { ...params, msg_type: 7, media: { file_info: (0, model_js_1.nonblank)(uploaded.file_info, "uploaded media file_info") } });
    }
    /** Compatibility wrapper for Waifu's image segments. */
    async sendImage(scene, target, source, messageId) {
        return this.sendMedia(scene, target, { kind: "image", source }, { msg_id: messageId });
    }
    /** Validates explicit rich attributes and strips internal envelopes only from visible fields. */
    messageBody(params) {
        if (params.msg_seq !== undefined)
            (0, model_js_1.integer)(params.msg_seq, "msg_seq", 1, Number.MAX_SAFE_INTEGER);
        const body = params.payload === undefined ? {} : { ...(0, model_js_1.object)(params.payload, "payload") };
        for (const field of ["markdown", "keyboard", "card", "media", "message_reference"]) {
            if (params[field] !== undefined)
                body[field] = (0, model_js_1.object)(params[field], field);
            if (body[field] !== undefined)
                body[field] = (0, model_js_1.object)(body[field], field);
        }
        if (params.content !== undefined)
            body.content = (0, model_js_1.text)(params.content, "content");
        if (params.msg_type !== undefined)
            body.msg_type = params.msg_type;
        if (body.msg_type === undefined)
            body.msg_type = body.media !== undefined ? 7 : body.markdown !== undefined ? 2 : body.card !== undefined ? 8 : 0;
        (0, model_js_1.integer)(body.msg_type, "msg_type", 0, 8);
        for (const field of ["msg_id", "event_id"]) {
            if (params[field] !== undefined)
                body[field] = (0, model_js_1.nonblank)(params[field], field);
            if (body[field] !== undefined)
                (0, model_js_1.nonblank)(body[field], field);
        }
        if (body.content !== undefined)
            body.content = (0, messages_js_1.externalText)((0, model_js_1.text)(body.content, "content"));
        if (body.msg_type === 0)
            (0, model_js_1.nonblank)(body.content, "text message content");
        for (const [field, type] of [["markdown", 2], ["media", 7], ["card", 8]]) {
            if (body[field] !== undefined && body.msg_type !== type)
                throw new Error(`${field} cannot be used with msg_type=${body.msg_type}`);
        }
        if (body.msg_type === 2) {
            const markdown = (0, model_js_1.object)(body.markdown, "markdown");
            if (markdown.content !== undefined)
                (0, model_js_1.nonblank)((0, messages_js_1.externalText)((0, model_js_1.text)(markdown.content, "markdown.content")), "markdown.content");
            if (body.content !== undefined && body.content !== "")
                throw new Error("Markdown messages cannot also contain text content");
            delete body.content;
        }
        if (body.msg_type === 7)
            (0, model_js_1.nonblank)((0, model_js_1.object)(body.media, "media").file_info, "media.file_info");
        if (body.msg_type === 8)
            (0, model_js_1.object)(body.card, "card");
        if (body.message_reference !== undefined) {
            const reference = (0, model_js_1.object)(body.message_reference, "message_reference");
            body.message_reference = { message_id: (0, model_js_1.nonblank)(reference.message_id, "message_reference.message_id") };
        }
        const visibleKeys = new Set(["content", "label", "visited_label", "title", "description", "desc", "prompt"]);
        const visible = (value, key = "") => {
            if (typeof value === "string")
                return visibleKeys.has(key) ? (0, messages_js_1.externalText)(value) : value;
            if (Array.isArray(value))
                return value.map(item => visible(item, key));
            if (typeof value !== "object" || value === null)
                return value;
            return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, visible(item, key)]));
        };
        for (const field of ["markdown", "keyboard", "card"])
            if (body[field] !== undefined)
                body[field] = visible(body[field]);
        return body;
    }
    /** Builds one authoritative message body and keeps reply sequences monotonic per source message. */
    async send(scene, target, params) {
        (0, model_js_1.nonblank)(target, "QQ target openid");
        if (scene !== "c2c" && scene !== "group")
            throw new Error("Sending requires a C2C or group scene");
        const body = this.messageBody(params);
        const source = body.msg_id === undefined ? "" : (0, model_js_1.nonblank)(body.msg_id, "msg_id");
        const key = JSON.stringify([scene, target, source || body.event_id || ""]);
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
