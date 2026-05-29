// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../../viewmodel/WorkspaceFileModels.dart';
import 'WorkspaceFileBrowserContent.dart';
import 'WorkspaceFilePreviewContent.dart';
import 'WorkspaceHomeContent.dart';
import 'WorkspaceTabModels.dart';

class WorkspaceTabContent extends StatelessWidget {
  const WorkspaceTabContent({
    super.key,
    required this.tab,
    required this.workspacePath,
    required this.onListWorkspaceFiles,
    required this.onReadWorkspaceTextFile,
    required this.onReadWorkspaceFileBytes,
    required this.onOpenWorkspaceFile,
    required this.onOpenFile,
    required this.onOpenFiles,
    required this.onOpenTerminal,
    required this.onOpenBrowser,
  });

  final WorkspaceTab tab;
  final String? workspacePath;
  final Future<List<WorkspaceFileEntry>> Function(String path)
  onListWorkspaceFiles;
  final Future<String> Function(String path) onReadWorkspaceTextFile;
  final Future<Uint8List> Function(String path) onReadWorkspaceFileBytes;
  final Future<void> Function(String path) onOpenWorkspaceFile;
  final Future<void> Function(WorkspaceFileEntry entry) onOpenFile;
  final VoidCallback onOpenFiles;
  final VoidCallback onOpenTerminal;
  final VoidCallback onOpenBrowser;

  @override
  Widget build(BuildContext context) {
    switch (tab.kind) {
      case WorkspaceTabKind.home:
        return WorkspaceHomeContent(
          workspacePath: workspacePath,
          onOpenFiles: onOpenFiles,
          onOpenTerminal: onOpenTerminal,
          onOpenBrowser: onOpenBrowser,
        );
      case WorkspaceTabKind.files:
        final rootPath = workspacePath?.trim();
        if (rootPath == null || rootPath.isEmpty) {
          return const _WorkspaceSimplePane(
            icon: Icons.folder_off_outlined,
            title: '文件',
            subtitle: '当前对话还没有绑定工作区。',
          );
        }
        return WorkspaceFileBrowserContent(
          rootLabel: rootPath,
          rootRelativePath: '',
          onListWorkspaceFiles: onListWorkspaceFiles,
          onOpenFile: onOpenFile,
        );
      case WorkspaceTabKind.terminal:
        return const _WorkspaceSimplePane(
          icon: Icons.terminal,
          title: '终端',
          subtitle: '这里会显示当前工作区的终端会话。',
        );
      case WorkspaceTabKind.browser:
        return const _WorkspaceSimplePane(
          icon: Icons.public,
          title: '浏览器',
          subtitle: '这里会显示项目预览或自动化浏览器。',
        );
      case WorkspaceTabKind.filePreview:
        return WorkspaceFilePreviewContent(
          tab: tab,
          onReadWorkspaceFileBytes: onReadWorkspaceFileBytes,
          onOpenWorkspaceFile: onOpenWorkspaceFile,
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
