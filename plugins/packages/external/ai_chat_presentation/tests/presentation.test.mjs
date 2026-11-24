import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { test } from "node:test";
import vm from "node:vm";

const require = createRequire(new URL("../dist/main.js", import.meta.url));
const source = readFileSync(new URL("../dist/main.js", import.meta.url), "utf8");
const toggleId = "ai_chat_presentation.enabled";

function harness({ waifu = false, messages, call } = {}) {
  const extensions = new Map();
  const hooks = {};
  const requests = [];
  const timers = new Map();
  const errors = [];
  let nextTimer = 0;
  vm.runInNewContext(source, {
    exports: hooks,
    require,
    getLang: () => "zh-CN",
    console: { error: (...args) => errors.push(args) },
    setTimeout(callback, ms) {
      const id = ++nextTimer;
      timers.set(id, { callback, ms });
      return id;
    },
    clearTimeout: id => timers.delete(id),
    ToolPkg: {
      registerInputMenuTogglePlugin: hook => { hooks.menu = hook.function; },
      registerChatResponseHook: hook => { hooks.response = hook.function; },
      async callDependency(id, method, payload) {
        assert.equal(id, "com.operit.waifu");
        assert.equal(method, "mode.status");
        assert.ok(payload.chatId);
        if (typeof waifu === "string") throw new Error(waifu);
        return { enabled: waifu };
      },
    },
    Tools: { Chat: {
      async readExtension({ kind, chatId }) {
        assert.equal(kind, "chat");
        return { extension: extensions.get(chatId) ?? null };
      },
      async writeExtension({ kind, chatId }, value) {
        assert.equal(kind, "chat");
        extensions.set(chatId, value);
      },
      async getMessages(chatId, options) {
        assert.equal(chatId, "a");
        assert.equal(options.order, "desc");
        return { messages: messages ?? [{ sender: "user", timestamp: 9, content: "What changed?" }] };
      },
      async call(options) {
        requests.push(options);
        if (!call) assert.fail("Unexpected model call");
        return { text: await call(options) };
      },
    } },
  });
  assert.equal(hooks.registerToolPkg(), true);
  return {
    menu: (action, chatId, id = toggleId) => hooks.menu({ eventPayload: { action, chatId, toggleId: id } }),
    response: (event, parts = [], chatId = "a") => hooks.response({
      event, eventPayload: { chatId, messageTimestamp: 10, variantIndex: 0, parts },
    }),
    timers,
    requests,
    errors,
  };
}

function markdown(content, partId = "part-0") {
  return { partId, kind: "markdown", sequence: 0, content, toolCallId: null, toolName: null, attributes: {} };
}

function checkBubbles(result, contents) {
  assert.deepEqual(Array.from(result.bubbles, bubble => {
    assert.equal(bubble.delayMs, 0);
    return Array.from(bubble.parts, part => {
      assert.equal(part.kind, "markdown");
      return part.content;
    });
  }), contents);
}

test("the input menu persists a switch for each conversation", async () => {
  const host = harness();
  assert.equal(await host.response("started"), false);
  const create = await host.menu("create", "a");
  assert.equal(create.toggles[0].id, toggleId);
  assert.equal(create.toggles[0].title, "AI 自主回复");
  assert.equal(create.toggles[0].isChecked, false);
  await host.menu("toggle", "a", "another-plugin");
  assert.equal(await host.response("started"), false);
  await host.menu("toggle", "a");
  assert.equal((await host.menu("create", "a")).toggles[0].isChecked, true);
  assert.equal(await host.response("started"), true);
  assert.equal(await host.response("started", [], "b"), false);
  await host.menu("toggle", "a");
  assert.equal(await host.response("started"), false);
});

test("active or uncheckable Waifu takes priority, while missing or disabled Waifu does not", async () => {
  for (const [waifu, expected] of [
    [true, false],
    [false, true],
    ["ToolPkg container not found: com.operit.waifu", true],
    ["ToolPkg package is disabled: com.operit.waifu", true],
    ["Dependency version mismatch: com.operit.waifu", false],
  ]) {
    const host = harness({ waifu });
    await host.menu("toggle", "a");
    assert.equal(await host.response("started"), expected);
    assert.equal(host.requests.length, 0);
  }
});

test("only finished requests a decision and sends the latest user message at the reply timestamp", async () => {
  const host = harness({
    messages: [
      { sender: "user", timestamp: 11, content: "Next turn" },
      { sender: "assistant", timestamp: 10, content: "Current reply" },
      { sender: "user", timestamp: 9, content: "What changed?" },
      { sender: "user", timestamp: 8, content: "Prior turn" },
    ],
    call: () => '{"action":"show","bubbles":["First.","Last."]}',
  });
  const reply = markdown("First. Omit this. Last.");
  await host.menu("toggle", "a");
  assert.equal(await host.response("started"), true);
  checkBubbles(await host.response("snapshot", [reply]), []);
  assert.equal(host.requests.length, 0);
  checkBubbles(await host.response("finished", [
    { ...markdown("Private thinking"), kind: "thinking" }, reply,
  ]), [["First."], ["Last."]]);
  assert.equal(await host.response("closed"), undefined);
  assert.equal(host.requests.length, 1);
  assert.equal(host.requests[0].functionType, "CHAT");
  assert.deepEqual(JSON.parse(host.requests[0].turns[1].content), { user: "What changed?", reply: reply.content });
  assert.equal(host.timers.size, 0);
});

test("explicit skip and empty replies produce no text bubbles", async () => {
  const host = harness({ call: () => '{"action":"skip"}' });
  checkBubbles(await host.response("finished", [markdown("  ")]), []);
  assert.equal(host.requests.length, 0);
  checkBubbles(await host.response("finished", [markdown("Thanks!")]), []);
  assert.equal(host.requests.length, 1);
});

test("malformed, modified, reordered, overlapping and oversized plans show original parts", async () => {
  for (const output of [
    "invalid JSON",
    '{"action":"skip","bubbles":[]}',
    '{"action":"show","bubbles":["Invented text"]}',
    '{"action":"show","bubbles":["Last.","First."]}',
    '{"action":"show","bubbles":["First.","rst."]}',
    JSON.stringify({ action: "show", bubbles: Array(9).fill("word") }),
  ]) {
    const parts = [markdown("First."), markdown("Last.", "part-1")];
    const host = harness({ call: () => output });
    const result = await host.response("finished", parts);
    checkBubbles(result, [["First.", "Last."]]);
    assert.deepEqual(result.bubbles[0].parts, parts);
  }
});

test("missing history, long replies and model errors fall back to the original", async () => {
  const missing = harness({ messages: [] });
  checkBubbles(await missing.response("finished", [markdown("Original.")]), [["Original."]]);
  assert.equal(missing.requests.length, 0);
  const oversized = "x".repeat(12_001);
  checkBubbles(await missing.response("finished", [markdown(oversized)]), [[oversized]]);
  assert.equal(missing.requests.length, 0);
  const failing = harness({ call: () => { throw new Error("Model unavailable"); } });
  checkBubbles(await failing.response("finished", [markdown("Original.")]), [["Original."]]);
  assert.equal(failing.errors.length, 1);
});

test("the eight-second deadline falls back and a late decision cannot change the bubbles", async () => {
  const model = Promise.withResolvers();
  const started = Promise.withResolvers();
  const host = harness({ call: () => { started.resolve(); return model.promise; } });
  const finished = host.response("finished", [markdown("Original.")]);
  await started.promise;
  assert.equal(host.timers.size, 1);
  const [{ ms, callback }] = host.timers.values();
  assert.equal(ms, 8_000);
  callback();
  const result = await finished;
  await host.response("closed");
  model.resolve('{"action":"skip"}');
  await model.promise;
  checkBubbles(result, [["Original."]]);
  assert.equal(host.timers.size, 0);
});
