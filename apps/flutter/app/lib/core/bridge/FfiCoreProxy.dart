// ignore_for_file: file_names
import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import '../link/CoreLinkCodec.dart';
import '../link/CoreLinkProtocol.dart';
import 'CoreByteTransport.dart';
import 'CoreProxy.dart';
import 'FfiCoreTransport.dart';
import 'MethodChannelRuntimeHost.dart';

class FfiCoreProxy extends CoreProxy {
  /// Shares the platform-owned runtime through the direct byte transport.
  const FfiCoreProxy({
    CoreByteTransport? transport,
    PlatformRuntimeHost host = const MethodChannelRuntimeHost(),
  }) : _providedTransport = transport,
       _host = host;

  final CoreByteTransport? _providedTransport;
  final PlatformRuntimeHost _host;
  static int _nextWatch = 0;

  /// Resolves the injected transport or the isolate's registered FFI connection.
  CoreByteTransport get _transport =>
      _providedTransport ?? FfiCoreTransport.instance;

  /// Reads default roots through the platform host.
  @override
  Future<Map<Object?, Object?>> runtimeStorageDefaults() =>
      _host.runtimeStorageDefaults();

  /// Normalizes roots through the platform host.
  @override
  Future<Map<Object?, Object?>> runtimeStoragePaths(
    String runtimeRoot,
    String workspaceRoot,
  ) => _host.runtimeStoragePaths(runtimeRoot, workspaceRoot);

  /// Configures the existing host-owned runtime.
  @override
  Future<void> setRuntimeStorageRoots(
    String runtimeRoot,
    String workspaceRoot,
  ) => _host.setRuntimeStorageRoots(runtimeRoot, workspaceRoot);

  /// Reads the host bootstrap record before connecting to FFI.
  @override
  Future<String?> runtimeBootstrapRead() => _host.runtimeBootstrapRead();

  /// Writes the host bootstrap record.
  @override
  Future<void> runtimeBootstrapWrite(String content) =>
      _host.runtimeBootstrapWrite(content);

  /// Requests application restart through the lifecycle owner.
  @override
  Future<void> restartApplication() => _host.restartApplication();

  /// Sends one call directly to Rust without platform-channel serialization.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) => _transport.request(
    CoreTransportOperation.call,
    encodeNativeCoreCallRequest(request),
  );

  /// Opens an ordered input stream on the shared runtime.
  @override
  Future<CorePushSink> push(CorePushRequest request) async {
    final response = await _transport.request(
      CoreTransportOperation.pushOpen,
      encodeNativeCorePushOpenRequest(request),
    );
    return _FfiPushSink(_transport, decodeNativeCorePushOpenResult(response));
  }

  /// Reads one snapshot without blocking the UI isolate.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) async =>
      decodeNativeCoreWatchSnapshotResult(
        await _transport.request(
          CoreTransportOperation.watchSnapshot,
          encodeNativeCoreWatchSnapshotRequest(request),
        ),
      );

  /// Decodes frames from this connection's independent watch subscription.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    final id =
        'ffi-watch-${DateTime.now().microsecondsSinceEpoch}-${_nextWatch++}';
    return _transport
        .watch(id, encodeNativeCoreWatchStreamRequest(id, request))
        .map((bytes) {
          final frame = decodeNativeCoreWatchFrame(bytes);
          if (frame.subscriptionId != id) {
            throw StateError(
              'FFI watch subscription mismatch: ${frame.subscriptionId}',
            );
          }
          return frame.event;
        });
  }
}

class _FfiPushSink implements CorePushSink {
  /// Creates a sequenced stream owned by one FFI connection.
  _FfiPushSink(this._transport, this._id);

  final CoreByteTransport _transport;
  final String _id;
  Future<void> _tail = Future<void>.value();
  Future<void>? _closing;
  int _sequence = 0;

  /// Serializes sends while preserving the original failure for subsequent callers.
  @override
  Future<void> add(Object? args) {
    if (_closing != null) throw StateError('Link push stream is closed');
    final sequence = _sequence++;
    _tail = _tail.then((_) async {
      decodeNativeCoreVoidResult(
        await _transport.request(
          CoreTransportOperation.pushItem,
          encodeNativeCorePushItem(_id, sequence, args),
        ),
      );
    });
    return _tail;
  }

  /// Releases the native stream even when a previous send failed.
  @override
  Future<void> close() => _closing ??= _close();

  /// Drains ordered sends and closes their native resource exactly once.
  Future<void> _close() async {
    try {
      await _tail;
    } finally {
      decodeNativeCoreVoidResult(
        await _transport.request(
          CoreTransportOperation.pushClose,
          CoreLinkMessage.bytes(utf8.encode(_id)),
        ),
      );
    }
  }
}
