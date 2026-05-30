// ignore_for_file: file_names

import 'WorkspaceUserscriptModels.dart';

class WorkspaceUserscriptMatcher {
  WorkspaceUserscriptMatcher._();

  static bool matches(
    WorkspaceUserscriptMetadata metadata,
    String pageUrl, {
    bool isTopFrame = true,
  }) {
    if (metadata.noFrames && !isTopFrame) {
      return false;
    }
    if (pageUrl.trim().isEmpty) {
      return false;
    }
    if (metadata.excludeMatches.any(
      (pattern) => _matchPattern(pattern, pageUrl),
    )) {
      return false;
    }
    if (metadata.excludes.any((pattern) => _globPattern(pattern, pageUrl))) {
      return false;
    }
    final hasPositiveRules =
        metadata.matches.isNotEmpty || metadata.includes.isNotEmpty;
    if (!hasPositiveRules) {
      return false;
    }
    if (metadata.matches.any((pattern) => _matchPattern(pattern, pageUrl))) {
      return true;
    }
    if (metadata.includes.any((pattern) => _globPattern(pattern, pageUrl))) {
      return true;
    }
    return false;
  }

  static bool isConnectAllowed(
    WorkspaceUserscriptMetadata metadata,
    String pageUrl,
    String targetUrl,
  ) {
    final pageUri = Uri.tryParse(pageUrl);
    final targetUri = Uri.tryParse(targetUrl);
    final pageOrigin = pageUri == null ? null : _originOf(pageUri);
    final targetOrigin = targetUri == null ? null : _originOf(targetUri);
    if (pageOrigin != null &&
        targetOrigin != null &&
        pageOrigin == targetOrigin) {
      return true;
    }
    if (metadata.connects.isEmpty || targetUri == null) {
      return false;
    }
    final targetHost = targetUri.host.toLowerCase();
    return metadata.connects.any((rule) {
      final normalized = rule.trim().toLowerCase();
      if (normalized == '*') {
        return true;
      }
      if (normalized == 'self') {
        return pageOrigin != null && pageOrigin == targetOrigin;
      }
      if (normalized == targetHost) {
        return true;
      }
      if (normalized.startsWith('*.')) {
        final suffix = normalized.substring(2);
        return targetHost == suffix || targetHost.endsWith('.$suffix');
      }
      return false;
    });
  }

  static bool _matchPattern(String pattern, String url) {
    final trimmed = pattern.trim();
    if (trimmed.isEmpty) {
      return false;
    }
    final uri = Uri.tryParse(url);
    if (uri == null) {
      return false;
    }
    final scheme = uri.scheme.toLowerCase();
    final host = uri.host.toLowerCase();
    final fullPath = StringBuffer(uri.path.isEmpty ? '/' : uri.path);
    if (uri.hasQuery) {
      fullPath.write('?${uri.query}');
    }
    if (uri.hasFragment) {
      fullPath.write('#${uri.fragment}');
    }

    final schemeIndex = trimmed.indexOf('://');
    if (schemeIndex < 0) {
      return false;
    }
    final schemePart = trimmed.substring(0, schemeIndex);
    final afterScheme = trimmed.substring(schemeIndex + 3);
    final slashIndex = afterScheme.indexOf('/');
    final hostPart = slashIndex < 0
        ? afterScheme
        : afterScheme.substring(0, slashIndex);
    final pathPart = slashIndex < 0
        ? ''
        : afterScheme.substring(slashIndex + 1);

    final schemeMatches = schemePart == '*'
        ? scheme == 'http' || scheme == 'https'
        : schemePart.toLowerCase() == scheme;
    if (!schemeMatches) {
      return false;
    }

    final normalizedHost = hostPart.toLowerCase();
    final hostMatches = normalizedHost == '*'
        ? host.isNotEmpty
        : normalizedHost.startsWith('*.')
        ? host == normalizedHost.substring(2) ||
              host.endsWith('.${normalizedHost.substring(2)}')
        : host == normalizedHost;
    if (!hostMatches) {
      return false;
    }

    return _globToRegex('/$pathPart').hasMatch(fullPath.toString());
  }

  static bool _globPattern(String pattern, String url) {
    final trimmed = pattern.trim();
    if (trimmed.isEmpty) {
      return false;
    }
    return _globToRegex(trimmed).hasMatch(url);
  }

  static RegExp _globToRegex(String pattern) {
    final buffer = StringBuffer('^');
    for (final unit in pattern.runes) {
      final char = String.fromCharCode(unit);
      if (char == '*') {
        buffer.write('.*');
      } else if (r'.?+()[]{}^$|\'.contains(char)) {
        buffer.write('\\$char');
      } else {
        buffer.write(char);
      }
    }
    buffer.write(r'$');
    return RegExp(buffer.toString(), caseSensitive: false);
  }

  static String? _originOf(Uri uri) {
    final scheme = uri.scheme.toLowerCase();
    final host = uri.host.toLowerCase();
    if (scheme.isEmpty || host.isEmpty) {
      return null;
    }
    final portPart =
        uri.hasPort &&
            !((scheme == 'http' && uri.port == 80) ||
                (scheme == 'https' && uri.port == 443))
        ? ':${uri.port}'
        : '';
    return '$scheme://$host$portPart';
  }
}
