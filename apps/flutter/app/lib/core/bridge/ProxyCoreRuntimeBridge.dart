// ignore_for_file: file_names

import 'dart:typed_data';

import '../link/CoreLinkProtocol.dart';
import 'CoreProxy.dart';
import 'OperitRuntimeBridge.dart';
import 'PlatformCoreProxy.dart';

class ProxyCoreRuntimeBridge extends OperitRuntimeBridge {
  const ProxyCoreRuntimeBridge({CoreProxy? coreProxy})
    : _coreProxyOverride = coreProxy;

  final CoreProxy? _coreProxyOverride;

  /// Returns the local platform proxy unless a caller explicitly supplies one.
  CoreProxy get _coreProxy => _coreProxyOverride ?? platformCoreProxy;

  /// Sends one Core call through the selected platform proxy without decoding its payload.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) {
    return _coreProxy.callBytes(request);
  }

  /// Sends one control call through the selected platform proxy without decoding its payload.
  @override
  Future<Uint8List> callControlBytes(CoreCallRequest request) {
    return _coreProxy.callControlBytes(request);
  }

  /// Opens a client-owned Link input stream.
  @override
  Future<CorePushSink> push(CorePushRequest request) {
    return _coreProxy.push(request);
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) {
    return _coreProxy.watchSnapshot(request);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    return _coreProxy.watchStream(request);
  }
}
