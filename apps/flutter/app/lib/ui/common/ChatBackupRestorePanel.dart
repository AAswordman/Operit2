// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../core/backup/ChatBackupDirectoryStorage.dart';
import '../../core/backup/ChatBackupService.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart';
import '../../l10n/generated/app_localizations.dart';

/// Browses database backups from a folder chosen for this restore session.
class ChatBackupRestorePanel extends StatefulWidget {
  const ChatBackupRestorePanel({
    super.key,
    this.service,
    this.onRestored,
    this.onBusyChanged,
  }) : _dialog = false;

  const ChatBackupRestorePanel._dialog({required this.service})
    : _dialog = true,
      onRestored = null,
      onBusyChanged = null;

  final ChatBackupService? service;
  final VoidCallback? onRestored;
  final ValueChanged<bool>? onBusyChanged;
  final bool _dialog;

  /// Opens the backup browser without listing files on the settings page.
  static Future<bool?> show(BuildContext context, ChatBackupService service) {
    return showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (context) => ChatBackupRestorePanel._dialog(service: service),
    );
  }

  /// Creates a fresh restore session for the selected entry point.
  @override
  State<ChatBackupRestorePanel> createState() => _ChatBackupRestorePanelState();
}

class _ChatBackupRestorePanelState extends State<ChatBackupRestorePanel> {
  List<ChatBackupSnapshot> _snapshots = [];
  PersistentDirectory? _source;
  ChatBackupSnapshot? _selected;
  bool _busy = false;
  bool _confirming = false;
  bool _restored = false;
  String? _error;

  /// Resolves the service shared with daily backups.
  ChatBackupService get _service =>
      widget.service ?? ChatBackupService.instance;

  /// Opens the configured folder when the restore flow starts.
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _load(() async => _service.configuredDirectory);
    });
  }

  /// Keeps the current source on picker cancellation and clears selection when listing a source.
  Future<void> _load(Future<PersistentDirectory?> Function() choose) async {
    if (_busy) return;
    _setBusy(true);
    try {
      final source = await choose();
      if (!mounted || source == null) return;
      setState(() {
        _source = source;
        _selected = null;
        _error = null;
        _snapshots = [];
      });
      final snapshots = await _service.listSnapshots(source);
      if (mounted) setState(() => _snapshots = snapshots);
    } catch (error) {
      if (mounted) setState(() => _error = '$error');
    } finally {
      if (mounted) _setBusy(false);
    }
  }

  /// Protects the current source and parent navigation during this operation.
  void _setBusy(bool busy) {
    setState(() => _busy = busy);
    widget.onBusyChanged?.call(busy);
  }

  /// Confirms database replacement and restores from the exact source shown in the browser.
  Future<void> _restore() async {
    final source = _source;
    final selected = _selected;
    if (_busy || source == null || selected == null) return;
    final l10n = AppLocalizations.of(context)!;
    _confirming = true;
    _setBusy(true);
    try {
      final confirmed = await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: Text(l10n.chatRestoreConfirmTitle),
          content: Text(l10n.chatRestoreConfirmDescription(selected.name)),
          scrollable: true,
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: Text(l10n.cancel),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, true),
              child: Text(l10n.chatRestoreAction),
            ),
          ],
        ),
      );
      if (confirmed != true || !mounted) return;
      setState(() {
        _confirming = false;
        _error = null;
      });
      await _service.restoreSnapshot(source, selected.name);
      if (!mounted) return;
      setState(() {
        _restored = true;
        _selected = null;
      });
      widget.onRestored?.call();
    } catch (error) {
      if (mounted) setState(() => _error = '$error');
    } finally {
      if (mounted) {
        _confirming = false;
        _setBusy(false);
      }
    }
  }

  /// Shows the restore source and dated choices, with one action for the selected backup.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final material = MaterialLocalizations.of(context);
    final source = _source;
    final restoreButton = FilledButton.icon(
      onPressed: _busy || _selected == null ? null : _restore,
      icon: const Icon(Icons.restore),
      label: Text(l10n.chatRestoreSelected),
    );
    final browser = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(l10n.chatRestoreDescription),
        if (source != null) ...[
          const SizedBox(height: 12),
          SelectableText(source.label, style: theme.textTheme.titleSmall),
        ],
        const SizedBox(height: 8),
        Wrap(
          spacing: 8,
          runSpacing: 4,
          children: [
            OutlinedButton.icon(
              onPressed: _busy || _restored
                  ? null
                  : () => _load(_service.selectRestoreDirectory),
              icon: const Icon(Icons.folder_open_outlined),
              label: Text(l10n.chatRestoreChooseFolder),
            ),
            if (source != null && !_restored)
              TextButton.icon(
                onPressed: _busy ? null : () => _load(() async => source),
                icon: const Icon(Icons.refresh),
                label: Text(l10n.chatRestoreRefresh),
              ),
          ],
        ),
        const SizedBox(height: 12),
        if (_busy && !_confirming) const LinearProgressIndicator(),
        if (_error != null)
          SelectableText(
            _error!,
            style: TextStyle(color: theme.colorScheme.error),
          ),
        if (_restored)
          Text(l10n.chatRestoreSuccess)
        else if (!_busy &&
            source != null &&
            _error == null &&
            _snapshots.isEmpty)
          Text(l10n.chatRestoreEmpty),
        if (!_restored)
          for (final snapshot in _snapshots)
            ListTile(
              contentPadding: const EdgeInsets.symmetric(horizontal: 8),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(8),
              ),
              selectedTileColor: theme.colorScheme.secondaryContainer,
              selected: _selected == snapshot,
              enabled: !_busy,
              onTap: () => setState(() => _selected = snapshot),
              leading: Icon(
                _selected == snapshot
                    ? Icons.radio_button_checked
                    : Icons.radio_button_unchecked,
              ),
              title: Text(
                '${material.formatMediumDate(DateTime.fromMillisecondsSinceEpoch(snapshot.createdAt))} 路 '
                '${material.formatTimeOfDay(TimeOfDay.fromDateTime(DateTime.fromMillisecondsSinceEpoch(snapshot.createdAt)))}',
              ),
              subtitle: Text(snapshot.name),
            ),
        if (!widget._dialog && !_restored && _snapshots.isNotEmpty) ...[
          const SizedBox(height: 8),
          restoreButton,
        ],
      ],
    );
    if (!widget._dialog) return browser;
    return PopScope(
      canPop: !_busy,
      child: AlertDialog(
        insetPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 24),
        scrollable: true,
        title: Text(l10n.chatRestoreTitle),
        content: SizedBox(width: 520, child: browser),
        actions: [
          TextButton(
            onPressed: _busy ? null : () => Navigator.pop(context, _restored),
            child: Text(_restored ? l10n.ok : l10n.cancel),
          ),
          if (!_restored) restoreButton,
        ],
      ),
    );
  }
}
