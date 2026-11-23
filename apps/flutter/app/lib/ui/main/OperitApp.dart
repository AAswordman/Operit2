// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:operit_folder_access/operit_folder_access.dart';

import '../../core/application/CoreApplicationService.dart';
import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/host/ComposeWebViewControllerBridge.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../core/runtime/RuntimeBootstrapManager.dart';
import '../../data/preferences/UserPreferencesManager.dart';
import '../../l10n/generated/app_localizations.dart';
import '../features/packages/screens/ToolPkgComposeDslWebView.dart';
import '../theme/OperitTheme.dart';
import '../common/AppPeerDialogHost.dart';
import 'navigation/AppStartupRouteCatalog.dart';

class OperitApp extends StatefulWidget {
  const OperitApp({super.key});

  /// Creates the main application bootstrap state.
  @override
  State<OperitApp> createState() => _OperitAppState();
}

class _OperitAppState extends State<OperitApp> {
  final RuntimeBootstrapManager _runtimeManager =
      RuntimeBootstrapManager.instance;
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
    _runtimeManager.addListener(_handleRuntimeBootstrapChanged);
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
    _runtimeManager.removeListener(_handleRuntimeBootstrapChanged);
    super.dispose();
  }

  /// Reacts to runtime configuration and preserves onboarding state on startup.
  void _handleRuntimeBootstrapChanged() {
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
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeMode: _runtimeManager.startupThemeMode,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      child: LocalDirectoryPickerHost(
        child: _AppDialogHost(
          startupWebAccessError: _startupWebAccessError,
          child: AppStartupRouteHost(key: ValueKey<int>(_startupRouteEpoch)),
        ),
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
  static const GeneratedCoreProxyClients _coreClients =
      GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  bool _shownStartupWebAccessError = false;
  final RuntimeBootstrapManager _runtimeManager =
      RuntimeBootstrapManager.instance;

  /// Subscribes to runtime node pairing prompts.
  @override
  void initState() {
    super.initState();
    _runtimeManager.addListener(_syncPairingSubscription);
    _syncPairingSubscription();
  }

  /// Runtime configuration controls the injected, independently tested dialog host.
  void _syncPairingSubscription() {
    if (mounted) setState(() {});
  }

  @override
  void dispose() {
    _runtimeManager.removeListener(_syncPairingSubscription);
    super.dispose();
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
  void didChangeDependencies() {
    super.didChangeDependencies();
    _showStartupWebAccessError();
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

  @override
  Widget build(BuildContext context) => AppPeerDialogHost(
    clients: _coreClients,
    enabled: _runtimeManager.runtimeConfigured,
    child: widget.child,
  );
}
