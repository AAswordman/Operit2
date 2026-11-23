import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

/// Observes Android's process-owned browser and embeds its actual WebView.
class AndroidWebVisitHost extends PlatformWebVisitHost {
  static const _commands = MethodChannel('operit/webview_visits');
  static const _events = EventChannel('operit/webview_visit_events');

  /// Receives replayed active tasks and subsequent authoritative updates.
  @override
  Stream<Map<String, Object?>> watchVisits() => _events
      .receiveBroadcastStream()
      .map((event) => Map<String, Object?>.from(event as Map));

  /// Executes the same host path for externally owned runtimes.
  @override
  Future<Map<String, Object?>> visit(Map<String, Object?> request) async =>
      Map<String, Object?>.from(
        (await _commands.invokeMapMethod<String, Object?>('visit', request))!,
      );

  /// Sends a control action without assuming that a surface is mounted.
  @override
  Future<void> control(String requestId, String action) =>
      _commands.invokeMethod<void>('control', {
        'requestId': requestId,
        'action': action,
      });

  /// Attaches the host's existing native view, preserving navigation and DOM.
  @override
  Widget buildSurface(String requestId) => AndroidView(
    key: ValueKey('host-visit-$requestId'),
    viewType: 'operit/host_web_visit',
    creationParams: requestId,
    creationParamsCodec: const StandardMessageCodec(),
    gestureRecognizers: {
      Factory<OneSequenceGestureRecognizer>(EagerGestureRecognizer.new),
    },
    layoutDirection: TextDirection.ltr,
  );
}
