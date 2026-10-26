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

/** Requires bounded queues and one shared owner instead of per-watch scheduler tasks. */
test('connection streams share one bounded Host task', () => {
  const transport = source(bridge + 'BridgeTransport.rs');
  assert.match(transport, /FuturesUnordered/);
  assert.equal((transport.match(/scheduleHostRuntimeAsyncTask\(/g) ?? []).length, 1);
  for (const code of ['PUSH_ALREADY_EXISTS', 'WATCH_ALREADY_EXISTS', 'PUSH_SEQUENCE_MISMATCH', 'STREAM_BACKPRESSURE', 'PUSH_BACKPRESSURE']) {
    assert.ok(transport.includes(code), code);
  }
  assert.match(transport, /checked_add\(1\)/);
  assert.match(transport, /Arc::ptr_eq/);
  assert.match(transport, /entry\.tag == tag/);
  assert.match(transport, /cancelled while opening/);
  assert.doesNotMatch(transport, /spawn_local|thread::|WatchLease|PushItemTask/);
  assert.doesNotMatch(source(bridge + 'BridgeRuntime.rs'), /watchSubscriptions|pushStreams/);
  const execution = source(bridge + 'PlatformRuntimeExecution.rs');
  assert.equal((execution.match(/spawn_local\(/g) ?? []).length, 1);
  assert.match(execution, /BridgeStreams::start/);
  const ffi = source(bridge + 'FfiTransport.rs');
  assert.match(ffi, /recv_many/);
  assert.match(ffi, /sendBatch/);
  assert.match(ffi, /BridgeStreams::start/);
});

/** Requires ABI v3 for native Core transport and preserves lifecycle and browser entry points. */
test('native Core transport exclusively exposes ABI v3', () => {
  const exports = source(bridge + 'BridgeExports.rs');
  for (const name of ['create', 'create_with_storage_roots', 'destroy', 'emit_runtime_event', 'free_string']) {
    assert.match(exports, new RegExp(`fn operit_flutter_bridge_${name}\\(`));
  }
  assert.doesNotMatch(exports, /fn operit_flutter_bridge_(?:native_call|push_open|push_item|push_close|watch_snapshot|watch_stream|close_watch_stream|next_watch_channel_event|close_watch_channel|free_bytes)\(/);
  assert.doesNotMatch(exports, /OperitByteBuffer|bytes_to_buffer|native_core_panic_result/);
  assert.doesNotMatch(exports, /fn bridge_(?:native_call|push_open|push_item|watch_snapshot|watch_stream)\(/);

  const ffi = source(bridge + 'FfiTransport.rs');
  assert.match(ffi, /fn operit_flutter_bridge_ffi_connect\(/);
  assert.match(ffi, /"version": 3/);
  for (const name of ['bufferAllocate', 'bufferData', 'bufferSeal', 'bufferRetain', 'bufferRelease']) {
    assert.match(ffi, new RegExp(`"${name}":`));
  }
  assert.doesNotMatch(ffi, /defaultHostRuntimeTaskSchedulerHost/);
  assert.match(ffi, /bridge\s*\.runtimeTaskScheduler\(\)/);
  assert.match(ffi, /SharedBuffer::retain\(\*handles\.add\(index\)\)/);

  const dart = source('apps/flutter/app/lib/core/bridge/FfiCoreTransport.dart');
  const versionCheck = dart.indexOf("descriptor['version'] != 3");
  const sessionBinding = dart.indexOf("_address(descriptor, 'session')");
  assert.notEqual(versionCheck, -1);
  assert.ok(sessionBinding > versionCheck);
  assert.match(dart.slice(versionCheck, sessionBinding), /throw StateError\('Unsupported Core FFI or Dart native API version'\)/);
});

/** Rejects the removed native polling queue while retaining host lifecycle scheduling. */
test('native watches are delivered through FFI sessions without a polling queue', () => {
  const execution = source(bridge + 'PlatformRuntimeExecution.rs');
  assert.doesNotMatch(execution, /NativeWatchChannel|watchChannel|nextWatchChannelEvent|nextEvent/);
  const native = section(execution.replaceAll('\r\n', '\n'), '#[cfg(not(target_arch = "wasm32"))]\nimpl OperitFlutterBridge {', '#[cfg(target_arch = "wasm32")]\nimpl OperitFlutterBridge {');
  assert.doesNotMatch(native, /fn (?:call|pushItem|pushClose|watchSnapshot|watchStream)\(/);
  assert.match(native, /fn runHostRuntimeAsyncTask</);
  assert.match(native, /fn emitRuntimeEvent\(/);

  const exports = source(bridge + 'BridgeExports.rs');
  assert.match(exports, /pub struct OperitFlutterBridgeWasm/);
  const browser = exports.slice(exports.indexOf('impl OperitFlutterBridgeWasm {'));
  for (const name of ['call', 'pushOpen', 'pushItem', 'pushClose', 'watchSnapshot', 'watchStream', 'closeWatchStream']) {
    assert.match(browser, new RegExp(`pub (?:async )?fn ${name}\\(`));
  }
});
