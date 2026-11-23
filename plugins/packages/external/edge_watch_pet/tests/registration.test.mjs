import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const ts = require('../../../../../.ci-tools/typescript/node_modules/typescript');

test('watch studio registration retains a callable exported screen after module reload', () => {
  const registry = {};
  const context = {
    ToolPkg: {
      ipc: { on() {} },
      registerChatRuntimeHook() {},
      registerToolPermissionHook() {},
      registerAppLifecycleHook() {},
      registerUiRoute(route) {
        registry.screen = context.OperitToolPkgExports.reference(route.screen, 'ToolPkg.registerUiRoute screen');
        registry.route = route.route;
      },
      registerNavigationEntry(entry) { registry.navigation = entry; },
    },
  };
  // Use the host's actual export validation, including its rejection of local functions.
  runInNewContext(readFileSync(new URL('../../../../../core/crates/plugin/sdk/src/toolpkg/ModuleExports.js', import.meta.url), 'utf8'), context);
  const load = () => {
    const module = { exports: {} };
    const source = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
    runInNewContext(ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }).outputText, {
      ...context, module, exports: module.exports, require: () => ({}),
    });
    context.OperitToolPkgExports.tag('dist/main.js', module.exports);
    return module.exports;
  };
  assert.equal(load().registerToolPkg(), true);
  assert.equal(registry.screen.path, 'dist/main.js');
  assert.equal(registry.screen.name, 'studio');
  assert.equal(registry.navigation.route, registry.route);
  // A new realm must resolve the recorded screen instead of depending on the capture closure.
  const screen = context.OperitToolPkgExports.require(load(), registry.screen.name);
  const node = screen({
    createWebViewController: () => ({}),
    useState: (_, initial) => [initial, () => {}],
    useEffect() {},
    UI: { Box: (props, child) => ({ props, child }), Text: props => props },
  });
  assert.equal(node.props.fillMaxSize, true);
  assert.equal(typeof node.props.onLoad, 'function');
  assert.equal(node.child.text, '正在打开手表桌宠…');
});

test('offline studio resolves each original animation and blocks unrelated URLs', async () => {
  const module = { exports: {} }, state = new Map(), resources = [], interfaces = {};
  const code = ts.transpileModule(readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText;
  runInNewContext(code, {
    module, exports: module.exports, require: () => ({}),
    ToolPkg: { ipc: { on() {} }, async readResource(key) { resources.push(key); return `vfs:/watch/${key}`; } },
  });
  const ctx = {
    createWebViewController: () => ({ addJavascriptInterface(name, handler) { interfaces[name] = handler; } }),
    useState(key, initial) { return [state.has(key) ? state.get(key) : initial, value => state.set(key, value)]; },
    useEffect() {},
    UI: { Box: (props, child) => ({ props, child }), Text: props => props, WebView: props => props },
  };
  await module.exports.studio(ctx).props.onLoad();
  assert(interfaces.WatchStudio);
  assert.deepEqual(resources, ['working', 'thinking', 'approval', 'completed', 'idle', 'error', 'watch_studio']);
  const web = module.exports.studio(ctx).child;
  for (const mood of resources.slice(0, 6)) {
    const response = web.onInterceptRequest({ url: `https://watch-pet.operit.local/${mood}.webp` });
    assert.equal(response.action, 'respond'); assert.equal(response.response.mimeType, 'image/webp');
    assert.equal(response.response.filePath, `vfs:/watch/${mood}`);
  }
  assert.equal(web.onInterceptRequest({ url: 'https://example.com/idle.webp' }).action, 'block');
});
