// ignore_for_file: file_names

import 'BrowserAutomationModels.dart';

typedef BrowserAutomationRequestHandler =
    Future<BrowserAutomationResponse> Function(
      BrowserAutomationRequest request,
    );

class BrowserAutomationBridge {
  const BrowserAutomationBridge();

  static BrowserAutomationRequestHandler? _handler;

  static Future<BrowserAutomationResponse> handle(
    BrowserAutomationRequest request,
  ) {
    final handler = _handler;
    if (handler == null) {
      throw StateError('browser automation handler is not registered');
    }
    return handler(request);
  }

  void Function() registerHandler(BrowserAutomationRequestHandler handler) {
    _handler = handler;
    return () {
      if (identical(_handler, handler)) {
        _handler = null;
      }
    };
  }
}
