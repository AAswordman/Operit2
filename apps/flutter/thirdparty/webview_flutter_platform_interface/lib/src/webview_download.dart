import 'dart:typed_data';

/// Lifecycle of a transfer owned by the browser engine.
enum WebViewDownloadState { running, completed, failed, cancelled }

/// Reports an engine transfer without replaying its HTTP request.
class WebViewDownloadEvent {
  /// Creates a transfer update; completed events expose the original file bytes.
  const WebViewDownloadEvent({
    required this.id,
    required this.url,
    required this.fileName,
    required this.state,
    this.receivedBytes = 0,
    this.totalBytes,
    this.error,
    this.readBytes,
    this.cancel,
  });

  final String id;
  final String url;
  final String fileName;
  final WebViewDownloadState state;
  final int receivedBytes;
  final int? totalBytes;
  final String? error;
  final Future<Uint8List> Function()? readBytes;
  final Future<void> Function()? cancel;
}
