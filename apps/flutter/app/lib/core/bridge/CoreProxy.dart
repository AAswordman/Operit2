// ignore_for_file: file_names

import '../link/CoreLinkProtocol.dart';

abstract class CoreProxy {
  const CoreProxy();

  Future<Object?> call(CoreCallRequest request);

  /// Opens a client-owned stream targeting one Core method.
  Future<CorePushSink> push(CorePushRequest request);

  Future<CoreEvent> watchSnapshot(CoreWatchRequest request);

  Stream<CoreEvent> watchStream(CoreWatchRequest request);
}
