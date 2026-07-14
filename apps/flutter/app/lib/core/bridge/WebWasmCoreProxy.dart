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
      encodeCoreLink(request.toJson()).toJS,
    ]);
    final response = decodeCoreLinkMap(responseBytes);
    final result = response['result'] as Map<String, Object?>;
    if (result.containsKey('Ok')) {
      return result['Ok'];
    }
    if (result.containsKey('Err')) {
      final error = CoreLinkError.fromJson(
        result['Err'] as Map<String, Object?>,
      );
      throw error;
    }
    throw const CoreLinkError(
      code: 'INVALID_RESPONSE',
      message: 'wasm runtime response result is invalid',
    );
  }

  /// Opens a client-owned Link input stream through the web runtime carrier.
  @override
  Future<CorePushSink> push(CorePushRequest request) async {
    final responseBytes = await _invokeBytes('pushOpen', <JSAny?>[
      encodeCoreLink(request.toJson()).toJS,
    ]);
    final response = decodeCoreLinkMap(responseBytes);
    final pushId = response['pushId'] as String;
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
      encodeCoreLink(request.toJson()).toJS,
    ]);
    final response = decodeCoreLinkMap(responseBytes);
    if (response.containsKey('code') && response.containsKey('message')) {
      final error = CoreLinkError.fromJson(response);
      throw error;
    }
    return CoreEvent.fromJson(response);
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
      _dispatchWatchFrame(
        controller,
        subscriptionId,
        frameBytes.toDart,
        onTransportError: () {
          opened = false;
        },
      );
    }).toJS;

    controller.onListen = () {
      unawaited(() async {
        try {
          final subscriptionBytes = await _invokeBytes('watchStream', <JSAny?>[
            encodeCoreLink(<String, Object?>{
              'channelId': 'web-watch-channel',
              'subscriptionId': subscriptionId,
              'request': request.toJson(),
            }).toJS,
            onEvent,
          ]);
          final subscriptionJson = decodeCoreLinkMap(subscriptionBytes);
          if (subscriptionJson.containsKey('code') &&
              subscriptionJson.containsKey('message')) {
            throw CoreLinkError.fromJson(subscriptionJson);
          }
          if (subscriptionJson['subscriptionId'] != subscriptionId) {
            throw CoreLinkError(
              code: 'INVALID_RESPONSE',
              message:
                  'web watch subscription id mismatch: ${subscriptionJson['subscriptionId']}',
            );
          }
          opened = true;
          if (canceled) {
            await _invokeBytes('closeWatchStream', <JSAny?>[
              subscriptionId.toJS,
            ]);
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
        await _invokeBytes('closeWatchStream', <JSAny?>[subscriptionId.toJS]);
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
      await _invokeBytes('pushItem', <JSAny?>[
        encodeCoreLink(<String, Object?>{
          'pushId': _pushId,
          'sequence': sequence,
          'args': args,
        }).toJS,
      ]);
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
    await _invokeBytes('pushClose', <JSAny?>[_pushId.toJS]);
  }
}

int _nextWebWatchSubscriptionIndex = 0;

void _dispatchWatchFrame(
  StreamController<CoreEvent> controller,
  String subscriptionId,
  Uint8List frameBytes, {
  required void Function() onTransportError,
}) {
  if (controller.isClosed) {
    return;
  }
  try {
    final frame = decodeCoreLinkMap(frameBytes);
    if (frame['subscriptionId'] != subscriptionId) {
      throw CoreLinkError(
        code: 'INVALID_RESPONSE',
        message:
            'web watch subscription id mismatch: ${frame['subscriptionId']}',
      );
    }
    final errorCode = frame['errorCode'];
    if (errorCode is String) {
      onTransportError();
      throw CoreLinkError(
        code: errorCode,
        message: frame['errorMessage'] as String,
      );
    }
    final event = CoreEvent.fromJson(frame['event'] as Map<String, Object?>);
    controller.add(event);
    if (event.kind == 'Completed') {
      unawaited(
        _invokeBytes('closeWatchStream', <JSAny?>[subscriptionId.toJS]),
      );
      unawaited(controller.close());
    }
  } catch (error, stackTrace) {
    controller.addError(error, stackTrace);
    unawaited(controller.close());
  }
}

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
