// ignore_for_file: file_names

import 'dart:typed_data';

import '../link/CoreLinkCodec.dart';
import '../link/CoreLinkProtocol.dart';

abstract class OperitRuntimeBridge {
  const OperitRuntimeBridge();

  /// Sends one encoded Core call and returns its MessagePack response unchanged.
  Future<Uint8List> callBytes(CoreCallRequest request);

  /// Decodes one Core call through the generic Link value representation.
  Future<Object?> call(CoreCallRequest request) async {
    return decodeNativeCoreResult(await callBytes(request));
  }

  /// Sends one control call and returns its MessagePack response unchanged.
  Future<Uint8List> callControlBytes(CoreCallRequest request) =>
      callBytes(request);

  /// Executes a control call concurrently with serialized runtime work.
  Future<Object?> callControl(CoreCallRequest request) async {
    return decodeNativeCoreResult(await callControlBytes(request));
  }

  /// Opens a client-owned stream targeting one Core method.
  Future<CorePushSink> push(CorePushRequest request);

  /// Reads one watch snapshot without materializing its payload in the bridge.
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request);

  /// Opens one watch stream and forwards raw Core events unchanged.
  Stream<CoreEvent> watchStream(CoreWatchRequest request);

  /// Opens one embedded stream through the generic Core property route.
  Stream<T> openEmbeddedCoreStream<T>(
    String streamId,
    int targetObjectId,
    String propertyName,
    Object? args,
    T Function(CoreLinkValueReader reader) decode,
  );

  Future<Object?> callApplication(
    String methodName, {
    Map<String, Object?> args = const {},
  }) {
    return call(
      CoreCallRequest(
        requestId: 'flutter-${DateTime.now().microsecondsSinceEpoch}',
        targetObjectId: 0,
        methodName: methodName,
        args: args,
      ),
    );
  }
}
