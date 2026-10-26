/** Measures real character editor actions without charging assertion-tree projection to the runtime. */
import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { performance } from 'node:perf_hooks';
import vm from 'node:vm';
import { buildUiScreenScripts } from '../../plugins/packages/buildin/character_cards/scripts/build.mjs';
import { fixture } from '../../plugins/packages/buildin/character_cards/tests/fixtures.mjs';
import { packageRuntime } from '../../plugins/packages/buildin/character_cards/tests/package-runtime.mjs';
import { composeExecutionScript } from '../runtime/compose_scripts.mjs';
import { composeStreamFixture } from '../tests/support/compose_stream_fixture.mjs';

const scripts = await buildUiScreenScripts();
const output = new URL('../../target/dsl-performance/character-editor/', import.meta.url);
mkdirSync(output, { recursive: true });
const reports = [];
for (const [length, tagCount] of [[100, 2], [10000, 2], [100000, 2], [100, 100], [100, 500]]) {
  const { snapshot } = fixture();
  snapshot.tags = Array.from({ length: tagCount }, (_, index) => ({ ...snapshot.tags[0], id: 'tag-' + index, name: '提示词标签' + index }));
  const scenario = length + '-' + tagCount;
  const card = { ...snapshot.cards[0], characterSetting: '角色设定内容。'.repeat(Math.ceil(length / 7)).slice(0, length) };
  const state = {
    snapshot, card, group: null, editing: true, tab: 0, previewConfigurationExpanded: false,
    loading: false, busy: false, error: '', fullscreen: false, deleting: false, finished: false,
    tags: snapshot.tags, tagChanges: { created: [], updated: [], deleted: [] }, tagDraft: null,
    tagDeleting: false, nextTag: 0, expanded: null, picker: null, tagManaging: false,
    toolDraft: null, toolTab: 0, toolQuery: '', exporting: false, exportContent: '', exportName: '',
    avatar: null, themes: [], themeError: '',
  };
  const runtime = packageRuntime(scripts);
  runtime.context.module = { exports: runtime.load('dist/ui/main/index.ui.js') };
  vm.runInContext(composeExecutionScript(), runtime.context);
  const stream = composeStreamFixture(runtime.context, { projectTree: false, inspectStorage: false });
  const frames = [];
  runtime.context.__operit_call_runtime_ref = {
    ...runtime.context.__operit_call_runtime_ref,
    /** Captures exact binary transactions without decoding them inside the timed boundary. */
    sendComposeResponse(phase, frame) { frames.push({ phase, frame }); },
  };
  const options = { packageName: 'com.operit.character_cards', executionContextKey: 'character-editor-perf' };
  /** Measures dispatch, then independently validates committed production transactions. */
  async function measure(name, operation) {
    frames.length = 0;
    const start = performance.now();
    assert.equal(await operation(), null);
    const ms = performance.now() - start;
    for (const { phase, frame } of frames) stream.accept(phase, frame);
    const update = stream.latest.update;
    const result = {
      name, length, tagCount, ms, frames: frames.length,
      bytes: frames.reduce((sum, { frame }) => sum + frame.byteLength, 0),
      changedInstances: update?.instances.length ?? 0,
      metrics: runtime.context.__operit_compose_bundle.composition.metrics(),
      reactive: runtime.context.OperitComposeReactive.metrics(runtime.context.__operit_compose_bundle.ctx),
    };
    writeFileSync(new URL(scenario + '-' + name + '.cpb', output), frames.at(-1).frame);
    return result;
  }
  const initial = await measure('initial', () => runtime.context.__operit_render_compose_dsl({ ...options, state: {
    presentation: { requestId: 'editor-perf', input: { mode: 'edit', entity: 'card', id: card.id } },
    'native-entity-state': state,
  } }));
  const fields = [...stream.nodes.values()].filter(node => node.props.key === 'entity-field-name');
  assert.equal(fields.length, 1);
  const actionId = fields[0].props.onValueChange.__actionId;
  const samples = [];
  for (let index = 0; index < 25; index++) {
    samples.push(await measure('input-' + index, () => runtime.context.__operit_dispatch_compose_dsl_action({
      ...options, actionId, payload: '编辑角色名称' + index,
    })));
    const field = [...stream.nodes.values()].find(node => node.props.key === 'entity-field-name');
    assert.equal(field.props.value, '编辑角色名称' + index);
  }
  const setting = [...stream.nodes.values()].find(node => node.props.key === 'entity-field-characterSetting');
  const contentSamples = [];
  for (let index = 0; index < 15; index++) {
    contentSamples.push(await measure('content-' + index, () => runtime.context.__operit_dispatch_compose_dsl_action({
      ...options, actionId: setting.props.onValueChange.__actionId, payload: card.characterSetting + 'a'.repeat(index + 1),
    })));
  }
  const contentSorted = contentSamples.slice(5).map(sample => sample.ms).sort((a, b) => a - b);
  const tabs = [];
  for (const tab of [1, 2, 0]) {
    const node = [...stream.nodes.values()].find(node => node.props.key === 'entity-tab-' + tab);
    tabs.push(await measure('tab-' + tab, () => runtime.context.__operit_dispatch_compose_dsl_action({ ...options, actionId: node.props.onClick.__actionId })));
  }
  const sorted = samples.slice(5).map(sample => sample.ms).sort((a, b) => a - b);
  const report = { length, tagCount, initial, tabs, content: { p50Ms: contentSorted[Math.ceil(contentSorted.length * .5) - 1], p95Ms: contentSorted[Math.ceil(contentSorted.length * .95) - 1], first: contentSamples[0] }, input: {
    p50Ms: sorted[Math.ceil(sorted.length * .5) - 1],
    p95Ms: sorted[Math.ceil(sorted.length * .95) - 1],
    first: samples[0], last: samples.at(-1),
  } };
  reports.push(report);
  console.log(JSON.stringify(report));
}
writeFileSync(new URL('javascript.json', output), JSON.stringify({
  scope: 'Node production Compose compiler/runtime and binary encoding; excludes CoreLink, QuickJS, Flutter and GPU',
  reports,
}, null, 2));



