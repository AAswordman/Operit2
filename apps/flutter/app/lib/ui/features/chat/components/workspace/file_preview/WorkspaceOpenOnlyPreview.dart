// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../l10n/generated/app_localizations.dart';
import '../WorkspaceTabModels.dart';

class WorkspaceOpenOnlyPreview extends StatelessWidget {
  const WorkspaceOpenOnlyPreview({
    super.key,
    required this.tab,
    required this.icon,
    required this.title,
    required this.detail,
    required this.onOpenWorkspaceFile,
  });

  final WorkspaceTab tab;
  final IconData icon;
  final String title;
  final String detail;
  final Future<void> Function(String path) onOpenWorkspaceFile;

  @override
  Widget build(BuildContext context) {
    return WorkspaceOpenOnlyPreviewBody(
      tab: tab,
      icon: icon,
      title: title,
      detail: detail,
      onOpenWorkspaceFile: onOpenWorkspaceFile,
    );
  }
}

class WorkspaceOpenOnlyPreviewBody extends StatelessWidget {
  const WorkspaceOpenOnlyPreviewBody({
    super.key,
    required this.tab,
    required this.icon,
    required this.title,
    required this.detail,
    required this.onOpenWorkspaceFile,
  });

  final WorkspaceTab tab;
  final IconData icon;
  final String title;
  final String detail;
  final Future<void> Function(String path) onOpenWorkspaceFile;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    final filePath = tab.filePath;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(icon, size: 44, color: theme.colorScheme.primary),
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
              detail,
              textAlign: TextAlign.center,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            if (filePath != null) ...<Widget>[
              const SizedBox(height: 16),
              FilledButton.icon(
                onPressed: () {
                  onOpenWorkspaceFile(filePath);
                },
                icon: const Icon(Icons.open_in_new),
                label: Text(l10n.openFile),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
