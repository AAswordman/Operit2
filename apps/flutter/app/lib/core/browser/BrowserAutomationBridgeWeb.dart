// ignore_for_file: file_names

import 'dart:convert';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';

import '../link/CoreLinkProtocol.dart';
import 'BrowserAutomationModels.dart';

class BrowserAutomationBridge {
  const BrowserAutomationBridge();

  Future<BrowserAutomationRequest?> nextRequest() async {
    final responseText = await _invokeString(
      'nextBrowserAutomationRequest',
      const <Object?>[],
    );
    return BrowserAutomationRequest.decode(responseText);
  }

  Future<void> handleResult(BrowserAutomationResponse response) async {
    await _invokeString('handleBrowserAutomationResult', <Object?>[
      jsonEncode(response.toJson()),
    ]);
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
  return value.jsify();
}
