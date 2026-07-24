// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import '../logging/ClientLogger.dart';
import 'RuntimeConnectionConfigStore.dart';

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

  /// Creates a storage configuration from its persisted representation.
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

  /// Converts this configuration into its persisted representation.
  Map<String, Object?> toJson() {
    return <String, Object?>{
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
    required this.localStorage,
    required this.updatedAt,
  });

  /// Creates the default local runtime configuration state.
  factory RuntimeConnectionConfig.local() {
    return RuntimeConnectionConfig(
      localStorage: LocalRuntimeStorageConfig.platformDefault(),
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
  }

  final LocalRuntimeStorageConfig localStorage;
  final int updatedAt;

  /// Creates a copy with updated fields.
  RuntimeConnectionConfig copyWith({
    LocalRuntimeStorageConfig? localStorage,
    int? updatedAt,
  }) {
    return RuntimeConnectionConfig(
      localStorage: localStorage ?? this.localStorage,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }
}

class RuntimeConnectionManager extends ChangeNotifier {
  RuntimeConnectionManager._();

  static final RuntimeConnectionManager instance = RuntimeConnectionManager._();
  static const String _logTag = 'RuntimeConnection';

  RuntimeConnectionConfig _config = RuntimeConnectionConfig.local();

  /// Returns the current local runtime storage configuration.
  RuntimeConnectionConfig get config => _config;

  /// Returns whether local runtime roots have been confirmed.
  bool get runtimeConfigured => _config.localStorage.confirmed;

  /// Loads persisted runtime storage configuration and applies local roots.
  Future<void> initialize() async {
    final stopwatch = Stopwatch()..start();
    ClientLogger.i('initialize start', tag: _logTag);
    try {
      final readStopwatch = Stopwatch()..start();
      final storedConfig = await RuntimeConnectionConfigStore.read();
      ClientLogger.i(
        'config read done localConfirmed=${storedConfig.localStorage.confirmed} elapsedMs=${readStopwatch.elapsedMilliseconds}',
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
    await _apply(
      _config.copyWith(
        localStorage: localStorage,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      ),
      persist: true,
    );
    ClientLogger.i(
      'persist migrated local runtime storage done elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
    );
  }

  /// Applies one local runtime storage configuration.
  Future<void> _apply(
    RuntimeConnectionConfig config, {
    required bool persist,
  }) async {
    final stopwatch = Stopwatch()..start();
    ClientLogger.d('apply start persist=$persist', tag: _logTag);
    _config = config;
    if (persist) {
      await RuntimeConnectionConfigStore.write(config);
    }
    notifyListeners();
    ClientLogger.i(
      'apply done persist=$persist elapsedMs=${stopwatch.elapsedMilliseconds}',
      tag: _logTag,
    );
  }
}
