import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const source = readFileSync(new URL(
  'apps/flutter/app/lib/ui/features/chat/components/workspace/WorkspaceHomeContent.dart', root,
), 'utf8');

/** Unbound chats get a setup surface rather than a disabled overview toolbar. */
test('unbound workspace has a dedicated responsive setup surface', () => {
  assert.match(source, /if \(!hasBoundWorkspace\) \{\s+return _WorkspaceUnboundContent\(/);
  const setup = source.split('class _WorkspaceUnboundContent ')[1]
    .split('class _WorkspaceWorkstationGrid ')[0];
  for (const widget of ['SingleChildScrollView', 'ConstrainedBox', 'Wrap', 'FilledButton.icon', 'OutlinedButton.icon']) {
    assert.ok(setup.includes(`${widget}(`), `Missing setup component: ${widget}`);
  }
  assert.match(setup, /minHeight: constraints\.maxHeight/);
  assert.match(setup, /maxWidth: 400/);
  assert.match(setup, /minimumSize: const Size\(164, 48\)/);
  assert.match(setup, /onPressed: onCreateWorkspace/);
  assert.match(setup, /onPressed: onChooseExistingWorkspace/);
  assert.doesNotMatch(setup, /Card\.|onOpenTerminal|onOpenBrowser|workspaceUsage/);
  assert.doesNotMatch(source, /_WorkspaceCapsuleButton|当前对话未绑定工作区/);
});

/** User-facing workstation names must not expose implementation acronyms. */
test('workstation titles omit backend annotations', () => {
  assert.match(source, /'终端工件'/);
  assert.match(source, /'浏览器工件'/);
  assert.doesNotMatch(source, /'终端工件 \(PTY\)'|'浏览器工件 \(WebView\)'/);
});

/** Pins the PR's compact geometry while allowing theme-owned Material surfaces. */
test('workspace home retains the PR toolbar and compact card geometry', () => {
  assert.doesNotMatch(source, /Color\(0x|_buildOverview|emptyDescription|_WorkspaceSectionIcon/);
  for (const widget of ['ListView', 'MenuAnchor', 'Card.filled', 'MenuItemButton']) {
    assert.ok(source.includes(`${widget}(`), `Missing Material component: ${widget}`);
  }
  assert.match(source, /padding: const EdgeInsets\.symmetric\(horizontal: 14, vertical: 12\)/);
  assert.match(source, /padding: const EdgeInsets\.symmetric\(horizontal: 10, vertical: 8\)/);
  assert.match(source, /padding: const EdgeInsets\.all\(12\)/);
  assert.match(source, /constraints\.maxWidth < 520/);
  assert.match(source, /minimumSize: const Size\(0, 28\)/);
  assert.match(source, /fontSize: 11\.5/);
  assert.match(source, /color: colorScheme\.surfaceContainerLow/);
  for (const group of ['_WorkspaceHeaderCard', '_TerminalWorkstationCard', '_BrowserWorkstationCard', '_WorkspaceFoldersCard']) {
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
    .split('class _WorkspaceFolderTile ')[0];
  assert.doesNotMatch(header, /onOpenTerminal|onOpenBrowser|onOpenDiff|SessionCount/);
  assert.doesNotMatch(source, /_TerminalMenuAnchor|_BrowserMenuAnchor|_WorkspaceActionCapsule|就绪|挂载新目录/);
  assert.match(source, /widget\.pluginContent/);
  assert.doesNotMatch(source, /_WorkspaceReviewHeroCard|openDiffTab/);
  for (const label of ['创建工作区', '选择工作区']) {
    assert.equal(source.split(`'${label}'`).length - 1, 1, `Duplicated action: ${label}`);
  }
  const terminal = source.split('class _TerminalWorkstationCard ')[1]
    .split('class _BrowserWorkstationCard ')[0];
  const browser = source.split('class _BrowserWorkstationCard ')[1]
    .split('class _WorkspaceFoldersCard ')[0];
  assert.match(terminal, /onPressed: onOpenTerminalSessions/);
  assert.doesNotMatch(terminal, /\btabs\b|onOpenBrowserSessions/);
  assert.match(browser, /onPressed: onOpenBrowserSessions/);
  assert.doesNotMatch(browser, /onOpenTerminalSessions/);
});

/** Creation belongs to the trailing plus, not a count badge or footer button. */
test('workstation headers have a single plus action without counts or footer buttons', () => {
  const terminal = source.split('class _TerminalWorkstationCard ')[1]
    .split('class _BrowserWorkstationCard ')[0];
  const browser = source.split('class _BrowserWorkstationCard ')[1]
    .split('class _WorkspaceFoldersCard ')[0];
  for (const [card, callback] of [[terminal, 'onOpenTerminal'], [browser, 'onOpenBrowser']]) {
    const header = card.split('ListenableBuilder(')[0];
    assert.match(header, /IconButton\(/);
    assert.ok(header.includes(`onPressed: ${callback},`));
    assert.match(header, /Icon\(Icons\.add_rounded/);
    assert.equal(card.split(`onPressed: ${callback},`).length - 1, 1);
    assert.doesNotMatch(card, /ValueListenableBuilder<int>|\$count 个|OutlinedButton\(/);
    assert.doesNotMatch(card, /Text\('新建终端会话'|Text\('新建浏览器页面'/);
  }
});

/** Folder navigation and mounting have one owner, outside the identity menu. */
test('folder card owns the only folder list and trailing mount action', () => {
  const card = source.split('class _WorkspaceFoldersCard ')[1]
    .split('class _WorkspaceHeaderCard ')[0];
  assert.match(card, /IconButton\(\s+onPressed: onAddFolder,/);
  assert.match(card, /Icon\(Icons\.add_rounded/);
  assert.match(card, /for \(final folder in workspaceUsage\.mountedFolders\)/);
  assert.match(card, /onTap: \(\) => onOpenFolder\(folder\)/);
  assert.match(card, /workspaceUsage\.mountedFoldersLoading/);
  assert.match(card, /workspaceUsage\.mountedFoldersError!/);
  assert.equal(source.split('for (final folder in').length - 1, 1);
  assert.doesNotMatch(source, /_WorkspaceRootPathCard|_WorkspaceFolderMenu|_CompactActionButton|切换工作区|挂载新文件夹/);
  const menu = source.split('class _WorkspaceMenuAnchor ')[1]
    .split('class _WorkspaceToolCapsule ')[0];
  assert.doesNotMatch(menu, /mountedFolders|onAddFolder|onOpenFolder|onChooseExistingWorkspace/);
  assert.equal(menu.split('MenuItemButton(').length - 1, 1);
  assert.match(menu, /onPressed: widget\.onUnbindWorkspace/);
  assert.match(menu, /leadingIcon: const Icon\(Icons\.link_off_rounded/);
  assert.match(menu, /foregroundColor: colorScheme\.error/);
  assert.doesNotMatch(menu, /TextButton|OutlinedButton|FilledButton/);
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
  assert.match(contextSource, /IconButton\(\s+tooltip: '添加设备',\s+onPressed: _openDeviceSpaceSettings,/);
  assert.doesNotMatch(contextSource, /(?:Text|label:)\s*\(?'添加设备'/);
  assert.match(contextSource, /'设备',\s+style: Theme\.of\(context\)\.textTheme\.titleSmall/);
});

/** Top-bar menus must be laid out against the chat pane, not the whole window. */
test('side panel gives primary popups an overlay bounded by the primary pane', () => {
  const panel = readFileSync(new URL(
    'apps/flutter/app/lib/ui/common/components/AdaptiveSidePanel.dart', root,
  ), 'utf8');
  assert.match(panel, /Expanded\(child: Overlay\.wrap\(child: content\)\)/);
  assert.ok(panel.indexOf('Overlay.wrap(child: content)') < panel.indexOf('AnimatedContainer('));
});
