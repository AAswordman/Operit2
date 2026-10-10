import { composeStreamFixture } from '../tests/support/compose_stream_fixture.mjs';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { performance } from 'node:perf_hooks';

const root = new URL('../../', import.meta.url);

/** Extracts the executing production SDK script without building Rust. */
function sdkScript(file) {
  const source = readFileSync(new URL('core/crates/plugin/sdk/src/toolpkg/' + file, root), 'utf8');
  const start = source.indexOf('r#"');
  const end = source.lastIndexOf('"#');
  assert.ok(start >= 0 && end > start);
  return source.slice(start + 3, end);
}

/** Creates a production DSL runtime with JSON tree transport disabled by a throwing sentinel. */
function mount(count = 100, screenSource = null) {
  const events = [];
  const context = vm.createContext({ console, module: { exports: {} },
    JSON: {
      /** Rejects JSON encoding in the production render and action path under test. */
      stringify() { throw new Error('UI tree JSON encoding is forbidden'); },
      /** Rejects JSON decoding in the production render and action path under test. */
      parse() { throw new Error('UI tree JSON decoding is forbidden'); },
    },
    __operit_call_runtime_ref: {
      /** Captures structured intermediate values as the host callback would receive them. */
      sendComposeResponse(phase, value) { events.push(value); },
    },
  });
  context.exports = context.module.exports;
  vm.runInContext(readFileSync(new URL('core/crates/plugin/sdk/src/toolpkg/vendor/acorn.js', root), 'utf8'), context);
  context.__operitAcorn = context.module.exports;
  context.module = { exports: {} };
  vm.runInContext(readFileSync(new URL('core/crates/plugin/sdk/src/toolpkg/ToolPkgComposeDslCompiler.js', root), 'utf8'), context);
  vm.runInContext(readFileSync(new URL('core/crates/plugin/sdk/src/toolpkg/ToolPkgComposeDslRetained.js', root), 'utf8'), context);
  vm.runInContext(readFileSync(new URL('core/crates/plugin/sdk/src/toolpkg/ToolPkgComposeDslReactive.js', root), 'utf8'), context);
  vm.runInContext(sdkScript('ToolPkgComposeDslBridge.rs'), context);
  // Instrument the real UI construction entry rather than counting transmitted patches.
  vm.runInContext(`
    globalThis.__generation = { entries: 0, nodes: 0 };
    const originalCreateContext = OperitComposeDslRuntime.createContext;
    /** Counts node factories while retaining ordinary plugin argument evaluation. */
    OperitComposeDslRuntime.createContext = function(options) {
      const bundle = originalCreateContext(options);
      const ui = bundle.ctx.UI;
      bundle.ctx.UI = new Proxy(ui, {
        /** Returns an instrumented production node constructor. */
        get(target, type) {
          const construct = target[type];
          if (typeof construct !== 'function') return construct;
          /** Counts every generated node, including discarded identical nodes. */
          return function(...args) {
            __generation.nodes++;
            return construct(...args);
          };
        },
      });
      return bundle;
    };
  `, context);
  const screen = screenSource ?? `module.exports.default = function(ctx) {
    __generation.entries++;
    const [text, setText] = ctx.useState('text', '');
    const [reverse, setReverse] = ctx.useState('reverse', false);
    const [visible, setVisible] = ctx.useState('visible', true);
    const rows = Array.from({length: ${count}}, (_, index) => index);
    if (reverse) rows.reverse();
    return ctx.UI.Column({
      edit: function(next) { setText(next); return next; },
      reorder: function() { setReverse(!reverse); return reverse; },
      hide: function() { setVisible(false); return null; },
      typed: function(value) { return value; },
      asyncEdit: async function(next) { setText(next); await Promise.resolve(); return next; },
      header: ctx.UI.Text({ text: 'Header ' + text }, []),
    }, [
      ctx.UI.Text({key: 'value', text: text}, []),
      ...rows.map(index => ctx.UI.Button({key: 'row:' + index, onClick: function() { return text + ':' + index; }}, [ctx.UI.Text({text: 'Row ' + index}, [])])),
      ...(visible ? [ctx.UI.Button({key: 'removable', onClick: function() { return 'removed'; }}, [])] : []),
    ]);
  };`;
  const forbiddenJSON = context.JSON;
  context.JSON = JSON;
  const compiledScreen = context.OperitComposeCompiler.compile(screen, 'test-screen');
  context.JSON = forbiddenJSON;
  vm.runInContext(sdkScript('ToolPkgComposeDslRuntimeScript.rs').replaceAll('{{', '{').replaceAll('}}', '}').replace('{script}', compiledScreen), context);
  const nodes = new Map();
  let revision = 0;
  let rootId;

  /** Applies flat production updates and validates every descendant reference. */
  function apply(response) {
    assert.equal(Object.hasOwn(response, 'tree'), false);
    const update = response.update;
    if (!update) return;
    if (update.reset) { nodes.clear(); revision = 0; }
    assert.equal(update.revision, revision + 1);
    for (const node of update.upserts) nodes.set(node.id, node);
    for (const id of update.removed) assert.equal(nodes.delete(id), true);
    for (const node of nodes.values()) {
      for (const id of [...node.children, ...Object.values(node.slots).flat()]) assert.ok(nodes.has(id), id);
    }
    rootId = update.rootId;
    assert.ok(nodes.has(rootId));
    revision = update.revision;
  }

  composeStreamFixture(context, { projectTree: false, inspectStorage: false, onResponse(phase, value) { if (phase === "intermediate") events.push(value); } }).adapt();
  const initial = context.__operit_render_compose_dsl({});
  apply(initial);

  /** Dispatches the existing callback API and consumes intermediate updates before final completion. */
  async function action(name, payload) {
    const id = nodes.get(rootId).props[name].__actionId;
    const firstEvent = events.length;
    const final = await context.__operit_dispatch_compose_dsl_action({ __action_id: id, __action_payload: payload });
    for (const event of events.slice(firstEvent)) apply(event);
    apply(final);
    return { final, events: events.slice(firstEvent) };
  }
  return { context, nodes, initial, action, root: () => nodes.get(rootId) };
}

const runtime = mount();
const initialCallbacks = Object.keys(runtime.context.__operit_compose_bundle.actionStore).length;
const edit = await runtime.action('edit', 'first');
assert.equal(edit.final.actionResult, 'first');
assert.ok(edit.events.length > 0);
const changed = edit.events.flatMap(event => event.update?.upserts || []).concat(edit.final.update.upserts);
assert.equal(changed.filter(node => node.nodeType === 'Text').length, 2);
assert.equal(changed.filter(node => node.nodeType === 'Button').length, 0);
assert.equal(edit.final.update.upserts.length, 0);
assert.equal(Object.keys(runtime.context.__operit_compose_bundle.actionStore).length, initialCallbacks);
const row = [...runtime.nodes.values()].find(node => node.props.key === 'row:0');
assert.equal(runtime.context.__operit_compose_bundle.invokeAction(row.props.onClick.__actionId), 'first:0');
const reordered = await runtime.action('reorder');
assert.equal(reordered.final.update.removed.length, 0);
assert.ok(runtime.nodes.has(row.id));
const removable = [...runtime.nodes.values()].find(node => node.props.key === 'removable');
const removedAction = removable.props.onClick.__actionId;
await runtime.action('hide');
assert.equal(runtime.nodes.has(removable.id), false);
assert.throws(() => runtime.context.__operit_compose_bundle.invokeAction(removedAction), /compose action not found/);
for (const value of ['{"success":true}', '[1,2]', '  ', '', null, false, 0, { value: 'object' }, [1, 'two']]) {
  const result = await runtime.action('typed', value);
  assert.deepEqual(result.final.actionResult, value);
}
await runtime.action('asyncEdit', 'async');
for (let index = 0; index < 100; index += 1) await runtime.action('edit', 'value:' + index);
assert.equal(Object.keys(runtime.context.__operit_compose_bundle.actionStore).length, initialCallbacks - 1);

const samples = [];
for (const count of [20, 100, 500]) {
  const mounted = mount(count);
  const durations = [];
  let records = 0;
  const beforeCommit = mounted.context.__operit_compose_bundle.composition.metrics();
  const beforeNodes = mounted.context.__generation.nodes;
  const beforeEntries = mounted.context.__generation.entries;
  for (let index = 0; index < 30; index += 1) {
    const start = performance.now();
    const response = await mounted.action('edit', 'iteration:' + index);
    durations.push(performance.now() - start);
    records += response.final.update.upserts.length + response.events.reduce((sum, event) => sum + event.update.upserts.length, 0);
  }
  durations.sort((a, b) => a - b);
  const afterCommit = mounted.context.__operit_compose_bundle.composition.metrics();
  samples.push({ rows: count,
    commitVisitedPerEdit: (afterCommit.visitedNodes - beforeCommit.visitedNodes) / 30,
    commitChildChecksPerEdit: (afterCommit.checkedChildReferences - beforeCommit.checkedChildReferences) / 30,
    generatedNodesPerEdit: (mounted.context.__generation.nodes - beforeNodes) / 30,
    rootExecutionsPerEdit: (mounted.context.__generation.entries - beforeEntries) / 30, initialNodes: mounted.initial.update.upserts.length, changedRecordsPerEdit: records / 30,
    retainedCallbacks: Object.keys(mounted.context.__operit_compose_bundle.actionStore).length,
    nodeV8ActionP50Ms: Number(durations[14].toFixed(3)), nodeV8ActionP95Ms: Number(durations[28].toFixed(3)) });
}
console.log('JS-only checks passed (not host/CoreLink/Flutter verification): no JSON tree codec, retained nodes, keyed reorder, slots, updated closures, callback release, typed results, synchronous and asynchronous edits.');
console.table(samples);

// This opt-in acceptance gate must stay red until generation, not merely delivery, is local.
if (process.argv.includes('--require-local-generation')) {
  const small = samples[0];
  const large = samples.at(-1);
  assert.ok(large.generatedNodesPerEdit <= small.generatedNodesPerEdit + 8,
    `Local generation is not implemented: ${small.generatedNodesPerEdit} nodes/edit at ${small.rows} rows, ` +
    `${large.generatedNodesPerEdit} nodes/edit at ${large.rows} rows; transmitting two records does not satisfy this gate.`);
}

// Verify dirty retained commits independently of eager root execution and the compiler's scope scan.
const dirtySamples = [];
for (const count of [20, 100, 500]) {
  const mounted = mount(0);
  const bundle = mounted.context.OperitComposeDslRuntime.createContext({});
  const ctx = bundle.ctx;
  const leaves = Array.from({ length: count }, (_, index) => ctx.UI.Text({ key: 'leaf:' + index, text: 'row:' + index }, []));
  const list = ctx.UI.Column({}, leaves);
  const tree = ctx.UI.Column({}, [ctx.UI.Column({}, [list])]);
  bundle.composition.commit(tree);
  const before = bundle.composition.metrics();
  leaves[0].props.text = 'changed';
  const update = bundle.composition.commit(tree);
  const after = bundle.composition.metrics();
  assert.equal(update.upserts.length, 1);
  assert.equal(update.upserts[0].props.text, 'changed');
  assert.equal(after.visitedNodes - before.visitedNodes, 1);
  assert.equal(after.checkedChildReferences - before.checkedChildReferences, 0);
  const unchanged = bundle.composition.commit(tree);
  const afterUnchanged = bundle.composition.metrics();
  assert.equal(unchanged.upserts.length, 0);
  assert.equal(afterUnchanged.visitedNodes, after.visitedNodes);
  assert.equal(afterUnchanged.checkedChildReferences, after.checkedChildReferences);
  dirtySamples.push({ rows: count, dirtyCommitVisits: after.visitedNodes - before.visitedNodes,
    dirtyChildChecks: after.checkedChildReferences - before.checkedChildReferences,
    unchangedCommitVisits: afterUnchanged.visitedNodes - after.visitedNodes });
}
console.log('Passed: a dirty leaf beneath retained ancestors visits exactly one node and no siblings; unchanged commits visit zero nodes.');
console.table(dirtySamples);

// These counters describe the real compiled screen, not only a hand-built dirty-node fixture.
for (const sample of samples) {
  assert.equal(sample.commitVisitedPerEdit, 2);
  assert.equal(sample.commitChildChecksPerEdit, 0);
}

/** Creates a retained graph fixture and validates every downstream reference after each commit. */
function retainedFixture() {
  const mounted = mount(0);
  const bundle = mounted.context.OperitComposeDslRuntime.createContext({});
  const records = new Map();
  /** Applies one real production commit and checks graph consistency. */
  function commit(tree) {
    const update = bundle.composition.commit(tree);
    for (const record of update.upserts) records.set(record.id, record);
    for (const id of update.removed) assert.equal(records.delete(id), true, id);
    for (const record of records.values()) {
      for (const id of [...record.children, ...Object.values(record.slots).flat()]) assert.ok(records.has(id), id);
    }
    assert.ok(records.has(update.rootId));
    return update;
  }
  return { bundle, ctx: bundle.ctx, records, commit };
}

const fixture = retainedFixture();
const { ctx, bundle } = fixture;
const shared = ctx.UI.Button({ key: 'shared', onClick: () => 'live' }, [ctx.UI.Text({ text: 'original' }, [])]);
const branch = ctx.UI.Column({}, [shared]);
const rootTree = ctx.UI.Column({ header: shared }, [branch]);
fixture.commit(rootTree);
assert.equal(Object.keys(bundle.actionStore).filter(id => id.startsWith('__action_')).length, 2);
shared.props.enabled = false;
let update = fixture.commit(rootTree);
assert.equal(update.upserts.length, 2, 'Both occurrences of an aliased node must update');
shared.children[0].props.text = 'nested';
update = fixture.commit(rootTree);
assert.equal(update.upserts.length, 2, 'Dirty descendants must update beneath both retained occurrences');
assert.ok(update.upserts.every(record => record.props.text === 'nested'));

shared.props.key = 'renamed';
update = fixture.commit(rootTree);
assert.equal(update.removed.length, 4, 'Both old keyed branches must be released');
assert.equal(Object.keys(bundle.actionStore).filter(id => id.startsWith('__action_')).length, 2);
shared.type = 'Surface';
update = fixture.commit(rootTree);
assert.equal(update.removed.length, 4, 'A type change replaces each occurrence identity');
assert.equal(Object.keys(bundle.actionStore).filter(id => id.startsWith('__action_')).length, 2);

const beforeRemoval = bundle.composition.metrics();
branch.children.splice(0, 1);
update = fixture.commit(rootTree);
assert.equal(update.removed.length, 2);
assert.equal(bundle.composition.metrics().removedNodes - beforeRemoval.removedNodes, 2);
assert.equal(Object.keys(bundle.actionStore).filter(id => id.startsWith('__action_')).length, 1, 'Removing one alias must preserve the other occurrence callback');
shared.props.enabled = true;
update = fixture.commit(rootTree);
assert.equal(update.upserts.length, 1, 'Detached occurrences must no longer enqueue dirty work');
delete rootTree.slots.header;
fixture.commit(rootTree);
assert.equal(Object.keys(bundle.actionStore).filter(id => id.startsWith('__action_')).length, 0);
shared.props.enabled = false;
update = fixture.commit(rootTree);
assert.equal(update.upserts.length, 0, 'Fully detached nodes must not enqueue dirty work');

const replacement = ctx.UI.Text({ text: 'replacement' }, []);
update = fixture.commit(replacement);
assert.equal(fixture.records.size, 1);
assert.equal(update.removed.length, 2);
replacement.props.style = { nested: { weight: 100 } };
fixture.commit(replacement);
replacement.props.style.nested.weight = 200;
update = fixture.commit(replacement);
assert.equal(update.upserts[0].props.style.nested.weight, 200);
Object.defineProperty(replacement.props, 'text', { value: 'defined', configurable: true, enumerable: true, writable: true });
assert.equal(fixture.commit(replacement).upserts[0].props.text, 'defined');
delete replacement.props.style;
assert.equal(Object.hasOwn(fixture.commit(replacement).upserts[0].props, 'style'), false);

// Newly allocated but unattached actions must be released without enumerating all live callbacks.
ctx.UI.Button({ onClick: () => 'unused' }, []);
fixture.commit(replacement);
assert.equal(Object.keys(bundle.actionStore).filter(id => id.startsWith('__action_')).length, 0);

const invalid = retainedFixture();
const duplicate = invalid.ctx.UI.Column({}, [invalid.ctx.UI.Text({ key: 'x' }, []), invalid.ctx.UI.Text({ key: 'x' }, [])]);
assert.throws(() => invalid.commit(duplicate), /duplicate compose node key/);
console.log('Passed: dirty compiled-screen gate, nested mutation, shared occurrences, key/type replacement, branch disposal, slot removal, callback ownership, root replacement, descriptors, and duplicate-key rejection.');

const remount = retainedFixture();
const cachedButton = remount.ctx.UI.Button({ onClick: () => 'cached' }, []);
const cachedRoot = remount.ctx.UI.Column({}, [cachedButton]);
remount.commit(cachedRoot);
const cachedAction = cachedButton.props.onClick.__actionId;
cachedRoot.children = [];
remount.commit(cachedRoot);
assert.throws(() => remount.bundle.invokeAction(cachedAction), /compose action not found/);
cachedRoot.children = [cachedButton];
remount.commit(cachedRoot);
assert.equal(remount.bundle.invokeAction(cachedButton.props.onClick.__actionId), 'cached');
console.log('Passed: cached nodes remount with fresh action ownership after detached action IDs are released.');

const inPlace = retainedFixture();
const inPlaceText = inPlace.ctx.UI.Text({ text: 'before' }, []);
inPlace.commit(inPlaceText);
inPlaceText.type = 'Surface';
update = inPlace.commit(inPlaceText);
assert.equal(update.upserts[0].nodeType, 'Surface');
assert.equal(update.removed.length, 1);

const reactiveScreen = `module.exports.default = function(ctx) {
  const [text, setText] = ctx.useState('text', '');
  const [rows, setRows] = ctx.useState('rows', [0, 1, 2]);
  const [visible, setVisible] = ctx.useState('visible', true);
  const label = 'Value ' + text;
  return ctx.UI.Column({
    edit: next => { setText(next); return next; },
    reorder: () => setRows([2, 1, 0]),
    hide: () => setVisible(false),
    typed: () => label,
    asyncEdit: async next => { setText(next); await Promise.resolve(); setText(next + '!'); return next; },
  }, [
    ctx.UI.Text({ key: 'value', text: label }, []),
    ...rows.map(index => ctx.UI.Button({ key: 'row:' + index, onClick: () => text + ':' + index }, [ctx.UI.Text({text: 'Row ' + index}, [])])),
    ...(visible ? [ctx.UI.Text({key: 'conditional', text: 'Conditional ' + text}, [])] : []),
  ]);
};`;
const reactive = mount(0, reactiveScreen);
const reactiveContext = reactive.context.__operit_compose_bundle.ctx;
const beforeReactive = reactive.context.OperitComposeReactive.metrics(reactiveContext);
await reactive.action('edit', 'changed');
const afterReactive = reactive.context.OperitComposeReactive.metrics(reactiveContext);
assert.equal(afterReactive.rootRuns, beforeReactive.rootRuns, 'A text write must not execute the screen entry');
assert.equal(afterReactive.expressionRuns - beforeReactive.expressionRuns, 2, 'Only two text expressions depend on text');
const reactiveRow = [...reactive.nodes.values()].find(node => node.props.key === 'row:0');
assert.equal(reactive.context.__operit_compose_bundle.invokeAction(reactiveRow.props.onClick.__actionId), 'changed:0', 'Skipped rows must capture refreshed lexical state');
assert.equal((await reactive.action('typed')).final.actionResult, 'Value changed', 'Derived values must refresh inside event closures');
await reactive.action('reorder');
assert.ok(reactive.nodes.has(reactiveRow.id));
await reactive.action('hide');
await reactive.action('edit', 'hidden');
assert.equal([...reactive.nodes.values()].some(node => node.props.key === 'conditional'), false);
await reactive.action('asyncEdit', 'async');
assert.equal((await reactive.action('typed')).final.actionResult, 'Value async!');
assert.equal(reactive.context.OperitComposeReactive.metrics(reactiveContext).rootRuns, 1);
console.log('Passed: state-to-expression recomposition, no root rerun, refreshed captured state/derived values, structural reorder, conditional subscription disposal and asynchronous writes.');

const reactiveSamples = [];
for (const count of [20, 100, 500]) {
  const source = reactiveScreen.replace("ctx.useState('rows', [0, 1, 2])", "ctx.useState('rows', [" + Array.from({length: count}, (_, i) => i).join(',') + "])");
  const mounted = mount(0, source);
  const ctx = mounted.context.__operit_compose_bundle.ctx;
  const initial = mounted.context.OperitComposeReactive.metrics(ctx);
  const before = mounted.context.__operit_compose_bundle.composition.metrics();
  const durations = [];
  for (let index = 0; index < 30; index += 1) {
    const started = performance.now();
    await mounted.action('edit', 'value:' + index);
    durations.push(performance.now() - started);
  }
  const current = mounted.context.OperitComposeReactive.metrics(ctx);
  const after = mounted.context.__operit_compose_bundle.composition.metrics();
  assert.equal(current.rootRuns - initial.rootRuns, 0);
  assert.equal(current.expressionRuns - initial.expressionRuns, 60);
  assert.equal(after.checkedChildReferences - before.checkedChildReferences, 0);
  durations.sort((a, b) => a - b);
  reactiveSamples.push({rows: count, rootRunsPerEdit: 0, expressionRunsPerEdit: 2,
    childChecksPerEdit: 0, nodeV8ActionP50Ms: Number(durations[14].toFixed(3))});
}
console.log('Reactive compiled-screen execution gate:');
console.table(reactiveSamples);

// Hidden callback properties retain one slot per mounted occurrence, not one closure per render.
const hiddenCallbacks = retainedFixture();
let callbackPhase = 'ready';
/** Enforces the plugin's real state guard even when its UI callback is no longer exposed. */
function guardedClick() {
  if (callbackPhase !== 'ready') throw new Error('controller ' + callbackPhase);
  return 'ready';
}
let hiddenButton = hiddenCallbacks.ctx.UI.Button({ key: 'owned', onClick: guardedClick }, []);
const hiddenRoot = hiddenCallbacks.ctx.UI.Column({}, [hiddenButton]);
hiddenCallbacks.commit(hiddenRoot);
const ownedAction = hiddenButton.props.onClick.__actionId;
const hiddenBaseline = Object.keys(hiddenCallbacks.bundle.actionStore).length;
for (let iteration = 0; iteration < 100; iteration += 1) {
  callbackPhase = 'busy';
  hiddenRoot.children = [hiddenCallbacks.ctx.UI.Button({key: 'owned', enabled: false}, [])];
  hiddenCallbacks.commit(hiddenRoot);
  const disabledRecord = [...hiddenCallbacks.records.values()].find(record => record.props.key === 'owned');
  assert.equal(Object.hasOwn(disabledRecord.props, 'onClick'), false, 'Ownership must not expose a hidden callback to Flutter');
  assert.throws(() => hiddenCallbacks.bundle.invokeAction(ownedAction), /controller busy/);
  assert.equal(Object.keys(hiddenCallbacks.bundle.actionStore).length, hiddenBaseline);
  callbackPhase = 'ready';
  hiddenButton = hiddenCallbacks.ctx.UI.Button({key: 'owned', onClick: () => iteration}, []);
  hiddenRoot.children = [hiddenButton];
  hiddenCallbacks.commit(hiddenRoot);
  assert.equal(hiddenButton.props.onClick.__actionId, ownedAction);
  assert.equal(hiddenCallbacks.bundle.invokeAction(ownedAction), iteration, 'Reenabled callback must use its new captured closure');
  assert.equal(Object.keys(hiddenCallbacks.bundle.actionStore).length, hiddenBaseline);
  // Restore a guarded closure in the same slot for the next disabled-state check.
  hiddenRoot.children = [hiddenCallbacks.ctx.UI.Button({key: 'owned', onClick: guardedClick}, [])];
  hiddenCallbacks.commit(hiddenRoot);
}
callbackPhase = 'finished';
hiddenRoot.children = [hiddenCallbacks.ctx.UI.Button({key: 'owned'}, [])];
hiddenCallbacks.commit(hiddenRoot);
assert.throws(() => hiddenCallbacks.bundle.invokeAction(ownedAction), /controller finished/);
hiddenRoot.children = [];
hiddenCallbacks.commit(hiddenRoot);
assert.throws(() => hiddenCallbacks.bundle.invokeAction(ownedAction), /compose action not found/);
assert.equal(Object.keys(hiddenCallbacks.bundle.actionStore).length, hiddenBaseline - 1);
console.log('Passed: hidden callbacks preserve controller guards; 100 disable/enable cycles retain one action ID, refresh captures, and release ownership on unmount.');
