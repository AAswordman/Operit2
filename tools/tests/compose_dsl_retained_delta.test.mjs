import { composeContextScript } from "../runtime/compose_scripts.mjs";
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

/** Uses the actual SDK occurrence graph and mutation observer, not a transport mock. */
function retained() {
  const context = vm.createContext({ console });
  vm.runInContext(composeContextScript(), context);
  const actions = context.OperitComposeActions.create();
  const runtime = { actions };
  const graph = context.OperitComposeRetained.create(runtime);
  const node = (type, key, children = []) => context.OperitComposeGeneration.observeNode({
    __composeNode: true, type, props: { key }, children, slots: {},
  });
  return { graph, node, runtime, context, generation: context.OperitComposeGeneration };
}

test('detached dirty branches never remove transient nodes that were not delivered to the client', () => {
  const { graph, node } = retained();
  const branch = node('Box', 'branch', [node('Text', 'old')]);
  const root = node('Column', 'root', [branch]);
  const initial = graph.commit(root);
  const client = new Map(Array.from(initial.upserts, record => [record.id, record]));

  // Matches a popup closing while an async host update has dirtied its descendants.
  // The branch is still queued when the root detaches it, so reconciliation can
  // create a temporary replacement which never belongs to a delivered commit.
  branch.children = [node('Text', 'temporary')];
  root.children = [];
  const update = graph.commit(root);
  for (const record of update.upserts) client.set(record.id, record);
  for (const id of update.removed) {
    assert.ok(client.has(id), `Delta removes a node the client never received: ${id}`);
    client.delete(id);
  }
  assert.equal(client.size, 1);
  assert.equal(client.get(update.rootId).children.length, 0);
  assert.equal(graph.commit(root).removed.length, 0);
});

/** Checks opaque handles, keyed identity, type changes, and removal through real retained commits. */
test('transport handles stay compact across deep nesting and preserve keyed sibling identity', () => {
  const { graph, node } = retained();
  const first = node('Text', 'first-' + 'x'.repeat(1000));
  const second = node('Text', 'second');
  const branch = node('Column', 'branch', [first, second]);
  let root = branch;
  for (let index = 0; index < 40; index++) root = node('Box', 'depth-' + index, [root]);
  const initial = graph.commit(root);
  assert.ok(initial.upserts.every(record => record.id.length < 12));
  const firstId = initial.upserts.find(record => record.props.key === first.props.key).id;
  const secondId = initial.upserts.find(record => record.props.key === second.props.key).id;
  branch.children = [second, first];
  const reordered = graph.commit(root);
  assert.deepEqual(Array.from(reordered.upserts.find(record => record.props.key === 'branch').children), [secondId, firstId]);
  assert.equal(reordered.removed.length, 0);
  const replacement = node('Button', first.props.key);
  branch.children = [replacement, second];
  const changed = graph.commit(root);
  assert.deepEqual(Array.from(changed.removed), [firstId]);
  const replacementId = changed.upserts.find(record => record.nodeType === 'Button').id;
  assert.notEqual(replacementId, firstId);
  branch.children = [second]; graph.commit(root);
  branch.children = [replacement];
  const reopened = graph.commit(root);
  assert.notEqual(reopened.upserts.find(record => record.nodeType === 'Button').id, replacementId);
  assert.ok(reopened.upserts.every(record => record.id.length < 12));
});


test('cached callbacks survive detachment and property observer replacement while detached action IDs expire', () => {
  const { graph, node, runtime, generation } = retained();
  const handler = () => 'the cached callback';
  const reference = runtime.actions.reference(handler);
  const cached = node('Button', 'cached');
  cached.props.onClick = reference;
  const root = node('Column', 'root', [cached]);
  const mounted = graph.commit(root);
  const oldId = mounted.upserts.find(record => record.nodeType === 'Button').props.onClick.__actionId;
  assert.equal(runtime.actions.store[oldId](), 'the cached callback');
  root.children = [];
  graph.commit(root);
  assert.equal(runtime.actions.store[oldId], undefined);
  // Generation replaces an occurrence's props with another observed container.
  const replacement = node('Button', 'replacement');
  replacement.props = cached.props;
  root.children = [replacement];
  const reopened = graph.commit(root);
  const newId = reopened.upserts.find(record => record.nodeType === 'Button').props.onClick.__actionId;
  assert.notEqual(newId, oldId);
  assert.equal(runtime.actions.store[newId](), 'the cached callback');
  root.children = [];
  graph.commit(root);
  assert.equal(runtime.actions.store[newId], undefined);
  root.children = [cached];
  const restored = graph.commit(root);
  const restoredId = restored.upserts.find(record => record.nodeType === 'Button').props.onClick.__actionId;
  assert.equal(runtime.actions.store[restoredId](), 'the cached callback');
});

test('callbacks of generated but unmounted nodes remain callable after discarded registration cleanup', () => {
  const { graph, node, runtime, generation } = retained();
  const reference = runtime.actions.reference(() => 'first mount');
  const cached = node('Button', 'cached');
  cached.props.onClick = reference;
  const root = node('Column', 'root');
  graph.commit(root);
  assert.equal(runtime.actions.store.__action_1, undefined);
  root.children = [cached];
  const mounted = graph.commit(root);
  const action = mounted.upserts.find(record => record.nodeType === 'Button').props.onClick.__actionId;
  assert.equal(runtime.actions.store[action](), 'first mount');
});


test('callbacks cannot be rebound into another SDK context even when transport IDs collide', () => {
  const { graph, node, runtime, context } = retained();
  const reference = runtime.actions.reference(() => 'first owner');
  const cached = node('Button', 'cached');
  cached.props.onClick = reference;
  const foreignActions = context.OperitComposeActions.create();
  foreignActions.register(() => 'second owner');
  const foreignGraph = context.OperitComposeRetained.create({ actions: foreignActions });
  assert.throws(() => foreignGraph.commit(cached), /another context/);
  assert.equal(runtime.actions.invoke(reference.__actionId), 'first owner');
});

test('generation rejects a copied context at the ownership boundary', () => {
  const context = vm.createContext({ console });
  vm.runInContext(composeContextScript(), context);
  const runtime = context.OperitComposeDslRuntime.createContext({});
  assert.throws(() => context.OperitComposeGeneration.begin({ ...runtime.ctx }), /original SDK context/);
  assert.doesNotThrow(() => context.OperitComposeGeneration.begin(runtime.ctx));
});
