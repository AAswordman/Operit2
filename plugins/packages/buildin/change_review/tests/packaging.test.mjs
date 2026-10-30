import assert from 'node:assert/strict';
import vm from 'node:vm';
import path from 'node:path';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { unzipSync } from 'fflate';
import { buildPackage } from '../scripts/build.mjs';
import { readSdkScript } from '../../../../../tools/runtime/compose_scripts.mjs';
import { composeStreamFixture } from '../../../../../tools/tests/support/compose_stream_fixture.mjs';
import { message } from './runtime.mjs';

/** Loads only archive-owned modules, preserving the runtime's exported-function metadata. */
function runtime(files, registrationOnly) {
  const context = vm.createContext({ console,
    __operitCurrentCallId: 'change-review-registration',
    __operitGetCallState: () => ({ params: { toolPkgId: 'com.operit.change_review', __operit_toolpkg_api_version: '2.0.0' } }),
  });
  context.__operitExpose = (name, value) => { context[name] = value; };
  vm.runInContext(readSdkScript('ToolPkgApiRuntimeScript.rs'), context);
  vm.runInContext(readSdkScript('ToolPkgRegistrationBridge.rs').replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', 'true'), context);
  vm.runInContext(readSdkScript('ToolPkgComposeDslBridge.rs'), context);
  const cache = new Map();
  /** Resolves exact relative module references without filesystem access or executable substitutes. */
  function load(name) {
    if (registrationOnly && name.endsWith('.ui.js')) return context.OperitToolPkgExports.placeholder(name);
    if (cache.has(name)) return cache.get(name).exports;
    assert.ok(Object.hasOwn(files, name), `Missing installed module: ${name}`);
    const module = { exports: {} };
    cache.set(name, module);
    let source = new TextDecoder().decode(files[name]);
    if (name.endsWith('.ui.js')) source = context.OperitComposeCompiler.prepare(source, `com.operit.change_review:${name}`);
    const factory = vm.runInContext(`(function(module,exports,require) {\n${source}\n})`, context, { filename: name });
    factory(module, module.exports, request => {
      assert.ok(request.startsWith('.'));
      return load(path.posix.normalize(path.posix.join(path.posix.dirname(name), request)));
    });
    context.OperitToolPkgExports.tag(name, module.exports);
    return module.exports;
  }
  context.__operitGetActiveModuleExports = () => load('dist/main.js');
  return { context, load };
}

test('the actual built-in archive registers and executes all three independently owned surfaces', async () => {
  await buildPackage();
  const archive = await readFile(new URL('../dist/change_review.toolpkg', import.meta.url));
  const production = await readFile(new URL('../../../../../core/crates/runtime/application/assets/plugins/buildin/change_review.toolpkg', import.meta.url));
  assert.deepEqual(archive, production);
  const files = unzipSync(archive);
  const manifest = JSON.parse(new TextDecoder().decode(files['manifest.json']));
  assert.equal(manifest.enabled_by_default, true);
  assert.equal(manifest.toolpkg_id, 'com.operit.change_review');
  const registered = runtime(files, true);
  assert.equal(registered.load(manifest.main).registerToolPkg(), true);
  const capture = registered.context.__operitToolPkgRegistrationCapture;
  const routes = Array.from(capture.uiRoutes, value => JSON.parse(value));
  const entries = Array.from(capture.navigationEntries, value => JSON.parse(value));
  assert.deepEqual(entries.map(entry => entry.surface), ['chat_message_after', 'workspace', 'workspace_home']);
  assert.equal(new Set(routes.map(route => route.screen)).size, 3);
  for (const route of routes) {
    assert.ok(Object.hasOwn(files, route.screen));
    const installed = runtime(files, false);
    installed.context.module = { exports: installed.load(route.screen) };
    vm.runInContext(readSdkScript('ToolPkgComposeDslRuntimeScript.rs'), installed.context);
    const fixture = composeStreamFixture(installed.context);
    fixture.adapt();
    await installed.context.__operit_render_compose_dsl({ executionContextKey: `installed-${route.id}`, moduleSpec: route,
      theme: { brightness: 'light', colors: {} },
      state: { message: message(), input: { files: [{ path: 'a.ts', diffContent: '@@ -1 +1 @@\n-old\n+new', additions: 1, deletions: 1 }] } } });
    assert.ok(fixture.latest.tree.type);
    assert.equal(route.screenExport, 'default');
  }
});
