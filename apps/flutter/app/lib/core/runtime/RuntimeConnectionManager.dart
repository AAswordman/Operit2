// ignore_for_file: file_names

import 'package:flutter/foundation.dart';

import '../bridge/CoreProxy.dart';
import '../bridge/PlatformCoreProxy.dart';
import '../bridge/RemoteCoreProxy.dart';
import 'RuntimeConnectionConfigStore.dart';

enum RuntimeConnectionMode { local, remote }

class RuntimeConnectionConfig {
  const RuntimeConnectionConfig({
    required this.mode,
    required this.remoteName,
    required this.remoteSession,
    required this.updatedAt,
  });

  factory RuntimeConnectionConfig.local() {
    return RuntimeConnectionConfig(
      mode: RuntimeConnectionMode.local,
      remoteName: '',
      remoteSession: null,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
  }

  factory RuntimeConnectionConfig.fromJson(Map<String, Object?> json) {
    final modeName = json['mode'] as String;
    return RuntimeConnectionConfig(
      mode: RuntimeConnectionMode.values.byName(modeName),
      remoteName: json['remoteName'] as String? ?? '',
      remoteSession: json['remoteSession'] == null
          ? null
          : PairedRemoteSessionRecord.fromJson(
              json['remoteSession'] as Map<String, Object?>,
            ),
      updatedAt: json['updatedAt'] as int,
    );
  }

  final RuntimeConnectionMode mode;
  final String remoteName;
  final PairedRemoteSessionRecord? remoteSession;
  final int updatedAt;

  RuntimeConnectionConfig copyWith({
    RuntimeConnectionMode? mode,
    String? remoteName,
    PairedRemoteSessionRecord? remoteSession,
    bool clearRemoteSession = false,
    int? updatedAt,
  }) {
    return RuntimeConnectionConfig(
      mode: mode ?? this.mode,
      remoteName: remoteName ?? this.remoteName,
      remoteSession: clearRemoteSession
          ? null
          : (remoteSession ?? this.remoteSession),
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }

  Map<String, Object?> toJson() {
    return {
      'mode': mode.name,
      'remoteName': remoteName,
      'remoteSession': remoteSession?.toJson(),
      'updatedAt': updatedAt,
    };
  }
}

class RuntimeConnectionManager extends ChangeNotifier {
  RuntimeConnectionManager._();

  static final RuntimeConnectionManager instance = RuntimeConnectionManager._();

  RuntimeConnectionConfig _config = RuntimeConnectionConfig.local();
  RemoteCoreProxy? _remoteProxy;

  RuntimeConnectionConfig get config => _config;

  CoreProxy get coreProxy {
    return switch (_config.mode) {
      RuntimeConnectionMode.local => platformCoreProxy,
      RuntimeConnectionMode.remote => _remoteProxy!,
    };
  }

  Future<void> initialize() async {
    await _apply(await RuntimeConnectionConfigStore.read(), persist: false);
  }

  Future<void> setLocal() async {
    await _apply(RuntimeConnectionConfig.local(), persist: true);
  }

  Future<void> setRemote({
    required String name,
    required PairedRemoteSessionRecord session,
  }) async {
    await _apply(
      RuntimeConnectionConfig(
        mode: RuntimeConnectionMode.remote,
        remoteName: name,
        remoteSession: session,
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
      final session = config.remoteSession;
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
}
