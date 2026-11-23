import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { loadModule, plain } from "../../../buildin/character_cards/tests/runtime.mjs";

/** Supplies stateful Compose primitives while retaining the real call screen and dependency client. */
function callScreen(resolveIdentity) {
  const states = new Map(), refs = new Map(), effects = new Map(), calls = [];
  const color = { copy() { return color; } };
  const ctx = {
    useState(key, initial) {
      if (!states.has(key)) states.set(key, initial);
      return [states.get(key), value => states.set(key, value)];
    },
    useRef(key, initial) {
      if (!refs.has(key)) refs.set(key, { current: initial });
      return refs.get(key);
    },
    useEffect(key, callback) { effects.set(key, callback); },
    UI: new Proxy({}, { get: (_, type) => (props, children = []) => ({ type, props, children }) }),
    Modifier: { clip: shape => shape },
    MaterialTheme: { colorScheme: new Proxy({}, { get: () => color }) },
  };
  const plugin = loadModule("../../external/voice_call/src/main.ts", {
    getLang: () => "en",
    ToolPkg: {
      ui: { floatingWindows: {
        async record() { calls.push("record"); return null; },
      } },
      async callDependency(packageId, method, payload) {
        assert.equal(packageId, "com.operit.character_cards");
        assert.equal(method, "chat.identity");
        calls.push(plain(payload));
        return resolveIdentity(payload);
      },
    },
    Tools: { Chat: { async createNew(options) {
      assert.deepEqual(plain(options), { setAsCurrentChat: false });
      calls.push("create");
      return { chatId: "actual-call" };
    } } },
  });
  return { states, effects, calls, render: () => plugin.call(ctx) };
}

/** Traverses actual Compose output to inspect rendered identity and invoke its start control. */
function nodes(node) { return [node, ...node.children.flatMap(nodes)]; }

/** Exercises the real screen's preview race and confirms recording uses the created chat identity. */
test("call renders the bound custom role and rejects a late active-role preview", async () => {
  let finishPreview;
  const screen = callScreen(({ chatId }) => chatId === null
    ? new Promise(resolve => { finishPreview = resolve; })
    : { title: "Bound caller", avatarUri: "vfs://shared/avatars/bound.png" });
  const initial = screen.render();
  screen.effects.get("call-avatar")();
  await nodes(initial).find(node => node.type === "FilledTonalButton").props.onClick();
  finishPreview({ title: "Different active role", avatarUri: "vfs://shared/avatars/other.png" });
  await Promise.resolve();
  assert.deepEqual(screen.calls, [{ chatId: null }, "create", { chatId: "actual-call" }, "record"]);
  const rendered = nodes(screen.render());
  assert.equal(rendered.find(node => node.type === "Image").props.path, "vfs://shared/avatars/bound.png");
  assert.equal(rendered.find(node => node.type === "Image").props.contentDescription, "Bound caller");
  assert.ok(rendered.some(node => node.type === "Text" && node.props.text === "Bound caller"));
});

/** A failed identity read must pause before recording and preserve its actual error. */
test("call pauses on identity failure and rereads the same chat when resumed", async () => {
  let rejectRead = true;
  const screen = callScreen(() => {
    if (rejectRead) throw new Error("Role storage unavailable");
    return { title: "Restored caller", avatarUri: null };
  });
  await nodes(screen.render()).find(node => node.type === "FilledTonalButton").props.onClick();
  assert.equal(screen.states.get("phase"), "error");
  assert.match(screen.states.get("failure"), /Role storage unavailable/);
  assert.ok(!screen.calls.includes("record"));
  rejectRead = false;
  await nodes(screen.render()).find(node => node.type === "FilledTonalButton").props.onClick();
  assert.equal(screen.calls.filter(call => call === "create").length, 1);
  assert.deepEqual(screen.calls, ["create", { chatId: "actual-call" }, { chatId: "actual-call" }, "record"]);
  assert.ok(!nodes(screen.render()).some(node => node.type === "Image"));
});

/** Locks the copied public contract and minimum provider version to the feature actually consumed. */
test("voice call ships the current dependency client and matching version requirement", () => {
  const client = readFileSync(new URL("../src/dependencies/character_cards/api.ts", import.meta.url), "utf8");
  assert.equal(client, readFileSync(new URL("../../../buildin/character_cards/src/api.ts", import.meta.url), "utf8"));
  const provider = JSON.parse(readFileSync(new URL("../../../buildin/character_cards/manifest.json", import.meta.url), "utf8"));
  const consumer = JSON.parse(readFileSync(new URL("../manifest.json", import.meta.url), "utf8"));
  assert.equal(consumer.requires.find(item => item.id === provider.toolpkg_id).min_version, provider.version);
});
