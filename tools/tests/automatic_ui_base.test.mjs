import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import test from 'node:test';
import vm from 'node:vm';

const require = createRequire(new URL('../../plugins/packages/buildin/workflow/package.json', import.meta.url));
const ts = require('typescript');
const source = readFileSync(new URL('../../plugins/packages/buildin/automatic_ui_base.ts', import.meta.url), 'utf8');
const metadata = JSON.parse(source.split('/* METADATA')[1].split('*/')[0]);
const compiled = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS },
}).outputText;

/** Loads the migrated package with recorded system calls and real completion behavior. */
function runtime(tools = {}, call = async () => { throw new Error('Unexpected tool call'); }) {
  const results = [];
  const context = vm.createContext({ exports: {}, Tools: tools, toolCall: call, complete: result => results.push(result) });
  vm.runInContext(compiled, context);
  return { tools: context.exports, results };
}

/** Converts values crossing the VM boundary into ordinary test objects. */
function plain(value) {
  return JSON.parse(JSON.stringify(value));
}

test('legacy package stays disabled and every declared tool has a display icon and localized name', () => {
  assert.equal(metadata.name, 'Automatic_ui_base');
  assert.equal(metadata.enabledByDefault, false);
  assert.equal(metadata.tools.length, 13);
  const { tools } = runtime();
  for (const tool of metadata.tools) {
    assert.ok(tool.display.label.zh.trim(), tool.name);
    assert.ok(tool.display.label.en.trim(), tool.name);
    assert.ok(tool.display.icon.trim(), tool.name);
    if (!tool.advice) assert.equal(typeof tools[tool.name], 'function', tool.name);
  }
  assert.equal(metadata.tools.find(tool => tool.name === 'get_page_info').parameters[0].name, 'window_id');
});

test('window discovery and focus preserve identities while model results contain only the compact summary', async () => {
  const calls = [];
  const page = {windowId: 'native:42', scope: 'desktop', truncated: false, summary: 'Button "Save" id="save" [0,0][10,10]', uiElements: {children: Array.from({length: 4501}, (_, index) => ({text: `node-${index}`}))}};
  const {tools, results} = runtime({UI: {
    async listWindows() { calls.push(['list']); return {windows: [{windowId: 'native:42'}]}; },
    async focusWindow(id) { calls.push(['focus', id]); return {}; },
    async getPageInfo(id) { calls.push(['tree', id]); return page; },
  }});
  await tools.list_windows({});
  await tools.focus_window({window_id: 'native:42'});
  await tools.get_page_info({window_id: 'native:42'});
  assert.deepEqual(calls, [['list'], ['focus', 'native:42'], ['tree', 'native:42']]);
  assert.equal(page.uiElements.children.at(-1).text, 'node-4500');
  assert.equal(results[2].data.uiElements, undefined);
  assert.equal(results[2].data.summary, page.summary);
  assert.equal(results[2].data.windowId, 'native:42');
});

test('swipe duration, input clearing and element criteria reach the v2 UI provider', async () => {
  const calls = [];
  const { tools, results } = runtime({ UI: {
    async swipe(...args) { calls.push(['swipe', ...args]); return {}; },
    async setText(...args) { calls.push(['setText', ...args]); return {}; },
    async clickElement(params) { calls.push(['clickElement', params]); return {}; },
  } });
  await tools.swipe({ start_x: -20, start_y: 10, end_x: 100, end_y: 200, duration: 765 });
  await tools.set_input_text({ text: '', resource_id: 'editor' });
  await tools.click_element({ text: 'Save', contentDesc: 'Save file', partialMatch: true, index: 1 });
  assert.deepEqual(plain(calls), [
    ['swipe', -20, 10, 100, 200, 765], ['setText', '', 'editor'],
    ['clickElement', { text: 'Save', contentDesc: 'Save file', partialMatch: true, index: 1 }],
  ]);
  assert.ok(results.every(result => result.success));
});

test('screenshots use the system capture tool and register its VFS image', async () => {
  const calls = [];
  const path = '/operitapp/data/temp/clean_on_exit/screenshots/test.png';
  const { tools, results } = runtime({ Media: {
    async registerFromPath(params) { calls.push(plain(params)); return { link: 'image:test' }; },
  } }, async (name, params) => {
    calls.push([name, plain(params)]);
    return path;
  });
  await tools.get_page_screenshot_image({});
  assert.deepEqual(calls, [['capture_screenshot', {}], { type: 'image', path }]);
  assert.deepEqual(plain(results[0].data), { file_path: path, image_link: 'image:test' });
});

test('app launch reports failure without listing or launching other apps', async () => {
  const calls = [];
  const operation = { success: false, details: 'Application is unavailable' };
  const { tools, results } = runtime({ System: {
    async startApp(name) { calls.push(name); return operation; },
    async listApps() { assert.fail('Unexpected installed-app discovery'); },
  } });
  await tools.app_launch({ package_name: 'missing.app' });
  assert.deepEqual(calls, ['missing.app']);
  assert.deepEqual(plain(results[0]), { success: false, message: operation.details, data: { operation } });
});

test('provider failures remain errors and package main performs no input', async () => {
  const { tools, results } = runtime({ UI: {
    async getPageInfo() { throw new Error('Whole-device automation is unsupported'); },
  } });
  await tools.get_page_info({});
  assert.equal(results[0].success, false);
  assert.match(results[0].message, /Whole-device automation is unsupported/);
  await tools.main();
  assert.equal(results[1].success, true);
});
