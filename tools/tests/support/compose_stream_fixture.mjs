import assert from 'node:assert/strict';
import { readComposeBuffer } from './compose_buffer_fixture.mjs';

/** Projects streamed flat records only for existing tree-oriented test assertions. */
export function composeStreamFixture(context, { projectTree = true, inspectStorage = true, onResponse = () => {} } = {}) {
  const nodes = new Map();
  const programs = new Map();
  const instances = new Map();
  const events = [];
  const errors = [];
  const cleanups = new Set();
  let rootId;
  let revision = 0;
  let latest;
  const mounts = new Map();
  const actionTargets = new Map();
  let actionSequence = 0;
  let liveMounts;
  /** Projects callback addresses through a stable test mount while production host behavior is tested in Dart. */
  function properties(value, mount, path = '') {
    if (value === null || typeof value !== 'object') return value;
    if (Object.hasOwn(value, '__actionId')) {
      if (!mount.actions.has(path)) mount.actions.set(path, '@fixture:' + ++actionSequence);
      const id = mount.actions.get(path);
      actionTargets.set(id, value.__actionId);
      return { __actionId: id };
    }
    return Array.isArray(value)
      ? value.map((child, index) => properties(child, mount, path + '/' + index))
      : Object.fromEntries(Object.entries(value).map(([key, child]) => [key, properties(child, mount, path + '/' + key.length + ':' + key)]));
  }
  /** Reconstructs host-facing assertion trees without changing the source records under test. */
  function tree(id, path = 'root') {
    const node = nodes.get(id);
    assert.ok(node, `Unknown construction test node: ${id}`);
    const identity = path + '/' + node.nodeType;
    if (!mounts.has(identity)) mounts.set(identity, { actions: new Map() });
    const mount = mounts.get(identity);
    liveMounts.add(identity);
    /** Gives keyed members stable assertion addresses across source-definition replacement. */
    function group(ids, slot) {
      return ids.map((childId, index) => {
        const child = nodes.get(childId);
        assert.ok(child);
        const key = child.props.key;
        return tree(childId, identity + '/' + JSON.stringify([slot, key == null ? 'position' : 'key', key == null ? index : String(key)]));
      });
    }
    return { type: node.nodeType, props: properties(node.props, mount), children: group(node.children, 'children'),
      slots: Object.fromEntries(Object.entries(node.slots).map(([name, ids]) => [name, group(ids, 'slot:' + name)])) };
  }
  /** Resolves a program operand for tree-oriented assertions, never for production execution. */
  function operand(instance, code) {
    switch (code[0]) {
      case 0: return code[1];
      case 1: return instance.values[code[1]];
      case 2: return Object.fromEntries(code[1].map((name, i) => [name, operand(instance, code[2][i])]));
      case 3: return code[1].map(member => operand(instance, member));
      case 4: return { __composeRef: [instance.id, code[1]] };
      case 5: return operand(instance, code[1]).concat(operand(instance, code[2]));
      default: throw Error('Unknown program operand');
    }
  }
  /** Collects encoded logical references from dynamic child containers. */
  function members(value, output = []) {
    if (Array.isArray(value)) value.forEach(child => members(child, output));
    else if (value?.__composeRef) output.push(value.__composeRef.join('#'));
    return output;
  }
  /** Projects the committed program store only when a tree assertion explicitly requests it. */
  function projectNodes() {
    nodes.clear();
    for (const instance of instances.values()) {
      const program = programs.get(instance.programId);
      assert.ok(program, 'Unknown layout program');
      program.nodes.forEach((instruction, index) => {
        const raw = operand(instance, instruction.props);
        const props = {}, slots = {};
        let content = operand(instance, instruction.children);
        for (const [key, value] of Object.entries(raw ?? {})) {
          if (key === 'content' && (!instruction.explicitChildren || content?.__composeUndefined === true)) { content = value; continue; }
          const children = members(value);
          if (children.length) slots[key] = children;
          else props[key] = value;
        }
        const node = { id: instance.id + '#' + index, nodeType: instruction.type, props, slots, children: members(content) };
        for (const patch of instance.patches) {
          if (patch.node !== index) continue;
          if (patch.field === 'type') node.nodeType = patch.value;
          else if (patch.field === 'children') node.children = members(patch.value);
          else if (patch.field === 'slots') node.slots = Object.fromEntries(Object.entries(patch.value).map(([key,value]) => [key,members(value)]));
          else node[patch.field] = patch.value;
        }
        nodes.set(node.id, node);
      });
    }
    return nodes;
  }
  /** Accepts the sole program transaction protocol and enforces ordered revisions. */
  function accept(phase, frame) {
    const decoded = readComposeBuffer(frame);
    assert.equal(decoded.phase, phase);
    const response = decoded.response;
    assert.equal(Object.hasOwn(response, 'tree'), false);
    assert.equal(Object.hasOwn(response, 'state'), false);
    assert.equal(Object.hasOwn(response, 'memo'), false);
    if (response.update) {
      const update = response.update;
      if (!update.reset) assert.equal(update.revision, revision + 1);
      if (update.reset) { programs.clear(); instances.clear(); }
      revision = update.revision;
      for (const program of update.programs) programs.set(program.id, program);
      for (const instance of update.instances) instances.set(instance.id, instance);
      for (const id of update.removed) {
        assert.ok(instances.has(id), `Compose update removes an unknown instance: ${id}`);
        instances.delete(id);
      }
      rootId = update.root.instanceId + '#' + update.root.node;
      if (projectTree) projectNodes();
    }
    const projected = { ...response };
    if (projectTree && response.update) {
      liveMounts = new Set();
      projected.tree = tree(rootId);
      for (const [path, mount] of mounts) {
        if (!liveMounts.has(path)) {
          for (const id of mount.actions.values()) actionTargets.delete(id);
          mounts.delete(path);
        }
      }
    }
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
      /** Models call-owned subscription cleanup independently of command completion. */
      onCleanup(cleanup) {
        const owner = { cleanup };
        cleanups.add(owner);
        return () => { cleanups.delete(owner); };
      },
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
        if (name === '__operit_dispatch_compose_dsl_action' && args[0].actionId.startsWith('@fixture:')) {
          assert.ok(actionTargets.has(args[0].actionId), 'Compose action not found: removed test mount');
          args[0] = { ...args[0], actionId: actionTargets.get(args[0].actionId) };
        }
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
  return { accept, adapt, get nodes() { return projectNodes(); }, programs, instances, events, errors, get latest() { return latest; }, get root() { return nodes.get(rootId); } };
}
