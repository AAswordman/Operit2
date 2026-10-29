import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const source = readFileSync(new URL(
  'apps/flutter/app/lib/ui/features/chat/components/workspace/WorkspaceHomeContent.dart', root,
), 'utf8');

/** Pins the PR's compact geometry while allowing theme-owned Material surfaces. */
test('workspace home retains the PR toolbar and compact card geometry', () => {
  assert.doesNotMatch(source, /Color\(0x|_buildOverview|emptyDescription|_WorkspaceSectionIcon/);
  for (const widget of ['ListView', 'MenuAnchor', 'Card.filled', 'Card.outlined', 'TextButton.icon']) {
    assert.ok(source.includes(`${widget}(`), `Missing Material component: ${widget}`);
  }
  assert.match(source, /padding: const EdgeInsets\.symmetric\(horizontal: 14, vertical: 12\)/);
  assert.match(source, /padding: const EdgeInsets\.symmetric\(horizontal: 10, vertical: 8\)/);
  assert.match(source, /padding: const EdgeInsets\.all\(12\)/);
  assert.match(source, /constraints\.maxWidth < 520/);
  assert.match(source, /minimumSize: const Size\(0, 28\)/);
  assert.match(source, /fontSize: 11\.5/);
  assert.match(source, /color: colorScheme\.surfaceContainerLow/);
  for (const group of ['_WorkspaceHeaderCard', '_WorkspaceReviewHeroCard', '_TerminalWorkstationCard', '_BrowserWorkstationCard', '_WorkspaceRootPathCard']) {
    assert.ok(source.includes(`class ${group} `), `Missing PR layout group: ${group}`);
  }
});

/** Preserves session sources, callback ownership and explicit contribution states. */
test('workspace home retains live sessions and owner-scoped plugin previews', () => {
  assert.match(source, /Listenable\.merge/);
  assert.doesNotMatch(source, /ValueNotifier</);
  for (const callback of ['onOpenFolder', 'onAddFolder', 'onUnbindWorkspace', 'onSelectTerminalSession', 'onCloseTerminalSession', 'onSelectBrowserTab', 'onCloseBrowserSession']) {
    assert.ok(source.includes(callback), `Missing workspace callback: ${callback}`);
  }
  assert.match(source, /sessions\.take\(3\)/);
  assert.match(source, /tabs\.take\(3\)/);
  assert.match(source, /usage\.preview\.present\(context: context, clients: _clients\)/);
  assert.match(source, /key: ValueKey\(usage\.identity\)/);
  assert.match(source, /case WorkspaceContributionsFailed\(:final error\)/);
});

/** Each workstation owns its status and actions, not the workspace header. */
test('workspace overview has one owner for each action and no idle badges', () => {
  const header = source.split('class _WorkspaceHeaderCard ')[1]
    .split('class _WorkspaceFolderMenuItemTile ')[0];
  assert.doesNotMatch(header, /onOpenTerminal|onOpenBrowser|onOpenDiff|SessionCount/);
  assert.doesNotMatch(source, /_TerminalMenuAnchor|_BrowserMenuAnchor|_WorkspaceActionCapsule|就绪|挂载新目录/);
  for (const label of ['查看变更', '新建终端会话', '新建浏览器页面', '挂载新文件夹', '创建工作区', '选择工作区']) {
    assert.equal(source.split(`'${label}'`).length - 1, 1, `Duplicated action: ${label}`);
  }
  const terminal = source.split('class _TerminalWorkstationCard ')[1]
    .split('class _BrowserWorkstationCard ')[0];
  const browser = source.split('class _BrowserWorkstationCard ')[1]
    .split('class _WorkspaceRootPathCard ')[0];
  assert.match(terminal, /onPressed: onOpenTerminalSessions/);
  assert.doesNotMatch(terminal, /\btabs\b|onOpenBrowserSessions/);
  assert.match(browser, /onPressed: onOpenBrowserSessions/);
  assert.doesNotMatch(browser, /onOpenTerminalSessions/);
});

/** Device rows remain available, but navigation and each metric appear only once. */
test('context menu keeps a single device action and non-repeating metrics', () => {
  const contextSource = readFileSync(new URL(
    'apps/flutter/app/lib/ui/features/chat/components/workspace/ChatContextStatsTopBarButton.dart', root,
  ), 'utf8');
  assert.match(contextSource, /MenuAnchor\(/);
  assert.match(contextSource, /for \(final device in _topology!\.devices\)/);
  assert.match(contextSource, /watchRouteStatus\(chatId\)/);
  assert.match(contextSource, /deviceSpaceTopology\(\)/);
  assert.equal(contextSource.split('onPressed: _openDeviceSpaceSettings').length - 1, 1);
  assert.equal(contextSource.split('AppRouterGateway.navigate(').length - 1, 1);
  for (const label of ['上下文窗口', '输入 Token', '输出 Token', '添加设备']) {
    assert.equal(contextSource.split(`'${label}'`).length - 1, 1, `Duplicated metric/action: ${label}`);
  }
  assert.doesNotMatch(contextSource, /OperitGlassSurface|LinearProgressIndicator|总 Token|单设备空间|点击添加|claimMissingBindingLocal|Colors\./);
  assert.doesNotMatch(contextSource, /\? '0%'/);
  assert.match(contextSource, /epoch != _requestEpoch/);
  assert.match(contextSource, /无法加载上下文和设备信息/);
});
