// ignore_for_file: file_names

import 'WebVisitModels.dart';

typedef WebVisitRequestHandler =
    Future<WebVisitResponse> Function(WebVisitRequest request);

class WebVisitBridge {
  const WebVisitBridge();

  static WebVisitRequestHandler? _handler;

  static Future<WebVisitResponse> handle(WebVisitRequest request) {
    final handler = _handler;
    if (handler == null) {
      throw StateError('web visit handler is not registered');
    }
    return handler(request);
  }

  void Function() registerHandler(WebVisitRequestHandler handler) {
    _handler = handler;
    return () {
      if (identical(_handler, handler)) {
        _handler = null;
      }
    };
  }
}
