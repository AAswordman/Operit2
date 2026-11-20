import assert from "node:assert/strict";
import test from "node:test";
import { setImmediate } from "node:timers/promises";
import { createDiskHarness } from "./disk-files.mjs";
import { openPlugin } from "./plugin-entry-harness.mjs";
import { loadModule, runtimeConsole } from "./runtime.mjs";

const sidebar = { input: { view: "characters" }, chatSidebar: { chats: [], currentChatId: null, activeStreamingChatIds: [] } };
const stores = ["cards", "groups", "tags", "stores", "conversationGroups", "meta", "chats", "memory"];

/** Pauses one real storage opener while keeping all file and SQLite operations in production adapters. */
function gatedPlugin(disk, boundary) {
  let release, entered;
  const gate = new Promise(resolve => { release = resolve; });
  const started = new Promise(resolve => { entered = resolve; });
  const opened = [], logs = [], errors = [];
  /** Records the exact open order and suspends only the selected host boundary. */
  async function open(name, work) {
    opened.push(name);
    if (name === boundary) { entered(); await gate; }
    return work();
  }
  const storage = {
    ...disk.storage,
    dataStore: {
      /** Delegates the unchanged options to the real native-preference test adapter. */
      open(options) {
        const name = options.path.slice(options.path.lastIndexOf("/") + 1).replace(/\.preferences\.json$/, "");
        return open(name, () => disk.storage.dataStore.open(options));
      },
    },
    objects: {
      /** Keeps the actual memory database adapter behind the controllable open boundary. */
      open(options) { return open("memory", () => disk.storage.objects.open(options)); },
    },
  };
  const globals = { ...disk.globals, Tools: { ...disk.globals.Tools, Storage: storage }, console: {
    ...runtimeConsole,
    /** Captures only diagnostics so the pending host operation can be identified without a timer. */
    verbose(level, message) { logs.push({ level, message }); },
    /** Captures the original failure report without treating it as a successful load. */
    error(message) { errors.push(message); },
  } };
  const plugin = openPlugin({ ...disk, globals });
  return { plugin, globals, started, release, opened, logs, errors };
}

for (const boundary of stores) {
  test(`startup callers share one initializer while ${boundary} is pending`, { timeout: 10000 }, async t => {
    const disk = await createDiskHarness(t, { legacyIdentity: "default" });
    const runtime = gatedPlugin(disk, boundary);
    t.after(() => runtime.release());
    const startup = runtime.plugin.main.onServiceInitialize({});
    await runtime.started;
    const client = loadModule("src/ui-contributions.ts", { ...runtime.globals, ToolPkg: {
      ...runtime.globals.ToolPkg,
      ipc: {
        /** Uses the real registered API provider while observing the separate client boundary. */
        call(channel, payload, options) {
          assert.equal(channel, "character-memory.domain");
          assert.equal(options.targetRuntime, "main");
          return runtime.plugin.api(payload.operation, payload.input);
        },
      },
    } });
    let completed = 0;
    const pending = Promise.all([
      startup, client.callMainDomain("character.list", {}), client.callMainDomain("group.list", {}),
      client.callMainDomain("activePrompt.get", {}), runtime.plugin.sidebar(sidebar),
    ].map(promise => promise.then(value => { completed++; return value; })));
    await setImmediate();
    assert.equal(completed, 0, "UI reads must await the retained storage owner");
    assert.deepEqual(runtime.opened, stores.slice(0, stores.indexOf(boundary) + 1));
    const stage = "operation=storage.open." + boundary + " ";
    assert.equal(runtime.logs.filter(log => log.message.includes(stage) && log.message.endsWith("phase=start")).length, 1);
    assert.equal(runtime.logs.some(log => log.message.includes(stage) && log.message.includes("phase=complete")), false);
    runtime.release();
    const [, cards, groups, active, catalog] = await pending;
    assert.equal(cards.length, 1);
    assert.equal(cards[0].id, "default");
    assert.equal(groups.length, 0);
    assert.equal(active.CharacterCard.id, "default");
    assert.equal(catalog.sections.some(section => section.id === "card:default"), true);
    assert.deepEqual(runtime.opened, stores, "Concurrent callers must not open duplicate storage owners");
    assert.equal(runtime.logs.filter(log => log.message.includes(stage) && log.message.includes("phase=complete")).length, 1);
    assert.deepEqual(runtime.errors, []);
  });
}

test("an opening failure reaches all callers without reopening or replacing its original error", { timeout: 10000 }, async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  const failure = new Error("Injected memory open failure"), errors = [], logs = [];
  let opens = 0;
  const plugin = openPlugin({ ...disk, globals: { ...disk.globals, console: {
    ...runtimeConsole,
    /** Observes the real failure boundary without changing the domain result. */
    verbose(level, message) { logs.push({ level, message }); },
    /** Records the first original error as it propagates through the shared initializer. */
    error(message) { errors.push(message); },
  }, Tools: { ...disk.globals.Tools, Storage: { ...disk.storage, objects: {
    /** Rejects the native open with one exact error shared by every pending business request. */
    async open() { opens++; throw failure; },
  } } } } });
  const results = await Promise.allSettled([
    plugin.main.onServiceInitialize({}), plugin.api("character.list", {}), plugin.sidebar(sidebar),
  ]);
  for (const result of results) { assert.equal(result.status, "rejected"); assert.equal(result.reason, failure); }
  await assert.rejects(plugin.api("character.list", {}), error => error === failure);
  assert.equal(opens, 1, "A retained initialization rejection must not trigger an implicit retry");
  assert.equal(errors.length, 1);
  assert.match(errors[0], /operation=storage\.open\.memory.*Injected memory open failure/);
  assert.ok(logs.some(log => log.message.includes("operation=repository.open ") && log.message.includes("phase=error")));
});

test("a failed opener reports its pending handle cleanup before propagating the original failure", { timeout: 10000 }, async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  const failure = new Error("Memory open failed before cleanup"), logs = [];
  let release, entered;
  const gate = new Promise(resolve => { release = resolve; });
  const closing = new Promise(resolve => { entered = resolve; });
  t.after(() => release());
  const plugin = openPlugin({ ...disk, globals: { ...disk.globals, console: {
    ...runtimeConsole,
    /** Retains the outstanding cleanup phase separately from the already failed opening phase. */
    verbose(level, message) { logs.push({ level, message }); },
    /** Keeps the intentionally injected error out of ordinary test output. */
    error() {},
  }, Tools: { ...disk.globals.Tools, Storage: { ...disk.storage,
    dataStore: {
      /** Holds one retained handle's real close operation while the remaining handles close normally. */
      async open(options) {
        const handle = await disk.storage.dataStore.open(options);
        if (options.path.endsWith("/cards.preferences.json")) {
          const close = handle.close;
          /** Preserves the original close acknowledgement after the test releases cleanup. */
          handle.close = async () => { entered(); await gate; await close(); };
        }
        return handle;
      },
    },
    objects: {
      /** Fails after preferences have opened so the actual cleanup path must release their handles. */
      async open() { throw failure; },
    },
  } } } });
  let settled = false;
  const startup = plugin.main.onServiceInitialize({}).then(
    () => { settled = true; throw new Error("Failed initialization must not succeed"); },
    error => { settled = true; return error; },
  );
  await closing;
  assert.equal(settled, false);
  const cleanup = "operation=storage.open.cleanup ";
  assert.ok(logs.some(log => log.message.includes(cleanup) && log.message.endsWith("phase=start")));
  assert.equal(logs.some(log => log.message.includes(cleanup) && log.message.includes("phase=complete")), false);
  release();
  assert.equal(await startup, failure);
  assert.ok(logs.some(log => log.message.includes(cleanup) && log.message.includes("phase=complete")));
});
