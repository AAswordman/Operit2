// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/services.dart';

import '../browser/BrowserAutomationModels.dart';
import '../web_visit/WebVisitModels.dart';

typedef BrowserAutomationRequestHandler =
    Future<BrowserAutomationResponse> Function(
      BrowserAutomationRequest request,
    );
typedef WebVisitRequestHandler =
    Future<WebVisitResponse> Function(WebVisitRequest request);
typedef RuntimeHostStringRequestHandler =
    Future<String> Function(String payloadJson);

class RuntimeHostBridge {
  RuntimeHostBridge._();

  static const MethodChannel _channel = MethodChannel('operit/runtime');
  static BrowserAutomationRequestHandler? _browserAutomationHandler;
  static WebVisitRequestHandler? _webVisitHandler;
  static final Map<String, RuntimeHostStringRequestHandler> _stringHandlers =
      <String, RuntimeHostStringRequestHandler>{};
  static bool _installed = false;

  static void Function() registerBrowserAutomationHandler(
    BrowserAutomationRequestHandler handler,
  ) {
    _browserAutomationHandler = handler;
    _ensureInstalled();
    return () {
      if (identical(_browserAutomationHandler, handler)) {
        _browserAutomationHandler = null;
      }
    };
  }

  static void Function() registerWebVisitHandler(
    WebVisitRequestHandler handler,
  ) {
    _webVisitHandler = handler;
    _ensureInstalled();
    return () {
      if (identical(_webVisitHandler, handler)) {
        _webVisitHandler = null;
      }
    };
  }

  static void Function() registerStringHandler(
    String methodName,
    RuntimeHostStringRequestHandler handler,
  ) {
    final normalizedMethodName = methodName.trim();
    if (normalizedMethodName.isEmpty) {
      throw ArgumentError.value(methodName, 'methodName');
    }
    _stringHandlers[normalizedMethodName] = handler;
    _ensureInstalled();
    return () {
      if (identical(_stringHandlers[normalizedMethodName], handler)) {
        _stringHandlers.remove(normalizedMethodName);
      }
    };
  }

  static void _ensureInstalled() {
    if (_installed) {
      return;
    }
    _channel.setMethodCallHandler(_handleMethodCall);
    _installed = true;
  }

  static Future<Object?> _handleMethodCall(MethodCall call) async {
    switch (call.method) {
      case 'browserAutomationRequest':
        final handler = _browserAutomationHandler;
        if (handler == null) {
          throw StateError('browser automation handler is not registered');
        }
        final request = BrowserAutomationRequest.decode(
          call.arguments as String?,
        );
        if (request == null) {
          throw StateError('browser automation request is empty');
        }
        final response = await handler(request);
        return jsonEncode(response.toJson());
      case 'webVisitRequest':
        final handler = _webVisitHandler;
        if (handler == null) {
          throw StateError('web visit handler is not registered');
        }
        final request = WebVisitRequest.decode(call.arguments as String?);
        if (request == null) {
          throw StateError('web visit request is empty');
        }
        final response = await handler(request);
        return jsonEncode(response.toJson());
      default:
        final handler = _stringHandlers[call.method];
        if (handler != null) {
          final payloadJson = call.arguments as String?;
          if (payloadJson == null) {
            throw StateError('runtime host request is empty: ${call.method}');
          }
          return handler(payloadJson);
        }
        throw MissingPluginException(
          'runtime host method is not registered: ${call.method}',
        );
    }
  }
}
