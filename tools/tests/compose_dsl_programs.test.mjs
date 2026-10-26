import assert from 'node:assert/strict';
import test from 'node:test';
import vm from 'node:vm';
import { readComposeBuffer } from './support/compose_buffer_fixture.mjs';
import { composeContextScript } from '../runtime/compose_scripts.mjs';

/** Runs the production program writer and callback registry without projecting a host tree. */
function runtime() {
  const context = vm.createContext({console});
  vm.runInContext(composeContextScript(), context);
  const runtime = context.OperitComposeDslRuntime.createContext({});
  const finish = runtime.composition.finish;
  /** Inspects the actual binary commit for assertion readability. */
  runtime.composition.finish = function (...args) { const frame = finish(...args); return frame === null ? null : readComposeBuffer(frame.seal('final', undefined, [])).response.update; };
  return {context, ...runtime};
}
/** Defines a repeated multi-instruction row with a single parameter vector. */
function rowProgram(count = 26) {
  return {id: 'rows:' + count, root: 0, nodes: Array.from({length:count}, (_, i) => ({
    type:i === 0 ? 'Row' : 'Text', props:i === 0 ? [2, ['key','onClick'], [[1,0],[1,1]]] : [2,['text'],[[1,2]]],
    children:i === 0 ? [3,Array.from({length:count-1},(_,n)=>[4,n+1])] : [0,null], explicitChildren:true
  }))};
}

test('128 repeated 26-instruction rows declare one program and never construct internal JS nodes', () => {
  const r = runtime(), program = rowProgram();
  const rows = Array.from({length:128}, (_, i) => r.composition.instance({}, program, [String(i), ()=>i, 'title']));
  const root = r.ctx.UI.Column({}, rows);
  const before = r.composition.metrics();
  const update = r.composition.finish(root);
  assert.equal(update.programs.length, 2);
  assert.equal(update.instances.length,129);
  assert.equal(update.programs.find(p=>p.id===program.id).nodes.length,26);
  assert.equal(before.projections,0);
  assert.equal(r.composition.metrics().projections,0);
  assert.equal(r.composition.metrics().normalizedValues,before.normalizedValues);
});

test('programs are installed once and unchanged closures refresh without parameter transport', () => {
  const r = runtime(), slot = {}, program = rowProgram(2);
  let root = r.composition.instance(slot,program,['row',()=>1,'title']);
  const first = r.composition.finish(root);
  const action = root.props.onClick.__actionId;
  root = r.composition.instance(slot,program,['row',()=>2,'title']);
  const second = r.composition.finish(root);
  assert.equal(second.programs.length,0);
  assert.equal(second.instances.length,0);
  assert.equal(second.revision,first.revision+1);
  assert.equal(r.actions.invoke(action),2);
});

test('shared sources have one event slot and expire only after the last alias detaches', () => {
  const r = runtime();
  const child = r.ctx.UI.Button({onClick:()=>7});
  const root = r.ctx.UI.Column({},Array(128).fill(child));
  assert.equal(r.composition.finish(root).instances.length,2);
  const action = child.props.onClick.__actionId;
  root.children = [child];
  assert.equal(r.composition.finish(root).removed.length,0);
  assert.equal(r.actions.invoke(action),7);
  root.children = [];
  assert.equal(r.composition.finish(root).removed.length,1);
  assert.throws(()=>r.actions.invoke(action),/not found/);
});

test('cached detached sources remount with fresh events and stable logical source identity', () => {
  const r = runtime();
  const child = r.ctx.UI.Button({onClick:()=>7});
  const old = child.props.onClick.__actionId;
  const root = r.ctx.UI.Column({});
  const initial = r.composition.finish(root);
  assert.equal(initial.instances.length,1);
  assert.throws(()=>r.actions.invoke(old),/not found/);
  root.children = [child];
  const mounted = r.composition.finish(root);
  const childId = mounted.instances.find(i=>i.id!==mounted.root.instanceId).id;
  const event = child.props.onClick.__actionId;
  assert.notEqual(event,old);
  assert.equal(r.actions.invoke(event),7);
  root.children = [];
  assert.deepEqual(Array.from(r.composition.finish(root).removed),[childId]);
  root.children = [child];
  const restored = r.composition.finish(root);
  assert.ok(restored.instances.some(i=>i.id===childId));
  assert.notEqual(child.props.onClick.__actionId,event);
  assert.equal(r.actions.invoke(child.props.onClick.__actionId),7);
});

test('explicit local props and children edits emit patches and preserve compiler source addresses', () => {
  const r = runtime(), root = r.composition.instance({},rowProgram(2),['row',()=>1,'title']);
  const initial = r.composition.finish(root);
  const child = root.children[0];
  child.props.text = 'edited';
  const changed = r.composition.finish(root);
  assert.equal(changed.instances.length,1);
  assert.equal(changed.instances[0].id,initial.root.instanceId);
  assert.equal(changed.instances[0].patches[0].node,1);
  assert.equal(changed.instances[0].patches[0].value.text,'edited');
  assert.equal(root.children[0],child);
});

test('discarded dirty sources never remove an instance the host has not received', () => {
  const r = runtime();
  const branch = r.ctx.UI.Box({},[r.ctx.UI.Text({text:'old'})]);
  const root = r.ctx.UI.Column({},[branch]);
  const first = r.composition.finish(root);
  const known = new Set(first.instances.map(i=>i.id));
  branch.children = [r.ctx.UI.Text({text:'discarded'})];
  root.children = [];
  const update = r.composition.finish(root);
  assert.ok(update.removed.every(id=>known.has(id)));
  assert.equal(update.instances.length,1);
  assert.equal(r.composition.finish(root).removed.length,0);
});

test('cross-source cycles reject without poisoning traversal state', () => {
  const r = runtime(), child = r.ctx.UI.Box({}), root = r.ctx.UI.Column({},[child]);
  r.composition.finish(root);
  child.children = [root];
  assert.throws(()=>r.composition.finish(root),/must not form a cycle/);
  child.children = [];
  assert.equal(r.composition.finish(root).revision,2);
});

test('local instruction cycles are rejected at the explicit mutation boundary', () => {
  const r = runtime(), root = r.composition.instance({},rowProgram(2),['row',()=>1,'title']);
  r.composition.finish(root);
  assert.throws(()=>{ root.children[0].children = [root]; },/must not form a cycle/);
});

test('boundary snapshots isolate constructor arguments, preserve aliases, and reject cycles', () => {
  const r = runtime(), model = {value:1};
  const root = r.ctx.UI.Text({a:model,b:model});
  model.value = 9;
  r.composition.finish(root);
  assert.equal(root.props.a.value,1);
  assert.equal(root.props.b.value,1);
  root.props.a.value = 3;
  assert.equal(root.props.b.value,1);
  const cyclic = {}; cyclic.self = cyclic;
  assert.throws(()=>r.ctx.UI.Text({cyclic}),/must not contain cycles/);
  const array=[]; array.push(array);
  assert.throws(()=>r.ctx.UI.Text({array}),/must not contain cycles/);
});

test('nested edits, descriptor writes, deletions and slots use the same instance transaction', () => {
  const r = runtime(), child = r.ctx.UI.Text({text:'slot'});
  const root = r.ctx.UI.Box({nested:{value:1}},[]);
  r.composition.finish(root);
  root.props.nested.value = 2;
  Object.defineProperty(root.props,'added',{value:3,enumerable:true,configurable:true});
  delete root.props.nested;
  root.slots = {title:[child]};
  root.type = 'Column';
  const update = r.composition.finish(root);
  assert.equal(update.instances.length,2);
  assert.equal(root.props.added,3);
  assert.equal(root.props.nested,undefined);
  assert.equal(root.slots.title[0],child);
  assert.equal(root.type,'Column');
  root.slots = {};
  assert.equal(r.composition.finish(root).removed.length,1);
});

test('foreign nodes and callback references reject even when action IDs collide', () => {
  const a = runtime(), b = a.context.OperitComposeDslRuntime.createContext({});
  const first = a.ctx.UI.Button({onClick:()=>1});
  b.ctx.UI.Button({onClick:()=>2});
  assert.throws(()=>b.ctx.UI.Column({},[first]),/another context/);
  assert.throws(()=>b.ctx.UI.Button({onClick:first.props.onClick}),/another context/);
  assert.throws(()=>a.context.OperitComposeComposition.evaluate({...a.ctx},'site',rowProgram(),[]),/original SDK context/);
});

test('unchanged intermediates do not advance revision and final fences remain contiguous', () => {
  const r = runtime(), root = r.ctx.UI.Text({text:'same'});
  assert.equal(r.composition.finish(root,true).revision,1);
  assert.equal(r.composition.finish(root,true),null);
  assert.equal(r.composition.finish(root).revision,2);
});

test('modifier methods retain captured receivers and independent operation arrays', () => {
  const r=runtime(), base=r.ctx.Modifier.padding(2), append=base.width;
  const a=append(10), b=base.height(20);
  assert.equal(a.__modifierOps.length,2);
  assert.equal(b.__modifierOps.length,2);
  assert.equal(base.__modifierOps.length,1);
  assert.equal(a.__modifierOps[1].name,'width');
  assert.equal(b.__modifierOps[1].name,'height');
});


test('rejected local edits leave the last valid snapshot publishable', () => {
  const r = runtime(), root = r.composition.instance({}, rowProgram(2), ['row', () => 1, 'title']);
  r.composition.finish(root);
  assert.throws(() => { root.children[0].children = [root]; }, /must not form a cycle/);
  assert.equal(root.children[0].children.length, 0);
  assert.equal(r.composition.finish(root).instances.length, 0);
});

test('patched transport parameters contain no references to removed sources', () => {
  const r = runtime(), child = r.ctx.UI.Text({text:'child'}), root = r.ctx.UI.Column({}, [child]);
  const first = r.composition.finish(root), childId = first.instances.find(i => i.id !== first.root.instanceId).id;
  root.children = [];
  const removed = r.composition.finish(root);
  assert.ok(removed.removed.includes(childId));
  assert.ok(!JSON.stringify(removed.instances).includes('"' + childId + '"'));
  root.children = [child];
  const restored = r.composition.finish(root);
  assert.ok(restored.instances.some(i => i.id === childId));
});

test('removed event slots expire even while their source remains live', () => {
  const r = runtime(), root = r.ctx.UI.Button({onClick:() => 7});
  r.composition.finish(root);
  const old = root.props.onClick.__actionId;
  root.props = {text:'plain'};
  r.composition.finish(root);
  assert.throws(() => r.actions.invoke(old), /not found/);
});

test('cached callback views retain ownership without retaining a live address', () => {
  const r = runtime(), child = r.ctx.UI.Button({onClick:() => 9});
  const cached = child.props.onClick;
  const root = r.ctx.UI.Column({}, []);
  r.composition.finish(root);
  assert.throws(() => r.actions.invoke(cached.__actionId), /not found/);
  root.props = {onClick:cached};
  r.composition.finish(root);
  assert.equal(r.actions.invoke(root.props.onClick.__actionId), 9);
});

test('reactivating an edited cached source invalidates its earlier rebound address', () => {
  const r = runtime(), child = r.ctx.UI.Button({onClick:() => 1}), root = r.ctx.UI.Column({}, [child]);
  r.composition.finish(root);
  root.children = [];
  r.composition.finish(root);
  child.props = {onClick:() => 2};
  const temporary = child.props.onClick.__actionId;
  root.children = [child];
  r.composition.finish(root);
  assert.throws(() => r.actions.invoke(temporary), /not found/);
  assert.equal(r.actions.invoke(child.props.onClick.__actionId), 2);
});
