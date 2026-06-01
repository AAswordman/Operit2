// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../viewmodel/WorkspaceFileModels.dart';
import 'browser/WorkspaceBrowserContent.dart';
import 'WorkspaceFileBrowserContent.dart';
import 'WorkspaceFilePreviewContent.dart';
import 'WorkspaceHomeContent.dart';
import 'WorkspaceTabModels.dart';
import 'WorkspaceTerminalContent.dart';

class WorkspaceTabContent extends StatelessWidget {
  const WorkspaceTabContent({
    super.key,
    required this.tab,
    required this.currentChatId,
    required this.workspacePath,
    required this.onListWorkspaceFiles,
    required this.onReadWorkspaceTextFile,
    required this.onReadWorkspaceFileBytes,
    required this.onWriteWorkspaceFileBytes,
    required this.onOpenWorkspaceFile,
    required this.onOpenFile,
    required this.onOpenFiles,
    required this.onOpenTerminal,
    required this.onOpenBrowser,
    required this.onOpenProjectBrowser,
  });

  final WorkspaceTab tab;
  final String currentChatId;
  final String? workspacePath;
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
  final void Function({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
  })
  onOpenBrowser;
  final VoidCallback onOpenProjectBrowser;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    switch (tab.kind) {
      case WorkspaceTabKind.home:
        return WorkspaceHomeContent(
          workspacePath: workspacePath,
          onOpenFiles: onOpenFiles,
          onOpenTerminal: onOpenTerminal,
          onOpenBrowser: onOpenProjectBrowser,
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
        final rootPath = workspacePath?.trim();
        if (rootPath == null || rootPath.isEmpty) {
          return _WorkspaceSimplePane(
            icon: Icons.terminal,
            title: l10n.terminal,
            subtitle: l10n.noWorkspaceBound,
          );
        }
        return WorkspaceTerminalContent(workspacePath: rootPath);
      case WorkspaceTabKind.browser:
        return WorkspaceBrowserContent(
          chatId: currentChatId,
          workspacePath: workspacePath,
          initialUrl: tab.url,
          initialFilePath: tab.absolutePath,
          initialWorkspaceHtmlPath: tab.workspaceHtmlPath,
          onReadWorkspaceTextFile: onReadWorkspaceTextFile,
          onReadWorkspaceFileBytes: onReadWorkspaceFileBytes,
          onWriteWorkspaceFileBytes: onWriteWorkspaceFileBytes,
          onOpenWorkspaceFile: onOpenWorkspaceFile,
          onOpenBrowserTab: onOpenBrowser,
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
    return ColoredBox(
      color: theme.colorScheme.surface,
      child: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
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
    );
  }
}
