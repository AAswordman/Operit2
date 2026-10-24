import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:webview_all_linux/src/linux_webview_constants.dart';
import 'package:webview_all_linux/src/linux_webview_controller.dart';
import 'package:webview_all_linux/src/linux_webview_widget.dart';
import 'package:webview_platform_interface/webview_platform_interface.dart';

/// Verifies scene markers and input ownership across Flutter modal overlays.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  late LinuxWebViewController controller;
  late List<MethodCall> calls;

  setUp(() {
    calls = <MethodCall>[];
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(
      WebViewLocalResourceBridge.channel,
      (call) async => null,
    );
    messenger.setMockMethodCallHandler(
      const MethodChannel(linuxWebViewChannelPrefix),
      (call) async => call.method == 'createWebView' ? 91 : null,
    );
    messenger.setMockMethodCallHandler(
      const MethodChannel('$linuxWebViewChannelPrefix/91'),
      (call) async {
        calls.add(call);
        return null;
      },
    );
    messenger.setMockMethodCallHandler(
      const MethodChannel('$linuxWebViewChannelPrefix/91/events'),
      (call) async => null,
    );
  });
  tearDown(() {
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    for (final name in <String>[
      WebViewLocalResourceBridge.channel.name,
      linuxWebViewChannelPrefix,
      '$linuxWebViewChannelPrefix/91',
      '$linuxWebViewChannelPrefix/91/events',
    ]) {
      messenger.setMockMethodCallHandler(MethodChannel(name), null);
    }
  });

  testWidgets('emits a platform layer with the clipped native viewport', (
    tester,
  ) async {
    controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.topLeft,
            child: ClipRect(
              child: SizedBox(
                width: 160,
                height: 100,
                child: OverflowBox(
                  alignment: Alignment.topLeft,
                  maxWidth: 300,
                  maxHeight: 200,
                  child: SizedBox(
                    width: 300,
                    height: 200,
                    child: _webview(controller),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump();
    final marker = tester.layers.whereType<PlatformViewLayer>().single;
    expect(marker.viewId, 91);
    expect(marker.rect.size, const Size(160, 100));
    final frame =
        calls.lastWhere((call) => call.method == 'setFrame').arguments as Map;
    expect(frame['width'], 300);
    expect(frame['height'], 200);
    expect(frame['clipWidth'], 160);
    expect(frame['clipHeight'], 100);
    await tester.pumpWidget(const SizedBox());
    await tester.runAsync(controller.dispose);
  });

  testWidgets('places native markers through the scene transform stack', (
    tester,
  ) async {
    controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Stack(
          children: [
            Positioned(
              left: 40,
              top: 60,
              width: 160,
              height: 100,
              child: _webview(controller),
            ),
          ],
        ),
      ),
    );
    await tester.pump();
    await tester.pump();
    final marker = tester.layers.whereType<PlatformViewLayer>().single;
    expect(marker.rect, const Rect.fromLTWH(0, 0, 160, 100));
    expect(marker.parent, isA<OffsetLayer>());
    expect((marker.parent! as OffsetLayer).offset, const Offset(40, 60));
    await tester.pumpWidget(const SizedBox());
    await tester.runAsync(controller.dispose);
  });

  testWidgets('modal barrier blocks browser clicks and wheel until dismissal', (
    tester,
  ) async {
    controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    late BuildContext routeContext;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) {
            routeContext = context;
            return Scaffold(body: _webview(controller));
          },
        ),
      ),
    );
    await tester.pump();
    await tester.pump();
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    const point = Offset(50, 500);
    await mouse.addPointer(location: point);
    await mouse.down(point);
    await mouse.up();
    await tester.pump();
    await tester.pump();
    expect(
      _events(calls, LinuxWebViewPointerEventType.buttonPress),
      hasLength(1),
    );
    showDialog<void>(
      context: routeContext,
      barrierDismissible: false,
      builder: (_) => const AlertDialog(
        title: Text('Flutter dialog'),
        content: TextField(autofocus: true),
      ),
    );
    await tester.pumpAndSettle();
    final int focusReleases = calls
        .where((call) => call.method == 'releaseFocus')
        .length;
    expect(focusReleases, greaterThan(0));
    await mouse.down(point);
    await mouse.up();
    tester.binding.handlePointerEvent(
      const PointerScrollEvent(position: point, scrollDelta: Offset(0, 40)),
    );
    await tester.pump();
    await tester.pump();
    expect(
      _events(calls, LinuxWebViewPointerEventType.buttonPress),
      hasLength(1),
    );
    expect(_events(calls, LinuxWebViewPointerEventType.scroll), isEmpty);
    Navigator.of(routeContext).pop();
    await tester.pumpAndSettle();
    await mouse.down(point);
    await mouse.up();
    tester.binding.handlePointerEvent(
      const PointerScrollEvent(position: point, scrollDelta: Offset(0, 40)),
    );
    await tester.pump();
    await tester.pump();
    expect(
      _events(calls, LinuxWebViewPointerEventType.buttonPress),
      hasLength(2),
    );
    expect(_events(calls, LinuxWebViewPointerEventType.scroll), hasLength(1));
    await mouse.removePointer();
    await tester.pumpWidget(const SizedBox());
    await tester.runAsync(controller.dispose);
  });

  testWidgets('an ancestor drag wins without sending browser touch events', (
    tester,
  ) async {
    controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    var dragUpdates = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: GestureDetector(
          onVerticalDragUpdate: (_) => dragUpdates++,
          child: _webview(controller),
        ),
      ),
    );
    await tester.pump();
    await tester.pump();
    await tester.dragFrom(const Offset(100, 200), const Offset(0, 150));
    await tester.pump();
    expect(dragUpdates, greaterThan(0));
    expect(_events(calls, LinuxWebViewPointerEventType.touchBegin), isEmpty);
    expect(_events(calls, LinuxWebViewPointerEventType.touchEnd), isEmpty);
    await tester.pumpWidget(const SizedBox());
    await tester.runAsync(controller.dispose);
  });

  testWidgets('a Flutter popup owns clicks above the platform layer', (
    tester,
  ) async {
    controller = LinuxWebViewController(
      const PlatformWebViewControllerCreationParams(),
    );
    int popupClicks = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Stack(
            children: [
              Positioned.fill(child: _webview(controller)),
              Positioned(
                left: 20,
                top: 20,
                width: 200,
                height: 100,
                child: Material(
                  child: InkWell(
                    onTap: () => popupClicks++,
                    child: const Center(child: Text('Popup above browser')),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump();
    await tester.tap(find.text('Popup above browser'));
    await tester.pump();
    await tester.pump();
    expect(popupClicks, 1);
    expect(_events(calls, LinuxWebViewPointerEventType.touchBegin), isEmpty);
    await tester.tapAt(const Offset(500, 400));
    await tester.pump();
    await tester.pump();
    expect(
      _events(calls, LinuxWebViewPointerEventType.touchBegin),
      hasLength(1),
    );
    expect(_events(calls, LinuxWebViewPointerEventType.touchEnd), hasLength(1));
    await tester.pumpWidget(const SizedBox());
    await tester.runAsync(controller.dispose);
  });
}

/// Builds the Linux implementation without changing global platform dispatch.
Widget _webview(LinuxWebViewController controller) {
  final widget = LinuxWebViewWidget(
    PlatformWebViewWidgetCreationParams(controller: controller),
  );
  return Builder(builder: widget.build);
}

/// Selects the native pointer messages for a specific event kind.
Iterable<MethodCall> _events(
  List<MethodCall> calls,
  LinuxWebViewPointerEventType type,
) => calls.where(
  (call) =>
      call.method == 'dispatchPointerEvent' &&
      (call.arguments as Map)['type'] == type.index,
);
