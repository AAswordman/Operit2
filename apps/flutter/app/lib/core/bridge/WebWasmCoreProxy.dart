// ignore_for_file: file_names

import 'dart:async';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';
import 'dart:typed_data';

import '../link/CoreLinkCodec.dart';
import '../link/CoreLinkProtocol.dart';
import 'CoreProxy.dart';

class WebWasmCoreProxy extends CoreProxy {
  const WebWasmCoreProxy();

  @override
  Future<Object?> call(CoreCallRequest request) async {
    final responseBytes = await _invokeBytes('call', <JSAny?>[
      encodeNativeCoreCallRequest(request).toJS,
    ]);
    return decodeNativeCoreResult(responseBytes);
  }

  /// Opens a client-owned Link input stream through the web runtime carrier.
  @override
  Future<CorePushSink> push(CorePushRequest request) async {
    final responseBytes = await _invokeBytes('pushOpen', <JSAny?>[
      encodeNativeCorePushOpenRequest(request).toJS,
    ]);
    final pushId = decodeNativeCorePushOpenResult(responseBytes);
    if (pushId != request.requestId) {
      throw CoreLinkError(
        code: 'INVALID_RESPONSE',
        message: 'web push id mismatch: $pushId',
      );
    }
    return _WebCorePushSink(pushId);
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) async {
    final responseBytes = await _invokeBytes('watchSnapshot', <JSAny?>[
      encodeNativeCoreWatchSnapshotRequest(request).toJS,
    ]);
    return decodeNativeCoreWatchSnapshotResult(responseBytes);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    final subscriptionId =
        'web-watch-${DateTime.now().microsecondsSinceEpoch}-${_nextWebWatchSubscriptionIndex++}';
    final controller = StreamController<CoreEvent>();
    var opened = false;
    var canceled = false;
    late final JSFunction onEvent;
    onEvent = ((JSUint8Array frameBytes) {
      _dispatchWatchFrame(controller, subscriptionId, frameBytes.toDart);
    }).toJS;

    controller.onListen = () {
      unawaited(() async {
        try {
          final subscriptionBytes = await _invokeBytes('watchStream', <JSAny?>[
            encodeNativeCoreWatchStreamRequest(subscriptionId, request).toJS,
            onEvent,
          ]);
          final openedSubscriptionId = decodeNativeCoreWatchStreamResult(
            subscriptionBytes,
          );
          if (openedSubscriptionId != subscriptionId) {
            throw CoreLinkError(
              code: 'INVALID_RESPONSE',
              message:
                  'web watch subscription id mismatch: $openedSubscriptionId',
            );
          }
          opened = true;
          if (canceled) {
            await _closeNativeWebWatchStream(subscriptionId);
          }
        } catch (error, stackTrace) {
          if (!controller.isClosed) {
            controller.addError(error, stackTrace);
            await controller.close();
          }
        }
      }());
    };
    controller.onCancel = () async {
      canceled = true;
      if (opened) {
        await _closeNativeWebWatchStream(subscriptionId);
      }
    };
    return controller.stream;
  }
}

class _WebCorePushSink implements CorePushSink {
  /// Creates an ordered web push sink.
  _WebCorePushSink(this._pushId);

  final String _pushId;
  Future<void> _tail = Future<void>.value();
  int _sequence = 0;
  bool _closed = false;

  /// Queues one input item on the persistent push carrier.
  @override
  Future<void> add(Object? args) {
    if (_closed) {
      throw StateError('Link push stream is closed');
    }
    final sequence = _sequence++;
    _tail = _tail.then((_) async {
      final responseBytes = await _invokeBytes('pushItem', <JSAny?>[
        encodeNativeCorePushItem(_pushId, sequence, args).toJS,
      ]);
      decodeNativeCoreVoidResult(responseBytes);
    });
    return _tail;
  }

  /// Flushes queued items and closes the persistent push stream.
  @override
  Future<void> close() async {
    if (_closed) {
      return;
    }
    _closed = true;
    await _tail;
    final responseBytes = await _invokeBytes('pushClose', <JSAny?>[
      _pushId.toJS,
    ]);
    decodeNativeCoreVoidResult(responseBytes);
  }
}

int _nextWebWatchSubscriptionIndex = 0;

void _dispatchWatchFrame(
  StreamController<CoreEvent> controller,
  String subscriptionId,
  Uint8List frameBytes,
) {
  if (controller.isClosed) {
    return;
  }
  try {
    final frame = decodeNativeCoreWatchFrame(frameBytes);
    if (frame.subscriptionId != subscriptionId) {
      throw CoreLinkError(
        code: 'INVALID_RESPONSE',
        message: 'web watch subscription id mismatch: ${frame.subscriptionId}',
      );
    }
    final event = frame.event;
    controller.add(event);
    if (event.kind == 'Completed') {
      unawaited(_closeNativeWebWatchStream(subscriptionId));
      unawaited(controller.close());
    }
  } catch (error, stackTrace) {
    controller.addError(error, stackTrace);
    unawaited(controller.close());
  }
}

/// Closes one wasm watch subscription and validates its direct acknowledgement.
Future<void> _closeNativeWebWatchStream(String subscriptionId) async {
  final responseBytes = await _invokeBytes('closeWatchStream', <JSAny?>[
    subscriptionId.toJS,
  ]);
  decodeNativeCoreVoidResult(responseBytes);
}

/// Invokes one binary wasm runtime method.
Future<Uint8List> _invokeBytes(String method, List<JSAny?> args) async {
  final runtime = globalContext.getProperty<JSAny?>('__operitRuntime'.toJS);
  if (runtime.isUndefinedOrNull) {
    throw const CoreLinkError(
      code: 'WEB_WASM_BRIDGE_NOT_INSTALLED',
      message: 'window.__operitRuntime is not installed',
    );
  }
  final promise = (runtime as JSObject).callMethodVarArgs<JSPromise<JSAny?>>(
    method.toJS,
    args,
  );
  final value = await promise.toDart;
  if (value.isA<JSUint8Array>()) {
    return (value as JSUint8Array).toDart;
  }
  throw CoreLinkError(
    code: 'WEB_WASM_BRIDGE_INVALID_RESPONSE',
    message: 'window.__operitRuntime.$method returned a non-binary value',
  );
}
