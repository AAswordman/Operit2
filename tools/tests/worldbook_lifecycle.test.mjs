import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';
import test from 'node:test';

const dist = fileURLToPath(new URL('../../plugins/packages/external/worldbook/dist/', import.meta.url));

/** Loads production CommonJS modules in one retained plugin owner with counted host calls. */
function runtime({ failOpen = false } = {}) {
  const files = new Map();
  const calls = [];
  const messages = new Map();
  const lifecycle = [];
  let root = '/config/worldbook';
  const context = vm.createContext({
    console, Icons: { Book: 'book' },
    ToolPkg: {
      getConfigDir: () => root,
      registerUiRoute() {}, registerNavigationEntry() {},
      registerSystemPromptComposeHook() {}, registerPromptFinalizeHook() {},
      registerAppLifecycleHook: (hook) => lifecycle.push(hook),
    },
    Tools: {
      Files: {
        async mkdir(file) {
          calls.push(['mkdir', file]);
          if (failOpen) throw new Error('open failed');
        },
        async exists(file) { calls.push(['exists', file]); return { exists: files.has(file) }; },
        async read(file) { calls.push(['read', file]); return { content: files.get(file) }; },
        async write(file, content) { calls.push(['write', file]); files.set(file, content); },
        async deleteFile(file) { calls.push(['delete', file]); files.delete(file); },
      },
      Chat: {
        async getMessages(chatId) {
          calls.push(['history', chatId]);
          return { messages: messages.get(chatId) || [] };
        },
      },
    },
  });
  const modules = new Map();
  function load(file) {
    if (file.endsWith('.ui.js')) return {};
    if (modules.has(file)) return modules.get(file).exports;
    const module = { exports: {} };
    modules.set(file, module);
    const body = readFileSync(file, 'utf8');
    const run = vm.runInContext(`(function(require,module,exports) {\n${body}\n})`, context, { filename: file });
    run((name) => load(path.resolve(path.dirname(file), name)), module, module.exports);
    return module.exports;
  }
  const main = load(path.join(dist, 'main.js'));
  const storage = load(path.join(dist, 'shared/worldbook_storage.js'));
  const variables = load(path.join(dist, 'shared/worldbook_variables.js'));
  return {
    main, storage, variables, files, calls, messages, lifecycle,
    changeRoot(value) { root = value; },
    setEntries(entries) { files.set(`${root}/entries.json`, JSON.stringify(entries)); },
    count(method, file) { return calls.filter(([m, f]) => m === method && (!file || f.endsWith(file))).length; },
  };
}

function entry(content, overrides = {}) {
  return { id: content, name: content, content, enabled: true, always_active: true, inject_target: 'system', ...overrides };
}
function event(stage, id, chatId = 'chat-a', participantId = 'card-a') {
  return { eventName: stage, eventPayload: {
    chatId, systemPrompt: 'base', rawInput: 'hello', processedInput: 'hello', preparedHistory: [],
    metadata: {
      ...(id ? { promptPreparationId: id } : {}),
      executionContext: { chatId, participantId },
    },
  } };
}
const system = (id, chat, card) => event('after_compose_system_prompt', id, chat, card);
const final = (id, chat, card) => event('before_finalize_prompt', id, chat, card);

test('registration performs no storage I/O; lifecycle and concurrent first reads share initialization', async () => {
  const r = runtime();
  r.main.registerToolPkg();
  assert.equal(r.calls.length, 0);
  assert.equal(r.lifecycle.length, 1);
  assert.equal(r.lifecycle[0].event, 'application_on_create');
  await Promise.all([
    r.lifecycle[0].function({}), r.storage.readWorldBookEntries(),
    r.storage.readWorldBookGroups(), r.variables.syncWorldBookVariableContext('a', [], ''),
  ]);
  assert.equal(r.count('mkdir'), 1);
  for (const file of ['entries.json', 'groups.json', 'variables.json']) {
    assert.equal(r.count('exists', file), file === 'entries.json' ? 3 : 1); // current + migration probe + legacy cleanup
  }
  r.calls.length = 0;
  await r.storage.readWorldBookEntries();
  await r.storage.writeWorldBookEntries([entry('new')]);
  await r.storage.readWorldBookGroups();
  await r.storage.writeWorldBookGroups([]);
  await r.variables.syncWorldBookVariableContext('a', [], '');
  assert.equal(r.count('mkdir') + r.count('exists') + r.count('delete'), 0);
});

test('failed initialization is retained until the plugin owner is reloaded', async () => {
  const r = runtime({ failOpen: true });
  await assert.rejects(r.storage.readWorldBookEntries(), /open failed/);
  await assert.rejects(r.storage.readWorldBookGroups(), /open failed/);
  await assert.rejects(r.main.onServiceInitialize({}), /open failed/);
  assert.equal(r.count('mkdir'), 1);
  const reloaded = runtime();
  await reloaded.storage.readWorldBookEntries();
  assert.equal(reloaded.count('mkdir'), 1);
});

test('legacy data migrates once and storage root changes open the new root', async () => {
  const r = runtime();
  r.files.set('/sdcard/Download/Operit/worldbook/entries.json', JSON.stringify([entry('legacy')]));
  assert.equal((await r.storage.readWorldBookEntries())[0].content, 'legacy');
  assert.equal(r.count('delete'), 1);
  r.changeRoot('/another/worldbook');
  r.setEntries([entry('new-root')]);
  assert.equal((await r.storage.readWorldBookEntries())[0].content, 'new-root');
  assert.equal(r.count('mkdir'), 2);
});

test('system and final hooks share one entries/variables/history snapshot; next send sees external edits', async () => {
  const r = runtime();
  r.setEntries([entry('old'), entry('user-old', { inject_target: 'user' })]);
  await r.storage.initializeWorldBookStorage();
  r.calls.length = 0;
  assert.match((await r.main.systemPromptHook(system('send-1'))).systemPrompt, /old/);
  r.setEntries([entry('new'), entry('user-new', { inject_target: 'user' })]);
  assert.match((await r.main.finalizeHook(final('send-1'))).processedInput, /user-old/);
  assert.equal(r.count('read', 'entries.json'), 1);
  assert.equal(r.count('read', 'variables.json'), 1);
  assert.equal(r.count('history'), 1);
  assert.equal(r.count('mkdir') + r.count('exists'), 0);
  assert.match((await r.main.systemPromptHook(system('send-2'))).systemPrompt, /new/);
  assert.match((await r.main.finalizeHook(final('send-2'))).processedInput, /user-new/);
  assert.equal(r.count('read', 'entries.json'), 2);
  assert.equal(r.count('history'), 2);
});

test('local storage writes and plugin reloads are visible to subsequent sends', async () => {
  const r = runtime();
  await r.storage.writeWorldBookEntries([entry('first')]);
  assert.match((await r.main.systemPromptHook(system('s1'))).systemPrompt, /first/);
  await r.main.finalizeHook(final('s1'));
  await r.storage.writeWorldBookEntries([entry('updated')]);
  assert.match((await r.main.systemPromptHook(system('s2'))).systemPrompt, /updated/);
  const reloaded = runtime();
  for (const [file, content] of r.files) reloaded.files.set(file, content);
  assert.match((await reloaded.main.systemPromptHook(system('s3'))).systemPrompt, /updated/);
  assert.equal(reloaded.count('mkdir'), 1);
});

test('independent estimate hooks without a preparation identity always read fresh data', async () => {
  const r = runtime();
  r.setEntries([entry('before')]);
  assert.match((await r.main.systemPromptHook(system())).systemPrompt, /before/);
  r.setEntries([entry('after')]);
  assert.match((await r.main.systemPromptHook(system())).systemPrompt, /after/);
  assert.equal(r.count('read', 'entries.json'), 2);
  assert.equal(r.count('history'), 2);
});

test('concurrent chats retain independent snapshots and do not lose variable store updates', async () => {
  const r = runtime();
  r.setEntries([entry('A', { character_card_id: 'card-a' }), entry('B', { character_card_id: 'card-b' })]);
  const [a, b] = await Promise.all([
    r.main.systemPromptHook(system('send-a', 'chat-a', 'card-a')),
    r.main.systemPromptHook(system('send-b', 'chat-b', 'card-b')),
  ]);
  assert.match(a.systemPrompt, /name="A"/);
  assert.doesNotMatch(a.systemPrompt, /name="B"/);
  assert.match(b.systemPrompt, /name="B"/);
  await Promise.all([
    r.main.finalizeHook(final('send-a', 'chat-a', 'card-a')),
    r.main.finalizeHook(final('send-b', 'chat-b', 'card-b')),
  ]);
  const store = JSON.parse(r.files.get('/config/worldbook/variables.json'));
  assert.deepEqual(Object.keys(store.chats).sort(), ['chat-a', 'chat-b']);
  assert.equal(r.count('history'), 2);
});

test('a preparation cannot be reused for a different chat or participant', async () => {
  const r = runtime();
  r.setEntries([]);
  await r.main.systemPromptHook(system('same-id'));
  await assert.rejects(r.main.finalizeHook(final('same-id', 'chat-b')), /does not match/);
  await assert.rejects(r.main.finalizeHook(final('same-id', 'chat-a', 'card-b')), /does not match/);
});

test('variable patches still run for an empty book and are applied only once', async () => {
  const r = runtime();
  r.setEntries([]);
  r.files.set('/config/worldbook/variables.json', JSON.stringify({
    global_variables: {}, character_variables: {}, chats: {
      'chat-a': { local_variables: { score: 0 }, processed_message_timestamps: [], last_scanned_timestamp: 0 },
    },
  }));
  r.messages.set('chat-a', [{ sender: 'assistant', timestamp: 100,
    content: '<UpdateVariable><JSONPatch>[{"op":"delta","path":"/score","value":3}]</JSONPatch></UpdateVariable>' }]);
  await r.main.systemPromptHook(system('s1'));
  await r.main.finalizeHook(final('s1'));
  await r.main.systemPromptHook(system('s2'));
  await r.main.finalizeHook(final('s2'));
  const store = JSON.parse(r.files.get('/config/worldbook/variables.json'));
  assert.equal(store.chats['chat-a'].local_variables.score, 3);
  assert.deepEqual(store.chats['chat-a'].processed_message_timestamps, [100]);
  assert.equal(r.count('history'), 2);
});

test('disabled initialization templates, rendered variables, keyword matching and depth injection survive snapshot reuse', async () => {
  const r = runtime();
  r.setEntries([
    entry('score: 1', { name: '[InitVar]', enabled: false }),
    entry('PRE {{getvar::stat_data.score}}', { inject_position: 'prepend' }),
    entry('APP {{getvar::stat_data.score}}'),
    entry('USER {{getvar::stat_data.score}}', { inject_target: 'user', always_active: false, keywords: ['hello'] }),
    entry('DEPTH {{getvar::stat_data.score}}', {
      inject_target: 'assistant', always_active: false, keywords: ['hello'], inject_position: 'at_depth', insertion_depth: 1,
    }),
  ]);
  r.messages.set('chat-a', [{ sender: 'assistant', timestamp: 100,
    content: '<UpdateVariable><JSONPatch>[{"op":"delta","path":"/score","value":2}]</JSONPatch></UpdateVariable>' }]);
  const composed = await r.main.systemPromptHook(system('s1'));
  assert.match(composed.systemPrompt, /PRE 3[\s\S]*base[\s\S]*APP 3/);
  assert.doesNotMatch(composed.systemPrompt, /InitVar/);
  const request = final('s1');
  request.eventPayload.preparedHistory = [
    { kind: 'SYSTEM', content: composed.systemPrompt },
    { kind: 'USER', content: 'old user' },
    { kind: 'ASSISTANT', content: 'old assistant' },
  ];
  const finalized = await r.main.finalizeHook(request);
  assert.match(finalized.processedInput, /hello[\s\S]*USER 3/);
  assert.equal(finalized.preparedHistory[2].kind, 'ASSISTANT');
  assert.match(finalized.preparedHistory[2].content, /DEPTH 3/);
  assert.equal(r.count('history'), 1);
  const store = JSON.parse(r.files.get('/config/worldbook/variables.json'));
  assert.equal(store.chats['chat-a'].local_variables.stat_data.score, 3);
  assert.equal(store.character_variables['card-a'].stat_data.score, 1);
});
