// ignore_for_file: file_names

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:webview_all/webview_all.dart';

typedef ComposeDslWebViewResourceDecisionDispatcher =
    Future<Object?> Function(Map<String, Object?> payload);

/// Routes native WebView requests to the existing plugin/VFS resource API.
/// No listening socket is used. Storage identity must never include a view,
/// route, execution ID or the exported resource's temporary directory.
class ComposeDslWebViewResourceLoader {
  ComposeDslWebViewResourceLoader({
    required this.packageName,
    required this.scheme,
    required this.dispatchDecision,
    required this.readFileBytes,
  }) {
    if (packageName.trim().isEmpty) {
      throw ArgumentError.value(packageName, 'packageName');
    }
    if (scheme != 'https' && scheme != 'operit-vfs') {
      throw ArgumentError.value(scheme, 'scheme');
    }
  }

  final String packageName;
  final String scheme;
  final ComposeDslWebViewResourceDecisionDispatcher dispatchDecision;
  final Future<List<int>> Function(String path) readFileBytes;
  final Map<String, Uri> _origins = {};
  final Set<HttpClient> _clients = {};
  bool _closed = false;

  Future<Uri> localUriFor(
    String originalUrl, {
    required bool isMainFrame,
  }) async {
    if (_closed) throw StateError('Resource loader is closed');
    if (ownsUrl(originalUrl)) return Uri.parse(originalUrl);
    final original = Uri.parse(originalUrl);
    final origin = _originOf(original);
    // Version this identity only when intentionally migrating browser storage.
    final digest = sha256
        .convert(
          utf8.encode(
            jsonEncode(['operit-webview-v1', packageName, origin.toString()]),
          ),
        )
        .toString();
    // Keep DNS labels below 63 characters, while retaining the full digest.
    final host =
        'p-${digest.substring(0, 32)}.${digest.substring(32)}.operit.invalid';
    _origins[host] = origin;
    return original.replace(
      scheme: scheme,
      host: host,
      port: scheme == 'https' ? 443 : 0,
    );
  }

  bool ownsUrl(String url) {
    final uri = Uri.tryParse(url);
    return !_closed &&
        uri != null &&
        uri.scheme == scheme &&
        uri.userInfo.isEmpty &&
        !uri.hasPort &&
        _origins.containsKey(uri.host);
  }

  bool matchesCurrentOrigin(String url) {
    try {
      return _origins.containsValue(_originOf(Uri.parse(url)));
    } on ArgumentError {
      return false;
    }
  }

  String originalUrlFor(String url) {
    if (!ownsUrl(url)) return url;
    final uri = Uri.parse(url);
    final origin = _origins[uri.host]!;
    return uri
        .replace(scheme: origin.scheme, host: origin.host, port: origin.port)
        .toString();
  }

  Future<void> close() async {
    _closed = true;
    _origins.clear();
    for (final client in _clients) {
      client.close(force: true);
    }
    _clients.clear();
  }

  Future<WebViewLocalResourceResponse> handleRequest(
    WebViewLocalResourceRequest request,
  ) async {
    if (_closed) return _empty(410, 'Gone');
    if (!ownsUrl(request.url)) return _empty(403, 'Forbidden');
    try {
      final result = await _resolve(request, originalUrlFor(request.url), 0);
      if (_closed) return _empty(410, 'Gone');
      if (request.method == 'HEAD') {
        return WebViewLocalResourceResponse(
          body: Uint8List(0),
          statusCode: result.statusCode,
          reasonPhrase: result.reasonPhrase,
          mimeType: result.mimeType,
          encoding: result.encoding,
          headers: result.headers,
        );
      }
      return result;
    } catch (_) {
      // Do not expose host file paths or internal exception details to pages.
      return _empty(500, 'Resource Error');
    }
  }

  Future<WebViewLocalResourceResponse> _resolve(
    WebViewLocalResourceRequest request,
    String originalUrl,
    int depth,
  ) async {
    if (depth > 8) return _empty(508, 'Rewrite Loop');
    final uri = Uri.parse(originalUrl);
    _originOf(uri); // Reject unsupported schemes and remote file authorities.
    final decision = _map(
      await dispatchDecision({
        'url': originalUrl,
        'method': request.method,
        'headers': request.headers,
        'isMainFrame': request.isMainFrame,
        'hasGesture': false,
        'isRedirect': depth > 0,
        'scheme': uri.scheme,
      }),
    );
    if (_closed) return _empty(410, 'Gone');
    switch (decision['action']) {
      case 'block':
        return _empty(204, 'No Content');
      case 'rewrite':
        final target = (decision['url'] as String? ?? '').trim();
        if (target.isEmpty) return _empty(204, 'No Content');
        // Android's native response API cannot represent HTTP redirects.
        // Resolve resource rewrites internally; navigation rewrites remain in
        // onShouldOverrideUrlLoading and update the document URL as before.
        return _resolve(
          request,
          originalUrlFor(uri.resolve(target).toString()),
          depth + 1,
        );
      case 'respond':
        return _response(decision['response']);
      default:
        return _proxy(request, uri);
    }
  }

  Future<WebViewLocalResourceResponse> _response(Object? raw) async {
    final value = _map(raw);
    final status = int.tryParse('${value['statusCode']}') ?? 200;
    if (status < 100 || status > 599 || (status >= 300 && status < 400)) {
      return _empty(502, 'Unsupported Resource Status');
    }
    final headers = _headers(value['headers']);
    final contentType = headers.entries
        .where((e) => e.key.toLowerCase() == 'content-type')
        .firstOrNull
        ?.value;
    final parsedType = contentType == null
        ? null
        : ContentType.parse(contentType);
    final mime =
        (value['mimeType'] as String?) ?? parsedType?.mimeType ?? 'text/plain';
    final encoding =
        Encoding.getByName(
          value['encoding'] as String? ?? parsedType?.charset ?? 'utf-8',
        ) ??
        utf8;
    final filePath = (value['filePath'] as String? ?? '').trim();
    final List<int> bytes;
    if (filePath.isNotEmpty) {
      bytes = await readFileBytes(filePath);
    } else if (value.containsKey('base64')) {
      bytes = base64.decode(value['base64'] as String? ?? '');
    } else {
      bytes = encoding.encode(value['text']?.toString() ?? '');
    }
    return WebViewLocalResourceResponse(
      body: Uint8List.fromList(bytes),
      statusCode: status,
      reasonPhrase: _reason(value['reasonPhrase'], status),
      mimeType: mime,
      encoding: encoding.name,
      headers: headers,
    );
  }

  Future<WebViewLocalResourceResponse> _proxy(
    WebViewLocalResourceRequest request,
    Uri uri,
  ) async {
    // Native interception APIs do not consistently supply request bodies. Never
    // silently forward a POST/PUT without its body or perform it twice.
    if (request.method != 'GET' && request.method != 'HEAD') {
      return _empty(405, 'Method Not Allowed');
    }
    if (uri.scheme != 'http' && uri.scheme != 'https') {
      return _empty(404, 'Not Found');
    }
    if (uri.host.endsWith('.operit.invalid')) return _empty(403, 'Forbidden');
    final client = HttpClient()
      ..connectionTimeout = const Duration(seconds: 15);
    _clients.add(client);
    try {
      final outgoing = await client.openUrl(request.method, uri);
      for (final header in request.headers.entries) {
        if (!const {
          'host',
          'connection',
          'content-length',
          'accept-encoding',
          'cookie',
          'origin',
          'referer',
        }.contains(header.key.toLowerCase())) {
          outgoing.headers.set(header.key, header.value);
        }
      }
      final incoming = await outgoing.close().timeout(
        const Duration(seconds: 20),
      );
      if (incoming.statusCode >= 300 && incoming.statusCode < 400) {
        return _empty(502, 'Unsupported Resource Redirect');
      }
      final bytes = BytesBuilder(copy: false);
      await for (final chunk in incoming.timeout(const Duration(seconds: 20))) {
        bytes.add(chunk);
      }
      final headers = <String, String>{};
      incoming.headers.forEach((name, values) {
        if (!const {
          'connection',
          'transfer-encoding',
          'content-encoding',
          'content-length',
          'set-cookie',
        }.contains(name)) {
          headers[name] = values.join(', ');
        }
      });
      return WebViewLocalResourceResponse(
        body: bytes.takeBytes(),
        statusCode: incoming.statusCode,
        reasonPhrase: _reason(incoming.reasonPhrase, incoming.statusCode),
        mimeType:
            incoming.headers.contentType?.mimeType ??
            'application/octet-stream',
        encoding: incoming.headers.contentType?.charset ?? 'utf-8',
        headers: headers,
      );
    } catch (_) {
      return _empty(502, 'Bad Gateway');
    } finally {
      _clients.remove(client);
      client.close(force: true);
    }
  }

  static Uri _originOf(Uri uri) {
    if (uri.userInfo.isNotEmpty ||
        (uri.scheme != 'file' &&
            uri.scheme != 'http' &&
            uri.scheme != 'https') ||
        (uri.scheme == 'file' && (uri.host.isNotEmpty || uri.hasPort)) ||
        (uri.scheme != 'file' &&
            (uri.host.isEmpty || uri.host.endsWith('.operit.invalid')))) {
      throw ArgumentError.value(uri, 'url', 'Not a supported resource origin');
    }
    return Uri(
      scheme: uri.scheme,
      host: uri.host,
      port: uri.hasPort ? uri.port : null,
    );
  }

  static Map<String, Object?> _map(Object? value) => value is Map
      ? value.map((key, value) => MapEntry(key.toString(), value))
      : {};
  static Map<String, String> _headers(Object? value) =>
      _map(value).map((key, value) => MapEntry(key, value.toString()));
  static String _reason(Object? value, int status) {
    final text = value?.toString().trim() ?? '';
    return text.isNotEmpty && text.codeUnits.every((c) => c >= 32 && c < 127)
        ? text
        : (status == 200 ? 'OK' : 'Resource Response');
  }

  static WebViewLocalResourceResponse _empty(int status, String reason) =>
      WebViewLocalResourceResponse(
        body: Uint8List(0),
        statusCode: status,
        reasonPhrase: reason,
      );
}
