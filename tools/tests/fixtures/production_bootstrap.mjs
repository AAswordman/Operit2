import { composeContextScript, composeManifest, composeSource, wrapComposeScript } from "../../runtime/compose_scripts.mjs";
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const root=new URL('../../../',import.meta.url);
const bridge='core/crates/plugin/javascript-bridge/src/javascript/';
const sdk='core/crates/plugin/sdk/src/';
function source(path) {return readFileSync(new URL(path,root),'utf8');}
function embedded(path) {const text=source(path),start=text.indexOf('r#"')+3;return text.slice(start,text.indexOf('"#',start));}

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
    embedded(sdk + 'toolpkg/ToolPkgRegistrationBridge.rs').replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', 'false'),
    '', '', // These tests exercise host operations, not generated Tools or external libraries.
    source(bridge + 'PluginConfig.script.js'),
    source(bridge + 'RuntimeContext.script.js'),
    lazyBootstrapLibraries(),
    source(sdk + 'JsExecutionRuntimeBridge.script.js'),
  ];
  let position = 0;
  const result = text.slice(start, end).replace(/\{\{|\}\}|\{\}/g, token => {
    if (token === '{{') return '{';
    if (token === '}}') return '}';
    assert.ok(position < replacements.length, 'Unexpected bootstrap placeholder');
    return replacements[position++];
  });
  assert.equal(position, replacements.length);
  return result;
}
