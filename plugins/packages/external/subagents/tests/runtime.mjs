import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const require = createRequire(new URL("../../../buildin/workflow/package.json", import.meta.url));
const ts = require("typescript");
const sourceRoot = fileURLToPath(new URL("../src/", import.meta.url));

/** Executes actual package TypeScript as isolated CommonJS modules, with explicit host boundaries supplied by each test. */
export function modules(globals = {}, prepareModule = /** Preserves ordinary source modules before their test execution. */ source => source) {
  const cache = new Map();
  const sandbox = vm.createContext({ console, setTimeout, clearTimeout, AbortController, ...globals });
  /** Loads one exact relative source path without reading stale production dist artifacts. */
  function load(name, parent = resolve(sourceRoot, "entry.ts")) {
    const file = resolve(dirname(parent), name.replace(/\.js$/, "") + ".ts");
    if (cache.has(file)) return cache.get(file).exports;
    const module = { exports: {} };
    cache.set(file, module);
    const javascript = ts.transpileModule(readFileSync(file, "utf8"), {
      compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS },
      fileName: file,
    }).outputText;
    const execute = vm.runInContext("(function(require,module,exports){\n" + prepareModule(javascript, file) + "\n})", sandbox, { filename: file });
    execute(specifier => load(specifier, file), module, module.exports);
    return module.exports;
  }
  return { load, sandbox };
}

/** Copies records through the same plain structured-data boundary used by host persistence. */
export function copy(value) { return structuredClone(value); }

/** Advances queued JavaScript continuations without relying on wall-clock sleeps. */
export async function flush() {
  for (let index = 0; index < 40; index++) await Promise.resolve();
}

/** Controls removable deadlines and cancellation-admission waits deterministically. */
export class TestClock {
  time = 1000;
  sequence = 0;
  timers = new Map();
  /** Returns the deterministic timestamp persisted by the manager. */
  now() { return this.time; }
  /** Registers a deadline and exposes exact cleanup to wait subscriptions. */
  schedule(callback, milliseconds) {
    const id = ++this.sequence;
    this.timers.set(id, { callback, at: this.time + milliseconds });
    return () => { this.timers.delete(id); };
  }
  /** Defers admission checks through the same controlled timer queue. */
  delay(milliseconds) { return new Promise(resolve => { this.schedule(resolve, milliseconds); }); }
  /** Fires currently scheduled deadlines up to the selected timestamp. */
  advance(milliseconds) {
    const end = this.time + milliseconds;
    for (const [id, timer] of [...this.timers]) {
      if (timer.at > end) continue;
      this.time = timer.at;
      this.timers.delete(id);
      timer.callback();
    }
    this.time = end;
  }
}

/** Supplies one bounded, pull-based source whose admission, semantic parts and terminal receipt are test-controlled. */
export class Observation {
  pending = null;
  queue = [];
  closed = false;
  returnCount = 0;
  /** Implements the native single-pending-pull constraint. */
  next() {
    assert.equal(this.pending, null, "concurrent native pulls");
    if (this.queue.length !== 0) return this.deliver(this.queue.shift());
    if (this.closed) return Promise.resolve({ done: true, value: undefined });
    return new Promise((resolve, reject) => { this.pending = { resolve, reject }; });
  }
  /** Preserves genuine observation failures rather than replacing them with completion data. */
  deliver(item) {
    if (item.kind === "error") return Promise.reject(item.error);
    return Promise.resolve({ done: false, value: item.value });
  }
  /** Publishes the next explicitly supplied event to a native pull. */
  emit(value) {
    if (this.pending === null) this.queue.push({ kind: "value", value });
    else { const pending = this.pending; this.pending = null; pending.resolve({ done: false, value }); }
  }
  /** Rejects the exact pending observation with an actual native error. */
  fail(error) {
    if (this.pending === null) this.queue.push({ kind: "error", error });
    else { const pending = this.pending; this.pending = null; pending.reject(error); }
  }
  /** Releases observation without sending a cancellation request. */
  return() {
    this.closed = true;
    this.returnCount++;
    if (this.pending !== null) { this.pending.resolve({ done: true, value: undefined }); this.pending = null; }
    return Promise.resolve({ done: true, value: undefined });
  }
  /** Exposes this single observation through the ordinary iterable contract. */
  [Symbol.asyncIterator]() { return this; }
}

/** Records Chat operations while retaining native control over admission and terminal finalization. */
export class TestChat {
  created = [];
  sends = [];
  cancellations = [];
  processing = new Set();
  automaticAdmission = true;
  automaticCancellation = false;
  createGate = null;
  /** Creates a distinct real identity while preserving the requested source chat. */
  async create(parentChatId) {
    const chatId = "child-" + (this.created.length + 1);
    this.created.push({ chatId, parentChatId });
    if (this.createGate !== null) await this.createGate;
    return chatId;
  }
  /** Retains each exact originating request and its independent observation. */
  send(request) {
    const observation = new Observation();
    this.sends.push({ request: copy(request), observation });
    if (this.automaticAdmission) this.processing.add(request.chatId);
    return observation;
  }
  /** Acknowledges only an admitted send, with finalization controlled separately. */
  async cancel(chatId) {
    this.cancellations.push(chatId);
    const cancelRequested = this.processing.has(chatId);
    if (cancelRequested && this.automaticCancellation) this.complete(chatId, "cancelled");
    return { chatId, cancelRequested };
  }
  /** Reads the independently controlled native processing flag. */
  async isProcessing(chatId) { return this.processing.has(chatId); }
  /** Returns the latest actual send for an exact conversation. */
  latest(chatId) {
    const sends = this.sends.filter(send => send.request.chatId === chatId);
    assert.ok(sends.length !== 0, "missing native send for " + chatId);
    return sends[sends.length - 1];
  }
  /** Releases the native execution before publishing its real terminal receipt. */
  complete(chatId, status = "completed", response = "finished") {
    this.processing.delete(chatId);
    this.latest(chatId).observation.emit({ type: "completed", result: {
      chatId, message: this.latest(chatId).request.input.text, aiResponse: response,
      outcome: { type: "committed", status, userMessageTimestamp: 10, assistant: { messageTimestamp: 20, variantIndex: 0 } },
    } });
  }
}

/** Persists detached manager records and exposes every actual save to behavioral assertions. */
export class TestRepository {
  records = new Map();
  writes = [];
  /** Reads explicitly seeded durable records. */
  async load() { return [...this.records.values()].map(copy); }
  /** Saves immutable snapshots exactly once per requested transition. */
  async save(agent) { this.records.set(agent.id, copy(agent)); this.writes.push(copy(agent)); }
}

/** Creates the real package manager with controlled host boundaries and no alternative orchestration implementation. */
export function harness(seed = []) {
  const loaded = modules();
  const { AgentManager } = loaded.load("./manager");
  const repository = new TestRepository();
  for (const agent of seed) repository.records.set(agent.id, copy(agent));
  const chat = new TestChat();
  const clock = new TestClock();
  const errors = [];
  const manager = new AgentManager(repository, chat, clock, error => errors.push(error));
  /** Sends an exact discriminated request from the selected parent. */
  function request(action, fields = {}, parentChatId = "parent") { return manager.request({ action, parentChatId, ...fields }); }
  /** Creates a named self-contained task through the same production manager entry. */
  function spawn(fields = {}, parentChatId = "parent") { return request("spawn", { name: "Inspect", task: "Inspect the source", participantId: "participant-a", ...fields }, parentChatId); }
  return { ...loaded, manager, repository, chat, clock, errors, request, spawn };
}

/** Creates a complete structured semantic part with exact identities and attributes. */
export function part(partId, kind, content) {
  return { partId, kind, content, sequence: 0, toolCallId: null, toolName: null, attributes: {} };
}
