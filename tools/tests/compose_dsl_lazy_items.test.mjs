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
    return ctx.UI.Column({ onClick: () => setCount(3) }, [
      ctx.UI.LazyColumn({ key: 'records', ...ctx.lazyItems({key: 'records', itemCount: count, itemExtent: 40, initialItemCount: 12},
        index => ctx.UI.Text({ key: 'item:' + index, text: 'Item ' + index, onClick: () => index })) })
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
  assert.equal(ui.first.update.upserts.length, 14);
  const item8 = ui.first.update.upserts.find(record => record.props.key === 'item:8').id;
  const discardedAction = initial.children[0].props.onClick;
  const shifted = await ui.dispatch(initial.props.viewport.onRangeChanged, { firstIndex: 8, lastIndex: 20 });
  const shiftedList = list(shifted);
  assert.equal(shiftedList.children.length, 12);
  assert.equal(shiftedList.props.viewport.firstIndex, 8);
  assert.equal(shiftedList.children[0].props.text, 'Item 8');
  assert.ok(!shifted.update.removed.includes(item8));
  assert.ok(!shifted.update.upserts.some(record => record.id === item8));
  const selected = await ui.dispatch(shiftedList.children[0].props.onClick);
  assert.equal(selected.actionResult, 8);
  assert.throws(() => ui.dispatch(discardedAction), /not found/);
  const end = await ui.dispatch(shiftedList.props.viewport.onRangeChanged, { firstIndex: 490, lastIndex: 500 });
  assert.equal(list(end).children.at(-1).props.text, 'Item 499');
  assert.ok(end.update.upserts.length <= 13);
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
