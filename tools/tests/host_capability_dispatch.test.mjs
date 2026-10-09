import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const source = path => readFileSync(new URL(path, root), 'utf8');
const production = path => source(path).split('#[cfg(test)]')[0];
const mcp = 'core/crates/tool/services/src/tools/mcp_runtime/plugins/';

test('MCP clock and transport selection use capabilities instead of wasm gates', () => {
  const bridge = production(`${mcp}MCPBridge.rs`);
  assert.doesNotMatch(bridge, /target_arch|TimeUtils|Instant|defaultHostRuntimeTaskSchedulerHost/);
  assert.match(bridge, /context\.hostRuntimeTaskSchedulerHost/);
  assert.match(bridge, /self\.scheduler\s*\.monotonicTimeMillis\(\)/);
  assert.match(bridge, /match session\.httpHost\.responseDelivery\(\)/);
  assert.doesNotMatch(production(`${mcp}MCPStreamableHttp.rs`), /Instant|target_arch/);
});

test('native HTTP wrappers forward delivery capability to their providers', () => {
  for (const host of [
    'hosts/android/src/http.rs', 'hosts/ohos/src/http.rs',
    'hosts/apple/src/tools/http/mod.rs', 'hosts/linux/src/tools/http/mod.rs',
    'hosts/windows/src/tools/http/mod.rs',
  ]) {
    assert.match(source(host), /fn responseDelivery\(&self\)[\s\S]*?self\.inner\.responseDelivery\(\)/, host);
  }
  assert.match(source('hosts/web/src/tools/http/mod.rs'),
    /fn responseDelivery\(&self\)[\s\S]*?HttpResponseDelivery::Buffered/);
});

test('mount persistence and permission checks never select a global storage Host', () => {
  const mounts = source('core/crates/tool/services/src/files/MountRegistry.rs');
  assert.doesNotMatch(mounts, /defaultRuntimeStorageHost/);
  const handler = source('core/crates/tool/services/src/tools/AIToolHandler.rs');
  const start = handler.indexOf('fn checkWorkspaceWritePath(');
  const end = handler.indexOf('\n    fn ', start + 1);
  const check = handler.slice(start, end === -1 ? undefined : end);
  assert.match(check, /withMountStorage\(storage\.clone\(\)\)/);
  assert.doesNotMatch(check, /RuntimeStorePaths::default\(\)/);
});

test('peer watch-close scheduling retains the channel Host instead of using Tokio directly', () => {
  // This file has earlier crypto-only tests, so inspect the actual callback separately.
  const full = source('core/crates/node/runtime/src/peer/crypto.rs');
  const start = full.indexOf('fn watchCloseCallback(');
  const end = full.indexOf('async fn closeWatch(');
  assert.ok(start >= 0 && end > start);
  const callback = full.slice(start, end);
  assert.doesNotMatch(callback, /tokio::spawn|target_arch|defaultHost/);
  assert.match(callback, /self\.scheduler\.clone\(\)/);
  assert.match(callback, /scheduleHostRuntimeAsyncTask/);
});
