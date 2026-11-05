// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

/// Members, presence and per-peer sync of the device space, as the extension
/// UI shows them.
///
/// One instance serves each Core connection; pages [attach] while visible.
/// Sync progress is keyed by peer node id, which is the topology device id.
class DeviceSpaceState extends ChangeNotifier {
  DeviceSpaceState._(this.clients);

  static final Expando<DeviceSpaceState> _states = Expando<DeviceSpaceState>();

  static DeviceSpaceState of(GeneratedCoreProxyClients clients) =>
      _states[clients.bridge] ??= DeviceSpaceState._(clients);

  static const Duration _retryDelay = Duration(seconds: 2);

  final GeneratedCoreProxyClients clients;

  core_proxy.RuntimeDeviceSpaceSnapshot? _snapshot;
  Map<String, core_proxy.SpaceSyncProgress> _sync =
      <String, core_proxy.SpaceSyncProgress>{};
  String? _error;
  int _attachments = 0;
  StreamSubscription<core_proxy.RuntimeDeviceSpaceSnapshot>? _snapshotWatch;
  StreamSubscription<List<core_proxy.SpaceSyncProgress>>? _syncWatch;
  Timer? _snapshotRetry;
  Timer? _syncRetry;
  final StreamController<void> _syncCompletions =
      StreamController<void>.broadcast();

  core_proxy.RuntimeDeviceSpaceSnapshot? get snapshot => _snapshot;
  String? get error => _error;

  List<core_proxy.RuntimeDeviceSpaceDevice> get devices =>
      _snapshot?.topology.devices ??
      const <core_proxy.RuntimeDeviceSpaceDevice>[];

  String? get currentDeviceId => _snapshot?.topology.currentDeviceId;

  core_proxy.RuntimeDeviceSpaceDevice? get currentDevice {
    final id = currentDeviceId;
    for (final device in devices) {
      if (device.deviceId == id) {
        return device;
      }
    }
    return null;
  }

  /// Members other than this device, online ones first.
  List<core_proxy.RuntimeDeviceSpaceDevice> get otherDevices {
    final id = currentDeviceId;
    final others = devices.where((device) => device.deviceId != id).toList();
    others.sort((a, b) {
      if (a.online != b.online) {
        return a.online ? -1 : 1;
      }
      return a.deviceName.compareTo(b.deviceName);
    });
    return others;
  }

  int get onlineCount => devices.where((device) => device.online).length;

  String get spaceName => _snapshot?.space.spaceName ?? '';

  /// Sync with [deviceId], if a sync with that peer has been reported.
  core_proxy.SpaceSyncProgress? syncWith(String deviceId) => _sync[deviceId];

  bool get syncing => _sync.values.any(
    (progress) => progress.status == core_proxy.SpaceSyncStatus.syncing,
  );

  /// Emits when a peer sync finishes, the moment synced extensions land.
  Stream<void> get syncCompletions => _syncCompletions.stream;

  void attach() {
    _attachments += 1;
    if (_attachments == 1) {
      _watchSnapshot();
      _watchSync();
    }
  }

  void detach() {
    if (_attachments == 0) {
      throw StateError('DeviceSpaceState.detach without attach');
    }
    _attachments -= 1;
    if (_attachments == 0) {
      _snapshotRetry?.cancel();
      _snapshotRetry = null;
      _syncRetry?.cancel();
      _syncRetry = null;
      unawaited(_snapshotWatch?.cancel());
      _snapshotWatch = null;
      unawaited(_syncWatch?.cancel());
      _syncWatch = null;
    }
  }

  void _watchSnapshot() {
    _snapshotWatch = clients.server.runtimeRemoteLinkService
        .deviceSpaceSnapshotFlow()
        .listen(
          (snapshot) {
            _snapshot = snapshot;
            _error = null;
            notifyListeners();
          },
          onError: (Object error, StackTrace stackTrace) {
            debugPrint('Device space watch failed: $error\n$stackTrace');
            _error = error.toString();
            notifyListeners();
            _snapshotRetry ??= _retry(() {
              _snapshotRetry = null;
              unawaited(_snapshotWatch?.cancel());
              _watchSnapshot();
            });
          },
          onDone: () {
            _snapshotRetry ??= _retry(() {
              _snapshotRetry = null;
              _watchSnapshot();
            });
          },
        );
  }

  void _watchSync() {
    _syncWatch = clients.server.runtimeRemoteLinkService
        .spaceSyncProgressFlow()
        .listen(
          _applySync,
          onError: (Object error, StackTrace stackTrace) {
            debugPrint('Space sync watch failed: $error\n$stackTrace');
            _syncRetry ??= _retry(() {
              _syncRetry = null;
              unawaited(_syncWatch?.cancel());
              _watchSync();
            });
          },
          onDone: () {
            _syncRetry ??= _retry(() {
              _syncRetry = null;
              _watchSync();
            });
          },
        );
  }

  /// Resubscribes a closed watch while some page still shows the space.
  Timer _retry(void Function() resubscribe) {
    return Timer(_retryDelay, () {
      if (_attachments > 0) {
        resubscribe();
      }
    });
  }

  void _applySync(List<core_proxy.SpaceSyncProgress> progress) {
    var finished = false;
    for (final item in progress) {
      final before = _sync[item.peerNodeId];
      if (before != null &&
          before.status != core_proxy.SpaceSyncStatus.completed &&
          item.status == core_proxy.SpaceSyncStatus.completed) {
        finished = true;
      }
    }
    _sync = <String, core_proxy.SpaceSyncProgress>{
      for (final item in progress) item.peerNodeId: item,
    };
    notifyListeners();
    if (finished) {
      _syncCompletions.add(null);
    }
  }
}
