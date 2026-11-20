import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit2/ui/features/packages/screens/compose_dsl/glow_border.dart';

/// Fixture exported from the production TypeScript screen through the real SDK:
/// OPERIT_AUTOMATION_PREVIEW_TREE=apps/flutter/app/test/fixtures/automation_indicator.json
/// node --test tools/tests/automatic_ui_base.test.mjs
void main() {
  final tree =
      (jsonDecode(
                File(
                  'test/fixtures/automation_indicator.json',
                ).readAsStringSync(),
              )
              as Map)
          .cast<String, Object?>();
  const capture = ValueKey('capture');

  Widget screen({bool reducedMotion = false}) => MaterialApp(
    home: MediaQuery(
      data: MediaQueryData(disableAnimations: reducedMotion),
      child: RepaintBoundary(
        key: capture,
        child: ColoredBox(
          color: const Color(0xFF131922),
          child: buildComposeDslLayoutForTest(
            node: tree,
            onTextInput: (_, _) async => null,
            hostContext: ComposeDslWebViewHostContext(
              packageName: 'com.operit.ui_automation',
              routeInstanceId: 'indicator',
              executionContextKey: 'indicator-test',
              dispatchAction: (_, [payload]) async => null,
              runtimeOptionsProvider: () => {},
            ),
            splitMarkdownContent: (_) async => [],
          ),
        ),
      ),
    ),
  );

  Future<Uint8List> glowPixels(
    WidgetTester tester, {
    ui.ImageByteFormat format = ui.ImageByteFormat.rawRgba,
  }) async => (await tester.runAsync(() async {
    final boundary =
        tester.renderObject(
              find.descendant(
                of: find.byType(ComposeGlowBorder),
                matching: find.byType(RepaintBoundary),
              ),
            )
            as RenderRepaintBoundary;
    final image = await boundary.toImage();
    final bytes = (await image.toByteData(
      format: format,
    ))!.buffer.asUint8List();
    image.dispose();
    return bytes;
  }))!;

  for (final scenario in <(Size, double)>[
    (const Size(1080, 2400), 3),
    (const Size(2400, 1080), 3),
    (const Size(3840, 2160), 1.5),
    (const Size(2560, 1600), 2),
  ]) {
    testWidgets(
      'indicator fills physical ${scenario.$1} at DPR ${scenario.$2}',
      (tester) async {
        tester.view.physicalSize = scenario.$1;
        tester.view.devicePixelRatio = scenario.$2;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        await tester.pumpWidget(screen());
        final logical = scenario.$1 / scenario.$2;
        expect(
          tester.getRect(
            find.byKey(const ValueKey<Object?>('automation-glow')),
          ),
          Offset.zero & logical,
        );
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox());
      },
    );
  }

  testWidgets(
    'glow reaches all four edges, feathers inward, animates and leaves center transparent',
    (tester) async {
      tester.view.physicalSize = const Size(960, 640);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      await tester.pumpWidget(screen());
      await tester.pump(const Duration(milliseconds: 100));
      final before = await glowPixels(tester);
      int alpha(int x, int y) => before[(y * 960 + x) * 4 + 3];
      expect(alpha(480, 320), 0);
      for (final point in [
        const Offset(480, 2),
        const Offset(2, 320),
        const Offset(957, 320),
        const Offset(480, 637),
      ]) {
        expect(alpha(point.dx.toInt(), point.dy.toInt()), greaterThan(100));
      }
      expect(alpha(480, 15), greaterThan(5));
      expect(alpha(480, 30), greaterThan(10));
      expect(alpha(480, 120), lessThan(3));
      for (final point in [
        const Offset(0, 0),
        const Offset(959, 0),
        const Offset(0, 639),
        const Offset(959, 639),
      ]) {
        expect(
          alpha(point.dx.toInt(), point.dy.toInt()),
          greaterThan(30),
          reason: 'Square display corners must contain light',
        );
      }
      await tester.pump(const Duration(milliseconds: 900));
      final after = await glowPixels(tester);
      expect(after, isNot(orderedEquals(before)));
      final preview = Platform.environment['OPERIT_AUTOMATION_PREVIEW_PNG'];
      if (preview != null) {
        await tester.runAsync(() async {
          final boundary =
              tester.renderObject(find.byKey(capture)) as RenderRepaintBoundary;
          final image = await boundary.toImage();
          File(preview).writeAsBytesSync(
            (await image.toByteData(
              format: ui.ImageByteFormat.png,
            ))!.buffer.asUint8List(),
          );
          image.dispose();
        });
      }
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'rotation reuses the animated surface and reduced motion stops its ticker',
    (tester) async {
      tester.view.physicalSize = const Size(390, 840);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      await tester.pumpWidget(screen());
      final state = tester.state(find.byType(ComposeGlowBorder));
      tester.view.physicalSize = const Size(840, 390);
      await tester.pump();
      expect(
        tester.getSize(find.byType(ComposeGlowBorder)),
        const Size(840, 390),
      );
      expect(tester.state(find.byType(ComposeGlowBorder)), same(state));
      await tester.pumpWidget(screen(reducedMotion: true));
      await tester.pump();
      final first = await glowPixels(tester);
      await tester.pump(const Duration(seconds: 1));
      final second = await glowPixels(tester);
      expect(first, orderedEquals(second));
      await tester.pumpWidget(const SizedBox());
      expect(tester.takeException(), isNull);
    },
  );
}
