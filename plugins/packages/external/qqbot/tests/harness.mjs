import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
const require = createRequire(new URL("../../../buildin/workflow/package.json", import.meta.url));
const { buildSync } = require("esbuild");

/** Drains asynchronous protocol continuations without advancing fake time. */
export async function settle() { for (let index = 0; index < 60; index++) await Promise.resolve(); }

/** Provides deterministic timers and a wall clock for protocol and cancellation checks. */
export function clock() {
  let now = 100000, next = 0;
  const tasks = new Map();
  class ClockDate extends Date {
    /** Supplies the deterministic wall clock to production Date calls. */
    constructor(...args) { super(...(args.length === 0 ? [now] : args)); }
    /** Reads the deterministic current time. */
    static now() { return now; }
  }
  return {
    Date: ClockDate,
    /** Records a one-shot timer. */
    setTimeout(callback, delay) { const id = ++next; tasks.set(id, { at: now + delay, callback }); return id; },
    /** Removes a pending timer. */
    clearTimeout(id) { tasks.delete(id); },
    /** Advances timer callbacks in time order and drains their asynchronous work. */
    async advance(milliseconds) {
      const end = now + milliseconds;
      for (;;) {
        await settle();
        const due = [...tasks.entries()].filter(([, task]) => task.at <= end).sort((a, b) => a[1].at - b[1].at)[0];
        if (due === undefined) break;
        tasks.delete(due[0]); now = due[1].at; due[1].callback();
      }
      now = end; await settle();
    },
    /** Counts pending resources after stop or terminal failure. */
    pending() { return tasks.size; },
  };
}

/** Loads the actual plugin source in memory with explicit Host ports. */
export function load(name, globals = {}) {
  const file = fileURLToPath(new URL(`../src/${name}.ts`, import.meta.url));
  const result = buildSync({ entryPoints: [file], bundle: true, format: "cjs", platform: "neutral", target: "es2020", write: false });
  const module = { exports: {} }, errors = [];
  const context = { module, exports: module.exports, console: { log() {}, error(...args) { errors.push(args); } }, ...globals };
  vm.runInNewContext(result.outputFiles[0].text, context, { filename: file });
  return { ...module.exports, errors };
}

/** Models only the engine-owned text-socket ABI and records exact protocol frames. */
export function socketPort() {
  let next = 0;
  const queues = new Map(), sent = [], closed = [], opened = [];
  return {
    sent, closed, opened,
    /** Submits one deterministic socket operation. */
    async request(request) {
      if (request.op === "open") { const handle = `socket-${++next}`; queues.set(handle, []); opened.push(request); return { op: "open", handle }; }
      if (!queues.has(request.handle)) throw new Error("Unknown test socket");
      if (request.op === "read") return { op: "read", events: queues.get(request.handle).splice(0) };
      if (request.op === "send") { sent.push({ handle: request.handle, payload: JSON.parse(request.text) }); return { op: "send" }; }
      if (request.op === "close") { queues.delete(request.handle); closed.push(request.handle); return { op: "close" }; }
      throw new Error("Unsupported test socket operation");
    },
    /** Delivers one exact socket event on the latest connection. */
    event(event) { const handle = `socket-${next}`; if (!queues.has(handle)) throw new Error("No open test socket"); queues.get(handle).push(event); },
    /** Delivers one QQ JSON text frame. */
    frame(payload) { this.event({ type: "message", text: JSON.stringify(payload) }); },
  };
}

/** Builds one official C2C message event without identifier aliases. */
export function c2c(id = "message-1", sender = "user-1") {
  return { op: 0, s: 2, t: "C2C_MESSAGE_CREATE", id: `event-${id}`, d: { id, content: "hello", timestamp: "2026-10-24T00:00:00Z", author: { user_openid: sender } } };
}

/** Removes cross-context object prototypes for structural assertions. */
export function plain(value) { return JSON.parse(JSON.stringify(value)); }
