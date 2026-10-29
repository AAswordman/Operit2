import { readFileSync } from 'node:fs';

/** The same SDK source manifest consumed by Rust's build.rs. */
export const composeRoot = new URL('../../core/crates/plugin/sdk/src/toolpkg/compose/', import.meta.url);
export const composeManifest = JSON.parse(readFileSync(new URL('sources.json', composeRoot), 'utf8'));

export function composeSource(name) {
  return readFileSync(new URL(name, composeRoot), 'utf8');
}
export function composeContextScript() {
  return composeParserScript() + '\n' + composeManifest.context.map(composeSource).join('\n') + '\n' + composeSource(composeManifest.compiler);
}
export function composeExecutionScript() {
  return '(function() {\n' + composeManifest.execution.map(composeSource).join('\n') + '\n})();';
}
export function wrapComposeScript(script = '') {
  return script + '\n;' + composeExecutionScript();
}
export function composeParserScript() {
  return composeSource(composeManifest.parser).replace(
    '__OPERIT_COMPOSE_PARSER_SOURCE__',
    () => "function() { return " + JSON.stringify(composeSource(composeManifest.parserSource)) + "; }",
  );
}

/** Loads exact SDK script sources; Compose uses its build manifest instead of Rust-string extraction. */
export function readSdkScript(file, functionName = null) {
  if (file === 'ToolPkgComposeDslBridge.rs') return composeContextScript();
  if (file === 'ToolPkgComposeDslRuntimeScript.rs') return wrapComposeScript();
  const source = composeSource('../' + file);
  if (file.endsWith('.js')) return source;
  const offset = functionName === null ? 0 : source.indexOf('pub fn ' + functionName + '(');
  if (offset < 0) throw new Error('Unknown SDK script builder: ' + functionName);
  const start = source.indexOf('r#"', offset), end = source.indexOf('"#', start + 3);
  if (start < 0 || end < 0) throw new Error('SDK script has no literal source: ' + file);
  return source.slice(start + 3, end).replace('__OPERIT_TOOLPKG_MODULE_EXPORTS__', () => composeSource('../ModuleExports.js'))
    .replace('__OPERIT_TOOLPKG_WASM_API__', () => composeSource('../ToolPkgWasmApi.js'));
}
