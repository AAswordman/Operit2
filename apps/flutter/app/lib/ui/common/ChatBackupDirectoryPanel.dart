// ignore_for_file: file_names

import 'dart:async';
import 'package:flutter/material.dart';
import '../../core/backup/ChatBackupService.dart';
import '../../l10n/generated/app_localizations.dart';

/// Shares directory selection and status across onboarding and repair.
class ChatBackupDirectoryPanel extends StatefulWidget {
  /// Reports setup or deferral of the scheduled backup directory.
  const ChatBackupDirectoryPanel({
    super.key,
    this.service,
    this.onSelected,
    this.onDeferred,
  });
  final ChatBackupService? service;
  final VoidCallback? onSelected;
  final VoidCallback? onDeferred;

  /// Creates the local picker busy state.
  @override
  State<ChatBackupDirectoryPanel> createState() =>
      _ChatBackupDirectoryPanelState();
}

class _ChatBackupDirectoryPanelState extends State<ChatBackupDirectoryPanel> {
  bool _busy = false;
  ChatBackupService get _service =>
      widget.service ?? ChatBackupService.instance;

  /// Revalidates a stored directory when onboarding or repair opens this panel.
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) unawaited(_service.checkAndBackup());
    });
  }

  /// Opens the platform directory selector and checks its selected capability.
  Future<void> _select() async {
    setState(() => _busy = true);
    try {
      if (await _service.selectDirectory()) {
        widget.onSelected?.call();
        unawaited(_service.checkAndBackup());
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  /// Presents the actual selected location and concrete authorization or I/O errors.
  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: _service,
    builder: (context, _) {
      final l10n = AppLocalizations.of(context)!;
      final selected = _service.directory;
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            l10n.chatBackupChooseTitle,
            style: Theme.of(context).textTheme.titleLarge,
          ),
          const SizedBox(height: 12),
          Text(l10n.chatBackupDescription),
          const SizedBox(height: 12),
          Text(l10n.chatBackupExternalDescription),
          const SizedBox(height: 16),
          if (selected != null) SelectableText(selected.label),
          Text(
            selected == null || _service.needsDirectory
                ? l10n.chatBackupMissing
                : l10n.chatBackupReady,
          ),
          if (_service.error != null) ...[
            const SizedBox(height: 8),
            SelectableText(
              _service.directorySelectionFailed || _service.needsDirectory
                  ? l10n.chatBackupDirectoryUnavailable
                  : _service.error!,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ],
          const SizedBox(height: 16),
          Wrap(
            spacing: 12,
            runSpacing: 8,
            children: [
              FilledButton.icon(
                onPressed: _busy ? null : _select,
                icon: _busy
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.folder_open),
                label: Text(
                  selected == null
                      ? l10n.chatBackupChoose
                      : l10n.chatBackupChange,
                ),
              ),
              if (widget.onDeferred != null)
                TextButton(
                  onPressed: _busy
                      ? null
                      : () {
                          _service.deferDirectoryPrompt();
                          widget.onDeferred!();
                        },
                  child: Text(l10n.chatBackupLater),
                ),
            ],
          ),
        ],
      );
    },
  );
}

/// Prompts existing users and repairs revoked or missing directories during normal use.
class ChatBackupHost extends StatefulWidget {
  /// Starts backup checks only after normal onboarding has completed.
  const ChatBackupHost({super.key, required this.child, this.service});
  final Widget child;
  final ChatBackupService? service;

  /// Creates the lifecycle and periodic directory check owner.
  @override
  State<ChatBackupHost> createState() => _ChatBackupHostState();
}

class _ChatBackupHostState extends State<ChatBackupHost>
    with WidgetsBindingObserver {
  Timer? _timer;
  late final ChatBackupService _service;

  /// Checks on startup and periodically while this application is active.
  @override
  void initState() {
    super.initState();
    _service = widget.service ?? ChatBackupService.instance;
    WidgetsBinding.instance.addObserver(this);
    _service.addListener(_changed);
    _timer = Timer.periodic(const Duration(minutes: 5), (_) => _check());
    WidgetsBinding.instance.addPostFrameCallback((_) => _check());
  }

  /// Prevents periodic work after the normal application route is removed.
  @override
  void dispose() {
    _timer?.cancel();
    _service.removeListener(_changed);
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  /// Revalidates grants and checks the daily backup after returning to the foreground.
  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) _check();
  }

  /// Updates the repair page without adding duplicate Navigator routes.
  void _changed() {
    if (mounted) setState(() {});
  }

  /// Runs only in the foreground, where platform directory access is available.
  void _check() {
    final state = WidgetsBinding.instance.lifecycleState;
    if (state == null || state == AppLifecycleState.resumed) {
      unawaited(_service.checkAndBackup());
    }
  }

  /// Keeps the application mounted beneath an opaque, modal directory prompt.
  @override
  Widget build(BuildContext context) => Stack(
    children: [
      widget.child,
      if (_service.needsDirectory && !_service.promptDeferred)
        Positioned.fill(
          child: BlockSemantics(
            child: Stack(
              fit: StackFit.expand,
              children: [
                ModalBarrier(
                  dismissible: false,
                  color: Theme.of(
                    context,
                  ).colorScheme.scrim.withValues(alpha: 0.38),
                ),
                SafeArea(
                  child: Dialog(
                    backgroundColor: Theme.of(
                      context,
                    ).colorScheme.surfaceContainerHigh,
                    child: SingleChildScrollView(
                      padding: const EdgeInsets.all(24),
                      child: ChatBackupDirectoryPanel(
                        service: _service,
                        onDeferred: _changed,
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
    ],
  );
}
