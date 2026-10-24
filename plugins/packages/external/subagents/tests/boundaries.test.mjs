import assert from "node:assert/strict";
import test from "node:test";
import vm from "node:vm";
import { compileComposeModule } from "../../../../../plugins/tools/compile_compose_module.mjs";
import { composeContextScript, wrapComposeScript } from "../../../../../tools/runtime/compose_scripts.mjs";
import { composeStreamFixture } from "../../../../../tools/tests/support/compose_stream_fixture.mjs";
import { copy, flush, harness, modules } from "./runtime.mjs";

/** Implements only the host Storage operations exercised by the production Repository. */
function objectDatabase() {
  const collections = new Map();
  const commits = [];
  const observers = new Map();
  /** Addresses exact collection identities without any alternate storage path. */
  function records(name) {
    if (!collections.has(name)) collections.set(name, new Map());
    return collections.get(name);
  }
  const database = {
    /** Reads ordered pages and exact-key subscriptions through the declared SDK surface. */
    collection(name) {
      return {
        /** Returns a primary-key-ordered bounded storage page. */
        async list(after, limit) {
          return [...records(name)].sort(([left], [right]) => left.localeCompare(right))
            .filter(([key]) => after === null || key > after).slice(0, limit)
            .map(([key, value]) => ({ key, value: copy(value), version: "1" }));
        },
        /** Emits the initial exact record and subsequent committed updates. */
        async *watch(key, { signal }) {
          const id = name + ":" + key;
          const observer = { wake: null, changed: false };
          if (!observers.has(id)) observers.set(id, new Set());
          observers.get(id).add(observer);
          /** Wakes a pending read when its owning view is disposed. */
          const abort = () => { if (observer.wake !== null) observer.wake(); };
          signal.addEventListener("abort", abort);
          try {
            while (!signal.aborted) {
              const value = records(name).get(key);
              yield value === undefined ? null : { value: copy(value), version: "1" };
              if (!signal.aborted && !observer.changed) await new Promise(resolve => { observer.wake = resolve; });
              observer.wake = null;
              observer.changed = false;
            }
          } finally {
            signal.removeEventListener("abort", abort);
            observers.get(id).delete(observer);
          }
        },
      };
    },
    /** Makes all mutations visible before waking any exact-key observer. */
    async commit(mutations) {
      commits.push(copy(mutations));
      for (const mutation of mutations) {
        assert.equal(mutation.op, "put");
        records(mutation.collection).set(mutation.key, copy(mutation.value));
      }
      for (const mutation of mutations) for (const observer of observers.get(mutation.collection + ":" + mutation.key) ?? []) {
        observer.changed = true;
        if (observer.wake !== null) observer.wake();
      }
      return { version: String(commits.length) };
    },
  };
  return { database, commits, collections, observers };
}

/** Verifies that parallel sibling commits preserve both identities and their matching detail records. */
test("real Repository atomically publishes sibling summaries and details", async () => {
  const source = harness(); const one = await source.spawn(); const two = await source.spawn();
  const storage = objectDatabase(); const paths = [];
  const runtime = modules({
    ToolPkg: { getSpaceDataDir: () => "/space/subagents" },
    Tools: { Storage: { objects: { async open(options) { paths.push(options.path); return storage.database; } } } },
  });
  const { Repository, watchParent, watchAgent } = runtime.load("./storage");
  const repository = new Repository();
  assert.equal((await repository.load()).length, 0);
  await Promise.all([repository.save(source.repository.records.get(one.id)), repository.save(source.repository.records.get(two.id))]);
  assert.deepEqual(paths, ["/space/subagents/subagents.sqlite"]);
  const parent = storage.collections.get("parents").get("parent");
  assert.deepEqual(parent.agents.map(agent => agent.id), [one.id, two.id]);
  for (const commit of storage.commits) {
    assert.equal(commit.length, 2);
    assert.equal(commit[0].collection, "agents");
    assert.equal(commit[1].collection, "parents");
    assert.equal(Object.hasOwn(commit[1].value.agents[0], "messages"), false);
  }
  const controller = new AbortController();
  const parentIterator = watchParent("parent", controller.signal);
  assert.equal((await parentIterator.next()).value.agents.length, 2);
  const detailIterator = watchAgent("parent", one.id, controller.signal);
  assert.equal((await detailIterator.next()).value.id, one.id);
  await assert.rejects(watchAgent("other-parent", one.id, controller.signal).next(), /identity mismatch/);
  controller.abort();
  await parentIterator.return(); await detailIterator.return();
  assert.equal([...storage.observers.values()].reduce((count, observers) => count + observers.size, 0), 0);
});

/** Loads the real tool exports and their actual IPC client while recording authenticated execution context. */
test("tool entry captures host origin before IPC and uses the current participant", async () => {
  const requests = [];
  let chatId = "actual-parent";
  const runtime = modules({
    getChatId: () => chatId,
    getCallerParticipantId: () => "actual-participant",
    ToolPkg: { ipc: { async call(channel, payload, options) { requests.push(copy({ channel, payload, options })); return payload; } } },
  });
  const tools = runtime.load("./tools");
  const result = tools.spawn_agent({ name: "Inspect", task: "source task", parentChatId: "spoofed" });
  chatId = "changed-parent";
  await result;
  assert.deepEqual(requests[0], {
    channel: "subagents.control", options: { targetRuntime: "main" },
    payload: { action: "spawn", parentChatId: "actual-parent", name: "Inspect", task: "source task", participantId: "actual-participant" },
  });
  await tools.wait_agent({ agent_ids: ["child"] });
  assert.equal(requests[1].payload.timeoutMs, 30000);
  await tools.send_input({ agent_id: "child", message: "hello" });
  assert.equal(requests[2].payload.interrupt, false);
  chatId = undefined;
  await assert.rejects(tools.list_agents(), /authenticated chatId/);
  assert.equal(requests.length, 3);
});

/** Exercises real main registration while denying accidental storage reads and model sends. */
test("main evaluation registers IPC and the composer slot without opening a business store", () => {
  const handlers = [];
  const slots = [];
  const runtime = modules({
    ToolPkg: { ipc: { on(channel, handler) { handlers.push({ channel, handler }); } }, registerChatComposerSlot(slot) { slots.push(slot); } },
    Tools: new Proxy({}, { get() { throw new Error("registration must not call host tools"); } }),
  });
  const main = runtime.load("./main");
  assert.equal(handlers.length, 1);
  assert.equal(handlers[0].channel, "subagents.control");
  assert.equal(main.registerToolPkg(), true);
  assert.equal(slots.length, 1);
  assert.equal(slots[0].slot, "above_input");
  assert.equal(slots[0].keepAlive, true);
  assert.equal(typeof slots[0].screen, "function");
  assert.equal(main.test_connection().passed, true);
});

/** Rejects malformed IPC input before creating any native task or silently changing operation semantics. */
test("service validates discriminated operations and authored parameter types", () => {
  const runtime = modules(); const { decodeRequest } = runtime.load("./service");
  for (const value of [null, [], {}, { action: "list", parentChatId: "" }, { action: "wait", parentChatId: "parent", agentIds: [1], timeoutMs: 1 },
    { action: "send", parentChatId: "parent", agentId: "child", message: "hello", interrupt: "true" },
    { action: "spawn", parentChatId: "parent", name: "n", task: "t" }]) {
    assert.throws(() => decodeRequest(value));
  }
  assert.equal(decodeRequest({ action: "list", parentChatId: "parent" }).parentChatId, "parent");
});

/** Proves that corrupt native or persisted outcomes cannot become apparently successful agent state. */
test("durable record validation rejects unsupported schemas and unknown outcomes", async () => {
  const h = harness(); const agent = await h.spawn();
  const { parseAgent } = h.load("./model"); const saved = h.repository.records.get(agent.id);
  assert.throws(() => parseAgent({ ...saved, schemaVersion: 9 }), /schema version/);
  assert.throws(() => parseAgent({ ...saved, result: { response: null, outcome: { type: "made-up" } } }), /outcome/);
  assert.throws(() => parseAgent({ ...saved, result: { response: null, outcome: { type: "committed", status: "pretend" } } }), /status/);
});

/** Renders the real Compose screen with retained keyed state and effect cleanup. */
function composition(screen, initialState = {}) {
  const state = new Map(Object.entries(initialState));
  const refs = new Map();
  const effects = new Map();
  const pending = [];
  const errors = [];
  const UI = new Proxy({}, {
    /** Retains concrete DSL props and child trees so tests invoke actual authored UI actions. */
    get(_target, type) { return (props = {}, children = []) => ({ type, props, children: Array.isArray(children) ? children : [children] }); },
  });
  const ctx = {
    UI, MaterialTheme: { colorScheme: { surfaceVariant: "surface", error: "error" } },
    /** Retains the initial host-injected chat identity and the screen's own keyed state. */
    useState(key, initial) { if (!state.has(key)) state.set(key, initial); return [state.get(key), value => state.set(key, value)]; },
    /** Preserves the mutable selected-agent identity across rerenders. */
    useRef(key, initial) { if (!refs.has(key)) refs.set(key, { current: initial }); return refs.get(key); },
    /** Starts effect work only after the current render and releases its previous dependency set. */
    useEffect(key, setup, dependencies) {
      const current = effects.get(key);
      if (current !== undefined && current.dependencies.length === dependencies.length && dependencies.every((value, index) => value === current.dependencies[index])) return;
      if (current !== undefined) current.cleanup();
      pending.push(() => effects.set(key, { dependencies, cleanup: setup() }));
    },
    /** Records failures that the actual screen forwards to host diagnostics. */
    reportError(error) { errors.push(error); },
  };
  /** Renders one actual production screen pass and commits its effects. */
  function render() { const tree = screen(ctx); for (const effect of pending.splice(0)) effect(); return tree; }
  /** Locates one exact DSL key without string guessing against rendered contents. */
  function find(tree, key) {
    if (tree.props.key === key) return tree;
    for (const child of tree.children) { const value = find(child, key); if (value !== null) return value; }
    return null;
  }
  /** Disposes every mounted effect just as closing a composer contribution does. */
  function dispose() { for (const effect of effects.values()) effect.cleanup(); }
  return { state, errors, render, find, dispose };
}

/** Exercises real tool, owner, Storage and UI source paths together with controlled native generation. */
test("composer embeds the native child chat and disposes observers without cancelling agents", async () => {
  const h = harness(); const child = await h.spawn();
  const storage = objectDatabase();
  const runtime = modules({
    ToolPkg: { getSpaceDataDir: () => "/space/subagents", ipc: { async call(_channel, request) { return h.manager.request(request); } } },
    Tools: { Storage: { objects: { async open() { return storage.database; } } }, Chat: { async switchTo() { throw new Error("not requested"); } } },
  });
  const { Repository } = runtime.load("./storage"); const repository = new Repository(); await repository.load();
  await repository.save(h.repository.records.get(child.id));
  const Screen = runtime.load("./ui/panel.ui").default;
  const ui = composition(Screen, { chatId: "parent" });
  ui.render(); await flush();
  let tree = ui.render();
  assert.equal(ui.find(tree, "agent-" + child.id), null, "the initial header must stay compact");
  assert.equal(ui.find(tree, "subagents.history").type, "IconButton");
  assert.equal(ui.find(tree, "subagents.create").type, "IconButton");
  ui.find(tree, "subagents.expand").props.onClick();
  tree = ui.render();
  assert.ok(ui.find(tree, "agent-" + child.id));
  ui.find(tree, "agent-" + child.id).props.onClick();
  ui.render(); await flush();
  const detailed = copy(h.repository.records.get(child.id));
  detailed.status = "running";
  detailed.messages = [{ messageTimestamp: 20, revision: 1, parts: [{ partId: "tool", sequence: 0, kind: "tool_call", content: "actual invocation", toolName: "read_file", toolCallId: "call-a", attributes: { path: "/source" } }] }];
  await repository.save(detailed); await flush();
  tree = ui.render();
  assert.ok(ui.find(tree, "subagents.detail-dialog"));
  const execution = ui.find(tree, "subagents.execution");
  assert.equal(execution.type, "AiChat");
  assert.equal(execution.props.chatId, child.chatId);
  assert.equal(execution.props.showInput, false);
  assert.equal(execution.children.length, 0);
  assert.equal(ui.find(tree, "subagents.send").props.enabled, false);
  assert.equal(ui.find(tree, "subagents.wait").props.enabled, true);
  const writesBeforeDispose = storage.commits.length;
  ui.dispose(); await flush();
  assert.equal(h.chat.cancellations.length, 0);
  assert.equal(storage.commits.length, writesBeforeDispose);
  assert.equal([...storage.observers.values()].reduce((count, values) => count + values.size, 0), 0);
  assert.equal(ui.errors.length, 0);
});

/** Exercises the real service adapter to distinguish native parent identity from configuration inheritance. */
test("service creates a native child with an independent transcript", async () => {
  const storage = objectDatabase(), creations = [];
  const runtime = modules({
    ToolPkg: { getSpaceDataDir: () => "/space/subagents" },
    Tools: { Storage: { objects: { async open() { return storage.database; } } }, Chat: {
      /** Records the exact SDK creation options used by the production service. */
      async createNew(options) { creations.push(copy(options)); return { chatId: "native-child" }; },
      /** Completes the actual manager's send through its native semantic event stream. */
      async *sendMessageStreaming() {},
    } },
  });
  const result = await runtime.load("./service").receive({ action: "spawn", parentChatId: "parent",
    name: "worker", task: "independent task", participantId: null });
  await flush();
  assert.deepEqual(creations, [{ setAsCurrentChat: false, sourceChatId: "parent", parentChatId: "parent" }]);
  assert.equal(result.chatId, "native-child");
});

/** Verifies collapse remains interactive during a wait and storage updates never force it open. */
test("running agents can be collapsed while waiting without cancelling their execution", async () => {
  const h = harness(); const child = await h.spawn();
  const storage = objectDatabase();
  const runtime = modules({
    ToolPkg: { getSpaceDataDir: () => "/space/subagents", ipc: { async call(_channel, request) { return h.manager.request(request); } } },
    Tools: { Storage: { objects: { async open() { return storage.database; } } } },
  });
  const { Repository } = runtime.load("./storage"); const repository = new Repository(); await repository.load();
  await repository.save(h.repository.records.get(child.id));
  const ui = composition(runtime.load("./ui/panel.ui").default, { chatId: "parent" });
  ui.render(); await flush();
  let tree = ui.render();
  ui.find(tree, "subagents.expand").props.onClick(); tree = ui.render();
  assert.ok(ui.find(tree, "agent-" + child.id));
  ui.state.set("subagents.waiting", true); ui.state.set("subagents.busy", true);
  tree = ui.render();
  const toggle = ui.find(tree, "subagents.expand");
  assert.notEqual(toggle.props.enabled, false);
  toggle.props.onClick(); tree = ui.render();
  assert.equal(ui.find(tree, "agent-" + child.id), null);
  const writes = storage.commits.length;
  const update = copy(h.repository.records.get(child.id)); update.status = "running";
  await repository.save(update); await flush(); tree = ui.render();
  assert.equal(ui.find(tree, "agent-" + child.id), null);
  ui.find(tree, "subagents.header").props.onClick(); tree = ui.render();
  assert.ok(ui.find(tree, "agent-" + child.id));
  assert.equal(storage.commits.length, writes + 1);
  assert.equal(h.chat.cancellations.length, 0);
  ui.dispose(); await flush();
});

/** Verifies visibility and live updates using the production Compose compiler and retained SDK. */
test("real Compose hides idle history and updates across active and terminal states", async () => {
  const h = harness();
  const storage = objectDatabase();
  const runtime = modules({
    ToolPkg: { getSpaceDataDir: () => "/space/subagents", ipc: { async call(_channel, request) { return h.manager.request(request); } } },
    Tools: { Storage: { objects: { async open() { return storage.database; } } } },
  },
  /** Applies the same artifact transformation the host uses for the authored screen. */
  (source, file) => file.endsWith("panel.ui.ts") ? compileComposeModule(source, file) : source);
  const { Repository } = runtime.load("./storage");
  const repository = new Repository(); await repository.load();
  const Screen = runtime.load("./ui/panel.ui").default;
  runtime.sandbox.module = { exports: { default: Screen } };
  runtime.sandbox.exports = runtime.sandbox.module.exports;
  vm.runInContext(composeContextScript(), runtime.sandbox);
  vm.runInContext(wrapComposeScript(), runtime.sandbox);
  const fixture = composeStreamFixture(runtime.sandbox);
  fixture.adapt();
  const render = runtime.sandbox.__operit_render_compose_dsl;
  await render({ executionContextKey: "subagents-parent", state: { chatId: "parent" } });
  await flush();
  assert.equal(fixture.root.children.length, 0);
  const child = await h.spawn();
  await repository.save(h.repository.records.get(child.id)); await flush();
  assert.ok([...fixture.nodes.values()].some(node => node.props.key === "subagents.surface"), "a committed agent must reveal the compact header");
  assert.ok(![...fixture.nodes.values()].some(node => node.props.key === "agent-" + child.id), "task rows start collapsed");
  const history = [...fixture.nodes.values()].find(node => node.props.key === "subagents.history");
  await runtime.sandbox.__operit_dispatch_compose_dsl_action({ actionId: history.props.onClick.__actionId });
  await flush();
  const row = [...fixture.nodes.values()].find(node => node.props.key === "agent-" + child.id);
  assert.ok(row, "history reveals the task list inside the queue surface");
  await runtime.sandbox.__operit_dispatch_compose_dsl_action({ actionId: row.props.onClick.__actionId });
  await flush();
  assert.ok([...fixture.nodes.values()].some(node => node.props.key === "subagents.detail-dialog"));
  const chat = [...fixture.nodes.values()].find(node => node.nodeType === "AiChat");
  assert.ok(chat);
  assert.equal(chat.props.chatId, child.chatId);
  assert.equal(chat.props.showInput, false);
  const completed = copy(h.repository.records.get(child.id));
  completed.status = "completed";
  await repository.save(completed); await flush();
  assert.equal(fixture.root.children.length, 1, "only the open modal may remain after the last completion");
  assert.equal(fixture.root.props.paddingHorizontal, 0);
  assert.ok(![...fixture.nodes.values()].some(node => node.props.key === "subagents.history"));
  assert.ok(![...fixture.nodes.values()].some(node => node.props.key === "agent-" + child.id));
  assert.ok([...fixture.nodes.values()].some(node => node.nodeType === "AiChat" && node.props.chatId === child.chatId));
  const detail = [...fixture.nodes.values()].find(node => node.props.key === "subagents.detail-dialog");
  await runtime.sandbox.__operit_dispatch_compose_dsl_action({ actionId: detail.props.onDismissRequest.__actionId });
  await flush();
  assert.equal(fixture.root.children.length, 0, "expanded history must not keep an idle composer visible");
  for (const status of ["starting", "running", "closing", "completed", "cancelled", "failed", "closed", "interrupted"]) {
    const record = copy(completed);
    record.status = status;
    await repository.save(record); await flush();
    const active = ["starting", "running", "closing"].includes(status);
    assert.equal([...fixture.nodes.values()].some(node => node.props.key === "agent-" + child.id), active, status);
    if (!active) {
      assert.equal(fixture.root.children.length, 0, status + " must take zero composer height");
      assert.equal(fixture.root.props.paddingHorizontal, 0);
      assert.equal(fixture.root.props.paddingBottom, 0);
    }
  }
  assert.equal(fixture.errors.length, 0);
  await render({ executionContextKey: "subagents-other", state: { chatId: "other" } });
  await flush();
  assert.equal(fixture.root.children.length, 0);
  assert.equal(storage.observers.get("parents:parent").size, 0);
  assert.equal(storage.observers.get("agents:" + child.id).size, 0);
  await repository.save(completed); await flush();
  assert.equal(fixture.root.children.length, 0, "the previous chat must not leak into the new composer");
  const other = await h.spawn({}, "other");
  await repository.save(h.repository.records.get(other.id)); await flush();
  assert.ok([...fixture.nodes.values()].some(node => node.props.key === "subagents.surface"));
  assert.ok(![...fixture.nodes.values()].some(node => node.props.key === "agent-" + other.id), "the new chat starts with a compact header");
  assert.ok(![...fixture.nodes.values()].some(node => node.props.key === "agent-" + child.id));
  runtime.sandbox.OperitComposeReactive.dispose(runtime.sandbox.__operit_compose_bundle.ctx);
  await flush();
  assert.equal([...storage.observers.values()].reduce((count, values) => count + values.size, 0), 0);
  assert.equal(h.chat.cancellations.length, 0);
});

/** Reports initial observation failures to host diagnostics without exposing an idle composer. */
test("composer reports a failed initial read and disposes without opening a storage observer", async () => {
  const storage = objectDatabase();
  const runtime = modules({
    ToolPkg: { getSpaceDataDir: () => "/space/subagents", ipc: { async call() { throw new Error("initial read failed"); } } },
    Tools: { Storage: { objects: { async open() { return storage.database; } } } },
  });
  const ui = composition(runtime.load("./ui/panel.ui").default, { chatId: "parent" });
  assert.equal(ui.render().children.length, 0);
  await flush();
  assert.equal(ui.render().children.length, 0);
  assert.equal(ui.errors.length, 1);
  assert.equal(String(ui.errors[0]), "Error: initial read failed");
  assert.equal(storage.observers.size, 0);
  ui.dispose();
});
