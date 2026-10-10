import { composeStreamFixture } from './support/compose_stream_fixture.mjs';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

const sourceRoot = new URL('../../core/crates/plugin/sdk/src/toolpkg/', import.meta.url);

/** Extracts the embedded JavaScript without compiling the Rust crate. */
function embeddedScript(name) {
  const source = readFileSync(new URL(name, sourceRoot), 'utf8');
  return source.slice(source.indexOf('r#"') + 3, source.lastIndexOf('"#'));
}

/** Runs the production context bridge and action wrapper with a clickable site card. */
function createRuntime(environment = {}) {
  const events = [];
  const context = vm.createContext({
    module: { exports: {} },
    __operit_call_runtime_ref: {
      /** Reads host environment storage across independent route contexts. */
      getEnv(key) { return environment[key]; },
    },
    __operitCurrentCallId: 'owner',
    /** Records the selected site through the structured host value contract. */
    __operitNativeSetEnv(owner, key, value) { assert.equal(owner, 'owner'); environment[key] = value; },
    /** Captures every intermediate response to verify commands are emitted once. */
    sendIntermediateResult(value) { events.push(value); },
  });
  for (const name of ['ToolPkgComposeDslCompiler.js', 'ToolPkgComposeDslRetained.js', 'ToolPkgComposeDslReactive.js']) {
    vm.runInContext(readFileSync(new URL(name, sourceRoot), 'utf8'), context);
  }
  vm.runInContext(embeddedScript('ToolPkgComposeDslBridge.rs'), context);
  const wrapper = embeddedScript('ToolPkgComposeDslRuntimeScript.rs')
    .replaceAll('{{', '{').replaceAll('}}', '}')
    .replace('{script}', `
      /** Builds a card whose async callback intentionally returns no value. */
      module.exports.default = function(ctx) {
        return ctx.UI.Card({
          modifier: ctx.Modifier.fillMaxWidth().clickable(async function() {
            await ctx.setEnv('selected_site', 'doubao');
            await ctx.navigate('toolpkg:sites:ui:viewer', { id: 'doubao' });
          })
        }, ctx.UI.Text({ text: 'Doubao' }));
      };
    `);
  vm.runInContext(wrapper, context);
  composeStreamFixture(context, { onResponse(phase, response) { if (phase === 'intermediate') events.push(response); } }).adapt();
  return { context, events, environment };
}

test('clickable navigation survives an async callback with no return value', async () => {
  const { context, events, environment } = createRuntime();
  const initial = await context.__operit_render_compose_dsl({});
  const actionId = initial.tree.props.modifier.__modifierOps[1].args[0].__actionId;
  const result = await context.__operit_dispatch_compose_dsl_action({ actionId });
  assert.equal(environment.selected_site, 'doubao');
  assert.equal(result.actionResult, undefined);
  assert.ok(events.every(response => response.navigationCommands.length === 0));
  const commands = [...events, result].flatMap(response => response.navigationCommands);
  assert.deepEqual(JSON.parse(JSON.stringify(commands)), [{
    route: 'toolpkg:sites:ui:viewer', args: { id: 'doubao' },
  }]);
  const rerendered = await context.__operit_render_compose_dsl({ __operit_update_inputs: true });
  assert.equal(rerendered.navigationCommands.length, 0);
});

/** Evaluates a packaged CommonJS screen and its relative modules in one runtime. */
function loadPackageModule(context, url) {
  const module = { exports: {} };
  const evaluate = vm.runInContext(
    `(function(module, exports, require) {${readFileSync(url, 'utf8')}\n})`, context,
  );
  evaluate(module, module.exports, path => loadPackageModule(context, new URL(path, url)));
  return module.exports;
}

/** Collects rendered DSL nodes for behavior assertions. */
function descendants(node) {
  return [node, ...node.children.flatMap(descendants)];
}

test('packaged site list selection reaches a fresh viewer context and WebView URL', async () => {
  const environment = {};
  const list = createRuntime(environment).context;
  const base = new URL('../../plugins/packages/external/sidebar_model_sites/dist/', import.meta.url);
  list.module.exports = loadPackageModule(list, new URL('ui/model_sites_list/index.ui.js', base));
  const rendered = await list.__operit_render_compose_dsl({});
  const card = descendants(rendered.tree).find(node => node.props.key === 'doubao');
  const click = card.props.modifier.__modifierOps.find(op => op.name === 'clickable');
  const result = await list.__operit_dispatch_compose_dsl_action({ actionId: click.args[0].__actionId });
  assert.equal(result.navigationCommands[0].route, 'toolpkg:com.operit.sidebar_model_sites:ui:model_sites_viewer');
  const viewer = createRuntime(environment).context;
  viewer.module.exports = loadPackageModule(viewer, new URL('ui/model_sites_viewer/index.ui.js', base));
  const destination = await viewer.__operit_render_compose_dsl({});
  const webView = descendants(destination.tree).find(node => node.type === 'WebView');
  assert.equal(webView.props.url, 'https://www.doubao.com/chat/');
});

test('DSL environment reads use the refreshed call runtime', () => {
  const { context } = createRuntime();
  const runtime = context.OperitComposeDslRuntime.createContext({
    __operit_call_runtime: { getEnv: key => `old:${key}` },
  });
  assert.equal(runtime.ctx.getEnv('site'), 'old:site');
  runtime.setCallRuntime({ getEnv: key => `new:${key}` });
  assert.equal(runtime.ctx.getEnv('site'), 'new:site');
});

test('navigation preserves ordinary action results and no-render responses', async () => {
  const { context } = createRuntime();
  await context.__operit_render_compose_dsl({});
  const runtime = context.__operit_compose_bundle;
  const actionId = runtime.registerAction(async () => {
    await runtime.ctx.navigate('first', { step: 1 });
    await runtime.ctx.navigate('second', { step: 2 });
    return { accepted: true };
  });
  const result = await context.__operit_dispatch_compose_dsl_action({
    actionId, payload: { __no_render: true },
  });
  assert.equal(result.tree, undefined);
  assert.deepEqual(result.actionResult, { accepted: true });
  assert.deepEqual(JSON.parse(JSON.stringify(result.navigationCommands)), [
    { route: 'first', args: { step: 1 } },
    { route: 'second', args: { step: 2 } },
  ]);
});
