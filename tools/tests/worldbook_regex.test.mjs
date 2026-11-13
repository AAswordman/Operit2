import assert from 'node:assert/strict';
import test from 'node:test';
import { loadModule, plain } from '../../plugins/packages/buildin/character_cards/tests/runtime.mjs';

/** Loads production rules with a strict, counted VFS boundary and real hook registrations. */
function runtime(main = false, language = 'zh-CN') {
  const files = new Map();
  const hooks = { message: [], stream: [], finalize: [], estimate: [], routes: [], navigation: [] };
  const writes = [];
  const reads = [];
  const globals = {
    getLang: () => language,
    Icons: { Book: "book" },
    Tools: { Files: {
      async exists(path) { return { exists: files.has(path) }; },
      async mkdir() {},
      async read(path) { reads.push(path); if (!files.has(path)) throw new Error(`Missing file ${path}`); return { content: files.get(path) }; },
      async write(path, content) { files.set(path, content); writes.push(path); },
      async deleteFile(path) { files.delete(path); },
    }, Chat: { async getMessages() { throw new Error('Regex transforms must not scan chat history'); } } },
    ToolPkg: {
      getConfigDir: () => '/config/worldbook',
      registerMessageTransformHook(hook) { hooks.message.push(hook); },
      registerMessageStreamTransform(hook) { hooks.stream.push(hook); },
      registerPromptFinalizeHook(hook) { hooks.finalize.push(hook); },
      registerPromptEstimateFinalizeHook(hook) { hooks.estimate.push(hook); },
      registerUiRoute(route) { hooks.routes.push(route); },
      registerNavigationEntry(entry) { hooks.navigation.push(entry); },
      registerSystemPromptComposeHook() {}, registerAppLifecycleHook() {},
      async callDependency() { return []; },
    },
  };
  const module = loadModule(`../../external/worldbook/src/${main ? 'main' : 'shared/worldbook_regex'}.ts`, globals);
  return { module, globals, files, writes, reads, hooks,
    setLanguage(value) { language = value; },
    setRules(rules) { files.set('/config/worldbook/regex_scripts.json', JSON.stringify(rules)); } };
}

const engine = runtime().module;
/** Creates one validated upstream-format test rule. */
function rule(patch = {}) {
  return engine.validateRegexRule({ id: 'rule', scriptName: 'Replace', findRegex: '/secret/g', replaceString: '', placement: [2], ...patch });
}
/** Builds an explicit engine boundary without requiring a live conversation. */
function context(patch = {}) {
  return { stage: 'source', placement: 2, depth: 0, isEdit: false, participantId: 'card-a', variables: null, ...patch };
}

/** Expands the indexed wire contract only in assertions that compare per-message semantics. */
function expand(batch) {
  const value = plain(batch);
  return value.messagePrograms.map(index => value.programs[index]);
}

test('ordered replacements retain flags and exact capture, named capture, trim and empty semantics', () => {
  const first = rule({ findRegex: '/(?<word>secret)-(\\d+)/gi', replaceString: '$<word>:$2:$0:{{match}}', trimStrings: ['e'] });
  const second = rule({ id: 'second', findRegex: '/scrt/gi', replaceString: 'ok' });
  assert.equal(engine.applyRegexRules('SECRET-12 secret-3', [first, second], context()), 'SECRET:12:SECRET-12:SECRET-12 ok:3:ok-3:ok-3');
  assert.equal(engine.applyRegexRules('secret', [rule()], context()), '');
  assert.equal(engine.applyRegexRules('secret secret', [rule({ findRegex: 'secret', replaceString: 'x' })], context()), 'x secret');
  assert.equal(engine.applyRegexRules('secret', [rule({ replaceString: ' \n' })], context()), ' \n');
});

test('source, display and model rules are independent and both only flags select both transient paths', () => {
  const rules = [rule({ id: 'source', replaceString: 'stored' }),
    rule({ id: 'display', replaceString: 'shown', markdownOnly: true }),
    rule({ id: 'prompt', replaceString: 'model', promptOnly: true })];
  assert.equal(engine.applyRegexRules('secret', rules, context()), 'stored');
  assert.equal(engine.applyRegexRules('secret', rules, context({ stage: 'display' })), 'shown');
  assert.equal(engine.applyRegexRules('secret', rules, context({ stage: 'prompt' })), 'model');
  const both = [rule({ markdownOnly: true, promptOnly: true, replaceString: 'both' })];
  assert.equal(engine.applyRegexRules('secret', both, context()), 'secret');
  for (const stage of ['display', 'prompt']) assert.equal(engine.applyRegexRules('secret', both, context({ stage })), 'both');
});

test('disabled, placement, scoped character, edit and depth filters agree with the upstream engine', () => {
  const enabled = rule({ characterCardId: 'card-a', minDepth: 1, maxDepth: 2 });
  assert.equal(engine.applyRegexRules('secret', [enabled], context({ depth: 1 })), '');
  for (const patch of [{ depth: 0 }, { depth: 3 }, { depth: 1, participantId: 'card-b' },
    { depth: 1, placement: 1 }, { depth: 1, isEdit: true }]) {
    assert.equal(engine.applyRegexRules('secret', [enabled], context(patch)), 'secret');
  }
  assert.equal(engine.applyRegexRules('secret', [rule({ runOnEdit: true })], context({ isEdit: true })), '');
  assert.equal(engine.applyRegexRules('secret', [rule({ disabled: true })], context()), 'secret');
});

test('variable substitution offers raw and escaped patterns without executing JavaScript replacements', () => {
  const variables = { globalVariables: { needle: 'a.b', replacement: '$&' }, characterVariables: {}, localVariables: {} };
  const escaped = rule({ findRegex: '/{{getglobalvar::needle}}/g', replaceString: '{{getglobalvar::replacement}}', substituteRegex: 2 });
  assert.equal(engine.applyRegexRules('a.b axb', [escaped], context({ variables })), '$& axb');
  assert.equal(engine.applyRegexRules('a.b axb', [{ ...escaped, substituteRegex: 1 }], context({ variables })), '$& $&');
});

test('imports accept standalone rules, arrays and character cards, preserve order, and reject unsupported or invalid data', () => {
  const source = { scriptName: 'Imported', findRegex: '/x/g', replaceString: 'y', placement: [1, 2], markdownOnly: false };
  for (const value of [source, [source], { regex_scripts: [source] }, { extensions: { regex_scripts: [source] } }, { data: { extensions: { regex_scripts: [source] } } }]) {
    const imported = plain(engine.importRegexRules(JSON.stringify(value)));
    assert.equal(imported.length, 1);
    assert.equal(imported[0].markdownOnly, false);
    assert.equal(imported[0].promptOnly, false);
    assert.deepEqual(imported[0].placement, [1, 2]);
  }
  for (const patch of [{ findRegex: '/[/' }, { placement: [3] }, { trimStrings: 'bad' },
    { maxDepth: -2 }, { minDepth: 3, maxDepth: 1 }, { disabled: 'yes' }, { substituteRegex: 4 }]) {
    assert.throws(() => engine.importRegexRules(JSON.stringify([{ ...source }, { ...source, ...patch }])));
  }
});

test('invalid configurations fail without partial writes and successful saves commit through VFS', async () => {
  const r = runtime();
  assert.deepEqual(plain(await r.module.readRegexRules()), []);
  await r.module.writeRegexRules([rule()]);
  assert.equal(typeof r.globals.ToolPkg.invalidateMessageTransforms, 'undefined');
  assert.equal(r.writes.length, 1);
  const previous = r.files.get('/config/worldbook/regex_scripts.json');
  await assert.rejects(r.module.writeRegexRules([rule(), rule()]), /ID/);
  assert.equal(r.files.get('/config/worldbook/regex_scripts.json'), previous);
  r.files.set('/config/worldbook/regex_scripts.json', 'broken');
  await assert.rejects(r.module.readRegexRules());
});

test('actual registrations expose source and display replacements and identical prompt mutations for estimates', async () => {
  const r = runtime(true);
  r.module.registerToolPkg();
  assert.equal(r.hooks.routes[0].id, 'worldbook_manager');
  assert.equal(r.hooks.routes[0].route, r.hooks.navigation.find(entry => entry.surface === 'toolbox').route);
  assert.ok(r.hooks.routes.some(route => route.id === 'regex_manager'));
  assert.equal(r.hooks.message.length, 1);
  const send = r.hooks.finalize.find(hook => hook.id === 'worldbook_regex_prompt');
  const estimate = r.hooks.estimate.find(hook => hook.id === 'worldbook_regex_estimate');
  assert.ok(send && estimate);
  r.setRules([rule({ placement: [1, 2], markdownOnly: true, replaceString: 'shown' }),
    rule({ id: 'model', placement: [1, 2, 6], promptOnly: true, replaceString: 'model' })]);
  const event = { eventName: 'before_send_to_model', eventPayload: {
    chatId: 'chat-a', rawInput: 'secret', processedInput: 'secret',
    metadata: { executionContext: { chatId: 'chat-a', participantId: 'card-a' } },
    preparedHistory: [
      { kind: 'SYSTEM', content: 'secret rules' },
      { kind: 'ASSISTANT', content: '<think>secret</think>secret', metadata: { id: 'assistant' } },
      { kind: 'TOOL_CALL', content: 'secret', toolName: 'secret' },
      { kind: 'TOOL_RESULT', content: 'secret', toolName: 'secret' },
      { kind: 'USER', content: 'secret' },
    ],
  } };
  const result = plain(await send.function(event));
  assert.deepEqual(plain(await estimate.function(event)), result);
  assert.equal(result.processedInput, 'model');
  assert.equal(result.preparedHistory[1].content, '<think>model</think>model');
  assert.equal(result.preparedHistory[4].content, 'model');
  assert.equal(result.preparedHistory[0].content, 'secret rules');
  assert.equal(result.preparedHistory[2].toolName, 'secret');
  assert.equal(result.preparedHistory[2].content, 'secret');
  assert.equal(event.eventPayload.preparedHistory[4].content, 'secret');
  const input = { chatId: 'chat-a', timestamp: 123, sender: 'ai', participantId: 'card-a', text: 'secret', kind: 'markdown', depth: 0, isEdit: false, partId: 'part' };
  await assert.rejects(r.hooks.message[0].function({ eventName: 'before_display', eventPayload: input }), /处理阶段/);
  assert.equal(r.hooks.stream.length, 1);
  assert.equal(r.hooks.stream[0].function, r.module.prepareRegexDisplay);
  assert.deepEqual(expand(await r.hooks.stream[0].function({ eventName: 'prepare_display', eventPayload: { chatId: input.chatId, messages: [input] } })), [{
    rules: [{ pattern: 'secret', replacement: 'shown', flags: 'g', kinds: ['markdown'], trimStrings: [] }],
  }]);
  assert.equal(await r.hooks.message[0].function({ eventName: 'before_persist', eventPayload: input }), 'secret');
  await assert.rejects(r.hooks.message[0].function({ eventName: 'unknown', eventPayload: input }), /处理阶段/);
  assert.equal(r.hooks.message[0].function, r.module.onRegexMessageTransform);
  assert.equal(send.function, r.module.onRegexPromptFinalize);
  assert.equal(r.writes.filter(path => path.endsWith('variables.json')).length, 0);
});

test('world-info replacements run before injection wrappers and preserve unrelated text', () => {
  const rules = [rule({ placement: [5], replaceString: 'source' }), rule({ id: 'prompt', placement: [5], promptOnly: true, findRegex: '/source/g', replaceString: 'lore' })];
  assert.equal(engine.regexWorldInfo('secret text', rules, 'card-a', null), 'lore text');
});

test('native preparation resolves context once, keeps capture tokens and scopes rules before reading variables', async () => {
  const r = runtime();
  r.setRules([
    rule({ id: 'dynamic', markdownOnly: true, placement: [2, 6], minDepth: 1, runOnEdit: true,
      characterCardId: 'card-a', findRegex: '/({{getglobalvar::needle}})/g', substituteRegex: 2,
      replaceString: '{{getglobalvar::replacement}}:$1:{{match}}', trimStrings: ['{{getvar::trim}}'] }),
    rule({ id: 'source' }), rule({ id: 'disabled', disabled: true, markdownOnly: true }),
  ]);
  r.files.set('/config/worldbook/variables.json', JSON.stringify({
    global_variables: { needle: 'a.b', replacement: '$1' }, character_variables: {},
    chats: { 'chat-a': { local_variables: { trim: 'e' } } },
  }));
  const event = { eventName: 'prepare_display', eventPayload: { chatId: 'chat-a', messages: [{ sender: 'ai', timestamp: 1,
    participantId: 'card-a', depth: 1, isEdit: true }] } };
  assert.deepEqual(expand(await r.module.prepareRegexDisplay(event)), [{ rules: [{
    pattern: '(a\\.b)', flags: 'g', replacement: '$$1:$1:$0',
    kinds: ['markdown', 'thinking'], trimStrings: ['e'],
  }] }]);
  assert.deepEqual(r.reads, ['/config/worldbook/regex_scripts.json', '/config/worldbook/variables.json']);
  assert.equal(r.writes.length, 0);
  r.reads.length = 0;
  event.eventPayload.messages[0].depth = 0;
  assert.deepEqual(expand(await r.module.prepareRegexDisplay(event)), [{ rules: [] }]);
  assert.deepEqual(r.reads, ['/config/worldbook/regex_scripts.json']);
});

test('runtime errors and default names follow the current host language on every operation', async () => {
  const r = runtime(false, 'en-US');
  assert.equal(r.module.newRegexRule().scriptName, 'New regex');
  assert.throws(() => r.module.compileRegex(''), /search pattern cannot be empty/);
  assert.throws(() => r.module.compileRegex('/[/'), /Invalid search pattern/);
  assert.throws(() => r.module.importRegexRules('{broken'), /Invalid regex JSON/);
  assert.throws(() => r.module.validateRegexRule({ ...rule(), placement: [3] }), /Supported placements/);
  r.files.set('/config/worldbook/regex_scripts.json', '{}');
  await assert.rejects(r.module.readRegexRules(), /configuration must be an array/);
  r.setRules([rule({ markdownOnly: true })]);
  r.files.set('/config/worldbook/variables.json', '{}');
  const event = { eventName: 'prepare_display', eventPayload: { chatId: 'chat', messages: [{ sender: 'ai', participantId: null, depth: 0, isEdit: false }] } };
  await assert.rejects(r.module.prepareRegexDisplay(event), /Invalid worldbook variables file format/);
  r.setLanguage('zh-CN');
  assert.equal(r.module.newRegexRule().scriptName, '新正则');
  assert.throws(() => r.module.compileRegex(''), /查找表达式不能为空/);
  await assert.rejects(r.module.prepareRegexDisplay(event), /世界书变量文件格式无效/);
});

test('one display batch shares reads while preserving depth, sender, participant and fresh variable values', async () => {
  const r = runtime();
  r.setRules([rule({ markdownOnly: true, placement: [2, 6], minDepth: 1, maxDepth: 2,
    replaceString: '{{get_character_variable::name}}' })]);
  const variableFile = '/config/worldbook/variables.json';
  r.files.set(variableFile, JSON.stringify({ global_variables: {}, chats: {},
    character_variables: { a: { name: 'A' }, b: { name: 'B' } } }));
  const messages = Array.from({ length: 80 }, (_, timestamp) => ({ timestamp,
    sender: timestamp % 4 === 0 ? 'user' : 'ai', participantId: timestamp % 2 ? 'a' : 'b',
    depth: timestamp % 4, isEdit: false }));
  const event = { eventName: 'prepare_display', eventPayload: { chatId: 'chat', messages } };
  const batch = plain(await r.module.prepareRegexDisplay(event));
  assert.equal(batch.programs.length, 3);
  assert.equal(batch.messagePrograms.length, 80);
  const programs = expand(batch);
  assert.equal(programs.length, 80);
  for (let index = 0; index < programs.length; index++) {
    const selected = index % 4 === 1 || index % 4 === 2;
    assert.equal(programs[index].rules.length, selected ? 1 : 0);
    if (selected) {
      assert.deepEqual(programs[index].rules[0].kinds, ['markdown', 'thinking']);
      assert.equal(programs[index].rules[0].replacement, JSON.stringify(index % 2 ? 'A' : 'B'));
    }
  }
  assert.deepEqual(r.reads, ['/config/worldbook/regex_scripts.json', variableFile]);
  r.reads.length = 0;
  r.files.set(variableFile, JSON.stringify({ global_variables: {}, chats: {}, character_variables: { a: { name: 'updated' } } }));
  assert.equal(expand(await r.module.prepareRegexDisplay(event))[1].rules[0].replacement, '"updated"');
  assert.deepEqual(r.reads, ['/config/worldbook/regex_scripts.json', variableFile]);
});
