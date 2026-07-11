// ignore_for_file: file_names

import 'dart:convert';

typedef ComposeWebViewControllerHandler =
    Future<String> Function(String commandJson);

class ComposeWebViewControllerBridge {
  const ComposeWebViewControllerBridge();

  static ComposeWebViewControllerHandler? _handler;

  /// Dispatches a Compose WebView command to the active foreground host.
  static Future<String> handle(String commandJson) {
    final handler = _handler;
    if (handler == null) {
      return Future<String>.value(
        jsonEncode(<String, Object?>{
          'success': false,
          'message': 'compose webview foreground host is not active',
        }),
      );
    }
    return handler(commandJson);
  }

  /// Registers the foreground Compose WebView command implementation.
  void Function() registerHandler(ComposeWebViewControllerHandler handler) {
    _handler = handler;
    return () {
      if (identical(_handler, handler)) {
        _handler = null;
      }
    };
  }
}
