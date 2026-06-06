// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../theme/OperitGlassSurface.dart';
import 'browser/automation/WorkspaceBrowserSessionRegistry.dart';

class WorkspaceHomeContent extends StatelessWidget {
  const WorkspaceHomeContent({
    super.key,
    required this.workspacePath,
    required this.terminalSessionCountListenable,
    required this.browserSessionRegistry,
    required this.onOpenFiles,
    required this.onOpenTerminal,
    required this.onOpenTerminalSessions,
    required this.onOpenBrowserSessions,
    required this.onOpenBrowser,
  });

  final String? workspacePath;
  final ValueListenable<int> terminalSessionCountListenable;
  final WorkspaceBrowserSessionRegistry browserSessionRegistry;
  final VoidCallback onOpenFiles;
  final VoidCallback onOpenTerminal;
  final VoidCallback onOpenTerminalSessions;
  final VoidCallback onOpenBrowserSessions;
  final VoidCallback onOpenBrowser;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final boundWorkspacePath = workspacePath?.trim();
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 18, 16, 18),
      children: <Widget>[
        _WorkspaceStatusSummary(
          workspacePath: boundWorkspacePath,
          terminalSessionCountListenable: terminalSessionCountListenable,
          browserSessionRegistry: browserSessionRegistry,
          onOpenTerminalSessions: onOpenTerminalSessions,
          onOpenBrowserSessions: onOpenBrowserSessions,
        ),
        const SizedBox(height: 10),
        _WorkspacePrimaryAction(
          icon: Icons.folder_open,
          title: l10n.selectFile,
          subtitle: l10n.selectFileDescription,
          onTap: onOpenFiles,
        ),
        const SizedBox(height: 10),
        _WorkspacePrimaryAction(
          icon: Icons.play_arrow,
          title: l10n.openTerminal,
          subtitle: l10n.openTerminalDescription,
          onTap: onOpenTerminal,
        ),
        const SizedBox(height: 10),
        _WorkspacePrimaryAction(
          icon: Icons.public,
          title: l10n.openBrowser,
          subtitle: l10n.openBrowserDescription,
          onTap: onOpenBrowser,
        ),
      ],
    );
  }
}

class _WorkspaceStatusSummary extends StatelessWidget {
  const _WorkspaceStatusSummary({
    required this.workspacePath,
    required this.terminalSessionCountListenable,
    required this.browserSessionRegistry,
    required this.onOpenTerminalSessions,
    required this.onOpenBrowserSessions,
  });

  final String? workspacePath;
  final ValueListenable<int> terminalSessionCountListenable;
  final WorkspaceBrowserSessionRegistry browserSessionRegistry;
  final VoidCallback onOpenTerminalSessions;
  final VoidCallback onOpenBrowserSessions;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(8, 2, 8, 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            '工作区总览',
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.labelLarge?.copyWith(
              color: colorScheme.onSurface,
              fontWeight: FontWeight.w700,
            ),
          ),
          if (workspacePath != null && workspacePath!.isNotEmpty) ...[
            const SizedBox(height: 3),
            Tooltip(
              message: workspacePath!,
              waitDuration: const Duration(milliseconds: 450),
              child: Text(
                workspacePath!,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
            ),
          ],
          const SizedBox(height: 6),
          GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: onOpenTerminalSessions,
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 2),
              child: ValueListenableBuilder<int>(
                valueListenable: terminalSessionCountListenable,
                builder: (context, terminalSessionCount, child) {
                  return Text(
                    '当前 $terminalSessionCount 个终端会话',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: colorScheme.primary,
                      fontWeight: FontWeight.w600,
                    ),
                  );
                },
              ),
            ),
          ),
          GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: onOpenBrowserSessions,
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 2),
              child: AnimatedBuilder(
                animation: browserSessionRegistry,
                builder: (context, child) {
                  return Text(
                    '当前 ${browserSessionRegistry.sessions.length} 个浏览器会话',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: colorScheme.primary,
                      fontWeight: FontWeight.w600,
                    ),
                  );
                },
              ),
            ),
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
    return OperitGlassSurface(
      color: theme.colorScheme.surfaceContainerLowest.withValues(alpha: 0.72),
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(8),
      border: Border.all(
        color: theme.colorScheme.outlineVariant.withValues(alpha: 0.34),
      ),
      material: true,
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
