import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

/// Verifies that surface ownership automatically releases platform browsers.
void main() {
  setUp(() => WebViewPlatform.instance = _Platform());

  for (final constructor in ['controller', 'params', 'platform']) {
    testWidgets('$constructor releases the browser after unmount', (
      tester,
    ) async {
      final controller = _Controller();
      final params = PlatformWebViewWidgetCreationParams(
        controller: controller,
      );
      final view = switch (constructor) {
        'controller' => WebViewWidget(
          controller: WebViewController.fromPlatform(controller),
        ),
        'params' => WebViewWidget.fromPlatformCreationParams(params: params),
        _ => WebViewWidget.fromPlatform(platform: _View(params)),
      };
      await tester.pumpWidget(view);
      await tester.pumpWidget(const SizedBox());
      await tester.pump();
      expect(controller.disposals, 1);
    });
  }

  testWidgets('rebuilds and hidden tabs retain their browser', (tester) async {
    final controller = _Controller();
    for (final hidden in [false, true, false, true]) {
      await tester.pumpWidget(
        Offstage(
          offstage: hidden,
          child: WebViewWidget.fromPlatform(
            platform: _View(
              PlatformWebViewWidgetCreationParams(controller: controller),
            ),
          ),
        ),
      );
      expect(controller.disposals, 0);
    }
    await tester.pumpWidget(const SizedBox());
    await tester.pump();
    expect(controller.disposals, 1);
  });

  testWidgets('replacing a controller releases only the old browser', (
    tester,
  ) async {
    final first = _Controller();
    final second = _Controller();
    for (final controller in [first, second]) {
      await tester.pumpWidget(
        WebViewWidget.fromPlatform(
          platform: _View(
            PlatformWebViewWidgetCreationParams(controller: controller),
          ),
        ),
      );
    }
    await tester.pump();
    expect(first.disposals, 1);
    expect(second.disposals, 0);
    await tester.pumpWidget(const SizedBox());
    await tester.pump();
    expect(second.disposals, 1);
  });

  testWidgets('same-frame surface transfers keep the browser alive', (
    tester,
  ) async {
    final controller = _Controller();
    for (final key in ['first', 'second']) {
      await tester.pumpWidget(
        WebViewWidget.fromPlatform(
          key: ValueKey(key),
          platform: _View(
            PlatformWebViewWidgetCreationParams(controller: controller),
          ),
        ),
      );
      expect(controller.disposals, 0);
    }
    await tester.pumpWidget(const SizedBox());
    await tester.pump();
    expect(controller.disposals, 1);
  });

  testWidgets('page resources finish cleanup before browser destruction', (
    tester,
  ) async {
    final cleanup = Completer<void>();
    final controller = _Controller();
    await tester.pumpWidget(
      WebViewWidget.fromPlatform(
        platform: _View(
          PlatformWebViewWidgetCreationParams(controller: controller),
        ),
        onBeforeDispose: () => cleanup.future,
      ),
    );
    await tester.pumpWidget(const SizedBox());
    expect(controller.disposals, 0);
    cleanup.complete();
    await tester.pump();
    expect(controller.disposals, 1);
  });

  testWidgets('cleanup errors still close the browser and are reported', (
    tester,
  ) async {
    final controller = _Controller();
    await tester.pumpWidget(
      WebViewWidget.fromPlatform(
        platform: _View(
          PlatformWebViewWidgetCreationParams(controller: controller),
        ),
        onBeforeDispose: () async => throw StateError('cleanup failed'),
      ),
    );
    await tester.pumpWidget(const SizedBox());
    await tester.pump();
    expect(controller.disposals, 1);
    expect(tester.takeException(), isStateError);
  });
}

class _Platform extends WebViewPlatform {
  /// Builds an inert surface through the shared creation parameters.
  @override
  PlatformWebViewWidget createPlatformWebViewWidget(
    PlatformWebViewWidgetCreationParams params,
  ) => _View(params);
}

class _View extends PlatformWebViewWidget {
  /// Creates a surface without loading a native browser.
  _View(super.params) : super.implementation();

  /// Fills the surface without affecting lifecycle behavior.
  @override
  Widget build(BuildContext context) => const SizedBox();
}

class _Controller extends PlatformWebViewController {
  /// Creates a platform controller that records native resource release.
  _Controller()
    : super.implementation(const PlatformWebViewControllerCreationParams());

  int disposals = 0;

  /// Counts releases to expose missing or duplicated destruction.
  @override
  Future<void> dispose() async => disposals++;
}
