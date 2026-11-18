import type { WebSocketResultData } from "../../../../types/results";
import type { Net } from "../../../../types/network";
import { errorText, inbound, integer, nonblank, object, type InboundEvent, type JsonObject } from "./model.js";
import { QQOpenApi } from "./openapi.js";

export interface SocketPort {
  /** Submits one typed operation to the engine-owned networking resource. */
  request(request: Net.WebSocketRequest): Promise<WebSocketResultData>;
}

class GatewayTransportError extends Error {}

/** Implements the QQ Gateway protocol over one Host-owned WebSocket. */
export class QQGateway {
  status: "stopped" | "connecting" | "ready" | "error" = "stopped";
  sessionId = "";
  lastError = "";
  receivedCount = 0;
  private handle: string | null = null;
  private timer: number | null = null;
  private generation = 0;
  private sequence: number | null = null;
  private heartbeatInterval = 0;
  private nextHeartbeat = 0;
  private awaitingAck = false;
  private helloDeadline = 0;
  private gatewayUrl = "";
  private reconnectAttempts = 0;

  /** Binds the QQ API and transport to an ordered inbound-event consumer. */
  constructor(private readonly api: QQOpenApi, private readonly socket: SocketPort, private readonly onEvent: (event: InboundEvent) => void | Promise<void>) {}

  /** Discovers the application gateway and starts a single connection generation. */
  async start(): Promise<void> {
    if (this.handle !== null || this.status === "connecting") throw new Error("QQ Gateway is already running");
    const generation = ++this.generation;
    this.status = "connecting";
    this.lastError = "";
    try {
      this.gatewayUrl = await this.api.gateway();
      if (generation !== this.generation) return;
      await this.open(generation);
      this.schedule(generation);
    } catch (error) {
      if (generation !== this.generation) return;
      this.status = "error";
      this.lastError = errorText(error);
      console.error("[qqbot.gateway]", error);
      throw error;
    }
  }

  /** Opens a discovered gateway with a bounded HELLO and authentication deadline. */
  private async open(generation: number): Promise<void> {
    const separator = this.gatewayUrl.indexOf("?") === -1 ? "?" : "&";
    const response = await this.socket.request({ op: "open", url: `${this.gatewayUrl}${separator}v=1&encoding=json`, connect_timeout_seconds: 20 });
    if (response.op !== "open") throw new Error("Unexpected WebSocket open response");
    if (generation !== this.generation) { await this.socket.request({ op: "close", handle: response.handle }); return; }
    this.handle = response.handle;
    this.status = "connecting";
    this.helloDeadline = Date.now() + 20000;
    this.heartbeatInterval = 0;
    this.awaitingAck = false;
  }

  /** Releases the connection and invalidates every queued protocol continuation. */
  async stop(): Promise<void> {
    ++this.generation;
    if (this.timer !== null) { clearTimeout(this.timer); this.timer = null; }
    const handle = this.handle;
    this.handle = null;
    this.status = "stopped";
    this.sessionId = "";
    this.sequence = null;
    this.reconnectAttempts = 0;
    if (handle !== null) await this.socket.request({ op: "close", handle });
  }

  /** Awaits READY or the exact startup error without declaring an open socket healthy. */
  async waitReady(): Promise<void> {
    const deadline = Date.now() + 22000;
    while (this.status === "connecting" && Date.now() < deadline) await new Promise<void>(resolve => { setTimeout(resolve, 100); });
    if (this.status !== "ready") throw new Error(this.lastError === "" ? `QQ Gateway startup ended in ${this.status}` : this.lastError);
  }

  /** Schedules one receive pass only after the prior pass has completed. */
  private schedule(generation: number): void {
    if (generation !== this.generation || this.handle === null) return;
    this.timer = setTimeout(() => { void this.tick(generation); }, 100);
  }

  /** Sends one JSON text frame for the currently active connection. */
  private async send(payload: JsonObject): Promise<void> {
    if (this.handle === null) throw new Error("QQ Gateway has no active socket");
    try {
      await this.socket.request({ op: "send", handle: this.handle, text: JSON.stringify(payload) });
    } catch (error) {
      throw new GatewayTransportError(errorText(error));
    }
  }

  /** Processes ordered frames, validates heartbeats and records terminal errors. */
  private async tick(generation: number): Promise<void> {
    try {
      if (generation !== this.generation || this.handle === null) return;
      const handle = this.handle;
      const response = await this.socket.request({ op: "read", handle }).catch(error => {
        throw new GatewayTransportError(errorText(error));
      });
      if (generation !== this.generation || handle !== this.handle) return;
      if (response.op !== "read") throw new Error("Unexpected WebSocket read response");
      for (const event of response.events) {
        if (generation !== this.generation || handle !== this.handle) break;
        if (event.type === "error") throw new GatewayTransportError(event.message);
        if (event.type === "closed") throw new GatewayTransportError("QQ Gateway closed the connection");
        if (event.type === "message") await this.dispatch(object(JSON.parse(event.text), "gateway frame"), generation);
      }
      if (generation !== this.generation) return;
      const now = Date.now();
      if (this.status === "connecting" && now >= this.helloDeadline) throw new Error("QQ Gateway authentication deadline exceeded");
      if (this.heartbeatInterval > 0 && now >= this.nextHeartbeat) {
        if (this.awaitingAck) throw new GatewayTransportError("QQ Gateway heartbeat acknowledgement deadline exceeded");
        await this.send({ op: 1, d: this.sequence });
        this.awaitingAck = true;
        this.nextHeartbeat = now + this.heartbeatInterval;
      }
      this.schedule(generation);
    } catch (error) {
      if (generation !== this.generation) return;
      if (error instanceof GatewayTransportError && this.sessionId !== "") {
        await this.reconnect(generation, error);
        return;
      }
      this.lastError = errorText(error);
      console.error("[qqbot.gateway]", error);
      try { await this.stop(); } catch (closeError) { console.error("[qqbot.gateway.close]", closeError); }
      // stop() invalidates the generation. Do not overwrite a newer start.
      if (this.generation === generation + 1) this.status = "error";
    }
  }

  /** Keeps the authenticated session and resumes after transient transport loss. */
  private async reconnect(generation: number, error: unknown): Promise<void> {
    if (generation !== this.generation) return;
    this.lastError = errorText(error);
    this.status = "connecting";
    const handle = this.handle;
    this.handle = null;
    if (handle !== null) {
      try { await this.socket.request({ op: "close", handle }); }
      catch (closeError) { console.error("[qqbot.gateway.close]", closeError); }
    }
    if (generation !== this.generation) return;
    const delay = 1000 * 2 ** Math.min(this.reconnectAttempts++, 5);
    this.timer = setTimeout(() => { void this.reopen(generation); }, delay);
  }

  private async reopen(generation: number): Promise<void> {
    if (generation !== this.generation) return;
    try {
      await this.open(generation);
      this.schedule(generation);
    } catch (error) {
      await this.reconnect(generation, error);
    }
  }

  private markReady(): void {
    this.status = "ready";
    this.lastError = "";
    this.reconnectAttempts = 0;
  }

  /** Handles exact QQ opcodes and server-directed session negotiation. */
  private async dispatch(frame: JsonObject, generation: number): Promise<void> {
    const op = integer(frame.op, "gateway.op", 0, 99);
    const sequence = frame.s !== null && frame.s !== undefined ? integer(frame.s, "gateway.s", 0, Number.MAX_SAFE_INTEGER) : null;
    if (op !== 0 && sequence !== null) this.sequence = sequence;
    switch (op) {
      case 10: {
        const hello = object(frame.d, "HELLO.d");
        this.heartbeatInterval = integer(hello.heartbeat_interval, "heartbeat_interval", 100, 300000);
        this.nextHeartbeat = Date.now() + this.heartbeatInterval;
        const token = await this.api.accessToken();
        if (generation !== this.generation) return;
        if (this.sessionId !== "") await this.send({ op: 6, d: { token: `QQBot ${token}`, session_id: this.sessionId, seq: this.sequence } });
        else await this.send({ op: 2, d: { token: `QQBot ${token}`, intents: (1 << 25), shard: [0, 1], properties: { os: "operit", browser: "operit", device: "operit" } } });
        return;
      }
      case 0: {
        const type = nonblank(frame.t, "dispatch.t");
        if (type === "READY") { if (sequence !== null) this.sequence = sequence; this.sessionId = nonblank(object(frame.d, "READY.d").session_id, "session_id"); this.markReady(); return; }
        if (type === "RESUMED") { if (sequence !== null) this.sequence = sequence; this.markReady(); return; }
        const event = inbound(frame);
        await this.onEvent(event);
        if (sequence !== null) this.sequence = sequence;
        this.receivedCount++;
        return;
      }
      case 1:
        await this.send({ op: 1, d: this.sequence });
        this.awaitingAck = true;
        this.nextHeartbeat = Date.now() + this.heartbeatInterval;
        return;
      case 11: this.awaitingAck = false; return;
      case 7:
      case 9: {
        if (op === 9) {
          if (typeof frame.d !== "boolean") throw new Error("INVALID_SESSION.d must be boolean");
          if (!frame.d) { this.sessionId = ""; this.sequence = null; }
        }
        if (this.handle === null) throw new Error("Server reconnect has no active socket");
        const handle = this.handle;
        this.handle = null;
        await this.socket.request({ op: "close", handle });
        if (generation === this.generation) await this.open(generation);
        return;
      }
      default: throw new Error(`Unsupported QQ Gateway opcode ${op}`);
    }
  }
}
