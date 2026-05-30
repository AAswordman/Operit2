// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';
import '../userscripts/WorkspaceUserscriptModels.dart';

class WorkspaceBrowserMenuSheet extends StatefulWidget {
  const WorkspaceBrowserMenuSheet({
    super.key,
    required this.onHistory,
    required this.onBookmarks,
    required this.onDownloads,
    required this.onUserscripts,
    required this.onPermissions,
    required this.onClearStorage,
    required this.desktopMode,
    required this.onDesktopModeChanged,
    required this.onLoadMenuCommands,
    required this.onRunMenuCommand,
    required this.activeDownloadCount,
  });

  final VoidCallback onHistory;
  final VoidCallback onBookmarks;
  final VoidCallback onDownloads;
  final VoidCallback onUserscripts;
  final VoidCallback onPermissions;
  final VoidCallback onClearStorage;
  final bool desktopMode;
  final ValueChanged<bool> onDesktopModeChanged;
  final Future<List<WorkspaceUserscriptMenuCommand>> Function()
  onLoadMenuCommands;
  final ValueChanged<int> onRunMenuCommand;
  final int activeDownloadCount;

  @override
  State<WorkspaceBrowserMenuSheet> createState() =>
      _WorkspaceBrowserMenuSheetState();
}

class _WorkspaceBrowserMenuSheetState extends State<WorkspaceBrowserMenuSheet> {
  late final Future<List<WorkspaceUserscriptMenuCommand>> _menuCommandsFuture;

  @override
  void initState() {
    super.initState();
    _menuCommandsFuture = widget.onLoadMenuCommands();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          _MenuTile(
            icon: Icons.history,
            title: l10n.history,
            onTap: widget.onHistory,
          ),
          _MenuTile(
            icon: Icons.star_border,
            title: l10n.bookmarks,
            onTap: widget.onBookmarks,
          ),
          _MenuTile(
            icon: Icons.download_outlined,
            title: l10n.downloads,
            value: widget.activeDownloadCount > 0
                ? widget.activeDownloadCount.toString()
                : '',
            onTap: widget.onDownloads,
          ),
          _MenuTile(
            icon: Icons.javascript_outlined,
            title: l10n.scripts,
            onTap: widget.onUserscripts,
          ),
          FutureBuilder<List<WorkspaceUserscriptMenuCommand>>(
            future: _menuCommandsFuture,
            builder: (context, snapshot) {
              final commands =
                  snapshot.data ?? const <WorkspaceUserscriptMenuCommand>[];
              if (commands.isEmpty) {
                return const SizedBox.shrink();
              }
              return Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  for (final command in commands)
                    _MenuTile(
                      icon: Icons.extension,
                      title: command.caption,
                      value: command.scriptName,
                      onTap: () => widget.onRunMenuCommand(command.index),
                    ),
                ],
              );
            },
          ),
          _MenuTile(
            icon: Icons.lock_outline,
            title: l10n.permissionsTitle,
            onTap: widget.onPermissions,
          ),
          _MenuTile(
            icon: Icons.desktop_windows_outlined,
            title: l10n.desktopMode,
            value: widget.desktopMode ? l10n.enable : l10n.disable,
            selected: widget.desktopMode,
            onTap: () => widget.onDesktopModeChanged(!widget.desktopMode),
          ),
          _MenuTile(
            icon: Icons.cleaning_services_outlined,
            title: l10n.clearLocalStorage,
            onTap: widget.onClearStorage,
          ),
        ],
      ),
    );
  }
}

class _MenuTile extends StatelessWidget {
  const _MenuTile({
    required this.icon,
    required this.title,
    required this.onTap,
    this.value = '',
    this.selected = false,
  });

  final IconData icon;
  final String title;
  final VoidCallback onTap;
  final String value;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return InkWell(
      onTap: onTap,
      child: ColoredBox(
        color: selected
            ? colorScheme.primaryContainer.withValues(alpha: 0.18)
            : Colors.transparent,
        child: ConstrainedBox(
          constraints: const BoxConstraints(minHeight: 40),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: Row(
              children: <Widget>[
                Icon(
                  icon,
                  size: 17,
                  color: selected
                      ? colorScheme.primary
                      : colorScheme.onSurfaceVariant,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      fontSize: 13,
                      color: selected
                          ? colorScheme.primary
                          : colorScheme.onSurface,
                      fontWeight: selected
                          ? FontWeight.w600
                          : FontWeight.normal,
                    ),
                  ),
                ),
                if (value.trim().isNotEmpty) ...<Widget>[
                  const SizedBox(width: 8),
                  Flexible(
                    child: Text(
                      value,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      textAlign: TextAlign.end,
                      style: TextStyle(
                        fontSize: 13,
                        color: selected
                            ? colorScheme.primary
                            : colorScheme.onSurfaceVariant,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}
