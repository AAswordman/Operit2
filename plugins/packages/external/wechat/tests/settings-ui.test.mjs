import assert from "node:assert/strict";
import test from "node:test";
import { fixture, load } from "./harness.mjs";

/** Renders the actual Compose screen with real service requests and controlled host state. */
async function screenFixture(options = {}) {
  const f = await fixture(options);
  const states = new Map(),
    refs = new Map(),
    effects = new Map();
  const screen = load("ui/settings.ui", {
    ...f.globals,
    ToolPkg: {
      ...f.globals.ToolPkg,
      ipc: {
        async call(channel, request, options) {
          assert.equal(channel, "wechat.service");
          assert.equal(options.targetRuntime, "main");
          return f.call(request);
        },
      },
    },
  }).default;
  const ctx = {
    useState(key, initial) {
      if (!states.has(key)) states.set(key, initial);
      return [states.get(key), (value) => states.set(key, value)];
    },
    useRef(key, initial) {
      if (!refs.has(key)) refs.set(key, { current: initial });
      return refs.get(key);
    },
    useMemo(_key, factory) {
      return factory();
    },
    useEffect(key, setup) {
      if (!effects.has(key)) effects.set(key, setup());
    },
    MaterialTheme: {
      colorScheme: new Proxy({}, { get: (_, key) => String(key) }),
    },
    UI: new Proxy(
      {},
      {
        get:
          (_, type) =>
          (props, children = []) => ({
            type,
            props,
            children: [children].flat(Infinity),
          }),
      },
    ),
  };
  /** Evaluates a fresh screen tree using retained Compose state. */
  function render() {
    return screen(ctx);
  }
  /** Flattens actual rendered nodes without interpreting their behavior. */
  function nodes(node = render()) {
    return [node, ...node.children.flatMap((child) => nodes(child))];
  }
  /** Finds a stable user-facing control from the rendered tree. */
  function find(key) {
    const result = nodes().find((node) => node.props.key === key);
    assert.ok(result, `Missing ${key}`);
    return result;
  }
  /** Invokes one enabled action just as the host action protocol does. */
  async function click(key) {
    const node = find(key);
    assert.notEqual(node.props.enabled, false);
    await node.props.onClick();
  }
  /** Updates an authored field using its actual Compose callback. */
  function change(key, value) {
    const node = find(key);
    (node.props.onValueChange ?? node.props.onCheckedChange)(value);
  }
  await render().props.onLoad();
  return {
    ...f,
    render,
    nodes,
    find,
    click,
    change,
    states,
    text: () =>
      nodes()
        .map((node) => node.props.text ?? "")
        .join("\n"),
    dispose: () => {
      for (const cleanup of effects.values()) cleanup();
    },
  };
}

test("native QR SVG renders without a WebView, network image service or external Node process", async () => {
  const f = await screenFixture({ loggedOut: true });
  await f.click("login");
  const qr = f.find("qrCode");
  assert.equal(qr.type, "Svg");
  assert.match(qr.props.source, /^<svg/);
  assert.match(qr.props.source, /<path/);
  assert.equal(
    f.nodes().some((node) => node.type === "WebView"),
    false,
  );
  await f.qr({
    status: "confirmed",
    bot_token: "token",
    ilink_bot_id: "my-bot",
    ilink_user_id: "owner",
    baseurl: "https://ilinkai.weixin.qq.com",
  });
  await f.time.advance(1600);
  assert.match(f.text(), /my-bot/);
  assert.equal(
    f.nodes().some((node) => node.props.key === "qrCode"),
    false,
  );
  f.dispose();
  assert.equal(f.time.pending(), 0);
});

test("status refresh does not overwrite unsaved role, timeout or reply instructions", async () => {
  const f = await screenFixture();
  await f.click("advanced");
  f.change("instruction", "尚未保存的要求");
  f.change("deadline", "35");
  f.find("role").props.onClick(2);
  await f.click("refresh");
  await f.time.advance(1600);
  assert.equal(f.find("instruction").props.value, "尚未保存的要求");
  assert.equal(f.find("deadline").props.value, "35");
  assert.equal(f.states.get("selection"), "card:alice");
  f.dispose();
});

test("one start action saves the visible drafts then starts the actual listener", async () => {
  const f = await screenFixture();
  await f.click("advanced");
  f.change("deadline", "30");
  f.change("instruction", "请简短回复");
  f.find("role").props.onClick(2);
  await f.click("start");
  assert.equal(f.config.autoReply.aiTimeoutMs, 30000);
  assert.equal(f.config.autoReply.characterSelection, "card:alice");
  assert.equal(f.config.autoReply.assistantInstruction, "请简短回复");
  assert.equal(f.config.listenerEnabled, true);
  assert.equal(
    f.requests.filter((request) => request.url.endsWith("/getupdates")).length,
    1,
  );
  f.dispose();
  await f.stop();
});

test("saving settings does not start a stopped listener and invalid inputs never persist", async () => {
  const f = await screenFixture();
  await f.click("advanced");
  f.change("deadline", "0");
  await f.click("saveReplies");
  assert.equal(f.config.autoReply.aiTimeoutMs, 120000);
  assert.match(f.text(), /1～600/);
  f.change("deadline", "45");
  await f.click("saveReplies");
  assert.equal(f.config.autoReply.aiTimeoutMs, 45000);
  assert.equal(f.config.listenerEnabled, false);
  assert.equal(f.requests.length, 0);
  f.dispose();
});

test("verification UI submits the code to the original QR session and clears the sensitive draft", async () => {
  const f = await screenFixture({ loggedOut: true });
  await f.click("login");
  await f.qr({ status: "need_verifycode" });
  await f.time.advance(1600);
  f.change("verifyCode", "987654");
  await f.click("verify");
  assert.match(f.requests.at(-1).url, /verify_code=987654/);
  assert.equal(f.states.get("verifyCode"), "");
  f.dispose();
  await f.finish(() => f.service.terminate());
});

test("logout requires an explicit second action before deleting the account", async () => {
  const f = await screenFixture();
  await f.click("logout");
  assert.equal(f.config.account.botId, "bot");
  await f.click("keepAccount");
  assert.equal(f.config.account.botId, "bot");
  await f.click("logout");
  await f.click("confirmLogout");
  assert.equal(f.config.account, null);
  f.dispose();
});
