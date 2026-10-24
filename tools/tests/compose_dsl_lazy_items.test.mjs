import assert from 'node:assert/strict';
import vm from 'node:vm';
import test from 'node:test';
import { composeContextScript, wrapComposeScript } from '../runtime/compose_scripts.mjs';
import { compileComposeModule } from '../../plugins/tools/compile_compose_module.mjs';
import { composeStreamFixture } from './support/compose_stream_fixture.mjs';

/** Executes a production compiled list whose catalog is independent of the materialized window. */
function screen(count = 500) {
  const context = vm.createContext({ module: { exports: {} } });
  vm.runInContext(composeContextScript(), context);
  const source = `module.exports.default = function(ctx) {
    const [count, setCount] = ctx.useState('count', ${count});
    const [label, setLabel] = ctx.useState('label', 'Item');
    return ctx.UI.Column({ onClick: (next) => setCount(next ?? 3), refresh: () => {}, rename: next => setLabel(next) }, [
      ctx.UI.LazyColumn({ key: 'records', ...ctx.lazyItems({key: 'records', itemCount: count, itemExtent: 40, initialItemCount: 12},
        index => { globalThis.builderRuns = (globalThis.builderRuns || 0) + 1; return ctx.UI.Text({ key: 'item:' + index, text: label + ' ' + index, onClick: () => index }); }) })
    ]);
  };`;
  vm.runInContext(wrapComposeScript(compileComposeModule(source, 'lazy-items-test')), context);
  const fixture = composeStreamFixture(context);
  fixture.adapt();
  const first = context.__operit_render_compose_dsl({ executionContextKey: 'lazy-items-test' });
  return {
    context, first,
    /** Uses the page's actual range action and retained update stream. */
    dispatch(action, payload) {
      return context.__operit_dispatch_compose_dsl_action({ actionId: action.__actionId, payload });
    },
  };
}

/** Selects the list in the fixture's projected tree without replacing SDK rendering. */
function list(response) { return response.tree.children[0]; }

/** Keeps first render and subsequent range work bounded independently of the catalog size. */
test('compiled lazy items materialize only the requested window and preserve retained identities', async () => {
  const ui = screen();
  const initial = list(ui.first);
  assert.equal(initial.children.length, 12);
  assert.equal(initial.props.viewport.itemCount, 500);
  assert.equal(ui.first.update.instances.length, 13);
  const item8 = ui.first.update.instances.find(record => record.values.includes('item:8')).id;
  const discardedAction = initial.children[0].props.onClick;
  const shifted = await ui.dispatch(initial.props.viewport.onRangeChanged, { firstIndex: 8, lastIndex: 20 });
  const shiftedList = list(shifted);
  assert.equal(shiftedList.children.length, 12);
  assert.equal(shiftedList.props.viewport.firstIndex, 8);
  assert.equal(shiftedList.children[0].props.text, 'Item 8');
  assert.ok(!shifted.update.removed.includes(item8));
  assert.ok(!shifted.update.instances.some(record => record.id === item8));
  const selected = await ui.dispatch(shiftedList.children[0].props.onClick);
  assert.equal(selected.actionResult, 8);
  assert.throws(() => ui.dispatch(discardedAction), /not found/);
  const end = await ui.dispatch(shiftedList.props.viewport.onRangeChanged, { firstIndex: 490, lastIndex: 500 });
  assert.equal(list(end).children.at(-1).props.text, 'Item 499');
  assert.ok(end.update.instances.length <= 13);
});

/** Reconciles a shorter catalog without retaining out-of-bounds rows or a second callback registry. */
test('lazy items clamp their live window when a compiled state binding shrinks the catalog', async () => {
  const ui = screen();
  await ui.dispatch(list(ui.first).props.viewport.onRangeChanged, { firstIndex: 490, lastIndex: 500 });
  const shrunk = await ui.dispatch(ui.first.tree.props.onClick);
  assert.equal(list(shrunk).props.viewport.itemCount, 3);
  assert.equal(list(shrunk).props.viewport.firstIndex, 0);
  assert.deepEqual(Array.from(list(shrunk).children, node => node.props.text), ['Item 0', 'Item 1', 'Item 2']);
});

/** Expansion must not restore a pre-collapse offscreen window or wait for a host refill. */
test('lazy items normalize a collapsed window before expanding again', async () => {
  const ui = screen();
  const end = await ui.dispatch(list(ui.first).props.viewport.onRangeChanged, { firstIndex: 490, lastIndex: 500 });
  const shrunk = await ui.dispatch(end.tree.props.onClick, 3);
  const expanded = await ui.dispatch(shrunk.tree.props.onClick, 500);
  assert.equal(list(expanded).props.viewport.firstIndex, 0);
  assert.equal(list(expanded).children.length, 12);
  assert.equal(list(expanded).children[0].props.text, 'Item 0');
});

/** A small catalog must not permanently reduce the initial/preloaded window after growing. */
test('lazy items fill at least the initial window after a small range grows', async () => {
  const ui = screen(3);
  const small = await ui.dispatch(list(ui.first).props.viewport.onRangeChanged, { firstIndex: 0, lastIndex: 3 });
  const expanded = await ui.dispatch(small.tree.props.onClick, 500);
  assert.equal(list(expanded).children.length, 12);
});

/** Rejects malformed public contracts instead of generating full-list substitute content. */
test('lazy item options and range events are validated at the SDK boundary', async () => {
  const ui = screen();
  const action = list(ui.first).props.viewport.onRangeChanged;
  for (const range of [{ firstIndex: -1, lastIndex: 5 }, { firstIndex: 4, lastIndex: 2 }, { firstIndex: 0.5, lastIndex: 2 }, null]) {
    assert.throws(() => ui.dispatch(action, range), /Invalid Compose lazy item range/);
  }
  const ctx = ui.context.__operit_compose_bundle.ctx;
  for (const options of [{ key: '', itemCount: 5, itemExtent: 40, initialItemCount: 12 },
    { key: 'invalid', itemCount: -1, itemExtent: 40, initialItemCount: 12 },
    { key: 'invalid', itemCount: 5, itemExtent: 0, initialItemCount: 12 }]) {
    assert.throws(() => ctx.lazyItems(options, index => ctx.UI.Text({text: index})), /Invalid Compose lazy item options/);
  }
});

/** Observable input changes invalidate rows without a plugin-maintained dependency array. */
test('lazy windows follow state and catalog changes automatically', async () => {
  const ui = screen(500);
  assert.equal(ui.context.builderRuns, 12);
  const renamed = await ui.dispatch(ui.first.tree.props.rename, 'Updated');
  assert.equal(ui.context.builderRuns, 24);
  assert.equal(list(renamed).children[0].props.text, 'Updated 0');
  const shifted = await ui.dispatch(list(renamed).props.viewport.onRangeChanged, { firstIndex: 8, lastIndex: 20 });
  assert.equal(ui.context.builderRuns, 32);
  assert.equal(list(shifted).children[0].props.text, 'Updated 8');
  const shrunk = await ui.dispatch(shifted.tree.props.onClick, 3);
  assert.equal(ui.context.builderRuns, 35);
  assert.equal(list(shrunk).props.viewport.itemCount, 3);
  assert.deepEqual(Array.from(list(shrunk).children, node => node.props.text), ['Updated 0', 'Updated 1', 'Updated 2']);
});

/** Plain JS callbacks have no inferred stability promise, so they reopen item content. */
test('ordinary actions conservatively refresh lazy content without observable state writes', async () => {
  const ui = screen(500);
  assert.equal(ui.context.builderRuns, 12);
  await ui.dispatch(ui.first.tree.props.refresh);
  assert.equal(ui.context.builderRuns, 24);
});

/** Retains overlap by absolute index and releases off-window callbacks instead of accumulating them. */
test('cached lazy windows construct only entering items and rebuild evicted items on return', async () => {
  const ui = screen(500);
  const initial = list(ui.first);
  const handles = ui.context.__operit_compose_bundle.composition.metrics();
  const removedAction = initial.children[0].props.onClick;
  const sharedAction = initial.children[8].props.onClick;
  const sharedId = ui.first.update.instances.find(record => record.values.includes('item:8')).id;
  const shifted = await ui.dispatch(initial.props.viewport.onRangeChanged, { firstIndex: 8, lastIndex: 20 });
  assert.equal(ui.context.builderRuns, 20, 'only eight entering items need a builder');
  assert.ok(!shifted.update.instances.some(record => record.id === sharedId));
  assert.equal(list(shifted).children[0].props.onClick.__actionId, sharedAction.__actionId);
  assert.equal((await ui.dispatch(sharedAction)).actionResult, 8);
  assert.throws(() => ui.dispatch(removedAction), /not found/);
  const runsBeforeReturn = ui.context.builderRuns;
  const returned = await ui.dispatch(list(shifted).props.viewport.onRangeChanged, { firstIndex: 0, lastIndex: 12 });
  assert.equal(ui.context.builderRuns, runsBeforeReturn + 8, 'evicted items are rebuilt; four overlapping items stay cached');
  assert.equal((await ui.dispatch(list(returned).children[0].props.onClick)).actionResult, 0);
  assert.ok(ui.context.__operit_compose_bundle.composition.metrics().releasedInstances > handles.releasedInstances);
});

/** Each item owns its effects even when a shifted window skips overlapping builders. */
test('cached lazy item effects survive overlap and dispose exactly when an item leaves', async () => {
  const context = vm.createContext({ module: { exports: {} } });
  vm.runInContext(composeContextScript(), context);
  const source = `
    globalThis.mounted = []; globalThis.disposed = []; globalThis.runs = [];
    module.exports.default = function(ctx) {
      return ctx.UI.LazyColumn({ ...ctx.lazyItems({key:'effects', itemCount:20, itemExtent:40,
        initialItemCount:4}, index => {
          globalThis.runs.push(index);
          ctx.useEffect('item-effect', () => {
            globalThis.mounted.push(index);
            return () => globalThis.disposed.push(index);
          }, []);
          return ctx.UI.Text({key:'row:' + index, text:'old ' + index, onClick:() => index});
        }) });
    };
  `;
  function install(text) {
    vm.runInContext(wrapComposeScript(compileComposeModule(text, 'lazy-effect-test')), context);
    fixture.adapt();
  }
  const fixture = composeStreamFixture(context);
  install(source);
  const first = context.__operit_render_compose_dsl({ executionContextKey:'lazy-effect-test' });
  const action = first.tree.props.viewport.onRangeChanged.__actionId;
  const shifted = await context.__operit_dispatch_compose_dsl_action({actionId:action, payload:{firstIndex:2,lastIndex:6}});
  assert.deepEqual(Array.from(context.runs), [0,1,2,3,4,5]);
  assert.deepEqual(Array.from(context.mounted), [0,1,2,3,4,5]);
  assert.deepEqual(Array.from(context.disposed).sort(), [0,1]);
  assert.equal((await context.__operit_dispatch_compose_dsl_action({ actionId:shifted.tree.children[0].props.onClick.__actionId })).actionResult, 2);
  install(source.replace("text:'old '", "text:'new '"));
  const reloaded = context.__operit_render_compose_dsl({ executionContextKey:'lazy-effect-test' });
  assert.deepEqual(Array.from(reloaded.tree.children, node => node.props.text), ['new 2','new 3','new 4','new 5']);
});

/** Nested unkeyed descendants must not be rebound to another row's positional template occurrence. */
test('cached overlap keeps unkeyed descendants and callbacks on their original row', async () => {
  const context = vm.createContext({ module: { exports: {} } });
  vm.runInContext(composeContextScript(), context);
  const source = `module.exports.default = function(ctx) {
    return ctx.UI.LazyColumn({ ...ctx.lazyItems({key:'nested', itemCount:100, itemExtent:40,
      initialItemCount:12}, index => ctx.UI.Row({key:'row:' + index},
        [ctx.UI.Text({text:'row ' + index, onClick:() => index})])) });
  };`;
  vm.runInContext(wrapComposeScript(compileComposeModule(source, 'lazy-nested-test')), context);
  const fixture = composeStreamFixture(context);
  fixture.adapt();
  const first = context.__operit_render_compose_dsl({ executionContextKey:'lazy-nested-test' });
  const action = first.tree.props.viewport.onRangeChanged.__actionId;
  const shared = first.tree.children[2].children[0].props.onClick.__actionId;
  const shifted = await context.__operit_dispatch_compose_dsl_action({actionId:action, payload:{firstIndex:2,lastIndex:14}});
  assert.equal(shifted.update.instances.length, 3, 'only two new row instances and the list root are sent');
  assert.deepEqual(Array.from(shifted.tree.children, node => node.children[0].props.text),
    Array.from({length:12}, (_, index) => 'row ' + (index + 2)));
  assert.equal(shifted.tree.children[0].children[0].props.onClick.__actionId, shared);
  assert.equal((await context.__operit_dispatch_compose_dsl_action({actionId:shared})).actionResult, 2);
});

/** Window invalidation uses engine revisions without serializing the captured catalog. */
test('lazy range shifts never serialize a captured catalog', async () => {
  const context = vm.createContext({ module: { exports: {} } });
  vm.runInContext(composeContextScript(), context);
  vm.runInContext(`
    globalThis.catalog = {marker:'large-catalog', entries:Array.from({length:1000}, (_, id) => ({id}))};
    globalThis.catalogSerializations = 0;
    const stringify = JSON.stringify;
    JSON.stringify = function(value, ...args) {
      if (Array.isArray(value) && value.includes(globalThis.catalog)) globalThis.catalogSerializations++;
      return stringify(value, ...args);
    };
  `, context);
  const source = `module.exports.default = function(ctx) {
    return ctx.UI.LazyColumn({ ...ctx.lazyItems({key:'catalog', itemCount:1000, itemExtent:40,
      initialItemCount:12}, index => ctx.UI.Text({key:'row:' + index, text:globalThis.catalog.entries[index].id})) });
  };`;
  vm.runInContext(wrapComposeScript(compileComposeModule(source, 'lazy-catalog-test')), context);
  const fixture = composeStreamFixture(context);
  fixture.adapt();
  const first = context.__operit_render_compose_dsl({ executionContextKey:'lazy-catalog-test' });
  assert.equal(context.catalogSerializations, 0);
  await context.__operit_dispatch_compose_dsl_action({
    actionId:first.tree.props.viewport.onRangeChanged.__actionId, payload:{firstIndex:2,lastIndex:14},
  });
  assert.equal(context.catalogSerializations, 0);
});

/** Drives invalidation through the real callback owner, with or without portable templates. */
function program(source, precompiled = true, runtimeOptions = {}) {
  const context = vm.createContext({ module: { exports: {} } });
  vm.runInContext(composeContextScript(), context);
  vm.runInContext(wrapComposeScript(precompiled ? compileComposeModule(source, 'automatic-lazy') : source), context);
  const fixture = composeStreamFixture(context);
  fixture.adapt();
  const first = context.__operit_render_compose_dsl({ executionContextKey: 'automatic-lazy', ...runtimeOptions });
  return { context, first,
    dispatch(action, payload, options = {}) {
      return context.__operit_dispatch_compose_dsl_action({ actionId: action.__actionId, payload, ...options });
    },
    render(options = {}) { return context.__operit_render_compose_dsl({ executionContextKey: 'automatic-lazy', ...options }); },
  };
}

for (const precompiled of [true, false]) {
  const mode = precompiled ? 'templates' : 'untransformed JS';
  test(`automatic lazy invalidation observes host input and refreshes captured callbacks (${mode})`, async () => {
    const ui = program(`module.exports.default = function(ctx) {
      const [label] = ctx.useState('label', 'before');
      return ctx.UI.LazyColumn({...ctx.lazyItems({key:'inputs',itemCount:20,itemExtent:40,initialItemCount:4},
        index => ctx.UI.Text({key:'row:' + index,text:label + index,onClick:() => label + index}))});
    };`, precompiled);
    const saved = ui.first.tree.children[2].props.onClick;
    const shifted = await ui.dispatch(ui.first.tree.props.viewport.onRangeChanged, {firstIndex:2,lastIndex:6});
    assert.equal(shifted.tree.children[0].props.text, 'before2');
    const updated = ui.render({__operit_update_inputs:true,__operit_input_state:{label:'after'}});
    assert.deepEqual(Array.from(updated.tree.children, node => node.props.text), ['after2','after3','after4','after5']);
    assert.equal((await ui.dispatch(saved)).actionResult, 'after2');
  });

  test(`ordinary no-render callbacks invalidate mutable cyclic refs before the next range (${mode})`, async () => {
    const ui = program(`module.exports.default = function(ctx) {
      const ref = ctx.useRef('model', {label:'before'}); ref.current.self = ref.current;
      return ctx.UI.LazyColumn({change:() => {ref.current.label='after';},
        ...ctx.lazyItems({key:'refs',itemCount:20,itemExtent:40,initialItemCount:4}, index => {
          globalThis.runs=(globalThis.runs || 0)+1;
          return ctx.UI.Text({key:'row:' + index,text:ref.current.label + index});
        })});
    };`, precompiled);
    await ui.dispatch(ui.first.tree.props.change, {__no_render:true});
    const updated = await ui.dispatch(ui.first.tree.props.viewport.onRangeChanged, {firstIndex:2,lastIndex:6});
    assert.equal(ui.context.runs, 8, 'all four rows see the untracked business mutation');
    assert.deepEqual(Array.from(updated.tree.children, node => node.props.text), ['after2','after3','after4','after5']);
    const shifted = await ui.dispatch(updated.tree.props.viewport.onRangeChanged, {firstIndex:3,lastIndex:7});
    assert.equal(ui.context.runs, 9, 'only the new row executes on the following pure viewport event');
    assert.equal(shifted.tree.children[0].props.text, 'after3');
  });

  test(`wrapping a range callback with business code never inherits viewport-only reuse (${mode})`, async () => {
    const ui = program(`module.exports.default = function(ctx) {
      const model = ctx.useRef('model', {label:'before'});
      const items = ctx.lazyItems({key:'wrapped',itemCount:20,itemExtent:40,initialItemCount:4}, index =>
        ctx.UI.Text({key:'row:' + index,text:model.current.label + index}));
      return ctx.UI.LazyColumn({...items,change:range => {
        model.current.label='after'; items.viewport.onRangeChanged(range);
      }});
    };`, precompiled);
    const updated = await ui.dispatch(ui.first.tree.props.change, {firstIndex:2,lastIndex:6});
    assert.deepEqual(Array.from(updated.tree.children, node => node.props.text), ['after2','after3','after4','after5']);
  });

  test(`a pending business action and concurrent viewport preserve before/after-await refs (${mode})`, async () => {
    const ui = program(`module.exports.default = function(ctx) {
      const model = ctx.useRef('async-model', {label:'initial'});
      return ctx.UI.LazyColumn({change:() => {
        model.current.label='pending';
        return new Promise(resolve => {globalThis.finish=() => {model.current.label='done';resolve();};});
      },...ctx.lazyItems({key:'async',itemCount:20,itemExtent:40,initialItemCount:4}, index =>
        ctx.UI.Text({key:'row:' + index,text:model.current.label + index}))});
    };`, precompiled);
    const pending = ui.dispatch(ui.first.tree.props.change);
    await new Promise(resolve => setImmediate(resolve));
    const viewport = await ui.dispatch(ui.first.tree.props.viewport.onRangeChanged, {firstIndex:2,lastIndex:6});
    assert.equal(viewport.tree.children[0].props.text, 'pending2');
    ui.context.finish();
    const completed = await pending;
    assert.equal(completed.tree.children[0].props.text, 'done2');
  });
}

test('changing a template callback slot from viewport to business clears its old classification', async () => {
  const ui = program(`module.exports.default = function(ctx) {
    const [viewport] = ctx.useState('viewport', true);
    const model=ctx.useRef('model', {label:'before'});
    const items=ctx.lazyItems({key:'slots',itemCount:20,itemExtent:40,initialItemCount:4}, index =>
      ctx.UI.Text({key:'row:' + index,text:model.current.label + index}));
    return ctx.UI.LazyColumn({...items, onClick:viewport ? items.viewport.onRangeChanged : () => {model.current.label='after';}});
  };`);
  const saved = ui.first.tree.props.onClick;
  await ui.dispatch(saved, {firstIndex:2,lastIndex:6});
  ui.render({__operit_update_inputs:true,__operit_input_state:{viewport:false}});
  const updated = await ui.dispatch(saved);
  assert.equal(updated.tree.children[0].props.text, 'after2');
});

test('theme changes invalidate cached rows even when delivered with a range event', async () => {
  const ui = program(`module.exports.default = function(ctx) {
    return ctx.UI.LazyColumn({...ctx.lazyItems({key:'theme',itemCount:20,itemExtent:40,initialItemCount:4}, index =>
      ctx.UI.Text({key:'row:' + index,text:ctx.Theme.getCurrent().brightness + index}))});
  };`, true, {theme:{brightness:'light',colors:{primary:'#ffffffff'}}});
  const updated = await ui.dispatch(ui.first.tree.props.viewport.onRangeChanged, {firstIndex:2,lastIndex:6},
    {theme:{brightness:'dark',colors:{primary:'#ff000000'}}});
  assert.deepEqual(Array.from(updated.tree.children, node => node.props.text), ['dark2','dark3','dark4','dark5']);
});
