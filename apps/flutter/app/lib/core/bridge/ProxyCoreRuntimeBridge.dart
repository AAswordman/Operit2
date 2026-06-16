// ignore_for_file: file_names

import '../host/HostEnvironmentDescriptor.dart';
import '../link/CoreLinkProtocol.dart';
import '../runtime/RuntimeConnectionManager.dart';
import 'CoreProxy.dart';
import 'OperitRuntimeBridge.dart';
export '../link/RemoteRuntimeLinkClient.dart';

class ProxyCoreRuntimeBridge extends OperitRuntimeBridge {
  const ProxyCoreRuntimeBridge({CoreProxy? coreProxy})
    : _coreProxyOverride = coreProxy;

  final CoreProxy? _coreProxyOverride;

  CoreProxy get _coreProxy =>
      _coreProxyOverride ?? RuntimeConnectionManager.instance.coreProxy;

  @override
  Future<Object?> call(CoreCallRequest request) {
    return _coreProxy.call(request);
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) {
    return _coreProxy.watchSnapshot(request);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) async* {
    await for (final event in _coreProxy.watchStream(request)) {
      yield event;
    }
  }

  @override
  Future<HostEnvironmentDescriptor> hostDescriptor() {
    return _coreProxy.hostDescriptor();
  }
}
