import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { installScopedHostRuntime, installGeneratedToolsRuntime } from './fixtures/scoped_host_runtime.mjs';

const bridgeSource = readFileSync(new URL('../../core/crates/plugin/javascript-bridge/src/javascript/JsJavaBridge.rs', import.meta.url), 'utf8');
const bridge = bridgeSource.slice(bridgeSource.indexOf('r#"') + 3, bridgeSource.indexOf('"#'));

/** Uses the actual Java proxy, generated SDK, v1 installer and adapter over a controlled native boundary.
 * The only persistence boundary is the existing installation-owned getEnv/setEnv transport. */
function engine(version = '1.0.0', disk = new Map(), owner = 'messenger') {
  let fail = false;
  const calls = [];
  const context = vm.createContext({
    __operitNativeGetEnvForCall(callId, key) {
      calls.push({op: 'read', callId, key});
      return disk.get(JSON.stringify([owner, key])) ?? null;
    },
    __operitNativeSetEnv(callId, key, value) {
      if (fail) throw new Error('disk unavailable');
      calls.push({op: 'write', callId, key, value});
      disk.set(JSON.stringify([owner, key]), value);
    },
    __operitNativeJavaClassExists: () => false,
    __operitNativeJavaGetApplicationContext: () => ({__javaClass: 'android.app.Application', __javaHandle: 'context'}),
    __operitNativeJavaGetCurrentActivity: () => null,
    __operitNativeJavaCallInstance: (_, method) => {
      if (method === 'toString') return '[ApplicationContext]';
      if (method === 'getApplicationContext') return {__javaClass: 'android.app.Application', __javaHandle: 'context'};
      throw new Error('Unsupported native Java context method');
    },
    __operitNativeJavaNewInstance: () => {}, __operitNativeJavaCallStatic: () => {},
    __operitNativeJavaCallStaticSuspend: () => {}, __operitNativeJavaGetStaticField: () => {}, __operitNativeJavaSetStaticField: () => {},
  });
  const registry = installScopedHostRuntime(context);
  vm.runInContext('globalThis.window = globalThis', context);
  vm.runInContext(bridge, context);
  installGeneratedToolsRuntime(context, version);
  const run = code => vm.runInContext(code, context);
  run('var prefs = Java.getApplicationContext().getSharedPreferences("toolpkg_messenger", 0);');
  return {context, calls, disk, registry, run, fail(value) { fail = value; }};
}

for (const version of ['1.0.0', '1.0.1']) {
  test(`${version}: Java preferences preserve synchronous reads and publish chained editors in one config write`, () => {
    const h = engine(version);
    assert.equal(h.run('prefs.getBoolean("bridge_enabled", false)'), false);
    assert.equal(h.run('prefs.edit().putBoolean("bridge_enabled", true).putString("bridges_json", "[]").putInt("limit", 5).commit()'), true);
    assert.equal(h.run('prefs.getBoolean("bridge_enabled", false)'), true);
    assert.equal(h.run('prefs.getString("bridges_json", "missing")'), '[]');
    assert.equal(h.run('prefs.getInt("limit", 0)'), 5);
    assert.equal(h.run('typeof prefs.getBoolean("bridge_enabled", false)'), 'boolean');
    assert.equal(h.run('prefs.edit().putBoolean("bridge_enabled", false).apply()'), undefined);
    assert.equal(h.run('Java.getApplicationContext().call("getSharedPreferences", "toolpkg_messenger", 0).getBoolean("bridge_enabled", true)'), false);
    const writes = h.calls.filter(c => c.op === 'write');
    assert.equal(writes.length, 2);
    assert.equal(Object.keys(JSON.parse(writes[0].value).values).length, 3);
    assert.ok(h.calls.every(c => c.callId === 'owner'));
    assert.ok(writes.every(c => c.key.startsWith('__OPERIT_V1_PREFS_')));
  });
}

test('fresh engines read saved config while names and plugin owners remain isolated', () => {
  const h = engine();
  h.run('prefs.edit().putString("bridges_json", "[{\\"id\\":\\"bridge_1\\"}]").apply()');
  const next = engine('1.0.0', h.disk);
  assert.equal(next.run('JSON.parse(prefs.getString("bridges_json", "[]"))[0].id'), 'bridge_1');
  assert.equal(next.run('Java.getApplicationContext().getSharedPreferences("different", 0).contains("bridges_json")'), false);
  assert.equal(engine('1.0.0', h.disk, 'different-plugin').run('prefs.contains("bridges_json")'), false);
});

test('editors stage values, clear before puts, remove null strings and refresh other facades', () => {
  const h = engine();
  h.run('prefs.edit().putString("old", "old").apply(); var edit = prefs.edit().putString("new", "new").clear();');
  assert.equal(h.run('prefs.contains("new")'), false);
  h.run('prefs.edit().putString("interleaved", "remove").apply(); edit.apply();');
  assert.deepEqual(JSON.parse(h.run('JSON.stringify(prefs.getAll())')), {new: 'new'});
  h.run('prefs.edit().putString("new", null).putStringSet("letters", new Set(["b", "a", "a"])).apply()');
  assert.equal(h.run('prefs.contains("new")'), false);
  assert.equal(h.run('prefs.getStringSet("letters", null).size'), 2);
  h.run('prefs.getStringSet("letters", null).add("leak")');
  assert.equal(h.run('prefs.getStringSet("letters", null).has("leak")'), false);
  h.run('prefs.edit().remove("letters").apply()');
  assert.equal(h.run('prefs.contains("letters")'), false);
});

test('typed numeric values reject lossy writes and mismatched getters', () => {
  const h = engine();
  h.run('prefs.edit().putInt("zero", 0).putLong("ts", 1780000000000).putFloat("ratio", 0.1).putString("empty", "").apply()');
  assert.equal(h.run('prefs.getInt("zero", 99)'), 0);
  assert.equal(h.run('prefs.getString("empty", "default")'), '');
  assert.equal(h.run('prefs.getLong("ts", 0)'), 1780000000000);
  assert.equal(h.run('prefs.getFloat("ratio", 0)'), Math.fround(0.1));
  assert.throws(() => h.run('prefs.getInt("ratio", 0)'), /type mismatch/);
  for (const code of ['putInt("x", 2147483648)', 'putLong("x", 9007199254740992)', 'putFloat("x", Infinity)', 'putBoolean("x", 1)', 'putStringSet("x", ["a", 1])']) {
    assert.throws(() => h.run(`prefs.edit().${code}`), /Invalid SharedPreferences/);
  }
});

test('prototype-like keys remain values and failed commits cannot report success', () => {
  const h = engine();
  h.run('prefs.edit().putString("__proto__", "safe").putString("constructor", "value").apply()');
  assert.equal(h.run('prefs.getAll()["__proto__"]'), 'safe');
  h.run('var pending = prefs.edit().putInt("retry", 42)');
  h.fail(true);
  assert.equal(h.run('pending.commit()'), false);
  assert.throws(() => h.run('pending.apply()'), /disk unavailable/);
  h.fail(false);
  assert.equal(h.run('pending.commit()'), true);
  assert.equal(h.run('prefs.getInt("retry", 0)'), 42);
});

test('v2 cannot call the compatibility method or reuse a retained v1 preference object', () => {
  const h = engine();
  h.context.version = '2.0.0';
  assert.throws(() => h.run('Java.getApplicationContext().getSharedPreferences("x", 0)'), /Unsupported native Java/);
  assert.throws(() => h.run('prefs.getBoolean("x", false)'), /requires|no implementation/);
});

test('Java aliases and context methods preserve delegation without native Java changes', () => {
  const h = engine();
  h.run('Kotlin.getApplicationContext().getSharedPreferences("aliases", 0).edit().putInt("value", 7).apply()');
  assert.equal(h.run('Java.getContext().getApplicationContext().getSharedPreferences("aliases", 0).getInt("value", 0)'), 7);
  assert.equal(h.run('Java.getContext().call("getApplicationContext").getSharedPreferences("aliases", 0).getInt("value", 0)'), 7);
  assert.equal(h.run('Java.getContext().toString()'), '[ApplicationContext]');
  assert.equal(h.run('Java.getContext().call("toString")'), '[ApplicationContext]');
  h.run('var retainedContext = Java.getContext(); var retainedPreferences = retainedContext.getSharedPreferences');
  h.context.version = '2.0.0';
  assert.throws(() => h.run('retainedContext.getSharedPreferences("aliases", 0)'), /Unsupported native Java/);
  assert.throws(() => h.run('retainedPreferences("aliases", 0).getInt("value", 0)'), /requires|no implementation/);
});

test('encoded preference names stay distinct and unrelated configuration survives', () => {
  const h = engine();
  h.disk.set(JSON.stringify(['messenger', 'PLUGIN_API_KEY']), 'untouched');
  for (const name of ['', 'a-b', 'a_b', '设置', '\ud800']) {
    h.context.preferenceName = name;
    h.run('Java.getContext().getSharedPreferences(preferenceName, 0).edit().putString("name", preferenceName).apply()');
  }
  for (const name of ['', 'a-b', 'a_b', '设置', '\ud800']) {
    h.context.preferenceName = name;
    assert.equal(h.run('Java.getContext().getSharedPreferences(preferenceName, 0).getString("name", null)'), name);
  }
  assert.equal(h.disk.get(JSON.stringify(['messenger', 'PLUGIN_API_KEY'])), 'untouched');
  const firstWrite = h.calls.find(c => c.op === 'write');
  h.disk.set(JSON.stringify(['messenger', firstWrite.key]), '{"format":2,"values":{}}');
  assert.throws(() => h.run('Java.getContext().getSharedPreferences("", 0).getAll()'), /Invalid v1 preference/);
});

/** Runs the exact downloaded market package when supplied, without checking external plugin code into the repo. */
test('unmodified marketplace Messenger saves settings, toggles and injects source messages', {skip: !process.env.OPERIT_MARKET_MESSENGER_DIR}, async () => {
  const h = engine();
  const handlers = new Map();
  h.context.ToolPkg = {ipc: {on(name, fn) { handlers.set(name, fn); }}};
  h.context.getChatId = () => 'target-chat';
  let messageTimestamp = 1780000000000;
  const toolCalls = [];
  h.context.__operitNativeCallToolStructured = h.registry.binding((requestId, type, name, params) => {
    toolCalls.push({name, params});
    assert.equal(name, 'get_chat_messages');
    queueMicrotask(() => h.registry.settle(requestId, {success: true, data: {
      chatId: 'source-chat', messages: [{sender: 'user', content: 'Market original message', timestamp: messageTimestamp}],
    }}, false));
  });
  const directory = process.env.OPERIT_MARKET_MESSENGER_DIR;
  h.context.require = path => {
    assert.equal(path, './ui/settings.ui.js');
    const ui = readFileSync(join(directory, 'dist/ui/settings.ui.js'), 'utf8');
    return h.run('(function(exports){' + ui + ';return exports;})({})');
  };
  const main = readFileSync(join(directory, 'dist/main.js'), 'utf8');
  h.run('var messenger = (function(exports,require){' + main + ';return exports;})({},require)');
  const input = {eventPayload: {stage: 'before_process', processedInput: 'Current input'}};
  assert.equal(await h.context.messenger.onPromptInput(input), null);
  await handlers.get('bridge.save_bridges')({bridges: [{id: 'one', enabled: true, sourceChatId: 'source-chat', sourceChatName: 'Source', targets: [], messageLimit: 5}]});
  h.context.messenger.onInputMenuToggle({eventPayload: {action: 'toggle'}});
  assert.equal((await handlers.get('bridge.get_state')()).enabled, true);
  const injected = await h.context.messenger.onPromptInput(input);
  assert.match(injected, /Current input.*<attachment/s);
  assert.match(injected, /Market original message/);
  assert.equal(await h.context.messenger.onPromptInput(input), null, 'only-new timestamps must persist');
  messageTimestamp += 1000;
  assert.match(await h.context.messenger.onPromptInput(input), /Market original message/);
  h.context.messenger.onInputMenuToggle({eventPayload: {action: 'toggle'}});
  assert.equal(await h.context.messenger.onPromptInput(input), null);
  assert.equal(engine('1.0.0', h.disk).run('prefs.getBoolean("bridge_enabled", true)'), false);
  assert.equal(toolCalls.length, 3);
});
