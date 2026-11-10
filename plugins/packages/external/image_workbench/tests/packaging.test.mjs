import assert from 'node:assert/strict';
import test from 'node:test';
import vm from 'node:vm';
import path from 'node:path';
import { readFile, access } from 'node:fs/promises';
import { unzipSync } from 'fflate';
import { buildPackage } from '../scripts/build.mjs';
import { readSdkScript } from '../../../../../tools/runtime/compose_scripts.mjs';
import { composeStreamFixture } from '../../../../../tools/tests/support/compose_stream_fixture.mjs';

/** Loads the installed archive with the same module ownership tags used by the production SDK. */
function installed(files, registrationOnly) {
  const context = vm.createContext({ console, Tools: { SoftwareSettings: { readEnvironmentVariable: async () => ({ exists: false, value: null }) } },
    __operitCurrentCallId: 'workbench-registration', __operitGetCallState: () => ({ params: { toolPkgId: 'com.operit.image_workbench', __operit_toolpkg_api_version: '2.0.0' } }) });
  context.__operitExpose = (name, value) => { context[name] = value; };
  vm.runInContext(readSdkScript('ToolPkgApiRuntimeScript.rs'), context);
  vm.runInContext(readSdkScript('ToolPkgRegistrationBridge.rs').replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', 'true'), context);
  vm.runInContext(readSdkScript('ToolPkgComposeDslBridge.rs'), context);
  const cache = new Map();
  /** Resolves only archive-relative modules, retaining each exported function's source identity. */
  function load(name) {
    if (registrationOnly && name.endsWith('.ui.js')) return context.OperitToolPkgExports.placeholder(name);
    if (cache.has(name)) return cache.get(name).exports;
    assert.ok(Object.hasOwn(files, name), `Missing packaged module: ${name}`);
    const module = { exports: {} }; cache.set(name, module);
    let source = new TextDecoder().decode(files[name]);
    if (name.endsWith('.ui.js')) source = context.OperitComposeCompiler.prepare(source, `com.operit.image_workbench:${name}`);
    const factory = vm.runInContext(`(function(module,exports,require) {\n${source}\n})`, context);
    factory(module, module.exports, request => load(path.posix.normalize(path.posix.join(path.posix.dirname(name), request))));
    context.OperitToolPkgExports.tag(name, module.exports);
    return module.exports;
  }
  context.__operitGetActiveModuleExports = () => load('dist/main.js');
  return { context, load };
}

test('the real archive has one tool namespace, registers both entries and renders the installed page', async () => {
  await buildPackage();
  const archive = await readFile(new URL('../dist/image_workbench.toolpkg', import.meta.url));
  const assets = await readFile(new URL('../../../../../core/crates/runtime/application/assets/plugins/external/image_workbench.toolpkg', import.meta.url));
  assert.deepEqual(archive, assets);
  const files = unzipSync(archive), decoder = new TextDecoder();
  const manifest = JSON.parse(decoder.decode(files['manifest.json']));
  assert.equal(manifest.enabled_by_default, false);
  assert.deepEqual(manifest.subpackages, [{ id: 'image_workbench', entry: 'dist/tools.js' }]);
  const metadata = JSON.parse(decoder.decode(files['dist/tools.js']).match(/^\/\* METADATA\s*([\s\S]*?)\*\//)[1]);
  assert.deepEqual(metadata.tools.map(tool => tool.name), ['list_methods', 'generate_image', 'generate_video']);
  assert.ok(metadata.env.every(item => item.required === false));
  const registration = installed(files, true);
  assert.equal(registration.load(manifest.main).registerToolPkg(), true);
  const capture = registration.context.__operitToolPkgRegistrationCapture;
  const routes = Array.from(capture.uiRoutes, value => JSON.parse(value));
  assert.equal(routes.length, 1);
  assert.deepEqual(Array.from(capture.navigationEntries, value => JSON.parse(value).surface), ['main_sidebar_plugins', 'toolbox']);
  const runtime = installed(files, false), route = routes[0];
  runtime.context.module = { exports: runtime.load(route.screen) };
  vm.runInContext(readSdkScript('ToolPkgComposeDslRuntimeScript.rs'), runtime.context);
  const fixture = composeStreamFixture(runtime.context); fixture.adapt();
  await runtime.context.__operit_render_compose_dsl({ executionContextKey: 'installed-workbench', moduleSpec: route, theme: { brightness: 'light', colors: {} }, state: {} });
  assert.ok(fixture.latest.tree.type);
  assert.equal(fixture.errors.length, 0);
});

test('the six old standalone sources are removed from the bundled plugin catalog', async () => {
  for (const name of ['openai', 'qwen', 'zhipu', 'xai', 'minimax', 'nanobanana']) await assert.rejects(access(new URL(`../../../external/${name}_draw.ts`, import.meta.url)), { code: 'ENOENT' });
});
