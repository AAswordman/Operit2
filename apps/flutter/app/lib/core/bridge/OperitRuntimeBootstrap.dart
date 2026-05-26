// ignore_for_file: file_names

import '../host/HostEnvironmentDescriptor.dart';
import 'OperitRuntimeBridge.dart';
import 'ProxyCoreRuntimeBridge.dart';

class OperitRuntimeBootstrap {
  const OperitRuntimeBootstrap({this.bridge = const ProxyCoreRuntimeBridge()});

  final OperitRuntimeBridge bridge;

  Future<OperitRuntimeSnapshot> load() async {
    final host = await bridge.hostDescriptor();
    final coreVersion = await bridge.callApplication('coreVersion');
    return OperitRuntimeSnapshot(
      host: host,
      coreVersion: coreVersion as String,
    );
  }
}

class OperitRuntimeSnapshot {
  const OperitRuntimeSnapshot({required this.host, required this.coreVersion});

  final HostEnvironmentDescriptor host;
  final String coreVersion;
}
