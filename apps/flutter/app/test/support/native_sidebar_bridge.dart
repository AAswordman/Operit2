import 'dart:typed_data';

import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';

/// A real protocol fixture with a native workspace and no registered plugins.
class NativeSidebarBridge extends OperitRuntimeBridge {
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    final value = switch (request.methodName) {
      'getPreferences' => <String, String>{},
      'getToolPkgNavigationEntries' || 'getToolPkgUiRoutes' => <Object?>[],
      _ => throw StateError('Unexpected sidebar call: ${request.methodName}'),
    };
    return encodeCoreLink(<Object?>[0, value]);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    if (request.propertyName != 'preferencesFlow') {
      throw StateError('Unexpected sidebar watch: ${request.propertyName}');
    }
    return Stream.value(
      CoreEvent.raw(
        requestId: request.requestId,
        target: request.target,
        propertyName: request.propertyName,
        kind: 'Snapshot',
        valueBytes: encodeCoreLink(<String, String>{}),
        decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
      ),
    );
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
}
