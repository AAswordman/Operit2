import assert from "node:assert/strict";
import test from "node:test";
import { load, plain } from "./harness.mjs";

/** Renders the real DSL module with controlled state and an administrative IPC port. */
function fixture(options = {}) {
  const state = new Map(), refs = new Map(), requests = [];
  const config = plain(load("model").defaultConfig());
  config.appId = options.empty ? "" : "saved-app";
  config.appSecret = options.empty ? "" : "saved-secret";
  config.listenerEnabled = options.running === true;
  config.autoReply.enabled = options.running === true;
  let running = options.running === true, reads = 0;
  const roles = [{ selection: "card:alice", name: "爱丽丝" }, { selection: "group:friends", name: "朋友们（角色组）" }];
  function status() {
    return plain({ appId: config.appId, configured: !!config.appId && !!config.appSecret, waifuProfile: config.waifuProfile,
      useSandbox: config.useSandbox, listenerEnabled: config.listenerEnabled,
      service: { status: config.listenerEnabled ? "ready" : "stopped", sessionId: "", lastError: "", receivedCount: 0 },
      queue: { size: 0, capacity: 200 }, autoReply: { config: config.autoReply, running, lastError: "", bindings: {}, repliedCount: 0 } });
  }
  const screen = load("ui/settings.ui", { ToolPkg: { ipc: { async call(channel, request) {
    assert.equal(channel, "qqbot.service"); requests.push(plain(request));
    if (request.action === "status") {
      if (++reads === 1 && options.failFirstLoad) throw new Error("offline");
      return { success: true, status: status() };
    }
    if (request.action === "reply_roles") {
      if (options.failRoles) throw new Error("role provider unavailable");
      return { success: true, roles };
    }
    if (request.action === "configure") {
      if (options.failConnection) throw new Error("AppSecret 无效");
      const p = request.params;
      config.appId = p.app_id ?? config.appId; config.appSecret = p.app_secret ?? config.appSecret;
      config.useSandbox = p.use_sandbox ?? config.useSandbox; config.listenerEnabled = p.listener_enabled ?? config.listenerEnabled;
      running = config.listenerEnabled && config.autoReply.enabled;
    } else if (request.action === "auto_configure") {
      const p = request.params;
      for (const [source, target] of Object.entries({ enabled: "enabled", c2c_enabled: "c2cEnabled", group_enabled: "groupEnabled", ai_timeout_ms: "aiTimeoutMs", character_selection: "characterSelection", assistant_instruction: "assistantInstruction", chat_group: "chatGroup" })) {
        if (p[source] !== undefined) config.autoReply[target] = p[source];
      }
      config.waifuProfile = p.waifu_profile ?? config.waifuProfile;
      running = p.start_now !== false && config.listenerEnabled && config.autoReply.enabled;
    } else if (request.action === "stop") { config.listenerEnabled = false; running = false; }
    else if (request.action === "receive") return { success: true, events: options.events ?? [], returnedCount: 0, remainingCount: 0 };
    else if (request.action === "clear") return { success: true, clearedCount: 0 };
    else throw new Error(`Unexpected request: ${request.action}`);
    return { success: true, status: status() };
  } } } }).default;
  const ctx = {
    useState(key, initial) {
      if (!state.has(key)) state.set(key, initial);
      return [state.get(key), value => { assert.notEqual(typeof value, "function"); state.set(key, value); }];
    },
    useRef(key, initial) { if (!refs.has(key)) refs.set(key, { current: initial }); return refs.get(key); },
    MaterialTheme: { colorScheme: new Proxy({}, { get: (_, key) => String(key) }) },
    UI: new Proxy({}, { get: (_, type) => (props, children = []) => ({ type, props, children: [children].flat(Infinity) }) }),
  };
  const render = () => screen(ctx);
  function nodes(node = render()) { return [node, ...node.children.flatMap(child => nodes(child))]; }
  function find(key) { const node = nodes().find(node => node.props.key === key); assert.ok(node, `Missing control ${key}`); return node; }
  async function click(key) { const node = find(key); assert.notEqual(node.props.enabled, false, `Disabled control ${key}`); await node.props.onClick(); }
  function change(key, value) { const node = find(key); assert.notEqual(node.props.enabled, false); (node.props.onValueChange ?? node.props.onCheckedChange)(value); }
  return { requests, config, render, find, click, change, state,
    text: () => nodes().map(node => node.props.text ?? "").join("\n"),
    initialize: async () => { await render().props.onLoad(); },
  };
}

test("connection validation uses edited credentials and preserves reply drafts", async () => {
  const f = fixture(); await f.initialize();
  await f.click("advanced"); f.change("instruction", "尚未保存的回复要求");
  await f.click("editConnection"); f.change("appId", "new-app"); f.change("secret", "new-secret");
  await f.click("connect");
  const request = f.requests.find(request => request.action === "configure");
  assert.deepEqual(request.params, { app_id: "new-app", app_secret: "new-secret", use_sandbox: false, test_connection: true });
  assert.equal(f.find("instruction").props.value, "尚未保存的回复要求");
  assert.equal(f.state.get("secret"), "");
  assert.match(f.text(), /连接验证通过/);
});

test("refresh and stop preserve unsaved drafts across both forms", async () => {
  const f = fixture({ running: true }); await f.initialize();
  await f.click("editConnection"); f.change("appId", "draft-app"); f.change("secret", "draft-secret");
  await f.click("advanced"); f.change("deadline", "35"); f.change("instruction", "draft reply");
  for (const key of ["refresh", "stop"]) {
    await f.click(key);
    assert.equal(f.find("appId").props.value, "draft-app");
    assert.equal(f.find("secret").props.value, "draft-secret");
    assert.equal(f.find("deadline").props.value, "35");
    assert.equal(f.find("instruction").props.value, "draft reply");
  }
  assert.equal(f.config.listenerEnabled, false);
});

test("one start action saves current credentials, role and reply scope before running", async () => {
  const f = fixture({ empty: true }); await f.initialize();
  f.change("appId", "my-app"); f.change("secret", "my-secret");
  f.find("role").props.onClick(2); f.change("group", false);
  await f.click("advanced"); f.change("deadline", "30");
  f.requests.length = 0;
  await f.click("start");
  assert.deepEqual(f.requests.map(request => request.action), ["auto_configure", "configure", "status"]);
  assert.equal(f.config.autoReply.characterSelection, "card:alice");
  assert.equal(f.config.autoReply.aiTimeoutMs, 30000);
  assert.equal(f.config.autoReply.groupEnabled, false);
  assert.equal(f.config.appSecret, "my-secret");
  assert.equal(f.config.listenerEnabled, true);
  assert.match(f.text(), /机器人运行中/);
  assert.match(f.find("stop").props.text, /停止机器人/);
});

test("saving reply edits neither starts a stopped bot nor overwrites credential drafts", async () => {
  const f = fixture(); await f.initialize();
  await f.click("editConnection"); f.change("secret", "unsaved-secret");
  f.change("c2c", false); await f.click("saveReplies");
  assert.equal(f.config.autoReply.c2cEnabled, false);
  assert.equal(f.config.listenerEnabled, false);
  assert.equal(f.config.autoReply.enabled, false);
  assert.equal(f.find("secret").props.value, "unsaved-secret");
  assert.equal(f.requests.filter(request => request.action === "configure").length, 0);
});

test("saving while running keeps automatic replies active", async () => {
  const f = fixture({ running: true }); await f.initialize();
  f.change("group", false); await f.click("saveReplies");
  assert.match(f.text(), /机器人运行中/);
  assert.equal(f.requests.find(request => request.action === "auto_configure").params.start_now, true);
});

test("invalid reply scope and timeout fail before any configuration is written", async () => {
  const f = fixture(); await f.initialize();
  await f.click("advanced");
  for (const invalid of ["", "abc", "0", "601"]) {
    f.change("deadline", invalid); await f.click("start");
    assert.match(f.text(), /1～600 秒/);
  }
  f.change("deadline", "120"); f.change("c2c", false); f.change("group", false);
  await f.click("start"); assert.match(f.text(), /至少选择一种回复范围/);
  assert.equal(f.requests.filter(request => ["configure", "auto_configure"].includes(request.action)).length, 0);
});

test("failed verification retains entered secret and presents the error beside connection controls", async () => {
  const f = fixture({ failConnection: true }); await f.initialize();
  await f.click("editConnection"); f.change("secret", "retry-secret");
  await f.click("connect");
  assert.equal(f.find("secret").props.value, "retry-secret");
  assert.match(JSON.stringify(f.find("connectionCard")), /AppSecret 无效/);
});

test("changing AppID requires its own secret instead of reusing the old bot credential", async () => {
  const f = fixture(); await f.initialize();
  await f.click("editConnection"); f.change("appId", "different-app"); await f.click("connect");
  assert.match(f.text(), /请填写这个机器人的 AppSecret/);
  assert.equal(f.requests.filter(request => request.action === "configure").length, 0);
});

test("duplicate clicks before re-render start only one operation", async () => {
  const f = fixture(); await f.initialize();
  const button = f.find("start");
  await Promise.all([button.props.onClick(), button.props.onClick()]);
  assert.equal(f.requests.filter(request => request.action === "configure").length, 1);
});

test("a failed initial read can be retried without leaving the form disabled", async () => {
  const f = fixture({ failFirstLoad: true }); await f.initialize();
  assert.match(f.text(), /配置读取失败/);
  await f.click("refresh");
  await f.click("editConnection");
  assert.equal(f.find("appId").props.value, "saved-app");
  assert.equal(f.find("appId").props.enabled, true);
});

test("unavailable role choices preserve saved selection and allow retry", async () => {
  const f = fixture({ failRoles: true }); f.config.autoReply.characterSelection = "card:old";
  await f.initialize();
  assert.equal(f.state.get("selection"), "card:old");
  assert.match(f.find("role").props.text, /已保存的角色/);
  assert.match(f.text(), /角色列表读取失败/);
  assert.ok(f.find("reloadRoles"));
});

test("diagnostics peek messages without consuming them or rendering protocol JSON", async () => {
  const f = fixture({ events: [{ scene: "c2c", timestamp: "2026-10-28", content: "你好", rawPayload: { private: "internal payload" } }] });
  await f.initialize(); await f.click("diagnostics"); await f.click("recentEvents");
  assert.match(f.text(), /私聊 · 2026-10-28\n你好/);
  assert.doesNotMatch(f.text(), /internal payload/);
  assert.equal(f.requests.find(request => request.action === "receive").params.consume, false);
});


test("fractional seconds preserve valid millisecond timeouts", async () => {
  const f = fixture(); f.config.autoReply.aiTimeoutMs = 1001;
  await f.initialize(); await f.click("advanced");
  assert.equal(f.find("deadline").props.value, "1.001");
  f.change("instruction", "updated"); await f.click("saveReplies");
  assert.equal(f.config.autoReply.aiTimeoutMs, 1001);
});
