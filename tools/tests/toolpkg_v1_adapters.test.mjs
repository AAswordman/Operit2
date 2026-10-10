import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import vm from 'node:vm';
import test from 'node:test';
import { installScopedHostRuntime } from './fixtures/scoped_host_runtime.mjs';
import { createDiskHarness } from '../../plugins/packages/buildin/character_cards/tests/disk-files.mjs';
import { loadModule } from '../../plugins/packages/buildin/character_cards/tests/runtime.mjs';

const root = new URL('../../', import.meta.url);
const adapterRoot = 'core/crates/plugin/sdk/src/compat/v1/';

/** Reads an actual production source file from the repository. */
function source(path) { return readFileSync(new URL(path, root), 'utf8'); }

/** Converts isolated-realm JSON data for assertions. */
function plain(value) { return JSON.parse(JSON.stringify(value)); }

/** Creates one engine using the production dispatcher, adapters, and installer. */
function engine(version = '1.0.0') {
  const calls = [];
  const files = {};
  // These fixtures model v2 result shapes, not the legacy input convention.
  for (const name of ['list', 'read', 'readPart', 'write', 'writeBinary', 'readBinary', 'deleteFile',
    'exists', 'move', 'copy', 'mkdir', 'find', 'grep', 'grepContext', 'info', 'apply', 'create',
    'edit', 'zip', 'unzip', 'open', 'share', 'download']) {
    /** Captures the actual current signature selected by the production adapter. */
    files[name] = async (...args) => {
      calls.push({ name, args });
      const path = typeof args[0] === 'string' ? args[0] : args[0].path;
      if (name === 'download') {
        return { operation: name, path: typeof args[0] === 'string' ? args[1] : args[0].destination, successful: true, details: 'done' };
      }
      if (name === 'apply' || name === 'edit' || name === 'create') {
        return { operation: { operation: name, path, successful: true, details: 'done' }, aiDiffInstructions: 'instructions' };
      }
      if (name === 'find') return { path, pattern: args[1], files: [path + '/found.txt'] };
      if (name === 'grep' || name === 'grepContext') return {
        searchPath: path, pattern: args[1], totalMatches: 1, filesSearched: 1,
        matches: [{ filePath: path + '/found.txt', lineMatches: [{ lineNumber: 1, lineContent: 'text', matchContext: null }] }],
      };
      return { path, operation: name, successful: true, details: 'done', content: 'text', size: 4, entries: [] };
    };
  }
  const tools = {
    Files: files,
    SoftwareSettings: {},
    Chat: {
      /** Captures the current object-based creation contract. */
      async createNew(options) { calls.push({ name: 'Chat.createNew', args: [options] }); return { chatId: 'new-chat', createdAt: 123 }; },
      /** Captures current native folder membership. */
      async updateGroup(ids, group) { calls.push({ name: 'Chat.updateGroup', args: [ids, group] }); return 'done'; },
      /** Captures the current structured send request. */
      async sendMessage(request) { calls.push({ name: 'Chat.sendMessage', args: [request] }); return { chatId: request.chatId, message: request.input.text, sentAt: 123 }; },
      /** Models the canonical async iterable without legacy callbacks. */
      async *sendMessageStreaming() { throw new Error('unused stream'); },
      /** Supplies a selected existing conversation. */
      async listAll() { return { currentChatId: 'current-chat', chats: [] }; },
      /** Models the current open-string role/result contract. */
      async call(options) {
        calls.push({ name: 'Chat.call', args: [options] });
        return {
          text: 'reply', turns: [{ kind: 'ASSISTANT', content: 'reply', metadata: {} }],
          finishReason: 'stop', metadata: { protocolMeta: [] }, receivedAt: 123,
        };
      },
    },
  };
  const context = vm.createContext({
    version, params: {}, Tools: tools, __operitCurrentCallId: 'call',
    /** Retains the actual owner call while the production dependency callback is outstanding. */
    __operitRetainCallReference(callId) { assert.equal(callId, 'call'); },
    /** Releases the same retained dependency call after native completion. */
    __operitReleaseCallReference(callId) { assert.equal(callId, 'call'); },
    /** Restores the authenticated callback owner before resolving its dependency result. */
    __operitActivateCall(callId) { assert.equal(callId, 'call'); },
    /** Captures executable tool requests and allows contract tests to attach a real provider. */
    async toolCall(name, params) { calls.push({ name, args: [params] }); return context.invokeTool(name, params); },
    /** Rejects unexpected provider requests instead of manufacturing successful empty data. */
    async dependency() { throw new Error('Unexpected dependency invocation'); },
    /** Supplies only the actual modeled builtin send result. */
    async invokeTool(name, params) {
      assert.equal(name, 'send_message_to_ai');
      return { chatId: 'sent-chat', message: params.message, sentAt: 123 };
    },
    /** Supplies the active call's declared contract. */
    __operitGetCallState() { return { params: { ...context.params, __operit_toolpkg_api_version: context.version } }; },
    /** Publishes the runtime namespace in this engine. */
    __operitExpose(name, value) { context[name] = value; },
  });
  const registry=installScopedHostRuntime(context);
  context.__operitCurrentCallId='call';
  context.__operitNativeCallToolStructured=registry.binding((requestId,type,name,params)=>{
    assert.equal(type,'default');calls.push({name,args:[params]});
    Promise.resolve().then(()=>context.invokeTool(name,params)).then(
      data=>registry.settle(requestId,{success:true,data},false),
      error=>registry.settle(requestId,String(error.message),true),
    );
  });
  context.__operitNativeCallDependency=registry.binding((requestId,packageName,method,payload)=>{
    calls.push({name:'dependency.'+method,packageName,args:[payload]});
    Promise.resolve().then(()=>context.dependency(packageName,method,payload)).then(
      value=>registry.settle(requestId,{success:true,value},false),
      error=>registry.settle(requestId,String(error.message),true),
    );
  });
  const rust = source('core/crates/plugin/sdk/src/toolpkg/ToolPkgApiRuntimeScript.rs');
  const start = rust.indexOf('r#"') + 3;
  vm.runInContext(rust.slice(start, rust.indexOf('"#', start)), context);
  for (const file of ['files.js', 'chat.js', 'workflow.js', 'characters.js', 'memory.js', 'install.js']) vm.runInContext(source(adapterRoot + file), context);
  return { context, tools, calls };
}

/** Checks the copied handwritten contract remains byte-identical to its recorded source. */
test('v1 declarations match their recorded source hashes', () => {
  const directory = new URL('plugins/types-v1/', root);
  const provenance = JSON.parse(readFileSync(new URL('source.json', directory), 'utf8'));
  assert.match(provenance.commit, /^[0-9a-f]{40}$/);
  assert.deepEqual(readdirSync(directory).filter(name => name.endsWith('.d.ts')).sort(), Object.keys(provenance.files).sort());
  for (const [file, digest] of Object.entries(provenance.files)) {
    assert.equal(createHash('sha256').update(readFileSync(new URL(file, directory))).digest('hex'), digest, file);
  }
});

/** Checks the shifted legacy zip argument and both path directions. */
test('v1 zip maps environment and restores the source result path', async () => {
  const { tools, calls } = engine();
  const result = await tools.Files.zip('/work', '/archive.zip', 'linux', false);
  assert.deepEqual(plain(calls[0]), { name: 'zip', args: ['/mnt/linux/work', '/mnt/linux/archive.zip', false] });
  assert.equal(result.env, 'linux');
  assert.equal(result.path, '/work');
});

/** Checks default environment semantics do not depend on the machine executing JavaScript. */
test('v1 defaults to its documented Android contract and delegates mounts to the host', async () => {
  const { tools, calls } = engine('1.0.1');
  const result = await tools.Files.read('/storage/emulated/0/file.txt');
  assert.equal(calls[0].args[0].path, '/sdcard/file.txt');
  assert.equal(result.env, 'android');
  assert.equal(result.path, '/storage/emulated/0/file.txt');
  await tools.Files.list('/app/data/plugins/demo');
  assert.equal(calls[1].args[0], '/app/data/plugins/demo');
  await assert.rejects(tools.Files.read('/unsupported-root/file.txt'), /no VFS mapping/);
  await assert.rejects(tools.Files.read('/../../file.txt'), /escapes/);
  await assert.rejects(tools.Files.list('/work', 'invented'), /Unsupported/);
  assert.equal(calls.length, 2);
});

/** Checks legacy options are converted without leaking environment into v2 options. */
test('v1 read options preserve intent and caller ownership', async () => {
  const { tools, calls } = engine();
  const options = Object.freeze({ path: '/work/file', environment: 'linux', intent: 'read', direct_image: false });
  await tools.Files.read(options);
  assert.deepEqual(plain(calls[0].args), [{ path: '/mnt/linux/work/file', intent: 'read', direct_image: false }]);
  assert.equal(options.environment, 'linux');
});

/** Checks the distinct source and destination environment arguments survive adaptation. */
test('v1 cross-environment copy preserves both addresses', async () => {
  const { tools, calls } = engine();
  const result = await tools.Files.copy('/tmp/a', '/sdcard/b', true, 'linux', 'android');
  assert.deepEqual(plain(calls[0].args), ['/mnt/linux/tmp/a', '/sdcard/b', true]);
  assert.equal(result.path, '/tmp/a');
  assert.equal(result.env, 'linux');
});

/** Checks nested operation data and search entries use legacy result shapes. */
test('v1 nested and search results restore environments and paths', async () => {
  const { tools } = engine();
  const edit = await tools.Files.edit('/work/a', 'old', 'new', 'linux');
  assert.equal(edit.operation.env, 'linux');
  assert.equal(edit.operation.path, '/work/a');
  const found = await tools.Files.find('/work', '*.txt', {}, 'linux');
  assert.deepEqual(plain(found.files), ['/work/found.txt']);
  const grep = await tools.Files.grep('/work', 'text', { environment: 'linux', file_pattern: '*.txt' });
  assert.equal(grep.env, 'linux');
  assert.equal(grep.searchPath, '/work');
  assert.equal(grep.filePattern, '*.txt');
  assert.equal(grep.matches[0].filePath, '/work/found.txt');
  assert.deepEqual(plain(grep.matches[0].lineMatches[0]), { lineNumber: 1, lineContent: 'text' });
});

/** Checks headers remain the third current argument rather than the legacy environment. */
test('v1 download supports both handwritten overloads', async () => {
  const { tools, calls } = engine();
  const headers = Object.freeze({ Accept: 'text/plain' });
  await tools.Files.download('https://example.invalid/file', '/out', 'linux', headers);
  assert.deepEqual(plain(calls[0].args), ['https://example.invalid/file', '/mnt/linux/out', headers]);
  const options = Object.freeze({ visit_key: 'visit', link_number: 2, destination: '/out', environment: 'linux', headers });
  const result = await tools.Files.download(options);
  assert.deepEqual(plain(calls[1].args), [{ visit_key: 'visit', link_number: 2, destination: '/mnt/linux/out', headers }]);
  assert.equal(result.path, '/out');
  assert.equal(result.env, 'linux');
  assert.equal(options.destination, '/out');
});

/** Checks independent calls sharing an engine never inherit another call's contract. */
test('v2 keeps its original signatures and receives no legacy result fields', async () => {
  const { tools, calls, context } = engine();
  await tools.Files.zip('/work', '/out', 'linux', false);
  context.version = '2.0.0';
  const result = await tools.Files.zip('/app/workspaces/a', '/app/data/out', false);
  assert.deepEqual(plain(calls[1].args), ['/app/workspaces/a', '/app/data/out', false]);
  assert.equal(Object.hasOwn(result, 'env'), false);
  context.version = '1.0.1';
  assert.equal((await tools.Files.list('/work', 'linux')).env, 'linux');
});

/** Checks host errors propagate unchanged and never trigger another implementation. */
test('v1 adapter preserves a host rejection without retry', async () => {
  const context = vm.createContext({});
  vm.runInContext(source(adapterRoot + 'files.js'), context);
  const error = new Error('mount unavailable');
  let count = 0;
  const current = {
    /** Rejects the exact operation selected by the compatibility adapter. */
    async zip() { count++; throw error; },
  };
  const legacy = context.__operitCreateV1Files(current);
  await assert.rejects(legacy.zip('/work', '/out', 'linux', false), caught => caught === error);
  assert.equal(count, 1);
});

/** Checks the actual v1 nullable field converts to the v2 optional field. */
test('v1.0.1 Chat.call converts nullable tool names and preserves caller data', async () => {
  const { tools, calls } = engine('1.0.1');
  const turn = Object.freeze({ kind: 'USER', content: 'hello', toolName: null });
  const result = await tools.Chat.call({ functionType: 'CHAT', turns: [turn] });
  assert.deepEqual(plain(calls[0].args[0].turns), [{ kind: 'USER', content: 'hello' }]);
  assert.equal(turn.toolName, null);
  assert.equal(result.finishReason, 'stop');
  assert.equal(result.turns[0].kind, 'ASSISTANT');
  const old = engine('1.0.0');
  assert.throws(() => old.tools.Chat.call({ functionType: 'CHAT', turns: [turn] }), /requires/);
  assert.equal(old.calls.length, 0);
});

/** Checks new v2 enum values cannot leak into the closed v1 return contract. */
test('v1 Chat.call rejects unrepresentable current results', async () => {
  const context = vm.createContext({});
  vm.runInContext(source(adapterRoot + 'chat.js'), context);
  const response = { text: '', turns: [], finishReason: 'new_v2_reason', metadata: {}, receivedAt: 1 };
  /** Supplies a current result with a deliberately unsupported legacy value. */
  const call = context.__operitCreateV1ChatCall(async () => response);
  const request = { functionType: 'CHAT', turns: [] };
  await assert.rejects(call(request), /finish reason/);
  response.finishReason = 'stop';
  response.turns = [{ kind: 'NEW_ROLE', content: '', metadata: {} }];
  await assert.rejects(call(request), /turn kind/);
});

/** Exercises the active-call getter across both contracts without aliasing another provider's participant. */
test('legacy caller IDs are provider-scoped and absent from the current contract', () => {
  const { context } = engine();
  context.params = { __operit_package_caller_participant_id: 'card-one', __operit_package_caller_owner: 'com.operit.character_cards' };
  assert.equal(context.getCallerCardId(), 'card-one');
  const retainedGetter = context.getCallerCardId;
  context.version = '2.0.0';
  assert.equal(context.getCallerCardId, undefined);
  assert.throws(() => retainedGetter(), /no implementation/);
  context.version = '1.0.1';
  context.params.__operit_package_caller_participant_id = 'card-two';
  assert.equal(context.getCallerCardId(), 'card-two');
  context.params.__operit_package_caller_owner = 'another.provider';
  assert.throws(() => context.getCallerCardId(), /another provider participant/);
  context.params = {};
  assert.equal(context.getCallerCardId(), undefined);
});

/** Ensures this task never registers the legacy settings-management methods. */
test('legacy role compatibility leaves SoftwareSettings unchanged', () => {
  const { tools } = engine();
  assert.deepEqual(Object.keys(tools.SoftwareSettings), []);
  assert.doesNotMatch(source('plugins/types/index.d.ts'), /function getCallerCardId\(/);
  assert.match(source('plugins/types/index.d.ts'), /function getCallerParticipantId\(\): string \| undefined/);
});

/** Exercises genuine public API handlers and memory tool IPC against the existing disk-backed plugin. */
async function roleProvider(t, version = '1.0.0') {
  const disk = await createDiskHarness(t), apis = new Map();
  const plugin = loadModule('src/public-api.ts', { ...disk.globals, ToolPkg: {
    ...disk.globals.ToolPkg,
    /** Captures the actual exported callbacks registered by the production provider. */
    registerApi(entry) { assert.equal(apis.has(entry.name), false); apis.set(entry.name, entry.function); },
  } });
  plugin.registerDomainApis();
  /** Calls an actual registered provider handler without reproducing its business logic. */
  async function domain(method, payload) {
    assert.equal(apis.has(method), true, method);
    return apis.get(method)({ payload, callerPackage: 'legacy.test.package' });
  }
  const exportedTools = loadModule('src/runtime-tools/tools.ts', { ...disk.globals, ToolPkg: {
    ...disk.globals.ToolPkg, ipc: {
      /** Routes the executable tool's real IPC request back to the one authoritative provider service. */
      async call(channel, request, options) {
        assert.equal(channel, 'character-memory.domain');
        assert.deepEqual(plain(options), { targetRuntime: 'main' });
        return domain(request.operation, request.input);
      },
    },
  } });
  const runtime = engine(version);
  runtime.context.params = {
    __operit_package_caller_participant_id: 'default', __operit_package_caller_owner: 'com.operit.character_cards',
    __operit_package_chat_id: 'actual-chat', __operit_package_caller_name: 'Actual speaker',
  };
  /** Sends dependency transport requests to genuine registered callbacks. */
  runtime.context.dependency = async (packageName, method, payload) => {
    assert.equal(packageName, 'com.operit.character_cards');
    return domain(method, payload);
  };
  /** Dispatches the exact package tool name to the actual executable plugin export. */
  runtime.context.invokeTool = async (name, params) => {
    const match = /^character_memory:([a-z_]+)$/.exec(name);
    assert.notEqual(match, null, name);
    assert.equal(typeof exportedTools[match[1]], 'function');
    return JSON.stringify(await exportedTools[match[1]](params));
  };
  const registry=runtime.context.__operitHostPromiseRegistry;
  runtime.context.__operitNativeCallToolStructured=registry.binding((requestId,type,name,params)=>{
    assert.equal(type,'default');
    runtime.calls.push({name,args:[params]});
    Promise.resolve().then(()=>runtime.context.invokeTool(name,params)).then(
      data=>registry.settle(requestId,{success:true,data},false),
      error=>registry.settle(requestId,String(error.message),true),
    );
  });
  return { ...runtime, disk, domain };
}

/** Verifies legacy role reads and chat creation use actual stored roles without interpreting settings bindings. */
test('legacy chat roles and creation use the real prerequisite provider', async t => {
  const runtime = await roleProvider(t);
  const library = await runtime.domain('memory.shared.create', { name: 'Actual shared library' });
  const card = await runtime.domain('character.create', { values: { name: 'Real legacy role', chatModelBindingMode: 'FIXED_MODEL', chatModelId: 'configured-model', memoryBindingMode: 'SHARED', sharedMemoryId: library.id } });
  const listed = await runtime.tools.Chat.listCharacterCards();
  assert.ok(listed.cards.some(entry => entry.id === card.id && entry.name === card.name));
  const created = await runtime.tools.Chat.createNew('native-folder', false, card.id);
  assert.equal(created.chatId, 'new-chat');
  const calls = runtime.calls.filter(entry => entry.name.startsWith('Chat.'));
  assert.deepEqual(plain(calls), [
    { name: 'Chat.createNew', args: [{ setAsCurrentChat: false, sourceChatId: null, input: { 'com.operit.character_cards': { version: 1, selection: 'card:' + card.id } } }] },
    { name: 'Chat.updateGroup', args: [['new-chat'], 'native-folder'] },
  ]);
  const before = runtime.calls.filter(entry => entry.name === 'Chat.createNew').length;
  await assert.rejects(runtime.tools.Chat.createNew(undefined, undefined, 'missing-role'), /character does not identify exactly one record: missing-role/);
  assert.equal(runtime.calls.filter(entry => entry.name === 'Chat.createNew').length, before);
  runtime.context.version = '2.0.0';
  const options = Object.freeze({ sourceChatId: 'source-chat', input: { owner: 'opaque' } });
  await runtime.tools.Chat.createNew(options);
  assert.deepEqual(plain(runtime.calls.at(-1).args), [options]);
});

/** Confirms ordinary legacy sends preserve native controls while translating only the role identity field. */
test('legacy ordinary chat sends retain positional controls and sender identity', async () => {
  const { context, tools, calls } = engine();
  context.dependency = async () => ({ id: 'role-one' });
  const options = Object.freeze({ runtime: 'floating', persist_turn: false, notify_reply: true, hide_user_message: true, disable_warning: true, timeout_ms: 1000 });
  await tools.Chat.sendMessage('hello', 'chat-one', 'role-one', 'Speaker', options);
  assert.deepEqual(plain(calls.at(-1)), { name: 'send_message_to_ai', args: [{ ...options, message: 'hello', chat_id: 'chat-one', participant_id: 'role-one', sender_name: 'Speaker' }] });
  context.version = '2.0.0';
  const request = Object.freeze({ kind: 'submit', chatId: 'current', input: { text: 'v2' } });
  await tools.Chat.sendMessage(request);
  assert.deepEqual(plain(calls.at(-1)), { name: 'Chat.sendMessage', args: [request] });
});

/** Checks both legacy overloads perform real persisted writes and preserve the structured query result. */
test('legacy memory operations reach the actual disk-backed provider', async t => {
  const { tools, context, disk, calls } = await roleProvider(t);
  const options = Object.freeze({ title: 'First memory', content: 'Persisted first content', callerCardId: 'default', tags: 'tag-one' });
  assert.match(await tools.Memory.create(options), /Successfully created/);
  await tools.Memory.create('Second memory', 'Persisted second content');
  assert.deepEqual(options, { title: 'First memory', content: 'Persisted first content', callerCardId: 'default', tags: 'tag-one' });
  const updates = Object.freeze({ content: 'Actually updated content', importance: 0.8 });
  await tools.Memory.update('First memory', updates, 'default');
  const query = await tools.Memory.query({ query: '*', callerCardId: 'default' });
  assert.equal(query.memories.length, 2);
  assert.ok(query.memories.some(item => item.title === 'First memory' && item.content === updates.content));
  assert.equal(typeof query.snapshotId, 'string');
  assert.equal(query.snapshotCreated, true);
  assert.equal(Object.hasOwn(query, 'ownerKey'), false);
  assert.equal(calls.at(-1).args[0].limit, 20);
  const fetched = JSON.parse(await tools.Memory.getByTitle('First memory'));
  assert.equal(fetched.memories[0].content, updates.content);
  assert.equal(Object.hasOwn(fetched, 'ownerKey'), false);
  const requests = calls.filter(call => call.name.startsWith('character_memory:'));
  for (const call of requests) {
    assert.equal(call.args[0].__operit_package_caller_participant_id, 'default');
    assert.equal(call.args[0].__operit_package_caller_owner, 'com.operit.character_cards');
  }
  await tools.Memory.move({ targetFolderPath: 'actual-folder', titles: ['First memory'] });
  const stored = disk.readState().owners.find(space => space.ownerKey === 'character:default');
  assert.equal(stored.memories.find(item => item.title === 'First memory').folderPath, 'actual-folder');
  await assert.rejects(tools.Memory.query('*', undefined, undefined, undefined, undefined, undefined, undefined, 'forged-role'), /authenticated execution participant/);
  context.params.__operit_package_caller_owner = 'another.provider';
  const beforeForeign = calls.length;
  await assert.rejects(tools.Memory.query('*'), /another provider participant/);
  assert.equal(calls.length, beforeForeign);
  context.params = {};
  const count = calls.length;
  await assert.rejects(tools.Memory.query({ query: '*', __operit_package_caller_participant_id: 'forged' }), /Unknown legacy memory option/);
  assert.equal(calls.length, count);
  await tools.Memory.deleteMemory('Second memory', 'default');
  assert.equal(disk.readState().owners.find(space => space.ownerKey === 'character:default').memories.length, 1);
});

/** Verifies lossless numeric relationship IDs round-trip through actual create, query, update and delete operations. */
test('legacy numeric memory links round-trip through decimal-string provider identities', async t => {
  const { tools, calls } = await roleProvider(t, '1.0.1');
  await tools.Memory.create('Source', 'Source content');
  await tools.Memory.create('Target', 'Target content');
  const created = await tools.Memory.link({ sourceTitle: 'Source', targetTitle: 'Target', description: 'Actual link' });
  assert.equal(created.description, 'Actual link');
  const query = await tools.Memory.queryLinks();
  assert.equal(query.totalCount, 1);
  assert.ok(Number.isSafeInteger(query.links[0].linkId));
  const updated = await tools.Memory.updateLink({ linkId: query.links[0].linkId, weight: 0.9 });
  assert.equal(updated.weight, 0.9);
  assert.equal(calls.at(-1).args[0].link_id, String(query.links[0].linkId));
  await tools.Memory.deleteLink(query.links[0].linkId);
  assert.equal((await tools.Memory.queryLinks()).totalCount, 0);
  const count = calls.length;
  await assert.rejects(tools.Memory.queryLinks(Number.MAX_SAFE_INTEGER + 1), /positive safe integer/);
  assert.equal(calls.length, count);
});

/** Validates the old callback and Promise contract against growing semantic snapshots and the final native receipt. */
test('legacy chat streaming projects growing snapshots and preserves completion', async () => {
  const events = [], result = { chatId: 'current-chat', message: 'hello', aiResponse: 'Hello world', sentAt: 12 };
  const { context } = engine();
  const adapter = context.__operitCreateV1Chat({
    /** Resolves the documented current-chat selection from the canonical chat list. */
    async listAll() { return { currentChatId: 'current-chat' }; },
    /** Emits genuine atomic snapshot shapes followed by one actual receipt. */
    async *sendMessageStreaming(request) {
      assert.equal(request.chatId, 'current-chat');
      yield { type: 'part', parts: [{ kind: 'thinking', content: 'private thought' }, { kind: 'markdown', content: 'Hello' }] };
      yield { type: 'part', parts: [{ kind: 'markdown', content: 'Hello world' }] };
      yield { type: 'completed', result };
    },
  }, async () => {}, async () => {}, async () => {});
  assert.equal(await adapter.sendMessageStreaming('hello', undefined, undefined, undefined, {
    /** Captures the original legacy callback envelope. */
    onIntermediateResult(event) { events.push(plain(event)); },
  }), result);
  assert.deepEqual(events.map(event => [event.type, event.chunk, event.receivedChars]), [['start', undefined, 0], ['chunk', 'Hello', 5], ['chunk', ' world', 11]]);
  assert.deepEqual(events.slice(1).map(event => event.chunkIndex), [0, 1]);
});

/** Ensures malformed large identities, real host failures and rewritten snapshots never become successful empty results. */
test('legacy adapters reject precision loss and propagate original provider failures', async t => {
  const runtime = await roleProvider(t);
  const failure = new Error('Exact disk failure');
  runtime.disk.failNext('storage.commit', failure);
  await assert.rejects(runtime.tools.Memory.create('Failed', 'Must not exist'), caught => caught.message === failure.message);
  const context = vm.createContext({});
  vm.runInContext(source(adapterRoot + 'memory.js'), context);
  const memory = context.__operitCreateV1Memory(async () => ({ totalCount: 1, links: [{ linkId: '9007199254740993', sourceTitle: 'a', targetTitle: 'b', linkType: 'related', weight: 0.7, description: '' }] }));
  await assert.rejects(memory.queryLinks(), /safe integer range/);
});

/** Checks that append-only consumers receive a precise rewrite error while receipt-only consumers can accept revised snapshots. */
test('legacy streams reject published text rewrites and missing completion receipts', async () => {
  const { context } = engine();
  let cancelled = false;
  const result = { chatId: 'chat-one', message: 'hello', aiResponse: 'Revised', sentAt: 1 };
  const adapter = context.__operitCreateV1Chat({
    /** Emits an actual text revision and records early stream cancellation. */
    async *sendMessageStreaming() {
      try {
        yield { type: 'part', parts: [{ kind: 'markdown', content: 'Original' }] };
        yield { type: 'part', parts: [{ kind: 'markdown', content: 'Revised' }] };
        yield { type: 'completed', result };
      } finally { cancelled = true; }
    },
  }, async () => {}, async () => {}, async () => {});
  await assert.rejects(adapter.sendMessageStreaming('hello', 'chat-one', undefined, undefined, {
    /** Represents an append-only legacy text consumer. */
    onIntermediateResult() {},
  }), /cannot retract text/);
  assert.equal(cancelled, true);
  assert.equal(await adapter.sendMessageStreaming('hello', 'chat-one'), result);
  const incomplete = context.__operitCreateV1Chat({
    /** Ends a real iterable without its required committed receipt. */
    async *sendMessageStreaming() { yield { type: 'part', parts: [] }; },
  }, async () => {}, async () => {}, async () => {});
  await assert.rejects(incomplete.sendMessageStreaming('hello', 'chat-one'), /without its committed completion/);
});

/** Confirms the role ID getter and legacy memory operations preserve genuine shared-memory bindings. */
test('legacy role callers keep the actual shared memory binding', async t => {
  const { domain, context, tools, disk } = await roleProvider(t);
  const library = await domain('memory.shared.create', { name: 'Real shared memory' });
  const card = await domain('character.create', { values: { name: 'Shared memory caller', memoryBindingMode: 'SHARED', sharedMemoryId: library.id } });
  context.params.__operit_package_caller_participant_id = card.id;
  assert.equal(context.getCallerCardId(), card.id);
  await tools.Memory.create('Shared item', 'Must be saved in the bound library');
  const shared = disk.readState().owners.find(owner => owner.ownerKey === 'shared:' + library.id);
  assert.equal(shared.memories.length, 1);
  assert.equal(shared.memories[0].title, 'Shared item');
  const query = await tools.Memory.query('*');
  assert.equal(query.memories[0].title, 'Shared item');
});
