import assert from "node:assert/strict";
import test from "node:test";
import { load, plain, clock, settle } from "./harness.mjs";

/** Boots the actual plugin once with isolated SDK ports and real cross-module handlers. */
function runtime() {
  const time = clock(), configs = new Map(), handlers = {}, apis = {}, channels = new Map(), requests = [], toasts = [];
  const bindings = { alice: "card:alice", bob: "card:bob" };
  const ToolPkg = {
    ipc: { on(name, handler) { channels.set(name, handler); }, async call(name, payload) { return channels.get(name)(payload); } },
    async callDependency(owner, method, input) {
      assert.equal(owner, "com.operit.character_cards");
      switch (method) {
        case "activePrompt.get": return { CharacterCard: { id: "alice" } };
        case "character.get": return { id: input.id, name: "Alice" };
        case "chat.configuration.binding.read": return { chatId: input.chatId, selection: bindings[input.chatId] };
        default: throw new Error(`Unexpected dependency: ${method}`);
      }
    },
    async readResource(key) { return `/host/resources/${key}.jpg`; },
    registerApi({ name, function: handler }) { apis[name] = handler; },
    registerInputMenuTogglePlugin({ function: handler }) { handlers.menu = handler; },
    registerChatResponseHook({ function: handler }) { handlers.response = handler; },
    registerChatInputHook({ function: handler }) { handlers.input = handler; },
    registerSystemPromptComposeHook({ function: handler }) { handlers.prompt = handler; },
    registerToolboxUiModule({ screen }) { handlers.screen = screen; },
  };
  const Tools = { Chat: { async sendMessage(request) {
    const decision = await handlers.input(input(request.input.text, { chatId: request.chatId, source: "Sequence" }));
    assert.equal(decision, undefined, "A native send must not re-enter the merge queue");
    requests.push(plain(request));
    return { outcome: { type: "committed", status: "completed" } };
  } }, System: { async toast(message) { toasts.push(message); } } };
  const PluginConfig = { async use(name, defaults) { if (!configs.has(name)) configs.set(name, defaults); return configs.get(name); }, async flush() {} };
  load("main", { ToolPkg, Tools, PluginConfig, ...time }).registerToolPkg();
  return { handlers, apis, time, configs, requests, toasts,
    admin: request => channels.get("waifu.settings")(request),
    async enable(changes = {}) {
      const context = await channels.get("waifu.settings")({ action: "mode.get" });
      const mode = { ...context.mode, ...changes, enabled: true };
      await channels.get("waifu.settings")({ action: "mode.set", selection: context.target.selection, mode });
      return mode;
    },
  };
}

/** Constructs the exact complete semantic markdown part consumed by production response hooks. */
function part(content, kind = "markdown") { return { partId: kind, sequence: 0, kind, content, toolCallId: null, toolName: null, attributes: {} }; }

/** Creates one native response lifecycle event with a stable revision identity. */
function response(event, content = "", chatId = "alice", parts = [part(content)]) {
  return { event, eventName: event, eventPayload: { chatId, messageTimestamp: 10, variantIndex: 0, parts } };
}

/** Preserves the complete native input context when checking merge behavior. */
function input(text, changes = {}) {
  return { event: "submit_requested", eventPayload: { chatId: "alice", runtime: "main", notifyReply: true, source: "Classic", text,
    attachments: [], attachmentCount: 0, hasAttachments: false, replyToMessageTimestamp: null, ...changes } };
}

test("ordinary Waifu starts disabled and its switch is bound to the actual character", async () => {
  const r = runtime();
  assert.equal(await r.handlers.response(response("started")), false);
  const context = await r.admin({ action: "mode.get" });
  assert.equal(context.target.name, "Alice"); assert.equal(context.mode.enabled, false);
  assert.equal(context.mode.mergeDelayMs, 5000); assert.equal(context.mode.settings.charDelayMs, 250);
  assert.match(context.mode.settings.customPrompt, /你必须遵守/); assert.match(context.mode.settings.selfiePrompt, /kipfel vrchat/);
  await r.handlers.menu({ eventPayload: { action: "toggle", chatId: "alice", toggleId: "waifu.mode" } });
  assert.equal(await r.handlers.response(response("started")), true);
  assert.equal(await r.handlers.response(response("started", "", "bob")), false);
  const menu = await r.handlers.menu({ eventPayload: { action: "create", chatId: "alice" } });
  assert.equal(menu.toggles[0].isChecked, true);
});

test("ordinary chat receives prompt rules, stable sentences and real emotion images without QQBot", async () => {
  const r = runtime(), mode = await r.enable();
  await r.enable({ settings: { ...mode.settings, enableEmoticons: true } });
  const prompt = await r.handlers.prompt({ event: "after_compose_system_prompt", eventPayload: { chatId: "alice", systemPrompt: "Character prompt" } });
  assert.match(prompt.systemPrompt, /^Character prompt/); assert.match(prompt.systemPrompt, /<emotion>/);
  await r.handlers.response(response("started"));
  const delta = await r.handlers.response(response("snapshot", "你好。<emotion>happy</emotion>后面"));
  assert.equal(delta.bubbles.length, 2); assert.equal(delta.bubbles[0].parts[0].content, "你好。"); assert.equal(delta.bubbles[0].delayMs, 0);
  assert.match(delta.bubbles[1].parts[0].content, /!\[happy\]\(<\/host\/resources\/emoji_happy_/);
  const final = await r.handlers.response(response("finished", "你好。<emotion>happy</emotion>后面也很好。"));
  assert.equal(final.bubbles.length, 1); assert.equal(final.bubbles[0].delayMs, 1500);
  await r.handlers.response(response("closed"));
  await assert.rejects(r.handlers.response(response("snapshot", "hello")), /does not exist/);
});

test("thinking and structured tool output cannot become ordinary Waifu sentences", async () => {
  const r = runtime(); await r.enable(); await r.handlers.response(response("started"));
  const parts = [part("private thinking!", "thinking"), part("secret tool payload!", "tool_result"), part("你好。"), part("done", "status")];
  const delta = await r.handlers.response(response("finished", "", "alice", parts));
  assert.deepEqual(plain(delta.bubbles.map(bubble => bubble.parts[0].content)), ["你好。"]);
});

test("dependency consumers keep their own profiles and do not enter the native presentation queue", async () => {
  const r = runtime(); await r.enable();
  const session = await r.apis["session.open"]({ callerPackage: "qqbot", payload: { chatId: "alice", profile: "default" } });
  assert.equal(await r.handlers.response(response("started")), false);
  const chunks = await r.apis["session.feed"]({ callerPackage: "qqbot", payload: { sessionId: session.sessionId, messageTimestamp: 10, snapshot: "外部插件。", final: true } });
  assert.deepEqual(plain(chunks), [{ kind: "text", text: "外部插件。" }]);
  await r.apis["session.close"]({ callerPackage: "qqbot", payload: { sessionId: session.sessionId } });
  assert.equal(await r.handlers.response(response("started")), true);
});

test("native response settings remain stable while the next reply can use edited settings", async () => {
  const r = runtime(), mode = await r.enable();
  await r.handlers.response(response("started"));
  await r.enable({ settings: { ...mode.settings, removePunctuation: true } });
  const delta = await r.handlers.response(response("finished", "保留句号。"));
  assert.equal(delta.bubbles[0].parts[0].content, "保留句号。");
  await r.handlers.response(response("closed")); await r.handlers.response(response("started"));
  const next = await r.handlers.response(response("finished", "移除句号。"));
  assert.equal(next.bubbles[0].parts[0].content, "移除句号");
});

test("human short messages reset one timer and execute exactly one real native send", async () => {
  const r = runtime(); await r.enable({ mergeSend: true, mergeDelayMs: 500 });
  assert.equal((await r.handlers.input(input("第一句"))).action, "Consume");
  await r.time.advance(400);
  assert.equal((await r.handlers.input(input("第二句"))).metadata.queuedMessages, 2);
  await r.time.advance(499); assert.equal(r.requests.length, 0);
  await r.time.advance(1); assert.equal(r.requests.length, 1);
  assert.deepEqual(r.requests[0].input, { text: "第一句\n第二句", attachments: [], replyToMessageTimestamp: null });
  assert.equal(r.time.pending(), 0); assert.deepEqual(r.toasts, []);
});

test("attachments and reply targets preserve their native submission after accepted text is drained", async () => {
  const r = runtime(); await r.enable({ mergeSend: true, mergeDelayMs: 500 });
  await r.handlers.input(input("先说一句"));
  const attachments = [{ filePath: "/host/photo.png", nodeId: "origin", fileName: "photo.png", mimeType: "image/png", fileSize: 4, content: "inline" }];
  assert.equal((await r.handlers.input(input("看图", { attachments, attachmentCount: 1, hasAttachments: true, replyToMessageTimestamp: 77 }))).action, "Consume");
  await settle();
  assert.equal(r.requests.length, 2); assert.equal(r.requests[0].input.text, "先说一句");
  assert.deepEqual(r.requests[1].input, { text: "看图", attachments, replyToMessageTimestamp: 77 });
  assert.equal(r.time.pending(), 0);
});

/** Creates a minimal persistent native Compose context with inspectable actual nodes. */
function context() {
  const values = new Map(), refs = new Map();
  const ctx = {
    useState(key, initial) { if (!values.has(key)) values.set(key, initial); return [values.get(key), value => values.set(key, value)]; },
    useRef(key, initial) { if (!refs.has(key)) refs.set(key, { current: initial }); return refs.get(key); },
    MaterialTheme: { colorScheme: new Proxy({}, { get(_target, key) { return String(key); } }) },
    Modifier: { clickable(onClick) { return { onClick }; } },
    UI: new Proxy({}, { get(_target, type) { return (props, children = []) => ({ type, props, children: Array.isArray(children) ? children : [children] }); } }),
  };
  return ctx;
}

/** Flattens the real native node tree for UI interaction assertions. */
function nodes(tree) { return [tree, ...tree.children.flatMap(nodes)]; }

test("native settings restore role cards, direct switches and sliders without protocol or JSON forms", async () => {
  const r = runtime(), ctx = context();
  let tree = r.handlers.screen(ctx); await tree.props.onLoad(); tree = r.handlers.screen(ctx);
  const all = nodes(tree), texts = all.map(node => node.props.text).filter(Boolean);
  for (const label of ["Waifu 模式", "绑定到角色卡", "Alice", "合并发送", "打字速度", "自定义 Waifu 提示词", "管理自定义表情包", "自拍"]) assert.ok(texts.includes(label), label);
  assert.ok(!texts.some(text => /QQBot|JSON|配置名称/.test(text)));
  assert.ok(all.some(node => node.type === "Slider" && node.props.min === 200 && node.props.max === 1000));
  assert.ok(!all.some(node => node.props.label === "角色外貌关键词"));
  await all.find(node => node.type === "Switch").props.onCheckedChange(true); await settle();
  assert.equal((await r.admin({ action: "mode.get" })).mode.enabled, true);
  const manage = all.find(node => node.type === "Card" && node.props.modifier !== undefined);
  await manage.props.modifier.onClick(); tree = r.handlers.screen(ctx);
  const emojiNodes = nodes(tree);
  assert.ok(emojiNodes.some(node => node.type === "Image"));
  assert.ok(emojiNodes.some(node => node.props.text === "从设备导入图片"));
});
