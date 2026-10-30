// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';

import '../../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../common/ChatAvatarImage.dart';
import 'browser/WorkspaceBrowserViewStore.dart';
import 'terminal/WorkspaceTerminalSessions.dart';
import 'WorkspaceOverviewModels.dart';

class WorkspaceHomeContent extends StatefulWidget {
  /// Creates the workspace home content surface.
  const WorkspaceHomeContent({
    super.key,
    required this.workspacePath,
    required this.workspaceUsage,
    required this.terminalSessionCountListenable,
    required this.browserSessionCountListenable,
    this.terminalSessionsListenable,
    required this.onOpenFolder,
    required this.onAddFolder,
    required this.onCreateWorkspace,
    required this.onChooseExistingWorkspace,
    required this.onUnbindWorkspace,
    required this.onOpenTerminal,
    required this.onOpenTerminalSessions,
    required this.onOpenBrowserSessions,
    required this.onOpenBrowser,
    this.onSelectTerminalSession,
    this.onCloseTerminalSession,
    this.onSelectBrowserTab,
    this.onCloseBrowserSession,
    this.pluginContent = const SizedBox.shrink(),
  });

  final String? workspacePath;
  final WorkspaceOverviewUsage workspaceUsage;
  final ValueListenable<int> terminalSessionCountListenable;
  final ValueListenable<int> browserSessionCountListenable;
  final ValueListenable<List<WorkspaceTerminalSessionInfo>>?
  terminalSessionsListenable;
  final ValueChanged<WorkspaceMountedFolder> onOpenFolder;
  final VoidCallback onAddFolder;
  final VoidCallback onCreateWorkspace;
  final VoidCallback onChooseExistingWorkspace;
  final VoidCallback onUnbindWorkspace;
  final VoidCallback onOpenTerminal;
  final VoidCallback onOpenTerminalSessions;
  final VoidCallback onOpenBrowserSessions;
  final VoidCallback onOpenBrowser;
  final ValueChanged<WorkspaceTerminalSessionInfo>? onSelectTerminalSession;
  final ValueChanged<WorkspaceTerminalSessionInfo>? onCloseTerminalSession;
  final ValueChanged<WorkspaceBrowserTabState>? onSelectBrowserTab;
  final ValueChanged<String>? onCloseBrowserSession;
  final Widget pluginContent;

  @override
  State<WorkspaceHomeContent> createState() => _WorkspaceHomeContentState();
}

class _WorkspaceHomeContentState extends State<WorkspaceHomeContent> {
  /// Builds the compact overview with one owner for each workspace action.
  @override
  Widget build(BuildContext context) {
    final boundWorkspacePath = widget.workspacePath?.trim();
    final boundWorkspaceName = widget.workspaceUsage.workspaceName?.trim();
    final hasBoundWorkspace =
        (boundWorkspacePath != null && boundWorkspacePath.isNotEmpty) ||
        (boundWorkspaceName != null && boundWorkspaceName.isNotEmpty);
    final title = boundWorkspaceName != null && boundWorkspaceName.isNotEmpty
        ? boundWorkspaceName
        : '工作区总览';
    return ListView(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      children: <Widget>[
        if (hasBoundWorkspace)
          Card.filled(
            margin: EdgeInsets.zero,
            color: Colors.transparent,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(10),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
              child: _WorkspaceHeaderCard(
                title: title,
                workspaceUsage: widget.workspaceUsage,
                onUnbindWorkspace: widget.onUnbindWorkspace,
              ),
            ),
          )
        else
          _WorkspaceUnboundContent(
            onCreateWorkspace: widget.onCreateWorkspace,
            onChooseExistingWorkspace: widget.onChooseExistingWorkspace,
          ),
        const SizedBox(height: 12),
        if (hasBoundWorkspace) widget.pluginContent,
        _WorkspaceWorkstationGrid(
          key: const ValueKey('workspace-workstations'),
          terminalSessionCountListenable: widget.terminalSessionCountListenable,
          browserSessionCountListenable: widget.browserSessionCountListenable,
          terminalSessionsListenable: widget.terminalSessionsListenable,
          onOpenTerminal: widget.onOpenTerminal,
          onOpenTerminalSessions: widget.onOpenTerminalSessions,
          onOpenBrowser: widget.onOpenBrowser,
          onOpenBrowserSessions: widget.onOpenBrowserSessions,
          onSelectTerminalSession: widget.onSelectTerminalSession,
          onCloseTerminalSession: widget.onCloseTerminalSession,
          onSelectBrowserTab: widget.onSelectBrowserTab,
          onCloseBrowserSession: widget.onCloseBrowserSession,
        ),
        if (hasBoundWorkspace) ...<Widget>[
          const SizedBox(height: 20),
          _WorkspaceFoldersCard(
            workspaceUsage: widget.workspaceUsage,
            onOpenFolder: widget.onOpenFolder,
            onAddFolder: widget.onAddFolder,
          ),
        ],
      ],
    );
  }
}

class _WorkspaceUnboundContent extends StatelessWidget {
  /// Offers optional project binding alongside the always available tools.
  const _WorkspaceUnboundContent({
    required this.onCreateWorkspace,
    required this.onChooseExistingWorkspace,
  });

  final VoidCallback onCreateWorkspace;
  final VoidCallback onChooseExistingWorkspace;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 2, vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            '工作区（可选）',
            style: theme.textTheme.titleSmall?.copyWith(
              fontWeight: FontWeight.w500,
            ),
          ),
          const SizedBox(height: 6),
          Text(
            '终端和浏览器可直接使用，绑定工作区后可管理项目文件。',
            style: theme.textTheme.bodySmall?.copyWith(
              color: colors.onSurfaceVariant,
              height: 1.6,
            ),
          ),
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            runSpacing: 4,
            children: <Widget>[
              FilledButton.tonalIcon(
                onPressed: onCreateWorkspace,
                style: FilledButton.styleFrom(
                  minimumSize: const Size(0, 40),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 8,
                  ),
                ),
                icon: const Icon(Icons.add_rounded, size: 18),
                label: const Text('创建工作区'),
              ),
              TextButton.icon(
                onPressed: onChooseExistingWorkspace,
                style: TextButton.styleFrom(
                  foregroundColor: colors.onSurfaceVariant,
                  minimumSize: const Size(0, 40),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 8,
                  ),
                ),
                icon: const Icon(Icons.folder_open_outlined, size: 18),
                label: const Text('选择工作区'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _WorkspaceWorkstationGrid extends StatelessWidget {
  /// Keeps terminal and browser controls in their own compact cards.
  const _WorkspaceWorkstationGrid({
    super.key,
    required this.terminalSessionCountListenable,
    required this.browserSessionCountListenable,
    required this.terminalSessionsListenable,
    required this.onOpenTerminal,
    required this.onOpenTerminalSessions,
    required this.onOpenBrowser,
    required this.onOpenBrowserSessions,
    required this.onSelectTerminalSession,
    required this.onCloseTerminalSession,
    required this.onSelectBrowserTab,
    required this.onCloseBrowserSession,
  });

  final ValueListenable<int> terminalSessionCountListenable;
  final ValueListenable<int> browserSessionCountListenable;
  final ValueListenable<List<WorkspaceTerminalSessionInfo>>?
  terminalSessionsListenable;
  final VoidCallback onOpenTerminal;
  final VoidCallback onOpenTerminalSessions;
  final VoidCallback onOpenBrowser;
  final VoidCallback onOpenBrowserSessions;
  final ValueChanged<WorkspaceTerminalSessionInfo>? onSelectTerminalSession;
  final ValueChanged<WorkspaceTerminalSessionInfo>? onCloseTerminalSession;
  final ValueChanged<WorkspaceBrowserTabState>? onSelectBrowserTab;
  final ValueChanged<String>? onCloseBrowserSession;

  /// Stacks the original cards only when the available width requires it.
  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final isNarrow = constraints.maxWidth < 520;
        final terminalCard = _TerminalWorkstationCard(
          terminalCountListenable: terminalSessionCountListenable,
          sessionsListenable: terminalSessionsListenable,
          onOpenTerminal: onOpenTerminal,
          onOpenTerminalSessions: onOpenTerminalSessions,
          onSelectSession: onSelectTerminalSession,
          onCloseSession: onCloseTerminalSession,
        );
        final browserCard = _BrowserWorkstationCard(
          browserCountListenable: browserSessionCountListenable,
          onOpenBrowser: onOpenBrowser,
          onOpenBrowserSessions: onOpenBrowserSessions,
          onSelectBrowserTab: onSelectBrowserTab,
          onCloseBrowserSession: onCloseBrowserSession,
        );

        if (isNarrow) {
          return Column(
            children: <Widget>[
              terminalCard,
              const SizedBox(height: 10),
              browserCard,
            ],
          );
        }

        return Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Expanded(child: terminalCard),
            const SizedBox(width: 10),
            Expanded(child: browserCard),
          ],
        );
      },
    );
  }
}

class _TerminalWorkstationCard extends StatelessWidget {
  /// Creates the sole terminal status and session action surface.
  const _TerminalWorkstationCard({
    required this.terminalCountListenable,
    required this.sessionsListenable,
    required this.onOpenTerminal,
    required this.onOpenTerminalSessions,
    required this.onSelectSession,
    required this.onCloseSession,
  });

  final ValueListenable<int> terminalCountListenable;
  final ValueListenable<List<WorkspaceTerminalSessionInfo>>? sessionsListenable;
  final VoidCallback onOpenTerminal;
  final VoidCallback onOpenTerminalSessions;
  final ValueChanged<WorkspaceTerminalSessionInfo>? onSelectSession;
  final ValueChanged<WorkspaceTerminalSessionInfo>? onCloseSession;

  /// Shows live sessions without a duplicate toolbar menu or idle badge.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final accentColor = colorScheme.onSurfaceVariant;

    return Card.filled(
      margin: EdgeInsets.zero,
      color: Colors.transparent,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(
                  Icons.terminal_rounded,
                  size: 16,
                  color: colorScheme.onSurfaceVariant,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    '终端工件',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                IconButton(
                  onPressed: onOpenTerminal,
                  tooltip: '新建终端',
                  visualDensity: VisualDensity.compact,
                  color: colorScheme.onSurfaceVariant,
                  icon: const Icon(Icons.add_rounded, size: 18),
                ),
              ],
            ),
            const SizedBox(height: 10),
            ListenableBuilder(
              listenable: Listenable.merge(<Listenable>[
                terminalCountListenable,
                if (sessionsListenable != null) sessionsListenable!,
              ]),
              builder: (context, _) {
                final sessions =
                    sessionsListenable?.value ??
                    const <WorkspaceTerminalSessionInfo>[];
                if (sessions.isEmpty && terminalCountListenable.value == 0) {
                  return Padding(
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    child: Text(
                      '暂无运行中的终端会话',
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant.withValues(
                          alpha: 0.65,
                        ),
                        fontSize: 11.5,
                      ),
                    ),
                  );
                }
                return Column(
                  children: <Widget>[
                    for (final session in sessions.take(3))
                      Padding(
                        padding: const EdgeInsets.only(bottom: 6),
                        child: Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 8,
                            vertical: 5,
                          ),
                          decoration: BoxDecoration(
                            color: colorScheme.surface.withValues(alpha: 0.5),
                            borderRadius: BorderRadius.circular(6),
                            border: Border.all(
                              color: colorScheme.outlineVariant.withValues(
                                alpha: 0.15,
                              ),
                            ),
                          ),
                          child: Row(
                            children: <Widget>[
                              Expanded(
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: <Widget>[
                                    Text(
                                      session.title.isNotEmpty
                                          ? session.title
                                          : '终端会话',
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                      style: theme.textTheme.bodySmall
                                          ?.copyWith(
                                            color: colorScheme.onSurface,
                                            fontWeight: FontWeight.w500,
                                            fontSize: 11,
                                          ),
                                    ),
                                    if (session.workingDir.isNotEmpty)
                                      Text(
                                        session.workingDir,
                                        maxLines: 1,
                                        overflow: TextOverflow.ellipsis,
                                        style: theme.textTheme.labelSmall
                                            ?.copyWith(
                                              color:
                                                  colorScheme.onSurfaceVariant,
                                              fontSize: 9.5,
                                            ),
                                      ),
                                  ],
                                ),
                              ),
                              const SizedBox(width: 6),
                              InkWell(
                                onTap: () => onSelectSession?.call(session),
                                borderRadius: BorderRadius.circular(4),
                                child: Padding(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 6,
                                    vertical: 3,
                                  ),
                                  child: Text(
                                    '切换',
                                    style: TextStyle(
                                      color: accentColor,
                                      fontSize: 11,
                                      fontWeight: FontWeight.w500,
                                    ),
                                  ),
                                ),
                              ),
                              if (onCloseSession != null)
                                InkWell(
                                  onTap: () => onCloseSession?.call(session),
                                  borderRadius: BorderRadius.circular(4),
                                  child: const Padding(
                                    padding: EdgeInsets.all(2),
                                    child: Icon(Icons.close_rounded, size: 13),
                                  ),
                                ),
                            ],
                          ),
                        ),
                      ),
                    if (sessions.length > 3 ||
                        terminalCountListenable.value > sessions.length)
                      Align(
                        alignment: AlignmentDirectional.centerEnd,
                        child: TextButton(
                          onPressed: onOpenTerminalSessions,
                          style: TextButton.styleFrom(
                            minimumSize: const Size(0, 28),
                            padding: const EdgeInsets.symmetric(horizontal: 6),
                            visualDensity: VisualDensity.compact,
                            tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                          ),
                          child: const Text(
                            '管理全部',
                            style: TextStyle(fontSize: 11.5),
                          ),
                        ),
                      ),
                  ],
                );
              },
            ),
          ],
        ),
      ),
    );
  }
}

class _BrowserWorkstationCard extends StatelessWidget {
  /// Creates the sole browser status and page action surface.
  const _BrowserWorkstationCard({
    required this.browserCountListenable,
    required this.onOpenBrowser,
    required this.onOpenBrowserSessions,
    required this.onSelectBrowserTab,
    required this.onCloseBrowserSession,
  });

  final ValueListenable<int> browserCountListenable;
  final VoidCallback onOpenBrowser;
  final VoidCallback onOpenBrowserSessions;
  final ValueChanged<WorkspaceBrowserTabState>? onSelectBrowserTab;
  final ValueChanged<String>? onCloseBrowserSession;

  /// Shows live pages and exposes management when the preview is incomplete.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final accentColor = colorScheme.onSurfaceVariant;
    final browserStore = WorkspaceBrowserViewStore.instance;

    return Card.filled(
      margin: EdgeInsets.zero,
      color: Colors.transparent,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(
                  Icons.public_rounded,
                  size: 16,
                  color: colorScheme.onSurfaceVariant,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    '浏览器工件',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                IconButton(
                  onPressed: onOpenBrowser,
                  tooltip: '新建浏览器页面',
                  visualDensity: VisualDensity.compact,
                  color: colorScheme.onSurfaceVariant,
                  icon: const Icon(Icons.add_rounded, size: 18),
                ),
              ],
            ),
            const SizedBox(height: 10),
            ListenableBuilder(
              listenable: Listenable.merge(<Listenable>[
                browserStore,
                browserCountListenable,
              ]),
              builder: (context, _) {
                final tabs = browserStore.tabs;
                if (tabs.isEmpty && browserCountListenable.value == 0) {
                  return Padding(
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    child: Text(
                      '暂无打开的浏览器页面',
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant.withValues(
                          alpha: 0.65,
                        ),
                        fontSize: 11.5,
                      ),
                    ),
                  );
                }
                return Column(
                  children: <Widget>[
                    for (final tab in tabs.take(3))
                      Padding(
                        padding: const EdgeInsets.only(bottom: 6),
                        child: Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 8,
                            vertical: 5,
                          ),
                          decoration: BoxDecoration(
                            color: colorScheme.surface.withValues(alpha: 0.5),
                            borderRadius: BorderRadius.circular(6),
                            border: Border.all(
                              color: colorScheme.outlineVariant.withValues(
                                alpha: 0.15,
                              ),
                            ),
                          ),
                          child: Row(
                            children: <Widget>[
                              Expanded(
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: <Widget>[
                                    Text(
                                      tab.title.isNotEmpty
                                          ? tab.title
                                          : '浏览器页面',
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                      style: theme.textTheme.bodySmall
                                          ?.copyWith(
                                            color: colorScheme.onSurface,
                                            fontWeight: FontWeight.w500,
                                            fontSize: 11,
                                          ),
                                    ),
                                    if (tab.url.isNotEmpty)
                                      Text(
                                        tab.url,
                                        maxLines: 1,
                                        overflow: TextOverflow.ellipsis,
                                        style: theme.textTheme.labelSmall
                                            ?.copyWith(
                                              color:
                                                  colorScheme.onSurfaceVariant,
                                              fontSize: 9.5,
                                            ),
                                      ),
                                  ],
                                ),
                              ),
                              const SizedBox(width: 6),
                              InkWell(
                                onTap: () => onSelectBrowserTab?.call(tab),
                                borderRadius: BorderRadius.circular(4),
                                child: Padding(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 6,
                                    vertical: 3,
                                  ),
                                  child: Text(
                                    '切换',
                                    style: TextStyle(
                                      color: accentColor,
                                      fontSize: 11,
                                      fontWeight: FontWeight.w500,
                                    ),
                                  ),
                                ),
                              ),
                              if (onCloseBrowserSession != null)
                                InkWell(
                                  onTap: () =>
                                      onCloseBrowserSession?.call(tab.id),
                                  borderRadius: BorderRadius.circular(4),
                                  child: const Padding(
                                    padding: EdgeInsets.all(2),
                                    child: Icon(Icons.close_rounded, size: 13),
                                  ),
                                ),
                            ],
                          ),
                        ),
                      ),
                    if (tabs.length > 3 ||
                        browserCountListenable.value > tabs.length)
                      Align(
                        alignment: AlignmentDirectional.centerEnd,
                        child: TextButton(
                          onPressed: onOpenBrowserSessions,
                          style: TextButton.styleFrom(
                            minimumSize: const Size(0, 28),
                            padding: const EdgeInsets.symmetric(horizontal: 6),
                            visualDensity: VisualDensity.compact,
                            tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                          ),
                          child: const Text(
                            '管理全部',
                            style: TextStyle(fontSize: 11.5),
                          ),
                        ),
                      ),
                  ],
                );
              },
            ),
          ],
        ),
      ),
    );
  }
}

class _WorkspaceFoldersCard extends StatelessWidget {
  /// Owns the only folder list and mount action in the overview.
  const _WorkspaceFoldersCard({
    required this.workspaceUsage,
    required this.onOpenFolder,
    required this.onAddFolder,
  });

  final WorkspaceOverviewUsage workspaceUsage;
  final ValueChanged<WorkspaceMountedFolder> onOpenFolder;
  final VoidCallback onAddFolder;

  /// Lists mounted folders below the workstations with one trailing add action.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Card.filled(
      margin: EdgeInsets.zero,
      color: Colors.transparent,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(
                  Icons.folder_outlined,
                  size: 16,
                  color: colorScheme.onSurfaceVariant,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    '文件夹',
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ),
                IconButton(
                  onPressed: onAddFolder,
                  tooltip: '挂载文件夹',
                  visualDensity: VisualDensity.compact,
                  color: colorScheme.onSurfaceVariant,
                  icon: const Icon(Icons.add_rounded, size: 18),
                ),
              ],
            ),
            const SizedBox(height: 6),
            if (workspaceUsage.mountedFoldersLoading)
              const Text('正在加载文件夹…')
            else if (workspaceUsage.mountedFoldersError != null)
              Text(
                workspaceUsage.mountedFoldersError!,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: colorScheme.error,
                ),
              )
            else if (workspaceUsage.mountedFolders.isEmpty)
              Text(
                '暂无挂载的文件夹',
                style: theme.textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              )
            else
              for (final folder in workspaceUsage.mountedFolders)
                _WorkspaceFolderTile(
                  folder: folder,
                  onTap: () => onOpenFolder(folder),
                ),
          ],
        ),
      ),
    );
  }
}

class _WorkspaceHeaderCard extends StatelessWidget {
  /// Creates workspace identity and contribution controls without folder actions.
  const _WorkspaceHeaderCard({
    required this.title,
    required this.workspaceUsage,
    required this.onUnbindWorkspace,
  });

  final String title;
  final WorkspaceOverviewUsage workspaceUsage;
  final VoidCallback onUnbindWorkspace;

  /// Keeps workspace actions and contribution previews in the compact header.
  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Wrap(
          spacing: 6,
          runSpacing: 6,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: <Widget>[
            _WorkspaceMenuAnchor(
              title: title,
              onUnbindWorkspace: onUnbindWorkspace,
            ),
            _WorkspaceContributionAvatarStrip(
              contributions: workspaceUsage.contributions,
            ),
          ],
        ),
        _WorkspaceContributionStateNotice(
          contributions: workspaceUsage.contributions,
        ),
      ],
    );
  }
}

class _WorkspaceFolderTile extends StatelessWidget {
  /// Creates an inline navigation row for one mounted folder.
  const _WorkspaceFolderTile({required this.folder, required this.onTap});

  final WorkspaceMountedFolder folder;
  final VoidCallback onTap;

  /// Shows each folder name and path once with an end-aligned navigation cue.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final path = folder.path.trim();

    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(6),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
          child: Row(
            children: <Widget>[
              Icon(
                Icons.folder_outlined,
                size: 14,
                color: colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    Text(
                      folder.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurface,
                        fontWeight: FontWeight.w500,
                        fontSize: 12,
                      ),
                    ),
                    if (path.isNotEmpty)
                      Tooltip(
                        message: path,
                        waitDuration: const Duration(milliseconds: 400),
                        child: Text(
                          path,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: colorScheme.onSurfaceVariant.withValues(
                              alpha: 0.7,
                            ),
                            fontSize: 10.5,
                          ),
                        ),
                      ),
                  ],
                ),
              ),
              const SizedBox(width: 4),
              Icon(
                Icons.arrow_forward_ios_rounded,
                size: 11,
                color: colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _WorkspaceMenuAnchor extends StatefulWidget {
  /// Creates the workspace identity menu without duplicating folder controls.
  const _WorkspaceMenuAnchor({
    required this.title,
    required this.onUnbindWorkspace,
  });

  final String title;
  final VoidCallback onUnbindWorkspace;

  /// Owns the workspace menu's expanded appearance.
  @override
  State<_WorkspaceMenuAnchor> createState() => _WorkspaceMenuAnchorState();
}

class _WorkspaceMenuAnchorState extends State<_WorkspaceMenuAnchor> {
  bool _isOpen = false;

  /// Presents unbinding as a left-aligned native menu item, not a centered button.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return MenuAnchor(
      alignmentOffset: const Offset(0, 4),
      onOpen: () => setState(() => _isOpen = true),
      onClose: () => setState(() => _isOpen = false),
      style: MenuStyle(
        backgroundColor: WidgetStatePropertyAll(
          colorScheme.surfaceContainerHigh,
        ),
        padding: const WidgetStatePropertyAll(EdgeInsets.all(4)),
        shape: WidgetStatePropertyAll(
          RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
        ),
      ),
      builder: (context, controller, _) => _WorkspaceToolCapsule(
        icon: Icons.work_outline_rounded,
        accentColor: colorScheme.onSurfaceVariant,
        label: widget.title,
        isExpanded: _isOpen,
        onTap: () {
          if (controller.isOpen) {
            controller.close();
          } else {
            controller.open();
          }
        },
      ),
      menuChildren: <Widget>[
        MenuItemButton(
          onPressed: widget.onUnbindWorkspace,
          leadingIcon: const Icon(Icons.link_off_rounded, size: 18),
          style: MenuItemButton.styleFrom(
            foregroundColor: colorScheme.error,
            minimumSize: const Size(180, 44),
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(6),
            ),
          ),
          child: const Text('解除绑定'),
        ),
      ],
    );
  }
}

class _WorkspaceToolCapsule extends StatelessWidget {
  const _WorkspaceToolCapsule({
    required this.icon,
    required this.accentColor,
    required this.label,
    required this.isExpanded,
    required this.onTap,
  });

  final IconData icon;
  final Color accentColor;
  final String label;
  final bool isExpanded;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return TextButton(
      style: TextButton.styleFrom(
        foregroundColor: colorScheme.onSurface,
        backgroundColor: isExpanded
            ? colorScheme.surfaceContainerHigh
            : Colors.transparent,
        minimumSize: const Size(0, 28),
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
        visualDensity: VisualDensity.compact,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        textStyle: theme.textTheme.labelMedium?.copyWith(fontSize: 11.5),
      ),
      onPressed: onTap,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(icon, size: 14, color: accentColor),
          const SizedBox(width: 5),
          ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 150),
            child: Text(label, maxLines: 1, overflow: TextOverflow.ellipsis),
          ),
          const SizedBox(width: 3),
          Icon(
            isExpanded
                ? Icons.keyboard_arrow_up_rounded
                : Icons.keyboard_arrow_down_rounded,
            size: 14,
            color: colorScheme.onSurfaceVariant,
          ),
        ],
      ),
    );
  }
}

class _WorkspaceContributionAvatarStrip extends StatelessWidget {
  /// Creates the compact avatar strip for successfully loaded contribution usage.
  const _WorkspaceContributionAvatarStrip({required this.contributions});

  final WorkspaceContributionsState contributions;

  /// Preserves catalog order for equal counts without assigning unused sections.
  @override
  Widget build(BuildContext context) {
    final state = contributions;
    if (state is! WorkspaceContributionsReady || state.usages.isEmpty) {
      return const SizedBox.shrink();
    }
    return Wrap(
      spacing: 4,
      runSpacing: 4,
      children: <Widget>[
        for (final usage in state.usages)
          _WorkspaceContributionAvatar(
            key: ValueKey(usage.identity),
            usage: usage,
          ),
      ],
    );
  }
}

class _WorkspaceContributionStateNotice extends StatelessWidget {
  /// Creates a visible status for pending or failed contribution requests.
  const _WorkspaceContributionStateNotice({required this.contributions});

  final WorkspaceContributionsState contributions;

  /// Displays the real failure instead of presenting an empty successful catalog.
  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final Widget status;
    switch (contributions) {
      case WorkspaceContributionsReady():
        return const SizedBox.shrink();
      case WorkspaceContributionsLoading():
        status = _WorkspaceInlineState(
          icon: Icons.sync,
          text: '正在读取工作区贡献项',
          progress: true,
          color: colors.onSurfaceVariant,
        );
      case WorkspaceContributionsFailed(:final error):
        status = _WorkspaceInlineState(
          icon: Icons.error_outline,
          text: '工作区贡献项加载失败：$error',
          color: colors.error,
        );
    }
    return Padding(padding: const EdgeInsets.only(top: 8), child: status);
  }
}

class _WorkspaceContributionAvatar extends StatefulWidget {
  /// Creates one plugin-owned preview affordance in the workspace title area.
  const _WorkspaceContributionAvatar({super.key, required this.usage});

  final WorkspaceContributionUsage usage;

  /// Owns the preview request so repeated taps cannot start overlapping dialogs.
  @override
  State<_WorkspaceContributionAvatar> createState() =>
      _WorkspaceContributionAvatarState();
}

class _WorkspaceContributionAvatarState
    extends State<_WorkspaceContributionAvatar> {
  static const _clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
  bool _presenting = false;

  /// Delegates the untouched preview action to its registered owning route.
  Future<void> _presentPreview() async {
    if (_presenting) {
      return;
    }
    setState(() => _presenting = true);
    try {
      await widget.usage.preview.present(context: context, clients: _clients);
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('贡献项预览失败：$error')));
    } finally {
      if (mounted) {
        setState(() => _presenting = false);
      }
    }
  }

  /// Keeps the original circular avatar dimensions and exposes a labeled action.
  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final usage = widget.usage;
    final label = '${usage.title} · ${usage.conversationCount} 个对话';
    return Tooltip(
      message: label,
      waitDuration: const Duration(milliseconds: 450),
      child: Semantics(
        button: true,
        label: label,
        enabled: !_presenting,
        child: Material(
          shape: CircleBorder(
            side: BorderSide(
              color: colors.surface.withValues(alpha: 0.90),
              width: 1.4,
            ),
          ),
          color: colors.primaryContainer,
          clipBehavior: Clip.antiAlias,
          child: InkWell(
            customBorder: const CircleBorder(),
            onTap: _presenting ? null : () => unawaited(_presentPreview()),
            child: SizedBox.square(
              dimension: 22,
              child: ChatAvatarImage(
                avatarUri: usage.avatarUri,
                fit: BoxFit.cover,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _WorkspaceInlineState extends StatelessWidget {
  /// Creates a compact inline state row for overview subsections.
  const _WorkspaceInlineState({
    required this.icon,
    required this.text,
    required this.color,
    this.progress = false,
  });

  final IconData icon;
  final String text;
  final Color color;
  final bool progress;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      children: <Widget>[
        if (progress)
          SizedBox.square(
            dimension: 14,
            child: CircularProgressIndicator(strokeWidth: 1.8, color: color),
          )
        else
          Icon(icon, size: 15, color: color),
        const SizedBox(width: 7),
        Expanded(
          child: Text(
            text,
            maxLines: 3,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.bodySmall?.copyWith(color: color),
          ),
        ),
      ],
    );
  }
}
