// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../core/application/CoreApplicationService.dart';
import '../../core/host/ComposeWebViewControllerBridge.dart';
import '../../core/link_host/LinkHostServer.dart';
import '../../core/runtime/RuntimeConnectionManager.dart';
import '../../l10n/generated/app_localizations.dart';
import '../features/packages/screens/ToolPkgComposeDslWebView.dart';
import '../theme/OperitTheme.dart';
import 'navigation/AppStartupRouteCatalog.dart';

class OperitApp extends StatefulWidget {
  const OperitApp({super.key});

  /// Creates the main application bootstrap state.
  @override
  State<OperitApp> createState() => _OperitAppState();
}

class _OperitAppState extends State<OperitApp> {
  final RuntimeConnectionManager _runtimeManager =
      RuntimeConnectionManager.instance;
  StreamSubscription<Object>? _startupErrorSubscription;
  void Function()? _unregisterComposeWebViewController;
  String? _startupWebAccessError;
  bool _lastRuntimeConfigured = false;
  int _startupRouteEpoch = 0;

  /// Subscribes to runtime state and process-level startup errors.
  @override
  void initState() {
    super.initState();
    _lastRuntimeConfigured = _runtimeManager.runtimeConfigured;
    _runtimeManager.addListener(_handleRuntimeConnectionChanged);
    _startupErrorSubscription = CoreApplicationService.instance.startupErrors
        .listen(_handleStartupError);
    final pendingStartupError = CoreApplicationService.instance
        .consumeStartupError();
    if (pendingStartupError != null) {
      _startupWebAccessError = pendingStartupError.toString();
    }
    _unregisterComposeWebViewController = const ComposeWebViewControllerBridge()
        .registerHandler(ComposeDslWebViewHostRegistry.handleControllerCommand);
  }

  /// Releases UI-only runtime and error listeners.
  @override
  void dispose() {
    _unregisterComposeWebViewController?.call();
    unawaited(_startupErrorSubscription?.cancel());
    _runtimeManager.removeListener(_handleRuntimeConnectionChanged);
    super.dispose();
  }

  /// Reacts to runtime configuration and preserves onboarding state on startup.
  void _handleRuntimeConnectionChanged() {
    final runtimeConfigured = _runtimeManager.runtimeConfigured;
    if (_lastRuntimeConfigured && !runtimeConfigured) {
      _startupRouteEpoch++;
    }
    _lastRuntimeConfigured = runtimeConfigured;
    if (mounted) {
      setState(() {});
    }
  }

  /// Presents a process-level Core startup error through the app dialog host.
  void _handleStartupError(Object error) {
    if (!mounted) {
      return;
    }
    setState(() {
      _startupWebAccessError = error.toString();
    });
  }

  /// Builds the runtime-gated main application.
  @override
  Widget build(BuildContext context) {
    return OperitTheme(
      unconfiguredChildEnabled: true,
      child: _AppDialogHost(
        startupWebAccessError: _startupWebAccessError,
        child: AppStartupRouteHost(key: ValueKey<int>(_startupRouteEpoch)),
      ),
    );
  }
}

class _AppDialogHost extends StatefulWidget {
  const _AppDialogHost({
    required this.startupWebAccessError,
    required this.child,
  });

  final String? startupWebAccessError;
  final Widget child;

  @override
  State<_AppDialogHost> createState() => _AppDialogHostState();
}

class _AppDialogHostState extends State<_AppDialogHost> {
  bool _shownStartupWebAccessError = false;
  String _shownPairingId = '';

  @override
  void initState() {
    super.initState();
    LinkHostServer.instance.addListener(_onWebAccessChanged);
    RuntimeConnectionManager.instance.addListener(_onManagerChanged);
  }

  /// Handles newly reported LinkHost startup errors.
  @override
  void didUpdateWidget(covariant _AppDialogHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.startupWebAccessError != widget.startupWebAccessError) {
      _shownStartupWebAccessError = false;
      _showStartupWebAccessError();
    }
  }

  @override
  void dispose() {
    LinkHostServer.instance.removeListener(_onWebAccessChanged);
    RuntimeConnectionManager.instance.removeListener(_onManagerChanged);
    super.dispose();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _showStartupWebAccessError();
    _showPendingRemoteError();
  }

  void _showStartupWebAccessError() {
    final error = widget.startupWebAccessError;
    if (_shownStartupWebAccessError || error == null) {
      return;
    }
    _shownStartupWebAccessError = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      final l10n = AppLocalizations.of(context)!;
      showDialog<void>(
        context: context,
        builder: (context) {
          return AlertDialog(
            title: Text(l10n.settingsWebAccessService),
            content: SingleChildScrollView(
              child: SelectableText(l10n.settingsWebAccessStartFailed(error)),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(l10n.ok),
              ),
            ],
          );
        },
      );
    });
  }

  void _onWebAccessChanged() {
    final record = LinkHostServer.instance.lastPairingCode;
    if (record == null || record.pairingId == _shownPairingId) {
      return;
    }
    _shownPairingId = record.pairingId;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      final l10n = AppLocalizations.of(context)!;
      showDialog<void>(
        context: context,
        builder: (context) {
          return AlertDialog(
            title: Text(l10n.settingsWebAccessPairingRequest),
            content: SelectableText(
              l10n.settingsWebAccessPairingRequestMessage(
                record.pairingCode,
                record.clientDeviceId,
              ),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(l10n.ok),
              ),
            ],
          );
        },
      );
    });
  }

  void _onManagerChanged() {
    _showPendingRemoteError();
  }

  void _showPendingRemoteError() {
    final error = RuntimeConnectionManager.instance.consumePendingRemoteError();
    if (error == null || !mounted) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final l10n = AppLocalizations.of(context)!;
      showDialog<void>(
        context: context,
        builder: (context) {
          return AlertDialog(
            title: Text(l10n.settingsRuntimeRemoteDisconnected),
            content: SingleChildScrollView(
              child: SelectableText(
                l10n.settingsRuntimeRemoteDisconnectedMessage(error.toString()),
              ),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(l10n.ok),
              ),
            ],
          );
        },
      );
    });
  }

  @override
  Widget build(BuildContext context) {
    return widget.child;
  }
}
