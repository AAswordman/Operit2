import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { composeContextScript, wrapComposeScript } from '../../../../../tools/runtime/compose_scripts.mjs';
import { composeStreamFixture } from '../../../../../tools/tests/support/compose_stream_fixture.mjs';
const root = fileURLToPath(new URL('..', import.meta.url));
const require = createRequire(new URL('../../../buildin/workflow/package.json', import.meta.url));
const ts = require('typescript');
const contextSource = readFileSync(new URL('../../../../../core/crates/plugin/javascript-bridge/src/javascript/RuntimeContext.script.js', import.meta.url), 'utf8');

/** Evaluates current TypeScript sources without depending on an older dist directory. */
function sourceLoader(context) {
  const modules = new Map();
  /** Loads each CommonJS module once inside its owning runtime realm. */
  function load(filename) {
    if (modules.has(filename)) return modules.get(filename).exports;
    const module = { exports: {} }; modules.set(filename, module);
    const source = ts.transpileModule(readFileSync(filename, 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 }, fileName: filename,
    }).outputText;
    const wrapper = vm.runInContext(`(function(exports, require, module) {${source}\n})`, context, { filename });
    wrapper(module.exports, specifier => load(path.resolve(path.dirname(filename), specifier + '.ts')), module);
    return module.exports;
  }
  return load;
}
const model = sourceLoader(vm.createContext({}))(path.join(root, 'src/model.ts'));
const plain = value => JSON.parse(JSON.stringify(value));

function host(seed) {
  let now = 1800000000000, saved = seed, api, generate = async () => {}, lostDelivery = false, failStorage = false, cancelled = false, waifu = false, language = 'zh-CN';
  const chats = [{ id: 'chat-1', title: '测试对话' }, { id: 'chat-2', title: '第二个对话' }];
  const messages = new Map(chats.map(chat => [chat.id, []]));
  const calls = [], handlers = new Map(), registrations = {};
  const extensions = new Map();
  const clone = value => value === undefined ? undefined : structuredClone(value);
  const tools = {
    Storage: { dataStore: { async open(options) {
      assert.equal(options.path, '/device/plugin/ai-chat-presentation.preferences.json');
      return {
      async data() { return { get() { return clone(saved); } }; },
      async edit(edit) { if (failStorage) throw Error('disk failed'); edit({ set(key, value) { assert.equal(key, 'state'); saved = clone(value); } }); },
    }; } } },
    System: { async toast(text) { calls.push(['toast', text]); } },
    Chat: {
      async listAll() { return { chats: clone(chats) }; },
      async agentStatus() { return { isProcessing: false }; },
      async createNew(options) {
        calls.push(['create', clone(options)]);
        const chatId = 'raw-' + chats.length; chats.push({ id: chatId, title: '新对话' }); messages.set(chatId, []);
        return { chatId, createdAt: now };
      },
      async updateTitle(chatId, title) { chats.find(chat => chat.id === chatId).title = title; },
      async switchTo(chatId) { calls.push(['switch', chatId]); },
      async getMessagesRange(chatId, { start, end }) { return { messages: clone(messages.get(chatId).slice(start, end + 1)) }; },
      async getMessages(chatId, { order, limit }) { return { messages: clone((order === 'desc' ? [...messages.get(chatId)].reverse() : messages.get(chatId)).slice(0, limit)) }; },
      async readExtension({ chatId }) { return { extension: extensions.get(chatId) ?? null }; },
      async writeExtension({ chatId }, value) { extensions.set(chatId, clone(value)); },
      async sendMessage(request) {
        calls.push(['send', clone(request)]);
        assert.equal(request.turn.kind, 'record_only', 'Only original native streaming may execute a model');
        const timestamp = ++now;
        messages.get(request.chatId).push({ sender: 'user', content: request.input.text, timestamp, attachments: clone(request.input.attachments) });
        return { chatId: request.chatId, outcome: { type: 'committed', status: 'completed', userMessageTimestamp: timestamp } };
      },
      async *sendMessageStreaming(request) {
        calls.push(['generate', clone(request)]); cancelled = false;
        const timestamp = ++now, assistantTimestamp = ++now;
        messages.get(request.chatId).push({ sender: 'user', content: request.input.text, timestamp, attachments: clone(request.input.attachments) });
        const prompt = await api.composePrompt({ event: 'after_compose_system_prompt', eventPayload: { chatId: request.chatId, systemPrompt: 'ROLE + MODEL + MEMORY' } });
        const sessionId = /session_id 固定为 ([^；]+)/.exec(prompt.systemPrompt)[1];
        calls.push(['prompt', prompt.systemPrompt]);
        yield { type: 'part', chatId: request.chatId, messageTimestamp: assistantTimestamp, parts: [{ kind: 'thinking', content: 'PRIVATE REASONING' }] };
        await generate({ request, sessionId, assistantTimestamp, show: (segments, messageId = 'batch-1') => api.show_messages({ session_id: sessionId, message_id: messageId, segments_json: JSON.stringify(segments) }) });
        messages.get(request.chatId).push({ sender: 'ai', content: 'RAW COMPLETE REPLY + THINKING + TOOLS', timestamp: assistantTimestamp });
        yield { type: 'completed', result: { chatId: request.chatId, outcome: { type: 'committed', status: cancelled ? 'cancelled' : 'completed', userMessageTimestamp: timestamp, assistant: { messageTimestamp: assistantTimestamp } } } };
      },
      async deliverPrepared(request) {
        calls.push(['deliver', clone(request)]);
        const existing = messages.get(request.chatId).find(item => item.timestamp === request.timestamp);
        if (existing) { assert.equal(existing.content, request.text); return { ...request, duplicate: true, notificationError: null }; }
        messages.get(request.chatId).push({ sender: 'ai', content: request.text, timestamp: request.timestamp });
        if (lostDelivery) { lostDelivery = false; throw Error('response lost'); }
        return { ...request, duplicate: false, notificationError: null };
      },
      async cancel(chatId) { cancelled = true; calls.push(['cancel', chatId]); },
    },
  };
  function boot() {
    class Clock extends Date { constructor(...args) { super(...(args.length ? args : [now])); } static now() { return now; } }
    const registry = {
      getLocalDataDir: () => '/device/plugin',
      ipc: { on(name, handler) { handlers.set(name, handler); }, async call(name, payload, options) { assert.equal(options.targetRuntime, 'main'); return handlers.get(name)(clone(payload)); } },
      registerToolboxUiModule(value) { registrations.ui = value; },
      registerChatInputHook(value) { registrations.input = value; },
      registerInputMenuTogglePlugin(value) { registrations.menu = value; },
      async callDependency(id, method, payload) {
        assert.equal(id, 'com.operit.waifu'); assert.equal(method, 'mode.status'); assert.ok(payload.chatId);
        if (typeof waifu === 'string') throw Error(waifu);
        return { enabled: waifu };
      },
      registerSystemPromptComposeHook(value) { registrations.prompt = value; },
    };
    const context = vm.createContext({ console, Date: Clock, Tools: tools, ToolPkg: registry, getLang: () => language, RuntimeContext: { register() {} },
      setTimeout(callback, ms) { return setTimeout(() => { now += ms; callback(); }, 0); }, clearTimeout });
    const load = sourceLoader(context);
    api = { ...load(path.join(root, 'src/service.ts')), ...load(path.join(root, 'src/main.ts')), ...load(path.join(root, 'src/tools.ts')) };
    api.registerToolPkg();
    return { api, context, load };
  }
  boot();
  async function enable(chatId = 'chat-1') { await api.saveRule({ ...model.defaultRule(chatId, '测试对话'), enabled: true, charDelayMs: 0 }); return saved.rules.find(rule => rule.chatId === chatId).originalChatId; }
  function submit(patch = {}) { return api.inputHook({ event: 'submit_requested', eventPayload: { chatId: 'chat-1', runtime: 'main', text: '你好', attachments: [], replyToMessageTimestamp: null, notifyReply: true, source: 'Classic', isProcessing: false, ...patch } }); }
  async function idle() {
    for (let count = 0; count < 200; count++) {
      await new Promise(resolve => setTimeout(resolve, 1));
      if (!saved?.tasks.some(task => ['queued', 'running'].includes(task.status))) { await new Promise(resolve => setTimeout(resolve, 1)); return; }
    }
    assert.fail('Plugin did not settle: ' + JSON.stringify(saved));
  }
  return { get api() { return api; }, boot, enable, submit, idle, chats, messages, calls, registrations, extensions,
    menu: (action, chatId = 'chat-1', toggleId = 'ai_chat_presentation.enabled') => api.inputMenu({ eventPayload: { action, chatId, toggleId } }),
    waifu: value => { waifu = value; }, language: value => { language = value; },
    state: () => clone(saved), generate: callback => { generate = callback; }, lost: () => { lostDelivery = true; },
    storageFail: value => { failStorage = value; } };
}

test('native child keeps original generation; parent shows only AI-selected message segments', async () => {
  const h = host(), raw = await h.enable();
  h.generate(async ({ show }) => { await show([{ text: '刚忙完吗？' }, { text: '我想到一个办法', delayMs: 600 }, { text: '要不要试试？', delayMs: 0 }]); });
  assert.equal((await h.submit()).action, 'Consume');
  await h.idle();
  assert.deepEqual(h.messages.get('chat-1').map(item => item.content), ['你好', '刚忙完吗？', '我想到一个办法', '要不要试试？']);
  assert.deepEqual(h.messages.get(raw).map(item => item.content), ['你好', 'RAW COMPLETE REPLY + THINKING + TOOLS']);
  assert.equal(h.state().tasks[0].displayed, 3); assert.equal(h.state().tasks[0].status, 'completed');
  const deliveries = h.calls.filter(call => call[0] === 'deliver').map(call => call[1]);
  assert.equal(deliveries[1].timestamp - deliveries[0].timestamp, 600);
  assert.deepEqual(deliveries.map(item => item.notify), [true, false, false]);
  assert.match(h.calls.find(call => call[0] === 'prompt')[1], /ROLE \+ MODEL \+ MEMORY/);
  assert.deepEqual(plain(h.calls.find(call => call[0] === 'create')[1]), { setAsCurrentChat: false, sourceChatId: 'chat-1', parentChatId: 'chat-1' });
});

test('AI can choose silence without leaking its normal final response into the parent', async () => {
  const h = host(), raw = await h.enable(); await h.submit(); await h.idle();
  assert.equal(h.messages.get('chat-1').length, 1); assert.equal(h.messages.get(raw).length, 2);
  assert.equal(h.state().tasks[0].status, 'completed');
});

test('display retries and lost responses do not duplicate messages; changed IDs conflict', async () => {
  const h = host(); await h.enable();
  h.generate(async ({ show }) => {
    h.lost(); await assert.rejects(show([{ text: '第一段' }, { text: '第二段' }]), /response lost/);
    await show([{ text: '第一段' }, { text: '第二段' }]);
    await show([{ text: '第一段' }, { text: '第二段' }]);
    await assert.rejects(show([{ text: '不同内容' }]), /message_id/);
  });
  await h.submit(); await h.idle();
  assert.deepEqual(h.messages.get('chat-1').map(item => item.content), ['你好', '第一段', '第二段']);
  assert.equal(h.state().tasks[0].displayed, 2);
});

test('attachments, floating runtime and reply references reach the real original conversation', async () => {
  const h = host(), raw = await h.enable();
  h.generate(async ({ show }) => { await show([{ text: '可以继续问我' }]); });
  const attachment = { filePath: '/image.png', fileName: 'image.png', mimeType: 'image/png', fileSize: 42, content: '', vfsPath: null, nodeId: null };
  await h.submit({ runtime: 'floating', attachments: [attachment], text: '看图片' }); await h.idle();
  const displayReply = h.messages.get('chat-1')[1];
  await h.submit({ replyToMessageTimestamp: displayReply.timestamp, text: '说详细点' }); await h.idle();
  const generations = h.calls.filter(call => call[0] === 'generate').map(call => call[1]);
  assert.equal(generations[0].runtime, 'floating'); assert.deepEqual(generations[0].input.attachments, [attachment]);
  assert.equal(generations[1].input.replyToMessageTimestamp, h.messages.get(raw)[1].timestamp);
  assert.equal(h.state().tasks[1].status, 'completed');
});

test('initial child copies existing history and is excluded from the display conversation picker', async () => {
  const h = host();
  h.messages.get('chat-1').push({ sender: 'user', content: '以前的问题', timestamp: 100 }, { sender: 'ai', content: '以前的回答', timestamp: 101 });
  const raw = await h.enable();
  assert.deepEqual(h.messages.get(raw).map(item => item.content), ['以前的问题', '以前的回答']);
  await h.api.saveRule({ ...h.state().rules[0], prompt: '只展示关键结果' });
  assert.equal(h.messages.get(raw).length, 2); assert.equal(h.calls.filter(call => call[0] === 'create').length, 1);
  assert.ok(!(await h.api.snapshot()).chats.some(chat => chat.id === raw));
  await h.api.openOriginal('chat-1'); assert.deepEqual(h.calls.at(-1), ['switch', raw]);
  await assert.rejects(h.api.saveRule({ ...model.defaultRule(raw, 'raw'), enabled: true }), /原始子对话/);
});

test('per-chat queue preserves multiple inputs and can handle a later turn after a model error', async () => {
  const h = host(); await h.enable(); let count = 0;
  h.generate(async ({ show }) => { if (count++ === 0) throw Error('model offline'); await show([{ text: '收到第二条' }]); });
  await h.submit({ text: '第一条' }); await h.submit({ text: '第二条' }); await h.idle();
  assert.deepEqual(h.state().tasks.map(task => task.status), ['failed', 'completed']);
  assert.deepEqual(h.messages.get('chat-1').map(item => item.content), ['第一条', '第二条', '收到第二条']);
});

test('stopping rejects late publications and removing a rule preserves both chat transcripts', async () => {
  const h = host(), raw = await h.enable();
  h.generate(async ({ show }) => { await show([{ text: '已经展示' }]); await h.api.cancelChat('chat-1'); await assert.rejects(show([{ text: '不应出现' }], 'late'), /已结束|已关闭/); });
  await h.submit(); await h.idle();
  assert.equal(h.state().tasks[0].status, 'interrupted');
  assert.deepEqual(h.messages.get('chat-1').map(item => item.content), ['你好', '已经展示']);
  await h.api.removeRule('chat-1'); assert.equal(h.state().rules.length, 0);
  assert.ok(h.messages.has(raw)); assert.ok(h.messages.has('chat-1'));
});

test('expired sessions, invalid segments and native recursion cannot publish', async () => {
  const h = host(); await h.enable(); let expired;
  h.generate(async ({ show, sessionId }) => {
    expired = sessionId;
    await assert.rejects(show([{ text: '' }]), /正文/);
    await assert.rejects(show([{ text: 'a', delayMs: 3001 }]), /延迟/);
    await assert.rejects(show(Array.from({ length: 11 }, () => ({ text: 'a', delayMs: 3000 }))), /30 秒/);
  });
  await h.submit(); await h.idle();
  await assert.rejects(h.api.publish({ sessionId: expired, messageId: 'late', segments: [{ text: 'late' }] }), /已经结束/);
  assert.equal(await h.submit({ source: 'Sequence' }), undefined);
  assert.equal(await h.submit({ source: 'Tool' }), undefined);
  assert.equal(await h.submit({ chatId: 'chat-2' }), undefined);
});

test('restart keeps chat bindings but interrupts unfinished tasks without repeating model execution', async () => {
  const h = host(); const raw = await h.enable();
  h.generate(async ({ show }) => { await show([{ text: '保存的回复' }]); });
  await h.submit(); await h.idle();
  h.boot(); const snapshot = await h.api.snapshot();
  assert.equal(snapshot.state.rules[0].originalChatId, raw); assert.equal(h.calls.filter(call => call[0] === 'generate').length, 1);
  const saved = h.state(); saved.tasks[0].status = 'running';
  const restarted = host(saved); const restored = await restarted.api.snapshot();
  assert.equal(restored.state.tasks[0].status, 'interrupted'); assert.equal(restarted.calls.filter(call => call[0] === 'generate').length, 0);
});

test('existing per-chat switch enables segmented chat and retains its ID in both languages', async () => {
  const h = host();
  let menu = await h.menu('create');
  assert.equal(menu.toggles[0].id, 'ai_chat_presentation.enabled');
  assert.equal(menu.toggles[0].title, 'AI 分段对话');
  assert.equal(menu.toggles[0].isChecked, false);
  await h.menu('toggle', 'chat-1', 'another-plugin');
  assert.equal(h.calls.filter(call => call[0] === 'create').length, 0);
  await h.menu('toggle');
  assert.equal((await h.menu('create')).toggles[0].isChecked, true);
  assert.equal((await h.menu('create', 'chat-2')).toggles[0].isChecked, false);
  const raw = h.state().rules[0].originalChatId;
  assert.ok(raw);
  h.extensions.set(raw, { enabled: true });
  assert.equal(await h.menu('create', raw), undefined);
  assert.equal(await h.submit({ chatId: raw }), undefined);
  await h.menu('toggle');
  assert.equal((await h.menu('create')).toggles[0].isChecked, false);
  assert.equal(await h.submit(), undefined);
  assert.equal(h.chats.some(chat => chat.id === raw), true);
  h.language('en-US');
  assert.equal((await h.menu('create')).toggles[0].title, 'AI Segmented Chat');
});

test('old enabled conversations migrate once on input without touching scheduled messages', async () => {
  const h = host();
  h.extensions.set('chat-1', { enabled: true });
  assert.equal((await h.menu('create')).toggles[0].isChecked, true);
  assert.equal(h.calls.filter(call => call[0] === 'create').length, 0);
  assert.equal((await h.submit({ isProcessing: true })).action, 'Block');
  assert.equal(h.calls.filter(call => call[0] === 'create').length, 0);
  assert.equal((await h.submit()).action, 'Consume'); await h.idle();
  const raw = h.state().rules[0].originalChatId;
  await h.submit(); await h.idle();
  assert.equal(h.calls.filter(call => call[0] === 'create').length, 1);
  assert.equal(h.calls.filter(call => call[0] === 'generate').length, 2);
  await h.api.removeRule('chat-1');
  assert.equal((await h.menu('create')).toggles[0].isChecked, false);
  assert.equal(await h.submit(), undefined);
  assert.ok(h.messages.has(raw));
});

test('legacy switches can be disabled without creating an original child', async () => {
  const h = host(); h.extensions.set('chat-1', { enabled: true });
  await h.menu('toggle');
  assert.equal((await h.menu('create')).toggles[0].isChecked, false);
  assert.equal(h.calls.filter(call => call[0] === 'create').length, 0);
  assert.equal(await h.submit(), undefined);
});

test('active or uncheckable Waifu keeps its priority, while missing or disabled Waifu allows input', async () => {
  const h = host(); await h.enable();
  for (const waifu of [true, 'Dependency version mismatch: com.operit.waifu', 'unavailable']) {
    h.waifu(waifu); assert.equal(await h.submit(), undefined);
  }
  assert.equal(h.calls.filter(call => call[0] === 'generate').length, 0);
  for (const waifu of [false, 'ToolPkg container not found: com.operit.waifu', 'ToolPkg package is disabled: com.operit.waifu']) {
    h.waifu(waifu); assert.equal((await h.submit()).action, 'Consume'); await h.idle();
  }
  assert.equal(h.calls.filter(call => call[0] === 'generate').length, 3);
});

test('storage errors retain input and corrupt state is rejected', async () => {
  const h = host(); await h.enable(); h.storageFail(true);
  await assert.rejects(h.submit(), /保存失败/); assert.equal(h.calls.filter(call => call[0] === 'generate').length, 0);
  const corrupt = host({ version: 2, rules: [model.defaultRule('chat-1', 'bad'), model.defaultRule('chat-1', 'duplicate')], tasks: [] });
  await assert.rejects(corrupt.api.snapshot(), /损坏/);
});

/** Uses the actual Compose and context runtimes to exercise settings actions. */
async function screenDriver(h) {
  const main = h.boot(), channels = new Map();
  function bridge(context) {
    context.ToolPkg.ipc = {
      on(name, handler) { if (context === main.context) channels.set(name, handler); },
      async call(name, request, options) {
        assert.equal(options.targetRuntime, 'main'); assert.ok(channels.has(name));
        const result = await channels.get(name)(structuredClone(request));
        return result === undefined ? undefined : structuredClone(result);
      },
    };
    vm.runInContext('globalThis.__operitExpose = (name, value) => { globalThis[name] = value; }; globalThis.__operitCompileJavaScript = source => new Function(source);', context);
    vm.runInContext(contextSource + '\nRuntimeContext.__operitEnsureContextRunnerRegistered();', context);
  }
  // The fixture replaces RuntimeContext with its production implementation before registering handlers.
  bridge(main.context);
  const service = main.load(path.join(root, 'src/service.ts'));
  main.context.RuntimeContext.register({ snapshot: service.snapshot, saveRule: service.saveRule, removeRule: service.removeRule, openOriginal: service.openOriginal, cancelChat: service.cancelChat });
  const context = vm.createContext({ console, Date: main.context.Date, ToolPkg: {}, module: { exports: {} } });
  bridge(context); vm.runInContext(composeContextScript() + wrapComposeScript(), context);
  context.module.exports = sourceLoader(context)(path.join(root, 'src/ui.ts'));
  composeStreamFixture(context).adapt();
  let frame = await context.__operit_render_compose_dsl({});
  function descendants(node) { return [node, ...(node.children ?? []).flatMap(descendants), ...Object.values(node.slots ?? {}).flatMap(nodes => nodes.flatMap(descendants))]; }
  async function invoke(action, payload) { assert.ok(action?.__actionId); frame = await context.__operit_dispatch_compose_dsl_action({ actionId: action.__actionId, payload }); return frame; }
  // onLoad is a native action in the production Compose protocol.
  const find = predicate => descendants(frame.tree).find(predicate);
  await invoke(find(node => node.props?.onLoad)?.props.onLoad);
  return { find, invoke };
}

test('settings configure selected chats and expose their original transcript', async () => {
  const h = host(), ui = await screenDriver(h);
  await ui.invoke(ui.find(node => node.props?.key === 'chat-1').props.onClick);
  await ui.invoke(ui.find(node => node.type === 'Switch').props.onCheckedChange, true);
  await ui.invoke(ui.find(node => node.props?.text === '保存配置').props.onClick);
  assert.equal(h.state().rules[0].enabled, true);
  assert.ok(ui.find(node => node.props?.text === '查看原始子对话'));
});


test('compiled display tool keeps its metadata and ships under the existing package ID', () => {
  const manifest = JSON.parse(readFileSync(path.join(root, 'manifest.json'), 'utf8'));
  assert.equal(manifest.toolpkg_id, 'com.operit.ai_chat_presentation');
  assert.deepEqual(manifest.subpackages, [{ id: 'ai_chat_presentation', entry: 'dist/tools.js' }]);
  assert.equal(manifest.requires, undefined);
  assert.equal(manifest.display_name.zh, 'AI 分段对话');
  assert.equal(manifest.display_name.en, 'AI Segmented Chat');
  assert.deepEqual(manifest.optional_requires, [{ id: 'com.operit.waifu', min_version: '1.1.4' }]);
  const compiled = ts.transpileModule(readFileSync(path.join(root, 'src/tools.ts'), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText;
  const metadata = JSON.parse(/\/\* METADATA\s*([\s\S]*?)\*\//.exec(compiled)[1]);
  assert.equal(metadata.name, 'ai_chat_presentation'); assert.equal(metadata.tools[0].name, 'show_messages');
  assert.deepEqual(metadata.tools[0].parameters.map(param => param.name), ['session_id', 'message_id', 'segments_json']);
});
