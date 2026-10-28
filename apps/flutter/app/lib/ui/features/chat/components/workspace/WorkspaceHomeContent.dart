// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';

import '../../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../theme/OperitGlassSurface.dart';
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
  void _handleOpenDiff() {
    if (widget.onOpenDiff != null) {
      widget.onOpenDiff!();
    } else {
      WorkspaceShell.session.openDiffTab();
    }
  }

  /// Builds the workspace home tab with modern hierarchy and unified tool actions.
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
        Container(
          decoration: BoxDecoration(
            color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.35),
            borderRadius: BorderRadius.circular(10),
            border: Border.all(
              color: colorScheme.outlineVariant.withValues(alpha: 0.25),
            ),
          ),
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
          child: _WorkspaceHeaderCard(
            hasBoundWorkspace: hasBoundWorkspace,
            title: hasBoundWorkspace ? title : '当前对话未绑定工作区',
            workspaceUsage: widget.workspaceUsage,
            terminalSessionCountListenable: widget.terminalSessionCountListenable,
            browserSessionCountListenable: widget.browserSessionCountListenable,
            terminalSessionsListenable: widget.terminalSessionsListenable,
            onOpenFolder: widget.onOpenFolder,
            onAddFolder: widget.onAddFolder,
            onOpenTerminal: widget.onOpenTerminal,
            onOpenTerminalSessions: widget.onOpenTerminalSessions,
            onOpenBrowser: widget.onOpenBrowser,
            onOpenBrowserSessions: widget.onOpenBrowserSessions,
            onSelectTerminalSession: widget.onSelectTerminalSession,
            onCloseTerminalSession: widget.onCloseTerminalSession,
            onSelectBrowserTab: widget.onSelectBrowserTab,
            onCloseBrowserSession: widget.onCloseBrowserSession,
            onOpenDiff: _handleOpenDiff,
            onCreateWorkspace: widget.onCreateWorkspace,
            onChooseExistingWorkspace: widget.onChooseExistingWorkspace,
            onUnbindWorkspace: widget.onUnbindWorkspace,
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
            onOpenBrowser: widget.onOpenBrowser,
            onSelectTerminalSession: widget.onSelectTerminalSession,
            onCloseTerminalSession: widget.onCloseTerminalSession,
            onSelectBrowserTab: widget.onSelectBrowserTab,
            onCloseBrowserSession: widget.onCloseBrowserSession,
          ),
          if (boundWorkspacePath != null && boundWorkspacePath.isNotEmpty) ...<Widget>[
            const SizedBox(height: 12),
            _WorkspaceRootPathCard(
              workspacePath: boundWorkspacePath,
              onAddFolder: widget.onAddFolder,
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
    const accentColor = Color(0xFFF59E0B);

    return Container(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.28),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.35),
        ),
      ),
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
            child: const Icon(
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
                    fontWeight: FontWeight.w700,
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
    );
  }
}

class _WorkspaceWorkstationGrid extends StatelessWidget {
  const _WorkspaceWorkstationGrid({
    required this.terminalSessionCountListenable,
    required this.browserSessionCountListenable,
    required this.terminalSessionsListenable,
    required this.onOpenTerminal,
    required this.onOpenBrowser,
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
  final VoidCallback onOpenBrowser;
  final ValueChanged<WorkspaceTerminalSessionInfo>? onSelectTerminalSession;
  final ValueChanged<WorkspaceTerminalSessionInfo>? onCloseTerminalSession;
  final ValueChanged<WorkspaceBrowserTabState>? onSelectBrowserTab;
  final ValueChanged<String>? onCloseBrowserSession;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final isNarrow = constraints.maxWidth < 520;
        final terminalCard = _TerminalWorkstationCard(
          terminalCountListenable: terminalSessionCountListenable,
          sessionsListenable: terminalSessionsListenable,
          onOpenTerminal: onOpenTerminal,
          onSelectSession: onSelectTerminalSession,
          onCloseSession: onCloseTerminalSession,
        );
        final browserCard = _BrowserWorkstationCard(
          browserCountListenable: browserSessionCountListenable,
          onOpenBrowser: onOpenBrowser,
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
  const _TerminalWorkstationCard({
    required this.terminalCountListenable,
    required this.sessionsListenable,
    required this.onOpenTerminal,
    required this.onSelectSession,
    required this.onCloseSession,
  });

  final ValueListenable<int> terminalCountListenable;
  final ValueListenable<List<WorkspaceTerminalSessionInfo>>? sessionsListenable;
  final VoidCallback onOpenTerminal;
  final ValueChanged<WorkspaceTerminalSessionInfo>? onSelectSession;
  final ValueChanged<WorkspaceTerminalSessionInfo>? onCloseSession;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    const accentColor = Color(0xFF10B981);

    return Container(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.28),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.25),
        ),
      ),
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: <Widget>[
              Row(
                children: <Widget>[
                  const Icon(
                    Icons.terminal_rounded,
                    size: 16,
                    color: accentColor,
                  ),
                  const SizedBox(width: 6),
                  Text(
                    '终端工件 (PTY)',
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w700,
                      color: colorScheme.onSurface,
                    ),
                  ),
                ],
              ),
              ValueListenableBuilder<int>(
                valueListenable: terminalCountListenable,
                builder: (context, count, _) {
                  return Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: count > 0
                          ? accentColor.withValues(alpha: 0.15)
                          : colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(
                        color: count > 0
                            ? accentColor.withValues(alpha: 0.3)
                            : colorScheme.outlineVariant.withValues(alpha: 0.2),
                      ),
                    ),
                    child: Text(
                      count > 0 ? '$count 个运行中' : '就绪',
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: count > 0 ? accentColor : colorScheme.onSurfaceVariant,
                        fontSize: 10,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  );
                },
              ),
            ],
          ),
          const SizedBox(height: 10),
          ListenableBuilder(
            listenable: sessionsListenable ??
                ValueNotifier<List<WorkspaceTerminalSessionInfo>>(const []),
            builder: (context, _) {
              final sessions = sessionsListenable?.value ??
                  const <WorkspaceTerminalSessionInfo>[];
              if (sessions.isEmpty) {
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
    );
  }
}

class _BrowserWorkstationCard extends StatelessWidget {
  const _BrowserWorkstationCard({
    required this.browserCountListenable,
    required this.onOpenBrowser,
    required this.onSelectBrowserTab,
    required this.onCloseBrowserSession,
  });

  final ValueListenable<int> browserCountListenable;
  final VoidCallback onOpenBrowser;
  final ValueChanged<WorkspaceBrowserTabState>? onSelectBrowserTab;
  final ValueChanged<String>? onCloseBrowserSession;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final accentColor = colorScheme.primary;
    final browserStore = WorkspaceBrowserViewStore.instance;

    return Container(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.28),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.25),
        ),
      ),
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: <Widget>[
              Row(
                children: <Widget>[
                  Icon(
                    Icons.public_rounded,
                    size: 16,
                    color: accentColor,
                  ),
                  const SizedBox(width: 6),
                  Text(
                    '浏览器工件 (WebView)',
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w700,
                      color: colorScheme.onSurface,
                    ),
                  ),
                ],
              ),
              ValueListenableBuilder<int>(
                valueListenable: browserCountListenable,
                builder: (context, count, _) {
                  return Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: count > 0
                          ? accentColor.withValues(alpha: 0.15)
                          : colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(
                        color: count > 0
                            ? accentColor.withValues(alpha: 0.3)
                            : colorScheme.outlineVariant.withValues(alpha: 0.2),
                      ),
                    ),
                    child: Text(
                      count > 0 ? '$count 个打开' : '就绪',
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: count > 0 ? accentColor : colorScheme.onSurfaceVariant,
                        fontSize: 10,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  );
                },
              ),
            ],
          ),
          const SizedBox(height: 10),
          ListenableBuilder(
            listenable: browserStore,
            builder: (context, _) {
              final tabs = browserStore.tabs;
              if (tabs.isEmpty) {
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
    );
  }
}

class _WorkspaceRootPathCard extends StatelessWidget {
  const _WorkspaceRootPathCard({
    required this.workspacePath,
    required this.onAddFolder,
  });

  final String? workspacePath;
  final VoidCallback onAddFolder;

  @override
  Widget build(BuildContext context) {
    final path = workspacePath?.trim();
    if (path == null || path.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Container(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.22),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.2),
        ),
      ),
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
          const SizedBox(width: 8),
          InkWell(
            onTap: onAddFolder,
            borderRadius: BorderRadius.circular(4),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Icon(Icons.add_rounded, size: 13, color: colorScheme.primary),
                  const SizedBox(width: 2),
                  Text(
                    '挂载新目录',
                    style: TextStyle(
                      color: colorScheme.primary,
                      fontSize: 11,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _WorkspaceHeaderCard extends StatelessWidget {
  const _WorkspaceHeaderCard({
    required this.hasBoundWorkspace,
    required this.title,
    required this.workspaceUsage,
    required this.terminalSessionCountListenable,
    required this.browserSessionCountListenable,
    required this.terminalSessionsListenable,
    required this.onOpenFolder,
    required this.onAddFolder,
    required this.onOpenTerminal,
    required this.onOpenTerminalSessions,
    required this.onOpenBrowser,
    required this.onOpenBrowserSessions,
    required this.onSelectTerminalSession,
    required this.onCloseTerminalSession,
    required this.onSelectBrowserTab,
    required this.onCloseBrowserSession,
    required this.onOpenDiff,
    required this.onCreateWorkspace,
    required this.onChooseExistingWorkspace,
    required this.onUnbindWorkspace,
  });

  final bool hasBoundWorkspace;
  final String title;
  final WorkspaceOverviewUsage workspaceUsage;
  final ValueListenable<int> terminalSessionCountListenable;
  final ValueListenable<int> browserSessionCountListenable;
  final ValueListenable<List<WorkspaceTerminalSessionInfo>>?
  terminalSessionsListenable;
  final ValueChanged<WorkspaceMountedFolder> onOpenFolder;
  final VoidCallback onAddFolder;
  final VoidCallback onOpenTerminal;
  final VoidCallback onOpenTerminalSessions;
  final VoidCallback onOpenBrowser;
  final VoidCallback onOpenBrowserSessions;
  final ValueChanged<WorkspaceTerminalSessionInfo>? onSelectTerminalSession;
  final ValueChanged<WorkspaceTerminalSessionInfo>? onCloseTerminalSession;
  final ValueChanged<WorkspaceBrowserTabState>? onSelectBrowserTab;
  final ValueChanged<String>? onCloseBrowserSession;
  final VoidCallback onOpenDiff;
  final VoidCallback onCreateWorkspace;
  final VoidCallback onChooseExistingWorkspace;
  final VoidCallback onUnbindWorkspace;

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
            _WorkspaceFolderMenuAnchor(
              title: title,
              hasBoundWorkspace: hasBoundWorkspace,
              folders: workspaceUsage.mountedFolders,
              loading: workspaceUsage.mountedFoldersLoading,
              errorMessage: workspaceUsage.mountedFoldersError,
              onOpenFolder: onOpenFolder,
              onAddFolder: onAddFolder,
              onCreateWorkspace: onCreateWorkspace,
              onChooseExistingWorkspace: onChooseExistingWorkspace,
              onUnbindWorkspace: onUnbindWorkspace,
            ),
            if (hasBoundWorkspace)
              _WorkspaceContributionAvatarStrip(
                contributions: workspaceUsage.contributions,
              ),
            if (!hasBoundWorkspace) ...<Widget>[
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
            ValueListenableBuilder<int>(
              valueListenable: terminalSessionCountListenable,
              builder: (context, terminalCount, _) {
                return _TerminalMenuAnchor(
                  terminalCount: terminalCount,
                  terminalSessionsListenable: terminalSessionsListenable,
                  onSelectTerminalSession: onSelectTerminalSession,
                  onCloseTerminalSession: onCloseTerminalSession,
                  onOpenTerminal: onOpenTerminal,
                  onOpenTerminalSessions: onOpenTerminalSessions,
                );
              },
            ),
            ValueListenableBuilder<int>(
              valueListenable: browserSessionCountListenable,
              builder: (context, browserCount, _) {
                return _BrowserMenuAnchor(
                  browserCount: browserCount,
                  onSelectBrowserTab: onSelectBrowserTab,
                  onCloseBrowserSession: onCloseBrowserSession,
                  onOpenBrowser: onOpenBrowser,
                  onOpenBrowserSessions: onOpenBrowserSessions,
                );
              },
            ),
            _WorkspaceActionCapsule(
              icon: Icons.difference_outlined,
              accentColor: const Color(0xFFF59E0B),
              label: '查看变更',
              onTap: onOpenDiff,
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
  const _WorkspaceFolderMenuAnchor({
    required this.title,
    required this.hasBoundWorkspace,
    required this.folders,
    required this.loading,
    required this.errorMessage,
    required this.onOpenFolder,
    required this.onAddFolder,
    required this.onCreateWorkspace,
    required this.onChooseExistingWorkspace,
    this.onUnbindWorkspace,
  });

  final String title;
  final bool hasBoundWorkspace;
  final List<WorkspaceMountedFolder> folders;
  final bool loading;
  final String? errorMessage;
  final ValueChanged<WorkspaceMountedFolder> onOpenFolder;
  final VoidCallback onAddFolder;
  final VoidCallback onCreateWorkspace;
  final VoidCallback onChooseExistingWorkspace;
  final VoidCallback? onUnbindWorkspace;

  @override
  State<_WorkspaceFolderMenuAnchor> createState() =>
      _WorkspaceFolderMenuAnchorState();
}

class _WorkspaceFolderMenuAnchorState
    extends State<_WorkspaceFolderMenuAnchor> {
  final MenuController _controller = MenuController();
  bool _isOpen = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final accentColor = colorScheme.primary;

    return MenuAnchor(
      controller: _controller,
      alignmentOffset: const Offset(0, 4),
      onOpen: () => setState(() => _isOpen = true),
      onClose: () => setState(() => _isOpen = false),
      style: MenuStyle(
        backgroundColor: WidgetStatePropertyAll(
          colorScheme.surfaceContainerHigh,
        ),
        surfaceTintColor: const WidgetStatePropertyAll(Colors.transparent),
        elevation: const WidgetStatePropertyAll(8),
        shadowColor: WidgetStatePropertyAll(
          Colors.black.withValues(alpha: 0.45),
        ),
        padding: const WidgetStatePropertyAll(EdgeInsets.zero),
        shape: WidgetStatePropertyAll(
          RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(8),
            side: BorderSide(
              color: colorScheme.outlineVariant.withValues(alpha: 0.3),
            ),
          ),
        ),
      ),
      builder: (context, controller, _) {
        return _WorkspaceToolCapsule(
          icon: widget.hasBoundWorkspace
              ? Icons.work_outline_rounded
              : Icons.work_off_outlined,
          accentColor: widget.hasBoundWorkspace
              ? accentColor
              : colorScheme.onSurfaceVariant,
          label: widget.title,
          isExpanded: _isOpen,
          hasActiveSessions: widget.hasBoundWorkspace,
          onTap: () {
            if (_controller.isOpen) {
              _controller.close();
            } else {
              _controller.open();
            }
          },
        );
      },
      menuChildren: <Widget>[
        SizedBox(
          width: 270,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              if (!widget.hasBoundWorkspace) ...<Widget>[
                Padding(
                  padding: const EdgeInsets.fromLTRB(12, 10, 12, 8),
                  child: Row(
                    children: <Widget>[
                      Icon(
                        Icons.info_outline_rounded,
                        size: 14,
                        color: colorScheme.onSurfaceVariant.withValues(
                          alpha: 0.7,
                        ),
                      ),
                      const SizedBox(width: 7),
                      Text(
                        '当前对话未绑定工作区',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant.withValues(
                            alpha: 0.75,
                          ),
                          fontSize: 11.5,
                        ),
                      ),
                    ],
                  ),
                ),
              ] else if (widget.loading) ...<Widget>[
                Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 10,
                  ),
                  child: Row(
                    children: <Widget>[
                      const SizedBox.square(
                        dimension: 13,
                        child: CircularProgressIndicator(strokeWidth: 1.6),
                      ),
                      const SizedBox(width: 8),
                      Text(
                        '正在读取挂载文件夹...',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant.withValues(
                            alpha: 0.75,
                          ),
                          fontSize: 11.5,
                        ),
                      ),
                    ],
                  ),
                ),
              ] else if (widget.errorMessage != null &&
                  widget.errorMessage!.trim().isNotEmpty) ...<Widget>[
                Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 10,
                  ),
                  child: Row(
                    children: <Widget>[
                      Icon(
                        Icons.error_outline_rounded,
                        size: 14,
                        color: colorScheme.error,
                      ),
                      const SizedBox(width: 7),
                      Expanded(
                        child: Text(
                          widget.errorMessage!.trim(),
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: colorScheme.error,
                            fontSize: 11.5,
                          ),
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                ),
              ] else if (widget.folders.isEmpty) ...<Widget>[
                Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 10,
                  ),
                  child: Row(
                    children: <Widget>[
                      Icon(
                        Icons.folder_off_outlined,
                        size: 14,
                        color: colorScheme.onSurfaceVariant.withValues(
                          alpha: 0.7,
                        ),
                      ),
                      const SizedBox(width: 7),
                      Text(
                        '暂无挂载文件夹',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant.withValues(
                            alpha: 0.75,
                          ),
                          fontSize: 11.5,
                        ),
                      ),
                    ],
                  ),
                ),
              ] else ...<Widget>[
                ConstrainedBox(
                  constraints: const BoxConstraints(maxHeight: 220),
                  child: SingleChildScrollView(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        Padding(
                          padding: const EdgeInsets.fromLTRB(10, 8, 10, 4),
                          child: Text(
                            '挂载文件夹 (${widget.folders.length})',
                            style: theme.textTheme.labelSmall?.copyWith(
                              color: colorScheme.onSurfaceVariant.withValues(
                                alpha: 0.8,
                              ),
                              fontWeight: FontWeight.w600,
                              fontSize: 11,
                            ),
                          ),
                        ),
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
              ],
              Divider(
                height: 1,
                thickness: 1,
                color: colorScheme.outlineVariant.withValues(alpha: 0.22),
              ),
              Padding(
                padding: const EdgeInsets.fromLTRB(8, 6, 8, 6),
                child: Column(
                  children: <Widget>[
                    if (widget.hasBoundWorkspace) ...<Widget>[
                      _CompactActionButton(
                        icon: Icons.add_rounded,
                        label: '挂载新文件夹',
                        accentColor: accentColor,
                        onTap: () {
                          _controller.close();
                          widget.onAddFolder();
                        },
                      ),
                      const SizedBox(height: 5),
                      _CompactActionButton(
                        icon: Icons.folder_open_rounded,
                        label: '切换工作区',
                        accentColor: colorScheme.secondary,
                        onTap: () {
                          _controller.close();
                          widget.onChooseExistingWorkspace();
                        },
                      ),
                      if (widget.onUnbindWorkspace != null) ...<Widget>[
                        const SizedBox(height: 5),
                        _CompactActionButton(
                          icon: Icons.link_off_rounded,
                          label: '解除绑定工作区',
                          accentColor: colorScheme.error,
                          onTap: () {
                            _controller.close();
                            widget.onUnbindWorkspace!();
                          },
                        ),
                      ],
                    ] else ...<Widget>[
                      _CompactActionButton(
                        icon: Icons.add_rounded,
                        label: '创建工作区',
                        accentColor: accentColor,
                        onTap: () {
                          _controller.close();
                          widget.onCreateWorkspace();
                        },
                      ),
                      const SizedBox(height: 5),
                      _CompactActionButton(
                        icon: Icons.folder_open_rounded,
                        label: '选择工作区',
                        accentColor: colorScheme.secondary,
                        onTap: () {
                          _controller.close();
                          widget.onChooseExistingWorkspace();
                        },
                      ),
                    ],
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

class _TerminalMenuAnchor extends StatefulWidget {
  const _TerminalMenuAnchor({
    required this.terminalCount,
    required this.terminalSessionsListenable,
    required this.onSelectTerminalSession,
    required this.onCloseTerminalSession,
    required this.onOpenTerminal,
    required this.onOpenTerminalSessions,
  });

  final int terminalCount;
  final ValueListenable<List<WorkspaceTerminalSessionInfo>>?
  terminalSessionsListenable;
  final ValueChanged<WorkspaceTerminalSessionInfo>? onSelectTerminalSession;
  final ValueChanged<WorkspaceTerminalSessionInfo>? onCloseTerminalSession;
  final VoidCallback onOpenTerminal;
  final VoidCallback onOpenTerminalSessions;

  @override
  State<_TerminalMenuAnchor> createState() => _TerminalMenuAnchorState();
}

class _TerminalMenuAnchorState extends State<_TerminalMenuAnchor> {
  final MenuController _controller = MenuController();
  bool _isOpen = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    const accentColor = Color(0xFF10B981);

    return MenuAnchor(
      controller: _controller,
      alignmentOffset: const Offset(0, 4),
      onOpen: () => setState(() => _isOpen = true),
      onClose: () => setState(() => _isOpen = false),
      style: MenuStyle(
        backgroundColor: WidgetStatePropertyAll(
          colorScheme.surfaceContainerHigh,
        ),
        surfaceTintColor: const WidgetStatePropertyAll(Colors.transparent),
        elevation: const WidgetStatePropertyAll(8),
        shadowColor: WidgetStatePropertyAll(
          Colors.black.withValues(alpha: 0.45),
        ),
        padding: const WidgetStatePropertyAll(EdgeInsets.zero),
        shape: WidgetStatePropertyAll(
          RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(8),
            side: BorderSide(
              color: colorScheme.outlineVariant.withValues(alpha: 0.3),
            ),
          ),
        ),
      ),
      builder: (context, controller, _) {
        return _WorkspaceToolCapsule(
          icon: Icons.terminal_rounded,
          accentColor: accentColor,
          label: '${widget.terminalCount} 个终端',
          isExpanded: _isOpen,
          hasActiveSessions: widget.terminalCount > 0,
          onTap: () {
            if (_controller.isOpen) {
              _controller.close();
            } else {
              _controller.open();
            }
          },
        );
      },
      menuChildren: <Widget>[
        SizedBox(
          width: 250,
          child: ListenableBuilder(
            listenable: widget.terminalSessionsListenable ??
                ValueNotifier<List<WorkspaceTerminalSessionInfo>>(const []),
            builder: (context, _) {
              final sessions = widget.terminalSessionsListenable?.value ??
                  const <WorkspaceTerminalSessionInfo>[];

              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  if (sessions.isEmpty)
                    Padding(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 10,
                      ),
                      child: Row(
                        children: <Widget>[
                          Icon(
                            Icons.info_outline_rounded,
                            size: 14,
                            color: colorScheme.onSurfaceVariant.withValues(
                              alpha: 0.7,
                            ),
                          ),
                          const SizedBox(width: 7),
                          Text(
                            '暂无运行中的终端',
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: colorScheme.onSurfaceVariant.withValues(
                                alpha: 0.75,
                              ),
                              fontSize: 11.5,
                            ),
                          ),
                        ],
                      ),
                    )
                  else
                    ConstrainedBox(
                      constraints: const BoxConstraints(maxHeight: 220),
                      child: SingleChildScrollView(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          mainAxisSize: MainAxisSize.min,
                          children: <Widget>[
                            Padding(
                              padding: const EdgeInsets.fromLTRB(10, 8, 10, 4),
                              child: Row(
                                children: <Widget>[
                                  Text(
                                    '运行中的终端 (${sessions.length})',
                                    style: theme.textTheme.labelSmall?.copyWith(
                                      color: colorScheme.onSurfaceVariant
                                          .withValues(alpha: 0.8),
                                      fontWeight: FontWeight.w600,
                                      fontSize: 11,
                                    ),
                                  ),
                                  const Spacer(),
                                  InkWell(
                                    borderRadius: BorderRadius.circular(4),
                                    onTap: () {
                                      _controller.close();
                                      widget.onOpenTerminalSessions();
                                    },
                                    child: Padding(
                                      padding: const EdgeInsets.symmetric(
                                        horizontal: 4,
                                        vertical: 2,
                                      ),
                                      child: Text(
                                        '管理全部',
                                        style: theme.textTheme.labelSmall
                                            ?.copyWith(
                                              color: accentColor,
                                              fontSize: 10.5,
                                              fontWeight: FontWeight.w500,
                                            ),
                                      ),
                                    ),
                                  ),
                                ],
                              ),
                            ),
                            for (final session in sessions)
                              _WorkspaceSessionItemTile(
                                icon: Icons.terminal_rounded,
                                iconColor: accentColor,
                                title: session.title,
                                subtitle: session.workingDir,
                                closeTooltip: '结束终端进程',
                                onTap: () {
                                  _controller.close();
                                  widget.onSelectTerminalSession?.call(session);
                                },
                                onClose: () =>
                                    widget.onCloseTerminalSession?.call(session),
                              ),
                          ],
                        ),
                      ),
                    ),
                  Divider(
                    height: 1,
                    thickness: 1,
                    color: colorScheme.outlineVariant.withValues(alpha: 0.22),
                  ),
                  Padding(
                    padding: const EdgeInsets.fromLTRB(8, 6, 8, 6),
                    child: _CompactActionButton(
                      icon: Icons.add_rounded,
                      label: '新建终端',
                      accentColor: accentColor,
                      onTap: () {
                        _controller.close();
                        widget.onOpenTerminal();
                      },
                    ),
                  ),
                ],
              );
            },
          ),
        ),
      ],
    );
  }
}

class _BrowserMenuAnchor extends StatefulWidget {
  const _BrowserMenuAnchor({
    required this.browserCount,
    required this.onSelectBrowserTab,
    required this.onCloseBrowserSession,
    required this.onOpenBrowser,
    required this.onOpenBrowserSessions,
  });

  final int browserCount;
  final ValueChanged<WorkspaceBrowserTabState>? onSelectBrowserTab;
  final ValueChanged<String>? onCloseBrowserSession;
  final VoidCallback onOpenBrowser;
  final VoidCallback onOpenBrowserSessions;

  @override
  State<_BrowserMenuAnchor> createState() => _BrowserMenuAnchorState();
}

class _BrowserMenuAnchorState extends State<_BrowserMenuAnchor> {
  final MenuController _controller = MenuController();
  bool _isOpen = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final accentColor = colorScheme.primary;
    final browserStore = WorkspaceBrowserViewStore.instance;

    return MenuAnchor(
      controller: _controller,
      alignmentOffset: const Offset(0, 4),
      onOpen: () => setState(() => _isOpen = true),
      onClose: () => setState(() => _isOpen = false),
      style: MenuStyle(
        backgroundColor: WidgetStatePropertyAll(
          colorScheme.surfaceContainerHigh,
        ),
        surfaceTintColor: const WidgetStatePropertyAll(Colors.transparent),
        elevation: const WidgetStatePropertyAll(8),
        shadowColor: WidgetStatePropertyAll(
          Colors.black.withValues(alpha: 0.45),
        ),
        padding: const WidgetStatePropertyAll(EdgeInsets.zero),
        shape: WidgetStatePropertyAll(
          RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(8),
            side: BorderSide(
              color: colorScheme.outlineVariant.withValues(alpha: 0.3),
            ),
          ),
        ),
      ),
      builder: (context, controller, _) {
        return _WorkspaceToolCapsule(
          icon: Icons.public_rounded,
          accentColor: accentColor,
          label: '${widget.browserCount} 个浏览器',
          isExpanded: _isOpen,
          hasActiveSessions: widget.browserCount > 0,
          onTap: () {
            if (_controller.isOpen) {
              _controller.close();
            } else {
              _controller.open();
            }
          },
        );
      },
      menuChildren: <Widget>[
        SizedBox(
          width: 250,
          child: ListenableBuilder(
            listenable: browserStore,
            builder: (context, _) {
              final tabs = browserStore.tabs;

              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  if (tabs.isEmpty)
                    Padding(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 10,
                      ),
                      child: Row(
                        children: <Widget>[
                          Icon(
                            Icons.info_outline_rounded,
                            size: 14,
                            color: colorScheme.onSurfaceVariant.withValues(
                              alpha: 0.7,
                            ),
                          ),
                          const SizedBox(width: 7),
                          Text(
                            '暂无打开的浏览器页面',
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: colorScheme.onSurfaceVariant.withValues(
                                alpha: 0.75,
                              ),
                              fontSize: 11.5,
                            ),
                          ),
                        ],
                      ),
                    )
                  else
                    ConstrainedBox(
                      constraints: const BoxConstraints(maxHeight: 220),
                      child: SingleChildScrollView(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          mainAxisSize: MainAxisSize.min,
                          children: <Widget>[
                            Padding(
                              padding: const EdgeInsets.fromLTRB(10, 8, 10, 4),
                              child: Row(
                                children: <Widget>[
                                  Text(
                                    '运行中的浏览器 (${tabs.length})',
                                    style: theme.textTheme.labelSmall?.copyWith(
                                      color: colorScheme.onSurfaceVariant
                                          .withValues(alpha: 0.8),
                                      fontWeight: FontWeight.w600,
                                      fontSize: 11,
                                    ),
                                  ),
                                  const Spacer(),
                                  InkWell(
                                    borderRadius: BorderRadius.circular(4),
                                    onTap: () {
                                      _controller.close();
                                      widget.onOpenBrowserSessions();
                                    },
                                    child: Padding(
                                      padding: const EdgeInsets.symmetric(
                                        horizontal: 4,
                                        vertical: 2,
                                      ),
                                      child: Text(
                                        '管理全部',
                                        style: theme.textTheme.labelSmall
                                            ?.copyWith(
                                              color: accentColor,
                                              fontSize: 10.5,
                                              fontWeight: FontWeight.w500,
                                            ),
                                      ),
                                    ),
                                  ),
                                ],
                              ),
                            ),
                            for (final tab in tabs)
                              _WorkspaceSessionItemTile(
                                icon: Icons.public_rounded,
                                iconColor: accentColor,
                                title: tab.title.isNotEmpty ? tab.title : '浏览器',
                                subtitle: tab.siteHost.isNotEmpty
                                    ? tab.siteHost
                                    : tab.url,
                                closeTooltip: '关闭浏览器页面',
                                onTap: () {
                                  _controller.close();
                                  widget.onSelectBrowserTab?.call(tab);
                                },
                                onClose: () =>
                                    widget.onCloseBrowserSession?.call(tab.id),
                              ),
                          ],
                        ),
                      ),
                    ),
                  Divider(
                    height: 1,
                    thickness: 1,
                    color: colorScheme.outlineVariant.withValues(alpha: 0.22),
                  ),
                  Padding(
                    padding: const EdgeInsets.fromLTRB(8, 6, 8, 6),
                    child: _CompactActionButton(
                      icon: Icons.add_rounded,
                      label: '新建浏览器',
                      accentColor: accentColor,
                      onTap: () {
                        _controller.close();
                        widget.onOpenBrowser();
                      },
                    ),
                  ),
                ],
              );
            },
          ),
        ),
      ],
    );
  }
}

class _WorkspaceSessionItemTile extends StatelessWidget {
  const _WorkspaceSessionItemTile({
    required this.icon,
    required this.iconColor,
    required this.title,
    required this.subtitle,
    required this.closeTooltip,
    required this.onTap,
    required this.onClose,
  });

  final IconData icon;
  final Color iconColor;
  final String title;
  final String subtitle;
  final String closeTooltip;
  final VoidCallback onTap;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final trimmedSubtitle = subtitle.trim();

    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(6),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
          child: Row(
            children: <Widget>[
              Icon(icon, size: 14, color: iconColor),
              const SizedBox(width: 8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    Text(
                      title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurface,
                        fontWeight: FontWeight.w600,
                        fontSize: 12,
                      ),
                    ),
                    if (trimmedSubtitle.isNotEmpty)
                      Text(
                        trimmedSubtitle,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: colorScheme.onSurfaceVariant.withValues(
                            alpha: 0.7,
                          ),
                          fontSize: 10.5,
                        ),
                      ),
                  ],
                ),
              ),
              const SizedBox(width: 6),
              SizedBox.square(
                dimension: 22,
                child: IconButton(
                  padding: EdgeInsets.zero,
                  iconSize: 14,
                  tooltip: closeTooltip,
                  onPressed: onClose,
                  icon: Icon(
                    Icons.close_rounded,
                    color: colorScheme.onSurfaceVariant.withValues(
                      alpha: 0.75,
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
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

    return Material(
      color: accentColor.withValues(alpha: 0.12),
      borderRadius: BorderRadius.circular(6),
      child: InkWell(
        borderRadius: BorderRadius.circular(6),
        onTap: onTap,
        child: Container(
          height: 28,
          alignment: Alignment.center,
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(6),
            border: Border.all(
              color: accentColor.withValues(alpha: 0.28),
            ),
          ),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(icon, size: 13, color: accentColor),
              const SizedBox(width: 5),
              Text(
                label,
                style: theme.textTheme.labelSmall?.copyWith(
                  color: colorScheme.onSurface,
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
    required this.hasActiveSessions,
    required this.onTap,
  });

  final IconData icon;
  final Color accentColor;
  final String label;
  final bool isExpanded;
  final bool hasActiveSessions;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final activeBg = isExpanded
        ? accentColor.withValues(alpha: 0.16)
        : hasActiveSessions
        ? accentColor.withValues(alpha: 0.10)
        : colorScheme.surfaceContainerHighest.withValues(alpha: 0.40);
    final activeBorder = isExpanded
        ? accentColor.withValues(alpha: 0.45)
        : colorScheme.outlineVariant.withValues(alpha: 0.24);

    return Material(
      color: activeBg,
      borderRadius: BorderRadius.circular(999),
      child: InkWell(
        borderRadius: BorderRadius.circular(999),
        onTap: onTap,
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(999),
            border: Border.all(color: activeBorder),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(icon, size: 14, color: accentColor),
              const SizedBox(width: 5),
              ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 150),
                child: Text(
                  label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.labelMedium?.copyWith(
                    fontSize: 11.5,
                    fontWeight: FontWeight.w600,
                    color: isExpanded ? accentColor : colorScheme.onSurface,
                  ),
                ),
              ),
              const SizedBox(width: 3),
              Icon(
                isExpanded
                    ? Icons.keyboard_arrow_up_rounded
                    : Icons.keyboard_arrow_down_rounded,
                size: 14,
                color: colorScheme.onSurfaceVariant.withValues(alpha: 0.8),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _WorkspaceActionCapsule extends StatelessWidget {
  const _WorkspaceActionCapsule({
    required this.icon,
    required this.accentColor,
    required this.label,
    required this.onTap,
  });

  final IconData icon;
  final Color accentColor;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Material(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.40),
      borderRadius: BorderRadius.circular(999),
      child: InkWell(
        borderRadius: BorderRadius.circular(999),
        onTap: onTap,
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(999),
            border: Border.all(
              color: colorScheme.outlineVariant.withValues(alpha: 0.24),
            ),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(icon, size: 14, color: accentColor),
              const SizedBox(width: 5),
              Text(
                label,
                style: theme.textTheme.labelMedium?.copyWith(
                  fontSize: 11.5,
                  fontWeight: FontWeight.w600,
                  color: colorScheme.onSurface,
                ),
              ),
            ],
          ),
        ),
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


