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
