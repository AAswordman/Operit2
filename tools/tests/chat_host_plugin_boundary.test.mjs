import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const chat = 'apps/flutter/app/lib/ui/features/chat/';
const source = path => readFileSync(new URL(path, root), 'utf8');

test('Flutter chat host has no plugin memory business actions or destinations', () => {
  assert.equal(existsSync(new URL(chat + 'viewmodel/PluginMemoryActions.dart', root)), false);
  for (const file of [
    'viewmodel/ChatViewModel.dart',
    'components/MessageContextMenu.dart',
    'components/ChatScreenContent.dart',
    'components/ChatMultiSelectBar.dart',
  ]) {
    assert.doesNotMatch(source(chat + file),
      /enqueuePluginMemoryCandidates|enqueueSelectedMessagesForMemory|memoryOwnerForChat|updateMemory|memory\.candidate\.enqueue|ownerKey|sourceType|queueMemory|modifyMemory|onQueueMemory|加入记忆|修改记忆/,
      file);
  }
});

test('plugin message menus remain generic discovery and callback forwarding', () => {
  const menu = source(chat + 'components/MessageContextMenu.dart');
  assert.match(menu, /getToolPkgChatMessageMenuItems\(/);
  assert.match(menu, /invokeToolPkgChatMessageMenuItem\(/);
  assert.match(menu, /containerPackageName: item\.containerPackageName/);
  assert.match(menu, /itemId: item\.itemId/);
  assert.match(menu, /message: _toolPkgMessageSnapshot\(\)/);
});

test('chat display and copy do not strip plugin-specific memory markup', () => {
  for (const file of [
    'components/MessageContextMenu.dart',
    'components/style/bubble/BubbleUserMessageComposable.dart',
    'components/style/cursor/UserMessageComposable.dart',
  ]) {
    assert.doesNotMatch(source(chat + file), /memoryTag|<memory>/, file);
  }
  assert.doesNotMatch(source('apps/flutter/app/lib/util/ChatMarkupRegex.dart'), /memoryTag|<memory>/);
});


test('native chat execution and composer protocol have no memory-business switches', () => {
  for (const file of [
    'core/crates/runtime/application/src/services/ChatServiceCore.rs',
    'core/crates/runtime/application/src/services/core/MessageCoordinationDelegate.rs',
    'core/crates/runtime/application/src/services/core/MessageProcessingDelegate.rs',
    'core/crates/runtime/application/src/core/chat/AIMessageManager.rs',
    'core/crates/runtime/application/src/data/preferences/ApiPreferences.rs',
    'core/crates/provider/services/src/chat/EnhancedAIService.rs',
    chat + 'components/style/input/agent/AgentInputMenuPopup.dart',
  ]) {
    assert.doesNotMatch(source(file), /enableMemoryAutoUpdate|saveEnableMemoryAutoUpdate|memoryOwnerKeyForChat|enable_memory_auto_update/, file);
  }
});


test('execution engine acquisition releases the proxy manager lock before worker authentication', () => {
  assert.match(source('core/crates/tool/services/src/tools/packTool/RuntimePackageManager.rs'), /pub async fn acquireToolPkgExecutionEngine/);
  const handler = source('core/crates/tool/services/src/tools/AIToolHandler.rs');
  assert.match(handler, /fn for_toolpkg_execution_context/);
  assert.match(handler, /registry\.packageRegistryReadiness\(\)\.require_ready\(\)\?/);
  assert.match(handler, /getEnabledToolPkgContainerRuntimes/);
});


test('Compose session resolves an acquired engine without executing work under the package manager lock', () => {
  const sourceText = source('core/crates/tool/services/src/tools/packTool/RuntimePackageManager.rs');
  const start = sourceText.indexOf('pub fn openComposeDslSession(');
  const end = sourceText.indexOf('pub fn findToolPkgExecutionEngine(', start);
  const open = sourceText.slice(start, end);
  assert.match(open, /self\.findToolPkgExecutionEngine/);
  assert.doesNotMatch(open, /execute_compose|dispatch_compose|self\.getToolPkgExecutionEngine/);
  assert.match(open, /Compose execution context has not been acquired or has been released/);
});

/** Ensures catalog discovery cannot revoke the execution leases held by live Compose sessions. */
test('package rescans clean only unleased engines and preserve page-owned Compose sessions', () => {
  const manager = source('core/crates/tool/services/src/tools/packTool/RuntimePackageManager.rs');
  const start = manager.indexOf('pub fn loadAvailablePackages(');
  const end = manager.indexOf('pub fn isToolPkgProtectionSecretConfigured(', start);
  assert.ok(start >= 0 && end > start);
  const scan = manager.slice(start, end);
  assert.match(scan, /applyPackageScanSnapshot\(mergedSnapshot\)/);
  assert.doesNotMatch(scan, /destroy\w*\(|revokeContainerExecutionContexts\(|session\.close\(|composeDslSessions/);

  const sdk = source('core/crates/plugin/sdk/src/toolpkg/ToolPkgManager.rs');
  const replacementStart = sdk.indexOf('pub fn replaceRuntimeMaps(');
  const replacementEnd = sdk.indexOf('pub fn getEnabledToolPkgContainerRuntimes(', replacementStart);
  assert.ok(replacementStart >= 0 && replacementEnd > replacementStart);
  const replacement = sdk.slice(replacementStart, replacementEnd);
  assert.match(replacement, /removeExecutionEnginesMatching\(\|entry\| entry\.activeLeases == 0\)/);
  assert.doesNotMatch(replacement, /activeLeases\s*(?:\+=|-=|=(?!=))|revokeContainerExecutionContexts\(/);
});

/** Keeps lease release and explicit container revocation distinct without parallel public cleanup APIs. */
test('ToolPkg lifecycle exposes owner release and explicit revocation, not interchangeable cleanup APIs', () => {
  const sdk = source('core/crates/plugin/sdk/src/toolpkg/ToolPkgManager.rs');
  assert.doesNotMatch(sdk, /pub fn (?:destroyUnleasedToolPkgExecutionEngines|destroyToolPkgExecutionEngines|clear|destroy)\(/);
  assert.match(sdk, /pub fn releaseToolPkgExecutionEngine\(/);
  assert.match(sdk, /pub fn revokeContainerExecutionContexts\(/);
  const releaseStart = sdk.indexOf('pub fn releaseToolPkgExecutionEngine(');
  const releaseEnd = sdk.indexOf('pub fn revokeContainerExecutionContexts(', releaseStart);
  assert.ok(releaseStart >= 0 && releaseEnd > releaseStart);
  assert.match(sdk.slice(releaseStart, releaseEnd), /entry\.activeLeases -= 1/);
  const revokeEnd = sdk.indexOf('fn removeExecutionEnginesMatching(', releaseEnd);
  assert.ok(revokeEnd > releaseEnd);
  const revoke = sdk.slice(releaseEnd, revokeEnd);
  assert.match(revoke, /entry\.containerPackageName == normalizedContainer/);
  assert.doesNotMatch(revoke, /activeLeases/);
});
