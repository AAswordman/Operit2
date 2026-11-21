// ignore_for_file: file_names

import 'package:flutter/foundation.dart' show AsyncCallback;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';
import '../../file_preview/WorkspaceExternalOpenAction.dart';
import '../chrome/WorkspaceBrowserPopupWidgets.dart';
import 'WorkspaceBrowserDownloadStore.dart';

enum _DownloadFilter { all, inProgress, completed, failed }

enum _DownloadAction { open, location, save, cancel, retry, remove }

class WorkspaceBrowserDownloadSheet extends StatefulWidget {
  /// Creates a download panel with explicit workspace export actions.
  const WorkspaceBrowserDownloadSheet({
    super.key,
    required this.store,
    required this.onOpenWorkspaceFile,
    required this.onSaveToWorkspace,
  });

  final WorkspaceBrowserDownloadStore store;
  final Future<void> Function(String path) onOpenWorkspaceFile;
  final Future<void> Function(String path) onSaveToWorkspace;

  /// Creates presentation state without hiding newly completed downloads.
  @override
  State<WorkspaceBrowserDownloadSheet> createState() =>
      _WorkspaceBrowserDownloadSheetState();
}

class _WorkspaceBrowserDownloadSheetState
    extends State<WorkspaceBrowserDownloadSheet> {
  _DownloadFilter _filter = _DownloadFilter.all;

  /// Builds a wrapping filter bar and download cards at the actual panel width.
  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.store,
      builder: (context, _) {
        final l10n = AppLocalizations.of(context)!;
        final all = widget.store.items;
        final items = all.where((item) => _matches(item, _filter));
        final labels = <_DownloadFilter, String>{
          _DownloadFilter.all: l10n.browserDownloadsAll,
          _DownloadFilter.inProgress: l10n.downloading,
          _DownloadFilter.completed: l10n.completed,
          _DownloadFilter.failed: l10n.failed,
        };
        return WorkspaceBrowserPopupBody(
          children: <Widget>[
            WorkspaceBrowserPopupHeader(title: l10n.downloads),
            Padding(
              padding: const EdgeInsets.fromLTRB(12, 0, 12, 8),
              child: Align(
                alignment: Alignment.centerLeft,
                child: Wrap(
                  spacing: 6,
                  runSpacing: 4,
                  children: <Widget>[
                    for (final filter in _DownloadFilter.values)
                      ChoiceChip(
                        selected: _filter == filter,
                        label: Text(
                          '${labels[filter]} (${all.where((item) => _matches(item, filter)).length})',
                        ),
                        onSelected: (_) => setState(() => _filter = filter),
                      ),
                  ],
                ),
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
              child: Text(
                l10n.browserDownloadStorageHint,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
            if (items.isEmpty)
              WorkspaceBrowserPopupEmpty(
                icon: Icons.download,
                text: l10n.noDownloadTasks,
              )
            else
              for (final item in items)
                WorkspaceExternalOpenAction(
                  key: ValueKey(item.id),
                  identity: (
                    widget.store,
                    item.url,
                    item.createdAt,
                    item.savedPath,
                  ),
                  builder: (context, opening, run) => _DownloadCard(
                    item: item,
                    opening: opening,
                    onOpen: opening || item.savedPath == null
                        ? null
                        : () => run(
                            () => widget.onOpenWorkspaceFile(item.savedPath!),
                          ),
                    onSave: opening || item.savedPath == null
                        ? null
                        : () => run(
                            () => widget.onSaveToWorkspace(item.savedPath!),
                          ),
                    onLocation: opening
                        ? null
                        : () => _showLocation(context, item),
                    onCancel: () => run(() => widget.store.cancel(item)),
                    onRetry: () => run(() => widget.store.retry(item)),
                    onRemove: () => run(() => widget.store.remove(item.id)),
                  ),
                ),
          ],
        );
      },
    );
  }

  /// Selects records without conflating interrupted transfers with active ones.
  bool _matches(WorkspaceBrowserDownloadItem item, _DownloadFilter filter) =>
      switch (filter) {
        _DownloadFilter.all => true,
        _DownloadFilter.inProgress =>
          item.state == WorkspaceBrowserDownloadState.pending ||
              item.state == WorkspaceBrowserDownloadState.running,
        _DownloadFilter.completed =>
          item.state == WorkspaceBrowserDownloadState.completed,
        _DownloadFilter.failed =>
          item.state == WorkspaceBrowserDownloadState.failed,
      };

  /// Presents a selectable actual path instead of opening a directory as a file.
  Future<void> _showLocation(
    BuildContext context,
    WorkspaceBrowserDownloadItem item,
  ) async {
    final l10n = AppLocalizations.of(context)!;
    await showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l10n.openLocation),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(l10n.browserDownloadStorageHint),
              if (item.runtimePath != null) ...<Widget>[
                const SizedBox(height: 12),
                SelectableText(item.runtimePath!),
              ],
              if (item.savedPath != null) ...<Widget>[
                const SizedBox(height: 12),
                Text(l10n.browserDownloadWorkspaceTarget),
                SelectableText(item.savedPath!),
              ],
            ],
          ),
        ),
        actions: <Widget>[
          if (item.runtimePath != null)
            TextButton(
              onPressed: () async {
                await Clipboard.setData(ClipboardData(text: item.runtimePath!));
                if (context.mounted) Navigator.of(context).pop();
              },
              child: Text(l10n.workspaceFileCopy),
            ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: Text(l10n.close),
          ),
        ],
      ),
    );
  }
}

class _DownloadCard extends StatelessWidget {
  /// Creates a card whose primary action does not depend on the overflow menu.
  const _DownloadCard({
    required this.item,
    required this.opening,
    required this.onOpen,
    required this.onSave,
    required this.onLocation,
    required this.onCancel,
    required this.onRetry,
    required this.onRemove,
  });

  final WorkspaceBrowserDownloadItem item;
  final bool opening;
  final AsyncCallback? onOpen;
  final AsyncCallback? onSave;
  final AsyncCallback? onLocation;
  final AsyncCallback onCancel;
  final AsyncCallback onRetry;
  final AsyncCallback onRemove;

  /// Keeps filenames, progress, paths, and action targets readable on phones.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final active =
        item.state == WorkspaceBrowserDownloadState.pending ||
        item.state == WorkspaceBrowserDownloadState.running;
    final completed = item.state == WorkspaceBrowserDownloadState.completed;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border.all(color: theme.colorScheme.outlineVariant),
          borderRadius: BorderRadius.circular(12),
        ),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Expanded(
                    child: Text(
                      item.fileName,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.titleSmall,
                    ),
                  ),
                  SizedBox(
                    width: 44,
                    height: 44,
                    child: PopupMenuButton<_DownloadAction>(
                      tooltip: l10n.menu,
                      icon: const Icon(Icons.more_vert),
                      padding: EdgeInsets.zero,
                      // These constraints size the popup route, never the trigger icon.
                      constraints: const BoxConstraints(
                        minWidth: 180,
                        maxWidth: 280,
                      ),
                      onSelected: (action) async {
                        switch (action) {
                          case _DownloadAction.open:
                            await onOpen?.call();
                          case _DownloadAction.location:
                            await onLocation?.call();
                          case _DownloadAction.save:
                            await onSave?.call();
                          case _DownloadAction.cancel:
                            await onCancel();
                          case _DownloadAction.retry:
                            await onRetry();
                          case _DownloadAction.remove:
                            await onRemove();
                        }
                      },
                      itemBuilder: (_) => <PopupMenuEntry<_DownloadAction>>[
                        if (completed) ...<PopupMenuEntry<_DownloadAction>>[
                          PopupMenuItem(
                            value: _DownloadAction.open,
                            enabled: onOpen != null,
                            child: Text(l10n.openFile),
                          ),
                          PopupMenuItem(
                            value: _DownloadAction.location,
                            enabled: onLocation != null,
                            child: Text(l10n.openLocation),
                          ),
                          PopupMenuItem(
                            value: _DownloadAction.save,
                            enabled: onSave != null,
                            child: Text(l10n.browserDownloadSaveToWorkspace),
                          ),
                        ],
                        if (active)
                          PopupMenuItem(
                            value: _DownloadAction.cancel,
                            enabled: !opening,
                            child: Text(l10n.cancel),
                          )
                        else if (!completed && item.canRetry)
                          PopupMenuItem(
                            value: _DownloadAction.retry,
                            enabled: !opening,
                            child: Text(l10n.retry),
                          ),
                        PopupMenuItem(
                          value: _DownloadAction.remove,
                          enabled: !opening,
                          child: Text(l10n.removeRecord),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
              Text(
                opening
                    ? l10n.workspaceFileOpening
                    : workspaceDownloadStatus(l10n, item),
                style: theme.textTheme.bodySmall?.copyWith(
                  color: item.state == WorkspaceBrowserDownloadState.failed
                      ? theme.colorScheme.error
                      : theme.colorScheme.onSurfaceVariant,
                ),
              ),
              if (opening || active) ...<Widget>[
                const SizedBox(height: 8),
                LinearProgressIndicator(
                  value:
                      opening ||
                          item.totalBytes == null ||
                          item.state == WorkspaceBrowserDownloadState.pending
                      ? null
                      : item.progress / 100,
                ),
              ],
              if (item.state == WorkspaceBrowserDownloadState.failed &&
                  item.detail?.isNotEmpty == true) ...<Widget>[
                const SizedBox(height: 6),
                SelectableText(item.detail!, style: theme.textTheme.bodySmall),
              ],
              if (completed && item.runtimePath != null) ...<Widget>[
                const SizedBox(height: 6),
                Text(
                  l10n.savedTo(item.runtimePath!),
                  maxLines: 3,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall,
                ),
              ],
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 4,
                children: <Widget>[
                  if (completed) ...<Widget>[
                    FilledButton.tonalIcon(
                      onPressed: onOpen,
                      icon: const Icon(Icons.open_in_new, size: 18),
                      label: Text(l10n.openFile),
                    ),
                    OutlinedButton(
                      onPressed: onSave,
                      child: Text(l10n.browserDownloadSaveToWorkspace),
                    ),
                  ] else if (active)
                    TextButton(
                      onPressed: opening ? null : onCancel,
                      child: Text(l10n.cancel),
                    )
                  else if (item.canRetry)
                    TextButton(
                      onPressed: opening ? null : onRetry,
                      child: Text(l10n.retry),
                    ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Formats truthful transfer status for both the panel and the persistent strip.
String workspaceDownloadStatus(
  AppLocalizations l10n,
  WorkspaceBrowserDownloadItem item,
) {
  final bytes = _formatBytes(item.receivedBytes);
  return switch (item.state) {
    WorkspaceBrowserDownloadState.pending => l10n.pending,
    WorkspaceBrowserDownloadState.running =>
      item.totalBytes == null
          ? '${l10n.downloading} · $bytes'
          : '${item.progress}% · $bytes / ${_formatBytes(item.totalBytes!)}',
    WorkspaceBrowserDownloadState.completed => '${l10n.completed} · $bytes',
    WorkspaceBrowserDownloadState.failed => l10n.failed,
    WorkspaceBrowserDownloadState.paused => l10n.browserDownloadInterrupted,
    WorkspaceBrowserDownloadState.cancelled => l10n.cancelled,
  };
}

/// Formats byte counts without pretending that an unknown total is zero.
String _formatBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  if (bytes < 1024 * 1024 * 1024) {
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
  return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(1)} GB';
}
