import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:webview_all_linux/webview_all_linux.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Verifies native preference delivery and browser-preserving theme updates.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('Linux registers into the shared browser interface', () {
    LinuxWebViewPlatform.registerWith();
    expect(WebViewPlatform.instance, isA<LinuxWebViewPlatform>());
  });

  testWidgets(
    'applies theme before navigation and changes it without reloading',
    (tester) async {
      final platform = _ThemeTestPlatform();
      WebViewPlatform.instance = platform;
      final events = <String>[];
      final ready = Completer<void>();
      platform.controller.events = events;
      const channel = MethodChannel('operit/webview_theme');
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, (
        call,
      ) async {
        expect(call.method, 'setPreferredColorScheme');
        events.add('theme:${call.arguments}');
        await ready.future;
        return null;
      });
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          channel,
          null,
        ),
      );

      /// Rebuilds the inherited app theme while retaining the same browser element.
      Widget screen(Brightness brightness) => MaterialApp(
        theme: ThemeData(brightness: brightness),
        home: ComposeDslWebView(
          props: const {'url': 'https://example.com'},
          onAction: (id, [payload]) async => null,
          hostContext: null,
        ),
      );

      await tester.pumpWidget(screen(Brightness.dark));
      await tester.pump();
      expect(events, ['theme:dark']);
      ready.complete();
      await tester.pumpAndSettle();
      expect(events, [
        'theme:dark',
        'document-start',
        'load:https://example.com',
      ]);
      await tester.pumpWidget(screen(Brightness.light));
      await tester.pumpAndSettle();
      expect(events, [
        'theme:dark',
        'document-start',
        'load:https://example.com',
        'theme:light',
      ]);
      await tester.pumpWidget(screen(Brightness.light));
      await tester.pumpAndSettle();
      expect(events.length, 4);
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      expect(events.last, 'remove-document-start');
    },
  );
  testWidgets('navigation awaits channel and native script registration', (
    tester,
  ) async {
    final platform = _ThemeTestPlatform();
    WebViewPlatform.instance = platform;
    final channelReady = Completer<void>();
    final scriptReady = Completer<void>();
    platform.controller.channelRegistration = channelReady.future;
    platform.controller.scriptRegistration = scriptReady.future;
    const themeChannel = MethodChannel('operit/webview_theme');
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      themeChannel,
      (_) async => null,
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        themeChannel,
        null,
      ),
    );
    await tester.pumpWidget(
      MaterialApp(
        home: ComposeDslWebView(
          props: const {'url': 'https://example.com'},
          onAction: (id, [payload]) async => null,
          hostContext: null,
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(platform.controller.events, isEmpty);
    channelReady.complete();
    await tester.pumpAndSettle();
    expect(platform.controller.events, ['document-start']);
    scriptReady.complete();
    await tester.pumpAndSettle();
    expect(platform.controller.events, [
      'document-start',
      'load:https://example.com',
    ]);
    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
    expect(platform.controller.events.last, 'remove-document-start');
  });
  testWidgets('DSL action errors do not dispose or reload the mounted WebView', (
    tester,
  ) async {
    final platform = _ThemeTestPlatform();
    WebViewPlatform.instance = platform;
    final actions = <Object?>[];
    const channel = MethodChannel('operit/webview_theme');
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      channel,
      (_) async => null,
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        channel,
        null,
      ),
    );
    final host = ComposeDslWebViewHostContext(
      packageName: 'regression',
      routeInstanceId: 'page',
      executionContextKey: 'webview-error',
      dispatchAction: (id, [payload]) async {
        actions.add(payload);
        return null;
      },
      runtimeOptionsProvider: () => {},
    );
    const tree = <String, Object?>{
      'type': 'WebView',
      'props': {
        'url': 'https://example.com',
        'onLifecycleEvent': {'__actionId': 'lifecycle'},
      },
      'children': [],
    };

    /// Drives actual error presentation without replacing the production WebView widget.
    Future<void> show(String? error) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: buildComposeDslHostForTest(
              node: tree,
              hostContext: host,
              error: error,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
    }

    await show(null);
    final browser = tester.element(find.byType(ComposeDslWebView));
    actions.clear();
    for (var index = 0; index < 3; index++) {
      await show('compose action not found: __action_3');
      expect(find.text('compose action not found: __action_3'), findsOneWidget);
      expect(tester.element(find.byType(ComposeDslWebView)), same(browser));
      await show(null);
      expect(tester.element(find.byType(ComposeDslWebView)), same(browser));
    }
    expect(actions, isEmpty);
    expect(
      platform.controller.events
          .where((event) => event == 'load:https://example.com')
          .length,
      1,
    );
    expect(
      platform.controller.events,
      isNot(contains('remove-document-start')),
    );
    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
    expect(platform.controller.events.last, 'remove-document-start');
    expect(actions, isNotEmpty);
    expect(tester.takeException(), isNull);
  });
}

class _ThemeTestPlatform extends WebViewPlatform {
  final controller = _ThemeTestController();

  /// Returns one stable browser for the lifecycle test.
  @override
  PlatformWebViewController createPlatformWebViewController(
    PlatformWebViewControllerCreationParams params,
  ) => controller;

  /// Creates a no-op delegate because this test does not emulate page events.
  @override
  PlatformNavigationDelegate createPlatformNavigationDelegate(
    PlatformNavigationDelegateCreationParams params,
  ) => _ThemeTestDelegate(params);

  /// Renders an inert view while exercising the production browser controller.
  @override
  PlatformWebViewWidget createPlatformWebViewWidget(
    PlatformWebViewWidgetCreationParams params,
  ) => _ThemeTestWidget(params);
}

class _ThemeTestWidget extends PlatformWebViewWidget {
  /// Creates a native-view placeholder.
  _ThemeTestWidget(super.params) : super.implementation();

  /// Avoids native view composition during widget tests.
  @override
  Widget build(BuildContext context) => const SizedBox.expand();
}

class _ThemeTestController extends PlatformWebViewController {
  /// Creates a controller that uses the real native theme channel contract.
  _ThemeTestController()
    : super.implementation(const PlatformWebViewControllerCreationParams());
  List<String> events = [];
  Future<void> channelRegistration = Future<void>.value();
  Future<void> scriptRegistration = Future<void>.value();

  /// Records navigation for detecting unexpected page reloads.
  @override
  Future<void> loadRequest(LoadRequestParams params) async {
    events.add('load:${params.uri}');
  }

  /// Accepts the blank page used during disposal.
  @override
  Future<void> loadHtmlString(String html, {String? baseUrl}) async {}

  /// Emulates native registration before the first page is permitted to load.
  @override
  Future<String> addUserScript(WebViewUserScript userScript) async {
    expect(
      userScript.injectionTime,
      WebViewUserScriptInjectionTime.documentStart,
    );
    expect(userScript.forMainFrameOnly, false);
    expect(userScript.source, contains('var initialInterfaces'));
    events.add('document-start');
    await scriptRegistration;
    return 'test-document-start-script';
  }

  /// Verifies ownership-based cleanup of the native document-start handle.
  @override
  Future<void> removeUserScript(String identifier) async {
    expect(identifier, 'test-document-start-script');
    events.add('remove-document-start');
  }

  /// Accepts page bridge injection.
  @override
  Future<void> runJavaScript(String javaScript) async {}

  /// Accepts the page's JavaScript policy.
  @override
  Future<void> setJavaScriptMode(JavaScriptMode mode) async {}

  /// Accepts the page's background color.
  @override
  Future<void> setBackgroundColor(Color color) async {}

  /// Accepts the page's navigation delegate.
  @override
  Future<void> setPlatformNavigationDelegate(
    PlatformNavigationDelegate handler,
  ) async {}

  /// Accepts the page bridge channel.
  @override
  Future<void> addJavaScriptChannel(JavaScriptChannelParams params) =>
      channelRegistration;

  /// Accepts console observation.
  @override
  Future<void> setOnConsoleMessage(
    void Function(JavaScriptConsoleMessage) callback,
  ) async {}

  /// Accepts permission observation.
  @override
  Future<void> setOnPlatformPermissionRequest(
    void Function(PlatformWebViewPermissionRequest) callback,
  ) async {}

  /// Accepts native alert handling.
  @override
  Future<void> setOnJavaScriptAlertDialog(
    Future<void> Function(JavaScriptAlertDialogRequest) callback,
  ) async {}

  /// Accepts native confirmation handling.
  @override
  Future<void> setOnJavaScriptConfirmDialog(
    Future<bool> Function(JavaScriptConfirmDialogRequest) callback,
  ) async {}

  /// Accepts native text dialog handling.
  @override
  Future<void> setOnJavaScriptTextInputDialog(
    Future<String> Function(JavaScriptTextInputDialogRequest) callback,
  ) async {}

  /// Accepts zoom configuration.
  @override
  Future<void> enableZoom(bool enabled) async {}

  /// Accepts vertical scrollbar configuration.
  @override
  Future<void> setVerticalScrollBarEnabled(bool enabled) async {}

  /// Accepts horizontal scrollbar configuration.
  @override
  Future<void> setHorizontalScrollBarEnabled(bool enabled) async {}
}

class _ThemeTestDelegate extends PlatformNavigationDelegate {
  /// Creates a delegate with inert callbacks.
  _ThemeTestDelegate(super.params) : super.implementation();

  /// Accepts navigation request callbacks.
  @override
  Future<void> setOnNavigationRequest(
    NavigationRequestCallback callback,
  ) async {}

  /// Accepts page start callbacks.
  @override
  Future<void> setOnPageStarted(PageEventCallback callback) async {}

  /// Accepts page completion callbacks.
  @override
  Future<void> setOnPageFinished(PageEventCallback callback) async {}

  /// Accepts loading progress callbacks.
  @override
  Future<void> setOnProgress(ProgressCallback callback) async {}

  /// Accepts resource failure callbacks.
  @override
  Future<void> setOnWebResourceError(WebResourceErrorCallback callback) async {}

  /// Accepts URL change callbacks.
  @override
  Future<void> setOnUrlChange(UrlChangeCallback callback) async {}

  /// Accepts HTTP authentication callbacks.
  @override
  Future<void> setOnHttpAuthRequest(HttpAuthRequestCallback callback) async {}

  /// Accepts HTTP error callbacks.
  @override
  Future<void> setOnHttpError(HttpResponseErrorCallback callback) async {}

  /// Accepts certificate error callbacks.
  @override
  Future<void> setOnSSlAuthError(SslAuthErrorCallback callback) async {}
}
