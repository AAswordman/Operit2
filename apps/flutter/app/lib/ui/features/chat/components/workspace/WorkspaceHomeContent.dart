// ignore_for_file: file_names

import 'package:flutter/material.dart';

class WorkspaceHomeContent extends StatelessWidget {
  const WorkspaceHomeContent({
    super.key,
    required this.workspacePath,
    required this.onOpenFiles,
    required this.onOpenTerminal,
    required this.onOpenBrowser,
  });

  final String? workspacePath;
  final VoidCallback onOpenFiles;
  final VoidCallback onOpenTerminal;
  final VoidCallback onOpenBrowser;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ColoredBox(
      color: theme.colorScheme.surface,
      child: ListView(
        padding: const EdgeInsets.fromLTRB(16, 18, 16, 18),
        children: <Widget>[
          _WorkspacePrimaryAction(
            icon: Icons.folder_open,
            title: '选择文件',
            subtitle: '从工作区里选择要查看、编辑或交给 AI 的文件',
            onTap: workspacePath?.trim().isNotEmpty == true
                ? onOpenFiles
                : () {},
          ),
          const SizedBox(height: 10),
          _WorkspacePrimaryAction(
            icon: Icons.play_arrow,
            title: '打开终端',
            subtitle: '进入当前工作区的命令行',
            onTap: onOpenTerminal,
          ),
          const SizedBox(height: 10),
          _WorkspacePrimaryAction(
            icon: Icons.public,
            title: '打开浏览器',
            subtitle: '查看项目预览或自动化浏览网页',
            onTap: onOpenBrowser,
          ),
        ],
      ),
    );
  }
}

class _WorkspacePrimaryAction extends StatelessWidget {
  const _WorkspacePrimaryAction({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      color: theme.colorScheme.surfaceContainerLow,
      borderRadius: BorderRadius.circular(8),
      child: InkWell(
        borderRadius: BorderRadius.circular(8),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Row(
            children: <Widget>[
              Container(
                width: 40,
                height: 40,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: theme.colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Icon(icon, color: theme.colorScheme.onPrimaryContainer),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.titleSmall?.copyWith(
                        color: theme.colorScheme.onSurface,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      subtitle,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Icon(
                Icons.chevron_right,
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
