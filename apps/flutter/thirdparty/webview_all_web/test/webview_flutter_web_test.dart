// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_web_plugins/flutter_web_plugins.dart';
import 'package:webview_all_web/webview_all_web.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

void main() {
  group('WebWebViewPlatform', () {
    test('registerWith', () {
      WebWebViewPlatform.registerWith(Registrar());
      expect(WebViewPlatform.instance, isA<WebWebViewPlatform>());
    });

    test('creates web platform implementation objects', () {
      final WebWebViewPlatform platform = WebWebViewPlatform();
      final PlatformWebViewController controller = platform
          .createPlatformWebViewController(
            const PlatformWebViewControllerCreationParams(),
          );

      expect(controller, isA<WebWebViewController>());
      expect(
        platform.createPlatformNavigationDelegate(
          const PlatformNavigationDelegateCreationParams(),
        ),
        isA<WebNavigationDelegate>(),
      );
      expect(
        platform.createPlatformWebViewWidget(
          PlatformWebViewWidgetCreationParams(controller: controller),
        ),
        isA<WebWebViewWidget>(),
      );
      expect(
        platform.createPlatformCookieManager(
          const PlatformWebViewCookieManagerCreationParams(),
        ),
        isA<WebWebViewCookieManager>(),
      );
    });
  });

  group('WebPlatformSslAuthError', () {
    test('documents unsupported recoverable certificate decisions', () async {
      final WebPlatformSslAuthError error = WebPlatformSslAuthError();

      expect(error.certificate, isNull);
      expect(
        error.description,
        'Recoverable SSL certificate decisions are not exposed by browser iframe APIs.',
      );
      await expectLater(error.proceed(), throwsUnsupportedError);
      await expectLater(error.cancel(), throwsUnsupportedError);
    });
  });
}
