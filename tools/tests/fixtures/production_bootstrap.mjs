import { readSdkScript, composeContextScript, composeManifest, composeSource, wrapComposeScript } from "../../runtime/compose_scripts.mjs";
import assert from 'node:assert/strict';
import { generatedToolsSource } from './scoped_host_runtime.mjs';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import vm from 'node:vm';
const { transformSync } = createRequire(new URL('../../../plugins/packages/buildin/workflow/package.json', import.meta.url))('esbuild');
const root=new URL('../../../',import.meta.url);
const bridge='core/crates/plugin/javascript-bridge/src/javascript/';
const sdk='core/crates/plugin/sdk/src/';
function source(path) {return readFileSync(new URL(path,root),'utf8');}
function embedded(path) {if (path.endsWith('/ToolPkgRegistrationBridge.rs')) return readSdkScript('ToolPkgRegistrationBridge.rs'); const text=source(path),start=text.indexOf('r#"')+3;return text.slice(start,text.indexOf('"#',start));}

/** Models the async compilation binding for Node transport tests; real Oxc behavior is tested in JsEngine. */
export function prepareTestJavaScript(source) {
  return transformSync(source, { target: 'es2016', loader: 'js', charset: 'utf8' }).code;
}

/** Installs the common compilation contract without duplicating constructors in each transport fixture. */
export function installTestJavaScriptCompiler(context) {
  context.__operitNativePrepareJavaScript = prepareTestJavaScript;
  vm.runInContext("globalThis.__operitNativeCompileModuleFactory = body => new Function('module','exports','require','__operit_call_runtime',body);", context);
}

/** Builds lazy descriptors from the production Rust registry. */
export function lazyBootstrapLibraries() {
  const text = source(bridge + 'JsLibraries.rs');
  const descriptors = [...text.matchAll(/JsLazyLibrary\s*\{\s*file_name:\s*"([^"]+)",\s*globals:\s*&\[([^\]]+)\]/g)]
    .map(([, name, exports]) => [name, [...exports.matchAll(/"([^"]+)"/g)].map(([, key]) => key)]);
  assert.equal(descriptors.length, 6, 'Expected all trusted lazy libraries');
  return source(bridge + 'JsLazyLibraries.script.js') + '\n__operitInstallBootstrapLibraries(' + JSON.stringify(descriptors) + ');';
}

/** Returns the exact trusted source used by a production library host binding. */
export function bootstrapLibrarySource(name) {
  const match = /^assets\/js\/(CryptoJS|Jimp|UINode|AndroidUtils|OkHttp3|pako)\.js$/.exec(name);
  assert.ok(match, 'Unknown bootstrap library: ' + name);
  return source(bridge + match[1] + '.script.js');
}

/** Expands the actual production bootstrap with its SDK bridge and trusted source channel. */
export function productionBootstrap() {
  const text = source(bridge + 'JsLibraries.rs');
  const begin = text.indexOf('pub fn buildRuntimeBootstrapScript()');
  const start = text.indexOf('r#"', begin) + 3;
  const end = text.indexOf('"#,', start);
  const replacements = [
    source(bridge + 'JsInitRuntime.script.js'),
    '"/runtime/clean-on-exit"',
    JSON.stringify(embedded(sdk + 'JsExecutionScriptBuilder.rs')),
    JSON.stringify(wrapComposeScript()),
    embedded(bridge + 'JsJavaBridge.rs'),
    composeManifest.context.map(composeSource).join('\n'),
    embedded(sdk + 'toolpkg/ToolPkgApiRuntimeScript.rs'),
    source(bridge + 'JsToolPkgIpc.script.js'),
    embedded(sdk + 'toolpkg/ToolPkgRegistrationBridge.rs').replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', 'false'),
    generatedToolsSource(), '', // Keep generated Tools in the complete bootstrap; omit utility libraries.
    source(bridge + 'PluginConfig.script.js'),
    source(bridge + 'RuntimeContext.script.js'),
    lazyBootstrapLibraries(),
    source(sdk + 'js_sdk/toolpkg_route_runtime.js') + '\n' +
      source(sdk + 'JsExecutionRuntimeBridge.script.js') + '\n__operitInstallV1Runtime(globalThis);',
  ];
  let position = 0;
  const result = text.slice(start, end).replace(/\{\{|\}\}|\{\}/g, token => {
    if (token === '{{') return '{';
    if (token === '}}') return '}';
    assert.ok(position < replacements.length, 'Unexpected bootstrap placeholder');
    return replacements[position++];
  });
  assert.equal(position, replacements.length);
  return prepareTestJavaScript(result);
}
