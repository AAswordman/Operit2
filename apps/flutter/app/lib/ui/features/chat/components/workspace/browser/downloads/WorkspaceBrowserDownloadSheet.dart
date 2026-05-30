// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';
import '../chrome/WorkspaceBrowserPopupWidgets.dart';
import 'WorkspaceBrowserDownloadStore.dart';

enum _DownloadFilter { inProgress, completed, failed }

class WorkspaceBrowserDownloadSheet extends StatefulWidget {
  const WorkspaceBrowserDownloadSheet({
    super.key,
    required this.store,
    required this.onOpenWorkspaceFile,
  });

  final WorkspaceBrowserDownloadStore store;
  final Future<void> Function(String path) onOpenWorkspaceFile;

  @override
  State<WorkspaceBrowserDownloadSheet> createState() =>
      _WorkspaceBrowserDownloadSheetState();
}

class _WorkspaceBrowserDownloadSheetState
    extends State<WorkspaceBrowserDownloadSheet> {
  _DownloadFilter _filter = _DownloadFilter.inProgress;

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.store,
      builder: (context, child) {
        final l10n = AppLocalizations.of(context)!;
        final theme = Theme.of(context);
        final allItems = widget.store.items;
        final activeCount = allItems.where(_isActive).length;
        final failedCount = allItems
            .where((item) => item.state == WorkspaceBrowserDownloadState.failed)
            .length;
        final items = allItems
            .where((item) {
              return switch (_filter) {
                _DownloadFilter.inProgress =>
                  _isActive(item) ||
                      item.state == WorkspaceBrowserDownloadState.paused ||
                      item.state == WorkspaceBrowserDownloadState.cancelled,
                _DownloadFilter.completed =>
                  item.state == WorkspaceBrowserDownloadState.completed,
                _DownloadFilter.failed =>
                  item.state == WorkspaceBrowserDownloadState.failed,
              };
            })
            .toList(growable: false);
        return WorkspaceBrowserPopupBody(
          children: <Widget>[
            WorkspaceBrowserPopupHeader(
              title: l10n.downloads,
              trailing: Text(
                '$activeCount / $failedCount',
                style: theme.textTheme.labelSmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(12, 0, 12, 4),
              child: Row(
                children: <Widget>[
                  _FilterChip(
                    selected: _filter == _DownloadFilter.inProgress,
                    label: l10n.downloading,
                    onTap: () =>
                        setState(() => _filter = _DownloadFilter.inProgress),
                  ),
                  const SizedBox(width: 6),
                  _FilterChip(
                    selected: _filter == _DownloadFilter.completed,
                    label: l10n.completed,
                    onTap: () =>
                        setState(() => _filter = _DownloadFilter.completed),
                  ),
                  const SizedBox(width: 6),
                  _FilterChip(
                    selected: _filter == _DownloadFilter.failed,
                    label: l10n.failed,
                    onTap: () =>
                        setState(() => _filter = _DownloadFilter.failed),
                  ),
                ],
              ),
            ),
            if (items.isEmpty)
              WorkspaceBrowserPopupEmpty(
                icon: Icons.download,
                text: l10n.noDownloadTasks,
              )
            else
              for (final item in items)
                _DownloadRow(
                  l10n: l10n,
                  item: item,
                  onOpen: item.savedPath == null
                      ? null
                      : () => widget.onOpenWorkspaceFile(item.savedPath!),
                  onOpenLocation: item.savedPath == null
                      ? null
                      : () => widget.onOpenWorkspaceFile('downloads'),
                  onPause: () => widget.store.pause(item),
                  onResume: () => widget.store.resume(item),
                  onCancel: () => widget.store.cancel(item),
                  onRetry: () => widget.store.retry(item),
                  onRemove: () => widget.store.remove(item.url),
                ),
          ],
        );
      },
    );
  }

  bool _isActive(WorkspaceBrowserDownloadItem item) {
    return item.state == WorkspaceBrowserDownloadState.pending ||
        item.state == WorkspaceBrowserDownloadState.running;
  }
}

class _FilterChip extends StatelessWidget {
  const _FilterChip({
    required this.selected,
    required this.label,
    required this.onTap,
  });

  final bool selected;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ChoiceChip(
      selected: selected,
      onSelected: (_) => onTap(),
      label: Text(label, style: const TextStyle(fontSize: 12)),
      visualDensity: VisualDensity.compact,
      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
    );
  }
}

class _DownloadRow extends StatelessWidget {
  const _DownloadRow({
    required this.l10n,
    required this.item,
    required this.onOpen,
    required this.onOpenLocation,
    required this.onPause,
    required this.onResume,
    required this.onCancel,
    required this.onRetry,
    required this.onRemove,
  });

  final AppLocalizations l10n;
  final WorkspaceBrowserDownloadItem item;
  final VoidCallback? onOpen;
  final VoidCallback? onOpenLocation;
  final VoidCallback onPause;
  final VoidCallback onResume;
  final VoidCallback onCancel;
  final VoidCallback onRetry;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        WorkspaceBrowserPopupRow(
          icon: Icons.download_outlined,
          iconColor: _stateColor(theme, item),
          title: item.fileName,
          subtitle: _detailText(l10n, item),
          detail: item.savedPath,
          highlighted: item.state == WorkspaceBrowserDownloadState.running,
          trailing: _DownloadMenu(
            l10n: l10n,
            item: item,
            onOpen: onOpen,
            onOpenLocation: onOpenLocation,
            onPause: onPause,
            onResume: onResume,
            onCancel: onCancel,
            onRetry: onRetry,
            onRemove: onRemove,
          ),
        ),
        if (item.state == WorkspaceBrowserDownloadState.running)
          Padding(
            padding: const EdgeInsets.fromLTRB(40, 0, 12, 4),
            child: LinearProgressIndicator(value: item.progress / 100),
          ),
      ],
    );
  }
}

class _DownloadMenu extends StatelessWidget {
  const _DownloadMenu({
    required this.l10n,
    required this.item,
    required this.onOpen,
    required this.onOpenLocation,
    required this.onPause,
    required this.onResume,
    required this.onCancel,
    required this.onRetry,
    required this.onRemove,
  });

  final AppLocalizations l10n;
  final WorkspaceBrowserDownloadItem item;
  final VoidCallback? onOpen;
  final VoidCallback? onOpenLocation;
  final VoidCallback onPause;
  final VoidCallback onResume;
  final VoidCallback onCancel;
  final VoidCallback onRetry;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    return PopupMenuButton<_DownloadAction>(
      tooltip: '',
      icon: const Icon(Icons.more_vert, size: 18),
      padding: EdgeInsets.zero,
      constraints: const BoxConstraints.tightFor(width: 30, height: 30),
      onSelected: (action) {
        switch (action) {
          case _DownloadAction.open:
            onOpen?.call();
          case _DownloadAction.location:
            onOpenLocation?.call();
          case _DownloadAction.pause:
            onPause();
          case _DownloadAction.resume:
            onResume();
          case _DownloadAction.cancel:
            onCancel();
          case _DownloadAction.retry:
            onRetry();
          case _DownloadAction.remove:
            onRemove();
        }
      },
      itemBuilder: (context) => <PopupMenuEntry<_DownloadAction>>[
        if (item.state == WorkspaceBrowserDownloadState.completed)
          PopupMenuItem(
            value: _DownloadAction.open,
            child: Text(l10n.openFile),
          ),
        if (item.state == WorkspaceBrowserDownloadState.completed)
          PopupMenuItem(
            value: _DownloadAction.location,
            child: Text(l10n.openLocation),
          ),
        if (item.state == WorkspaceBrowserDownloadState.running)
          PopupMenuItem(value: _DownloadAction.pause, child: Text(l10n.pause)),
        if (item.state == WorkspaceBrowserDownloadState.running)
          PopupMenuItem(
            value: _DownloadAction.cancel,
            child: Text(l10n.cancel),
          ),
        if (item.state == WorkspaceBrowserDownloadState.paused)
          PopupMenuItem(
            value: _DownloadAction.resume,
            child: Text(l10n.resume),
          ),
        if (item.state == WorkspaceBrowserDownloadState.failed)
          PopupMenuItem(value: _DownloadAction.retry, child: Text(l10n.retry)),
        PopupMenuItem(
          value: _DownloadAction.remove,
          child: Text(l10n.removeRecord),
        ),
      ],
    );
  }
}

enum _DownloadAction { open, location, pause, resume, cancel, retry, remove }

Color _stateColor(ThemeData theme, WorkspaceBrowserDownloadItem item) {
  switch (item.state) {
    case WorkspaceBrowserDownloadState.failed:
      return theme.colorScheme.error;
    case WorkspaceBrowserDownloadState.completed:
      return theme.colorScheme.primary;
    case WorkspaceBrowserDownloadState.pending:
    case WorkspaceBrowserDownloadState.running:
    case WorkspaceBrowserDownloadState.paused:
    case WorkspaceBrowserDownloadState.cancelled:
      return theme.colorScheme.onSurfaceVariant;
  }
}

String _stateLabel(AppLocalizations l10n, WorkspaceBrowserDownloadItem item) {
  switch (item.state) {
    case WorkspaceBrowserDownloadState.pending:
      return l10n.pending;
    case WorkspaceBrowserDownloadState.running:
      return '${item.progress}%';
    case WorkspaceBrowserDownloadState.completed:
      return l10n.completed;
    case WorkspaceBrowserDownloadState.failed:
      return l10n.failed;
    case WorkspaceBrowserDownloadState.paused:
      return l10n.paused;
    case WorkspaceBrowserDownloadState.cancelled:
      return l10n.cancelled;
  }
}

String _detailText(AppLocalizations l10n, WorkspaceBrowserDownloadItem item) {
  if (item.state == WorkspaceBrowserDownloadState.completed &&
      item.savedPath != null) {
    return l10n.savedTo(item.savedPath!);
  }
  if (item.state == WorkspaceBrowserDownloadState.failed &&
      item.detail != null &&
      item.detail!.isNotEmpty) {
    return item.detail!;
  }
  if (item.state == WorkspaceBrowserDownloadState.running) {
    return l10n.downloading;
  }
  return _stateLabel(l10n, item);
}
