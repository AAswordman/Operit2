import { readFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import vm from 'node:vm';

/** Installs production host ownership/snapshot semantics and the shared SDK Promise helper. */
export function installScopedHostRuntime(sandbox) {
  /** Receives context changes without introducing execution deadlines in this fixture. */
  sandbox.__operitNativeActivateCall = () => {};
  vm.runInContext(`
    globalThis.__operitCurrentCallId = 'owner';
    globalThis.refs = 0;
    globalThis.__operitRetainCallReference = function() { refs++; };
    globalThis.__operitReleaseCallReference = function() { refs--; };
    globalThis.__operitActivateCall = function(id) { __operitCurrentCallId = id; };
    globalThis.__operitExpose = function(name, value) { globalThis[name] = value; };
  `, sandbox);
  for (const path of [
    '../../../hosts/web/src/javascript_promises.js',
    '../../../core/crates/plugin/sdk/src/JsExecutionRuntimeBridge.script.js',
  ]) vm.runInContext(readFileSync(new URL(path, import.meta.url), 'utf8'), sandbox);
  return sandbox.__operitHostPromiseRegistry;
}

let generatedTools;

/** Generates the actual current Tools runtime instead of reimplementing generator templates in tests. */
export function generatedToolsSource() {
  if (generatedTools !== undefined) return generatedTools;
  const root = fileURLToPath(new URL('../../../', import.meta.url));
  const output = join(mkdtempSync(join(tmpdir(), 'operit-tools-contract-')), 'js_tools.js');
  execFileSync('cargo', ['run', '--quiet', '--manifest-path', 'core/Cargo.toml', '--package', 'operit-plugin-sdk-codegen', '--', 'generate-runtime', 'core/crates/plugin/sdk/src', output], { cwd: root, stdio: ['ignore', 'pipe', 'pipe'] });
  generatedTools = readFileSync(output, 'utf8');
  return generatedTools;
}

/** Installs official version gates and freshly generated Tools over the production structured transport. */
export function installGeneratedToolsRuntime(sandbox, version = '2.0.0') {
  sandbox.version = version;
  sandbox.__operitGetCallState = () => ({ params: { __operit_toolpkg_api_version: sandbox.version } });
  const api = readFileSync(new URL('../../../core/crates/plugin/sdk/src/toolpkg/ToolPkgApiRuntimeScript.rs', import.meta.url), 'utf8');
  const start = api.indexOf('r#"') + 3;
  vm.runInContext(api.slice(start, api.indexOf('"#', start)), sandbox);
  vm.runInContext(generatedToolsSource(), sandbox);
}

/** Builds explicit native tool result fields for tests supplying business data completions. */
export function domainToolResult(name, payload, request) {
  const fields = {
    models_list_summaries:'models', tts_list_configs:'configs',
    tts_get_current_config_id:'configId', tts_set_current_config_id:'configId',
    theme_list_configs:'configs', chat_read_extension:'extension',
    chat_write_extension:'extension', chat_delete_extension:'deleted',
  };
  if (fields[name]) return { [fields[name]]:payload };
  if (name === 'chat_observe') {
    const op = request.observation.op;
    if (op === 'open') return {op,handle:payload};
    if (op === 'next') return {op,event:payload};
    return {op};
  }
  if (name === 'storage_request') {
    const op = request.request.op;
    switch (op) {
      case 'open': return {op,...payload};
      case 'open_preferences': return {op:'open',...payload};
      case 'preferences_data': case 'preferences_finish_edit': return {op,snapshot:payload};
      case 'preferences_begin_edit': return {op,edit:payload};
      case 'preferences_cancel_edit': return {op};
      case 'close': return {op};
      case 'query': return {op,rows:payload};
      case 'execute': case 'transaction': return {op,results:payload};
      case 'define_table': return {op,...payload};
      case 'get': return {op,entry:payload};
      case 'get_many': case 'list': return {op,entries:payload};
      case 'commit': return {op,...payload};
      case 'changes': return {op,page:payload};
      default: throw new Error('Unexpected test Storage operation: ' + op);
    }
  }
  return payload;
}
