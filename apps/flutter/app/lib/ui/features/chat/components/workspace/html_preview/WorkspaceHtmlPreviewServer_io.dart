// ignore_for_file: file_names

import 'dart:async';
import 'dart:io';

import 'WorkspaceHtmlPreviewServer.dart';

class WorkspaceHtmlPreviewServerImpl implements WorkspaceHtmlPreviewServer {
  WorkspaceHtmlPreviewServerImpl({required this.onReadWorkspaceFileBytes});

  final WorkspaceHtmlFileReader onReadWorkspaceFileBytes;
  HttpServer? _server;
  StreamSubscription<HttpRequest>? _subscription;

  @override
  Future<Uri> start(String entryPath) async {
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    _server = server;
    _subscription = server.listen(_handleRequest);
    return Uri(
      scheme: 'http',
      host: server.address.address,
      port: server.port,
      path: _encodeWorkspacePath(entryPath),
    );
  }

  @override
  Future<void> stop() async {
    await _subscription?.cancel();
    _subscription = null;
    await _server?.close(force: true);
    _server = null;
  }

  Future<void> _handleRequest(HttpRequest request) async {
    final relativePath = _decodeWorkspacePath(request.uri.path);
    try {
      final bytes = await onReadWorkspaceFileBytes(relativePath);
      request.response.headers.contentType = _contentTypeForPath(relativePath);
      request.response.headers.set('Access-Control-Allow-Origin', '*');
      request.response.add(bytes);
      await request.response.close();
    } on Object catch (error) {
      request.response.statusCode = HttpStatus.notFound;
      request.response.headers.contentType = ContentType.text;
      request.response.write(error.toString());
      await request.response.close();
    }
  }
}

String _encodeWorkspacePath(String path) {
  final segments = path
      .replaceAll('\\', '/')
      .split('/')
      .where((segment) => segment.isNotEmpty)
      .toList(growable: false);
  return Uri(pathSegments: segments).path;
}

String _decodeWorkspacePath(String path) {
  return Uri.decodeFull(path).replaceFirst(RegExp(r'^/+'), '');
}

ContentType _contentTypeForPath(String path) {
  final extension = path.split('.').last.toLowerCase();
  switch (extension) {
    case 'html':
    case 'htm':
      return ContentType.html;
    case 'css':
      return ContentType('text', 'css', charset: 'utf-8');
    case 'js':
    case 'mjs':
    case 'cjs':
      return ContentType('application', 'javascript', charset: 'utf-8');
    case 'json':
      return ContentType.json;
    case 'svg':
      return ContentType('image', 'svg+xml');
    case 'png':
      return ContentType('image', 'png');
    case 'jpg':
    case 'jpeg':
      return ContentType('image', 'jpeg');
    case 'gif':
      return ContentType('image', 'gif');
    case 'webp':
      return ContentType('image', 'webp');
    case 'mp3':
      return ContentType('audio', 'mpeg');
    case 'wav':
      return ContentType('audio', 'wav');
    case 'mp4':
    case 'm4v':
      return ContentType('video', 'mp4');
    case 'webm':
      return ContentType('video', 'webm');
    case 'txt':
    case 'md':
    case 'log':
      return ContentType.text;
  }
  return ContentType.binary;
}
