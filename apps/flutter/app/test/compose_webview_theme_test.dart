import 'package:operit2/core/compose/compose_program_buffer.dart' as compose_buffer;
import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_all_linux/webview_all_linux.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;

import 'support/compose_program_fixture.dart';

/// Verifies native preference, theme, and platform-view visibility behavior.
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

  testWidgets(
    'program WebView resolves navigation and interception callbacks after parameter updates',
    (tester) async {
      final platform = _ThemeTestPlatform();
      WebViewPlatform.instance = platform;
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
      final calls = <(String, Object?)>[];
      final delivered = <Object?>[];
      final options = <String, Object?>{'scope': 'workflow'};
      final host = ComposeDslWebViewHostContext(
        packageName: 'com.operit.workflow',
        routeInstanceId: 'workflow',
        executionContextKey: 'workflow-session',
        runtimeOptionsProvider: () => options,
        onActionResultDelivered: delivered.add,
        dispatchAction: (id, [payload]) async {
          calls.add((id, payload));
          switch (id) {
            case 'navigation:first':
            case 'navigation:second':
              return {'action': 'cancel'};
            case 'resource:first':
            case 'resource:second':
              return {
                'action': 'respond',
                'response': {
                  'statusCode': 200,
                  'mimeType': 'text/html',
                  'base64': base64Encode(utf8.encode('<html>$id</html>')),
                },
              };
            case 'controller:method':
              return {'value': 7};
            default:
              throw StateError('Unexpected source callback: $id');
          }
        },
      );

      /// Replaces source callbacks while retaining the same Flutter WebView occurrence.
      compose_buffer.ToolPkgComposeDslInstance source(String generation) =>
          composeTestInstance(
            id: 'web',
            nodeType: 'WebView',
            props: {
              'key': 'workflow-web',
              'url': 'https://workflow.operit.local/',
              'onShouldOverrideUrlLoading': {
                '__actionId': 'navigation:$generation',
              },
              'onInterceptRequest': {'__actionId': 'resource:$generation'},
            },
          );
      late void Function(core.ToolPkgComposeDslTransaction) apply;
      late VoidCallback dispose;
      final body = buildComposeProgramLayoutForTest(
        update: composeTestTransaction(
          reset: true,
          revision: 1,
          rootInstanceId: 'web',
          instances: [source('first')],
        ),
        hostContext: host,
        registerApply: (callback) => apply = callback,
        registerDispose: (callback) => dispose = callback,
      );
      await tester.pumpWidget(MaterialApp(home: Scaffold(body: body)));
      await tester.pumpAndSettle();
      final view = tester.widget<ComposeDslWebView>(
        find.byType(ComposeDslWebView),
      );
      final occurrence = tester.element(find.byType(ComposeDslWebView));
      final navigationId =
          (view.props['onShouldOverrideUrlLoading'] as Map)['__actionId']
              as String;
      final interceptionId =
          (view.props['onInterceptRequest'] as Map)['__actionId'] as String;
      expect(navigationId, startsWith('@program:'));
      expect(interceptionId, startsWith('@program:'));
      final mappedUrl = platform.controller.events
          .singleWhere((value) => value.startsWith('load:'))
          .substring(5);
      final navigation = platform.navigation.onNavigationRequest;
      final resource = platform.controller.resourceHandler!;
      for (final generation in ['first', 'second']) {
        if (generation == 'second') {
          apply(
            composeTestTransaction(
              reset: false,
              revision: 2,
              rootInstanceId: 'web',
              instances: [source(generation)],
            ),
          );
          await tester.pumpAndSettle();
          expect(
            tester.element(find.byType(ComposeDslWebView)),
            same(occurrence),
          );
        }
        expect(
          await navigation(
            NavigationRequest(url: mappedUrl, isMainFrame: true),
          ),
          NavigationDecision.prevent,
        );
        expect(calls.last.$1, 'navigation:$generation');
        expect((calls.last.$2 as Map)['url'], 'https://workflow.operit.local/');
        final response = await resource(
          WebViewLocalResourceRequest(url: mappedUrl, isMainFrame: true),
        );
        expect(response.statusCode, 200);
        expect(response.mimeType, 'text/html');
        expect(utf8.decode(response.body), '<html>resource:$generation</html>');
        expect(calls.last.$1, 'resource:$generation');
        expect((calls.last.$2 as Map)['url'], 'https://workflow.operit.local/');
      }
      final boundHost = tester
          .widget<ComposeDslWebView>(find.byType(ComposeDslWebView))
          .hostContext!;
      final queuedNavigation = await view.hostContext!.executeAction(
        actionId: navigationId,
      );
      expect(queuedNavigation.message, isNull);
      expect(queuedNavigation.actionResult, {'action': 'cancel'});
      expect(calls.last.$1, 'navigation:second');
      final queuedInterception = await view.hostContext!.executeAction(
        actionId: interceptionId,
      );
      expect(queuedInterception.message, isNull);
      expect(calls.last.$1, 'resource:second');
      expect(boundHost.packageName, host.packageName);
      expect(boundHost.routeInstanceId, host.routeInstanceId);
      expect(boundHost.executionContextKey, host.executionContextKey);
      expect(boundHost.clients, same(host.clients));
      expect(boundHost.runtimeOptionsProvider(), same(options));
      expect(
        boundHost.onActionResultDelivered,
        same(host.onActionResultDelivered),
      );
      expect(
        (await boundHost.executeAction(
          actionId: 'controller:method',
        )).actionResult,
        {'value': 7},
      );
      expect(calls.every((call) => !call.$1.startsWith('@program:')), isTrue);
      expect(
        platform.controller.events.where((value) => value.startsWith('load:')),
        hasLength(1),
      );
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      dispose();
    },
  );

  // Dialog routes must preserve the platform view and its loaded page on every host.
  testWidgets(
    'keeps the WebView mounted and visible beneath dialog routes',
    (tester) async {
      final platform = _ThemeTestPlatform();
      WebViewPlatform.instance = platform;
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

      final webView = find.byType(WebViewWidget);
      expect(webView, findsOneWidget);
      final element = tester.element(webView);
      final events = List<String>.of(platform.controller.events);
      final context = tester.element(find.byType(ComposeDslWebView));
      unawaited(
        showDialog<void>(
          context: context,
          builder: (_) => const AlertDialog(title: Text('dialog')),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('dialog'), findsOneWidget);
      expect(webView, findsOneWidget);
      expect(tester.element(webView), same(element));
      expect(platform.controller.events, events);

      Navigator.of(context).pop();
      await tester.pumpAndSettle();
      expect(webView, findsOneWidget);
      expect(tester.element(webView), same(element));
      expect(platform.controller.events, events);

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    },
    variant: TargetPlatformVariant(TargetPlatform.values.toSet()),
  );
}

class _ThemeTestPlatform extends WebViewPlatform {
  final controller = _ThemeTestController();
  late _ThemeTestDelegate navigation;

  /// Returns one stable browser for the lifecycle test.
  @override
  PlatformWebViewController createPlatformWebViewController(
    PlatformWebViewControllerCreationParams params,
  ) => controller;

  /// Creates a no-op delegate because this test does not emulate page events.
  @override
  PlatformNavigationDelegate createPlatformNavigationDelegate(
    PlatformNavigationDelegateCreationParams params,
  ) => navigation = _ThemeTestDelegate(params);

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
  WebViewLocalResourceHandler? resourceHandler;

  /// Exposes the shared resource-handler contract without creating a native browser.
  @override
  String get localResourceScheme => 'https';

  /// Retains the production resource callback for request interception tests.
  @override
  Future<void> setLocalResourceHandler(
    WebViewLocalResourceHandler? handler,
  ) async {
    resourceHandler = handler;
  }

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
  late NavigationRequestCallback onNavigationRequest;

  /// Accepts navigation request callbacks.
  @override
  Future<void> setOnNavigationRequest(
    NavigationRequestCallback callback,
  ) async {
    onNavigationRequest = callback;
  }

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
