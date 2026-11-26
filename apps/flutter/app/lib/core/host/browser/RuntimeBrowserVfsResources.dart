// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:mime/mime.dart';
import 'package:webview_all/webview_all.dart';

/// Maps file URLs to a WebView-owned virtual origin backed by Core VFS.
class RuntimeBrowserVfsResources {
  /// Creates a resource handler with the runtime's file reader.
  RuntimeBrowserVfsResources(this.readBytes);

  static const host = 'browser.operit.invalid';
  final Future<Uint8List> Function(String path) readBytes;

  /// Converts a public file URL into the platform's local-resource origin.
  Uri navigationUri(String url, String? scheme) {
    final uri = Uri.parse(url);
    if (uri.scheme != 'file') return uri;
    if (uri.host.isNotEmpty) {
      throw ArgumentError('Browser VFS file URLs cannot have a host');
    }
    if (scheme == null) {
      throw UnsupportedError('This WebView does not support VFS resources');
    }
    return uri.replace(scheme: scheme, host: host);
  }

  /// Converts a virtual-origin URL back to its public file identity.
  String logicalUrl(String url) {
    final uri = Uri.tryParse(url);
    if (uri == null ||
        uri.host != host ||
        (uri.scheme != 'https' && uri.scheme != 'operit-vfs')) {
      return url;
    }
    return uri.replace(scheme: 'file', host: '').toString();
  }

  /// Reads each local resource through VFS, including relative page assets.
  Future<WebViewLocalResourceResponse> handle(
    WebViewLocalResourceRequest request,
  ) async {
    final uri = Uri.parse(request.url);
    if (uri.host != host ||
        (uri.scheme != 'https' && uri.scheme != 'operit-vfs')) {
      return WebViewLocalResourceResponse(
        body: Uint8List(0),
        statusCode: 404,
        reasonPhrase: 'Not Found',
      );
    }
    if (request.method != 'GET' && request.method != 'HEAD') {
      return WebViewLocalResourceResponse(
        body: Uint8List(0),
        statusCode: 405,
        reasonPhrase: 'Method Not Allowed',
      );
    }
    final path = Uri.decodeComponent(uri.path);
    final bytes = await readBytes(path);
    return WebViewLocalResourceResponse(
      body: request.method == 'HEAD' ? Uint8List(0) : bytes,
      mimeType:
          lookupMimeType(path, headerBytes: bytes) ??
          'application/octet-stream',
    );
  }
}
