// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

import 'windows_webview_cookie.dart';
import 'windows_webview_native.dart' as native_webview;

/// Creation parameters for [WindowsWebViewCookieManager].
class WindowsWebViewCookieManagerCreationParams
    extends PlatformWebViewCookieManagerCreationParams {
  /// Creates a new [WindowsWebViewCookieManagerCreationParams].
  const WindowsWebViewCookieManagerCreationParams();

  /// Creates a [WindowsWebViewCookieManagerCreationParams] from generic params.
  const WindowsWebViewCookieManagerCreationParams.fromPlatformWebViewCookieManagerCreationParams(
    PlatformWebViewCookieManagerCreationParams params,
  );
}

/// Cookie manager implementation for Windows.
class WindowsWebViewCookieManager extends PlatformWebViewCookieManager {
  /// Creates a [WindowsWebViewCookieManager].
  WindowsWebViewCookieManager(PlatformWebViewCookieManagerCreationParams params)
    : super.implementation(
        params is WindowsWebViewCookieManagerCreationParams
            ? params
            : WindowsWebViewCookieManagerCreationParams.fromPlatformWebViewCookieManagerCreationParams(
                params,
              ),
      );

  @override
  Future<bool> clearCookies() async {
    final controller = native_webview.WebviewController();
    await controller.initialize();
    try {
      return await controller.clearCookiesWithResult();
    } finally {
      await controller.dispose();
    }
  }

  @override
  Future<void> setCookie(WebViewCookie cookie) async {
    _validateCookieFields(
      name: cookie.name,
      domain: cookie.domain,
      path: cookie.path,
    );

    await setWindowsCookie(
      WindowsWebViewCookie(
        name: cookie.name,
        value: cookie.value,
        domain: cookie.domain,
        path: cookie.path,
      ),
    );
  }

  @override
  Future<List<WebViewCookie>> getCookies(Uri url) async {
    final cookies = await getWindowsCookies(url);
    return cookies
        .map(
          (WindowsWebViewCookie cookie) => WebViewCookie(
            name: cookie.name,
            value: cookie.value,
            domain: cookie.domain,
            path: cookie.path,
          ),
        )
        .toList();
  }

  /// Sets a full Windows WebView2 cookie.
  Future<void> setWindowsCookie(WindowsWebViewCookie cookie) async {
    _validateCookieFields(
      name: cookie.name,
      domain: cookie.domain,
      path: cookie.path,
    );

    final controller = native_webview.WebviewController();
    await controller.initialize();
    try {
      await controller.setCookie(
        name: cookie.name,
        value: cookie.value,
        domain: cookie.domain,
        path: cookie.path,
        expires: cookie.expires,
        isHttpOnly: cookie.isHttpOnly,
        isSecure: cookie.isSecure,
        sameSite: cookie.sameSite?.index,
      );
    } finally {
      await controller.dispose();
    }
  }

  /// Returns full Windows WebView2 cookies visible for [url].
  Future<List<WindowsWebViewCookie>> getWindowsCookies(Uri url) async {
    final controller = native_webview.WebviewController();
    await controller.initialize();
    try {
      final cookies = await controller.getCookies(url.toString());
      return cookies;
    } finally {
      await controller.dispose();
    }
  }

  /// Deletes a cookie by its native WebView2 cookie identity.
  Future<void> deleteWindowsCookie(WindowsWebViewCookie cookie) async {
    final controller = native_webview.WebviewController();
    await controller.initialize();
    try {
      await controller.deleteCookie(cookie);
    } finally {
      await controller.dispose();
    }
  }

  /// Deletes cookies matching [name] and [url].
  Future<void> deleteCookiesWithNameAndUrl({
    required String name,
    required Uri url,
  }) async {
    final controller = native_webview.WebviewController();
    await controller.initialize();
    try {
      await controller.deleteCookiesWithNameAndUrl(name, url.toString());
    } finally {
      await controller.dispose();
    }
  }

  /// Deletes cookies matching [name], [domain], and [path].
  Future<void> deleteCookiesWithNameDomainAndPath({
    required String name,
    required String domain,
    required String path,
  }) async {
    final controller = native_webview.WebviewController();
    await controller.initialize();
    try {
      await controller.deleteCookiesWithNameDomainAndPath(name, domain, path);
    } finally {
      await controller.dispose();
    }
  }

  void _validateCookieFields({
    required String name,
    required String domain,
    required String path,
  }) {
    _validateCookieName(name);
    _validateCookieAttribute('domain', domain);
    _validateCookieAttribute('path', path);

    if (path.isNotEmpty && !path.startsWith('/')) {
      throw ArgumentError.value(
        path,
        'cookie.path',
        'Cookie path must start with "/".',
      );
    }
    if (!_isValidPath(path)) {
      throw ArgumentError(
        'The path property for the provided cookie was not given a legal value.',
      );
    }
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

  bool _isValidPath(String path) {
    // Permitted ranges based on RFC6265bis section 4.1.1.
    for (final char in path.codeUnits) {
      if ((char < 0x20 || char > 0x3A) && (char < 0x3C || char > 0x7E)) {
        return false;
      }
    }
    return true;
  }
}
