import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:webview_all_linux/src/linux_webview_constants.dart';
import 'package:webview_all_linux/webview_all_linux.dart';
import 'package:webview_platform_interface/webview_platform_interface.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  setUp(_mockLinuxWebViewCreation);
  tearDown(_clearLinuxWebViewCreationMock);

  test('registerWith sets the Linux WebView platform implementation', () {
    final WebViewPlatform? previousInstance = WebViewPlatform.instance;
    addTearDown(() {
      if (previousInstance != null) {
        WebViewPlatform.instance = previousInstance;
      }
    });

    LinuxWebViewPlatform.registerWith();

    expect(WebViewPlatform.instance, isA<LinuxWebViewPlatform>());
  });

  test('creates Linux platform implementation objects', () {
    final LinuxWebViewPlatform platform = LinuxWebViewPlatform();
    final LinuxWebViewController controller = platform
        .createPlatformWebViewController(
          const PlatformWebViewControllerCreationParams(),
        );

    expect(controller, isA<LinuxWebViewController>());
    expect(
      platform.createPlatformNavigationDelegate(
        const PlatformNavigationDelegateCreationParams(),
      ),
      isA<LinuxNavigationDelegate>(),
    );
    expect(
      platform.createPlatformWebViewWidget(
        PlatformWebViewWidgetCreationParams(controller: controller),
      ),
      isA<LinuxWebViewWidget>(),
    );
    expect(
      platform.createPlatformCookieManager(
        const PlatformWebViewCookieManagerCreationParams(),
      ),
      isA<LinuxWebViewCookieManager>(),
    );
    expect(
      platform.createPlatformWebViewDataManager(
        const PlatformWebViewDataManagerCreationParams(),
      ),
      isA<LinuxWebViewDataManager>(),
    );
  });

  testWidgets('moves the native view when the controller changes', (
    WidgetTester tester,
  ) async {
    final Map<int, List<MethodCall>> calls = <int, List<MethodCall>>{
      101: <MethodCall>[],
      102: <MethodCall>[],
    };
    _mockLinuxWebViewCreation(
      instanceIds: <int>[101, 102],
      onInstanceCallWithId: (int id, MethodCall call) {
        calls[id]!.add(call);
      },
    );
    final LinuxWebViewController firstController = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final LinuxWebViewController secondController = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    Widget buildWebView(LinuxWebViewController controller) {
      final LinuxWebViewWidget platformWidget = LinuxWebViewWidget(
        PlatformWebViewWidgetCreationParams(
          key: const ValueKey<String>('linux-webview'),
          controller: controller,
        ),
      );
      return Directionality(
        textDirection: TextDirection.ltr,
        child: SizedBox(
          width: 320,
          height: 180,
          child: Builder(builder: platformWidget.build),
        ),
      );
    }

    await tester.pumpWidget(buildWebView(firstController));
    await tester.pump();
    await tester.pump();

    expect(
      calls[101]!.where((MethodCall call) => call.method == 'setFrame'),
      contains(
        isA<MethodCall>().having(
          (MethodCall call) =>
              (call.arguments as Map<Object?, Object?>)['visible'],
          'visible',
          isTrue,
        ),
      ),
    );

    await tester.pumpWidget(buildWebView(secondController));
    await tester.pump();
    await tester.pump();

    final MethodCall oldControllerFrame = calls[101]!.lastWhere(
      (MethodCall call) => call.method == 'setFrame',
    );
    final MethodCall newControllerFrame = calls[102]!.lastWhere(
      (MethodCall call) => call.method == 'setFrame',
    );
    expect(
      (oldControllerFrame.arguments as Map<Object?, Object?>)['visible'],
      isFalse,
    );
    expect(
      (newControllerFrame.arguments as Map<Object?, Object?>)['visible'],
      isTrue,
    );

    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump();

    expect(
      (calls[102]!
              .lastWhere((MethodCall call) => call.method == 'setFrame')
              .arguments
          as Map<Object?, Object?>)['visible'],
      isFalse,
    );
  });

  test('sends monotonically increasing native frame sequences', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    await controller.setFrame(
      const Rect.fromLTWH(10, 20, 300, 180),
      clipRect: const Rect.fromLTWH(10, 40, 300, 160),
      visible: true,
    );
    await controller.setFrame(Rect.zero, visible: false);

    final List<MethodCall> frameCalls = calls
        .where((MethodCall call) => call.method == 'setFrame')
        .toList();
    expect(frameCalls, hasLength(2));
    expect((frameCalls[0].arguments as Map<Object?, Object?>)['sequence'], 1);
    expect((frameCalls[1].arguments as Map<Object?, Object?>)['sequence'], 2);
    expect(
      (frameCalls[0].arguments as Map<Object?, Object?>),
      containsPair('clipX', 10),
    );
    expect(
      (frameCalls[0].arguments as Map<Object?, Object?>),
      containsPair('clipY', 40),
    );
    expect(
      (frameCalls[0].arguments as Map<Object?, Object?>),
      containsPair('clipWidth', 300),
    );
    expect(
      (frameCalls[0].arguments as Map<Object?, Object?>),
      containsPair('clipHeight', 160),
    );
  });

  testWidgets('keeps full native geometry at the Flutter viewport edge', (
    WidgetTester tester,
  ) async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final LinuxWebViewWidget platformWidget = LinuxWebViewWidget(
      PlatformWebViewWidgetCreationParams(controller: controller),
    );

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: Transform.translate(
            offset: const Offset(-40, -20),
            child: SizedBox(
              width: 100,
              height: 80,
              child: Builder(builder: platformWidget.build),
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    final MethodCall frameCall = calls.lastWhere(
      (MethodCall call) =>
          call.method == 'setFrame' &&
          (call.arguments as Map<Object?, Object?>)['visible'] == true,
    );
    final Map<Object?, Object?> arguments =
        frameCall.arguments as Map<Object?, Object?>;
    expect(arguments['x'], -40);
    expect(arguments['y'], -20);
    expect(arguments['width'], 100);
    expect(arguments['height'], 80);
    expect(arguments['clipX'], 0);
    expect(arguments['clipY'], 0);
    expect(arguments['clipWidth'], 60);
    expect(arguments['clipHeight'], 60);
  });

  testWidgets(
    'tracks scrolling with ancestor clipping and restores after re-entry',
    (WidgetTester tester) async {
      final List<MethodCall> calls = <MethodCall>[];
      _mockLinuxWebViewCreation(onInstanceCall: calls.add);
      final LinuxWebViewController controller = LinuxWebViewController(
        const PlatformWebViewControllerCreationParams(),
      );
      final LinuxWebViewWidget platformWidget = LinuxWebViewWidget(
        PlatformWebViewWidgetCreationParams(controller: controller),
      );
      final ScrollController scrollController = ScrollController();
      addTearDown(scrollController.dispose);

      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 200,
              height: 180,
              child: ListView(
                controller: scrollController,
                children: <Widget>[
                  const SizedBox(height: 100),
                  SizedBox(
                    height: 160,
                    child: Builder(builder: platformWidget.build),
                  ),
                  const SizedBox(height: 400),
                ],
              ),
            ),
          ),
        ),
      );
      await tester.pump();

      Map<Object?, Object?> latestFrame() =>
          calls
                  .lastWhere((MethodCall call) => call.method == 'setFrame')
                  .arguments
              as Map<Object?, Object?>;

      expect(latestFrame()['y'], 100);
      expect(latestFrame()['height'], 160);
      expect(latestFrame()['clipY'], 100);
      expect(latestFrame()['clipHeight'], 80);

      scrollController.jumpTo(150);
      await tester.pump();
      expect(latestFrame()['visible'], isTrue);
      expect(latestFrame()['y'], -50);
      expect(latestFrame()['height'], 160);
      expect(latestFrame()['clipY'], 0);
      expect(latestFrame()['clipHeight'], 110);

      final int stableCallCount = calls
          .where((MethodCall call) => call.method == 'setFrame')
          .length;
      await tester.pump();
      await tester.pump();
      expect(
        calls.where((MethodCall call) => call.method == 'setFrame'),
        hasLength(stableCallCount),
      );

      scrollController.jumpTo(300);
      await tester.pump();
      expect(latestFrame()['visible'], isFalse);

      scrollController.jumpTo(150);
      await tester.pump();
      await tester.pump();
      expect(latestFrame()['visible'], isTrue);
      expect(latestFrame()['y'], -50);
      expect(latestFrame()['clipY'], 0);
      expect(latestFrame()['clipHeight'], 110);
    },
  );

  testWidgets('tracks a WebView that starts outside the scroll viewport', (
    WidgetTester tester,
  ) async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final LinuxWebViewWidget platformWidget = LinuxWebViewWidget(
      PlatformWebViewWidgetCreationParams(controller: controller),
    );
    final ScrollController scrollController = ScrollController(
      initialScrollOffset: 300,
    );
    addTearDown(scrollController.dispose);

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 200,
            height: 180,
            child: ListView(
              controller: scrollController,
              children: <Widget>[
                const SizedBox(height: 100),
                SizedBox(
                  height: 160,
                  child: Builder(builder: platformWidget.build),
                ),
                const SizedBox(height: 400),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    expect(
      calls.where(
        (MethodCall call) =>
            call.method == 'setFrame' &&
            (call.arguments as Map<Object?, Object?>)['visible'] == true,
      ),
      isEmpty,
    );

    scrollController.jumpTo(150);
    await tester.pump();
    await tester.pump();
    final Map<Object?, Object?> arguments =
        calls
                .lastWhere(
                  (MethodCall call) =>
                      call.method == 'setFrame' &&
                      (call.arguments as Map<Object?, Object?>)['visible'] ==
                          true,
                )
                .arguments
            as Map<Object?, Object?>;
    expect(arguments['y'], -50);
    expect(arguments['height'], 160);
    expect(arguments['clipY'], 0);
    expect(arguments['clipHeight'], 110);
  });

  testWidgets('intersects translated geometry with a ClipRect ancestor', (
    WidgetTester tester,
  ) async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final LinuxWebViewWidget platformWidget = LinuxWebViewWidget(
      PlatformWebViewWidgetCreationParams(controller: controller),
    );

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 800,
            height: 600,
            child: Row(
              children: <Widget>[
                const Expanded(child: SizedBox.expand()),
                Expanded(
                  child: ClipRect(
                    child: Transform.translate(
                      offset: const Offset(-90, 0),
                      child: Builder(builder: platformWidget.build),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    final Map<Object?, Object?> arguments =
        calls
                .lastWhere(
                  (MethodCall call) =>
                      call.method == 'setFrame' &&
                      (call.arguments as Map<Object?, Object?>)['visible'] ==
                          true,
                )
                .arguments
            as Map<Object?, Object?>;
    expect(arguments['x'], 310);
    expect(arguments['y'], 0);
    expect(arguments['width'], 400);
    expect(arguments['height'], 600);
    expect(arguments['clipX'], 400);
    expect(arguments['clipY'], 0);
    expect(arguments['clipWidth'], 310);
    expect(arguments['clipHeight'], 600);
  });

  testWidgets('hides native view for non-rectangular ancestor clips', (
    WidgetTester tester,
  ) async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final LinuxWebViewWidget platformWidget = LinuxWebViewWidget(
      PlatformWebViewWidgetCreationParams(controller: controller),
    );
    Widget Function(Widget child)? clipBuilder;

    Widget buildWebView() {
      final Widget webView = SizedBox(
        width: 100,
        height: 80,
        child: Builder(builder: platformWidget.build),
      );
      return Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: clipBuilder?.call(webView) ?? webView,
        ),
      );
    }

    await tester.pumpWidget(buildWebView());
    await tester.pump();
    expect(
      (calls.lastWhere((MethodCall call) => call.method == 'setFrame').arguments
          as Map<Object?, Object?>)['visible'],
      isTrue,
    );

    clipBuilder = (Widget child) => ClipOval(child: child);
    await tester.pumpWidget(buildWebView());
    await tester.pump();
    expect(
      (calls.lastWhere((MethodCall call) => call.method == 'setFrame').arguments
          as Map<Object?, Object?>)['visible'],
      isFalse,
    );

    clipBuilder = (Widget child) => ClipRSuperellipse(
      borderRadius: BorderRadius.circular(16),
      child: child,
    );
    await tester.pumpWidget(buildWebView());
    await tester.pump();
    expect(
      (calls.lastWhere((MethodCall call) => call.method == 'setFrame').arguments
          as Map<Object?, Object?>)['visible'],
      isFalse,
    );
  });

  testWidgets('keeps native geometry in logical pixels on HiDPI displays', (
    WidgetTester tester,
  ) async {
    tester.view.devicePixelRatio = 2;
    tester.view.physicalSize = const Size(1600, 1200);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(tester.view.resetPhysicalSize);

    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final LinuxWebViewWidget platformWidget = LinuxWebViewWidget(
      PlatformWebViewWidgetCreationParams(controller: controller),
    );

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: Transform.translate(
            offset: const Offset(20, 30),
            child: SizedBox(
              width: 100,
              height: 80,
              child: Builder(builder: platformWidget.build),
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    final MethodCall frameCall = calls.lastWhere(
      (MethodCall call) =>
          call.method == 'setFrame' &&
          (call.arguments as Map<Object?, Object?>)['visible'] == true,
    );
    final Map<Object?, Object?> arguments =
        frameCall.arguments as Map<Object?, Object?>;
    expect(arguments['x'], 20);
    expect(arguments['y'], 30);
    expect(arguments['width'], 100);
    expect(arguments['height'], 80);
  });

  testWidgets('hides native view for unsupported transforms', (
    WidgetTester tester,
  ) async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final LinuxWebViewWidget platformWidget = LinuxWebViewWidget(
      PlatformWebViewWidgetCreationParams(controller: controller),
    );
    var angle = 0.0;

    Widget buildWebView() {
      return Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: Transform.rotate(
            angle: angle,
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 100,
              height: 80,
              child: Builder(builder: platformWidget.build),
            ),
          ),
        ),
      );
    }

    await tester.pumpWidget(buildWebView());
    await tester.pump();
    angle = 0.2;
    await tester.pumpWidget(buildWebView());
    await tester.pump();

    final MethodCall frameCall = calls.lastWhere(
      (MethodCall call) => call.method == 'setFrame',
    );
    expect((frameCall.arguments as Map<Object?, Object?>)['visible'], isFalse);
  });

  testWidgets(
    'hides native view instead of reflowing it for scale transforms',
    (WidgetTester tester) async {
      final List<MethodCall> calls = <MethodCall>[];
      _mockLinuxWebViewCreation(onInstanceCall: calls.add);
      final LinuxWebViewController controller = LinuxWebViewController(
        const PlatformWebViewControllerCreationParams(),
      );
      final LinuxWebViewWidget platformWidget = LinuxWebViewWidget(
        PlatformWebViewWidgetCreationParams(controller: controller),
      );
      var scale = 1.0;

      Widget buildWebView() {
        return Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: Transform.scale(
              scale: scale,
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: 100,
                height: 80,
                child: Builder(builder: platformWidget.build),
              ),
            ),
          ),
        );
      }

      await tester.pumpWidget(buildWebView());
      await tester.pump();
      scale = 1.5;
      await tester.pumpWidget(buildWebView());
      await tester.pump();

      final MethodCall frameCall = calls.lastWhere(
        (MethodCall call) => call.method == 'setFrame',
      );
      expect(
        (frameCall.arguments as Map<Object?, Object?>)['visible'],
        isFalse,
      );
    },
  );

  testWidgets('hides and restores the native view with app lifecycle', (
    WidgetTester tester,
  ) async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final LinuxWebViewWidget platformWidget = LinuxWebViewWidget(
      PlatformWebViewWidgetCreationParams(controller: controller),
    );
    addTearDown(() {
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    });

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: SizedBox(
          width: 320,
          height: 180,
          child: Builder(builder: platformWidget.build),
        ),
      ),
    );
    await tester.pump();

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    await tester.pump();
    expect(
      (calls.lastWhere((MethodCall call) => call.method == 'setFrame').arguments
          as Map<Object?, Object?>)['visible'],
      isFalse,
    );

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pump();
    await tester.pump();
    expect(
      (calls.lastWhere((MethodCall call) => call.method == 'setFrame').arguments
          as Map<Object?, Object?>)['visible'],
      isTrue,
    );
  });

  testWidgets('hides and restores a retained native view after fading', (
    WidgetTester tester,
  ) async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final LinuxWebViewWidget platformWidget = LinuxWebViewWidget(
      PlatformWebViewWidgetCreationParams(controller: controller),
    );
    final child = RepaintBoundary(
      child: SizedBox(
        width: 320,
        height: 180,
        child: Builder(builder: platformWidget.build),
      ),
    );
    var opacity = 1.0;

    Widget buildWebView() {
      return Directionality(
        textDirection: TextDirection.ltr,
        child: AnimatedOpacity(
          opacity: opacity,
          duration: Duration.zero,
          child: child,
        ),
      );
    }

    try {
      await tester.pumpWidget(buildWebView());
      await tester.pumpAndSettle();
      for (final visible in <bool>[false, true, false, true]) {
        opacity = visible ? 1 : 0;
        await tester.pumpWidget(buildWebView());
        await tester.pumpAndSettle();
        final frameCall = calls.lastWhere((call) => call.method == 'setFrame');
        expect(
          (frameCall.arguments as Map<Object?, Object?>)['visible'],
          visible,
        );
        final frameCallCount = calls
            .where((call) => call.method == 'setFrame')
            .length;
        expect(tester.binding.hasScheduledFrame, isFalse);
        await tester.pump();
        expect(
          calls.where((call) => call.method == 'setFrame'),
          hasLength(frameCallCount),
        );
      }
    } finally {
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      await tester.runAsync(controller.dispose);
    }
  });

  test('dispose releases the native WebView exactly once', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    await controller.currentUrl();
    await controller.dispose();
    await controller.dispose();

    expect(
      calls.where((MethodCall call) => call.method == 'dispose'),
      hasLength(1),
    );
    await expectLater(controller.reload(), throwsStateError);
  });

  test('controller operations do not require a WebView widget', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    await controller.loadHtmlString('<p>headless</p>');
    await controller.runJavaScript('window.ready = true;');

    expect(
      calls.map((MethodCall call) => call.method),
      containsAll(<String>['loadHtmlString', 'runJavaScript']),
    );
    expect(
      calls.where((MethodCall call) => call.method == 'setFrame'),
      isEmpty,
    );
  });

  test('rejects invalid cookies before root channel setCookie', () async {
    final List<MethodCall> rootCalls = <MethodCall>[];
    _mockLinuxWebViewCreation(onRootCall: rootCalls.add);

    final LinuxWebViewCookieManager cookieManager = LinuxWebViewCookieManager(
      const PlatformWebViewCookieManagerCreationParams(),
    );
    const List<WebViewCookie> invalidCookies = <WebViewCookie>[
      WebViewCookie(name: 'bad name', value: 'value', domain: '', path: '/'),
      WebViewCookie(
        name: 'session',
        value: 'value',
        domain: 'example.com;bad',
        path: '/',
      ),
      WebViewCookie(
        name: 'session',
        value: 'value',
        domain: '',
        path: 'relative',
      ),
      WebViewCookie(
        name: 'session',
        value: 'value',
        domain: '',
        path: '/bad;path',
      ),
    ];

    for (final WebViewCookie cookie in invalidCookies) {
      await expectLater(
        () => cookieManager.setCookie(cookie),
        throwsA(isA<ArgumentError>()),
      );
    }

    expect(
      rootCalls.where((MethodCall call) => call.method == 'setCookie'),
      isEmpty,
    );
  });

  test('sets scrollbar visibility through separate platform calls', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    expect(controller.supportsSetScrollBarsEnabled(), isTrue);

    await controller.setVerticalScrollBarEnabled(false);
    await controller.setHorizontalScrollBarEnabled(false);
    await controller.setVerticalScrollBarEnabled(true);

    expect(calls, hasLength(3));
    expect(calls[0].method, 'setVerticalScrollBarEnabled');
    expect(calls[0].arguments, <String, Object?>{'enabled': false});
    expect(calls[1].method, 'setHorizontalScrollBarEnabled');
    expect(calls[1].arguments, <String, Object?>{'enabled': false});
    expect(calls[2].method, 'setVerticalScrollBarEnabled');
    expect(calls[2].arguments, <String, Object?>{'enabled': true});
  });

  test('sets overscroll mode through native platform calls', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    await controller.setOverScrollMode(WebViewOverScrollMode.never);
    await controller.setOverScrollMode(WebViewOverScrollMode.ifContentScrolls);
    await controller.setOverScrollMode(WebViewOverScrollMode.always);

    expect(calls, hasLength(3));
    expect(calls[0].method, 'setOverScrollMode');
    expect(calls[0].arguments, <String, Object?>{'mode': 'never'});
    expect(calls[1].method, 'setOverScrollMode');
    expect(calls[1].arguments, <String, Object?>{'mode': 'ifContentScrolls'});
    expect(calls[2].method, 'setOverScrollMode');
    expect(calls[2].arguments, <String, Object?>{'mode': 'always'});
  });

  test('applies Linux-specific WebKit settings from creation params', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const LinuxWebViewControllerCreationParams(
        developerExtrasEnabled: true,
        downloadsEnabled: false,
        javascriptCanOpenWindowsAutomatically: true,
        mediaPlaybackRequiresUserGesture: false,
        mediaPlaybackAllowsInline: true,
        pageCacheEnabled: true,
        allowFileAccessFromFileUrls: true,
        allowUniversalAccessFromFileUrls: false,
        zoomTextOnly: true,
        defaultFontSize: 18,
        defaultMonospaceFontSize: 14,
        minimumFontSize: 9,
        zoomFactor: 1.25,
      ),
    );

    await controller.currentUrl();

    expect(calls.first.method, 'applySettings');
    expect(calls.first.arguments, <String, Object?>{
      'developerExtrasEnabled': true,
      'downloadsEnabled': false,
      'javascriptCanOpenWindowsAutomatically': true,
      'mediaPlaybackRequiresUserGesture': false,
      'mediaPlaybackAllowsInline': true,
      'pageCacheEnabled': true,
      'allowFileAccessFromFileUrls': true,
      'allowUniversalAccessFromFileUrls': false,
      'zoomTextOnly': true,
      'defaultFontSize': 18,
      'defaultMonospaceFontSize': 14,
      'minimumFontSize': 9,
      'zoomFactor': 1.25,
    });
  });

  test('sends Linux-specific WebKit settings through platform calls', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    await controller.setDeveloperExtrasEnabled(true);
    await controller.openDevTools();
    await controller.setJavaScriptCanOpenWindowsAutomatically(true);
    await controller.setMediaPlaybackRequiresUserGesture(false);
    await controller.setMediaPlaybackAllowsInline(true);
    await controller.setPageCacheEnabled(true);
    await controller.setAllowFileAccessFromFileUrls(true);
    await controller.setAllowUniversalAccessFromFileUrls(false);
    await controller.setZoomTextOnly(true);
    await controller.setDefaultFontSize(18);
    await controller.setDefaultMonospaceFontSize(14);
    await controller.setMinimumFontSize(9);
    await controller.setZoomFactor(1.25);

    expect(calls.map((MethodCall call) => call.method), <String>[
      'setDeveloperExtrasEnabled',
      'openDevTools',
      'setJavaScriptCanOpenWindowsAutomatically',
      'setMediaPlaybackRequiresUserGesture',
      'setMediaPlaybackAllowsInline',
      'setPageCacheEnabled',
      'setAllowFileAccessFromFileUrls',
      'setAllowUniversalAccessFromFileUrls',
      'setZoomTextOnly',
      'setDefaultFontSize',
      'setDefaultMonospaceFontSize',
      'setMinimumFontSize',
      'setZoomFactor',
    ]);
    expect(calls[0].arguments, <String, Object?>{'enabled': true});
    expect(calls[1].arguments, isNull);
    expect(calls[2].arguments, <String, Object?>{'enabled': true});
    expect(calls[3].arguments, <String, Object?>{'require': false});
    expect(calls[4].arguments, <String, Object?>{'allow': true});
    expect(calls[5].arguments, <String, Object?>{'enabled': true});
    expect(calls[6].arguments, <String, Object?>{'allow': true});
    expect(calls[7].arguments, <String, Object?>{'allow': false});
    expect(calls[8].arguments, <String, Object?>{'enabled': true});
    expect(calls[9].arguments, <String, Object?>{'fontSize': 18});
    expect(calls[10].arguments, <String, Object?>{'fontSize': 14});
    expect(calls[11].arguments, <String, Object?>{'fontSize': 9});
    expect(calls[12].arguments, <String, Object?>{'zoomFactor': 1.25});
  });

  test(
    'leaves downloads enabled by default and forwards runtime changes',
    () async {
      final calls = <MethodCall>[];
      _mockLinuxWebViewCreation(onInstanceCall: calls.add);
      final controller = LinuxWebViewController(
        const LinuxWebViewControllerCreationParams.fromPlatformWebViewControllerCreationParams(
          PlatformWebViewControllerCreationParams(),
        ),
      );
      addTearDown(controller.dispose);
      await controller.setDownloadsEnabled(false);
      await controller.setDownloadsEnabled(true);
      expect(calls.map((call) => call.method), <String>[
        'setDownloadsEnabled',
        'setDownloadsEnabled',
      ]);
      expect(calls.map((call) => call.arguments), <Object?>[
        <String, Object?>{'enabled': false},
        <String, Object?>{'enabled': true},
      ]);
      await controller.dispose();
      await expectLater(
        controller.setDownloadsEnabled(false),
        throwsStateError,
      );
    },
  );

  test('loads requests with method headers and body', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final Uint8List body = Uint8List.fromList(<int>[1, 2, 3]);

    await controller.loadRequest(
      LoadRequestParams(
        uri: Uri.parse('https://example.test/form'),
        method: LoadRequestMethod.post,
        headers: const <String, String>{'X-Test': 'true'},
        body: body,
      ),
    );

    expect(calls, hasLength(1));
    expect(calls.single.method, 'loadRequest');
    expect(calls.single.arguments, <String, Object?>{
      'url': 'https://example.test/form',
      'method': 'post',
      'headers': <String, String>{'X-Test': 'true'},
      'body': body,
    });
  });

  test('loads files with params through the platform loadFile path', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final Directory tempDir = Directory.systemTemp.createTempSync(
      'webview_all_linux_test_',
    );
    addTearDown(() {
      tempDir.deleteSync(recursive: true);
    });
    final File file = File('${tempDir.path}/index.html')
      ..writeAsStringSync('<html></html>');

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    await controller.loadFileWithParams(
      LoadFileParams(absoluteFilePath: file.path),
    );

    expect(calls, hasLength(1));
    expect(calls.single.method, 'loadFile');
    expect(calls.single.arguments, <String, Object?>{
      'path': file.resolveSymbolicLinksSync(),
    });
  });

  test('rejects relative files and traversing asset keys', () async {
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    await expectLater(
      controller.loadFile('relative/index.html'),
      throwsArgumentError,
    );
    for (final String key in <String>[
      '../pubspec.yaml',
      'docs/../../pubspec.yaml',
      '/absolute.html',
      r'docs\index.html',
    ]) {
      await expectLater(controller.loadFlutterAsset(key), throwsArgumentError);
    }
  });

  test('clears local storage through the native WebKit data manager', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    await controller.clearLocalStorage();

    expect(calls, hasLength(1));
    expect(calls.single.method, 'clearLocalStorage');
    expect(calls.single.arguments, isNull);
  });

  test('clears all website data without creating a controller', () async {
    final List<MethodCall> rootCalls = <MethodCall>[];
    _mockLinuxWebViewCreation(onRootCall: rootCalls.add);
    final LinuxWebViewDataManager manager = LinuxWebViewDataManager(
      const PlatformWebViewDataManagerCreationParams(),
    );

    final WebViewDataClearingResult result = await manager
        .clearAllWebsiteData();

    expect(result.isComplete, isTrue);
    expect(rootCalls, hasLength(1));
    expect(rootCalls.single.method, 'clearAllWebsiteData');
  });

  test('invokes asynchronous JavaScript and correlates its result', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    final Future<Object?> result = controller.callAsyncJavaScript(
      JavaScriptInvocationParams(
        functionBody: 'return value + 1;',
        arguments: const <String, Object?>{'value': 6},
      ),
    );
    await _flushAsyncEvents();
    final MethodCall invocation = calls.singleWhere(
      (MethodCall call) => call.method == 'runJavaScript',
    );
    final String script =
        (invocation.arguments as Map<Object?, Object?>)['script']! as String;
    final String identifier = RegExp(
      r'const __identifier="([^"]+)"',
    ).firstMatch(script)!.group(1)!;
    expect(script, isNot(contains('AsyncFunction')));
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'asyncJavaScriptResult',
      'message': jsonEncode(<String, Object?>{
        'identifier': 42,
        'success': true,
        'value': 'ignored',
      }),
    });
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'asyncJavaScriptResult',
      'message': jsonEncode(<String, Object?>{
        'identifier': identifier,
        'success': true,
        'value': 7,
      }),
    });

    await expectLater(result, completion(7));
  });

  test('registers and removes document-start user scripts', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    expect(
      await controller.isUserScriptInjectionSupported(
        WebViewUserScriptInjectionTime.documentStart,
      ),
      isTrue,
    );
    final String identifier = await controller.addUserScript(
      const WebViewUserScript(source: 'window.provider = {};'),
    );
    await controller.removeUserScript(identifier);

    expect(
      calls.map((MethodCall call) => call.method),
      containsAllInOrder(<String>['addUserScript', 'removeUserScript']),
    );
    final MethodCall addCall = calls.firstWhere(
      (MethodCall call) => call.method == 'addUserScript',
    );
    final Map<Object?, Object?> arguments =
        addCall.arguments! as Map<Object?, Object?>;
    expect(arguments['source'], contains('window.provider = {};'));
    expect(arguments['source'], contains('}).call(globalThis);'));
    expect(arguments['mainFrameOnly'], isTrue);
  });

  test('dispatches HTTP response errors from Linux events', () async {
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final LinuxNavigationDelegate delegate = LinuxNavigationDelegate(
      const PlatformNavigationDelegateCreationParams(),
    );
    final List<HttpResponseError> errors = <HttpResponseError>[];

    await controller.currentUrl();
    await delegate.setOnHttpError(errors.add);
    await controller.setPlatformNavigationDelegate(delegate);
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'httpError',
      'url': 'https://example.test/missing',
      'method': 'POST',
      'requestHeaders': <String, Object?>{'Accept': 'text/plain'},
      'isForMainFrame': true,
      'statusCode': 404,
      'headers': <String, Object?>{'Content-Type': 'text/plain'},
      'mimeType': 'text/plain',
    });

    expect(errors, hasLength(1));
    expect(
      errors.single.request?.uri,
      Uri.parse('https://example.test/missing'),
    );
    expect(errors.single.request, isA<LinuxWebResourceRequest>());
    final LinuxWebResourceRequest request =
        errors.single.request! as LinuxWebResourceRequest;
    expect(request.method, 'POST');
    expect(request.headers, const <String, String>{'Accept': 'text/plain'});
    expect(request.isForMainFrame, isTrue);
    expect(
      errors.single.response?.uri,
      Uri.parse('https://example.test/missing'),
    );
    expect(errors.single.response, isA<LinuxWebResourceResponse>());
    final LinuxWebResourceResponse response =
        errors.single.response! as LinuxWebResourceResponse;
    expect(errors.single.response?.statusCode, 404);
    expect(response.headers, <String, String>{'Content-Type': 'text/plain'});
    expect(response.mimeType, 'text/plain');
  });

  test('dispatches web resource errors from Linux events', () async {
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final LinuxNavigationDelegate delegate = LinuxNavigationDelegate(
      const PlatformNavigationDelegateCreationParams(),
    );
    final List<WebResourceError> errors = <WebResourceError>[];

    await controller.currentUrl();
    await delegate.setOnWebResourceError(errors.add);
    await controller.setPlatformNavigationDelegate(delegate);
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'webResourceError',
      'errorCode': 7,
      'description': 'Timed out',
      'errorType': 'timeout',
      'isForMainFrame': false,
      'url': 'https://example.test/slow',
    });

    expect(errors, hasLength(1));
    expect(errors.single, isA<LinuxWebResourceError>());
    final LinuxWebResourceError error = errors.single as LinuxWebResourceError;
    expect(error.errorCode, 7);
    expect(error.description, 'Timed out');
    expect(error.errorType, WebResourceErrorType.timeout);
    expect(error.isForMainFrame, isFalse);
    expect(error.url, 'https://example.test/slow');
  });

  test(
    'dispatches console and scroll position events from Linux events',
    () async {
      final List<MethodCall> calls = <MethodCall>[];
      _mockLinuxWebViewCreation(onInstanceCall: calls.add);

      final LinuxWebViewController controller = LinuxWebViewController(
        const PlatformWebViewControllerCreationParams(),
      );
      final List<JavaScriptConsoleMessage> consoleMessages =
          <JavaScriptConsoleMessage>[];
      final List<ScrollPositionChange> scrollChanges = <ScrollPositionChange>[];

      await controller.currentUrl();
      await controller.setOnConsoleMessage(consoleMessages.add);
      await controller.setOnScrollPositionChange(scrollChanges.add);
      await _emitLinuxWebViewEvent(<String, Object?>{
        'type': 'consoleMessage',
        'level': 'warning',
        'message': 'careful',
      });
      await _emitLinuxWebViewEvent(<String, Object?>{
        'type': 'consoleMessage',
        'level': 'debug',
        'message': 'details',
      });
      await _emitLinuxWebViewEvent(<String, Object?>{
        'type': 'scrollPositionChange',
        'x': 12.5,
        'y': 34,
      });
      await _flushAsyncEvents();

      expect(
        calls.where((MethodCall call) => call.method == 'setOnConsoleMessage'),
        hasLength(1),
      );
      expect(
        calls
            .singleWhere(
              (MethodCall call) => call.method == 'setOnConsoleMessage',
            )
            .arguments,
        <String, Object?>{'enabled': true},
      );
      expect(
        calls.where(
          (MethodCall call) => call.method == 'setOnScrollPositionChange',
        ),
        hasLength(1),
      );
      expect(
        calls
            .singleWhere(
              (MethodCall call) => call.method == 'setOnScrollPositionChange',
            )
            .arguments,
        <String, Object?>{'enabled': true},
      );
      expect(consoleMessages, hasLength(2));
      expect(consoleMessages[0].level, JavaScriptLogLevel.warning);
      expect(consoleMessages[0].message, 'careful');
      expect(consoleMessages[1].level, JavaScriptLogLevel.debug);
      expect(consoleMessages[1].message, 'details');
      expect(scrollChanges, hasLength(1));
      expect(scrollChanges.single.x, 12.5);
      expect(scrollChanges.single.y, 34);
    },
  );

  test(
    'Linux console bridge script safely stringifies non-json values',
    () async {
      final String source = File(
        '../thirdparty/webview_all_linux/linux/src/webview/webview_javascript.cc',
      ).readAsStringSync();

      expect(source, contains('function stringifyArg'));
      expect(
        source,
        contains('return json === undefined ? String(arg) : json'),
      );
      expect(source, contains('Array.from(arguments).map(stringifyArg)'));
    },
  );

  test('completes JavaScript dialog requests from Linux events', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final List<JavaScriptAlertDialogRequest> alertRequests =
        <JavaScriptAlertDialogRequest>[];
    final List<JavaScriptConfirmDialogRequest> confirmRequests =
        <JavaScriptConfirmDialogRequest>[];
    final List<JavaScriptTextInputDialogRequest> promptRequests =
        <JavaScriptTextInputDialogRequest>[];

    await controller.currentUrl();
    await controller.setOnJavaScriptAlertDialog((
      JavaScriptAlertDialogRequest request,
    ) async {
      alertRequests.add(request);
    });
    await controller.setOnJavaScriptConfirmDialog((
      JavaScriptConfirmDialogRequest request,
    ) async {
      confirmRequests.add(request);
      return false;
    });
    await controller.setOnJavaScriptTextInputDialog((
      JavaScriptTextInputDialogRequest request,
    ) async {
      promptRequests.add(request);
      return 'typed value';
    });

    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'javaScriptDialog',
      'requestId': 21,
      'dialogType': 'alert',
      'message': 'hello alert',
      'url': 'https://example.test/alert',
    });
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'javaScriptDialog',
      'requestId': 22,
      'dialogType': 'confirm',
      'message': 'continue?',
      'url': 'https://example.test/confirm',
    });
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'javaScriptDialog',
      'requestId': 23,
      'dialogType': 'prompt',
      'message': 'name',
      'url': 'https://example.test/prompt',
      'defaultText': 'default value',
    });
    await _flushAsyncEvents();

    expect(alertRequests, hasLength(1));
    expect(alertRequests.single.message, 'hello alert');
    expect(alertRequests.single.url, 'https://example.test/alert');
    expect(confirmRequests, hasLength(1));
    expect(confirmRequests.single.message, 'continue?');
    expect(confirmRequests.single.url, 'https://example.test/confirm');
    expect(promptRequests, hasLength(1));
    expect(promptRequests.single.message, 'name');
    expect(promptRequests.single.url, 'https://example.test/prompt');
    expect(promptRequests.single.defaultText, 'default value');

    final List<MethodCall> dialogCallbackCalls = calls
        .where(
          (MethodCall call) =>
              call.method == 'setJavaScriptDialogCallbacksEnabled',
        )
        .toList();
    expect(dialogCallbackCalls, hasLength(3));
    expect(dialogCallbackCalls[0].arguments, <String, Object?>{
      'alert': true,
      'confirm': false,
      'prompt': false,
    });
    expect(dialogCallbackCalls[1].arguments, <String, Object?>{
      'alert': true,
      'confirm': true,
      'prompt': false,
    });
    expect(dialogCallbackCalls[2].arguments, <String, Object?>{
      'alert': true,
      'confirm': true,
      'prompt': true,
    });

    final List<MethodCall> dialogCalls = calls
        .where((MethodCall call) => call.method == 'completeJavaScriptDialog')
        .toList();
    expect(dialogCalls, hasLength(3));
    expect(dialogCalls[0].arguments, <String, Object?>{
      'requestId': 21,
      'action': 'confirm',
      'text': null,
    });
    expect(dialogCalls[1].arguments, <String, Object?>{
      'requestId': 22,
      'action': 'cancel',
      'text': null,
    });
    expect(dialogCalls[2].arguments, <String, Object?>{
      'requestId': 23,
      'action': 'confirm',
      'text': 'typed value',
    });
  });

  test('uses safe JavaScript dialog defaults without Linux handlers', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    await controller.currentUrl();
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'javaScriptDialog',
      'requestId': 24,
      'dialogType': 'alert',
      'message': 'hello alert',
      'url': 'https://example.test/alert',
    });
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'javaScriptDialog',
      'requestId': 25,
      'dialogType': 'confirm',
      'message': 'continue?',
      'url': 'https://example.test/confirm',
    });
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'javaScriptDialog',
      'requestId': 26,
      'dialogType': 'prompt',
      'message': 'name',
      'url': 'https://example.test/prompt',
      'defaultText': 'default value',
    });
    await _flushAsyncEvents();

    final List<MethodCall> dialogCalls = calls
        .where((MethodCall call) => call.method == 'completeJavaScriptDialog')
        .toList();
    expect(dialogCalls, hasLength(3));
    expect(dialogCalls[0].arguments, <String, Object?>{
      'requestId': 24,
      'action': 'confirm',
      'text': null,
    });
    expect(dialogCalls[1].arguments, <String, Object?>{
      'requestId': 25,
      'action': 'confirm',
      'text': null,
    });
    expect(dialogCalls[2].arguments, <String, Object?>{
      'requestId': 26,
      'action': 'confirm',
      'text': 'default value',
    });
  });

  test('cancels JavaScript dialogs when Linux handlers throw', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    await controller.currentUrl();
    await controller.setOnJavaScriptConfirmDialog((
      JavaScriptConfirmDialogRequest request,
    ) async {
      throw StateError('dialog failure');
    });
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'javaScriptDialog',
      'requestId': 27,
      'dialogType': 'confirm',
      'message': 'continue?',
      'url': 'https://example.test/confirm',
    });
    await _flushAsyncEvents();

    final List<MethodCall> dialogCalls = calls
        .where((MethodCall call) => call.method == 'completeJavaScriptDialog')
        .toList();
    expect(dialogCalls, hasLength(1));
    expect(dialogCalls.single.arguments, <String, Object?>{
      'requestId': 27,
      'action': 'cancel',
      'text': null,
    });
  });

  test('completes HTTP auth requests once from Linux events', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final LinuxNavigationDelegate delegate = LinuxNavigationDelegate(
      const PlatformNavigationDelegateCreationParams(),
    );
    final List<HttpAuthRequest> requests = <HttpAuthRequest>[];

    await controller.currentUrl();
    await delegate.setOnHttpAuthRequest((HttpAuthRequest request) {
      requests.add(request);
      request.onProceed(
        const WebViewCredential(user: 'test-user', password: 'test-password'),
      );
      request.onCancel();
      request.onProceed(
        const WebViewCredential(user: 'other-user', password: 'other-password'),
      );
    });
    await controller.setPlatformNavigationDelegate(delegate);
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'httpAuthRequest',
      'requestId': 12,
      'host': 'secure.example.test',
      'realm': 'Restricted Area',
    });
    await _flushAsyncEvents();

    expect(requests, hasLength(1));
    expect(requests.single.host, 'secure.example.test');
    expect(requests.single.realm, 'Restricted Area');
    final List<MethodCall> authCalls = calls
        .where((MethodCall call) => call.method == 'completeHttpAuthRequest')
        .toList();
    expect(authCalls, hasLength(1));
    expect(authCalls.single.arguments, <String, Object?>{
      'requestId': 12,
      'action': 'proceed',
      'user': 'test-user',
      'password': 'test-password',
    });
  });

  test('cancels HTTP auth requests without a Linux handler', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    await controller.currentUrl();
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'httpAuthRequest',
      'requestId': 13,
      'host': 'secure.example.test',
      'realm': 'Restricted Area',
    });
    await _flushAsyncEvents();

    final List<MethodCall> authCalls = calls
        .where((MethodCall call) => call.method == 'completeHttpAuthRequest')
        .toList();
    expect(authCalls, hasLength(1));
    expect(authCalls.single.arguments, <String, Object?>{
      'requestId': 13,
      'action': 'cancel',
    });
  });

  test('completes SSL auth errors once from Linux events', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final LinuxNavigationDelegate delegate = LinuxNavigationDelegate(
      const PlatformNavigationDelegateCreationParams(),
    );
    final List<PlatformSslAuthError> errors = <PlatformSslAuthError>[];

    await controller.currentUrl();
    await delegate.setOnSSlAuthError((PlatformSslAuthError error) {
      errors.add(error);
      unawaited(error.proceed());
      unawaited(error.cancel());
      unawaited(error.proceed());
    });
    await controller.setPlatformNavigationDelegate(delegate);
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'sslAuthError',
      'requestId': 7,
      'url': 'https://expired.example.test/',
      'description':
          'TLS certificate error for expired.example.test. '
          'Certificate has expired.',
    });
    await _flushAsyncEvents();

    expect(errors, hasLength(1));
    expect(errors.single.certificate, isNull);
    expect(errors.single.description, contains('expired.example.test'));
    expect(errors.single.description, contains('Certificate has expired'));
    final List<MethodCall> sslCalls = calls
        .where((MethodCall call) => call.method == 'completeSslAuthError')
        .toList();
    expect(sslCalls, hasLength(1));
    expect(sslCalls.single.arguments, <String, Object?>{
      'requestId': 7,
      'proceed': true,
    });
  });

  test('cancels SSL auth errors without a Linux handler', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    await controller.currentUrl();
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'sslAuthError',
      'requestId': 8,
      'url': 'https://expired.example.test/',
      'description': 'TLS certificate error',
    });
    await _flushAsyncEvents();

    final List<MethodCall> sslCalls = calls
        .where((MethodCall call) => call.method == 'completeSslAuthError')
        .toList();
    expect(sslCalls, hasLength(1));
    expect(sslCalls.single.arguments, <String, Object?>{
      'requestId': 8,
      'proceed': false,
    });
  });

  test('dispatches permission requests from Linux events', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final List<PlatformWebViewPermissionRequest> requests =
        <PlatformWebViewPermissionRequest>[];
    int requestCount = 0;

    await controller.currentUrl();
    await controller.setOnPlatformPermissionRequest((
      PlatformWebViewPermissionRequest request,
    ) {
      requests.add(request);
      requestCount += 1;
      if (requestCount == 1) {
        unawaited(request.grant());
        unawaited(request.deny());
        unawaited(request.grant());
      } else {
        unawaited(request.deny());
      }
    });
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'permissionRequest',
      'requestId': 9,
      'types': <String>['camera', 'microphone', 'unknown'],
    });
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'permissionRequest',
      'requestId': 10,
      'types': <String>['microphone'],
    });
    await _flushAsyncEvents();

    expect(requests, hasLength(2));
    expect(requests[0].types, <WebViewPermissionResourceType>{
      WebViewPermissionResourceType.camera,
      WebViewPermissionResourceType.microphone,
    });
    expect(requests[1].types, <WebViewPermissionResourceType>{
      WebViewPermissionResourceType.microphone,
    });
    final List<MethodCall> permissionCalls = calls
        .where((MethodCall call) => call.method == 'completePermissionRequest')
        .toList();
    expect(permissionCalls, hasLength(2));
    expect(permissionCalls[0].arguments, <String, Object?>{
      'requestId': 9,
      'grant': true,
    });
    expect(permissionCalls[1].arguments, <String, Object?>{
      'requestId': 10,
      'grant': false,
    });
  });

  test('denies permission requests without a Linux handler', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);

    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );

    await controller.currentUrl();
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'permissionRequest',
      'requestId': 11,
      'types': <String>['camera'],
    });
    await _flushAsyncEvents();

    final List<MethodCall> permissionCalls = calls
        .where((MethodCall call) => call.method == 'completePermissionRequest')
        .toList();
    expect(permissionCalls, hasLength(1));
    expect(permissionCalls.single.arguments, <String, Object?>{
      'requestId': 11,
      'grant': false,
    });
  });

  test(
    'denies Linux permission requests without recognized resources',
    () async {
      final List<MethodCall> calls = <MethodCall>[];
      _mockLinuxWebViewCreation(onInstanceCall: calls.add);

      final LinuxWebViewController controller = LinuxWebViewController(
        const PlatformWebViewControllerCreationParams(),
      );
      var callbackCalled = false;

      await controller.currentUrl();
      await controller.setOnPlatformPermissionRequest((_) {
        callbackCalled = true;
      });
      await _emitLinuxWebViewEvent(<String, Object?>{
        'type': 'permissionRequest',
        'requestId': 12,
        'types': <String>['unknown'],
      });
      await _flushAsyncEvents();

      final List<MethodCall> permissionCalls = calls
          .where(
            (MethodCall call) => call.method == 'completePermissionRequest',
          )
          .toList();
      expect(callbackCalled, isFalse);
      expect(permissionCalls, hasLength(1));
      expect(permissionCalls.single.arguments, <String, Object?>{
        'requestId': 12,
        'grant': false,
      });
    },
  );

  test('uses safe decisions when application callbacks throw', () async {
    final List<MethodCall> calls = <MethodCall>[];
    _mockLinuxWebViewCreation(onInstanceCall: calls.add);
    final LinuxWebViewController controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    final LinuxNavigationDelegate delegate = LinuxNavigationDelegate(
      const PlatformNavigationDelegateCreationParams(),
    );

    await controller.currentUrl();
    await delegate.setOnNavigationRequest((NavigationRequest request) {
      throw StateError('navigation callback failed');
    });
    await delegate.setOnHttpAuthRequest((HttpAuthRequest request) {
      throw StateError('authentication callback failed');
    });
    await delegate.setOnSSlAuthError((PlatformSslAuthError error) {
      throw StateError('TLS callback failed');
    });
    await controller.setPlatformNavigationDelegate(delegate);
    await controller.setOnPlatformPermissionRequest((_) {
      throw StateError('permission callback failed');
    });
    await controller.setOnConsoleMessage((_) {
      throw StateError('console callback failed');
    });
    await controller.setOnJavaScriptConfirmDialog((_) {
      throw StateError('dialog callback failed');
    });

    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'navigationRequest',
      'requestId': 31,
      'url': 'https://example.test/navigation',
      'isMainFrame': true,
    });
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'httpAuthRequest',
      'requestId': 32,
      'host': 'secure.example.test',
    });
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'sslAuthError',
      'requestId': 33,
      'description': 'TLS certificate error',
    });
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'permissionRequest',
      'requestId': 34,
      'types': <String>['camera'],
    });
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'consoleMessage',
      'level': 'error',
      'message': 'test',
    });
    await _emitLinuxWebViewEvent(<String, Object?>{
      'type': 'javaScriptDialog',
      'requestId': 35,
      'dialogType': 'confirm',
      'message': 'Continue?',
      'url': 'https://example.test/dialog',
    });
    await _flushAsyncEvents();

    expect(
      calls
          .singleWhere(
            (MethodCall call) => call.method == 'completeNavigationRequest',
          )
          .arguments,
      <String, Object?>{'requestId': 31, 'allow': false},
    );
    expect(
      calls
          .singleWhere(
            (MethodCall call) => call.method == 'completeHttpAuthRequest',
          )
          .arguments,
      <String, Object?>{'requestId': 32, 'action': 'cancel'},
    );
    expect(
      calls
          .singleWhere(
            (MethodCall call) => call.method == 'completeSslAuthError',
          )
          .arguments,
      <String, Object?>{'requestId': 33, 'proceed': false},
    );
    expect(
      calls
          .singleWhere(
            (MethodCall call) => call.method == 'completePermissionRequest',
          )
          .arguments,
      <String, Object?>{'requestId': 34, 'grant': false},
    );
    expect(
      calls
          .singleWhere(
            (MethodCall call) => call.method == 'completeJavaScriptDialog',
          )
          .arguments,
      <String, Object?>{'requestId': 35, 'action': 'cancel', 'text': null},
    );
  });

  test(
    'serves local resources through the operit-vfs bridge protocol',
    () async {
      _mockLinuxWebViewCreation(instanceIds: <int>[77]);
      final List<MethodCall> resourceCalls = <MethodCall>[];
      final TestDefaultBinaryMessenger messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      messenger.setMockMethodCallHandler(WebViewLocalResourceBridge.channel, (
        MethodCall call,
      ) async {
        resourceCalls.add(call);
        return null;
      });
      addTearDown(() {
        messenger.setMockMethodCallHandler(
          WebViewLocalResourceBridge.channel,
          null,
        );
      });
      final LinuxWebViewController controller = LinuxWebViewController(
        const PlatformWebViewControllerCreationParams(),
      );
      addTearDown(controller.dispose);

      expect(controller.localResourceScheme, 'operit-vfs');
      await controller.setLocalResourceHandler((
        WebViewLocalResourceRequest request,
      ) async {
        expect(request.url, 'operit-vfs://p-test.operit.invalid/resource.html');
        expect(request.isMainFrame, isTrue);
        return WebViewLocalResourceResponse(
          body: Uint8List.fromList(<int>[1, 2, 3]),
          mimeType: 'text/html',
        );
      });
      expect(resourceCalls.single.method, 'configure');
      expect(resourceCalls.single.arguments, <String, Object?>{
        'identifier': 77,
        'enabled': true,
      });

      ByteData? reply;
      await messenger.handlePlatformMessage(
        WebViewLocalResourceBridge.channel.name,
        const StandardMethodCodec().encodeMethodCall(
          const MethodCall('request', <String, Object?>{
            'identifier': 77,
            'url': 'operit-vfs://p-test.operit.invalid/resource.html',
            'method': 'GET',
            'headers': <String, String>{},
            'isMainFrame': true,
          }),
        ),
        (ByteData? data) {
          reply = data;
        },
      );
      final Map<Object?, Object?> result =
          const StandardMethodCodec().decodeEnvelope(reply!)
              as Map<Object?, Object?>;
      expect(result['statusCode'], 200);
      expect(result['mimeType'], 'text/html');
      expect(result['body'], <int>[1, 2, 3]);
    },
  );
}

void _mockLinuxWebViewCreation({
  void Function(MethodCall call)? onRootCall,
  void Function(MethodCall call)? onInstanceCall,
  void Function(int id, MethodCall call)? onInstanceCallWithId,
  List<int>? instanceIds,
}) {
  final TestDefaultBinaryMessenger messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  _clearLinuxInstanceMocks();
  final List<int> ids = instanceIds ?? <int>[++_lastMockLinuxWebViewInstanceId];
  _activeMockLinuxWebViewInstanceIds = ids;
  var nextInstanceIndex = 0;
  messenger.setMockMethodCallHandler(
    const MethodChannel(linuxWebViewChannelPrefix),
    (MethodCall methodCall) async {
      onRootCall?.call(methodCall);
      if (methodCall.method == 'createWebView') {
        final int index = nextInstanceIndex < ids.length
            ? nextInstanceIndex++
            : ids.length - 1;
        return ids[index];
      }
      return null;
    },
  );
  for (final int id in ids) {
    messenger.setMockMethodCallHandler(
      MethodChannel('$linuxWebViewChannelPrefix/$id'),
      (MethodCall methodCall) async {
        onInstanceCall?.call(methodCall);
        onInstanceCallWithId?.call(id, methodCall);
        return null;
      },
    );
    messenger.setMockMethodCallHandler(
      MethodChannel('$linuxWebViewChannelPrefix/$id/events'),
      (MethodCall methodCall) async => null,
    );
  }
}

int _lastMockLinuxWebViewInstanceId = 0;
List<int> _activeMockLinuxWebViewInstanceIds = <int>[];

void _clearLinuxInstanceMocks() {
  final TestDefaultBinaryMessenger messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  for (final int id in _activeMockLinuxWebViewInstanceIds) {
    messenger.setMockMethodCallHandler(
      MethodChannel('$linuxWebViewChannelPrefix/$id'),
      null,
    );
    messenger.setMockMethodCallHandler(
      MethodChannel('$linuxWebViewChannelPrefix/$id/events'),
      null,
    );
  }
}

void _clearLinuxWebViewCreationMock() {
  final TestDefaultBinaryMessenger messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  messenger.setMockMethodCallHandler(
    const MethodChannel(linuxWebViewChannelPrefix),
    null,
  );
  _clearLinuxInstanceMocks();
  _activeMockLinuxWebViewInstanceIds = <int>[];
}

Future<void> _emitLinuxWebViewEvent(Map<String, Object?> event) async {
  final TestDefaultBinaryMessenger messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  final Completer<ByteData?> completer = Completer<ByteData?>();
  final int id = _activeMockLinuxWebViewInstanceIds.first;
  await messenger.handlePlatformMessage(
    '$linuxWebViewChannelPrefix/$id/events',
    const StandardMethodCodec().encodeSuccessEnvelope(event),
    completer.complete,
  );
  await completer.future;
}

Future<void> _flushAsyncEvents() async {
  await Future<void>.delayed(Duration.zero);
  await Future<void>.delayed(Duration.zero);
}
