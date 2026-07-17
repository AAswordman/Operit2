// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';

import '../link/RemoteRuntimeLinkClient.dart';
import '../link_access/LinkAccessHost.dart';
import '../logging/ClientLogger.dart';
import 'RuntimeConnectionManager.dart';
import 'RuntimeDataSyncBridge.dart';

class RuntimeDiscoveredDevice {
  const RuntimeDiscoveredDevice({
    required this.deviceId,
    required this.displayName,
    required this.platform,
    required this.model,
    required this.baseUrl,
    required this.hostname,
    required this.port,
    required this.tokenHash,
    required this.version,
  });

  /// Creates a discovered runtime device from the native discovery payload.
  factory RuntimeDiscoveredDevice.fromJson(Map<String, Object?> json) {
    return RuntimeDiscoveredDevice(
      deviceId: json['device_id'] as String,
      displayName: json['display_name'] as String,
      platform: json['platform'] as String,
      model: json['model'] as String,
      baseUrl: json['base_url'] as String,
      hostname: json['hostname'] as String,
      port: json['port'] as int,
      tokenHash: json['token_hash'] as String,
      version: json['version'] as String,
    );
  }

  final String deviceId;
  final String displayName;
  final String platform;
  final String model;
  final String baseUrl;
  final String hostname;
  final int port;
  final String tokenHash;
  final String version;
}

class RuntimeAutoSyncManager extends ChangeNotifier {
  RuntimeAutoSyncManager._();

  static final RuntimeAutoSyncManager instance = RuntimeAutoSyncManager._();
  static const String _logTag = 'RuntimeAutoSync';
  static const int _discoveryTimeoutMs = 2000;
  static const Duration _scanInterval = Duration(seconds: 60);

  final RuntimeConnectionManager _runtimeManager =
      RuntimeConnectionManager.instance;
  final RuntimeDataSyncBridge _syncBridge = const RuntimeDataSyncBridge();
  final Set<String> _syncingRemoteNames = <String>{};
  final Map<String, RuntimeDataSyncResult> _lastResults =
      <String, RuntimeDataSyncResult>{};
  final Map<String, Object> _lastErrors = <String, Object>{};

  Future<void>? _initializeFuture;
  Timer? _scanTimer;
  bool _scanRunning = false;

  /// Returns the persisted automatic sync remote names from runtime config.
  Set<String> get _enabledRemoteNames {
    return _runtimeManager.config.autoSyncRemoteNames;
  }

  /// Returns the remote names currently enabled for automatic sync.
  Set<String> get enabledRemoteNames {
    return _enabledRemoteNames;
  }

  /// Returns the remote names currently running an automatic sync.
  Set<String> get syncingRemoteNames {
    return Set<String>.unmodifiable(_syncingRemoteNames);
  }

  /// Returns whether automatic sync is enabled for one paired remote name.
  bool isRemoteEnabled(String name) {
    return _enabledRemoteNames.contains(name);
  }

  /// Returns whether one paired remote name is currently syncing.
  bool isRemoteSyncing(String name) {
    return _syncingRemoteNames.contains(name);
  }

  /// Returns the latest successful sync result for one paired remote name.
  RuntimeDataSyncResult? lastResultFor(String name) {
    return _lastResults[name];
  }

  /// Returns the latest sync error for one paired remote name.
  Object? lastErrorFor(String name) {
    return _lastErrors[name];
  }

  /// Loads persisted state and starts the automatic discovery loop.
  Future<void> initialize() {
    final activeInitialize = _initializeFuture;
    if (activeInitialize != null) {
      return activeInitialize;
    }
    final initialize = _initializeOnce();
    _initializeFuture = initialize;
    return initialize;
  }

  /// Enables or disables automatic sync for one paired remote name.
  Future<void> setRemoteEnabled(String name, bool enabled) async {
    await initialize();
    final remoteSession = _runtimeManager.config.remoteSessions[name];
    if (remoteSession == null) {
      throw StateError('paired remote runtime does not exist: $name');
    }
    final names = Set<String>.of(_enabledRemoteNames);
    if (enabled) {
      names.add(name);
    } else {
      names.remove(name);
    }
    await _runtimeManager.storeAutoSyncRemoteNames(names);
    _syncScanTimer();
    notifyListeners();
    if (enabled) {
      unawaited(scanAndSync());
    }
  }

  /// Discovers LAN devices and syncs enabled paired remotes that are visible.
  Future<void> scanAndSync() async {
    await initialize();
    if (_scanRunning || _enabledRemoteNames.isEmpty) {
      return;
    }
    _scanRunning = true;
    try {
      final json = await LinkAccessHost.instance.discoverDevices(
        _discoveryTimeoutMs,
      );
      final devices = (jsonDecode(json) as List<dynamic>)
          .cast<Map<String, Object?>>()
          .map(RuntimeDiscoveredDevice.fromJson)
          .toList(growable: false);
      await syncDiscoveredDevices(devices);
    } catch (error, stackTrace) {
      ClientLogger.w(
        'automatic sync scan failed',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
    } finally {
      _scanRunning = false;
    }
  }

  /// Syncs enabled paired remotes that are present in one discovery result.
  Future<void> syncDiscoveredDevices(
    List<RuntimeDiscoveredDevice> devices,
  ) async {
    await initialize();
    final enabledNames = Set<String>.of(_enabledRemoteNames);
    if (enabledNames.isEmpty) {
      return;
    }
    final localDeviceId = LinkAccessHost.instance.deviceId;
    final deviceByCoreDeviceId = <String, RuntimeDiscoveredDevice>{};
    for (final device in devices) {
      if (device.deviceId == localDeviceId) {
        continue;
      }
      deviceByCoreDeviceId[device.deviceId] = device;
    }
    final sessions = _runtimeManager.config.remoteSessions;
    for (final name in enabledNames) {
      final session = sessions[name];
      if (session == null) {
        continue;
      }
      final device = deviceByCoreDeviceId[session.coreDeviceId];
      if (device == null) {
        continue;
      }
      unawaited(_syncEnabledRemote(name, session, device.baseUrl));
    }
  }

  /// Installs listeners and starts the automatic sync loop once per process.
  Future<void> _initializeOnce() async {
    _runtimeManager.addListener(_handleRuntimeConnectionChanged);
    await _pruneRemovedRemoteNames();
    _syncScanTimer();
    notifyListeners();
    unawaited(scanAndSync());
  }

  /// Responds to paired remote additions and removals.
  void _handleRuntimeConnectionChanged() {
    unawaited(_pruneRemovedRemoteNames());
  }

  /// Removes enabled names that no longer exist in paired remotes.
  Future<void> _pruneRemovedRemoteNames() async {
    final pairedNames = _runtimeManager.config.remoteSessions.keys.toSet();
    final enabledNames = _enabledRemoteNames;
    final retainedNames = enabledNames.where(pairedNames.contains).toSet();
    if (retainedNames.length == enabledNames.length) {
      _syncScanTimer();
      return;
    }
    await _runtimeManager.storeAutoSyncRemoteNames(retainedNames);
    _syncScanTimer();
    notifyListeners();
  }

  /// Starts or stops the periodic discovery timer from current configuration.
  void _syncScanTimer() {
    _scanTimer?.cancel();
    _scanTimer = null;
    if (_enabledRemoteNames.isEmpty) {
      return;
    }
    _scanTimer = Timer.periodic(_scanInterval, (_) => unawaited(scanAndSync()));
  }

  /// Runs one verified sync for a discovered paired remote.
  Future<void> _syncEnabledRemote(
    String name,
    PairedRemoteSessionRecord session,
    String baseUrl,
  ) async {
    if (_syncingRemoteNames.contains(name)) {
      return;
    }
    _syncingRemoteNames.add(name);
    notifyListeners();
    try {
      await _runtimeManager.storeVerifiedRemoteBaseUrl(
        name: name,
        baseUrl: baseUrl,
      );
      final updatedSession = _runtimeManager.config.remoteSessions[name];
      if (updatedSession == null) {
        throw StateError('paired remote runtime does not exist: $name');
      }
      final result = await _syncBridge.syncPairedRemote(
        session: updatedSession,
      );
      _lastResults[name] = result;
      _lastErrors.remove(name);
      ClientLogger.i(
        'automatic sync completed name=$name localApplied=${result.localApplied} remoteApplied=${result.remoteApplied} rounds=${result.rounds}',
        tag: _logTag,
      );
    } catch (error, stackTrace) {
      _lastErrors[name] = error;
      ClientLogger.w(
        'automatic sync failed name=$name',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
    } finally {
      _syncingRemoteNames.remove(name);
      notifyListeners();
    }
  }
}
