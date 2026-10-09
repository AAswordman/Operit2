import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const bridge = 'apps/flutter/native/operit-flutter-bridge/src/';

/** Reads one production file for host boundary contract checks. */
function source(path) { return readFileSync(new URL(path, root), 'utf8'); }

/** Extracts one documented section without accepting missing markers. */
function section(content, start, end) {
  const begin = content.indexOf(start);
  assert.notEqual(begin, -1, `Missing section: ${start}`);
  const finish = content.indexOf(end, begin + start.length);
  assert.notEqual(finish, -1, `Missing section end: ${end}`);
  return content.slice(begin, finish);
}

/** Keeps shared modules free of platform selection and arbitrary string heuristics. */
test('shared bridge ownership, protocols and owner adapters are platform independent', () => {
  for (const name of ['lib.rs', 'BridgeRuntime.rs', 'BridgeTransport.rs', 'FlutterHostAdapters.rs', 'FlutterOwnerCapabilities.rs', 'BridgeCodec.rs']) {
    assert.doesNotMatch(source(bridge + name), /target_arch|target_os|target_env|wasm_bindgen|\.contains\(|\.starts_with\(/, name);
    assert.doesNotMatch(source(bridge + name).split('#[cfg(test)]')[0], /use (super|crate)::\*/, name);
  }
});

/** Requires a single lifecycle and explicit scheduler ownership instead of a second runtime. */
test('all platform constructors enter one Core lifecycle', () => {
  const factory = source(bridge + 'PlatformRuntimeFactory.rs');
  const runtime = source(bridge + 'BridgeRuntime.rs');
  assert.equal((runtime.match(/localApplicationMut\(\)\.onCreate\(/g) ?? []).length, 1);
  assert.equal((runtime.match(/CoreApplication::startWithSharedLocalClient/g) ?? []).length, 1);
  assert.doesNotMatch(factory + runtime, /tokio::runtime::Builder|defaultHostRuntimeTaskSchedulerHost/);
  assert.match(factory, /OperitFlutterBridge::start\(/);
  assert.match(runtime, /hostRuntimeTaskSchedulerHost[\s\S]*?ok_or_else/);
  assert.match(runtime, /coreApplication: Mutex::new\(Some\(coreApplication\)\)/);
});

/** Prevents platform media commands from being serialized and then parsed inside the bridge. */
test('owner media implementations use the host traits directly', () => {
  const owner = source(bridge + 'FlutterOwnerCapabilities.rs');
  for (const name of ['AudioPlaybackHost', 'TtsPlaybackHost', 'TtsSynthesisHost', 'LocalInferenceHost']) {
    assert.match(owner, new RegExp(`impl ${name} for Flutter`));
  }
  assert.doesNotMatch(owner, /NativeMusicCommand|command\.command\.as_str|fromPlayers|fromController/);
  assert.match(owner, /fn speakText\([\s\S]*?request\.text/);
});

/** Keeps platform selection in one readable selector and one file per target. */
test('platform construction has one explicit selection boundary', () => {
  const factory = source(bridge + 'PlatformRuntimeFactory.rs');
  const selector = source(bridge + 'platform_runtime/mod.rs');
  assert.doesNotMatch(factory, /target_arch|target_os|target_env/);
  for (const name of ['android', 'ios', 'linux', 'macos', 'ohos', 'web', 'windows']) {
    const platform = source(`${bridge}platform_runtime/${name}.rs`);
    assert.match(platform, /fn create_host_context\(/, name);
    assert.match(platform, /fn startup_device_info\(/, name);
    assert.match(platform, /fn default_native_storage_roots\(/, name);
  }
  assert.match(selector, /use android as selected/);
  assert.match(selector, /use ios as selected/);
  assert.match(selector, /use ohos as selected/);
  assert.match(selector, /use web as selected/);
  assert.match(source(bridge + 'PlatformRuntimeExecution.rs'), /only place where the ABI execution style is selected/);
});

/** Prevents duplicate push ids from replacing the live original session. */
test('shared push registration owns duplicate detection and sequence validation', () => {
  const transport = source(bridge + 'BridgeTransport.rs');
  const open = section(transport, 'pub(crate) fn pushOpen(', '/// Validates sequence ownership');
  assert.match(open, /Entry::Occupied[\s\S]*?PUSH_ALREADY_EXISTS/);
  assert.match(open, /Entry::Vacant[\s\S]*?openPushLocal[\s\S]*?entry\.insert/);
  assert.match(transport, /PUSH_SEQUENCE_MISMATCH/);
  assert.match(transport, /checked_add\(1\)/);
  assert.equal((transport.match(/\.send\(self\.item\.args\)/g) ?? []).length, 1);
});

/** Protects cancellation during source opening and reuse of a completed subscription id. */
test('watch registration generations prevent stale completion from cancelling new watches', () => {
  const transport = source(bridge + 'BridgeTransport.rs');
  assert.match(transport, /Arc::ptr_eq\(&entry\.generation, &self\.generation\)/);
  assert.match(transport, /impl Drop for WatchLease/);
  assert.match(transport, /WATCH_CLOSED[\s\S]*?cancelled while opening/);
  assert.match(transport, /opened = CoreLinkSharedClient::watch[\s\S]*?=> opened\?/);
  assert.match(transport, /subscription\.cancel\.send\(\(\)\)/);
  assert.equal((transport.match(/pub\(crate\) fn closeWatchStream/g) ?? []).length, 1);
});

/** Preserves the public transport exports while leaving shared Core dispatch unmodified by ABI choices. */
test('C and JavaScript transport entry points retain their public names', () => {
  const exports = source(bridge + 'BridgeExports.rs');
  for (const name of ['create', 'create_with_storage_roots', 'destroy', 'native_call', 'push_open', 'push_item', 'push_close', 'watch_snapshot', 'watch_stream', 'close_watch_stream']) {
    assert.match(exports, new RegExp(`fn operit_flutter_bridge_${name}\\(`));
  }
  assert.match(exports, /pub struct OperitFlutterBridgeWasm/);
  const ffi = source(bridge + 'FfiTransport.rs');
  assert.doesNotMatch(ffi, /defaultHostRuntimeTaskSchedulerHost/);
  assert.match(ffi, /bridge\s*\.runtimeTaskScheduler\(\)/);
});
