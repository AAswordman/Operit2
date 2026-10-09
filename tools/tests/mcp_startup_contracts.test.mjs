import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const base = 'core/crates/tool/services/src/tools/mcp_runtime/';

/** Reads production Rust code for source-level architecture regression checks. */
function source(path) {
  return readFileSync(new URL(path, root), 'utf8');
}

/** Extracts a function body delimited by the next known declaration. */
function section(text, start, end) {
  const first = text.indexOf(start);
  assert.notEqual(first, -1, `Missing declaration: ${start}`);
  const last = text.indexOf(end, first + start.length);
  assert.notEqual(last, -1, `Missing following declaration: ${end}`);
  return text.slice(first, last);
}

/** Guards against reintroducing the batch-wide timeout break. */
test('MCP startup attempts every server independently', () => {
  const starter = source(`${base}plugins/MCPStarter.rs`);
  const batch = section(starter, 'pub async fn startAllDeployedPluginsWithTimeout(', 'fn startPluginInternal');
  assert.match(batch, /startEveryPlugin\(&plugins,/);
  assert.doesNotMatch(batch, /\bbreak\b/);
  const traversal = section(starter, 'async fn startEveryPlugin<', '#[cfg(test)]');
  assert.match(traversal, /for plugin in plugins/);
  assert.match(traversal, /PluginInitStatus::OTHER_ERROR/);
});

/** Keeps metadata generation outside service startup and its readiness path. */
test('MCP startup has no model runtime dependency', () => {
  const starter = source(`${base}plugins/MCPStarter.rs`);
  assert.doesNotMatch(starter, /generateMissingDescription|generatePluginDescription|ToolRuntimeSupport|block_on/);
});

/** Requires host-owned directory preparation before stdio service registration. */
test('imported MCP processes prepare their working directory through the host', () => {
  const starter = source(`${base}plugins/MCPStarter.rs`);
  assert.ok(starter.indexOf('preparePluginRuntimeDirectory(pluginId)') < starter.indexOf('bridge.registerMcpService('));
  const local = source(`${base}MCPLocalServer.rs`);
  const prepare = section(local, 'pub fn preparePluginRuntimeDirectory(', 'pub fn getPluginConfig(');
  assert.match(prepare, /pluginDirectoryPath\(pluginId\)\?/);
  assert.match(prepare, /\.fileSystemHost\s*\.makeDirectory\(&directory, true\)/);
  assert.doesNotMatch(prepare, /std::fs|cfg\(|unwrap_or/);
});

/** Prohibits replaying an already-sent tool call based on response error text. */
test('MCP bridge client sends each tool invocation once', () => {
  const client = source(`${base}plugins/MCPBridgeClient.rs`);
  const call = section(client, 'pub async fn callTool(', 'pub async fn callToolWithArguments(');
  assert.equal((call.match(/\.callTool\(/g) ?? []).length, 1);
  assert.doesNotMatch(call, /retryParams|errorMessage|\.contains\(/);
});

/** Keeps tool cache replacement unconditional, including empty discovery results. */
test('empty MCP tool discovery replaces old cache', () => {
  const starter = source(`${base}plugins/MCPStarter.rs`);
  const publish = section(starter, 'let tools = client.getTools();', 'MCPManager::getInstance');
  assert.doesNotMatch(publish, /if !tools\.is_empty\(\)/);
  assert.match(publish, /if let Err\(error\) = localServer\.cacheServerTools/);
});

/** Ensures every initialization phase consumes the shared deadline. */
test('MCP handshake phases share one deadline', () => {
  const bridge = source(`${base}plugins/MCPBridge.rs`);
  const local = section(bridge, 'fn initializeService(', 'fn callMcpTool(');
  const remote = section(bridge, 'fn initializeRemoteService(', 'fn initializeService(');
  assert.match(local, /deadline: &StartupDeadline/);
  assert.match(remote, /deadline: &StartupDeadline/);
  assert.match(local, /readJsonResponse\(active, initializeId, deadline\.remainingMs\(\)\?\)/);
  assert.match(local, /readJsonResponse\(active, listId, deadline\.remainingMs\(\)\?\)/);
  assert.ok((remote.match(/deadline\.remainingMs\(\)\?/g) ?? []).length >= 4);
});

/** Requires process-liveness checks after an empty stdout read. */
test('exited MCP processes fail without consuming the entire timeout', () => {
  const bridge = source(`${base}plugins/MCPBridge.rs`);
  const read = section(bridge, 'fn readJsonResponse(', 'fn successResponse(');
  assert.match(read, /process\.isRunning\(\)/);
  assert.match(read, /active\.ready = false/);
  assert.match(read, /MCP process exited before response/);
});
