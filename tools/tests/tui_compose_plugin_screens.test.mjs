import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

/** Reads the production JavaScript embedded in a Rust source file. */
function embedded(name) {
  const source = readFileSync(new URL(`../../core/crates/plugin/sdk/src/toolpkg/${name}`, import.meta.url), 'utf8');
  return source.slice(source.indexOf('r#"') + 3, source.lastIndexOf('"#'));
}

/** Loads the actual packaged screen without replacing its callback implementation. */
function load(context, url) {
  const module = { exports: {} };
  vm.runInContext(`(function(module, exports, require) {${readFileSync(url, 'utf8')}\n})`, context)(
    module, module.exports, name => load(context, new URL(name, url)),
  );
  return module.exports;
}

/** Creates a UI runtime with recorded IPC endpoints instead of a running chat service. */
function runtime(screen) {
  const calls = [];
  const context = vm.createContext({ console, module: { exports: {} }, Icons: { Assignment: 'assignment' },
    getLang: () => 'en',
    async toolCall(name) { assert.equal(name, 'toast'); return { success: true }; },
    ToolPkg: { ipc: { async call(channel, payload) {
      calls.push({ channel, payload });
      if (channel === 'plan_mode.is_plan_started') return false;
      return { success: true };
    } } },
    Tools: { System: { async toast() {} } },
  });
  vm.runInContext(embedded('ToolPkgComposeDslBridge.rs'), context);
  vm.runInContext(embedded('ToolPkgComposeDslRuntimeScript.rs').replaceAll('{{', '{').replaceAll('}}', '}').replace('{script}', ''), context);
  context.module.exports = load(context, new URL(`../../plugins/packages/buildin/plan_mode/dist/ui/${screen}/index.ui.js`, import.meta.url));
  return { context, calls };
}

/** Enumerates every visible node to obtain the actual serialized callback id. */
function nodes(node) { return [node, ...node.children.flatMap(nodes)]; }

/** Invokes a serialized button callback exactly as the terminal host does. */
async function click(context, node, property = 'onClick', payload = null) {
  return context.__operit_dispatch_compose_dsl_action({ actionId: node.props[property].__actionId, payload });
}

/** Verifies question navigation uses the plugin callbacks and preserves selections. */
test('question arrows advance and return without submitting answers', async () => {
  const { context, calls } = runtime('planask');
  let result = await context.__operit_render_compose_dsl({ state: { chatId: 'test-chat', xmlContent:
    '<planask><title>Plan</title><question id="q1"><title>First?</title><option id="a">A</option><option id="b">B</option></question><question id="q2"><title>Second?</title><option id="a">C</option><option id="b">D</option></question></planask>',
  } });
  result = await click(context, nodes(result.tree).find(node => node.props.key === 'planask-option-q1-a'));
  result = await click(context, nodes(result.tree).find(node => node.type === 'IconButton' && node.props.icon === 'chevronRight'));
  assert.ok(nodes(result.tree).some(node => node.props.text === 'Second?'));
  assert.equal(nodes(result.tree).find(node => node.props.icon === 'chevronRight').props.enabled, false);
  result = await click(context, nodes(result.tree).find(node => node.props.icon === 'chevronLeft'));
  assert.ok(nodes(result.tree).some(node => node.props.text === 'First?'));
  assert.equal(nodes(result.tree).find(node => node.props.key === 'planask-option-q1-a').type, 'FilledTonalButton');
  assert.equal(calls.length, 0);
});

test('question selections submit answers through the existing UI callback', async () => {
  const { context, calls } = runtime('planask');
  let result = await context.__operit_render_compose_dsl({ state: { chatId: 'test-chat', xmlContent:
    '<planask><title>Login</title><description>Choose</description><question id="q1"><title>Method?</title><option id="a">Password</option><option id="b">Passkey</option></question></planask>',
  } });
  const option = nodes(result.tree).find(node => node.props.key === 'planask-option-q1-a');
  assert.ok(option);
  result = await click(context, option);
  const submit = nodes(result.tree).find(node => node.type === 'Button' && node.props.enabled === true);
  assert.ok(submit);
  await click(context, submit);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].channel, 'plan_mode.submit_answers');
  assert.equal(calls[0].payload.chatId, 'test-chat');
  assert.match(calls[0].payload.message, /Password/);
});

test('only a closed plan exposes the existing implementation action', async () => {
  const { context, calls } = runtime('plantodo');
  const streaming = await context.__operit_render_compose_dsl({ state: { chatId: 'test-chat', xmlContent: '<plantodo>Build login' } });
  assert.equal(nodes(streaming.tree).filter(node => node.type === 'Button').length, 0);
  const completed = await context.__operit_render_compose_dsl({ state: { chatId: 'test-chat', xmlContent: '<plantodo>Build login</plantodo>' } });
  const button = nodes(completed.tree).find(node => node.type === 'Button');
  assert.ok(button);
  await click(context, button);
  assert.equal(calls[0].channel, 'plan_mode.start_implementation');
  assert.deepEqual(JSON.parse(JSON.stringify(calls[0].payload)), { planContent: 'Build login', chatId: 'test-chat' });
});

test('custom answers and selected options survive a refreshed XML render', async () => {
  const { context, calls } = runtime('planask');
  const xmlContent = '<planask><title>Login</title><question id="q1"><title>Method?</title><option id="a">Password</option><option id="b">Passkey</option></question></planask>';
  let result = await context.__operit_render_compose_dsl({ state: { chatId: 'test-chat', xmlContent } });
  result = await click(context, nodes(result.tree).find(node => node.props.key === 'planask-option-q1-b'));
  result = await click(context, nodes(result.tree).find(node => node.type === 'TextField'), 'onValueChange', 'Use hardware keys');
  result = await context.__operit_render_compose_dsl({ state: { ...result.state, xmlContent }, memo: result.memo });
  assert.equal(nodes(result.tree).find(node => node.type === 'TextField').props.value, 'Use hardware keys');
  await click(context, nodes(result.tree).find(node => node.type === 'Button' && node.props.enabled === true));
  assert.equal(calls[0].payload.chatId, 'test-chat');
  assert.match(calls[0].payload.message, /Use hardware keys/);
});
