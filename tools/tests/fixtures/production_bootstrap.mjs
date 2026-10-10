import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const root=new URL('../../../',import.meta.url);
const bridge='core/crates/plugin/javascript-bridge/src/javascript/';
const sdk='core/crates/plugin/sdk/src/';
function source(path) {return readFileSync(new URL(path,root),'utf8');}
function embedded(path) {const text=source(path),start=text.indexOf('r#"')+3;return text.slice(start,text.indexOf('"#',start));}

/** Expands the actual production bootstrap with its embedded libraries and SDK bridge. */
export function productionBootstrap() {
  const text = source(bridge + 'JsLibraries.rs');
  const begin = text.indexOf('pub fn buildRuntimeBootstrapScript()');
  const start = text.indexOf('r#"', begin) + 3;
  const end = text.indexOf('"#,', start);
  const replacements = [
    source(bridge + 'JsInitRuntime.script.js'),
    '"/runtime/clean-on-exit"',
    JSON.stringify(embedded(sdk + 'JsExecutionScriptBuilder.rs')),
    JSON.stringify(embedded(sdk + 'toolpkg/ToolPkgComposeDslRuntimeScript.rs').replace('{script}', '').replace(/\{\{|\}\}/g, token => token[0])),
    embedded(bridge + 'JsJavaBridge.rs'),
    'var __operitAcorn={}; (function(exports,module){' + source(sdk + 'toolpkg/vendor/acorn.js') + '})(__operitAcorn,{exports:__operitAcorn});\n' +
    ['ToolPkgComposeDslCompiler.js','ToolPkgComposeDslRetained.js','ToolPkgComposeDslReactive.js'].map(name=>source(sdk+'toolpkg/'+name)).join('\n') +
    embedded(sdk + 'toolpkg/ToolPkgComposeDslBridge.rs'),
    embedded(sdk + 'toolpkg/ToolPkgApiRuntimeScript.rs'),
    embedded(sdk + 'toolpkg/ToolPkgRegistrationBridge.rs').replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', 'false'),
    '', '', // These tests exercise host operations, not generated Tools or external libraries.
    source(bridge + 'PluginConfig.script.js'),
    source(bridge + 'RuntimeContext.script.js'),
    source(bridge + 'CryptoJS.script.js'),
    source(bridge + 'Jimp.script.js'),
    source(bridge + 'UINode.script.js'),
    source(bridge + 'AndroidUtils.script.js'),
    source(bridge + 'OkHttp3.script.js'),
    source(bridge + 'pako.script.js'),
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
