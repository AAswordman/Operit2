import { composeStreamFixture } from '../tests/support/compose_stream_fixture.mjs';
import assert from 'node:assert/strict';
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import vm from 'node:vm';
import { performance } from 'node:perf_hooks';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const root = new URL('../../', import.meta.url);
const output = new URL('target/dsl-performance/', root);
const require = createRequire(new URL('../../plugins/packages/buildin/workflow/package.json', import.meta.url));
const { buildSync } = require('esbuild');
mkdirSync(output, { recursive: true });
// Packaging TypeScript is a development step and is excluded from runtime measurements.
const bundle = buildSync({ entryPoints: [fileURLToPath(new URL('plugins/packages/buildin/character_cards/src/ui-selector.ts', root))],
  bundle: true, format: 'cjs', platform: 'neutral', target: 'es2020', write: false });
const screenFile = new URL('first-render-selector.cjs', output);
writeFileSync(screenFile, bundle.outputFiles[0].text + `
module.exports.default = function(ctx) { return selectorView(ctx, { mode: 'select', kind: 'all', selected: null, chatId: null }, data, controller); };`);
/** Extracts one exact production SDK JavaScript string. */
function sdkScript(file) {
  const text = readFileSync(new URL('core/crates/plugin/sdk/src/toolpkg/' + file, root), 'utf8');
  return text.slice(text.indexOf('r#"') + 3, text.lastIndexOf('"#'));
}
/** Summarizes independent measured samples without combining process clocks. */
function stats(values) {
  const sorted = values.slice().sort((a, b) => a - b);
  return { p50: +sorted[Math.ceil(sorted.length * .5) - 1].toFixed(3), p95: +sorted[Math.ceil(sorted.length * .95) - 1].toFixed(3) };
}
/** Measures cold JS contexts with explicitly synthetic production selector data. */
function sample(rows) {
  const data = { data: { selected: null, options: Array.from({ length: rows }, (_, index) => ({
    key: 'group:' + index, token: 'group:' + index, kind: 'group', id: String(index),
    title: 'Diagnostic group ' + index, description: 'Synthetic performance row',
    selection: { CharacterGroup: { id: String(index) } }, avatar: { type: 'group' },
  })) }, loading: false, switchingKey: null, error: '', finished: false };
  const controller = { load() {}, select() { throw Error('No user data access'); }, cancel() { throw Error('No presentation'); } };
  const times = {};
  let start = performance.now();
  const acorn = readFileSync(new URL('core/crates/plugin/sdk/src/toolpkg/vendor/acorn.js', root), 'utf8');
  const compiler = readFileSync(new URL('core/crates/plugin/sdk/src/toolpkg/ToolPkgComposeDslCompiler.js', root), 'utf8');
  const retained = readFileSync(new URL('core/crates/plugin/sdk/src/toolpkg/ToolPkgComposeDslRetained.js', root), 'utf8');
  const reactive = readFileSync(new URL('core/crates/plugin/sdk/src/toolpkg/ToolPkgComposeDslReactive.js', root), 'utf8');
  const bridge = sdkScript('ToolPkgComposeDslBridge.rs');
  const wrapper = sdkScript('ToolPkgComposeDslRuntimeScript.rs');
  const source = readFileSync(screenFile, 'utf8');
  times.readScriptsMs = performance.now() - start;
  start = performance.now();
  const context = vm.createContext({ module: { exports: {} }, console, data, controller, __probeNow: () => performance.now() });
  context.exports = context.module.exports;
  vm.runInContext(acorn, context);
  context.__operitAcorn = context.module.exports;
  context.module = { exports: {} };
  for (const script of [compiler, retained, reactive, bridge]) vm.runInContext(script, context);
  times.sdkInitMs = performance.now() - start;
  start = performance.now();
  const compiled = context.OperitComposeCompiler.compile(source, 'first-render-probe');
  vm.runInContext(wrapper.replaceAll('{{', '{').replaceAll('}}', '}').replace('{script}', compiled), context);
  times.entryCompileEvalMs = performance.now() - start;
  vm.runInContext(`
    const originalCreate = OperitComposeDslRuntime.createContext;
    /** Times only production retained commit without inspecting the resulting graph. */
    OperitComposeDslRuntime.createContext = function(options) {
      const bundle = originalCreate(options);
      const originalCommit = bundle.composition.commit;
      bundle.composition.commit = function(tree) {
        const start = __probeNow();
        const result = originalCommit(tree);
        globalThis.__probeCommitMs = __probeNow() - start;
        return result;
      };
      return bundle;
    };
  `, context);
  start = performance.now();
  composeStreamFixture(context, { projectTree: false, inspectStorage: false }).adapt();
  const response = context.__operit_render_compose_dsl({});
  times.firstRenderCommitMs = performance.now() - start;
  times.retainedCommitMs = context.__probeCommitMs;
  times.contextAndNodeGenerationMs = times.firstRenderCommitMs - times.retainedCommitMs;
  assert.equal(Object.hasOwn(response, 'state'), false);
  assert.equal(Object.hasOwn(response, 'memo'), false);
  assert.equal(response.update.reset, true);
  times.totalJsMs = times.readScriptsMs + times.sdkInitMs + times.entryCompileEvalMs + times.firstRenderCommitMs;
  return { times, response };
}
mkdirSync(output, { recursive: true });
const results = [];
for (const rows of [20, 100, 500]) {
  const first = sample(rows);
  for (let i = 0; i < 3; i++) sample(rows);
  const samples = Array.from({ length: 15 }, () => sample(rows));
  const summary = { rows, nodes: first.response.update.upserts.length, firstSampleMs: +first.times.totalJsMs.toFixed(3) };
  for (const key of Object.keys(first.times)) summary[key] = stats(samples.map(item => item.times[key]));
  results.push(summary);
  // JSON is only the offline test fixture format, never the measured production transport.
  writeFileSync(new URL(`first-render-${rows}.json`, output), JSON.stringify(first.response.update));
}
const report = { measuredAt: new Date().toISOString(), engine: process.version + ' Node/V8', samples: 15,
  scope: 'Current bundled selector module + production SDK in fresh Node VM contexts; synthetic loaded data; no QuickJS, CoreLink, storage queries or Flutter; repeated filesystem reads use OS cache', results };
writeFileSync(new URL('first-render-js.json', output), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
