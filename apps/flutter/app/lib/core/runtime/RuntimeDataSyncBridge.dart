// ignore_for_file: file_names

import '../bridge/PlatformCoreProxy.dart';
import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../link/CoreLinkProtocol.dart';
import '../proxy/generated/CoreProxyClients.g.dart';

class RuntimeDataSyncResult {
  const RuntimeDataSyncResult({
    required this.rounds,
    required this.localApplied,
    required this.remoteApplied,
  });

  final int rounds;
  final int localApplied;
  final int remoteApplied;
}

class RuntimeDataSyncBridge {
  const RuntimeDataSyncBridge();

  static const int defaultLimit = 512;
  static const List<String> _domains = <String>[
    'preferences',
    'chat',
    'objectbox',
  ];

  Future<RuntimeDataSyncResult> syncPairedRemote({
    required PairedRemoteSessionRecord session,
    int limit = defaultLimit,
  }) async {
    if (limit <= 0) {
      throw ArgumentError.value(
        limit,
        'limit',
        'sync limit must be greater than 0',
      );
    }

    final local = const GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(coreProxy: platformCoreProxy),
    ).application;
    final remoteClient = RemoteRuntimeLinkClient(session: session);
    try {
      await _assertRemoteIdentity(remoteClient, session);
      final remote = GeneratedCoreProxyClients(
        ProxyCoreRuntimeBridge(coreProxy: remoteClient),
      ).application;
      await _assertCoreVersionsMatch(local, remote);

      var rounds = 0;
      var localApplied = 0;
      var remoteApplied = 0;
      while (true) {
        rounds += 1;
        final localClock = await local.syncClock();
        final remoteClock = await remote.syncClock();
        final localOperations = await local.syncOperationsSince(
          clock: remoteClock,
          domains: _domains,
          limit: limit,
        );
        final remoteOperations = await remote.syncOperationsSince(
          clock: localClock,
          domains: _domains,
          limit: limit,
        );
        final mergedOperations = _mergeSyncOperations(
          localOperations,
          remoteOperations,
        );
        final count = _syncOperationCount(mergedOperations);
        if (count == 0) {
          break;
        }
        final remoteResult = await remote.syncApplyOperations(
          operations: mergedOperations,
        );
        final localResult = await local.syncApplyOperations(
          operations: mergedOperations,
        );
        remoteApplied += _appliedCount(remoteResult);
        localApplied += _appliedCount(localResult);
        if (count < limit) {
          break;
        }
      }

      return RuntimeDataSyncResult(
        rounds: rounds,
        localApplied: localApplied,
        remoteApplied: remoteApplied,
      );
    } finally {
      remoteClient.dispose();
    }
  }

  Future<void> _assertCoreVersionsMatch(
    GeneratedApplicationCoreProxy local,
    GeneratedApplicationCoreProxy remote,
  ) async {
    final localVersion = await local.coreVersion();
    final remoteVersion = await remote.coreVersion();
    if (localVersion != remoteVersion) {
      throw StateError(
        'core version mismatch: local=$localVersion, remote=$remoteVersion. sync blocked',
      );
    }
  }

  Future<void> _assertRemoteIdentity(
    RemoteRuntimeLinkClient remoteClient,
    PairedRemoteSessionRecord session,
  ) async {
    final info = await remoteClient.sessionInfo();
    if (info.coreDeviceId != session.coreDeviceId) {
      throw const CoreLinkError(
        code: 'REMOTE_DEVICE_CHANGED',
        message: 'remote runtime identity changed',
      );
    }
  }

  List<Object?> _mergeSyncOperations(Object? left, Object? right) {
    final byId = <String, Object?>{};
    for (final value in <Object?>[
      ..._syncOperationArray(left),
      ..._syncOperationArray(right),
    ]) {
      final operation = _syncOperation(value);
      final opId = operation['opId'] as String;
      byId[opId] = value;
    }
    final operations = byId.values
        .map((value) => MapEntry(_syncSortKey(value), value))
        .toList(growable: false);
    operations.sort((left, right) => left.key.compareTo(right.key));
    return operations.map((entry) => entry.value).toList(growable: false);
  }

  List<Object?> _syncOperationArray(Object? value) {
    return (value as List<Object?>).toList(growable: false);
  }

  Map<String, Object?> _syncOperation(Object? value) {
    return (value as Map<Object?, Object?>).cast<String, Object?>();
  }

  _SyncSortKey _syncSortKey(Object? value) {
    final operation = _syncOperation(value);
    return _SyncSortKey(
      createdAt: operation['createdAt'] as int,
      originDeviceId: operation['originDeviceId'] as String,
      sequence: operation['sequence'] as int,
      opId: operation['opId'] as String,
    );
  }

  int _syncOperationCount(Object? value) {
    return (value as List<Object?>).length;
  }

  int _appliedCount(Object? value) {
    final result = (value as Map<Object?, Object?>).cast<String, Object?>();
    return result['applied'] as int;
  }
}

class _SyncSortKey implements Comparable<_SyncSortKey> {
  const _SyncSortKey({
    required this.createdAt,
    required this.originDeviceId,
    required this.sequence,
    required this.opId,
  });

  final int createdAt;
  final String originDeviceId;
  final int sequence;
  final String opId;

  @override
  int compareTo(_SyncSortKey other) {
    final createdAtResult = createdAt.compareTo(other.createdAt);
    if (createdAtResult != 0) {
      return createdAtResult;
    }
    final originDeviceIdResult = originDeviceId.compareTo(other.originDeviceId);
    if (originDeviceIdResult != 0) {
      return originDeviceIdResult;
    }
    final sequenceResult = sequence.compareTo(other.sequence);
    if (sequenceResult != 0) {
      return sequenceResult;
    }
    return opId.compareTo(other.opId);
  }
}
