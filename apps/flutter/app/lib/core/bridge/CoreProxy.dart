// ignore_for_file: file_names

import 'dart:typed_data';

import '../link/CoreLinkCodec.dart';
import '../link/CoreLinkProtocol.dart';

abstract interface class PlatformRuntimeHost {
  /// Reads the platform default base roots before the Runtime is created.
  Future<Map<Object?, Object?>> runtimeStorageDefaults();

  /// Normalizes explicit base roots through the platform storage Host.
  Future<Map<Object?, Object?>> runtimeStoragePaths(
    String runtimeRoot,
    String workspaceRoot,
  );

  /// Installs the identity-isolated roots used by the local Runtime.
  Future<void> setRuntimeStorageRoots(String runtimeRoot, String workspaceRoot);

  /// Reads the bootstrap record outside the active Runtime storage root.
  Future<String?> runtimeBootstrapRead();

  /// Atomically persists the bootstrap record outside the active Runtime.
  Future<void> runtimeBootstrapWrite(String content);

  /// Restarts the application after selecting another bootstrap identity.
  Future<void> restartApplication();
}

abstract class CoreProxy implements PlatformRuntimeHost {
  const CoreProxy();

  /// Sends one encoded Core call and returns its MessagePack response unchanged.
  Future<Uint8List> callBytes(CoreCallRequest request);

  /// Decodes one Core call through the generic Link value representation.
  Future<Object?> call(CoreCallRequest request) async {
    return decodeNativeCoreResult(await callBytes(request));
  }

  /// Sends one control call and returns its MessagePack response unchanged.
  Future<Uint8List> callControlBytes(CoreCallRequest request) => callBytes(request);

  /// Executes a control call concurrently with serialized runtime work.
  Future<Object?> callControl(CoreCallRequest request) async {
    return decodeNativeCoreResult(await callControlBytes(request));
  }

  /// Opens a client-owned stream targeting one Core method.
  Future<CorePushSink> push(CorePushRequest request);

  Future<CoreEvent> watchSnapshot(CoreWatchRequest request);

  Stream<CoreEvent> watchStream(CoreWatchRequest request);
}
