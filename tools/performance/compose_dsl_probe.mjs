import { composeStreamFixture } from '../tests/support/compose_stream_fixture.mjs';
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import vm from 'node:vm';
import { loadModule } from '../../plugins/packages/buildin/character_cards/tests/runtime.mjs';

const root = new URL('../../', import.meta.url);
const output = new URL('target/dsl-performance/', root);
const { selectorView } = loadModule('src/ui-selector.ts');

/** Reads exactly one production SDK script without building Rust or changing it. */
function sdkScript(file) {
  const source = readFileSync(new URL('core/crates/plugin/sdk/src/toolpkg/' + file, root), 'utf8');
  const start = source.indexOf('r#"');
  const end = source.lastIndexOf('"#');
  assert.ok(start >= 0 && end > start);
  return source.slice(start + 3, end);
}

/** Reports distribution summaries without mixing setup or warmup into measurements. */
function distribution(values) {
  const sorted = [...values].sort(
    /** Orders measured durations numerically. */
    (left, right) => left - right,
  );
  return { p50: sorted[Math.ceil(sorted.length * 0.50) - 1], p95: sorted[Math.ceil(sorted.length * 0.95) - 1] };
}

/** Counts every serialized node, including named content slots. */
function nodeCount(node) {
  let count = 1;
  for (const child of node.children) count += nodeCount(child);
  if (Object.hasOwn(node, 'slots')) {
    for (const children of Object.values(node.slots)) {
      for (const child of children) count += nodeCount(child);
    }
  }
  return count;
}

/** Generates explicitly synthetic selector rows; no user records or native IO are measured. */
function options(count) {
  return Array.from({ length: count },
    /** Supplies exact stable identities and the production group-avatar component. */
    (_, index) => ({ key: 'group:' + index, token: 'group:' + index, kind: 'group', id: String(index), title: 'Diagnostic group ' + index, description: 'Synthetic performance row', selection: { CharacterGroup: { id: String(index) } }, avatar: { type: 'group' } }),
  );
}

/** Mounts the production selector view on the unchanged production SDK runtime. */
function runtime(rowCount, stateBytes, asynchronous) {
  const renders = [];
  const wire = [];
  const payload = 'x'.repeat(stateBytes);
  const state = { data: { options: options(rowCount), selected: null }, loading: false, switchingKey: null, error: '', finished: false };
  const controller = {
    /** Rejects loading because the fixture explicitly owns its synthetic data. */
    load() { throw new Error('The performance fixture does not perform storage reads'); },
    /** Rejects cancellation because no genuine presentation request is open. */
    cancel() { throw new Error('The performance fixture cannot cancel a presentation'); },
    /** Rejects commits to prevent synthetic fixture rows from becoming real records. */
    select() { throw new Error('The performance fixture cannot modify a binding'); },
  };
  const context = vm.createContext({ module: { exports: {} }, console,
    __operit_call_runtime_ref: {
      /** Measures the actual complete response crossing the intermediate-result boundary. */
      sendComposeResponse(phase, value) { capture(value, 'intermediate'); },
    },
    /** Times the actual selector view and exposes one explicit synthetic state-edit action. */
    probeScreen(ctx) {
      const start = performance.now();
      const [value, setValue] = ctx.useState('probe-input', '');
      ctx.useState('probe-payload', payload);
      const tree = selectorView(ctx, { mode: 'select', kind: 'all', selected: null, chatId: null }, state, controller);
      tree.props.probeEdit = ctx.h('ProbeAction', {
        /** Applies one state edit using the unchanged SDK notification and action machinery. */
        onClick(next) {
          setValue(next);
          if (asynchronous) return Promise.resolve(value);
          return value;
        },
      }, []).props.onClick;
      renders.push(performance.now() - start);
      return tree;
    },
  });
  vm.runInContext(sdkScript('ToolPkgComposeDslBridge.rs'), context);
  const wrapper = sdkScript('ToolPkgComposeDslRuntimeScript.rs').replaceAll('{{', '{').replaceAll('}}', '}').replace('{script}', 'module.exports.default = probeScreen;');
  vm.runInContext(wrapper, context);
  composeStreamFixture(context, { inspectStorage: false, onResponse(phase, value) { if (phase === "intermediate") capture(value, phase); } }).adapt();
  let current = context.__operit_render_compose_dsl({});

  /** Captures separate inner-string and outer-event serialization costs. */
  function capture(value, phase) {
    const start = performance.now();
    const result = JSON.stringify(value);
    const innerMs = performance.now() - start;
    const envelopeStart = performance.now();
    const event = JSON.stringify({ phase, result });
    wire.push({ event, innerBytes: Buffer.byteLength(result), eventBytes: Buffer.byteLength(event), innerMs, envelopeMs: performance.now() - envelopeStart });
  }

  return {
    /** Samples a real SDK action while separating request encoding from action execution. */
    async sample(index) {
      const request = { actionId: current.tree.props.probeEdit.__actionId, payload: 'edit-' + index, state: current.state, memo: current.memo };
      const requestStart = performance.now();
      const requestBytes = Buffer.byteLength(JSON.stringify(request));
      const requestEncodeMs = performance.now() - requestStart;
      renders.length = 0;
      wire.length = 0;
      const actionStart = performance.now();
      current = await context.__operit_dispatch_compose_dsl_action(request);
      capture(current, 'final');
      const actionMs = performance.now() - actionStart;
      assert.equal(wire.length, 2, 'Each state edit currently produces intermediate and final full-tree responses');
      return { actionMs, requestEncodeMs, requestBytes, renders: renders.length, nodes: nodeCount(current.tree), events: wire.length,
        renderMs: renders.reduce(
          /** Adds both complete tree construction passes. */
          (sum, value) => sum + value, 0),
        serializeMs: wire.reduce(
          /** Adds inner serialization at both response boundaries. */
          (sum, value) => sum + value.innerMs, 0),
        envelopeMs: wire.reduce(
          /** Adds the second JSON layer wrapping both result strings. */
          (sum, value) => sum + value.envelopeMs, 0),
        eventBytes: wire.reduce(
          /** Counts total response traffic rather than only the final result. */
          (sum, value) => sum + value.eventBytes, 0),
        actionHandlers: Object.keys(context.__operit_compose_bundle.actionStore).length,
        finalEvent: wire[wire.length - 1].event,
      };
    },
  };
}

/** Produces reproducible evidence with explicit V8-only and synthetic-host limitations. */
async function main() {
  mkdirSync(output, { recursive: true });
  const scenarios = [
    { name: 'rows-20', rows: 20, stateBytes: 0, asynchronous: false },
    { name: 'rows-100', rows: 100, stateBytes: 0, asynchronous: false },
    { name: 'rows-500', rows: 500, stateBytes: 0, asynchronous: false },
    { name: 'state-1mib', rows: 20, stateBytes: 1024 * 1024, asynchronous: false },
    { name: 'state-4mib', rows: 20, stateBytes: 4 * 1024 * 1024, asynchronous: false },
    { name: 'async-rows-20', rows: 20, stateBytes: 0, asynchronous: true },
  ];
  const results = [];
  for (const scenario of scenarios) {
    const probe = runtime(scenario.rows, scenario.stateBytes, scenario.asynchronous);
    for (let index = 0; index < 8; index += 1) await probe.sample(index);
    const samples = [];
    for (let index = 0; index < 25; index += 1) samples.push(await probe.sample(index));
    const last = samples[samples.length - 1];
    const timing = {};
    for (const metric of ['actionMs', 'requestEncodeMs', 'renderMs', 'serializeMs', 'envelopeMs']) timing[metric] = distribution(samples.map(
      /** Selects one independent phase from each warmed sample. */
      sample => sample[metric],
    ));
    results.push({ ...scenario, timing, nodes: last.nodes, eventsPerAction: last.events, rendersPerAction: last.renders, requestBytes: last.requestBytes, responseBytes: last.eventBytes,
      actionHandlersAfter33Edits: last.actionHandlers });
    writeFileSync(new URL(scenario.name + '.event.json', output), last.finalEvent);
  }
  const report = { engine: process.version + ' V8 (NOT the native QuickJS host)', scope: 'Production SDK and production selector view; synthetic rows, no IPC, IO, Flutter layout, raster or native scheduler', warmup: 8, samples: 25, results };
  writeFileSync(new URL('js-probe.json', output), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));
  console.log('Report directory: ' + fileURLToPath(output));
}

main().catch(
  /** Preserves the original diagnostic failure instead of returning substitute measurements. */
  error => { console.error(error); process.exitCode = 1; },
);

