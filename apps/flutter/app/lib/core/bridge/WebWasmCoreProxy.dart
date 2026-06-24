// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';

import '../link/CoreLinkProtocol.dart';
import 'CoreProxy.dart';

class WebWasmCoreProxy extends CoreProxy {
  const WebWasmCoreProxy();

  @override
  Future<Object?> call(CoreCallRequest request) async {
    final requestText = jsonEncode(request.toJson());
    final responseText = await _invokeString('call', <JSAny?>[
      requestText.toJS,
    ]);
    final response = jsonDecode(responseText) as Map<String, Object?>;
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

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) async {
    final responseText = await _invokeString('watchSnapshot', <JSAny?>[
      jsonEncode(request.toJson()).toJS,
    ]);
    final response = jsonDecode(responseText) as Map<String, Object?>;
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
    onEvent = ((JSString frameText) {
      _dispatchWatchFrame(controller, subscriptionId, frameText.toDart);
    }).toJS;

    controller.onListen = () {
      unawaited(() async {
        try {
          final subscriptionText = await _invokeString('watchStream', <JSAny?>[
            jsonEncode(<String, Object?>{
              'channelId': 'web-watch-channel',
              'subscriptionId': subscriptionId,
              'request': request.toJson(),
            }).toJS,
            onEvent,
          ]);
          final subscriptionJson =
              jsonDecode(subscriptionText) as Map<String, Object?>;
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
            await _invokeString('closeWatchStream', <JSAny?>[
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
        await _invokeString('closeWatchStream', <JSAny?>[subscriptionId.toJS]);
      }
    };
    return controller.stream;
  }
}

int _nextWebWatchSubscriptionIndex = 0;

void _dispatchWatchFrame(
  StreamController<CoreEvent> controller,
  String subscriptionId,
  String frameText,
) {
  if (controller.isClosed) {
    return;
  }
  try {
    final frame = jsonDecode(frameText) as Map<String, Object?>;
    if (frame['subscriptionId'] != subscriptionId) {
      throw CoreLinkError(
        code: 'INVALID_RESPONSE',
        message:
            'web watch subscription id mismatch: ${frame['subscriptionId']}',
      );
    }
    final event = CoreEvent.fromJson(frame['event'] as Map<String, Object?>);
    controller.add(event);
    if (event.kind == 'Completed') {
      unawaited(
        _invokeString('closeWatchStream', <JSAny?>[subscriptionId.toJS]),
      );
      unawaited(controller.close());
    }
  } catch (error, stackTrace) {
    controller.addError(error, stackTrace);
    unawaited(controller.close());
  }
}

Future<String> _invokeString(String method, List<JSAny?> args) async {
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
  if (value.isA<JSString>()) {
    return (value as JSString).toDart;
  }
  throw CoreLinkError(
    code: 'WEB_WASM_BRIDGE_INVALID_RESPONSE',
    message: 'window.__operitRuntime.$method returned a non-string value',
  );
}
