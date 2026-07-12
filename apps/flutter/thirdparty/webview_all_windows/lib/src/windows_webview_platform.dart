// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

import 'windows_webview_controller.dart';
import 'windows_webview_cookie_manager.dart';

/// Implementation of [WebViewPlatform] using WebView2 on Windows.
class WindowsWebViewPlatform extends WebViewPlatform {
  /// Registers this class as the default instance of [WebViewPlatform].
  static void registerWith() {
    WebViewPlatform.instance = WindowsWebViewPlatform();
  }

  @override
  WindowsWebViewController createPlatformWebViewController(
    PlatformWebViewControllerCreationParams params,
  ) {
    return WindowsWebViewController(params);
  }

  @override
  WindowsNavigationDelegate createPlatformNavigationDelegate(
    PlatformNavigationDelegateCreationParams params,
  ) {
    return WindowsNavigationDelegate(params);
  }

  @override
  WindowsWebViewWidget createPlatformWebViewWidget(
    PlatformWebViewWidgetCreationParams params,
  ) {
    return WindowsWebViewWidget(params);
  }

  @override
  WindowsWebViewCookieManager createPlatformCookieManager(
    PlatformWebViewCookieManagerCreationParams params,
  ) {
    return WindowsWebViewCookieManager(params);
  }
}
