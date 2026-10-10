import { readFileSync } from 'node:fs';
import vm from 'node:vm';

/** Installs production host ownership/snapshot semantics and the shared SDK Promise helper. */
export function installScopedHostRuntime(sandbox) {
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
