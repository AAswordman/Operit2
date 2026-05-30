// ignore_for_file: file_names

import 'dart:convert';

import 'WorkspaceHtmlPreviewServer.dart';

class WorkspaceHtmlPreviewServerImpl implements WorkspaceHtmlPreviewServer {
  WorkspaceHtmlPreviewServerImpl({required this.onReadWorkspaceFileBytes});

  final WorkspaceHtmlFileReader onReadWorkspaceFileBytes;
  Uri? _uri;

  @override
  Future<Uri> start(String entryPath) async {
    final bytes = await onReadWorkspaceFileBytes(entryPath);
    final html = utf8.decode(bytes, allowMalformed: true);
    final expandedHtml = await _inlineHtmlResources(html, entryPath);
    _uri = Uri.dataFromBytes(utf8.encode(expandedHtml), mimeType: 'text/html');
    return _uri!;
  }

  @override
  Future<void> stop() async {
    _uri = null;
  }

  Future<String> _inlineHtmlResources(String html, String entryPath) async {
    final basePath = _parentPath(entryPath);
    var output = html;
    final attrPattern = RegExp(
      r'''(?<name>\b(?:src|href)\s*=\s*)(?<quote>["'])(?<value>[^"']+)(?<endQuote>["'])''',
      caseSensitive: false,
    );
    final matches = attrPattern.allMatches(html).toList().reversed;
    for (final match in matches) {
      final value = match.namedGroup('value')!;
      if (!_isWorkspaceLocalReference(value)) {
        continue;
      }
      final resourcePath = _resolveResourcePath(basePath, value);
      final bytes = await onReadWorkspaceFileBytes(resourcePath);
      final content = _dataUriForPath(resourcePath, bytes);
      output = output.replaceRange(
        match.start,
        match.end,
        '${match.namedGroup('name')}${match.namedGroup('quote')}$content${match.namedGroup('endQuote')}',
      );
    }
    return output;
  }
}

String _dataUriForPath(String path, List<int> bytes) {
  final mimeType = _mimeTypeForPath(path);
  if (mimeType == 'text/css') {
    final css = utf8.decode(bytes, allowMalformed: true);
    return Uri.dataFromString(css, mimeType: mimeType).toString();
  }
  return Uri.dataFromBytes(bytes, mimeType: mimeType).toString();
}

String _mimeTypeForPath(String path) {
  final extension = path.split('.').last.toLowerCase();
  switch (extension) {
    case 'html':
    case 'htm':
      return 'text/html';
    case 'css':
      return 'text/css';
    case 'js':
    case 'mjs':
    case 'cjs':
      return 'application/javascript';
    case 'json':
      return 'application/json';
    case 'svg':
      return 'image/svg+xml';
    case 'png':
      return 'image/png';
    case 'jpg':
    case 'jpeg':
      return 'image/jpeg';
    case 'gif':
      return 'image/gif';
    case 'webp':
      return 'image/webp';
    case 'mp3':
      return 'audio/mpeg';
    case 'wav':
      return 'audio/wav';
    case 'mp4':
    case 'm4v':
      return 'video/mp4';
    case 'webm':
      return 'video/webm';
  }
  return 'application/octet-stream';
}

bool _isWorkspaceLocalReference(String value) {
  final trimmed = value.trim();
  if (trimmed.isEmpty || trimmed.startsWith('#')) {
    return false;
  }
  final uri = Uri.tryParse(trimmed);
  return uri == null || !uri.hasScheme;
}

String _resolveResourcePath(String basePath, String value) {
  final cleanValue = value.split('#').first.split('?').first;
  if (cleanValue.startsWith('/')) {
    return cleanValue.replaceFirst(RegExp(r'^/+'), '');
  }
  final parts = <String>[...basePath.split('/'), ...cleanValue.split('/')];
  final resolved = <String>[];
  for (final part in parts) {
    if (part.isEmpty || part == '.') {
      continue;
    }
    if (part == '..') {
      if (resolved.isNotEmpty) {
        resolved.removeLast();
      }
    } else {
      resolved.add(part);
    }
  }
  return resolved.join('/');
}

String _parentPath(String path) {
  final normalized = path.replaceAll('\\', '/');
  final index = normalized.lastIndexOf('/');
  if (index <= 0) {
    return '';
  }
  return normalized.substring(0, index);
}
