import assert from 'node:assert/strict';

/** Projects streamed flat records only for existing tree-oriented test assertions. */
export function composeStreamFixture(context, { projectTree = true, inspectStorage = true, onResponse = () => {} } = {}) {
  const nodes = new Map();
  const events = [];
  const errors = [];
  let rootId;
  let revision = 0;
  let latest;
  /** Reconstructs a tree in the assertion harness, never in the production transport. */
  function tree(id) {
    const node = nodes.get(id);
    assert.ok(node, `Unknown retained test node: ${id}`);
    return { type: node.nodeType, props: node.props, children: node.children.map(tree),
      slots: Object.fromEntries(Object.entries(node.slots).map(([name, ids]) => [name, ids.map(tree)])) };
  }
  /** Accepts the sole production response shape and enforces ordered commits. */
  function accept(phase, response) {
    assert.equal(Object.hasOwn(response, 'tree'), false);
    assert.equal(Object.hasOwn(response, 'state'), false);
    assert.equal(Object.hasOwn(response, 'memo'), false);
    if (response.update) {
      const update = response.update;
      if (!update.reset) assert.equal(update.revision, revision + 1);
      if (update.reset) nodes.clear();
      revision = update.revision;
      for (const node of update.upserts) nodes.set(node.id, node);
      for (const id of update.removed) nodes.delete(id);
      rootId = update.rootId;
    }
    const projected = { ...response };
    if (projectTree && response.update) projected.tree = tree(rootId);
    if (inspectStorage) {
      projected.state = context.__operit_compose_bundle.stateStore;
      projected.memo = context.__operit_compose_bundle.memoStore;
    }
    events.push({ phase, response });
    onResponse(phase, projected);
    if (phase === 'final') latest = projected;
    return projected;
  }
  /** Supplies an actual call-owned sink while retaining fixture-specific host environment methods. */
  function sink() {
    return { ...context.__operit_call_runtime_ref, sendComposeResponse: accept,
      /** Records detached errors independently of an already settled command. */
      reportError(error) { errors.push(error); },
    };
  }
  /** Makes existing assertion callers consume the final stream response rather than a JS return snapshot. */
  function adapt() {
    for (const name of ['__operit_render_compose_dsl', '__operit_dispatch_compose_dsl_action']) {
      const operation = context[name];
      assert.equal(typeof operation, 'function');
      /** Awaits only command completion and reads the captured final response for assertions. */
      function invoke(...args) {
        let final;
        context.__operit_call_runtime_ref = { ...sink(),
          /** Retains command-local completion ownership for overlapping asynchronous fixture calls. */
          sendComposeResponse(phase, response) {
            const projected = accept(phase, response);
            if (phase === 'final') final = projected;
          },
        };
        const result = operation(...args);
        /** Rejects a second response route at the actual production completion boundary. */
        function completed(value) { assert.equal(value, null); assert.ok(final); return final; }
        return result && typeof result.then === 'function' ? result.then(completed) : completed(result);
      }
      context[name] = invoke;
      context.module.exports[name] = invoke;
    }
  }
  context.__operit_call_runtime_ref = sink();
  return { accept, adapt, nodes, events, errors, get latest() { return latest; }, get root() { return nodes.get(rootId); } };
}
