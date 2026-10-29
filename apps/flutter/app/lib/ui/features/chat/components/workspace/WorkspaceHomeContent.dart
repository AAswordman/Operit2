// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';

import '../../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../WorkspaceShell.dart';
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
    this.onOpenDiff,
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
  final VoidCallback? onOpenDiff;

  @override
  State<WorkspaceHomeContent> createState() => _WorkspaceHomeContentState();
}

class _WorkspaceHomeContentState extends State<WorkspaceHomeContent> {
  /// Opens review through the owning workspace navigation callback.
  void _handleOpenDiff() {
    if (widget.onOpenDiff != null) {
      widget.onOpenDiff!();
    } else {
      WorkspaceShell.session.openDiffTab();
    }
  }

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
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return ListView(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      children: <Widget>[
        Card.filled(
          margin: EdgeInsets.zero,
          color: colorScheme.surfaceContainerLow,
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
            child: _WorkspaceHeaderCard(
              hasBoundWorkspace: hasBoundWorkspace,
              title: hasBoundWorkspace ? title : '当前对话未绑定工作区',
              workspaceUsage: widget.workspaceUsage,
              onOpenFolder: widget.onOpenFolder,
              onAddFolder: widget.onAddFolder,
              onCreateWorkspace: widget.onCreateWorkspace,
              onChooseExistingWorkspace: widget.onChooseExistingWorkspace,
              onUnbindWorkspace: widget.onUnbindWorkspace,
            ),
          ),
        ),
        if (hasBoundWorkspace) ...<Widget>[
          const SizedBox(height: 12),
          _WorkspaceReviewHeroCard(
            onOpenDiff: _handleOpenDiff,
          ),
          const SizedBox(height: 12),
          _WorkspaceWorkstationGrid(
            terminalSessionCountListenable:
                widget.terminalSessionCountListenable,
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
          if (boundWorkspacePath != null && boundWorkspacePath.isNotEmpty) ...<Widget>[
            const SizedBox(height: 12),
            _WorkspaceRootPathCard(
              workspacePath: boundWorkspacePath,
            ),
          ],
        ],
      ],
    );
  }
}

class _WorkspaceReviewHeroCard extends StatelessWidget {
  const _WorkspaceReviewHeroCard({required this.onOpenDiff});

  final VoidCallback onOpenDiff;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final accentColor = colorScheme.primary;

    return Card.outlined(
      margin: EdgeInsets.zero,
      color: colorScheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(10),
        side: BorderSide(color: colorScheme.outlineVariant),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        child: Row(
          children: <Widget>[
            Container(
              width: 36,
              height: 36,
              decoration: BoxDecoration(
                color: accentColor.withValues(alpha: 0.15),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(
                  color: accentColor.withValues(alpha: 0.3),
                ),
              ),
              child: Icon(
                Icons.difference_outlined,
                size: 20,
                color: accentColor,
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Text(
                    '代码变更审查',
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                      color: colorScheme.onSurface,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    '查看工作区代码改动、行级对比与文件审查',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant.withValues(alpha: 0.8),
                      fontSize: 11.5,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            FilledButton.tonal(
              onPressed: onOpenDiff,
              style: FilledButton.styleFrom(
                backgroundColor: colorScheme.surfaceContainerHigh,
                foregroundColor: colorScheme.onSurface,
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(7),
                  side: BorderSide(
                    color: colorScheme.outlineVariant.withValues(alpha: 0.4),
                  ),
                ),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Text(
                    '查看变更',
                    style: TextStyle(
                      fontWeight: FontWeight.w600,
                      fontSize: 12,
                      color: colorScheme.onSurface,
                    ),
                  ),
                  const SizedBox(width: 4),
                  Icon(
                    Icons.arrow_forward_rounded,
                    size: 14,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _WorkspaceWorkstationGrid extends StatelessWidget {
  /// Keeps terminal and browser controls in their own compact cards.
  const _WorkspaceWorkstationGrid({
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
      color: colorScheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(10),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(Icons.terminal_rounded, size: 16, color: colorScheme.onSurfaceVariant),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    '终端工件 (PTY)',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                ValueListenableBuilder<int>(
                  valueListenable: terminalCountListenable,
                  builder: (context, count, _) => count == 0
                      ? const SizedBox.shrink()
                      : Text(
                          '$count 个运行中',
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                            fontSize: 10,
                          ),
                        ),
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
                final sessions = sessionsListenable?.value ??
                    const <WorkspaceTerminalSessionInfo>[];
                if (sessions.isEmpty && terminalCountListenable.value == 0) {
                  return Padding(
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    child: Text(
                      '暂无运行中的终端会话',
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant.withValues(alpha: 0.65),
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
                          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 5),
                          decoration: BoxDecoration(
                            color: colorScheme.surface.withValues(alpha: 0.5),
                            borderRadius: BorderRadius.circular(6),
                            border: Border.all(
                              color: colorScheme.outlineVariant.withValues(alpha: 0.15),
                            ),
                          ),
                          child: Row(
                            children: <Widget>[
                              Expanded(
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: <Widget>[
                                    Text(
                                      session.title.isNotEmpty ? session.title : '终端会话',
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                      style: theme.textTheme.bodySmall?.copyWith(
                                        color: colorScheme.onSurface,
                                        fontWeight: FontWeight.w600,
                                        fontSize: 11,
                                      ),
                                    ),
                                    if (session.workingDir.isNotEmpty)
                                      Text(
                                        session.workingDir,
                                        maxLines: 1,
                                        overflow: TextOverflow.ellipsis,
                                        style: theme.textTheme.labelSmall?.copyWith(
                                          color: colorScheme.onSurfaceVariant,
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
                                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
                                  child: Text(
                                    '切换',
                                    style: TextStyle(
                                      color: accentColor,
                                      fontSize: 11,
                                      fontWeight: FontWeight.w600,
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
            const SizedBox(height: 6),
            SizedBox(
              width: double.infinity,
              child: OutlinedButton(
                onPressed: onOpenTerminal,
                style: OutlinedButton.styleFrom(
                  foregroundColor: colorScheme.onSurface,
                  side: BorderSide(
                    color: colorScheme.outlineVariant.withValues(alpha: 0.3),
                  ),
                  padding: const EdgeInsets.symmetric(vertical: 8),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(7),
                  ),
                ),
                child: const Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: <Widget>[
                    Icon(Icons.add_rounded, size: 14),
                    SizedBox(width: 4),
                    Text('新建终端会话', style: TextStyle(fontSize: 11.5, fontWeight: FontWeight.w600)),
                  ],
                ),
              ),
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
      color: colorScheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(10),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(Icons.public_rounded, size: 16, color: colorScheme.onSurfaceVariant),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    '浏览器工件 (WebView)',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                ValueListenableBuilder<int>(
                  valueListenable: browserCountListenable,
                  builder: (context, count, _) => count == 0
                      ? const SizedBox.shrink()
                      : Text(
                          '$count 个打开',
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                            fontSize: 10,
                          ),
                        ),
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
                        color: colorScheme.onSurfaceVariant.withValues(alpha: 0.65),
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
                          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 5),
                          decoration: BoxDecoration(
                            color: colorScheme.surface.withValues(alpha: 0.5),
                            borderRadius: BorderRadius.circular(6),
                            border: Border.all(
                              color: colorScheme.outlineVariant.withValues(alpha: 0.15),
                            ),
                          ),
                          child: Row(
                            children: <Widget>[
                              Expanded(
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: <Widget>[
                                    Text(
                                      tab.title.isNotEmpty ? tab.title : '浏览器页面',
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                      style: theme.textTheme.bodySmall?.copyWith(
                                        color: colorScheme.onSurface,
                                        fontWeight: FontWeight.w600,
                                        fontSize: 11,
                                      ),
                                    ),
                                    if (tab.url.isNotEmpty)
                                      Text(
                                        tab.url,
                                        maxLines: 1,
                                        overflow: TextOverflow.ellipsis,
                                        style: theme.textTheme.labelSmall?.copyWith(
                                          color: colorScheme.onSurfaceVariant,
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
                                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
                                  child: Text(
                                    '切换',
                                    style: TextStyle(
                                      color: accentColor,
                                      fontSize: 11,
                                      fontWeight: FontWeight.w600,
                                    ),
                                  ),
                                ),
                              ),
                              if (onCloseBrowserSession != null)
                                InkWell(
                                  onTap: () => onCloseBrowserSession?.call(tab.id),
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
            const SizedBox(height: 6),
            SizedBox(
              width: double.infinity,
              child: OutlinedButton(
                onPressed: onOpenBrowser,
                style: OutlinedButton.styleFrom(
                  foregroundColor: colorScheme.onSurface,
                  side: BorderSide(
                    color: colorScheme.outlineVariant.withValues(alpha: 0.3),
                  ),
                  padding: const EdgeInsets.symmetric(vertical: 8),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(7),
                  ),
                ),
                child: const Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: <Widget>[
                    Icon(Icons.add_rounded, size: 14),
                    SizedBox(width: 4),
                    Text('新建浏览器页面', style: TextStyle(fontSize: 11.5, fontWeight: FontWeight.w600)),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _WorkspaceRootPathCard extends StatelessWidget {
  /// Displays the root path without repeating the folder menu's mount action.
  const _WorkspaceRootPathCard({
    required this.workspacePath,
  });

  final String? workspacePath;

  /// Keeps the original compact path footer.
  @override
  Widget build(BuildContext context) {
    final path = workspacePath?.trim();
    if (path == null || path.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Card.filled(
      margin: EdgeInsets.zero,
      color: colorScheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(10),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        child: Row(
          children: <Widget>[
            Icon(
              Icons.folder_outlined,
              size: 16,
              color: colorScheme.primary,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                path,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                  fontSize: 11.5,
                  fontFamily: 'monospace',
                ),
              ),
            ),

          ],
        ),
      ),
    );
  }
}

class _WorkspaceHeaderCard extends StatelessWidget {
  /// Creates the workspace selector without repeating workstation controls.
  const _WorkspaceHeaderCard({
    required this.hasBoundWorkspace,
    required this.title,
    required this.workspaceUsage,
    required this.onOpenFolder,
    required this.onAddFolder,
    required this.onCreateWorkspace,
    required this.onChooseExistingWorkspace,
    required this.onUnbindWorkspace,
  });

  final bool hasBoundWorkspace;
  final String title;
  final WorkspaceOverviewUsage workspaceUsage;
  final ValueChanged<WorkspaceMountedFolder> onOpenFolder;
  final VoidCallback onAddFolder;
  final VoidCallback onCreateWorkspace;
  final VoidCallback onChooseExistingWorkspace;
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
            if (hasBoundWorkspace) ...<Widget>[
              _WorkspaceFolderMenuAnchor(
                title: title,
                folders: workspaceUsage.mountedFolders,
                loading: workspaceUsage.mountedFoldersLoading,
                errorMessage: workspaceUsage.mountedFoldersError,
                onOpenFolder: onOpenFolder,
                onAddFolder: onAddFolder,
                onChooseExistingWorkspace: onChooseExistingWorkspace,
                onUnbindWorkspace: onUnbindWorkspace,
              ),
              _WorkspaceContributionAvatarStrip(
                contributions: workspaceUsage.contributions,
              ),
            ] else ...<Widget>[
              Text(title, style: Theme.of(context).textTheme.bodySmall),
              _WorkspaceCapsuleButton(
                icon: Icons.add_rounded,
                label: '创建工作区',
                isPrimary: true,
                onTap: onCreateWorkspace,
              ),
              _WorkspaceCapsuleButton(
                icon: Icons.folder_open_rounded,
                label: '选择工作区',
                isPrimary: false,
                onTap: onChooseExistingWorkspace,
              ),
            ],
          ],
        ),
        if (hasBoundWorkspace)
          _WorkspaceContributionStateNotice(
            contributions: workspaceUsage.contributions,
          ),
      ],
    );
  }
}

class _WorkspaceFolderMenuItemTile extends StatelessWidget {
  const _WorkspaceFolderMenuItemTile({
    required this.folder,
    required this.onTap,
  });

  final WorkspaceMountedFolder folder;
  final VoidCallback onTap;

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
                color: colorScheme.primary,
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
                        fontWeight: FontWeight.w600,
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

class _WorkspaceFolderMenuAnchor extends StatefulWidget {
  /// Creates the sole menu for mounted folders and workspace management.
  const _WorkspaceFolderMenuAnchor({
    required this.title,
    required this.folders,
    required this.loading,
    required this.errorMessage,
    required this.onOpenFolder,
    required this.onAddFolder,
    required this.onChooseExistingWorkspace,
    required this.onUnbindWorkspace,
  });

  final String title;
  final List<WorkspaceMountedFolder> folders;
  final bool loading;
  final String? errorMessage;
  final ValueChanged<WorkspaceMountedFolder> onOpenFolder;
  final VoidCallback onAddFolder;
  final VoidCallback onChooseExistingWorkspace;
  final VoidCallback onUnbindWorkspace;

  /// Owns the workspace menu controller.
  @override
  State<_WorkspaceFolderMenuAnchor> createState() =>
      _WorkspaceFolderMenuAnchorState();
}

class _WorkspaceFolderMenuAnchorState extends State<_WorkspaceFolderMenuAnchor> {
  final MenuController _controller = MenuController();
  bool _isOpen = false;

  /// Builds one compact menu for folder and binding actions.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return MenuAnchor(
      controller: _controller,
      alignmentOffset: const Offset(0, 4),
      onOpen: () => setState(() => _isOpen = true),
      onClose: () => setState(() => _isOpen = false),
      style: MenuStyle(
        backgroundColor: WidgetStatePropertyAll(colorScheme.surfaceContainerHigh),
        padding: const WidgetStatePropertyAll(EdgeInsets.zero),
      ),
      builder: (context, controller, _) => _WorkspaceToolCapsule(
        icon: Icons.work_outline_rounded,
        accentColor: colorScheme.primary,
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
        SizedBox(
          width: 270,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              if (widget.loading)
                const Padding(
                  padding: EdgeInsets.all(12),
                  child: Text('正在加载文件夹…'),
                )
              else if (widget.errorMessage != null)
                Padding(
                  padding: const EdgeInsets.all(12),
                  child: Text(
                    widget.errorMessage!,
                    style: TextStyle(color: colorScheme.error),
                  ),
                )
              else if (widget.folders.isNotEmpty)
                ConstrainedBox(
                  constraints: const BoxConstraints(maxHeight: 240),
                  child: SingleChildScrollView(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        for (final folder in widget.folders)
                          _WorkspaceFolderMenuItemTile(
                            folder: folder,
                            onTap: () {
                              _controller.close();
                              widget.onOpenFolder(folder);
                            },
                          ),
                      ],
                    ),
                  ),
                ),
              const Divider(height: 1),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    _CompactActionButton(
                      icon: Icons.add_rounded,
                      label: '挂载新文件夹',
                      accentColor: colorScheme.onSurface,
                      onTap: () {
                        _controller.close();
                        widget.onAddFolder();
                      },
                    ),
                    _CompactActionButton(
                      icon: Icons.folder_open_rounded,
                      label: '切换工作区',
                      accentColor: colorScheme.onSurface,
                      onTap: () {
                        _controller.close();
                        widget.onChooseExistingWorkspace();
                      },
                    ),
                    _CompactActionButton(
                      icon: Icons.link_off_rounded,
                      label: '解除绑定',
                      accentColor: colorScheme.error,
                      onTap: () {
                        _controller.close();
                        widget.onUnbindWorkspace();
                      },
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _CompactActionButton extends StatelessWidget {
  const _CompactActionButton({
    required this.icon,
    required this.label,
    required this.accentColor,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final Color accentColor;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return TextButton.icon(
      onPressed: onTap,
      style: TextButton.styleFrom(
        foregroundColor: colorScheme.onSurface,
        minimumSize: const Size(double.infinity, 28),
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
        visualDensity: VisualDensity.compact,
        textStyle: theme.textTheme.labelSmall?.copyWith(fontSize: 11.5),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
      ),
      icon: Icon(icon, size: 13, color: accentColor),
      label: Text(label),
    );
  }
}

class _WorkspaceCapsuleButton extends StatelessWidget {
  const _WorkspaceCapsuleButton({
    required this.icon,
    required this.label,
    required this.isPrimary,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final bool isPrimary;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    final bg = isPrimary
        ? colorScheme.primary
        : colorScheme.surfaceContainerHighest.withValues(alpha: 0.40);
    final fg = isPrimary ? colorScheme.onPrimary : colorScheme.onSurface;
    final border = isPrimary
        ? null
        : Border.all(
            color: colorScheme.outlineVariant.withValues(alpha: 0.35),
          );

    return Material(
      color: bg,
      borderRadius: BorderRadius.circular(999),
      child: InkWell(
        borderRadius: BorderRadius.circular(999),
        onTap: onTap,
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(999),
            border: border,
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(icon, size: 13, color: fg),
              const SizedBox(width: 4),
              Text(
                label,
                style: theme.textTheme.labelSmall?.copyWith(
                  color: fg,
                  fontWeight: FontWeight.w600,
                  fontSize: 11.5,
                ),
              ),
            ],
          ),
        ),
      ),
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
        foregroundColor: isExpanded ? colorScheme.primary : colorScheme.onSurface,
        backgroundColor: isExpanded
            ? colorScheme.primaryContainer
            : colorScheme.surfaceContainerHigh,
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
            isExpanded ? Icons.keyboard_arrow_up_rounded : Icons.keyboard_arrow_down_rounded,
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
              child: _WorkspaceContributionAvatarImage(
                avatarUri: usage.avatarUri,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _WorkspaceContributionAvatarImage extends StatefulWidget {
  /// Creates a display-only image backed by the existing runtime asset host.
  const _WorkspaceContributionAvatarImage({required this.avatarUri});

  final String? avatarUri;

  /// Retains the image request until the plugin changes its display URI.
  @override
  State<_WorkspaceContributionAvatarImage> createState() =>
      _WorkspaceContributionAvatarImageState();
}

class _WorkspaceContributionAvatarImageState
    extends State<_WorkspaceContributionAvatarImage> {
  static const _clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
  Future<Uint8List>? _bytes;

  /// Starts the request for the exact display URI published by the contribution.
  @override
  void initState() {
    super.initState();
    _loadImage();
  }

  /// Replaces image requests when the authoritative display URI changes.
  @override
  void didUpdateWidget(covariant _WorkspaceContributionAvatarImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.avatarUri != widget.avatarUri) {
      _loadImage();
    }
  }

  /// Reads the runtime asset without searching character stores or platform paths.
  Future<Uint8List> _readImage(String uri) async {
    final encoded = await _clients.repositoryRuntimeStorageRepository
        .readBase64(path: uri);
    if (encoded == null) {
      throw StateError('The contribution avatar asset is missing: $uri');
    }
    return base64Decode(encoded);
  }

  /// Uses explicit image absence only when the catalog publishes a null URI.
  void _loadImage() {
    final uri = widget.avatarUri;
    _bytes = uri == null ? null : _readImage(uri);
  }

  /// Displays loading and asset errors without synthesizing an avatar identity.
  @override
  Widget build(BuildContext context) {
    final bytes = _bytes;
    if (bytes == null) {
      return Icon(
        Icons.extension_outlined,
        size: 14,
        color: Theme.of(context).colorScheme.onPrimaryContainer,
      );
    }
    return FutureBuilder<Uint8List>(
      key: ValueKey(widget.avatarUri),
      future: bytes,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return _imageError(snapshot.error!);
        }
        final data = snapshot.data;
        if (data == null) {
          return const Padding(
            padding: EdgeInsets.all(5),
            child: CircularProgressIndicator(strokeWidth: 1.5),
          );
        }
        return Image.memory(
          data,
          fit: BoxFit.cover,
          width: 22,
          height: 22,
          errorBuilder: (context, error, stackTrace) => _imageError(error),
        );
      },
    );
  }

  /// Makes failed avatar assets explicit rather than substituting another image.
  Widget _imageError(Object error) {
    return Tooltip(
      message: '贡献项头像加载失败：$error',
      child: Icon(
        Icons.broken_image_outlined,
        size: 14,
        color: Theme.of(context).colorScheme.error,
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
