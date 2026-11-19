// ignore_for_file: file_names
import 'package:flutter/material.dart';
import '../../core/backup/ChatBackupService.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart';
import '../../l10n/generated/app_localizations.dart';

/// Shares external-folder snapshot recovery between settings and first-run configuration.
class ChatBackupRestorePanel extends StatefulWidget {
  /// Reports recovery and protects parent navigation while database operations run.
  const ChatBackupRestorePanel({
    super.key,
    this.service,
    this.onRestored,
    this.onBusyChanged,
    this.enabled = true,
  });
  final ChatBackupService? service;
  final VoidCallback? onRestored;
  final ValueChanged<bool>? onBusyChanged;
  final bool enabled;

  /// Creates snapshot browsing state.
  @override
  State<ChatBackupRestorePanel> createState() => _ChatBackupRestorePanelState();
}

class _ChatBackupRestorePanelState extends State<ChatBackupRestorePanel> {
  List<ChatBackupSnapshot> _snapshots = [];
  bool _busy = false;
  bool _loaded = false;
  bool _restored = false;
  String? _error;
  String? _listedDirectory;

  /// Resolves the owner shared with daily backups.
  ChatBackupService get _service =>
      widget.service ?? ChatBackupService.instance;

  /// Browses existing snapshots without creating a backup.
  @override
  void initState() {
    super.initState();
    _service.addListener(_directoryChanged);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _load(select: false);
    });
  }

  /// Removes the directory observer when leaving settings or onboarding.
  @override
  void dispose() {
    _service.removeListener(_directoryChanged);
    super.dispose();
  }

  /// Refreshes the list when another backup panel changes the selected external mount.
  void _directoryChanged() {
    if (mounted && !_busy && _service.directory?.path != _listedDirectory) {
      _load(select: false);
    }
  }

  /// Protects parent navigation during directory and database operations.
  void _setBusy(bool busy) {
    setState(() => _busy = busy);
    widget.onBusyChanged?.call(busy);
  }

  /// Includes snapshots from previous installation identities in date order.
  Future<void> _load({required bool select}) async {
    if (_busy || !widget.enabled) return;
    _setBusy(true);
    try {
      if (select && !await _service.selectDirectory()) {
        if (mounted) setState(() => _error = _service.error);
        return;
      }
      final snapshots = await _service.listSnapshots();
      if (mounted) {
        setState(() {
          _snapshots = snapshots;
          if (_listedDirectory != _service.directory?.path) _restored = false;
          _listedDirectory = _service.directory?.path;
          _error = _service.error;
          _loaded = true;
        });
      }
    } catch (failure) {
      if (mounted) setState(() => _error = '$failure');
    } finally {
      if (mounted) _setBusy(false);
    }
  }

  /// Confirms replacement before saving current chats and restoring the selected snapshot.
  Future<void> _restore(ChatBackupSnapshot snapshot) async {
    if (_busy || !widget.enabled) return;
    final l10n = AppLocalizations.of(context)!;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l10n.chatRestoreConfirmTitle),
        content: Text(l10n.chatRestoreConfirmDescription(snapshot.name)),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(l10n.chatRestoreCancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: Text(l10n.chatRestoreAction),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;
    _setBusy(true);
    try {
      final restored = await _service.restoreSnapshot(snapshot.name);
      if (!mounted) return;
      setState(() {
        _restored = restored;
        _error = restored ? null : _service.error;
      });
      if (restored) widget.onRestored?.call();
    } finally {
      if (mounted) _setBusy(false);
    }
  }

  /// Presents dates, record counts, filenames and explicit recovery actions.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(l10n.chatRestoreDescription),
        const SizedBox(height: 12),
        if (_service.directory != null)
          SelectableText(_service.directory!.label),
        Wrap(
          spacing: 12,
          children: [
            OutlinedButton.icon(
              onPressed: _busy || !widget.enabled
                  ? null
                  : () => _load(select: true),
              icon: const Icon(Icons.folder_open),
              label: Text(l10n.chatRestoreChooseFolder),
            ),
            TextButton.icon(
              onPressed: _busy || !widget.enabled
                  ? null
                  : () => _load(select: false),
              icon: const Icon(Icons.refresh),
              label: Text(l10n.chatRestoreRefresh),
            ),
          ],
        ),
        if (_busy) const LinearProgressIndicator(),
        if (_error != null)
          SelectableText(
            _error!,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
        if (_restored)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 12),
            child: Text(l10n.chatRestoreSuccess),
          ),
        if (_loaded && _snapshots.isEmpty) Text(l10n.chatRestoreEmpty),
        for (final snapshot in _snapshots)
          ListTile(
            contentPadding: EdgeInsets.zero,
            title: Text(
              MaterialLocalizations.of(context).formatMediumDate(
                DateTime.fromMillisecondsSinceEpoch(
                  snapshot.createdAt,
                ).toLocal(),
              ),
            ),
            subtitle: Text(
              '${DateTime.fromMillisecondsSinceEpoch(snapshot.createdAt).toLocal()}\n${l10n.chatRestoreCounts(snapshot.chatCount, snapshot.messageCount)}\n${snapshot.name}',
            ),
            isThreeLine: true,
            trailing: TextButton(
              onPressed: _busy || !widget.enabled
                  ? null
                  : () => _restore(snapshot),
              child: Text(l10n.chatRestoreAction),
            ),
          ),
      ],
    );
  }
}
