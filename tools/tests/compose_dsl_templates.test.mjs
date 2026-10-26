import assert from 'node:assert/strict';
import vm from 'node:vm';
import test from 'node:test';
import { composeContextScript, wrapComposeScript } from '../runtime/compose_scripts.mjs';
import { compileComposeModule } from '../../plugins/tools/compile_compose_module.mjs';
import { composeStreamFixture } from './support/compose_stream_fixture.mjs';

/** Executes generated artifacts against the same SDK, retained graph and stream protocol as production. */
function screen(source) {
  const context = vm.createContext({ console, module: { exports: {} } });
  vm.runInContext(composeContextScript(), context);
  const fixture = composeStreamFixture(context);
  function install(next) {
    const compiled = compileComposeModule(next, 'template-behavior');
    vm.runInContext(wrapComposeScript(compiled), context);
    fixture.adapt();
    return compiled;
  }
  const compiled = install(source);
  return { context, fixture, compiled, install,
    render(options = {}) { return context.__operit_render_compose_dsl({ executionContextKey: 'templates', ...options }); },
    action(id, payload) { return context.__operit_dispatch_compose_dsl_action({ actionId: id, payload }); },
  };
}

test('templates retain strict mode, source evaluation order and latest callback closures', async () => {
  const s = screen(`'use strict';
    function draw(ctx) {
      globalThis.strictReceiver = (function(){ return this; })();
      const [count, setCount] = ctx.useState('count', 0);
      const mark = value => { globalThis.order.push(value); return value; };
      return ctx.UI.Button({ label: mark('first'), onClick: () => setCount(count + 1),
        style: { padding: 4, label: mark('second') } }, [ctx.UI.Text({ text: String(count) })]);
    }
    module.exports.default = ctx => draw(ctx);
  `);
  s.context.order = [];
  const first = s.render();
  assert.equal(s.context.strictReceiver, undefined);
  assert.deepEqual(Array.from(s.context.order), ['first', 'second']);
  const id = first.tree.props.onClick.__actionId;
  assert.equal((await s.action(id)).tree.children[0].props.text, '1');
  assert.equal((await s.action(id)).tree.children[0].props.text, '2');
  assert.deepEqual(Array.from(s.context.order), ['first', 'second', 'first', 'second', 'first', 'second']);
  assert.match(s.compiled, /OperitComposeComposition\.evaluate/);
  assert.match(s.compiled, /var __operitUiPrograms_/);
});

test('dynamic objects, node handles and plugin node mutations preserve their semantics', () => {
  const s = screen(`
    globalThis.model = { nested: { value: 1 } };
    module.exports.default = function(ctx) {
      const child = ctx.UI.Text({ text: 'constant child', metadata: globalThis.model });
      globalThis.child = child;
      return ctx.UI.Column({ key: 'root' }, [child]);
    };
  `);
  s.render();
  const first = s.context.child;
  s.context.child.props.text = 'plugin mutation';
  assert.equal(s.render().tree.children[0].props.text, 'plugin mutation');
  s.context.model.nested.value = 2;
  const changed = s.render();
  assert.equal(s.context.child, first);
  assert.equal(changed.tree.children[0].props.text, 'constant child');
  assert.equal(changed.tree.children[0].props.metadata.nested.value, 2);
});

test('module reload updates constant template structure even when dynamic values match', () => {
  const source = `module.exports.default = function(ctx) {
    const [value] = ctx.useState('value', 'same');
    return ctx.UI.Text({ text: value, fontSize: 12 });
  };`;
  const s = screen(source);
  assert.equal(s.render().tree.props.fontSize, 12);
  s.install(source.replace('fontSize: 12', 'fontSize: 13'));
  assert.equal(s.render().tree.props.fontSize, 13);
});

for (const [name, props, expected] of [
  ['duplicate keys', "ignored: poison, ignored: 'safe', text: 'ready'", []],
  ['numeric enumeration', "2: value('two'), 1: value('one'), text: 'ready'", ['one', 'two']],
  ['computed keys', "['computed']: value('computed'), text: 'ready'", ['computed']],
  ['spreads', "...spread, text: 'ready'", ['spread']],
  ['accessor properties', "get text(){ globalThis.reads.push('getter'); return 'ready'; }", ['getter']],
  ['prototype property', "__proto__: { hidden: 'prototype' }, text: 'ready'", []],
]) {
  test('templates preserve object semantics for ' + name, () => {
    const s = screen(`
      globalThis.reads = [];
      const value = id => ({ get observed(){ globalThis.reads.push(id); return id; } });
      module.exports.default = function(ctx) {
        const poison = { get fail(){ throw Error('overwritten value was inspected'); } };
        const spread = { get extra(){ globalThis.reads.push('spread'); return 3; } };
        return ctx.UI.Text({ ${props} });
      };
    `);
    assert.equal(s.render().tree.props.text, 'ready');
    assert.deepEqual(Array.from(s.context.reads), expected);
  });
}

test('spread UI arguments keep dependency subscriptions in compiled lexical frames', async () => {
  const s = screen(`module.exports.default = function(ctx) {
    const [count, setCount] = ctx.useState('count', 0);
    const args = [{ text: count, onClick: () => setCount(count + 1) }];
    return ctx.UI.Text(...args);
  };`);
  assert.match(s.compiled, /OperitComposeReactive\.frame/);
  const first = s.render();
  const next = await s.action(first.tree.props.onClick.__actionId);
  assert.equal(String(next.tree.props.text), '1');
});

test('old executable artifacts are rejected before entering the program runtime', () => {
  for (const version of [1, 2, 3]) assert.throws(() => screen('// @operit-compose-artifact ' + version + '\nmodule.exports.default=ctx=>ctx.UI.Text({});'), /rebuild the plugin/);
});

/** A skipped memo must not donate its call-site occurrence to a freshly evaluated sibling. */
test('memoized UI retains independent generation scopes and callback captures', async () => {
  const s = screen(`
    function row(ctx, label) {
      return ctx.UI.Button({ text: label, onClick: () => label });
    }
    module.exports.default = function(ctx) {
      const [count, setCount] = ctx.useState('count', 0);
      const cached = ctx.useMemo('cached-row', () => row(ctx, 'cached'), []);
      return ctx.UI.Column({ next: () => setCount(count + 1) }, [cached, row(ctx, 'live:' + count)]);
    };
  `);
  const first = s.render();
  const cachedId = first.tree.children[0].props.onClick.__actionId;
  const next = await s.action(first.tree.props.next.__actionId);
  assert.deepEqual(Array.from(next.tree.children, node => node.props.text), ['cached', 'live:1']);
  assert.equal((await s.action(cachedId)).actionResult, 'cached');
  assert.equal((await s.action(next.tree.children[1].props.onClick.__actionId)).actionResult, 'live:1');
});

/** Hot reload must refresh generated UI while leaving ordinary JS memo identity intact. */
test('source replacement refreshes node-producing memos and preserves memoized data', () => {
  const source = `
    function row(ctx) { return ctx.UI.Text({ text: 'old source' }); }
    module.exports.default = function(ctx) {
      globalThis.data = ctx.useMemo('data', () => ({ count: 1 }), []);
      return ctx.useMemo('parent', () => ctx.useMemo('child', () => row(ctx), []), []);
    };
  `;
  const s = screen(source);
  assert.equal(s.render().tree.props.text, 'old source');
  const data = s.context.data;
  data.count = 9;
  s.install(source.replace('old source', 'new source'));
  assert.equal(s.render().tree.props.text, 'new source');
  assert.equal(s.context.data, data);
  assert.equal(s.context.data.count, 9);
});

/** ctx.h nodes need the same source-lifetime handling as compiler-generated templates. */
test('memoized direct node factories also refresh on source replacement', () => {
  const source = `module.exports.default = ctx => ctx.useMemo('direct', () => ctx.h('Text', { text: 'old' }), []);`;
  const s = screen(source);
  assert.equal(s.render().tree.props.text, 'old');
  s.install(source.replace("text: 'old'", "text: 'new'"));
  assert.equal(s.render().tree.props.text, 'new');
});

/** A retained memo can leave the visible graph and remount without sharing another row's capture cells. */
test('memoized UI can disappear and remount while live sibling captures keep changing', async () => {
  const s = screen(`
    function row(ctx, label) { return ctx.UI.Button({ text: label, onClick: () => label }); }
    module.exports.default = function(ctx) {
      const [visible, show] = ctx.useState('visible', true);
      const [count, countTo] = ctx.useState('count', 0);
      const cached = visible ? ctx.useMemo('cached', () => row(ctx, 'cached'), []) : null;
      return ctx.UI.Column({ toggle: () => show(!visible), next: () => countTo(count + 1) }, [cached, row(ctx, 'live:' + count)]);
    };
  `);
  const first = s.render();
  const hidden = await s.action(first.tree.props.toggle.__actionId);
  const changed = await s.action(hidden.tree.props.next.__actionId);
  const restored = await s.action(changed.tree.props.toggle.__actionId);
  assert.deepEqual(Array.from(restored.tree.children, node => node.props.text), ['cached', 'live:1']);
  assert.equal((await s.action(restored.tree.children[0].props.onClick.__actionId)).actionResult, 'cached');
});

/** Re-evaluating a memo can reuse every node; it still owns generated UI across a later reload. */
test('node-producing memos retain reload ownership when changed dependencies reuse the same template', async () => {
  const source = `module.exports.default = function(ctx) {
    const [count, setCount] = ctx.useState('count', 0);
    const row = ctx.useMemo('row', () => ctx.UI.Text({ text: 'old' }), [count]);
    return ctx.UI.Column({ next: () => setCount(count + 1) }, [row]);
  };`;
  const s = screen(source);
  const first = s.render();
  assert.equal((await s.action(first.tree.props.next.__actionId)).tree.children[0].props.text, 'old');
  s.install(source.replace("text: 'old'", "text: 'new'"));
  assert.equal(s.render().tree.children[0].props.text, 'new');
});

/** A cached child must keep its independent state subscriptions while its factory is skipped. */
test('memoized compiled children retain state subscriptions after a parent render', async () => {
  const s = screen(`
    function child(ctx) {
      const [count, setCount] = ctx.useState('child-count', 0);
      return ctx.UI.Button({ text: count, onClick: () => setCount(count + 1) });
    }
    module.exports.default = function(ctx) {
      const [outer, setOuter] = ctx.useState('outer', 0);
      const cached = ctx.useMemo('child', () => child(ctx), []);
      return ctx.UI.Column({ text: outer, next: () => setOuter(outer + 1) }, [cached]);
    };
  `);
  const first = s.render();
  const next = await s.action(first.tree.props.next.__actionId);
  assert.equal(next.tree.props.text, 1);
  const updated = await s.action(next.tree.children[0].props.onClick.__actionId);
  assert.equal(updated.tree.children[0].props.text, 1);
  assert.equal((await s.action(updated.tree.children[0].props.onClick.__actionId)).tree.children[0].props.text, 2);
});

/** Effects in a retained factory belong to the memo branch, including its removal and remount. */
test('memoized effects remain active until their owning branch leaves the composition', async () => {
  const s = screen(`
    function child(ctx) {
      ctx.useEffect('listen', () => {
        globalThis.events.push('open');
        return () => globalThis.events.push('close');
      }, []);
      return ctx.UI.Text({ text: 'cached' });
    }
    module.exports.default = function(ctx) {
      const [visible, setVisible] = ctx.useState('visible', true);
      const [count, setCount] = ctx.useState('count', 0);
      const cached = visible ? ctx.useMemo('child', () => child(ctx), []) : null;
      return ctx.UI.Column({ next: () => setCount(count + 1), toggle: () => setVisible(!visible) }, [cached]);
    };
  `);
  s.context.events = [];
  const first = s.render();
  const next = await s.action(first.tree.props.next.__actionId);
  assert.deepEqual(Array.from(s.context.events), ['open']);
  const hidden = await s.action(next.tree.props.toggle.__actionId);
  assert.deepEqual(Array.from(s.context.events), ['open', 'close']);
  const restored = await s.action(hidden.tree.props.toggle.__actionId);
  assert.equal(restored.tree.children[0].props.text, 'cached');
  assert.deepEqual(Array.from(s.context.events), ['open', 'close', 'open']);
  s.install(`module.exports.default = ctx => ctx.UI.Text({ text: 'reloaded' });`);
  s.render();
  assert.deepEqual(Array.from(s.context.events), ['open', 'close', 'open', 'close']);
});

/** Repeated compiled helpers inside separate memo factories must not share reactive occurrence counters. */
test('separate memoized compiled children retain independent state bindings', async () => {
  const s = screen(`
    function child(ctx) {
      const [count, setCount] = ctx.useState('child-count', 0);
      return ctx.UI.Button({ text: count, onClick: () => setCount(count + 1) });
    }
    module.exports.default = ctx => ctx.UI.Column({}, [
      ctx.useMemo('first', () => child(ctx), []),
      ctx.useMemo('second', () => child(ctx), [])
    ]);
  `);
  const first = s.render();
  s.render();
  const next = await s.action(first.tree.children[1].props.onClick.__actionId);
  assert.deepEqual(Array.from(next.tree.children, node => node.props.text), [1, 1]);
});


/** Repeated list templates keep independent latest captures while unchanged rows produce no updates. */
test('large compiled lists retain row callbacks across unchanged renders and reordering', async () => {
  const s = screen(`
    globalThis.rows = Array.from({ length: 256 }, (_, id) => id);
    globalThis.epoch = 0;
    /** Builds one repeated row with a separately captured identity and epoch. */
    function row(ctx, id, epoch) {
      return ctx.UI.Button({ key: 'row-' + id, text: id, onClick: () => id + ':' + epoch });
    }
    module.exports.default = function(ctx) {
      return ctx.UI.Column({}, globalThis.rows.map(id => row(ctx, id, globalThis.epoch)));
    };
  `);
  const first = s.render();
  const lastAction = first.tree.children[255].props.onClick.__actionId;
  s.context.epoch = 1;
  const unchanged = s.render();
  assert.equal(unchanged.update.instances.length, 0);
  assert.equal((await s.action(lastAction)).actionResult, '255:1');
  s.context.rows.reverse();
  s.context.epoch = 2;
  const reordered = s.render();
  assert.equal(reordered.tree.children[0].props.text, 255);
  assert.equal(reordered.tree.children[255].props.text, 0);
  assert.equal(reordered.tree.children[0].props.onClick.__actionId, lastAction);
  assert.equal((await s.action(lastAction)).actionResult, '255:2');
  assert.equal((await s.action(reordered.tree.children[255].props.onClick.__actionId)).actionResult, '0:2');
});

/** Fixed records and child layouts must not enter the opaque property interpreter during construction. */
test('compiled fixed layouts construct normalized properties and children directly', () => {
  const s = screen(`module.exports.default = ctx => ctx.UI.Column({
    style: { padding: 4, border: { widths: [1, 2, 3] } }
  }, [[ctx.UI.Text({ text: 'first' })], null, 7, [ctx.UI.Text({ text: 'second' })]]);`);
  const counts = { property: 0, value: 0, children: 0 };
  for (const name of Object.keys(counts)) {
    const original = s.context.OperitComposeNodes[name];
    /** Counts interpreter entry points without changing their return values. */
    s.context.OperitComposeNodes[name] = function (...args) { counts[name]++; return original(...args); };
  }
  const result = s.render();
  assert.deepEqual(counts, { property: 0, value: 0, children: 0 });
  assert.deepEqual(Array.from(result.tree.children, node => node.props.text), ['first', 'second']);
  assert.deepEqual(JSON.parse(JSON.stringify(result.tree.props.style)), { padding: 4, border: { widths: [1, 2, 3] } });
});

/** Modifier syntax is lowered from the AST while dynamic arguments retain source order and updated values. */
test('compiled modifier plans preserve operation order, captures and independent row storage', async () => {
  const s = screen(`
    globalThis.events = [];
    const mark = value => { globalThis.events.push(value); return value; };
    function row(ctx, size) {
      return ctx.UI.Text({ text: size, modifier: ctx.Modifier.clip({ cornerRadius: mark(size) }).padding(mark(size + 1)) });
    }
    module.exports.default = function(ctx) {
      const [size, change] = ctx.useState('size', 3);
      return ctx.UI.Column({ next: () => change(size + 1) }, [row(ctx, size), row(ctx, size + 10)]);
    };
  `);
  const first = s.render();
  assert.deepEqual(Array.from(s.context.events), [3, 4, 13, 14]);
  assert.doesNotMatch(s.compiled, /ctx\.Modifier\./);
  assert.deepEqual(JSON.parse(JSON.stringify(first.tree.children[0].props.modifier)), {
    __modifierOps: [{ name: 'clip', args: [{ cornerRadius: 3 }] }, { name: 'padding', args: [4] }],
  });
  const changed = await s.action(first.tree.props.next.__actionId);
  assert.equal(changed.tree.children[0].props.modifier.__modifierOps[0].args[0].cornerRadius, 4);
  assert.equal(changed.tree.children[1].props.modifier.__modifierOps[0].args[0].cornerRadius, 14);
});

/** Only the active context's fluent modifier API participates in modifier lowering. */
test('modifier plans preserve foreign receivers and terminal serialization methods', () => {
  const s = screen(`
    const other = { Modifier: { custom(value) { return { value }; } } };
    module.exports.default = ctx => ctx.UI.Text({
      foreign: other.Modifier.custom(4), modifier: ctx.Modifier.padding(7).toJSON()
    });
  `);
  const result = s.render();
  assert.equal(result.tree.props.foreign.value, 4);
  assert.deepEqual(JSON.parse(JSON.stringify(result.tree.props.modifier)), { __modifierOps: [{ name: 'padding', args: [7] }] });
});

/** Slot construction and content precedence must remain identical for explicit, absent and undefined children. */
test('compiled slots and content keep child membership and content precedence', () => {
  const s = screen(`module.exports.default = function(ctx) {
    const content = ctx.UI.Text({ text: 'content' });
    return ctx.UI.Column({}, [
      ctx.UI.Box({ content, leading: ctx.UI.Text({ text: 'slot' }) }),
      ctx.UI.Box({ content }, undefined),
      ctx.UI.Box({ content }, []),
      ctx.UI.Box({ content }, null)
    ]);
  };`);
  const result = s.render();
  const boxes = result.tree.children;
  assert.deepEqual(Array.from(boxes, box => box.children.length), [1, 1, 0, 0]);
  assert.equal(boxes[0].slots.leading[0].props.text, 'slot');
  assert.equal(boxes[2].slots.content[0].props.text, 'content');
  assert.equal(boxes[3].slots.content[0].props.text, 'content');
});

/** Callback-property metadata must include nested callbacks and explicit references alongside ordinary data. */
test('compiled callback binding preserves nested paths and explicit action references', async () => {
  const s = screen(`module.exports.default = function(ctx) {
    const button = ctx.UI.Button({ onClick: () => 'shared' });
    return ctx.UI.Column({
      metadata: { ordinary: { padding: 4 }, nested: [() => 'nested'] },
      explicit: { __actionId: button.props.onClick.__actionId },
      onClick: () => 'root'
    }, [button]);
  };`);
  const first = s.render();
  assert.equal((await s.action(first.tree.props.metadata.nested[0].__actionId)).actionResult, 'nested');
  assert.equal((await s.action(first.tree.props.explicit.__actionId)).actionResult, 'shared');
  assert.equal((await s.action(first.tree.props.onClick.__actionId)).actionResult, 'root');
});

/** Plugin mutations before first mount must invalidate the constructor's callback-property metadata. */
test('compiled property metadata observes callbacks added before mount and after detachment', async () => {
  const s = screen(`module.exports.default = function(ctx) {
    const [visible, show] = ctx.useState('visible', true);
    const cached = ctx.useMemo('cached', () => ctx.UI.Text({ text: 'cached' }), []);
    const trigger = ctx.UI.Button({ toggle: () => show(!visible) });
    if (visible) cached.props.event = trigger.props.toggle;
    globalThis.cached = cached;
    return ctx.UI.Column({}, visible ? [trigger, cached] : [trigger]);
  };`);
  const first = s.render();
  assert.ok(first.tree.children[1].props.event.__actionId);
  const hidden = await s.action(first.tree.children[1].props.event.__actionId);
  assert.equal(hidden.tree.children.length, 1);
  const restored = await s.action(hidden.tree.children[0].props.toggle.__actionId);
  assert.ok(restored.tree.children[1].props.event.__actionId);
});

/** Compiled fluent modifiers inherit the SDK chain rather than reading its replaceable serialization field. */
test('compiled modifier plans retain base operations and serialization replacement semantics', () => {
  const s = screen(`module.exports.default = function(ctx) {
    ctx.Modifier = ctx.Modifier.padding(5);
    ctx.Modifier.__modifierOps = [{ name: 'serialized', args: [99] }];
    return ctx.UI.Text({ modifier: ctx.Modifier.clip({ cornerRadius: 3 }) });
  };`);
  assert.deepEqual(JSON.parse(JSON.stringify(s.render().tree.props.modifier)), {
    __modifierOps: [{ name: 'padding', args: [5] }, { name: 'clip', args: [{ cornerRadius: 3 }] }],
  });
});

/** Each fluent operation captures its base after its own arguments and before later operation arguments run. */
test('compiled modifier operations retain argument mutation timing across a chain', () => {
  const s = screen(`module.exports.default = function(ctx) {
    ctx.Modifier = ctx.Modifier.padding(1);
    const base = ctx.Modifier.__modifierOps;
    return ctx.UI.Text({ modifier: ctx.Modifier
      .width((base.push({name: 'first', args: []}), 2))
      .height((base.push({name: 'second', args: []}), 3)) });
  };`);
  assert.deepEqual(Array.from(s.render().tree.props.modifier.__modifierOps, operation => operation.name), ['padding', 'first', 'width', 'height']);
});
