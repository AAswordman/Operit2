// ignore_for_file: file_names

import '../link/CoreLinkProtocol.dart';

abstract class OperitRuntimeBridge {
  const OperitRuntimeBridge();

  Future<Object?> call(CoreCallRequest request);

  Future<CoreEvent> watchSnapshot(CoreWatchRequest request);

  Stream<CoreEvent> watchStream(CoreWatchRequest request);

  Future<Object?> callApplication(
    String methodName, {
    Map<String, Object?> args = const {},
  }) {
    return call(
      CoreCallRequest(
        requestId: 'flutter-${DateTime.now().microsecondsSinceEpoch}',
        targetPath: CoreObjectPath.parse('application'),
        methodName: methodName,
        args: args,
      ),
    );
  }

  Future<CoreEvent> watch(
    String targetPath,
    String propertyName, {
    Map<String, Object?> args = const {},
  }) {
    return watchSnapshot(
      CoreWatchRequest(
        requestId: 'flutter-${DateTime.now().microsecondsSinceEpoch}',
        targetPath: CoreObjectPath.parse(targetPath),
        propertyName: propertyName,
        args: args,
      ),
    );
  }

  Stream<CoreEvent> watchChanges(
    String targetPath,
    String propertyName, {
    Map<String, Object?> args = const {},
  }) {
    return watchStream(
      CoreWatchRequest(
        requestId: 'flutter-${DateTime.now().microsecondsSinceEpoch}',
        targetPath: CoreObjectPath.parse(targetPath),
        propertyName: propertyName,
        args: args,
      ),
    );
  }

  Future<String> dispatchHostEvent(String source, String payloadJson);
}
