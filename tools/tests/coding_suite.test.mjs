import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import vm from 'node:vm';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const require = createRequire(new URL('.ci-tools/typescript/package.json', root));
const ts = require('typescript');

/** Runs production TypeScript in a retained realm, with only Host boundaries replaced. */
function fixture(persisted = null) {
  const files = new Map(), reads = [], writes = [], calls = [], hooks = [], channels = new Map();
  const state = { settings: persisted, messages: [], failAppend: false, summaryError: false, toasts: [] };
  const context = vm.createContext({
    console,
    PluginConfig: {
      /** Retains the same memory-backed settings object as the actual plugin runtime. */
      async use(_name, defaults) { state.settings ??= structuredClone(defaults); return state.settings; },
      /** Captures durable checkpoints for simulated restarts. */
      async flush() {},
    },
    Tools: {
      Files: {
        async exists(file) { return { exists: files.has(file), isDirectory: false }; },
        async read(file) { reads.push(file); return { content: files.get(file) }; },
        async mkdir() { return { successful: true }; },
        async grep(file, pattern) {
          return { matches: files.get(file).split('\n').some(line => new RegExp(pattern).test(line)) ? [{}] : [] };
        },
        async write(file, content, append) {
          writes.push({ file, content, append });
          if (state.failAppend) return { operation: { successful: false, details: 'disk unavailable' } };
          files.set(file, (append ? files.get(file) ?? '' : '') + content);
          return { operation: { successful: true } };
        },
      },
      Chat: {
        async getMessages() { return { messages: structuredClone(state.messages) }; },
        async call(options) {
          calls.push(options);
          if (state.summaryError) throw new Error('model unavailable');
          return { text: '- Changed cancel.py; verified six tests. Follow up on cancellation.' };
        },
      },
      System: { async toast(text) { state.toasts.push(text); } },
    },
    ToolPkg: {
      ipc: {
        on(channel, handler) { channels.set(channel, handler); },
        async call(channel, request, options) {
          assert.equal(options.targetRuntime, 'main');
          assert.ok(channels.has(channel), channel);
          return channels.get(channel)(request);
        },
      },
      registerPromptFinalizeHook(hook) { hooks.push({ kind: 'send', ...hook }); },
      registerPromptEstimateFinalizeHook(hook) { hooks.push({ kind: 'estimate', ...hook }); },
      registerChatMessageHook(hook) { hooks.push({ kind: 'message', ...hook }); },
      registerHostEventHook(hook) { hooks.push({ kind: 'clock', ...hook }); },
      registerUiRoute() {}, registerNavigationEntry() {},
    },
  });
  const modules = new Map();
  /** Loads the real module graph without writing any application build output. */
  function load(name) {
    if (modules.has(name)) return modules.get(name).exports;
    const module = { exports: {} };
    modules.set(name, module);
    const source = readFileSync(new URL(`plugins/packages/buildin/coding_suite/src/${name}.ts`, root), 'utf8');
    const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS } }).outputText;
    vm.runInContext(`(function(require,module,exports){${javascript}\n})`, context)(specifier => {
      assert.ok(specifier.startsWith('./'));
      return load(path.posix.normalize(specifier.slice(2)).replace(/\.js$/, ''));
    }, module, module.exports);
    return module.exports;
  }
  const main = load('main');
  main.registerToolPkg();
  return { state, files, reads, writes, calls, hooks, main, load };
}

/** Constructs the actual final-stage hook payload with authoritative VFS roots. */
function event(chatId = 'chat-a', workspace = '/operitapp/workspaces/project') {
  return { eventName: 'before_send_to_model', eventPayload: { stage: 'before_send_to_model',
    promptFunctionType: 'CHAT', chatId, metadata: { workspacePath: workspace, outputPath: '/operitapp/outputs/family' },
    preparedHistory: [{ kind: 'SYSTEM', content: 'Host prompt' }, { kind: 'USER', content: 'Implement cancellation' }] } };
}

/** Enqueues one exact persisted revision through the production message hook. */
async function reply(f, timestamp = 200, variant = 0) {
  f.state.messages = [
    { sender: 'ai', content: 'Old exchange should not be summarized again.', timestamp: 50, variantIndex: 0 },
    { sender: 'user', content: 'Fix cancellation', timestamp: 100, variantIndex: 0 },
    { sender: 'ai', content: 'Changed cancel.py. Six checks passed.', timestamp, variantIndex: variant },
  ];
  await f.main.onCodingMessage({ eventName: 'message_persisted', eventPayload: {
    chatId: 'chat-a', sender: 'ai', timestamp, selectedVariantIndex: variant, content: 'Changed cancel.py. Six checks passed.' } });
}

test('fresh AGENTS.md and plain prompt edits appear without reading the timeline', async () => {
  const f = fixture();
  const path = '/operitapp/workspaces/project/AGENTS.md';
  f.files.set(path, 'Use Host APIs.');
  f.files.set('/operitapp/workspaces/project/.operit/coding-memory.md', 'PRIVATE OLD MEMORY');
  let result = await f.main.onCodingPrompt(event());
  assert.match(result.preparedHistory[1].content, /Use Host APIs/);
  assert.doesNotMatch(result.preparedHistory[1].content, /PRIVATE OLD MEMORY/);
  assert.deepEqual(f.reads, [path]);
  await f.main.settingsRequest({ action: 'save', prompt: 'Editable coding prompt', memoryEnabled: true });
  f.files.set(path, 'New rule.');
  result = await f.main.onCodingPrompt(event());
  assert.match(result.preparedHistory[1].content, /Editable coding prompt[\s\S]*New rule/);
});

test('estimate and send use identical fresh context and repeated hooks replace their own contribution', async () => {
  const f = fixture();
  const send = f.hooks.find(hook => hook.kind === 'send');
  const estimate = f.hooks.find(hook => hook.kind === 'estimate');
  await estimate.function(event('estimate-only'));
  assert.equal(f.state.settings.bindings['estimate-only'], undefined);
  const first = await send.function(event());
  const secondEvent = event();
  secondEvent.eventPayload.preparedHistory = first.preparedHistory;
  const second = await estimate.function(secondEvent);
  assert.equal(second.preparedHistory.length, 3);
  assert.equal(second.preparedHistory[1].content, first.preparedHistory[1].content);
});

test('functional MEMORY calls and unrelated prompt stages do no context reads or writes', async () => {
  const f = fixture();
  const input = event(); input.eventPayload.promptFunctionType = 'MEMORY';
  assert.equal(await f.main.onCodingPrompt(input), null);
  input.eventPayload.promptFunctionType = 'CHAT'; input.eventPayload.stage = 'before_finalize_prompt';
  assert.equal(await f.main.onCodingPrompt(input), null);
  assert.equal(f.state.settings, null);
});

test('unbound chats use the actual inherited output directory; switching projects changes both rules and memory', async () => {
  const f = fixture();
  let result = await f.main.onCodingPrompt(event('chat-a', null));
  assert.match(result.preparedHistory[1].content, /\/operitapp\/outputs\/family\/\.operit\/coding-memory.md/);
  assert.deepEqual(f.reads, []);
  await f.main.onCodingPrompt(event('chat-a', '/operitapp/workspaces/second'));
  await reply(f);
  assert.equal(f.state.settings.jobs[0].path, '/operitapp/workspaces/second/.operit/coding-memory.md');
});

test('missing output identity is an explicit error instead of inventing a folder', async () => {
  const f = fixture(), input = event('chat-a', null);
  delete input.eventPayload.metadata.outputPath;
  await assert.rejects(f.main.onCodingPrompt(input), /host conversation output path/);
});

test('reply persistence queues work without making a model call; Host clock appends one dated entry', async () => {
  const f = fixture();
  await f.main.onCodingPrompt(event());
  await reply(f);
  assert.equal(f.calls.length, 0);
  assert.equal(f.state.settings.jobs.length, 1);
  assert.equal(f.state.settings.jobs[0].turns.length, 2);
  await f.main.onCodingClock();
  assert.equal(f.calls.length, 1);
  assert.equal(f.calls[0].functionType, 'MEMORY');
  assert.equal(f.calls[0].recordTokenUsage, true);
  assert.equal(f.writes[0].append, true);
  assert.match(f.writes[0].content, /1970-01-01T00:00:00.200Z[\s\S]*cancel.py/);
  assert.equal(f.state.settings.jobs.length, 0);
  await reply(f);
  await f.main.onCodingClock();
  assert.equal(f.writes.length, 1);
});

test('append preserves existing history, and projects share one file across conversations', async () => {
  const f = fixture();
  const file = '/operitapp/workspaces/project/.operit/coding-memory.md';
  f.files.set(file, '# Historical entry\n');
  await f.main.onCodingPrompt(event()); await reply(f); await f.main.onCodingClock();
  assert.ok(f.files.get(file).startsWith('# Historical entry\n'));
  await f.main.onCodingPrompt(event('other-chat'));
  assert.equal(f.state.settings.bindings['other-chat'].base, f.state.settings.bindings['chat-a'].base);
});

test('restart resumes a saved summary without another model call; complete append receipts prevent duplicates', async () => {
  const f = fixture();
  await f.main.onCodingPrompt(event()); await reply(f);
  f.state.failAppend = true;
  await f.main.onCodingClock();
  assert.equal(f.calls.length, 1);
  assert.match(f.state.settings.jobs[0].error, /disk unavailable/);
  await f.main.onCodingClock();
  assert.equal(f.calls.length, 1, 'failed jobs wait for explicit retry');
  const restarted = fixture(structuredClone(f.state.settings));
  await restarted.main.settingsRequest({ action: 'retry' });
  await restarted.main.onCodingClock();
  assert.equal(restarted.calls.length, 0);
  assert.equal(restarted.writes.length, 1);
  const finishedFile = restarted.files.get(restarted.writes[0].file);
  const lostCheckpoint = fixture(structuredClone(f.state.settings));
  lostCheckpoint.files.set(restarted.writes[0].file, finishedFile);
  await lostCheckpoint.main.settingsRequest({ action: 'retry' });
  await lostCheckpoint.main.onCodingClock();
  assert.equal(lostCheckpoint.calls.length, 0);
  assert.equal(lostCheckpoint.writes.length, 0);
});

test('empty prompt and disabled memory remove stale coding context and queue no work', async () => {
  const f = fixture();
  const first = await f.main.onCodingPrompt(event());
  assert.equal(f.state.settings.prompt, f.load('config').DEFAULT_PROMPT);
  await f.main.settingsRequest({ action: 'save', prompt: '', memoryEnabled: false });
  const input = event();
  input.eventPayload.preparedHistory = first.preparedHistory;
  const result = await f.main.onCodingPrompt(input);
  assert.deepEqual(JSON.parse(JSON.stringify(result.preparedHistory)), event().eventPayload.preparedHistory);
  await reply(f); await f.main.onCodingClock();
  assert.equal(f.calls.length, 0);
  assert.equal(f.state.settings.jobs.length, 0);
});

test('Host clock overlap is serialized and message revisions have separate append identities', async () => {
  const f = fixture();
  await f.main.onCodingPrompt(event()); await reply(f);
  await Promise.all([f.main.onCodingClock(), f.main.onCodingClock()]);
  assert.equal(f.writes.length, 1);
  await reply(f, 200, 1); await f.main.onCodingClock();
  assert.equal(f.writes.length, 2);
  assert.match(f.writes[1].content, /revision 1/);
});
