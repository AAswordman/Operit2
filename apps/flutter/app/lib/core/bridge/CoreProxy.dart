// ignore_for_file: file_names

import '../link/CoreLinkProtocol.dart';

abstract class CoreProxy {
  const CoreProxy();

  Future<Object?> call(CoreCallRequest request);

  /// Executes a control call concurrently with serialized runtime work.
  Future<Object?> callControl(CoreCallRequest request) => call(request);

  /// Opens a client-owned stream targeting one Core method.
  Future<CorePushSink> push(CorePushRequest request);

  Future<CoreEvent> watchSnapshot(CoreWatchRequest request);

  Stream<CoreEvent> watchStream(CoreWatchRequest request);
}
