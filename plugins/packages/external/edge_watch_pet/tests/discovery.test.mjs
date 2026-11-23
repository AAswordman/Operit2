import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';

const require = createRequire(import.meta.url);
const ts = require('../../../../../.ci-tools/typescript/node_modules/typescript');
const config = { version: 1, accent: 'mint', haptics: true, keepAwake: false };

/** Loads the actual driver against an observable typed Space and Edge boundary. */
function fixture(nodes, responses = {}) {
  const calls = [], module = { exports: {} };
  runInNewContext(ts.transpileModule(readFileSync(new URL('../src/driver.ts', import.meta.url), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText, {
    module, exports: module.exports,
    async toolCall(name, args) {
      assert.equal(name, 'list_core_nodes'); assert.equal(Object.keys(args).length, 0);
      return JSON.stringify({ currentNodeId: 'local', nodes });
    },
    Tools: { device(nodeId, options) {
      assert.equal(options.timeoutMs, 12000);
      return { Edge: { async execute(info, args) {
        assert.equal(info.target, 'edge.watchPet'); assert.equal(info.method, 'getConfig'); assert.equal(Object.keys(args).length, 0);
        calls.push(nodeId);
        if (responses[nodeId] instanceof Error) throw responses[nodeId];
        return { data: responses[nodeId] ?? config };
      } } };
    } },
  });
  return { driver: module.exports, calls };
}

/** Provides the same structured fields as list_core_nodes, including non-executable Edge nodes. */
function node(nodeId, fields = {}) {
  return { nodeId, displayName: '我的 手表', platform: 'android-edge', model: 'Round watch', reachable: true, canExecute: false, ...fields };
}

test('discovers directly connected watch services and excludes local, offline, Core and relayed devices', async () => {
  const f = fixture([node('local'), node('offline', { reachable: false }), node('core', { platform: 'android' }),
    node('sensor', { platform: 'esp32' }), node('relay', { relayHops: 2 }), node('watch'), node('watch')]);
  const result = await f.driver.discoverWatches();
  assert.equal(result.watches.length, 1); assert.equal(result.watches[0].target.nodeId, 'watch');
  assert.equal(result.watches[0].target.label, '我的 手表'); assert.equal(result.issues.length, 0);
  assert.deepEqual(f.calls, ['watch']);
});

test('keeps every verified watch for selection and exposes failed capability probes', async () => {
  const f = fixture([node('one'), node('two'), node('old'), node('broken')], {
    old: Error('METHOD_NOT_FOUND'), broken: { version: 3 },
  });
  const result = await f.driver.discoverWatches();
  assert.equal(result.watches.length, 2); assert.equal(result.issues.length, 2);
  assert.equal(result.issues[0].message, 'Error: METHOD_NOT_FOUND');
  assert.match(result.issues[1].message, /字段不受支持/);
});

test('selects by the exact live node identity and rejects names or disconnected selections', async () => {
  const f = fixture([node('one'), node('two', { reachable: false })]);
  assert.equal((await f.driver.connectWatch('one')).target.nodeId, 'one');
  await assert.rejects(f.driver.connectWatch('我的 手表'), /已断开/);
  await assert.rejects(f.driver.connectWatch('two'), /已断开/);
  await assert.rejects(f.driver.connectWatch(undefined), /请选择/);
  assert.deepEqual(f.calls, ['one']);
});

test('an empty Space produces an empty list and malformed membership is an explicit error', async () => {
  assert.equal((await fixture([]).driver.discoverWatches()).watches.length, 0);
  await assert.rejects(fixture([{ nodeId: 'bad' }]).driver.discoverWatches(), /设备信息无效/);
});

/** Loads the actual main-runtime selection controller with observable persistence and discovery. */
function selectionFixture(watches, previous = null) {
  let selected = previous;
  const writes = [], module = { exports: {} };
  /** Models only the retained owner's selection contract for controller boundary checks. */
  class WatchService {
    async selected() { return selected; }
    async select(target) { selected = target; writes.push(target); }
  }
  runInNewContext(ts.transpileModule(readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText, {
    module, exports: module.exports, ToolPkg: { ipc: { on() {} } },
    require(path) { return path === './service' ? { WatchService } : {
      async discoverWatches() { return { watches, issues: [] }; },
      async connectWatch(nodeId) { return watches.find(watch => watch.target.nodeId === nodeId); },
    }; },
  });
  return { selection: module.exports.selection, writes };
}

test('main owner auto-selects one watch, waits for multiple, and preserves a verified previous choice', async () => {
  const one = { target: { nodeId: 'one', label: '第一块' }, config, model: 'Round' };
  const two = { target: { nodeId: 'two', label: '第二块' }, config, model: 'Round' };
  const single = selectionFixture([one]);
  assert.equal((await single.selection({ action: 'discover', previous: null })).connected.target.nodeId, 'one');
  const multiple = selectionFixture([one, two]);
  assert.equal((await multiple.selection({ action: 'discover', previous: null })).connected, null);
  assert.equal(multiple.writes[0], null);
  const retained = selectionFixture([one, two], two.target);
  assert.equal((await retained.selection({ action: 'discover', previous: null })).connected.target.nodeId, 'two');
  const stale = selectionFixture([], one.target);
  assert.equal((await stale.selection({ action: 'discover', previous: null })).connected, null);
  assert.equal(stale.writes[0], null);
});
