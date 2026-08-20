// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
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

  LocalRuntimeStorageConfig _config =
      LocalRuntimeStorageConfig.platformDefault();

  /// Returns the current local bootstrap configuration.
  LocalRuntimeStorageConfig get config => _config;

  /// Returns whether local runtime roots have been confirmed.
  bool get runtimeConfigured => _config.confirmed;

  /// Returns every configured isolated user identity.
  List<RuntimeIdentity> get identities => _config.identities;

  /// Returns the identity selected for this application process.
  RuntimeIdentity get activeIdentity => _config.activeIdentity;

  /// Loads persisted runtime storage configuration and applies local roots.
  Future<void> initialize() async {
    final stopwatch = Stopwatch()..start();
    ClientLogger.i('initialize start', tag: _logTag);
    try {
      final readStopwatch = Stopwatch()..start();
      final encoded = await platformCoreProxy.runtimeBootstrapRead();
      final LocalRuntimeStorageConfig storedConfig;
      if (encoded == null) {
        storedConfig = LocalRuntimeStorageConfig.platformDefault();
        await _writeBootstrapConfig(storedConfig);
      } else {
        storedConfig = LocalRuntimeStorageConfig.fromJson(
          jsonDecode(encoded) as Map<String, Object?>,
        );
      }
      ClientLogger.i(
        'config read done localConfirmed=${storedConfig.confirmed} elapsedMs=${readStopwatch.elapsedMilliseconds}',
        tag: _logTag,
      );
      if (storedConfig.confirmed) {
        final storageStopwatch = Stopwatch()..start();
        ClientLogger.i(
          'local storage apply start runtimeRoot=${storedConfig.runtimeRoot} workspaceRoot=${storedConfig.workspaceRoot}',
          tag: _logTag,
        );
        await LocalRuntimeStorageBridge.apply(storedConfig);
        ClientLogger.i(
          'local storage apply done elapsedMs=${storageStopwatch.elapsedMilliseconds}',
          tag: _logTag,
        );
      }
      await _apply(storedConfig, persist: false);
      ClientLogger.i(
        'initialize done elapsedMs=${stopwatch.elapsedMilliseconds}',
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
    await _apply(
      _config.copyWith(
        identities: <RuntimeIdentity>[...identities, identity],
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      ),
      persist: true,
    );
    return identity;
  }

  /// Renames one isolated identity without changing its storage roots.
  Future<void> renameIdentity(String identityId, String name) async {
    final renamed = identities
        .map(
          (identity) =>
              identity.id == identityId ? identity.rename(name) : identity,
        )
        .toList(growable: false);
    if (!renamed.any((identity) => identity.id == identityId)) {
      throw StateError('runtime identity does not exist: $identityId');
    }
    await _apply(
      _config.copyWith(
        identities: renamed,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      ),
      persist: true,
    );
  }

  /// Persists one identity selection and restarts before runtime state can diverge.
  Future<void> switchIdentity(String identityId) async {
    if (!identities.any((identity) => identity.id == identityId)) {
      throw StateError('runtime identity does not exist: $identityId');
    }
    final sourceIdentityId = activeIdentity.id;
    ClientLogger.i(
      'switch identity start sourceIdentityId=$sourceIdentityId targetIdentityId=$identityId',
      tag: _logTag,
    );
    final now = DateTime.now().millisecondsSinceEpoch;
    final selectedConfig = _config.copyWith(
      activeIdentityId: identityId,
      updatedAt: now,
    );
    selectedConfig.validate();
    await _writeBootstrapConfig(selectedConfig);
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
    final stopwatch = Stopwatch()..start();
    ClientLogger.i(
      'confirm local runtime storage start runtimeRoot=$runtimeRoot workspaceRoot=$workspaceRoot',
      tag: _logTag,
    );
    final localStorage = LocalRuntimeStorageConfig(
      confirmed: true,
      runtimeRoot: runtimeRoot.trim(),
      workspaceRoot: workspaceRoot.trim(),
      identities: identities,
      activeIdentityId: activeIdentity.id,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    await LocalRuntimeStorageBridge.apply(localStorage);
    await _apply(localStorage, persist: true);
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
      identities: identities,
      activeIdentityId: activeIdentity.id,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    await _apply(localStorage, persist: true);
    ClientLogger.i(
      'persist migrated local runtime storage done elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
    );
  }

  /// Applies one local runtime storage configuration.
  Future<void> _apply(
    LocalRuntimeStorageConfig config, {
    required bool persist,
  }) async {
    final stopwatch = Stopwatch()..start();
    ClientLogger.d('apply start persist=$persist', tag: _logTag);
    _config = config;
    if (persist) {
      await _writeBootstrapConfig(config);
    }
    notifyListeners();
    ClientLogger.i(
      'apply done persist=$persist elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
    );
  }

  /// Serializes one validated bootstrap configuration through the platform Host.
  Future<void> _writeBootstrapConfig(LocalRuntimeStorageConfig config) async {
    config.validate();
    await platformCoreProxy.runtimeBootstrapWrite(jsonEncode(config.toJson()));
  }
}
