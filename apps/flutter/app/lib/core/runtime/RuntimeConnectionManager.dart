// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';

import '../bridge/CoreProxy.dart';
import '../bridge/PlatformCoreProxy.dart';
import '../bridge/RemoteCoreProxy.dart';
import '../logging/ClientLogger.dart';
import 'RuntimeConnectionConfigStore.dart';

enum RuntimeConnectionMode { local, remote }

class RuntimeConnectionConfig {
  const RuntimeConnectionConfig({
    required this.mode,
    required this.activeRemoteName,
    required this.remoteSessions,
    required this.updatedAt,
  });

  factory RuntimeConnectionConfig.local() {
    return RuntimeConnectionConfig(
      mode: RuntimeConnectionMode.local,
      activeRemoteName: '',
      remoteSessions: const <String, PairedRemoteSessionRecord>{},
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
  }

  factory RuntimeConnectionConfig.fromJson(Map<String, Object?> json) {
    final modeName = json['mode'] as String;
    final remoteSessionsJson = json['remoteSessions'] as Map<String, Object?>;
    final remoteSessions = remoteSessionsJson.map(
      (key, value) => MapEntry(
        key,
        PairedRemoteSessionRecord.fromJson(value as Map<String, Object?>),
      ),
    );
    return RuntimeConnectionConfig(
      mode: RuntimeConnectionMode.values.byName(modeName),
      activeRemoteName: json['activeRemoteName'] as String,
      remoteSessions: Map<String, PairedRemoteSessionRecord>.unmodifiable(
        remoteSessions,
      ),
      updatedAt: json['updatedAt'] as int,
    );
  }

  final RuntimeConnectionMode mode;
  final String activeRemoteName;
  final Map<String, PairedRemoteSessionRecord> remoteSessions;
  final int updatedAt;

  PairedRemoteSessionRecord? get activeRemoteSession {
    return remoteSessions[activeRemoteName];
  }

  RuntimeConnectionConfig copyWith({
    RuntimeConnectionMode? mode,
    String? activeRemoteName,
    Map<String, PairedRemoteSessionRecord>? remoteSessions,
    int? updatedAt,
  }) {
    return RuntimeConnectionConfig(
      mode: mode ?? this.mode,
      activeRemoteName: activeRemoteName ?? this.activeRemoteName,
      remoteSessions: remoteSessions ?? this.remoteSessions,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }

  Map<String, Object?> toJson() {
    return {
      'mode': mode.name,
      'activeRemoteName': activeRemoteName,
      'remoteSessions': remoteSessions.map(
        (key, value) => MapEntry(key, value.toJson()),
      ),
      'updatedAt': updatedAt,
    };
  }
}

class RuntimeRemoteFailureEvent {
  const RuntimeRemoteFailureEvent({
    required this.id,
    required this.remoteName,
    required this.error,
  });

  final int id;
  final String remoteName;
  final String error;
}

class RuntimeConnectionManager extends ChangeNotifier {
  RuntimeConnectionManager._();

  static final RuntimeConnectionManager instance = RuntimeConnectionManager._();
  static const Duration _remoteStartupProbeTimeout = Duration(seconds: 4);

  RuntimeConnectionConfig _config = RuntimeConnectionConfig.local();
  RemoteCoreProxy? _remoteProxy;
  RuntimeRemoteFailureEvent? _lastRemoteFailure;
  int _remoteFailureId = 0;

  RuntimeConnectionConfig get config => _config;
  RuntimeRemoteFailureEvent? get lastRemoteFailure => _lastRemoteFailure;

  CoreProxy get coreProxy {
    return switch (_config.mode) {
      RuntimeConnectionMode.local => platformCoreProxy,
      RuntimeConnectionMode.remote => _remoteProxy!,
    };
  }

  Future<void> initialize() async {
    final storedConfig = await RuntimeConnectionConfigStore.read();
    if (storedConfig.mode == RuntimeConnectionMode.remote) {
      try {
        await _applyRemote(storedConfig, persist: false, verify: true);
        return;
      } catch (error, stackTrace) {
        ClientLogger.w(
          'Remote runtime unavailable during startup; switching to local runtime',
          error: error,
          stackTrace: stackTrace,
        );
        await _apply(
          storedConfig.copyWith(
            mode: RuntimeConnectionMode.local,
            updatedAt: DateTime.now().millisecondsSinceEpoch,
          ),
          persist: true,
        );
        return;
      }
    }
    await _apply(storedConfig, persist: false);
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

  Future<void> handleRemoteFailure(
    Object error, [
    StackTrace? stackTrace,
  ]) async {
    if (_config.mode != RuntimeConnectionMode.remote) {
      return;
    }
    final remoteName = _config.activeRemoteName;
    ClientLogger.e(
      'Remote runtime failed; switching to local runtime',
      error: error,
      stackTrace: stackTrace,
    );
    _lastRemoteFailure = RuntimeRemoteFailureEvent(
      id: ++_remoteFailureId,
      remoteName: remoteName,
      error: error.toString(),
    );
    await _apply(
      _config.copyWith(
        mode: RuntimeConnectionMode.local,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      ),
      persist: true,
    );
  }

  Future<void> setRemote({
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
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    try {
      await _applyRemote(remoteConfig, persist: true, verify: true);
    } catch (_) {
      await _apply(
        remoteConfig.copyWith(
          mode: RuntimeConnectionMode.local,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        ),
        persist: true,
      );
      rethrow;
    }
  }

  Future<void> usePairedRemote(String name) async {
    if (!_config.remoteSessions.containsKey(name)) {
      throw StateError('paired remote runtime does not exist: $name');
    }
    final remoteConfig = _config.copyWith(
      mode: RuntimeConnectionMode.remote,
      activeRemoteName: name,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    try {
      await _applyRemote(remoteConfig, persist: true, verify: true);
    } catch (_) {
      await _apply(
        remoteConfig.copyWith(
          mode: RuntimeConnectionMode.local,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        ),
        persist: true,
      );
      rethrow;
    }
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
    await _apply(
      RuntimeConnectionConfig(
        mode: activeRemoved ? RuntimeConnectionMode.local : _config.mode,
        activeRemoteName: activeRemoved ? '' : _config.activeRemoteName,
        remoteSessions: Map<String, PairedRemoteSessionRecord>.unmodifiable(
          remoteSessions,
        ),
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      ),
      persist: true,
    );
  }

  Future<void> _apply(
    RuntimeConnectionConfig config, {
    required bool persist,
  }) async {
    _remoteProxy?.dispose();
    _remoteProxy = null;
    if (config.mode == RuntimeConnectionMode.remote) {
      final session = config.activeRemoteSession;
      if (session == null) {
        throw StateError('remote runtime session is required');
      }
      _remoteProxy = RemoteCoreProxy(session: session);
    }
    _config = config;
    if (persist) {
      await RuntimeConnectionConfigStore.write(config);
    }
    notifyListeners();
  }

  Future<void> _applyRemote(
    RuntimeConnectionConfig config, {
    required bool persist,
    required bool verify,
  }) async {
    _remoteProxy?.dispose();
    _remoteProxy = null;
    final session = config.activeRemoteSession;
    if (session == null) {
      throw StateError('remote runtime session is required');
    }
    final proxy = RemoteCoreProxy(session: session);
    try {
      if (verify) {
        await proxy.hostDescriptor().timeout(_remoteStartupProbeTimeout);
      }
      _remoteProxy = proxy;
      _config = config;
      if (persist) {
        await RuntimeConnectionConfigStore.write(config);
      }
      notifyListeners();
    } catch (_) {
      proxy.dispose();
      rethrow;
    }
  }
}
