import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/layout/ApplicationZoom.dart';
import 'package:operit2/ui/window/DesktopWindowChrome.dart';
import 'package:operit_ui/operit_ui.dart';

const _channel = MethodChannel('operit/window_chrome');
const _contentKey = ValueKey('content');

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final calls = <MethodCall>[];

  setUp(() {
    calls.clear();
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(_channel, (call) async {
          calls.add(call);
          return defaultTargetPlatform == TargetPlatform.macOS ? 28.0 : 0.0;
        });
  });

  tearDown(() {
    debugDefaultTargetPlatformOverride = null;
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(_channel, null);
  });

  testWidgets(
    'macOS reserves controls and releases their inset in full screen',
    (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
      await tester.pumpWidget(_app());
      await tester.pumpAndSettle();
      expect(tester.getTopLeft(find.byKey(_contentKey)).dy, 34);
      expect(find.byType(ThemeDesktopSidebarSurface), findsOneWidget);
      final clip = tester.widget<ClipRRect>(
        find.descendant(
          of: find.byType(DesktopContentPanel),
          matching: find.byType(ClipRRect),
        ),
      );
      expect(clip.borderRadius, BorderRadius.circular(14));
      expect(
        tester.getBottomRight(find.byKey(_contentKey)),
        const Offset(792, 592),
      );

      await _nativeInset(tester, 0);
      expect(tester.getTopLeft(find.byKey(_contentKey)).dy, 6);
      await _nativeInset(tester, 28);
      expect(tester.getTopLeft(find.byKey(_contentKey)).dy, 34);
      expect(tester.takeException(), isNull);
      await _finish(tester);
    },
  );

  testWidgets('caption clearance stays in native points when zoom changes', (
    tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
    for (final zoom in [0.8, 1.0, 1.5]) {
      await tester.pumpWidget(_app(zoom: zoom));
      await tester.pumpAndSettle();
      expect(
        tester.getTopLeft(find.byKey(_contentKey)).dy,
        closeTo(28 + 6 * zoom, 0.01),
      );
    }
    expect(tester.takeException(), isNull);
    await _finish(tester);
  });

  for (final platform in [TargetPlatform.windows, TargetPlatform.linux]) {
    testWidgets(
      '${platform.name} keeps native caption space and follows the app theme',
      (tester) async {
        debugDefaultTargetPlatformOverride = platform;
        await tester.pumpWidget(_app());
        await tester.pumpAndSettle();
        expect(tester.getTopLeft(find.byKey(_contentKey)).dy, 6);
        expect(find.byType(ThemeDesktopSidebarSurface), findsOneWidget);
        expect(
          tester.getBottomRight(find.byKey(_contentKey)),
          const Offset(792, 592),
        );
        expect(calls.last.arguments['dark'], false);
        expect(
          calls.last.arguments['foregroundColor'],
          _lightScheme.onSurface.toARGB32(),
        );

        await tester.pumpWidget(_app(mode: ThemeMode.dark));
        await tester.pumpAndSettle();
        expect(calls.last.arguments['dark'], true);
        expect(
          calls.last.arguments['backgroundColor'],
          _darkScheme.surfaceContainerLow.toARGB32(),
        );
        expect(
          calls.last.arguments['foregroundColor'],
          _darkScheme.onSurface.toARGB32(),
        );
        expect(tester.takeException(), isNull);
        await _finish(tester);
      },
    );
  }

  for (final platform in [TargetPlatform.macOS, TargetPlatform.linux]) {
    testWidgets(
      '${platform.name} system appearance changes update the native caption',
      (tester) async {
        debugDefaultTargetPlatformOverride = platform;
        tester.platformDispatcher.platformBrightnessTestValue =
            Brightness.light;
        addTearDown(tester.platformDispatcher.clearPlatformBrightnessTestValue);
        await tester.pumpWidget(_app(mode: ThemeMode.system));
        await tester.pumpAndSettle();
        expect(calls.last.arguments['dark'], false);
        tester.platformDispatcher.platformBrightnessTestValue = Brightness.dark;
        await tester.pumpAndSettle();
        expect(calls.last.arguments['dark'], true);
        await _finish(tester);
      },
    );
  }

  testWidgets('full screen preserves the display safe area', (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
    tester.view.padding = FakeViewPadding(
      top: 24 * tester.view.devicePixelRatio,
    );
    addTearDown(tester.view.resetPadding);
    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();
    expect(
      MediaQuery.paddingOf(tester.element(find.byKey(_contentKey))).top,
      0,
    );
    await _nativeInset(tester, 0);
    expect(
      MediaQuery.paddingOf(tester.element(find.byKey(_contentKey))).top,
      24,
    );
    await _finish(tester);
  });

  testWidgets('mobile keeps its existing geometry and system safe area', (
    tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();
    expect(calls, isEmpty);
    expect(find.byType(ClipRRect), findsNothing);
    expect(find.byType(ThemeDesktopSidebarSurface), findsNothing);
    expect(
      tester.getRect(find.byKey(_contentKey)),
      const Rect.fromLTWH(0, 0, 800, 600),
    );
    await _finish(tester);
  });
}

final _lightScheme = ColorScheme.fromSeed(seedColor: Colors.blue);
final _darkScheme = ColorScheme.fromSeed(
  seedColor: Colors.blue,
  brightness: Brightness.dark,
);

Widget _app({double zoom = 1, ThemeMode mode = ThemeMode.light}) =>
    DesktopWindowChrome(
      themeMode: mode,
      lightColorScheme: _lightScheme,
      darkColorScheme: _darkScheme,
      child: MaterialApp(
        theme: ThemeData(colorScheme: _lightScheme),
        darkTheme: ThemeData(colorScheme: _darkScheme),
        themeMode: mode,
        home: ApplicationZoomHost(
          zoom: zoom,
          onZoomChanged: (_) {},
          child: DesktopWindowFrame(
            transparentSurface: false,
            zoom: zoom,
            child: DesktopContentPanel(
              child: ColoredBox(
                key: _contentKey,
                color: Colors.white,
                child: const SizedBox.expand(),
              ),
            ),
          ),
        ),
      ),
    );

Future<void> _nativeInset(WidgetTester tester, double inset) async {
  await tester.binding.defaultBinaryMessenger.handlePlatformMessage(
    _channel.name,
    _channel.codec.encodeMethodCall(MethodCall('topInsetChanged', inset)),
    (_) {},
  );
  await tester.pumpAndSettle();
}

Future<void> _finish(WidgetTester tester) async {
  await tester.pumpWidget(const SizedBox.shrink());
  debugDefaultTargetPlatformOverride = null;
}
