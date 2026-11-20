import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';

test('watch studio registration retains a callable exported screen after module reload', () => {
  const registry = {};
  const context = {
    ToolPkg: {
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
    runInNewContext(readFileSync(new URL('../dist/main.js', import.meta.url), 'utf8'), {
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
