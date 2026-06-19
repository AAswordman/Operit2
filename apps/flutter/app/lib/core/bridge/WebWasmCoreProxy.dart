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
    final responseText = await _invokeString('call', <Object?>[requestText]);
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
    final responseText = await _invokeString('watchSnapshot', <Object?>[
      jsonEncode(request.toJson()),
    ]);
    final response = jsonDecode(responseText) as Map<String, Object?>;
    if (response.containsKey('code') && response.containsKey('message')) {
      final error = CoreLinkError.fromJson(response);
      throw error;
    }
    return CoreEvent.fromJson(response);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) async* {
    final subscriptionText = await _invokeString('watchStream', <Object?>[
      jsonEncode(request.toJson()),
    ]);
    final subscriptionJson =
        jsonDecode(subscriptionText) as Map<String, Object?>;
    if (subscriptionJson.containsKey('code') &&
        subscriptionJson.containsKey('message')) {
      throw CoreLinkError.fromJson(subscriptionJson);
    }
    final subscriptionId = subscriptionJson['subscriptionId'] as String;
    var completed = false;
    try {
      while (!completed) {
        await Future<void>.delayed(const Duration(milliseconds: 24));
        final eventsText = await _invokeString('pollWatchStream', <Object?>[
          subscriptionId,
        ]);
        final decodedEvents = jsonDecode(eventsText);
        if (decodedEvents is Map<String, Object?> &&
            decodedEvents.containsKey('code') &&
            decodedEvents.containsKey('message')) {
          throw CoreLinkError.fromJson(decodedEvents);
        }
        final eventsJson = decodedEvents as List<Object?>;
        for (final eventJson in eventsJson.cast<Map<String, Object?>>()) {
          final event = CoreEvent.fromJson(eventJson);
          yield event;
          if (event.kind == 'Completed') {
            completed = true;
          }
        }
      }
    } finally {
      await _invokeString('closeWatchStream', <Object?>[subscriptionId]);
    }
  }

  @override
  Future<String> dispatchHostEvent(String source, String payloadJson) {
    return _invokeString('dispatchHostEvent', <Object?>[source, payloadJson]);
  }
}

Future<String> _invokeString(String method, List<Object?> args) async {
  final runtime = globalContext.getProperty<JSAny?>('__operitRuntime'.toJS);
  if (runtime.isUndefinedOrNull) {
    throw const CoreLinkError(
      code: 'WEB_WASM_BRIDGE_NOT_INSTALLED',
      message: 'window.__operitRuntime is not installed',
    );
  }
  final promise = (runtime as JSObject).callMethodVarArgs<JSPromise<JSAny?>>(
    method.toJS,
    args.map(_toJsValue).toList(growable: false),
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

JSAny? _toJsValue(Object? value) {
  if (value == null) {
    return null;
  }
  if (value is String) {
    return value.toJS;
  }
  if (value is bool) {
    return value.toJS;
  }
  if (value is int) {
    return value.toJS;
  }
  if (value is double) {
    return value.toJS;
  }
  return value.jsify();
}
