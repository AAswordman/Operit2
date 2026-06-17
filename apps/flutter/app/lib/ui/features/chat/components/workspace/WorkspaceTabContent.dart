// ignore_for_file: file_names

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:operit2/core/web_visit/WebVisitModels.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../theme/OperitGlassSurface.dart';
import '../../viewmodel/WorkspaceFileModels.dart';
import 'browser/WorkspaceBrowserContent.dart';
import 'browser/automation/WorkspaceBrowserSessionRegistry.dart';
import 'browser/automation/WorkspaceWebVisitContent.dart';
import 'WorkspaceFileBrowserContent.dart';
import 'WorkspaceFilePreviewContent.dart';
import 'WorkspaceHomeContent.dart';
import 'WorkspaceSetupContent.dart';
import 'WorkspaceTabModels.dart';
import 'terminal/WorkspaceTerminalContent.dart';

class WorkspaceTabContent extends StatelessWidget {
  const WorkspaceTabContent({
    super.key,
    required this.tab,
    required this.workspacePath,
    required this.terminalSessionCountListenable,
    required this.browserSessionRegistry,
    required this.onListWorkspaceFiles,
    required this.onReadWorkspaceTextFile,
    required this.onReadWorkspaceFileBytes,
    required this.onWriteWorkspaceFileBytes,
    required this.onOpenWorkspaceFile,
    required this.onOpenFile,
    required this.onOpenFiles,
    required this.onOpenTerminal,
    required this.onOpenTerminalSessions,
    required this.onOpenBrowserSessions,
    required this.onOpenBrowser,
    required this.onFinishWebVisit,
    required this.onActivateCurrentTab,
    required this.onCloseCurrentTab,
    required this.onCreateDefaultWorkspace,
    required this.onBindWorkspace,
  });

  final WorkspaceTab tab;
  final String? workspacePath;
  final ValueListenable<int> terminalSessionCountListenable;
  final WorkspaceBrowserSessionRegistry browserSessionRegistry;
  final Future<List<WorkspaceFileEntry>> Function(String path)
  onListWorkspaceFiles;
  final Future<String> Function(String path) onReadWorkspaceTextFile;
  final Future<Uint8List> Function(String path) onReadWorkspaceFileBytes;
  final Future<void> Function(String path, Uint8List bytes)
  onWriteWorkspaceFileBytes;
  final Future<void> Function(String path) onOpenWorkspaceFile;
  final Future<void> Function(WorkspaceFileEntry entry) onOpenFile;
  final VoidCallback onOpenFiles;
  final VoidCallback onOpenTerminal;
  final VoidCallback onOpenTerminalSessions;
  final VoidCallback onOpenBrowserSessions;
  final void Function({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
  })
  onOpenBrowser;
  final void Function(WorkspaceTab tab, WebVisitResponse response)
  onFinishWebVisit;
  final VoidCallback onActivateCurrentTab;
  final VoidCallback onCloseCurrentTab;
  final Future<void> Function(String? projectType) onCreateDefaultWorkspace;
  final Future<void> Function(String workspace) onBindWorkspace;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    switch (tab.kind) {
      case WorkspaceTabKind.home:
        return WorkspaceHomeContent(
          workspacePath: workspacePath,
          terminalSessionCountListenable: terminalSessionCountListenable,
          browserSessionRegistry: browserSessionRegistry,
          onOpenFiles: onOpenFiles,
          onOpenTerminal: onOpenTerminal,
          onOpenTerminalSessions: onOpenTerminalSessions,
          onOpenBrowserSessions: onOpenBrowserSessions,
          onOpenBrowser: () => onOpenBrowser(),
        );
      case WorkspaceTabKind.setup:
        return WorkspaceSetupContent(
          onCreateDefaultWorkspace: onCreateDefaultWorkspace,
          onBindWorkspace: onBindWorkspace,
        );
      case WorkspaceTabKind.files:
        final rootPath = workspacePath?.trim();
        if (rootPath == null || rootPath.isEmpty) {
          return _WorkspaceSimplePane(
            icon: Icons.folder_off_outlined,
            title: l10n.files,
            subtitle: l10n.noWorkspaceBound,
          );
        }
        return WorkspaceFileBrowserContent(
          rootLabel: rootPath,
          rootRelativePath: '',
          onListWorkspaceFiles: onListWorkspaceFiles,
          onOpenFile: onOpenFile,
        );
      case WorkspaceTabKind.terminal:
        final sessionId = tab.terminalSessionId;
        final sessionKind = tab.terminalSessionKind;
        final terminalType = tab.terminalType;
        if (sessionId == null || sessionKind == null || terminalType == null) {
          return _WorkspaceSimplePane(
            icon: Icons.terminal,
            title: l10n.terminal,
            subtitle: '终端会话未指定。',
          );
        }
        return WorkspaceTerminalContent(
          sessionId: sessionId,
          sessionKind: sessionKind,
          terminalType: terminalType,
          workingDir: tab.terminalWorkingDir ?? '',
        );
      case WorkspaceTabKind.browser:
        return WorkspaceBrowserContent(
          workspacePath: workspacePath,
          initialUrl: tab.url,
          initialUserAgent: tab.userAgent,
          initialHeaders: tab.headers,
          initialFilePath: tab.absolutePath,
          initialWorkspaceHtmlPath: tab.workspaceHtmlPath,
          onReadWorkspaceTextFile: onReadWorkspaceTextFile,
          onReadWorkspaceFileBytes: onReadWorkspaceFileBytes,
          onWriteWorkspaceFileBytes: onWriteWorkspaceFileBytes,
          onOpenWorkspaceFile: onOpenWorkspaceFile,
          onOpenBrowserTab: onOpenBrowser,
          onActivateRequested: onActivateCurrentTab,
          onCloseRequested: onCloseCurrentTab,
        );
      case WorkspaceTabKind.webVisit:
        final request = tab.webVisitRequest;
        if (request == null) {
          return _WorkspaceSimplePane(
            icon: Icons.travel_explore,
            title: 'visit_web',
            subtitle: 'visit_web 请求未指定。',
          );
        }
        return WorkspaceWebVisitContent(
          request: request,
          onFinished: (response) => onFinishWebVisit(tab, response),
        );
      case WorkspaceTabKind.filePreview:
        return WorkspaceFilePreviewContent(
          tab: tab,
          onReadWorkspaceFileBytes: onReadWorkspaceFileBytes,
          onOpenWorkspaceFile: onOpenWorkspaceFile,
          onOpenBrowser: onOpenBrowser,
        );
    }
  }
}

class _WorkspaceSimplePane extends StatelessWidget {
  const _WorkspaceSimplePane({
    required this.icon,
    required this.title,
    required this.subtitle,
  });

  final IconData icon;
  final String title;
  final String subtitle;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: OperitGlassSurface(
          color: theme.colorScheme.surfaceContainerHighest.withValues(
            alpha: 0.36,
          ),
          layer: OperitGlassSurfaceLayer.card,
          borderRadius: BorderRadius.circular(18),
          border: Border.all(
            color: theme.colorScheme.outlineVariant.withValues(alpha: 0.18),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 22, vertical: 20),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Icon(icon, size: 42, color: theme.colorScheme.primary),
                const SizedBox(height: 12),
                Text(
                  title,
                  style: theme.textTheme.titleMedium?.copyWith(
                    color: theme.colorScheme.onSurface,
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: 6),
                Text(
                  subtitle,
                  textAlign: TextAlign.center,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
