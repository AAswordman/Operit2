// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

import 'android_webview_controller.dart';
import 'android_webview_cookie_manager.dart';
import 'android_web_visit_host.dart';

/// Implementation of [WebViewPlatform] using the WebKit API.
class AndroidWebViewPlatform extends WebViewPlatform {
  /// Shares the process-owned visit bridge with execution and workspace clients.
  @override
  final PlatformWebVisitHost webVisitHost = AndroidWebVisitHost();

  /// Registers this class as the default instance of [WebViewPlatform].
  static void registerWith() {
    WebViewPlatform.instance = AndroidWebViewPlatform();
  }

  @override
  AndroidWebViewController createPlatformWebViewController(
    PlatformWebViewControllerCreationParams params,
  ) {
    return AndroidWebViewController(params);
  }

  @override
  AndroidNavigationDelegate createPlatformNavigationDelegate(
    PlatformNavigationDelegateCreationParams params,
  ) {
    return AndroidNavigationDelegate(params);
  }

  @override
  AndroidWebViewWidget createPlatformWebViewWidget(
    PlatformWebViewWidgetCreationParams params,
  ) {
    return AndroidWebViewWidget(params);
  }

  @override
  AndroidWebViewCookieManager createPlatformCookieManager(
    PlatformWebViewCookieManagerCreationParams params,
  ) {
    return AndroidWebViewCookieManager(params);
  }
}
