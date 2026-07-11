// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';
import 'package:webview_all_web/webview_all_web.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('WebWebViewWidget', () {
    test('wraps generic widget creation params', () {
      final controller = WebWebViewController(
        WebWebViewControllerCreationParams(),
      );

      final widget = WebWebViewWidget(
        PlatformWebViewWidgetCreationParams(
          key: const Key('keyValue'),
          controller: controller,
        ),
      );

      expect(widget.params, isA<WebWebViewWidgetCreationParams>());
      expect(widget.params.key, const Key('keyValue'));
      expect(widget.params.controller, controller);
    });

    testWidgets('build returns a HtmlElementView', (WidgetTester tester) async {
      final controller = WebWebViewController(
        WebWebViewControllerCreationParams(),
      );

      final widget = WebWebViewWidget(
        PlatformWebViewWidgetCreationParams(
          key: const Key('keyValue'),
          controller: controller,
        ),
      );

      await tester.pumpWidget(
        Builder(builder: (BuildContext context) => widget.build(context)),
      );

      expect(find.byType(HtmlElementView), findsOneWidget);
      expect(find.byKey(const Key('keyValue')), findsOneWidget);
    });
  });
}
