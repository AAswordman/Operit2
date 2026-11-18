import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/markdown/EnhancedCodeBlock.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

/// Verifies previews opened from code blocks with unbounded list height.
void main() {
  for (final (language, tooltip, code, mode) in [
    ('html', 'HTML', '<h1>Preview</h1>', JavaScriptMode.disabled),
    ('htm', 'HTML', '<p>Preview</p>', JavaScriptMode.disabled),
    ('mermaid', 'Mermaid', 'graph TD; A-->B;', JavaScriptMode.unrestricted),
  ]) {
    testWidgets('$language fullscreen preview owns a bounded dialog route', (
      tester,
    ) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(800, 600);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final previousPlatform = WebViewPlatform.instance;
      final platform = _PreviewPlatform();
      WebViewPlatform.instance = platform;
      if (previousPlatform != null) {
        addTearDown(() => WebViewPlatform.instance = previousPlatform);
      }

      final chatNavigator = GlobalKey<NavigatorState>();
      await tester.pumpWidget(
        MaterialApp(
          home: Navigator(
            key: chatNavigator,
            onGenerateRoute: (_) => MaterialPageRoute<void>(
              builder: (_) => Scaffold(
                body: Align(
                  alignment: Alignment.topLeft,
                  child: SizedBox(
                    width: 345,
                    child: ListView(
                      children: [
                        EnhancedCodeBlock(code: code, language: language),
                        const SizedBox(height: 1000),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.byTooltip(tooltip));
      await tester.pumpAndSettle();
      final inlineBlock = find.byType(EnhancedCodeBlock);
      final inlineSize = tester.getSize(inlineBlock);
      final inlineController = tester
          .widget<WebViewWidget>(find.byType(WebViewWidget))
          .platform
          .params
          .controller;

      await tester.tap(find.byTooltip('Fullscreen'));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(chatNavigator.currentState!.canPop(), isFalse);
      final dialog = find.byType(Dialog);
      expect(tester.getRect(dialog), Offset.zero & const Size(800, 600));
      final preview = find.descendant(
        of: dialog,
        matching: find.byType(WebViewWidget),
      );
      expect(tester.getSize(preview), const Size(800, 600));
      expect(tester.getSize(inlineBlock), inlineSize);
      final controller =
          tester.widget<WebViewWidget>(preview).platform.params.controller
              as _PreviewController;
      expect(controller.html, platform.controllers.first.html);
      expect(controller.mode, mode);

      for (final size in [const Size(390, 844), const Size(1000, 700)]) {
        tester.view.physicalSize = size;
        await tester.pumpAndSettle();
        expect(tester.getRect(dialog), Offset.zero & size);
        expect(tester.getSize(preview), size);
        expect(tester.takeException(), isNull);
      }

      await tester.tap(find.byTooltip('Close'));
      await tester.pumpAndSettle();
      expect(dialog, findsNothing);
      expect(tester.getSize(inlineBlock), inlineSize);
      expect(
        tester
            .widget<WebViewWidget>(find.byType(WebViewWidget))
            .platform
            .params
            .controller,
        same(inlineController),
      );

      await tester.tap(find.byTooltip('Fullscreen'));
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pumpAndSettle();
      expect(dialog, findsNothing);

      await tester.tap(find.byTooltip('Fullscreen'));
      await tester.pumpAndSettle();
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(dialog, findsNothing);
      expect(inlineBlock, findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  }
}

class _PreviewPlatform extends WebViewPlatform {
  final controllers = <_PreviewController>[];

  /// Creates a separate controller for each inline or fullscreen preview.
  @override
  PlatformWebViewController createPlatformWebViewController(
    PlatformWebViewControllerCreationParams params,
  ) {
    final controller = _PreviewController(params);
    controllers.add(controller);
    return controller;
  }

  /// Exercises Flutter layout without a native browser process.
  @override
  PlatformWebViewWidget createPlatformWebViewWidget(
    PlatformWebViewWidgetCreationParams params,
  ) => _PreviewWidget(params);
}

class _PreviewWidget extends PlatformWebViewWidget {
  /// Creates a surface using the production WebView widget parameters.
  _PreviewWidget(super.params) : super.implementation();

  /// Fills the supplied preview viewport to expose unbounded constraints.
  @override
  Widget build(BuildContext context) => const SizedBox.expand();
}

class _PreviewController extends PlatformWebViewController {
  /// Creates a controller that records the rendered document.
  _PreviewController(super.params) : super.implementation();

  String? html;
  JavaScriptMode? mode;

  /// Records source loaded into either preview surface.
  @override
  Future<void> loadHtmlString(String html, {String? baseUrl}) async {
    this.html = html;
  }

  /// Records the script policy selected by the renderer.
  @override
  Future<void> setJavaScriptMode(JavaScriptMode mode) async {
    this.mode = mode;
  }

  /// Accepts background styling without invoking a platform channel.
  @override
  Future<void> setBackgroundColor(Color color) async {}
}
