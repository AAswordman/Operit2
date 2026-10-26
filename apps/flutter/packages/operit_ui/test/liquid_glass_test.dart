import 'dart:io';
import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';

/// Exercises actual GPU shader output and the shared background lifecycle.
void main() {
  testWidgets(
    'lens preserves the center and bends straight edges like liquid 1.1.1',
    (tester) async {
      final key = GlobalKey();
      await tester.pumpWidget(_gradientScene(key));
      final pixels = await _readScene(tester, key);
      expect(_red(pixels, 160, 120), closeTo(128, 2));
      const distance = 10.5;
      final depth = 1 - distance / (160 * 0.25);
      final displacement = (1 - math.sqrt(1 - depth * depth)) * 0.25 * 160;
      final expected = (50.5 + displacement) / 320 * 255;
      expect(_red(pixels, 50, 120), closeTo(expected, 2));
      expect(_red(pixels, 50, 120), greaterThan(50.5 / 320 * 255 + 7));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('each corner uses its own radius and foreground stays sharp', (
    tester,
  ) async {
    final key = GlobalKey();
    await tester.pumpWidget(_gradientScene(key, foreground: true));
    final pixels = await _readScene(tester, key);
    // The rounded top-left corner exposes the source without lens displacement.
    expect(_red(pixels, 42, 42), closeTo(42.5 / 320 * 255, 2));
    // The square top-right corner belongs to the lens and is refracted inward.
    expect(_red(pixels, 277, 46), lessThan(277.5 / 320 * 255 - 5));
    final center = (120 * 320 + 160) * 4;
    expect(pixels.sublist(center, center + 4), <int>[33, 150, 243, 255]);
  });

  testWidgets(
    'moving a lens samples its new source position without repainting the source',
    (tester) async {
      final key = GlobalKey();
      final left = ValueNotifier<double>(40);
      addTearDown(left.dispose);
      var sourcePaints = 0;
      await tester.pumpWidget(
        _gradientScene(key, left: left, onSourcePaint: () => sourcePaints++),
      );
      final initial = await _readScene(tester, key);
      final initialPaints = sourcePaints;
      left.value = 60;
      await tester.pump();
      final moved = await _readScene(tester, key);
      expect(sourcePaints, initialPaints);
      expect(
        _red(initial, 50, 120),
        closeTo(_red(moved, 70, 120) - 20 / 320 * 255, 2),
      );
    },
  );

  testWidgets('scaled lenses keep their local shape and source coordinates', (
    tester,
  ) async {
    final key = GlobalKey();
    await tester.pumpWidget(_gradientScene(key, scale: 0.75));
    final pixels = await _readScene(tester, key);
    // Local center (120,80) is source position (130,100) after scaling.
    expect(_red(pixels, 130, 100), closeTo(130.5 / 320 * 255, 2));
    expect(_red(pixels, 48, 100), greaterThan(48.5 / 320 * 255 + 5));
    expect(tester.takeException(), isNull);
  });

  for (final lensSize in <Size>[const Size(240, 160), const Size(160, 480)]) {
    for (final radii in <BorderRadius>[
      BorderRadius.zero,
      BorderRadius.circular(8),
      const BorderRadius.only(
        topLeft: Radius.circular(4),
        topRight: Radius.circular(20),
        bottomLeft: Radius.circular(12),
      ),
    ]) {
      testWidgets('corner refraction stays continuous for $lensSize with $radii', (
        tester,
      ) async {
        final key = GlobalKey();
        await tester.pumpWidget(
          _gradientScene(
            key,
            lensSize: lensSize,
            sceneHeight: lensSize.height + 80,
            borderRadius: radii,
          ),
        );
        final pixels = await _readScene(tester, key);
        for (final right in <bool>[false, true]) {
          for (final bottom in <bool>[false, true]) {
            for (var distance = 8; distance <= 32; distance++) {
              // Adjacent samples straddle the equal-distance line between edges.
              final x1 = right
                  ? 39 + lensSize.width.toInt() - distance
                  : 40 + distance;
              final y1 = bottom
                  ? 39 + lensSize.height.toInt() - distance - 1
                  : 40 + distance + 1;
              final x2 = right ? x1 - 1 : x1 + 1;
              final y2 = bottom ? y1 + 1 : y1 - 1;
              expect(
                (_red(pixels, x1, y1) - _red(pixels, x2, y2)).abs(),
                lessThanOrEqualTo(2),
                reason:
                    'No diagonal crease at right=$right bottom=$bottom distance=$distance',
              );
            }
          }
        }
        expect(tester.takeException(), isNull);
      });
    }
  }
  testWidgets(
    'clear glass preserves one-pixel background detail at the center',
    (tester) async {
      final key = GlobalKey();
      await tester.pumpWidget(_gradientScene(key, detail: true));
      final pixels = await _readScene(tester, key);
      for (var x = 150; x < 170; x++) {
        expect(_red(pixels, x, 120), x.isEven ? 0 : 255);
      }
      await tester.pumpAndSettle();
      expect(tester.binding.hasScheduledFrame, isFalse);
    },
  );

  for (final ratio in <double>[1, 2, 3]) {
    testWidgets('optical coordinates remain correct at pixel ratio $ratio', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1920, 1440);
      tester.view.devicePixelRatio = ratio;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final key = GlobalKey();
      await tester.pumpWidget(_gradientScene(key));
      final pixels = await _readScene(tester, key);
      expect(_red(pixels, 160, 120), closeTo(128, 2));
      expect(_red(pixels, 50, 120), greaterThan(50.5 / 320 * 255 + 7));
      tester.view.devicePixelRatio = ratio + 0.5;
      await tester.pump();
      final changedPixels = await _readScene(tester, key);
      expect(_red(changedPixels, 160, 120), closeTo(128, 2));
      expect(_red(changedPixels, 50, 120), greaterThan(50.5 / 320 * 255 + 7));
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('a repainted background updates every glass surface', (
    tester,
  ) async {
    final color = ValueNotifier<Color>(const Color(0xFFFF0000));
    addTearDown(color.dispose);
    final key = GlobalKey();
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Center(
          child: RepaintBoundary(
            key: key,
            child: SizedBox(
              width: 320,
              height: 240,
              child: LiquidGlassBackdrop(
                background: ValueListenableBuilder<Color>(
                  valueListenable: color,
                  builder: (context, value, child) => ColoredBox(color: value),
                ),
                child: const Row(
                  children: <Widget>[
                    Expanded(
                      child: LiquidGlassSurface(
                        borderRadius: BorderRadius.zero,
                        tint: Colors.transparent,
                        saturation: 1,
                        contrast: 1,
                        child: SizedBox.expand(),
                      ),
                    ),
                    Expanded(
                      child: LiquidGlassSurface(
                        borderRadius: BorderRadius.zero,
                        tint: Colors.transparent,
                        saturation: 1,
                        contrast: 1,
                        child: SizedBox.expand(),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
    var pixels = await _readScene(tester, key);
    expect(_red(pixels, 80, 120), 255);
    expect(_red(pixels, 240, 120), 255);
    color.value = const Color(0xFF0000FF);
    await tester.pump();
    pixels = await _readScene(tester, key);
    expect(_red(pixels, 80, 120), 0);
    expect(_red(pixels, 240, 120), 0);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  testWidgets('post-frame source invalidation never captures dirty paint', (
    tester,
  ) async {
    await tester.runAsync(prepareLiquidGlass);
    final key = GlobalKey();
    await tester.pumpWidget(_gradientScene(key));
    final source = tester.renderObject<RenderCustomPaint>(
      find.byWidgetPredicate(
        (widget) =>
            widget is CustomPaint && widget.painter is _CoordinateBackground,
      ),
    );
    source.markNeedsPaint();
    tester.binding.addPostFrameCallback((_) => source.markNeedsPaint());
    await tester.pump();
    expect(tester.takeException(), isNull);
    await tester.pump();
    final pixels = await _readComposedScene(tester, key);
    expect(_red(pixels, 50, 120), greaterThan(50.5 / 320 * 255 + 7));
    expect(tester.binding.hasScheduledFrame, isFalse);
  });

  testWidgets('both lenses update in the first submitted background frame', (
    tester,
  ) async {
    await tester.runAsync(prepareLiquidGlass);
    final color = ValueNotifier<Color>(const Color(0xFFFF0000));
    addTearDown(color.dispose);
    final key = GlobalKey();
    await tester.pumpWidget(
      _solidScene(
        key,
        background: ValueListenableBuilder<Color>(
          valueListenable: color,
          builder: (context, value, child) => ColoredBox(color: value),
        ),
      ),
    );
    var pixels = await _readComposedScene(tester, key);
    expect(_red(pixels, 80, 120), 255);
    expect(_red(pixels, 240, 120), 255);
    color.value = const Color(0xFF0000FF);
    await tester.pump();
    pixels = await _readComposedScene(tester, key);
    expect(_red(pixels, 80, 120), 0);
    expect(_red(pixels, 240, 120), 0);
    expect(tester.binding.hasScheduledFrame, isFalse);
  });

  testWidgets(
    'retained background layers refresh lenses without source paint',
    (tester) async {
      await tester.runAsync(prepareLiquidGlass);
      final color = ValueNotifier<Color>(const Color(0xFFFF0000));
      addTearDown(color.dispose);
      final key = GlobalKey();
      var sourcePaints = 0;
      await tester.pumpWidget(
        _solidScene(
          key,
          background: _CompositionBackground(color, () => sourcePaints++),
        ),
      );
      await tester.pump();
      final initial = await _readComposedScene(tester, key);
      expect(_red(initial, 80, 120), 255);
      final initialPaints = sourcePaints;
      color.value = const Color(0xFF0000FF);
      // Submit a frame without invalidating any source or foreground pictures.
      tester.binding.scheduleFrame();
      await tester.pump();
      final updated = await _readComposedScene(tester, key);
      expect(sourcePaints, initialPaints);
      expect(_red(updated, 80, 120), 0);
      expect(_red(updated, 240, 120), 0);
      expect(tester.binding.hasScheduledFrame, isFalse);
    },
  );

  testWidgets('offstage scenes resume with current source pixels', (
    tester,
  ) async {
    await tester.runAsync(prepareLiquidGlass);
    final visible = ValueNotifier<bool>(true);
    final color = ValueNotifier<Color>(const Color(0xFFFF0000));
    addTearDown(visible.dispose);
    addTearDown(color.dispose);
    final key = GlobalKey();
    await tester.pumpWidget(
      ValueListenableBuilder<bool>(
        valueListenable: visible,
        child: _solidScene(
          key,
          background: ValueListenableBuilder<Color>(
            valueListenable: color,
            builder: (context, value, child) => ColoredBox(color: value),
          ),
        ),
        builder: (context, value, child) =>
            Offstage(offstage: !value, child: child),
      ),
    );
    visible.value = false;
    await tester.pump();
    color.value = const Color(0xFF0000FF);
    await tester.pump();
    expect(tester.takeException(), isNull);
    visible.value = true;
    await tester.pump();
    final pixels = await _readComposedScene(tester, key);
    expect(_red(pixels, 80, 120), 0);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump();
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'playing texture frames compose without repaint and stop when paused',
    (tester) async {
      await tester.runAsync(prepareLiquidGlass);
      final color = ValueNotifier<Color>(const Color(0xFFFF0000));
      final playing = ValueNotifier<bool>(true);
      final enabled = ValueNotifier<bool>(true);
      addTearDown(color.dispose);
      addTearDown(playing.dispose);
      addTearDown(enabled.dispose);
      final key = GlobalKey();
      var sourcePaints = 0;
      final background = ValueListenableBuilder<bool>(
        valueListenable: playing,
        child: _CompositionBackground(color, () => sourcePaints++),
        builder: (context, active, child) =>
            LiquidGlassSourceAnimation(active: active, child: child!),
      );
      await tester.pumpWidget(
        ValueListenableBuilder<bool>(
          valueListenable: enabled,
          child: _solidScene(key, background: background),
          builder: (context, value, child) =>
              TickerMode(enabled: value, child: child!),
        ),
      );
      await tester.pump(const Duration(milliseconds: 16));
      final initialPaints = sourcePaints;
      expect(tester.binding.hasScheduledFrame, isTrue);
      color.value = const Color(0xFF0000FF);
      await tester.pump(const Duration(milliseconds: 16));
      final pixels = await _readComposedScene(tester, key);
      expect(_red(pixels, 80, 120), 0);
      expect(_red(pixels, 240, 120), 0);
      expect(sourcePaints, initialPaints);
      enabled.value = false;
      await tester.pump();
      await tester.pump();
      expect(tester.binding.hasScheduledFrame, isFalse);
      enabled.value = true;
      await tester.pump();
      await tester.pump();
      expect(tester.binding.hasScheduledFrame, isTrue);
      playing.value = false;
      await tester.pump();
      await tester.pump();
      expect(tester.binding.hasScheduledFrame, isFalse);
      await tester.pumpWidget(
        _solidScene(key, background: background, showSurfaces: false),
      );
      playing.value = true;
      await tester.pump();
      await tester.pump();
      expect(tester.binding.hasScheduledFrame, isFalse);
      await tester.pumpWidget(_solidScene(key, background: background));
      await tester.pump();
      expect(tester.binding.hasScheduledFrame, isTrue);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();
      expect(tester.binding.hasScheduledFrame, isFalse);
      expect(tester.takeException(), isNull);
    },
  );
  for (final entry in <ThemeGlassSurfaceLayer, double>{
    ThemeGlassSurfaceLayer.panel: 16,
    ThemeGlassSurfaceLayer.card: 14,
    ThemeGlassSurfaceLayer.control: 10,
  }.entries) {
    testWidgets(
      'general ${entry.key.name} surfaces retain their original frost',
      (tester) async {
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: ThemeGlassSurface(
              transparentSurface: true,
              color: Colors.white,
              layer: entry.key,
              clip: false,
              child: const SizedBox(width: 100, height: 80),
            ),
          ),
        );
        final filter = tester.widget<BackdropFilter>(
          find.byType(BackdropFilter),
        );
        expect(
          filter.filter,
          ui.ImageFilter.blur(sigmaX: entry.value, sigmaY: entry.value),
        );
        expect(
          find.ancestor(
            of: find.byType(BackdropFilter),
            matching: find.byType(ClipRect),
          ),
          findsOneWidget,
        );
        expect(find.byType(LiquidGlassSurface), findsNothing);
        expect(tester.takeException(), isNull);
      },
    );
  }
  testWidgets(
    'renders a comparison of the previous blur and the ported liquid optics',
    (tester) async {
      await tester.runAsync(() async {
        expect(
          await rootBundle.loadString(
            'packages/operit_ui/assets/licenses/liquid/NOTICE',
          ),
          isNotEmpty,
        );
        expect(
          await rootBundle.loadString(
            'packages/operit_ui/assets/licenses/liquid/LICENSE',
          ),
          isNotEmpty,
        );
        final loader = FontLoader('LiquidComparison')
          ..addFont(
            rootBundle.load(
              'packages/operit_ui/assets/fonts/cascadia-mono.ttf',
            ),
          );
        await loader.load();
      });
      final key = GlobalKey();
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: RepaintBoundary(
              key: key,
              child: SizedBox(
                width: 680,
                height: 420,
                child: LiquidGlassBackdrop(
                  background: CustomPaint(painter: _ComparisonBackground()),
                  child: Stack(
                    children: <Widget>[
                      const Positioned(
                        left: 30,
                        top: 24,
                        child: Text(
                          'PREVIOUS: BLUR + TINT',
                          style: TextStyle(
                            fontFamily: 'LiquidComparison',
                            color: Colors.white,
                            fontSize: 13,
                          ),
                        ),
                      ),
                      const Positioned(
                        left: 360,
                        top: 24,
                        child: Text(
                          'LIQUID: PORTED OPTICS',
                          style: TextStyle(
                            fontFamily: 'LiquidComparison',
                            color: Colors.white,
                            fontSize: 13,
                          ),
                        ),
                      ),
                      Positioned(
                        left: 30,
                        top: 90,
                        width: 290,
                        height: 240,
                        child: ClipRRect(
                          borderRadius: BorderRadius.circular(36),
                          child: BackdropFilter(
                            filter: ui.ImageFilter.blur(sigmaX: 14, sigmaY: 14),
                            child: ColoredBox(
                              color: Colors.white.withValues(alpha: 0.16),
                              child: _comparisonContent(),
                            ),
                          ),
                        ),
                      ),
                      Positioned(
                        left: 360,
                        top: 90,
                        width: 290,
                        height: 240,
                        child: LiquidGlassSurface(
                          borderRadius: BorderRadius.circular(36),
                          child: _comparisonContent(),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await _readScene(tester, key);
      await tester.runAsync(() async {
        final image =
            await (key.currentContext!.findRenderObject()!
                    as RenderRepaintBoundary)
                .toImage();
        final png = await image.toByteData(format: ui.ImageByteFormat.png);
        final output = File('.dart_tool/liquid_glass_comparison.png');
        await output.parent.create(recursive: true);
        await output.writeAsBytes(png!.buffer.asUint8List());
        image.dispose();
      });
      expect(tester.takeException(), isNull);
    },
  );
}

/// Builds the same coordinate-encoded source for lens and transform tests.
Widget _gradientScene(
  GlobalKey key, {
  bool foreground = false,
  ValueNotifier<double>? left,
  double scale = 1,
  VoidCallback? onSourcePaint,
  bool detail = false,
  Size lensSize = const Size(240, 160),
  double sceneHeight = 240,
  BorderRadius borderRadius = const BorderRadius.only(
    topLeft: Radius.circular(32),
    bottomRight: Radius.circular(16),
  ),
}) {
  final lens = Transform.scale(
    scale: scale,
    alignment: Alignment.topLeft,
    child: LiquidGlassSurface(
      borderRadius: borderRadius,
      tint: Colors.transparent,
      edge: 0,
      saturation: 1,
      dispersion: 0,
      contrast: 1,
      child: foreground
          ? const Center(
              child: SizedBox(
                width: 20,
                height: 20,
                child: ColoredBox(color: Colors.blue),
              ),
            )
          : const SizedBox.expand(),
    ),
  );
  return Directionality(
    textDirection: TextDirection.ltr,
    child: Center(
      child: RepaintBoundary(
        key: key,
        child: SizedBox(
          width: 320,
          height: sceneHeight,
          child: LiquidGlassBackdrop(
            background: CustomPaint(
              painter: _CoordinateBackground(onSourcePaint, detail: detail),
            ),
            child: Stack(
              children: <Widget>[
                if (left != null)
                  ValueListenableBuilder<double>(
                    valueListenable: left,
                    child: lens,
                    builder: (context, value, child) => Positioned(
                      left: value,
                      top: 40,
                      width: lensSize.width,
                      height: lensSize.height,
                      child: child!,
                    ),
                  )
                else
                  Positioned(
                    left: 40,
                    top: 40,
                    width: lensSize.width,
                    height: lensSize.height,
                    child: lens,
                  ),
              ],
            ),
          ),
        ),
      ),
    ),
  );
}

/// Waits for asynchronous asset loading and texture readback before sampling.
Future<Uint8List> _readScene(WidgetTester tester, GlobalKey key) async {
  await tester.runAsync(prepareLiquidGlass);
  await tester.pump();
  await tester.pump();
  return (await tester.runAsync(() async {
    final image =
        await (key.currentContext!.findRenderObject()! as RenderRepaintBoundary)
            .toImage();
    final bytes = await image.toByteData();
    image.dispose();
    return bytes!.buffer.asUint8List();
  }))!;
}

/// Reads the red channel from a fixed-width coordinate-test image.
int _red(Uint8List pixels, int x, int y) => pixels[(y * 320 + x) * 4];

/// Supplies identical foreground content for the visual comparison.
Widget _comparisonContent() => const Padding(
  padding: EdgeInsets.all(24),
  child: Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: <Widget>[
      Text(
        'Liquid glass',
        style: TextStyle(
          fontFamily: 'LiquidComparison',
          color: Colors.white,
          fontSize: 22,
          fontWeight: FontWeight.w600,
        ),
      ),
      SizedBox(height: 14),
      Text(
        'Curved edges.\nClear center.',
        style: TextStyle(
          fontFamily: 'LiquidComparison',
          color: Colors.white,
          fontSize: 15,
          height: 1.5,
        ),
      ),
      Spacer(),
      Row(
        children: <Widget>[
          Icon(Icons.blur_on, color: Colors.white),
          SizedBox(width: 12),
          Text(
            'Shared texture',
            style: TextStyle(
              fontFamily: 'LiquidComparison',
              color: Colors.white,
              fontSize: 14,
            ),
          ),
        ],
      ),
    ],
  ),
);

class _CoordinateBackground extends CustomPainter {
  /// Creates a deterministic red gradient with optional paint accounting.
  _CoordinateBackground(this.onPaint, {required this.detail});

  final VoidCallback? onPaint;
  final bool detail;

  /// Encodes horizontal source coordinates into the red channel.
  @override
  void paint(Canvas canvas, Size size) {
    onPaint?.call();
    if (detail) {
      for (var x = 0; x < size.width; x++) {
        canvas.drawRect(
          Rect.fromLTWH(x.toDouble(), 0, 1, size.height),
          Paint()..color = x.isEven ? Colors.black : Colors.white,
        );
      }
      return;
    }
    canvas.drawRect(
      Offset.zero & size,
      Paint()
        ..shader = ui.Gradient.linear(
          Offset.zero,
          Offset(size.width, 0),
          const <Color>[Color(0xFF000000), Color(0xFFFF0000)],
        ),
    );
  }

  /// Repaints only when the paint-accounting owner changes.
  @override
  bool shouldRepaint(_CoordinateBackground oldDelegate) =>
      onPaint != oldDelegate.onPaint || detail != oldDelegate.detail;
}

class _ComparisonBackground extends CustomPainter {
  /// Paints repeated colorful bands so both comparison panels share a pattern.
  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(
      Offset.zero & size,
      Paint()
        ..shader = ui.Gradient.linear(
          Offset.zero,
          Offset(0, size.height),
          const <Color>[Color(0xFF142B59), Color(0xFF634275)],
        ),
    );
    for (var x = -420.0; x < size.width + 420; x += 110) {
      final path = Path()
        ..moveTo(x, 0)
        ..lineTo(x + 45, 0)
        ..lineTo(x + 300, size.height)
        ..lineTo(x + 255, size.height)
        ..close();
      canvas.drawPath(path, Paint()..color = const Color(0xFFED9C89));
    }
    for (var y = 75.0; y < size.height; y += 70) {
      canvas.drawLine(
        Offset(0, y),
        Offset(size.width, y),
        Paint()
          ..color = const Color(0xFF55CECE)
          ..strokeWidth = 4,
      );
    }
  }

  /// Keeps the comparison source static across foreground updates.
  @override
  bool shouldRepaint(_ComparisonBackground oldDelegate) => false;
}

/// Reads the current composed scene without pumping away a one-frame delay.
Future<Uint8List> _readComposedScene(WidgetTester tester, GlobalKey key) async {
  return (await tester.runAsync(() async {
    final boundary =
        key.currentContext!.findRenderObject()! as RenderRepaintBoundary;
    final image = await boundary.toImage();
    final bytes = await image.toByteData();
    image.dispose();
    return bytes!.buffer.asUint8List();
  }))!;
}

/// Builds two retained foreground lenses that sample the same source texture.
Widget _solidScene(
  GlobalKey key, {
  required Widget background,
  bool showSurfaces = true,
}) {
  return Directionality(
    textDirection: TextDirection.ltr,
    child: Center(
      child: RepaintBoundary(
        key: key,
        child: SizedBox(
          width: 320,
          height: 240,
          child: LiquidGlassBackdrop(
            background: background,
            child: showSurfaces
                ? const Row(
                    children: <Widget>[
                      Expanded(
                        child: LiquidGlassSurface(
                          borderRadius: BorderRadius.zero,
                          tint: Colors.transparent,
                          edge: 0,
                          child: SizedBox.expand(),
                        ),
                      ),
                      Expanded(
                        child: LiquidGlassSurface(
                          borderRadius: BorderRadius.zero,
                          tint: Colors.transparent,
                          edge: 0,
                          child: SizedBox.expand(),
                        ),
                      ),
                    ],
                  )
                : const SizedBox.expand(),
          ),
        ),
      ),
    ),
  );
}

class _CompositionBackground extends LeafRenderObjectWidget {
  /// Creates a source that changes during composition without repainting.
  const _CompositionBackground(this.color, this.onPaint);

  final ValueNotifier<Color> color;
  final VoidCallback onPaint;

  /// Creates a retained source whose pixels update independently of Dart paint.
  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderCompositionBackground(color, onPaint);
}

class _RenderCompositionBackground extends RenderBox {
  /// Stores the external source state and its paint instrumentation.
  _RenderCompositionBackground(this.color, this.onPaint);

  final ValueNotifier<Color> color;
  final VoidCallback onPaint;

  /// Gives the source an independently composited layer.
  @override
  bool get alwaysNeedsCompositing => true;

  /// Takes the tight dimensions assigned by the background stack.
  @override
  void performLayout() {
    size = constraints.biggest;
  }

  /// Records the retained layer without subscribing paint to source changes.
  @override
  void paint(PaintingContext context, Offset offset) {
    onPaint();
    final sourceLayer =
        layer as _CompositionColorLayer? ?? _CompositionColorLayer(color);
    sourceLayer.bounds = offset & size;
    layer = sourceLayer;
    context.addLayer(sourceLayer);
  }
}

class _CompositionColorLayer extends ContainerLayer {
  /// Stores the externally updated color for the submitted scene.
  _CompositionColorLayer(this.color);

  final ValueNotifier<Color> color;
  Rect bounds = Rect.zero;

  /// Refreshes the source as a TextureLayer would during scene submission.
  @override
  bool get alwaysNeedsAddToScene => true;

  /// Adds current source pixels without issuing any render-object repaint.
  @override
  void addToScene(ui.SceneBuilder builder) {
    final recorder = ui.PictureRecorder();
    Canvas(recorder).drawRect(bounds, Paint()..color = color.value);
    final picture = recorder.endRecording();
    builder.addPicture(Offset.zero, picture);
    picture.dispose();
  }
}
