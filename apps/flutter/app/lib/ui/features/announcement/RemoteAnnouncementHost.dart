// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/logging/ClientLogger.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart';

/// Shows an announcement that was fetched on an earlier launch.
class RemoteAnnouncementHost extends StatefulWidget {
  /// Creates the announcement owner for the main application screen.
  const RemoteAnnouncementHost({super.key, required this.child});

  final Widget child;

  /// Creates the announcement lifecycle state.
  @override
  State<RemoteAnnouncementHost> createState() => _RemoteAnnouncementHostState();
}

class _RemoteAnnouncementHostState extends State<RemoteAnnouncementHost> {
  static const _clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
  late final AppLifecycleListener _lifecycle;
  bool _checking = false;

  /// Schedules the first check after the main screen has been attached.
  @override
  void initState() {
    super.initState();
    _lifecycle = AppLifecycleListener(onResume: _checkAnnouncement);
    WidgetsBinding.instance.addPostFrameCallback((_) => _checkAnnouncement());
  }

  /// Removes lifecycle callbacks when the main application screen is removed.
  @override
  void dispose() {
    _lifecycle.dispose();
    super.dispose();
  }

  /// Fetches one announcement and prevents duplicate requests or dialogs.
  Future<void> _checkAnnouncement() async {
    if (!mounted || _checking) return;
    _checking = true;
    try {
      final announcement = await _clients.servicesRemoteAnnouncementService
          .fetchDisplayableAnnouncement();
      if (!mounted || announcement == null) return;
      await showDialog<void>(
        context: context,
        barrierDismissible: false,
        builder: (context) => RemoteAnnouncementDialog(
          announcement: announcement,
          onAcknowledge: () => _clients.servicesRemoteAnnouncementService
              .acknowledgeAnnouncement(version: announcement.version),
        ),
      );
    } catch (error, stackTrace) {
      ClientLogger.e(
        'Remote announcement check failed',
        tag: 'RemoteAnnouncement',
        error: error,
        stackTrace: stackTrace,
      );
    } finally {
      _checking = false;
    }
  }

  /// Keeps the main application visible beneath the announcement dialog.
  @override
  Widget build(BuildContext context) => widget.child;
}

/// Displays a non-dismissible announcement with a bounded confirmation countdown.
class RemoteAnnouncementDialog extends StatefulWidget {
  /// Creates a dialog that persists confirmation before closing.
  const RemoteAnnouncementDialog({
    super.key,
    required this.announcement,
    required this.onAcknowledge,
  });

  final RemoteAnnouncementDisplay announcement;
  final Future<void> Function() onAcknowledge;

  /// Creates the countdown and acknowledgement state.
  @override
  State<RemoteAnnouncementDialog> createState() =>
      _RemoteAnnouncementDialogState();
}

class _RemoteAnnouncementDialogState extends State<RemoteAnnouncementDialog> {
  Timer? _timer;
  late int _remainingSeconds;
  bool _saving = false;
  String? _error;

  /// Starts the countdown for the published announcement.
  @override
  void initState() {
    super.initState();
    _remainingSeconds = widget.announcement.countdownSec;
    if (_remainingSeconds > 0) {
      _timer = Timer.periodic(const Duration(seconds: 1), _tick);
    }
  }

  /// Enables confirmation once the entire countdown has elapsed.
  void _tick(Timer timer) {
    setState(() => _remainingSeconds--);
    if (_remainingSeconds == 0) timer.cancel();
  }

  /// Releases the countdown when the dialog is removed.
  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  /// Saves confirmation and keeps the dialog open when persistence fails.
  Future<void> _acknowledge() async {
    if (_saving || _remainingSeconds > 0) return;
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      await widget.onAcknowledge();
      if (mounted) Navigator.of(context).pop();
    } catch (error, stackTrace) {
      ClientLogger.e(
        'Remote announcement acknowledgement failed',
        tag: 'RemoteAnnouncement',
        error: error,
        stackTrace: stackTrace,
      );
      if (mounted) {
        setState(() {
          _saving = false;
          _error = error.toString();
        });
      }
    }
  }

  /// Blocks back navigation and exposes only the countdown-gated confirmation.
  @override
  Widget build(BuildContext context) {
    final announcement = widget.announcement;
    return PopScope(
      canPop: false,
      child: AlertDialog(
        title: Text(announcement.title),
        content: ConstrainedBox(
          constraints: const BoxConstraints(maxHeight: 320),
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                SelectableText(announcement.body),
                if (_error case final error?) ...[
                  const SizedBox(height: 16),
                  Text(
                    error,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: _remainingSeconds == 0 && !_saving ? _acknowledge : null,
            child: Text(
              _remainingSeconds == 0
                  ? announcement.acknowledgeText
                  : '${announcement.acknowledgeText} (${_remainingSeconds}s)',
            ),
          ),
        ],
      ),
    );
  }
}
