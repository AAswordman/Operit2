import 'package:flutter/foundation.dart';
import 'package:web/web.dart' as web;
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

@immutable
class WebWebViewCookieManagerCreationParams
    extends PlatformWebViewCookieManagerCreationParams {
  const WebWebViewCookieManagerCreationParams();

  const WebWebViewCookieManagerCreationParams.fromPlatformWebViewCookieManagerCreationParams(
    PlatformWebViewCookieManagerCreationParams params,
  );
}

class WebWebViewCookieManager extends PlatformWebViewCookieManager {
  WebWebViewCookieManager(PlatformWebViewCookieManagerCreationParams params)
    : super.implementation(
        params is WebWebViewCookieManagerCreationParams
            ? params
            : WebWebViewCookieManagerCreationParams.fromPlatformWebViewCookieManagerCreationParams(
                params,
              ),
      );

  @override
  Future<bool> clearCookies() async {
    final String cookieString = web.document.cookie;
    if (cookieString.isEmpty) {
      return false;
    }

    final List<String> cookies = cookieString.split(';');
    for (final String cookie in cookies) {
      final String name = cookie.split('=').first.trim();
      if (name.isEmpty) {
        continue;
      }
      web.document.cookie =
          '$name=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/';
    }

    return cookies.any((String cookie) => cookie.trim().isNotEmpty);
  }

  @override
  Future<void> setCookie(WebViewCookie cookie) async {
    _validateCookieName(cookie.name);
    _validateCookieAttribute('domain', cookie.domain);
    _validateCookieAttribute('path', cookie.path);

    if (cookie.path.isNotEmpty && !cookie.path.startsWith('/')) {
      throw ArgumentError.value(
        cookie.path,
        'cookie.path',
        'Cookie path must start with "/".',
      );
    }

    final StringBuffer buffer = StringBuffer(
      '${cookie.name}=${Uri.encodeComponent(cookie.value)}',
    );
    if (cookie.domain.isNotEmpty) {
      buffer.write('; domain=${cookie.domain}');
    }
    if (cookie.path.isNotEmpty) {
      buffer.write('; path=${cookie.path}');
    } else {
      buffer.write('; path=/');
    }
    web.document.cookie = buffer.toString();
  }

  @override
  Future<List<WebViewCookie>> getCookies(Uri url) async {
    final String cookieString = web.document.cookie;
    if (cookieString.isEmpty) {
      return <WebViewCookie>[];
    }

    return cookieString
        .split(';')
        .map((String cookie) => cookie.trim())
        .where((String cookie) => cookie.isNotEmpty)
        .map((String cookie) {
          final int splitIndex = cookie.indexOf('=');
          if (splitIndex == -1) {
            return WebViewCookie(
              name: cookie,
              value: '',
              domain: url.host,
              path: '/',
            );
          }
          return WebViewCookie(
            name: cookie.substring(0, splitIndex),
            value: _decodeCookieValue(cookie.substring(splitIndex + 1)),
            domain: url.host,
            path: '/',
          );
        })
        .toList();
  }

  void _validateCookieName(String name) {
    if (name.isEmpty) {
      throw ArgumentError.value(name, 'cookie.name', 'Cookie name is empty.');
    }

    if (RegExp(r'[\x00-\x20\x7F()<>@,;:\\"/\[\]?={}]+').hasMatch(name)) {
      throw ArgumentError.value(
        name,
        'cookie.name',
        'Cookie name contains characters rejected by browsers.',
      );
    }
  }

  void _validateCookieAttribute(String field, String value) {
    if (value.isEmpty) {
      return;
    }

    if (RegExp(r'[\x00-\x1F\x7F;]').hasMatch(value)) {
      throw ArgumentError.value(
        value,
        'cookie.$field',
        'Cookie $field contains characters rejected by browsers.',
      );
    }
  }

  String _decodeCookieValue(String value) {
    try {
      return Uri.decodeComponent(value);
    } on FormatException {
      return value;
    }
  }
}
