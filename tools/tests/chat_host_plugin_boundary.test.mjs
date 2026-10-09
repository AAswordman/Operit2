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
  assert.match(handler, /registry\.isPackageEnabled\(&context\.container_package_name\)/);
});


test('Compose action watch resolves only an existing engine after scheduling, never under the proxy manager lock', () => {
  const sourceText = source('core/crates/tool/services/src/tools/packTool/RuntimePackageManager.rs');
  const start = sourceText.indexOf('pub fn dispatchToolPkgComposeDslActionEvents(');
  const end = sourceText.indexOf('pub fn findToolPkgExecutionEngine(', start);
  const dispatch = sourceText.slice(start, end);
  assert.doesNotMatch(dispatch, /self\.getToolPkgExecutionEngine/);
  assert.ok(dispatch.indexOf('scheduleHostRuntimeAsyncTask') < dispatch.indexOf('manager.findToolPkgExecutionEngine'));
  assert.match(dispatch, /Compose execution context has been released/);
});
