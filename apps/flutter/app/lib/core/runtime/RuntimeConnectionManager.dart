// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import '../bridge/CoreProxy.dart';
import '../bridge/PlatformCoreProxy.dart';
import '../link/CoreLinkProtocol.dart';
import '../link/RemoteRuntimeLinkClient.dart';
import '../link_host/LinkHostServer.dart';
import '../logging/ClientLogger.dart';
import 'RuntimeConnectionConfigStore.dart';

enum RuntimeConnectionMode { local, remote }

class LocalRuntimeStorageConfig {
  const LocalRuntimeStorageConfig({
    required this.confirmed,
    required this.runtimeRoot,
    required this.workspaceRoot,
    required this.updatedAt,
  });

  /// Creates an unconfirmed local runtime storage config.
  factory LocalRuntimeStorageConfig.platformDefault() {
    return LocalRuntimeStorageConfig(
      confirmed: false,
      runtimeRoot: '',
      workspaceRoot: '',
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
  }

  /// Creates a config from persisted JSON.
  factory LocalRuntimeStorageConfig.fromJson(Map<String, Object?> json) {
    return LocalRuntimeStorageConfig(
      confirmed: json['confirmed'] as bool,
      runtimeRoot: json['runtimeRoot'] as String,
      workspaceRoot: json['workspaceRoot'] as String,
      updatedAt: json['updatedAt'] as int,
    );
  }

  final bool confirmed;
  final String runtimeRoot;
  final String workspaceRoot;
  final int updatedAt;

  /// Creates a copy with updated fields.
  LocalRuntimeStorageConfig copyWith({
    bool? confirmed,
    String? runtimeRoot,
    String? workspaceRoot,
    int? updatedAt,
  }) {
    return LocalRuntimeStorageConfig(
      confirmed: confirmed ?? this.confirmed,
      runtimeRoot: runtimeRoot ?? this.runtimeRoot,
      workspaceRoot: workspaceRoot ?? this.workspaceRoot,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }

  /// Converts the config into persisted JSON.
  Map<String, Object?> toJson() {
    return {
      'confirmed': confirmed,
      'runtimeRoot': runtimeRoot,
      'workspaceRoot': workspaceRoot,
      'updatedAt': updatedAt,
    };
  }
}

class RuntimeStoragePaths {
  const RuntimeStoragePaths({
    required this.runtimeRoot,
    required this.workspaceRoot,
  });

  /// Creates a storage path snapshot from native channel values.
  factory RuntimeStoragePaths.fromMap(Map<Object?, Object?> map) {
    return RuntimeStoragePaths(
      runtimeRoot: map['runtimeRoot'] as String,
      workspaceRoot: map['workspaceRoot'] as String,
    );
  }

  final String runtimeRoot;
  final String workspaceRoot;
}

class LocalRuntimeStorageBridge {
  const LocalRuntimeStorageBridge._();

  static const MethodChannel _channel = MethodChannel('operit/runtime');

  /// Reads the platform default runtime and workspace roots.
  static Future<RuntimeStoragePaths> defaultPaths() async {
    if (kIsWeb) {
      return const RuntimeStoragePaths(
        runtimeRoot: 'browser-storage/runtime',
        workspaceRoot: 'browser-storage/workspaces',
      );
    }
    final result = await _channel.invokeMapMethod<Object?, Object?>(
      'localRuntimeStorageDefaults',
    );
    if (result == null) {
      throw StateError('local runtime storage defaults response is empty');
    }
    return RuntimeStoragePaths.fromMap(result);
  }

  /// Validates and resolves explicit runtime and workspace roots.
  static Future<RuntimeStoragePaths> pathsForRoots(
    String runtimeRoot,
    String workspaceRoot,
  ) async {
    final normalizedRuntimeRoot = runtimeRoot.trim();
    final normalizedWorkspaceRoot = workspaceRoot.trim();
    if (normalizedRuntimeRoot.isEmpty || normalizedWorkspaceRoot.isEmpty) {
      throw ArgumentError('runtime and workspace roots must not be empty');
    }
    if (kIsWeb) {
      return RuntimeStoragePaths(
        runtimeRoot: normalizedRuntimeRoot,
        workspaceRoot: normalizedWorkspaceRoot,
      );
    }
    final result = await _channel.invokeMapMethod<Object?, Object?>(
      'localRuntimeStoragePaths',
      <String, Object?>{
        'runtimeRoot': normalizedRuntimeRoot,
        'workspaceRoot': normalizedWorkspaceRoot,
      },
    );
    if (result == null) {
      throw StateError('local runtime storage paths response is empty');
    }
    return RuntimeStoragePaths.fromMap(result);
  }

  /// Installs local storage roots and permits repeated identical configuration.
  static Future<void> apply(LocalRuntimeStorageConfig config) async {
    if (!config.confirmed) {
      throw StateError('local runtime storage config is not confirmed');
    }
    if (kIsWeb) {
      return;
    }
    await _channel.invokeMethod<void>(
      'setLocalRuntimeStorage',
      <String, Object?>{
        'runtimeRoot': config.runtimeRoot,
        'workspaceRoot': config.workspaceRoot,
      },
    );
  }
}

class RuntimeConnectionConfig {
  const RuntimeConnectionConfig({
    required this.mode,
    required this.activeRemoteName,
    required this.remoteSessions,
    required this.autoSyncRemoteNames,
    required this.localStorage,
    required this.updatedAt,
  });

  factory RuntimeConnectionConfig.local() {
    return RuntimeConnectionConfig(
      mode: RuntimeConnectionMode.local,
      activeRemoteName: '',
      remoteSessions: const <String, PairedRemoteSessionRecord>{},
      autoSyncRemoteNames: const <String>{},
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
      autoSyncRemoteNames: Set<String>.unmodifiable(
        ((json['autoSyncRemoteNames'] as List<Object?>?) ?? const <Object?>[])
            .cast<String>(),
      ),
      localStorage: LocalRuntimeStorageConfig.platformDefault(),
      updatedAt: json['updatedAt'] as int,
    );
  }

  final RuntimeConnectionMode mode;
  final String activeRemoteName;
  final Map<String, PairedRemoteSessionRecord> remoteSessions;
  final Set<String> autoSyncRemoteNames;
  final LocalRuntimeStorageConfig localStorage;
  final int updatedAt;

  PairedRemoteSessionRecord? get activeRemoteSession {
    return remoteSessions[activeRemoteName];
  }

  RuntimeConnectionConfig copyWith({
    RuntimeConnectionMode? mode,
    String? activeRemoteName,
    Map<String, PairedRemoteSessionRecord>? remoteSessions,
    Set<String>? autoSyncRemoteNames,
    LocalRuntimeStorageConfig? localStorage,
    int? updatedAt,
  }) {
    return RuntimeConnectionConfig(
      mode: mode ?? this.mode,
      activeRemoteName: activeRemoteName ?? this.activeRemoteName,
      remoteSessions: remoteSessions ?? this.remoteSessions,
      autoSyncRemoteNames: autoSyncRemoteNames ?? this.autoSyncRemoteNames,
      localStorage: localStorage ?? this.localStorage,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }

  Map<String, Object?> toJson() {
    return {
      'mode': mode.name,
      'activeRemoteName': activeRemoteName,
      'autoSyncRemoteNames': autoSyncRemoteNames.toList(growable: false),
      'updatedAt': updatedAt,
    };
  }
}

class RuntimeConnectionManager extends ChangeNotifier {
  RuntimeConnectionManager._();

  static final RuntimeConnectionManager instance = RuntimeConnectionManager._();
  static const String _logTag = 'RuntimeConnection';
  static const int _remoteStartupDiscoveryTimeoutMs = 2000;
  static const Duration _remoteStartupProbeTimeout = Duration(seconds: 4);
  static const Duration _remoteIssueProbeDelay = Duration(milliseconds: 700);
  static const Duration _remoteIssueProbeTimeout = Duration(seconds: 2);
  static const int _remoteIssueProbeAttempts = 3;

  RuntimeConnectionConfig _config = RuntimeConnectionConfig.local();
  RemoteRuntimeLinkClient? _remoteLinkClient;
  CoreLinkError? _pendingRemoteError;
  bool _remoteIssueProbeRunning = false;

  RuntimeConnectionConfig get config => _config;

  /// Returns whether the selected runtime can accept core calls.
  bool get runtimeConfigured {
    return _config.mode == RuntimeConnectionMode.remote ||
        _config.localStorage.confirmed;
  }

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
    ClientLogger.w(
      'remote link issue received code=${error.code} message=${error.message}',
      tag: _logTag,
    );
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
    final stopwatch = Stopwatch()..start();
    var latestError = firstError;
    try {
      for (var attempt = 0; attempt < _remoteIssueProbeAttempts; attempt++) {
        ClientLogger.d(
          'remote issue probe start attempt=${attempt + 1} code=${latestError.code}',
          tag: _logTag,
        );
        await Future<void>.delayed(_remoteIssueProbeDelay);
        if (_config.mode != RuntimeConnectionMode.remote ||
            !identical(_remoteLinkClient, linkClient)) {
          ClientLogger.i(
            'remote issue probe stopped mode=${_config.mode.name} elapsedMs=${stopwatch.elapsedMilliseconds}',
            tag: _logTag,
          );
          return;
        }
        try {
          await _verifyRemoteSession(
            linkClient,
            linkClient.session,
            _remoteIssueProbeTimeout,
          );
          ClientLogger.i(
            'remote issue probe recovered attempt=${attempt + 1} elapsedMs=${stopwatch.elapsedMilliseconds}',
            tag: _logTag,
          );
          return;
        } catch (error) {
          latestError = _asCoreLinkError(error, 'REMOTE_CONNECT_FAILED');
          ClientLogger.w(
            'remote issue probe failed attempt=${attempt + 1} code=${latestError.code} message=${latestError.message}',
            tag: _logTag,
          );
        }
      }
      if (_config.mode != RuntimeConnectionMode.remote ||
          !identical(_remoteLinkClient, linkClient)) {
        return;
      }
      _pendingRemoteError = latestError;
      ClientLogger.e(
        'remote issue confirmed code=${latestError.code} message=${latestError.message} elapsedMs=${stopwatch.elapsedMilliseconds}',
        tag: _logTag,
      );
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
    final stopwatch = Stopwatch()..start();
    ClientLogger.d(
      'verify remote session start session=${session.sessionId} coreDevice=${session.coreDeviceId} timeoutMs=${timeout.inMilliseconds}',
      tag: _logTag,
    );
    final info = await linkClient.sessionInfo().timeout(timeout);
    if (info.coreDeviceId != session.coreDeviceId) {
      throw CoreLinkError(
        code: 'REMOTE_DEVICE_CHANGED',
        message: 'remote runtime identity changed',
      );
    }
    ClientLogger.i(
      'verify remote session done session=${session.sessionId} coreDevice=${session.coreDeviceId} elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
    );
  }

  /// Loads persisted runtime configuration and applies the selected runtime.
  Future<void> initialize() async {
    final stopwatch = Stopwatch()..start();
    ClientLogger.i('initialize start', tag: _logTag);
    try {
      final readStopwatch = Stopwatch()..start();
      final storedConfig = await RuntimeConnectionConfigStore.read();
      ClientLogger.i(
        'config read done mode=${storedConfig.mode.name} localConfirmed=${storedConfig.localStorage.confirmed} remoteCount=${storedConfig.remoteSessions.length} elapsedMs=${readStopwatch.elapsedMilliseconds}',
        tag: _logTag,
      );
      if (storedConfig.localStorage.confirmed) {
        final storageStopwatch = Stopwatch()..start();
        ClientLogger.i(
          'local storage apply start runtimeRoot=${storedConfig.localStorage.runtimeRoot} workspaceRoot=${storedConfig.localStorage.workspaceRoot}',
          tag: _logTag,
        );
        await LocalRuntimeStorageBridge.apply(storedConfig.localStorage);
        ClientLogger.i(
          'local storage apply done elapsedMs=${storageStopwatch.elapsedMilliseconds}',
          tag: _logTag,
        );
      }
      if (storedConfig.mode == RuntimeConnectionMode.remote) {
        final applied = await _applyRemote(
          storedConfig,
          persist: false,
          verify: true,
        );
        ClientLogger.i(
          'initialize done mode=${_config.mode.name} remoteApplied=$applied elapsedMs=${stopwatch.elapsedMilliseconds}',
          tag: _logTag,
        );
        return;
      }
      await _apply(storedConfig, persist: false);
      ClientLogger.i(
        'initialize done mode=${_config.mode.name} elapsedMs=${stopwatch.elapsedMilliseconds}',
        tag: _logTag,
      );
    } catch (error, stackTrace) {
      ClientLogger.e(
        'initialize failed elapsedMs=${stopwatch.elapsedMilliseconds}',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
      rethrow;
    }
  }

  /// Returns native storage paths for the stored local runtime config.
  Future<RuntimeStoragePaths> localRuntimeStoragePaths() {
    final localStorage = _config.localStorage;
    if (!localStorage.confirmed) {
      throw StateError('local runtime storage config is not confirmed');
    }
    return LocalRuntimeStorageBridge.pathsForRoots(
      localStorage.runtimeRoot,
      localStorage.workspaceRoot,
    );
  }

  /// Returns the platform default runtime and workspace roots.
  Future<RuntimeStoragePaths> localRuntimeStorageDefaultPaths() {
    return LocalRuntimeStorageBridge.defaultPaths();
  }

  /// Returns native storage paths for candidate runtime and workspace roots.
  Future<RuntimeStoragePaths> localRuntimeStoragePathsForRoots(
    String runtimeRoot,
    String workspaceRoot,
  ) {
    return LocalRuntimeStorageBridge.pathsForRoots(runtimeRoot, workspaceRoot);
  }

  /// Confirms and persists the local runtime and workspace roots.
  Future<void> confirmLocalRuntimeStorage(
    String runtimeRoot,
    String workspaceRoot,
  ) async {
    final stopwatch = Stopwatch()..start();
    ClientLogger.i(
      'confirm local runtime storage start runtimeRoot=$runtimeRoot workspaceRoot=$workspaceRoot',
      tag: _logTag,
    );
    final localStorage = LocalRuntimeStorageConfig(
      confirmed: true,
      runtimeRoot: runtimeRoot.trim(),
      workspaceRoot: workspaceRoot.trim(),
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
    ClientLogger.i(
      'confirm local runtime storage done elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
    );
  }

  /// Persists migrated local runtime and workspace roots.
  Future<void> persistMigratedLocalRuntimeStorage(
    String runtimeRoot,
    String workspaceRoot,
  ) async {
    final stopwatch = Stopwatch()..start();
    ClientLogger.i(
      'persist migrated local runtime storage start runtimeRoot=$runtimeRoot workspaceRoot=$workspaceRoot',
      tag: _logTag,
    );
    final localStorage = LocalRuntimeStorageConfig(
      confirmed: true,
      runtimeRoot: runtimeRoot.trim(),
      workspaceRoot: workspaceRoot.trim(),
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
    ClientLogger.i(
      'persist migrated local runtime storage done elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
    );
  }

  /// Selects the local runtime connection.
  Future<void> setLocal() async {
    ClientLogger.i('set local start', tag: _logTag);
    await _apply(
      _config.copyWith(
        mode: RuntimeConnectionMode.local,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      ),
      persist: true,
    );
    ClientLogger.i('set local done mode=${_config.mode.name}', tag: _logTag);
  }

  /// Stores and selects a paired remote runtime.
  Future<bool> setRemote({
    required String name,
    required PairedRemoteSessionRecord session,
  }) async {
    ClientLogger.i('set remote start name=$name', tag: _logTag);
    final remoteSessions = Map<String, PairedRemoteSessionRecord>.of(
      _config.remoteSessions,
    )..[name] = session;
    final remoteConfig = RuntimeConnectionConfig(
      mode: RuntimeConnectionMode.remote,
      activeRemoteName: name,
      remoteSessions: Map<String, PairedRemoteSessionRecord>.unmodifiable(
        remoteSessions,
      ),
      autoSyncRemoteNames: _config.autoSyncRemoteNames,
      localStorage: _config.localStorage,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    final result = await _applyRemote(
      remoteConfig,
      persist: true,
      verify: true,
    );
    ClientLogger.i(
      'set remote done name=$name applied=$result mode=${_config.mode.name}',
      tag: _logTag,
    );
    return result;
  }

  /// Selects one existing paired remote runtime.
  Future<bool> usePairedRemote(String name) async {
    ClientLogger.i('use paired remote start name=$name', tag: _logTag);
    if (!_config.remoteSessions.containsKey(name)) {
      throw StateError('paired remote runtime does not exist: $name');
    }
    final remoteConfig = _config.copyWith(
      mode: RuntimeConnectionMode.remote,
      activeRemoteName: name,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    final result = await _applyRemote(
      remoteConfig,
      persist: true,
      verify: true,
    );
    ClientLogger.i(
      'use paired remote done name=$name applied=$result mode=${_config.mode.name}',
      tag: _logTag,
    );
    return result;
  }

  /// Verifies and stores a changed LAN endpoint for one paired remote runtime.
  Future<bool> storeVerifiedRemoteBaseUrl({
    required String name,
    required String baseUrl,
  }) async {
    final existing = _config.remoteSessions[name];
    if (existing == null) {
      throw StateError('paired remote runtime does not exist: $name');
    }
    final updated = existing.withBaseUrl(baseUrl);
    if (updated.baseUrl == existing.baseUrl) {
      return false;
    }
    final linkClient = RemoteRuntimeLinkClient(session: updated);
    try {
      await _verifyRemoteSession(
        linkClient,
        updated,
        _remoteStartupProbeTimeout,
      );
    } finally {
      linkClient.dispose();
    }
    final remoteSessions = Map<String, PairedRemoteSessionRecord>.of(
      _config.remoteSessions,
    )..[name] = updated;
    await _apply(
      _config.copyWith(
        remoteSessions: Map<String, PairedRemoteSessionRecord>.unmodifiable(
          remoteSessions,
        ),
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      ),
      persist: true,
    );
    ClientLogger.i(
      'paired remote baseUrl updated name=$name baseUrl=${updated.baseUrl}',
      tag: _logTag,
    );
    return true;
  }

  /// Stores the remote names that should participate in automatic sync.
  Future<void> storeAutoSyncRemoteNames(Set<String> names) async {
    final existingNames = _config.remoteSessions.keys.toSet();
    final retainedNames = names.where(existingNames.contains).toSet();
    final config = _config.copyWith(
      autoSyncRemoteNames: Set<String>.unmodifiable(retainedNames),
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    _config = config;
    await RuntimeConnectionConfigStore.write(config);
    notifyListeners();
  }

  /// Removes one paired remote runtime record.
  Future<void> removePairedRemote(String name) async {
    ClientLogger.i('remove paired remote start name=$name', tag: _logTag);
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
      autoSyncRemoteNames: Set<String>.unmodifiable(
        _config.autoSyncRemoteNames.where((value) => value != name),
      ),
      localStorage: _config.localStorage,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    await _apply(next, persist: true);
    ClientLogger.i(
      'remove paired remote done name=$name mode=${_config.mode.name}',
      tag: _logTag,
    );
  }

  /// Applies one runtime connection configuration.
  Future<void> _apply(
    RuntimeConnectionConfig config, {
    required bool persist,
  }) async {
    final stopwatch = Stopwatch()..start();
    ClientLogger.d(
      'apply start mode=${config.mode.name} persist=$persist remoteName=${config.activeRemoteName}',
      tag: _logTag,
    );
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
    ClientLogger.i(
      'apply done mode=${_config.mode.name} persist=$persist elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
    );
  }

  /// Applies and verifies one remote runtime connection configuration.
  Future<bool> _applyRemote(
    RuntimeConnectionConfig config, {
    required bool persist,
    required bool verify,
  }) async {
    final stopwatch = Stopwatch()..start();
    ClientLogger.i(
      'apply remote start name=${config.activeRemoteName} persist=$persist verify=$verify',
      tag: _logTag,
    );
    _remoteLinkClient?.dispose();
    _remoteLinkClient = null;
    final session = config.activeRemoteSession;
    if (session == null) {
      throw StateError('remote runtime session is required');
    }
    final resolution = await _remoteConfigWithDiscoveredBaseUrl(config);
    final resolvedConfig = resolution.config;
    final resolvedSession = resolvedConfig.activeRemoteSession;
    if (resolvedSession == null) {
      throw StateError('remote runtime session is required');
    }
    final linkClient = RemoteRuntimeLinkClient(session: resolvedSession);
    try {
      if (verify) {
        await _verifyRemoteSession(
          linkClient,
          resolvedSession,
          _remoteStartupProbeTimeout,
        );
      }
      linkClient.setConnectionIssueHandler(_onRemoteLinkConnectionIssue);
      _remoteLinkClient = linkClient;
      _config = resolvedConfig;
      if (persist || resolution.baseUrlChanged) {
        await OutboundLinkSessionStore.write(resolvedConfig.remoteSessions);
        await RuntimeConnectionConfigStore.write(resolvedConfig);
      }
      notifyListeners();
      ClientLogger.i(
        'apply remote done name=${resolvedConfig.activeRemoteName} elapsedMs=${stopwatch.elapsedMilliseconds}',
        tag: _logTag,
      );
      return true;
    } catch (error) {
      linkClient.dispose();
      _pendingRemoteError = _asCoreLinkError(error, 'REMOTE_CONNECT_FAILED');
      ClientLogger.e(
        'apply remote failed name=${config.activeRemoteName} elapsedMs=${stopwatch.elapsedMilliseconds}',
        tag: _logTag,
        error: error,
      );
      await _apply(
        RuntimeConnectionConfig(
          mode: RuntimeConnectionMode.local,
          activeRemoteName: '',
          remoteSessions: config.remoteSessions,
          autoSyncRemoteNames: config.autoSyncRemoteNames,
          localStorage: config.localStorage,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        ),
        persist: persist,
      );
      return false;
    }
  }

  /// Builds a remote config with the currently announced LAN endpoint.
  Future<({RuntimeConnectionConfig config, bool baseUrlChanged})>
  _remoteConfigWithDiscoveredBaseUrl(RuntimeConnectionConfig config) async {
    final session = config.activeRemoteSession;
    if (session == null) {
      throw StateError('remote runtime session is required');
    }
    final discoveredBaseUrl = await _discoveredBaseUrlForCoreDevice(
      session.coreDeviceId,
    );
    if (discoveredBaseUrl == null) {
      return (config: config, baseUrlChanged: false);
    }
    final updatedSession = session.withBaseUrl(discoveredBaseUrl);
    if (updatedSession.baseUrl == session.baseUrl) {
      return (config: config, baseUrlChanged: false);
    }
    final remoteSessions = Map<String, PairedRemoteSessionRecord>.of(
      config.remoteSessions,
    )..[config.activeRemoteName] = updatedSession;
    final resolvedConfig = config.copyWith(
      remoteSessions: Map<String, PairedRemoteSessionRecord>.unmodifiable(
        remoteSessions,
      ),
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    ClientLogger.i(
      'remote baseUrl resolved name=${config.activeRemoteName} baseUrl=${updatedSession.baseUrl}',
      tag: _logTag,
    );
    return (config: resolvedConfig, baseUrlChanged: true);
  }

  /// Finds the LAN endpoint announced by a specific core device identity.
  Future<String?> _discoveredBaseUrlForCoreDevice(String coreDeviceId) async {
    final json = await LinkHostServer.instance.discoverDevices(
      _remoteStartupDiscoveryTimeoutMs,
    );
    final devices = (jsonDecode(json) as List<dynamic>)
        .cast<Map<String, Object?>>();
    for (final device in devices) {
      final deviceId = device['device_id'] as String;
      if (deviceId == coreDeviceId) {
        return device['base_url'] as String;
      }
    }
    return null;
  }
}
