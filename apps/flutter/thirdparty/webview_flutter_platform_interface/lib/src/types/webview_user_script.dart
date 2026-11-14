import 'package:flutter/foundation.dart';

/// The lifecycle point at which a user script is injected.
enum WebViewUserScriptInjectionTime {
  /// Inject after the document object is created and before page scripts run.
  ///
  /// Android providers without document-start support inject into the main frame
  /// from navigation callbacks, which may run after the page's own scripts.
  documentStart,
}

/// A script registered for injection into future documents.
@immutable
class WebViewUserScript {
  /// Creates a user script.
  const WebViewUserScript({
    required this.source,
    this.injectionTime = WebViewUserScriptInjectionTime.documentStart,
    this.forMainFrameOnly = true,
  });

  /// The JavaScript source to inject.
  ///
  /// The source executes in a private function scope on every supported
  /// platform. Assign values to `globalThis` when they must be visible to page
  /// scripts or other registered user scripts.
  final String source;

  /// The lifecycle point at which the script is injected.
  final WebViewUserScriptInjectionTime injectionTime;

  /// Whether the script is restricted to the main frame.
  final bool forMainFrameOnly;
}
