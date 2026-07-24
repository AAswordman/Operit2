// ignore_for_file: file_names

import 'package:flutter/foundation.dart';

import '../bridge/PlatformCoreProxy.dart';
import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../proxy/generated/CoreProxyClients.g.dart';

/// Reflects runtime-owned automatic sync configuration for Flutter settings state.
class RuntimeAutoSyncManager extends ChangeNotifier {
  /// Creates the process-wide automatic sync configuration presenter.
  RuntimeAutoSyncManager._();

  static final RuntimeAutoSyncManager instance = RuntimeAutoSyncManager._();

  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(coreProxy: platformCoreProxy),
  );

  final Set<String> _enabledRemoteNames = <String>{};
  Future<void>? _initializeFuture;

  /// Returns the paired remote names whose runtime-owned automatic sync is enabled.
  Set<String> get enabledRemoteNames {
    return Set<String>.unmodifiable(_enabledRemoteNames);
  }

  /// Returns whether runtime-owned automatic sync is enabled for one paired remote.
  bool isRemoteEnabled(String name) {
    return _enabledRemoteNames.contains(name);
  }

  /// Reads runtime-owned automatic sync configuration once for this Flutter process.
  Future<void> initialize() {
    final activeInitialize = _initializeFuture;
    if (activeInitialize != null) {
      return activeInitialize;
    }
    final initialize = refresh();
    _initializeFuture = initialize;
    return initialize;
  }

  /// Refreshes the local UI cache from the runtime-owned automatic sync configuration.
  Future<void> refresh() async {
    final config = await _clients.runtimeRemoteLinkService.autoSyncConfig();
    _enabledRemoteNames
      ..clear()
      ..addAll(config.autoSyncRemoteNames);
    notifyListeners();
  }

  /// Updates runtime-owned automatic sync for one paired remote and refreshes UI state.
  Future<void> setRemoteEnabled(String name, bool enabled) async {
    await initialize();
    final config = await _clients.runtimeRemoteLinkService
        .setPairedRemoteAutoSync(name: name, enabled: enabled);
    _enabledRemoteNames
      ..clear()
      ..addAll(config.autoSyncRemoteNames);
    notifyListeners();
  }
}
