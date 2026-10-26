"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.QQGateway = void 0;
const model_js_1 = require("./model.js");
/** Implements the QQ Gateway protocol over one Host-owned WebSocket. */
class QQGateway {
    /** Binds the QQ API and transport to an ordered inbound-event consumer. */
    constructor(api, socket, onEvent) {
        this.api = api;
        this.socket = socket;
        this.onEvent = onEvent;
        this.status = "stopped";
        this.sessionId = "";
        this.lastError = "";
        this.receivedCount = 0;
        this.handle = null;
        this.timer = null;
        this.generation = 0;
        this.sequence = null;
        this.heartbeatInterval = 0;
        this.nextHeartbeat = 0;
        this.awaitingAck = false;
        this.helloDeadline = 0;
        this.gatewayUrl = "";
    }
    /** Discovers the application gateway and starts a single connection generation. */
    async start() {
        if (this.handle !== null || this.status === "connecting")
            throw new Error("QQ Gateway is already running");
        const generation = ++this.generation;
        this.status = "connecting";
        this.lastError = "";
        try {
            this.gatewayUrl = await this.api.gateway();
            if (generation !== this.generation)
                return;
            await this.open(generation);
            this.schedule(generation);
        }
        catch (error) {
            this.status = "error";
            this.lastError = (0, model_js_1.errorText)(error);
            console.error("[qqbot.gateway]", error);
            throw error;
        }
    }
    /** Opens a discovered gateway with a bounded HELLO and authentication deadline. */
    async open(generation) {
        const separator = this.gatewayUrl.indexOf("?") === -1 ? "?" : "&";
        const response = await this.socket.request({ op: "open", url: `${this.gatewayUrl}${separator}v=1&encoding=json`, connect_timeout_seconds: 20 });
        if (response.op !== "open")
            throw new Error("Unexpected WebSocket open response");
        if (generation !== this.generation) {
            await this.socket.request({ op: "close", handle: response.handle });
            return;
        }
        this.handle = response.handle;
        this.status = "connecting";
        this.helloDeadline = Date.now() + 20000;
        this.heartbeatInterval = 0;
        this.awaitingAck = false;
    }
    /** Releases the connection and invalidates every queued protocol continuation. */
    async stop() {
        ++this.generation;
        if (this.timer !== null) {
            clearTimeout(this.timer);
            this.timer = null;
        }
        const handle = this.handle;
        this.handle = null;
        this.status = "stopped";
        this.sessionId = "";
        this.sequence = null;
        if (handle !== null)
            await this.socket.request({ op: "close", handle });
    }
    /** Awaits READY or the exact startup error without declaring an open socket healthy. */
    async waitReady() {
        const deadline = Date.now() + 22000;
        while (this.status === "connecting" && Date.now() < deadline)
            await new Promise(resolve => { setTimeout(resolve, 100); });
        if (this.status !== "ready")
            throw new Error(this.lastError === "" ? `QQ Gateway startup ended in ${this.status}` : this.lastError);
    }
    /** Schedules one receive pass only after the prior pass has completed. */
    schedule(generation) {
        if (generation !== this.generation || this.handle === null)
            return;
        this.timer = setTimeout(() => { void this.tick(generation); }, 100);
    }
    /** Sends one JSON text frame for the currently active connection. */
    async send(payload) {
        if (this.handle === null)
            throw new Error("QQ Gateway has no active socket");
        await this.socket.request({ op: "send", handle: this.handle, text: JSON.stringify(payload) });
    }
    /** Processes ordered frames, validates heartbeats and records terminal errors. */
    async tick(generation) {
        try {
            if (generation !== this.generation || this.handle === null)
                return;
            const handle = this.handle;
            const response = await this.socket.request({ op: "read", handle });
            if (generation !== this.generation || handle !== this.handle)
                return;
            if (response.op !== "read")
                throw new Error("Unexpected WebSocket read response");
            for (const event of response.events) {
                if (generation !== this.generation || handle !== this.handle)
                    break;
                if (event.type === "error")
                    throw new Error(event.message);
                if (event.type === "closed")
                    throw new Error("QQ Gateway closed the connection");
                if (event.type === "message")
                    await this.dispatch((0, model_js_1.object)(JSON.parse(event.text), "gateway frame"), generation);
            }
            if (generation !== this.generation)
                return;
            const now = Date.now();
            if (this.status === "connecting" && now >= this.helloDeadline)
                throw new Error("QQ Gateway authentication deadline exceeded");
            if (this.heartbeatInterval > 0 && now >= this.nextHeartbeat) {
                if (this.awaitingAck)
                    throw new Error("QQ Gateway heartbeat acknowledgement deadline exceeded");
                await this.send({ op: 1, d: this.sequence });
                this.awaitingAck = true;
                this.nextHeartbeat = now + this.heartbeatInterval;
            }
            this.schedule(generation);
        }
        catch (error) {
            if (generation !== this.generation)
                return;
            this.lastError = (0, model_js_1.errorText)(error);
            console.error("[qqbot.gateway]", error);
            try {
                await this.stop();
            }
            catch (closeError) {
                console.error("[qqbot.gateway.close]", closeError);
            }
            this.status = "error";
        }
    }
    /** Handles exact QQ opcodes and server-directed session negotiation. */
    async dispatch(frame, generation) {
        const op = (0, model_js_1.integer)(frame.op, "gateway.op", 0, 99);
        if (frame.s !== null && frame.s !== undefined)
            this.sequence = (0, model_js_1.integer)(frame.s, "gateway.s", 0, Number.MAX_SAFE_INTEGER);
        switch (op) {
            case 10: {
                const hello = (0, model_js_1.object)(frame.d, "HELLO.d");
                this.heartbeatInterval = (0, model_js_1.integer)(hello.heartbeat_interval, "heartbeat_interval", 100, 300000);
                this.nextHeartbeat = Date.now() + this.heartbeatInterval;
                const token = await this.api.accessToken();
                if (generation !== this.generation)
                    return;
                if (this.sessionId !== "")
                    await this.send({ op: 6, d: { token: `QQBot ${token}`, session_id: this.sessionId, seq: this.sequence } });
                else
                    await this.send({ op: 2, d: { token: `QQBot ${token}`, intents: (1 << 25), shard: [0, 1], properties: { os: "operit", browser: "operit", device: "operit" } } });
                return;
            }
            case 0: {
                const type = (0, model_js_1.nonblank)(frame.t, "dispatch.t");
                if (type === "READY") {
                    this.sessionId = (0, model_js_1.nonblank)((0, model_js_1.object)(frame.d, "READY.d").session_id, "session_id");
                    this.status = "ready";
                    return;
                }
                if (type === "RESUMED") {
                    this.status = "ready";
                    return;
                }
                const event = (0, model_js_1.inbound)(frame);
                this.onEvent(event);
                this.receivedCount++;
                return;
            }
            case 1:
                await this.send({ op: 1, d: this.sequence });
                this.awaitingAck = true;
                this.nextHeartbeat = Date.now() + this.heartbeatInterval;
                return;
            case 11:
                this.awaitingAck = false;
                return;
            case 7:
            case 9: {
                if (op === 9) {
                    if (typeof frame.d !== "boolean")
                        throw new Error("INVALID_SESSION.d must be boolean");
                    if (!frame.d) {
                        this.sessionId = "";
                        this.sequence = null;
                    }
                }
                if (this.handle === null)
                    throw new Error("Server reconnect has no active socket");
                const handle = this.handle;
                this.handle = null;
                await this.socket.request({ op: "close", handle });
                if (generation === this.generation)
                    await this.open(generation);
                return;
            }
            default: throw new Error(`Unsupported QQ Gateway opcode ${op}`);
        }
    }
}
exports.QQGateway = QQGateway;
