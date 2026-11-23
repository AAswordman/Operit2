// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:path/path.dart' as path;

import '../bridge/PlatformCoreProxy.dart';
import '../logging/ClientLogger.dart';
import 'RuntimeBootstrapModels.dart';

export 'RuntimeBootstrapModels.dart';

class LocalRuntimeStorageBridge {
  const LocalRuntimeStorageBridge._();

  /// Resolves the isolated runtime and workspace roots for one identity.
  static Future<RuntimeStoragePaths> pathsForConfig(
    LocalRuntimeStorageConfig config,
  ) {
    config.validate();
    return pathsForRoots(
      path.join(config.runtimeRoot, 'identities', config.activeIdentityId),
      path.join(config.workspaceRoot, 'identities', config.activeIdentityId),
    );
  }

  /// Reads the platform default runtime and workspace roots.
  static Future<RuntimeStoragePaths> defaultPaths() async {
    return RuntimeStoragePaths.fromMap(
      await platformCoreProxy.runtimeStorageDefaults(),
    );
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
    return RuntimeStoragePaths.fromMap(
      await platformCoreProxy.runtimeStoragePaths(
        normalizedRuntimeRoot,
        normalizedWorkspaceRoot,
      ),
    );
  }

  /// Installs local storage roots and permits repeated identical configuration.
  static Future<void> apply(LocalRuntimeStorageConfig config) async {
    if (!config.confirmed) {
      throw StateError('local runtime storage config is not confirmed');
    }
    final resolved = await pathsForConfig(config);
    await platformCoreProxy.setRuntimeStorageRoots(
      resolved.runtimeRoot,
      resolved.workspaceRoot,
    );
  }
}

class RuntimeBootstrapManager extends ChangeNotifier {
  RuntimeBootstrapManager._();

  static final RuntimeBootstrapManager instance = RuntimeBootstrapManager._();
  static const String _logTag = 'RuntimeBootstrap';
  static const String _startupThemeModeKey = 'startupThemeMode';

  LocalRuntimeStorageConfig _config =
      LocalRuntimeStorageConfig.platformDefault();
  ThemeMode _startupThemeMode = ThemeMode.system;
  Future<void>? _bootstrapWriteTail;
  String? _nextIdentityId;

  /// Returns the current local bootstrap configuration.
  LocalRuntimeStorageConfig get config => _config;

  /// Returns whether local runtime roots have been confirmed.
  bool get runtimeConfigured => _config.confirmed;

  /// Returns every configured isolated user identity.
  List<RuntimeIdentity> get identities => _config.identities;

  /// Returns the identity selected for this application process.
  RuntimeIdentity get activeIdentity => _config.activeIdentity;

  /// Returns the client-cached theme mode used during startup rendering.
  ThemeMode get startupThemeMode => _startupThemeMode;

  /// Loads persisted runtime storage configuration and applies local roots.
  Future<void> initialize() async {
    final stopwatch = Stopwatch()..start();
    ClientLogger.v(
      'initialize start',
      tag: _logTag,
      verboseLevel: VerboseLevel.level1,
    );
    try {
      final readStopwatch = Stopwatch()..start();
      final encoded = await platformCoreProxy.runtimeBootstrapRead();
      final LocalRuntimeStorageConfig storedConfig;
      if (encoded == null) {
        _startupThemeMode = ThemeMode.system;
        storedConfig = LocalRuntimeStorageConfig.platformDefault();
        await _serializeBootstrapWrite(
          () => _writeBootstrapConfig(storedConfig),
        );
      } else {
        final decoded = jsonDecode(encoded) as Map<String, Object?>;
        _startupThemeMode = _decodeStartupThemeMode(
          decoded[_startupThemeModeKey],
        );
        storedConfig = LocalRuntimeStorageConfig.fromJson(decoded);
      }
      ClientLogger.v(
        'config read done localConfirmed=${storedConfig.confirmed} elapsedMs=${readStopwatch.elapsedMilliseconds}',
        tag: _logTag,
        verboseLevel: VerboseLevel.level1,
      );
      _config = storedConfig;
      if (storedConfig.confirmed) {
        final storageStopwatch = Stopwatch()..start();
        ClientLogger.v(
          'local storage apply start runtimeRoot=${storedConfig.runtimeRoot} workspaceRoot=${storedConfig.workspaceRoot}',
          tag: _logTag,
          verboseLevel: VerboseLevel.level1,
        );
        await LocalRuntimeStorageBridge.apply(storedConfig);
        ClientLogger.v(
          'local storage apply done elapsedMs=${storageStopwatch.elapsedMilliseconds}',
          tag: _logTag,
          verboseLevel: VerboseLevel.level1,
        );
      }
      await _apply(storedConfig, persist: false);
      ClientLogger.v(
        'initialize done elapsedMs=${stopwatch.elapsedMilliseconds}',
        tag: _logTag,
        verboseLevel: VerboseLevel.level1,
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

  /// Loads persisted bootstrap state for a secondary Flutter engine.
  ///
  /// Detached windows share the native Rust runtime with the main engine, so
  /// they must not apply storage roots or start the process-owned runtime a
  /// second time. They still need the persisted configuration in order to
  /// render the normal themed UI instead of the unconfigured bootstrap view.
  Future<void> initializeForDetachedWindow() async {
    final encoded = await platformCoreProxy.runtimeBootstrapRead();
    if (encoded == null) {
      await _apply(LocalRuntimeStorageConfig.platformDefault(), persist: false);
      return;
    }
    final decoded = jsonDecode(encoded) as Map<String, Object?>;
    _startupThemeMode = _decodeStartupThemeMode(decoded[_startupThemeModeKey]);
    await _apply(LocalRuntimeStorageConfig.fromJson(decoded), persist: false);
  }

  /// Returns native storage paths for the stored local runtime config.
  Future<RuntimeStoragePaths> localRuntimeStoragePaths() {
    if (!_config.confirmed) {
      throw StateError('local runtime storage config is not confirmed');
    }
    return LocalRuntimeStorageBridge.pathsForConfig(_config);
  }

  /// Returns normalized base roots shared by every configured identity.
  Future<RuntimeStoragePaths> localRuntimeStorageBasePaths() {
    if (!_config.confirmed) {
      throw StateError('local runtime storage config is not confirmed');
    }
    return LocalRuntimeStorageBridge.pathsForRoots(
      _config.runtimeRoot,
      _config.workspaceRoot,
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

  /// Resolves candidate base roots into the active identity's isolated roots.
  Future<RuntimeStoragePaths> localRuntimeStoragePathsForIdentityRoots(
    String runtimeRoot,
    String workspaceRoot,
  ) {
    final candidate = _config.copyWith(
      runtimeRoot: runtimeRoot.trim(),
      workspaceRoot: workspaceRoot.trim(),
    );
    return LocalRuntimeStorageBridge.pathsForConfig(candidate);
  }

  /// Creates a new isolated identity without changing the running runtime.
  Future<RuntimeIdentity> createIdentity([String name = '']) async {
    final identity = RuntimeIdentity.create(name);
    await _updateBootstrapConfig(
      () => _config.copyWith(
        identities: <RuntimeIdentity>[...identities, identity],
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      ),
    );
    return identity;
  }

  /// Renames one isolated identity without changing its storage roots.
  Future<void> renameIdentity(String identityId, String name) {
    return _updateBootstrapConfig(() {
      _requireIdentity(identityId);
      return _config.copyWith(
        identities: <RuntimeIdentity>[
          for (final identity in identities)
            identity.id == identityId ? identity.rename(name) : identity,
        ],
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      );
    });
  }

  /// Removes an inactive identity from the local registry while retaining its files.
  Future<void> deleteIdentity(String identityId) {
    return _updateBootstrapConfig(() {
      _requireIdentity(identityId);
      if (identityId == activeIdentity.id || identityId == _nextIdentityId) {
        throw StateError(
          'cannot delete a running or selected runtime identity',
        );
      }
      return _config.copyWith(
        identities: identities
            .where((identity) => identity.id != identityId)
            .toList(growable: false),
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      );
    });
  }

  /// Rejects operations targeting an identity outside the local registry.
  void _requireIdentity(String identityId) {
    if (!identities.any((identity) => identity.id == identityId)) {
      throw StateError('runtime identity does not exist: $identityId');
    }
  }

  /// Persists one identity selection and restarts before runtime state can diverge.
  Future<void> switchIdentity(String identityId) async {
    await _serializeBootstrapWrite(() async {
      _requireIdentity(identityId);
      ClientLogger.v(
        'switch identity start sourceIdentityId=${activeIdentity.id} targetIdentityId=$identityId',
        tag: _logTag,
        verboseLevel: VerboseLevel.level1,
      );
      await _writeBootstrapConfig(
        _config.copyWith(
          activeIdentityId: identityId,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        ),
      );
      _nextIdentityId = identityId;
    });
    ClientLogger.i(
      'switch identity persisted targetIdentityId=$identityId; requesting required application exit',
      tag: _logTag,
    );
    await platformCoreProxy.restartApplication();
  }

  /// Confirms and persists the local runtime and workspace roots.
  Future<void> confirmLocalRuntimeStorage(
    String runtimeRoot,
    String workspaceRoot,
  ) async {
    final normalizedRuntimeRoot = runtimeRoot.trim();
    final normalizedWorkspaceRoot = workspaceRoot.trim();
    // Identity switches keep these base roots and only remount
    // identities/<id> during startup. Confirming them again is not a change.
    if (_config.confirmed &&
        _config.runtimeRoot == normalizedRuntimeRoot &&
        _config.workspaceRoot == normalizedWorkspaceRoot) {
      ClientLogger.i(
        'confirm local runtime storage skipped unchanged roots',
        tag: _logTag,
      );
      return;
    }
    final stopwatch = Stopwatch()..start();
    ClientLogger.v(
      'confirm local runtime storage start runtimeRoot=$normalizedRuntimeRoot workspaceRoot=$normalizedWorkspaceRoot',
      tag: _logTag,
      verboseLevel: VerboseLevel.level1,
    );
    final localStorage = LocalRuntimeStorageConfig(
      confirmed: true,
      runtimeRoot: normalizedRuntimeRoot,
      workspaceRoot: normalizedWorkspaceRoot,
      identities: identities,
      activeIdentityId: activeIdentity.id,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
      chatBackupDirectory: _config.chatBackupDirectory,
    );
    await LocalRuntimeStorageBridge.apply(localStorage);
    await _apply(localStorage, persist: true);
    // Startup logs were kept in memory while onboarding was unconfigured.
    // Attach only after the native bridge has accepted the confirmed roots so
    // the queued entries can be flushed safely.
    ClientLogger.attachPersistentStorage();
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
    ClientLogger.v(
      'persist migrated local runtime storage start runtimeRoot=$runtimeRoot workspaceRoot=$workspaceRoot',
      tag: _logTag,
      verboseLevel: VerboseLevel.level1,
    );
    final localStorage = LocalRuntimeStorageConfig(
      confirmed: true,
      runtimeRoot: runtimeRoot.trim(),
      workspaceRoot: workspaceRoot.trim(),
      identities: identities,
      activeIdentityId: activeIdentity.id,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
      chatBackupDirectory: _config.chatBackupDirectory,
    );
    await _apply(localStorage, persist: true);
    ClientLogger.attachPersistentStorage();
    ClientLogger.i(
      'persist migrated local runtime storage done elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
    );
  }

  /// Persists a machine-local backup directory without changing runtime or workspace roots.
  Future<void> saveChatBackupDirectory(String directory) async {
    await _updateBootstrapConfig(
      () => _config.copyWith(chatBackupDirectory: directory),
    );
  }

  /// Persists the theme mode needed before Core preferences become available.
  Future<void> saveStartupThemeMode(ThemeMode themeMode) {
    return _serializeBootstrapWrite(() async {
      if (_startupThemeMode == themeMode) {
        return;
      }
      await _writeBootstrapConfig(
        _config.copyWith(activeIdentityId: _nextIdentityId),
        startupThemeMode: themeMode,
      );
      _startupThemeMode = themeMode;
    });
  }

  /// Applies one local runtime storage configuration.
  Future<void> _apply(
    LocalRuntimeStorageConfig config, {
    required bool persist,
  }) async {
    final stopwatch = Stopwatch()..start();
    ClientLogger.v(
      'apply start persist=$persist',
      tag: _logTag,
      verboseLevel: VerboseLevel.level1,
    );
    if (persist) {
      await _updateBootstrapConfig(
        () => _config.copyWith(
          confirmed: config.confirmed,
          runtimeRoot: config.runtimeRoot,
          workspaceRoot: config.workspaceRoot,
          updatedAt: config.updatedAt,
        ),
      );
    } else {
      _config = config;
      _nextIdentityId = null;
      notifyListeners();
    }
    if (config.confirmed) {
      ClientLogger.attachPersistentStorage();
    }
    ClientLogger.v(
      'apply done persist=$persist elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
      verboseLevel: VerboseLevel.level1,
    );
  }

  /// Publishes a configuration only after the Host has persisted it.
  Future<void> _updateBootstrapConfig(
    LocalRuntimeStorageConfig Function() update,
  ) {
    return _serializeBootstrapWrite(() async {
      final next = update();
      await _writeBootstrapConfig(
        next.copyWith(activeIdentityId: _nextIdentityId),
      );
      _config = next;
      notifyListeners();
    });
  }

  /// Queues each bootstrap mutation against the latest persisted state.
  Future<void> _serializeBootstrapWrite(Future<void> Function() operation) {
    final pending = _bootstrapWriteTail;
    final write = pending == null
        ? operation()
        : pending.then((_) => operation());
    final tail = write.then<void>((_) {}, onError: (Object _, StackTrace _) {});
    _bootstrapWriteTail = tail;
    return write.whenComplete(() {
      if (identical(_bootstrapWriteTail, tail)) {
        _bootstrapWriteTail = null;
      }
    });
  }

  /// Writes one validated bootstrap configuration through the platform Host.
  Future<void> _writeBootstrapConfig(
    LocalRuntimeStorageConfig config, {
    ThemeMode? startupThemeMode,
  }) async {
    config.validate();
    final encodedConfig = <String, Object?>{
      ...config.toJson(),
      _startupThemeModeKey: (startupThemeMode ?? _startupThemeMode).name,
    };
    await platformCoreProxy.runtimeBootstrapWrite(jsonEncode(encodedConfig));
  }
}

/// Decodes the persisted client startup theme mode.
ThemeMode _decodeStartupThemeMode(Object? value) {
  if (value == null) {
    return ThemeMode.system;
  }
  if (value is! String) {
    throw const FormatException('startup theme mode must be a string');
  }
  return switch (value) {
    'system' => ThemeMode.system,
    'light' => ThemeMode.light,
    'dark' => ThemeMode.dark,
    _ => throw FormatException('invalid startup theme mode: $value'),
  };
}
