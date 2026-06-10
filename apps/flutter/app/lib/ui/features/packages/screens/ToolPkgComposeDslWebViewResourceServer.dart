// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:io';

typedef ComposeDslWebViewResourceDecisionDispatcher =
    Future<Object?> Function(Map<String, Object?> payload);

class ComposeDslWebViewResourceServer {
  ComposeDslWebViewResourceServer({required this.dispatchDecision});

  final ComposeDslWebViewResourceDecisionDispatcher dispatchDecision;

  HttpServer? _server;
  String? _origin;

  Future<Uri> localUriFor(
    String originalUrl, {
    required bool isMainFrame,
  }) async {
    final originalUri = Uri.parse(originalUrl);
    _origin = _originOf(originalUri);
    await _ensureStarted();
    final server = _server!;
    final queryParameters = <String, List<String>>{
      ...originalUri.queryParametersAll,
      if (isMainFrame) '__operit_main_frame': const <String>['1'],
    };
    return Uri(
      scheme: 'http',
      host: server.address.address,
      port: server.port,
      path: originalUri.path.isEmpty ? '/' : originalUri.path,
      queryParameters: queryParameters.isEmpty ? null : queryParameters,
    );
  }

  bool ownsUrl(String url) {
    final uri = Uri.tryParse(url);
    final server = _server;
    return uri != null &&
        server != null &&
        uri.scheme == 'http' &&
        uri.host == server.address.address &&
        uri.port == server.port;
  }

  bool matchesCurrentOrigin(String url) {
    final uri = Uri.tryParse(url);
    final origin = _origin;
    return uri != null && origin != null && _originOf(uri) == origin;
  }

  String originalUrlFor(String url) {
    final uri = Uri.tryParse(url);
    if (uri == null || !ownsUrl(url)) {
      return url;
    }
    return _originalUrlForLocalUri(uri);
  }

  Future<void> close() async {
    final server = _server;
    _server = null;
    await server?.close(force: true);
  }

  Future<void> _ensureStarted() async {
    if (_server != null) {
      return;
    }
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    _server = server;
    unawaited(_serve(server));
  }

  Future<void> _serve(HttpServer server) async {
    await for (final request in server) {
      await _handleRequest(request);
    }
  }

  Future<void> _handleRequest(HttpRequest request) async {
    final originalUrl = _originalUrlForLocalUri(request.uri);
    final payload = <String, Object?>{
      'url': originalUrl,
      'method': request.method,
      'headers': _headersMap(request.headers),
      'isMainFrame': request.uri.queryParameters['__operit_main_frame'] == '1',
      'hasGesture': false,
      'isRedirect': false,
      'scheme': Uri.tryParse(originalUrl)?.scheme,
    };
    try {
      final rawDecision = await dispatchDecision(payload);
      await _writeDecision(request, originalUrl, rawDecision);
    } catch (error) {
      await _writeText(
        request.response,
        statusCode: HttpStatus.internalServerError,
        reasonPhrase: 'Internal Server Error',
        mimeType: 'text/plain',
        text: error.toString(),
      );
    }
  }

  Future<void> _writeDecision(
    HttpRequest request,
    String originalUrl,
    Object? rawDecision,
  ) async {
    final decision = _objectMap(rawDecision);
    final action = _string(decision['action']).trim();
    switch (action) {
      case 'block':
        request.response.statusCode = HttpStatus.noContent;
        await request.response.close();
        return;
      case 'rewrite':
        final url = _string(decision['url']).trim();
        if (url.isEmpty) {
          request.response.statusCode = HttpStatus.noContent;
          await request.response.close();
          return;
        }
        final redirectUrl = matchesCurrentOrigin(url)
            ? (await localUriFor(url, isMainFrame: false)).toString()
            : url;
        request.response.statusCode = HttpStatus.found;
        request.response.headers.set(HttpHeaders.locationHeader, redirectUrl);
        await request.response.close();
        return;
      case 'respond':
        await _writeResourceResponse(request.response, decision['response']);
        return;
      case 'allow':
        await _proxyOriginalRequest(request, originalUrl);
        return;
      default:
        await _proxyOriginalRequest(request, originalUrl);
        return;
    }
  }

  Future<void> _writeResourceResponse(
    HttpResponse response,
    Object? rawResponse,
  ) async {
    final resource = _objectMap(rawResponse);
    final statusCode = _int(resource['statusCode'], HttpStatus.ok);
    final reasonPhrase = _string(resource['reasonPhrase']).trim();
    final mimeType = _string(resource['mimeType']).trim();
    final headers = _stringMap(resource['headers']);
    for (final entry in headers.entries) {
      response.headers.set(entry.key, entry.value);
    }
    response.statusCode = statusCode;
    if (reasonPhrase.isNotEmpty) {
      response.reasonPhrase = reasonPhrase;
    }
    final filePath = _string(resource['filePath']).trim();
    if (filePath.isNotEmpty) {
      await _writeFile(response, filePath, mimeType);
      return;
    }
    final base64Text = _string(resource['base64']).trim();
    if (base64Text.isNotEmpty) {
      final bytes = base64.decode(base64Text);
      _setContentType(response, mimeType);
      response.add(bytes);
      await response.close();
      return;
    }
    await _writeText(
      response,
      statusCode: statusCode,
      reasonPhrase: reasonPhrase,
      mimeType: mimeType,
      text: _string(resource['text']),
      encodingName: _string(resource['encoding']).trim(),
    );
  }

  Future<void> _writeFile(
    HttpResponse response,
    String filePath,
    String mimeType,
  ) async {
    final file = File(filePath);
    if (!await file.exists()) {
      response.statusCode = HttpStatus.notFound;
      await response.close();
      return;
    }
    _setContentType(response, mimeType);
    response.contentLength = await file.length();
    await response.addStream(file.openRead());
    await response.close();
  }

  Future<void> _writeText(
    HttpResponse response, {
    required int statusCode,
    required String reasonPhrase,
    required String mimeType,
    required String text,
    String encodingName = 'UTF-8',
  }) async {
    response.statusCode = statusCode;
    if (reasonPhrase.trim().isNotEmpty) {
      response.reasonPhrase = reasonPhrase.trim();
    }
    final encoding = Encoding.getByName(encodingName) ?? utf8;
    final normalizedMimeType = mimeType.trim().isEmpty
        ? 'text/plain'
        : mimeType.trim();
    response.headers.set(
      HttpHeaders.contentTypeHeader,
      '$normalizedMimeType; charset=${encoding.name}',
    );
    response.write(text);
    await response.close();
  }

  Future<void> _proxyOriginalRequest(
    HttpRequest request,
    String originalUrl,
  ) async {
    final client = HttpClient();
    try {
      final originalUri = Uri.parse(originalUrl);
      final proxyRequest = await client.openUrl(request.method, originalUri);
      final proxyResponse = await proxyRequest.close();
      request.response.statusCode = proxyResponse.statusCode;
      proxyResponse.headers.forEach((name, values) {
        for (final value in values) {
          request.response.headers.add(name, value);
        }
      });
      await request.response.addStream(proxyResponse);
      await request.response.close();
    } catch (error) {
      await _writeText(
        request.response,
        statusCode: HttpStatus.badGateway,
        reasonPhrase: 'Bad Gateway',
        mimeType: 'text/plain',
        text: error.toString(),
      );
    } finally {
      client.close(force: true);
    }
  }

  String _originalUrlForLocalUri(Uri localUri) {
    final origin = _origin;
    if (origin == null) {
      return localUri.toString();
    }
    final originUri = Uri.parse(origin);
    final queryParameters = <String, List<String>>{
      ...localUri.queryParametersAll,
    }..remove('__operit_main_frame');
    return Uri(
      scheme: originUri.scheme,
      host: originUri.host,
      port: originUri.hasPort ? originUri.port : null,
      path: localUri.path.isEmpty ? '/' : localUri.path,
      queryParameters: queryParameters.isEmpty ? null : queryParameters,
    ).toString();
  }

  static String _originOf(Uri uri) {
    return Uri(
      scheme: uri.scheme,
      host: uri.host,
      port: uri.hasPort ? uri.port : null,
    ).toString().replaceFirst(RegExp(r'/$'), '');
  }

  static Map<String, Object?> _objectMap(Object? raw) {
    if (raw is Map) {
      return raw.map((key, value) => MapEntry(key.toString(), value));
    }
    return const <String, Object?>{};
  }

  static Map<String, String> _stringMap(Object? raw) {
    if (raw is Map) {
      return raw.map((key, value) => MapEntry(key.toString(), _string(value)));
    }
    return const <String, String>{};
  }

  static Map<String, String> _headersMap(HttpHeaders headers) {
    final output = <String, String>{};
    headers.forEach((name, values) {
      output[name] = values.join(', ');
    });
    return output;
  }

  static String _string(Object? value) => value?.toString() ?? '';

  static int _int(Object? value, int defaultValue) {
    if (value is int) {
      return value;
    }
    if (value is num) {
      return value.toInt();
    }
    return int.tryParse(_string(value).trim()) ?? defaultValue;
  }

  static void _setContentType(HttpResponse response, String mimeType) {
    final normalized = mimeType.trim();
    if (normalized.isEmpty) {
      return;
    }
    response.headers.contentType = ContentType.parse(normalized);
  }
}
