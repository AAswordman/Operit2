// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import '../bridge/CoreProxy.dart';
import '../bridge/PlatformCoreProxy.dart';
import '../link/CoreLinkProtocol.dart';
import '../link/RemoteRuntimeLinkClient.dart';
import 'RuntimeConnectionConfigStore.dart';

enum RuntimeConnectionMode { local, remote }

class LocalRuntimeStorageConfig {
  const LocalRuntimeStorageConfig({
    required this.confirmed,
    required this.storageRoot,
    required this.updatedAt,
  });

  /// Creates a config that keeps the platform-provided storage root.
  factory LocalRuntimeStorageConfig.platformDefault() {
    return LocalRuntimeStorageConfig(
      confirmed: false,
      storageRoot: '',
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
  }

  /// Creates a config from persisted JSON.
  factory LocalRuntimeStorageConfig.fromJson(Map<String, Object?> json) {
    return LocalRuntimeStorageConfig(
      confirmed: json['confirmed'] as bool,
      storageRoot: json['storageRoot'] as String,
      updatedAt: json['updatedAt'] as int,
    );
  }

  final bool confirmed;
  final String storageRoot;
  final int updatedAt;

  /// Returns true when the config names a custom storage root.
  bool get hasCustomStorageRoot => storageRoot.trim().isNotEmpty;

  /// Creates a copy with updated fields.
  LocalRuntimeStorageConfig copyWith({
    bool? confirmed,
    String? storageRoot,
    int? updatedAt,
  }) {
    return LocalRuntimeStorageConfig(
      confirmed: confirmed ?? this.confirmed,
      storageRoot: storageRoot ?? this.storageRoot,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }

  /// Converts the config into persisted JSON.
  Map<String, Object?> toJson() {
    return {
      'confirmed': confirmed,
      'storageRoot': storageRoot,
      'updatedAt': updatedAt,
    };
  }
}

class RuntimeStoragePaths {
  const RuntimeStoragePaths({
    required this.storageRoot,
    required this.runtimeRoot,
    required this.workspaceRoot,
  });

  /// Creates a storage path snapshot from native channel values.
  factory RuntimeStoragePaths.fromMap(Map<Object?, Object?> map) {
    return RuntimeStoragePaths(
      storageRoot: map['storageRoot'] as String,
      runtimeRoot: map['runtimeRoot'] as String,
      workspaceRoot: map['workspaceRoot'] as String,
    );
  }

  final String storageRoot;
  final String runtimeRoot;
  final String workspaceRoot;
}

class LocalRuntimeStorageBridge {
  const LocalRuntimeStorageBridge._();

  static const MethodChannel _channel = MethodChannel('operit/runtime');

  /// Reads native runtime storage paths for the supplied client config.
  static Future<RuntimeStoragePaths> pathsForConfig(
    LocalRuntimeStorageConfig config,
  ) async {
    if (kIsWeb) {
      final root = config.storageRoot.trim().isEmpty
          ? 'browser-storage'
          : config.storageRoot.trim();
      return RuntimeStoragePaths(
        storageRoot: root,
        runtimeRoot: '$root/runtime',
        workspaceRoot: '$root/workspaces',
      );
    }
    final result = await _channel.invokeMapMethod<Object?, Object?>(
      'localRuntimeStoragePaths',
      <String, Object?>{'storageRoot': config.storageRoot},
    );
    if (result == null) {
      throw StateError('local runtime storage paths response is empty');
    }
    return RuntimeStoragePaths.fromMap(result);
  }

  /// Applies the local runtime storage config before the runtime is created.
  static Future<void> apply(LocalRuntimeStorageConfig config) async {
    if (kIsWeb) {
      return;
    }
    await _channel.invokeMethod<void>(
      'setLocalRuntimeStorage',
      <String, Object?>{'storageRoot': config.storageRoot},
    );
  }
}

class RuntimeConnectionConfig {
  const RuntimeConnectionConfig({
    required this.mode,
    required this.activeRemoteName,
    required this.remoteSessions,
    required this.localStorage,
    required this.updatedAt,
  });

  factory RuntimeConnectionConfig.local() {
    return RuntimeConnectionConfig(
      mode: RuntimeConnectionMode.local,
      activeRemoteName: '',
      remoteSessions: const <String, PairedRemoteSessionRecord>{},
      localStorage: LocalRuntimeStorageConfig.platformDefault(),
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
  }

  factory RuntimeConnectionConfig.fromJson(
    Map<String, Object?> json, {
    Map<String, PairedRemoteSessionRecord> remoteSessions =
        const <String, PairedRemoteSessionRecord>{},
  }) {
    final modeName = json['mode'] as String;
    return RuntimeConnectionConfig(
      mode: RuntimeConnectionMode.values.byName(modeName),
      activeRemoteName: json['activeRemoteName'] as String,
      remoteSessions: Map<String, PairedRemoteSessionRecord>.unmodifiable(
        remoteSessions,
      ),
      localStorage: LocalRuntimeStorageConfig.platformDefault(),
      updatedAt: json['updatedAt'] as int,
    );
  }

  final RuntimeConnectionMode mode;
  final String activeRemoteName;
  final Map<String, PairedRemoteSessionRecord> remoteSessions;
  final LocalRuntimeStorageConfig localStorage;
  final int updatedAt;

  PairedRemoteSessionRecord? get activeRemoteSession {
    return remoteSessions[activeRemoteName];
  }

  RuntimeConnectionConfig copyWith({
    RuntimeConnectionMode? mode,
    String? activeRemoteName,
    Map<String, PairedRemoteSessionRecord>? remoteSessions,
    LocalRuntimeStorageConfig? localStorage,
    int? updatedAt,
  }) {
    return RuntimeConnectionConfig(
      mode: mode ?? this.mode,
      activeRemoteName: activeRemoteName ?? this.activeRemoteName,
      remoteSessions: remoteSessions ?? this.remoteSessions,
      localStorage: localStorage ?? this.localStorage,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }

  Map<String, Object?> toJson() {
    return {
      'mode': mode.name,
      'activeRemoteName': activeRemoteName,
      'updatedAt': updatedAt,
    };
  }
}

class RuntimeConnectionManager extends ChangeNotifier {
  RuntimeConnectionManager._();

  static final RuntimeConnectionManager instance = RuntimeConnectionManager._();
  static const Duration _remoteStartupProbeTimeout = Duration(seconds: 4);
  static const Duration _remoteIssueProbeDelay = Duration(milliseconds: 700);
  static const Duration _remoteIssueProbeTimeout = Duration(seconds: 2);
  static const int _remoteIssueProbeAttempts = 3;

  RuntimeConnectionConfig _config = RuntimeConnectionConfig.local();
  RemoteRuntimeLinkClient? _remoteLinkClient;
  CoreLinkError? _pendingRemoteError;
  bool _remoteIssueProbeRunning = false;

  RuntimeConnectionConfig get config => _config;

  CoreProxy get coreProxy {
    return switch (_config.mode) {
      RuntimeConnectionMode.local => platformCoreProxy,
      RuntimeConnectionMode.remote => _remoteLinkClient!,
    };
  }

  CoreLinkError? consumePendingRemoteError() {
    final error = _pendingRemoteError;
    _pendingRemoteError = null;
    return error;
  }

  void _onRemoteLinkConnectionIssue(CoreLinkError error) {
    final linkClient = _remoteLinkClient;
    if (_config.mode != RuntimeConnectionMode.remote || linkClient == null) {
      return;
    }
    if (_remoteIssueProbeRunning) {
      return;
    }
    _remoteIssueProbeRunning = true;
    unawaited(_confirmRemoteConnection(error, linkClient));
  }

  Future<void> _confirmRemoteConnection(
    CoreLinkError firstError,
    RemoteRuntimeLinkClient linkClient,
  ) async {
    var latestError = firstError;
    try {
      for (var attempt = 0; attempt < _remoteIssueProbeAttempts; attempt++) {
        await Future<void>.delayed(_remoteIssueProbeDelay);
        if (_config.mode != RuntimeConnectionMode.remote ||
            !identical(_remoteLinkClient, linkClient)) {
          return;
        }
        try {
          await _verifyRemoteSession(
            linkClient,
            linkClient.session,
            _remoteIssueProbeTimeout,
          );
          return;
        } catch (error) {
          latestError = _asCoreLinkError(error, 'REMOTE_CONNECT_FAILED');
        }
      }
      if (_config.mode != RuntimeConnectionMode.remote ||
          !identical(_remoteLinkClient, linkClient)) {
        return;
      }
      _pendingRemoteError = latestError;
      await _apply(
        _config.copyWith(
          mode: RuntimeConnectionMode.local,
          activeRemoteName: '',
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        ),
        persist: true,
      );
    } finally {
      _remoteIssueProbeRunning = false;
    }
  }

  CoreLinkError _asCoreLinkError(Object error, String code) {
    return error is CoreLinkError
        ? error
        : CoreLinkError(code: code, message: error.toString());
  }

  Future<void> _verifyRemoteSession(
    RemoteRuntimeLinkClient linkClient,
    PairedRemoteSessionRecord session,
    Duration timeout,
  ) async {
    final info = await linkClient.sessionInfo().timeout(timeout);
    if (info.coreDeviceId != session.coreDeviceId) {
      throw CoreLinkError(
        code: 'REMOTE_DEVICE_CHANGED',
        message: 'remote runtime identity changed',
      );
    }
  }

  Future<void> initialize() async {
    final storedConfig = await RuntimeConnectionConfigStore.read();
    await LocalRuntimeStorageBridge.apply(storedConfig.localStorage);
    if (storedConfig.mode == RuntimeConnectionMode.remote) {
      await _applyRemote(storedConfig, persist: false, verify: true);
      return;
    }
    await _apply(storedConfig, persist: false);
  }

  /// Returns native storage paths for the stored local runtime config.
  Future<RuntimeStoragePaths> localRuntimeStoragePaths() {
    return LocalRuntimeStorageBridge.pathsForConfig(_config.localStorage);
  }

  /// Returns native storage paths for a candidate local runtime root.
  Future<RuntimeStoragePaths> localRuntimeStoragePathsForRoot(
    String storageRoot,
  ) {
    return LocalRuntimeStorageBridge.pathsForConfig(
      _config.localStorage.copyWith(storageRoot: storageRoot),
    );
  }

  /// Confirms and persists the local runtime storage root.
  Future<void> confirmLocalRuntimeStorage(String storageRoot) async {
    final localStorage = LocalRuntimeStorageConfig(
      confirmed: true,
      storageRoot: storageRoot.trim(),
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    await LocalRuntimeStorageBridge.apply(localStorage);
    await _apply(
      _config.copyWith(
        mode: RuntimeConnectionMode.local,
        activeRemoteName: '',
        localStorage: localStorage,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      ),
      persist: true,
    );
  }

  /// Persists a migrated local runtime storage root.
  Future<void> persistMigratedLocalRuntimeStorage(String storageRoot) async {
    final localStorage = LocalRuntimeStorageConfig(
      confirmed: true,
      storageRoot: storageRoot.trim(),
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    final config = _config.copyWith(
      localStorage: localStorage,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    _config = config;
    await OutboundLinkSessionStore.write(config.remoteSessions);
    await RuntimeConnectionConfigStore.write(config);
    notifyListeners();
  }

  Future<void> setLocal() async {
    await _apply(
      _config.copyWith(
        mode: RuntimeConnectionMode.local,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      ),
      persist: true,
    );
  }

  Future<bool> setRemote({
    required String name,
    required PairedRemoteSessionRecord session,
  }) async {
    final remoteSessions = Map<String, PairedRemoteSessionRecord>.of(
      _config.remoteSessions,
    )..[name] = session;
    final remoteConfig = RuntimeConnectionConfig(
      mode: RuntimeConnectionMode.remote,
      activeRemoteName: name,
      remoteSessions: Map<String, PairedRemoteSessionRecord>.unmodifiable(
        remoteSessions,
      ),
      localStorage: _config.localStorage,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    return _applyRemote(remoteConfig, persist: true, verify: true);
  }

  Future<bool> usePairedRemote(String name) async {
    if (!_config.remoteSessions.containsKey(name)) {
      throw StateError('paired remote runtime does not exist: $name');
    }
    final remoteConfig = _config.copyWith(
      mode: RuntimeConnectionMode.remote,
      activeRemoteName: name,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    return _applyRemote(remoteConfig, persist: true, verify: true);
  }

  Future<void> removePairedRemote(String name) async {
    if (!_config.remoteSessions.containsKey(name)) {
      throw StateError('paired remote runtime does not exist: $name');
    }
    final remoteSessions = Map<String, PairedRemoteSessionRecord>.of(
      _config.remoteSessions,
    )..remove(name);
    final activeRemoved =
        _config.mode == RuntimeConnectionMode.remote &&
        _config.activeRemoteName == name;
    final next = RuntimeConnectionConfig(
      mode: activeRemoved ? RuntimeConnectionMode.local : _config.mode,
      activeRemoteName: activeRemoved ? '' : _config.activeRemoteName,
      remoteSessions: Map<String, PairedRemoteSessionRecord>.unmodifiable(
        remoteSessions,
      ),
      localStorage: _config.localStorage,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    await _apply(next, persist: true);
  }

  Future<void> _apply(
    RuntimeConnectionConfig config, {
    required bool persist,
  }) async {
    _remoteLinkClient?.dispose();
    _remoteLinkClient = null;
    if (config.mode == RuntimeConnectionMode.remote) {
      final session = config.activeRemoteSession;
      if (session == null) {
        throw StateError('remote runtime session is required');
      }
      _remoteLinkClient = RemoteRuntimeLinkClient(
        session: session,
        onConnectionIssue: _onRemoteLinkConnectionIssue,
      );
    }
    _config = config;
    if (persist) {
      await OutboundLinkSessionStore.write(config.remoteSessions);
      await RuntimeConnectionConfigStore.write(config);
    }
    notifyListeners();
  }

  Future<bool> _applyRemote(
    RuntimeConnectionConfig config, {
    required bool persist,
    required bool verify,
  }) async {
    _remoteLinkClient?.dispose();
    _remoteLinkClient = null;
    final session = config.activeRemoteSession;
    if (session == null) {
      throw StateError('remote runtime session is required');
    }
    final linkClient = RemoteRuntimeLinkClient(session: session);
    try {
      if (verify) {
        await _verifyRemoteSession(
          linkClient,
          session,
          _remoteStartupProbeTimeout,
        );
      }
      linkClient.setConnectionIssueHandler(_onRemoteLinkConnectionIssue);
      _remoteLinkClient = linkClient;
      _config = config;
      if (persist) {
        await OutboundLinkSessionStore.write(config.remoteSessions);
        await RuntimeConnectionConfigStore.write(config);
      }
      notifyListeners();
      return true;
    } catch (error) {
      linkClient.dispose();
      _pendingRemoteError = _asCoreLinkError(error, 'REMOTE_CONNECT_FAILED');
      await _apply(
        RuntimeConnectionConfig(
          mode: RuntimeConnectionMode.local,
          activeRemoteName: '',
          remoteSessions: config.remoteSessions,
          localStorage: config.localStorage,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        ),
        persist: persist,
      );
      return false;
    }
  }
}
