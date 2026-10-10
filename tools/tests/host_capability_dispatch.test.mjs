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
  assert.doesNotMatch(bridge, /responseDelivery|Buffered|sendRemoteBuffered|recv_timeout|block_on/);
  assert.match(bridge, /streamable_http::sendJsonRpc[\s\S]*?\.await/);
  const transport = production(`${mcp}MCPStreamableHttp.rs`);
  assert.match(transport, /openHttpResponseStream/);
  assert.match(transport, /self\.events\.recv\(\)/);
  assert.match(transport, /waitForHostRuntimeDelay/);
  assert.doesNotMatch(transport, /recv_timeout|block_on|executeHttpRequest/);
  assert.doesNotMatch(production(`${mcp}MCPStreamableHttp.rs`), /Instant|target_arch/);
});

test('HTTP Hosts provide response streams instead of target-selected delivery modes', () => {
  for (const host of [
    'hosts/android/src/http.rs', 'hosts/ohos/src/http.rs',
    'hosts/apple/src/tools/http/mod.rs', 'hosts/linux/src/tools/http/mod.rs',
    'hosts/windows/src/tools/http/mod.rs',
  ]) {
    assert.match(source(host), /fn openHttpResponseStream[\s\S]*?self\.inner\s*\.openHttpResponseStream/, host);
  }
  assert.match(source('hosts/web/src/tools/http/mod.rs'), /fn openHttpResponseStream/);
  assert.doesNotMatch(source('core/crates/foundation/host-api/src/lib.rs'), /HttpResponseDelivery|responseDelivery/);
  assert.match(source('apps/flutter/app/web/runtime/src/operit_runtime_bridge.ts'), /\.\.\.httpStreamHost/);
  assert.match(source('apps/flutter/app/hook/build.dart'), /'browser_http_stream\.js'/);
});

test('mount persistence and permission checks never select a global storage Host', () => {
  const mounts = source('core/crates/tool/services/src/files/MountRegistry.rs');
  assert.doesNotMatch(mounts, /defaultRuntimeStorageHost/);
  const handler = source('core/crates/tool/services/src/tools/AIToolHandler.rs');
  const start = handler.indexOf('fn checkWorkspaceWritePath(');
  const end = handler.indexOf('\n    fn ', start + 1);
  const check = handler.slice(start, end === -1 ? undefined : end);
  assert.match(check, /PathMapper::new\(runtimeRoot, workspaceRoot, storage\.clone\(\)\)/);
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

test('persistent path mappers require their storage Host at construction', () => {
  const mapper = production('core/crates/tool/services/src/files/PathMapper.rs');
  assert.match(mapper, /pub fn new\([\s\S]*?storage: Arc<dyn RuntimeStorageHost>/);
  assert.doesNotMatch(mapper, /pub fn withMount(?:Storage|Registry)|impl Default for PathMapper/);
  assert.match(mapper, /pub fn builtinOnly\(/);
  for (const path of [
    'core/crates/command/core/src/commands/workspace.rs',
    'core/crates/tool/services/src/tools/AIToolHandler.rs',
    'core/crates/tool/services/src/tools/defaultTool/standard/StandardFileSystemTools.rs',
    'core/crates/provider/services/src/chat/EnhancedAIService.rs',
    'core/crates/runtime/application/src/core/application/OperitApplication.rs',
    'core/crates/runtime/application/src/services/WorkspaceService.rs',
    'core/crates/runtime/application/src/services/ChatServiceCore.rs',
    'core/crates/runtime/application/src/services/RuntimeTerminalService.rs',
    'core/crates/runtime/application/src/ui/features/chat/webview/workspace/WorkspaceBackupManager.rs',
  ]) {
    assert.doesNotMatch(production(path), /withMountStorage|PathMapper::builtinOnly/, path);
  }
});


test('MCP tools register the async executor and metadata never connects synchronously', () => {
  const executor = production('core/crates/tool/services/src/tools/mcp/MCPToolExecutor.rs');
  assert.match(executor, /impl AsyncToolExecutor for MCPToolExecutor/);
  assert.match(executor, /callToolWithArguments[\s\S]*?\.await/);
  const client = production(`${mcp}MCPBridgeClient.rs`);
  const metadata = client.slice(client.indexOf('pub fn getTools('), client.indexOf('pub fn getServiceInfo('));
  assert.doesNotMatch(metadata, /connect\(|spawn|block_on/);
  assert.match(production(`${mcp}MCPBridge.rs`), /active\s*\.get\(serviceName\)\s*\.cloned\(\)/);
});

test('extension and CLI command futures run on the owning Host rather than nested runtimes', () => {
  const extensions = production('core/crates/runtime/application/src/services/ExtensionRuntimeService.rs');
  assert.match(extensions, /scheduleHostRuntimeAsyncTask/);
  assert.match(extensions, /async fn refresh/);
  assert.match(extensions, /self\.refresh\(changes\)\.await/);
  const command = production('core/crates/command/core/src/commands/mcp.rs');
  assert.match(command, /generate_mcp_description\(application, args, output\)\.await/);
  assert.doesNotMatch(command, /block_in_place|block_on/);
  const cli = production('apps/cli/src/bootstrap.rs');
  assert.match(cli, /commandContext\.hostRuntimeTaskSchedulerHost/);
  assert.match(cli, /scheduleHostRuntimeAsyncTask\("cli-core-command"/);
  assert.match(cli, /sender\.closed\(\)/);
});
