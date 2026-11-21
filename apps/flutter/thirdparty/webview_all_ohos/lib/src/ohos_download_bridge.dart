import 'package:flutter/services.dart';
import 'package:webview_platform_interface/webview_platform_interface.dart';

/// Routes engine downloads to the controller owning the originating WebView.
class OhosDownloadBridge {
  static const _channel = MethodChannel('operit/webview_downloads');
  static final _listeners = <int, void Function(WebViewDownloadEvent)>{};

  /// Installs one shared channel handler and registers a live browser owner.
  static Future<void> attach(
    int identifier,
    void Function(WebViewDownloadEvent) callback,
  ) async {
    if (_listeners.isEmpty) {
      _channel.setMethodCallHandler(_handle);
    }
    _listeners[identifier] = callback;
    try {
      await _channel.invokeMethod<void>('configure', {
        'identifier': identifier,
      });
    } catch (_) {
      _listeners.remove(identifier);
      rethrow;
    }
  }

  /// Cancels transfers before releasing an owning browser session.
  static Future<void> detach(int identifier) async {
    _listeners.remove(identifier);
    await _channel.invokeMethod<void>('dispose', {'identifier': identifier});
  }

  /// Preserves native request ownership while forwarding progress and file access.
  static Future<void> _handle(MethodCall call) async {
    if (call.method != 'download') {
      throw MissingPluginException(call.method);
    }
    final event = Map<String, Object?>.from(call.arguments as Map);
    final id = event['id'] as String;
    final identifier = (event['identifier'] as num).toInt();
    final state = WebViewDownloadState.values.byName(event['state'] as String);
    final callback = _listeners[identifier];
    if (callback == null) {
      await _channel.invokeMethod<void>('cancel', {'id': id});
      return;
    }
    final total = (event['totalBytes'] as num).toInt();
    callback(
      WebViewDownloadEvent(
        id: id,
        url: event['url'] as String,
        fileName: event['fileName'] as String,
        state: state,
        receivedBytes: (event['receivedBytes'] as num).toInt(),
        totalBytes: total > 0 ? total : null,
        error: event['error'] as String?,
        cancel: () => _channel.invokeMethod<void>('cancel', {'id': id}),
        readBytes: state == WebViewDownloadState.completed
            ? () async {
                final bytes = await _channel.invokeMethod<Uint8List>('read', {
                  'id': id,
                });
                if (bytes == null) {
                  throw StateError('The engine download file is missing');
                }
                return bytes;
              }
            : null,
      ),
    );
  }
}
