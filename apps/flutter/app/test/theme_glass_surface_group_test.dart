import 'dart:math' as math;
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';

/// Verifies shared frost identities preserve the original composited output.
void main() {
  testWidgets('matching frost filters share identities by blur strength', (
    tester,
  ) async {
    final captureKey = GlobalKey();
    await _pumpScene(tester, captureKey, _surfaceRows());
    final original = await _pixels(tester, captureKey);
    await _pumpScene(
      tester,
      captureKey,
      ThemeGlassSurfaceGroup(child: _surfaceRows()),
    );
    final keys = <BackdropKey>[];
    for (final layer in ThemeGlassSurfaceLayer.values) {
      final filters = tester
          .widgetList<BackdropFilter>(
            find.descendant(
              of: find.byKey(ValueKey(layer)),
              matching: find.byType(BackdropFilter),
            ),
          )
          .toList();
      expect(filters, hasLength(2));
      expect(filters.first.backdropGroupKey, isNotNull);
      expect(
        filters.last.backdropGroupKey,
        same(filters.first.backdropGroupKey),
      );
      keys.add(filters.first.backdropGroupKey!);
    }
    expect(keys.toSet(), hasLength(3));
    _expectSamePixels(original, await _pixels(tester, captureKey));
    await tester.pump();
    expect(
      tester
          .widgetList<BackdropFilter>(find.byType(BackdropFilter))
          .map((filter) => filter.backdropGroupKey)
          .toSet(),
      keys.toSet(),
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('nested frost samples its parent and shares only with siblings', (
    tester,
  ) async {
    final captureKey = GlobalKey();
    final content = _nestedSurfaces();
    await _pumpScene(tester, captureKey, content);
    final original = await _pixels(tester, captureKey);
    await _pumpScene(
      tester,
      captureKey,
      ThemeGlassSurfaceGroup(child: content),
    );
    final filters = tester
        .widgetList<BackdropFilter>(find.byType(BackdropFilter))
        .toList();
    expect(filters, hasLength(3));
    expect(
      filters.map((filter) => filter.backdropGroupKey),
      everyElement(isNotNull),
    );
    expect(filters[1].backdropGroupKey, same(filters[2].backdropGroupKey));
    expect(filters[0].backdropGroupKey, isNot(filters[1].backdropGroupKey));
    _expectSamePixels(original, await _pixels(tester, captureKey));
    expect(tester.takeException(), isNull);
  });

  testWidgets('overlapping pages keep independent backdrop identities', (
    tester,
  ) async {
    final captureKey = GlobalKey();
    await _pumpScene(tester, captureKey, _overlappingPages(grouped: false));
    final original = await _pixels(tester, captureKey);
    await _pumpScene(tester, captureKey, _overlappingPages(grouped: true));
    final filters = tester
        .widgetList<BackdropFilter>(find.byType(BackdropFilter))
        .toList();
    expect(filters, hasLength(2));
    expect(
      filters.map((filter) => filter.backdropGroupKey),
      everyElement(isNotNull),
    );
    expect(filters[0].backdropGroupKey, isNot(filters[1].backdropGroupKey));
    _expectSamePixels(original, await _pixels(tester, captureKey));
    expect(tester.takeException(), isNull);
  });

  for (final direction in AxisDirection.values) {
    testWidgets(
      'stretch and frost stay live in $direction, including interrupted release',
      (tester) async {
        final captureKey = GlobalKey();
        final controller = ScrollController();
        addTearDown(controller.dispose);
        final axis = axisDirectionToAxis(direction);
        final sign = axisDirectionIsReversed(direction) ? -1.0 : 1.0;
        final pull = axis == Axis.vertical
            ? Offset(0, sign * 45)
            : Offset(sign * 45, 0);
        await _pumpScene(
          tester,
          captureKey,
          Theme(
            data: ThemeData(platform: TargetPlatform.android),
            child: ScrollConfiguration(
              behavior: const MaterialScrollBehavior(),
              child: ThemeGlassSurfaceGroup(
                child: ListView.builder(
                  controller: controller,
                  scrollDirection: axis,
                  reverse: axisDirectionIsReversed(direction),
                  itemExtent: 90,
                  itemCount: 12,
                  itemBuilder: (context, index) => Padding(
                    padding: const EdgeInsets.all(8),
                    child: ThemeGlassSurface(
                      transparentSurface: true,
                      color: Colors.white,
                      borderRadius: BorderRadius.circular(8),
                      child: const SizedBox.expand(),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        expect(find.byType(ThemeGlassStretchIndicator), findsOneWidget);
        final list = find.byType(ListView);
        var gesture = await tester.startGesture(tester.getCenter(list));
        for (var frame = 0; frame < 3; frame++) {
          await gesture.moveBy(pull);
          await tester.pump(const Duration(milliseconds: 16));
          await _expectMovingFrost(tester, captureKey);
        }
        final stretch = _stretchTransform(tester);
        expect(
          stretch.transform.entry(
            axis == Axis.horizontal ? 0 : 1,
            axis == Axis.horizontal ? 0 : 1,
          ),
          greaterThan(1),
        );
        expect(stretch.filterQuality, isNull);
        expect(controller.offset, 0);
        await gesture.up();
        await tester.pump(const Duration(milliseconds: 16));
        await _expectMovingFrost(tester, captureKey);

        // A new pull must continue the live stretch while its spring is returning.
        gesture = await tester.startGesture(tester.getCenter(list));
        await gesture.moveBy(pull);
        await tester.pump(const Duration(milliseconds: 16));
        await gesture.moveBy(pull);
        await tester.pump(const Duration(milliseconds: 16));
        await _expectMovingFrost(tester, captureKey);
        await gesture.up();
        await tester.pumpAndSettle();
        expect(_stretchTransform(tester).transform, Matrix4.identity());

        // A fling into the other edge must also animate live frosted content.
        controller.jumpTo(controller.position.maxScrollExtent - 10);
        await tester.pump();
        await tester.fling(list, -pull * 3, 1500);
        for (var frame = 0; frame < 4; frame++) {
          await tester.pump(const Duration(milliseconds: 16));
          await _expectMovingFrost(tester, captureKey);
        }
        await tester.pumpAndSettle();
        expect(_stretchTransform(tester).transform, Matrix4.identity());
        await tester.pumpWidget(const SizedBox.shrink());
        expect(tester.takeException(), isNull);
      },
    );
  }
}

/// Finds the live transform owned by the scroll-edge indicator.
Transform _stretchTransform(WidgetTester tester) {
  return tester.widget<Transform>(
    find
        .descendant(
          of: find.byType(ThemeGlassStretchIndicator),
          matching: find.byType(Transform),
        )
        .first,
  );
}

/// Checks that a visible surface retains blur while its geometry is stretching.
Future<void> _expectMovingFrost(
  WidgetTester tester,
  GlobalKey captureKey,
) async {
  final boundary = tester.renderObject<RenderRepaintBoundary>(
    find.byKey(captureKey),
  );
  final viewport = tester.getRect(find.byKey(captureKey)).deflate(12);
  for (final element in find.byType(ThemeGlassSurface).evaluate()) {
    final rect = tester
        .getRect(find.byWidget(element.widget))
        .deflate(14)
        .intersect(viewport);
    if (rect.width > 10 && rect.height > 10) {
      final local = boundary.globalToLocal(rect.center);
      final pixels = await _pixels(tester, captureKey);
      final offset = (local.dy.toInt() * 320 + local.dx.toInt()) * 4;
      for (var sample = 0; sample < 5; sample++) {
        expect(pixels[offset + sample * 4], inInclusiveRange(60, 195));
      }
      return;
    }
  }
  fail('Expected a visible frosted surface during stretch.');
}

/// Builds two disjoint surfaces for every supported frost strength.
Widget _surfaceRows() {
  return Column(
    children: [
      for (final layer in ThemeGlassSurfaceLayer.values)
        SizedBox(
          key: ValueKey(layer),
          height: 70,
          child: Row(
            children: [
              for (var index = 0; index < 2; index++)
                Expanded(
                  child: Padding(
                    padding: const EdgeInsets.all(8),
                    child: ThemeGlassSurface(
                      transparentSurface: true,
                      color: Colors.white,
                      layer: layer,
                      borderRadius: BorderRadius.circular(8),
                      child: const SizedBox.expand(),
                    ),
                  ),
                ),
            ],
          ),
        ),
    ],
  );
}

/// Builds nested surfaces with the same blur radius to expose invalid sharing.
Widget _nestedSurfaces() {
  return Padding(
    padding: const EdgeInsets.all(12),
    child: ThemeGlassSurface(
      transparentSurface: true,
      color: Colors.white,
      child: Column(
        children: [
          for (var index = 0; index < 2; index++)
            Expanded(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: ThemeGlassSurface(
                  transparentSurface: true,
                  color: Colors.blue,
                  child: const SizedBox.expand(),
                ),
              ),
            ),
        ],
      ),
    ),
  );
}

/// Models two pages whose frosted rectangles overlap during a transition.
Widget _overlappingPages({required bool grouped}) {
  return Stack(
    children: [
      for (var index = 0; index < 2; index++)
        Positioned(
          left: 16 + index * 70,
          top: 16 + index * 40,
          width: 210,
          height: 150,
          child: grouped
              ? ThemeGlassSurfaceGroup(child: _pageSurface(index))
              : _pageSurface(index),
        ),
    ],
  );
}

/// Gives each page a distinct tint while keeping the same blur strength.
Widget _pageSurface(int index) {
  return ThemeGlassSurface(
    transparentSurface: true,
    color: index == 0 ? Colors.white : Colors.blue,
    borderRadius: BorderRadius.circular(12),
    child: const SizedBox.expand(),
  );
}

/// Installs a deterministic high-contrast background behind the frost surfaces.
Future<void> _pumpScene(
  WidgetTester tester,
  GlobalKey captureKey,
  Widget child,
) async {
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: Center(
        child: RepaintBoundary(
          key: captureKey,
          child: SizedBox(
            width: 320,
            height: 240,
            child: Stack(
              fit: StackFit.expand,
              children: [
                const CustomPaint(painter: _StripePainter()),
                child,
              ],
            ),
          ),
        ),
      ),
    ),
  );
}

/// Captures actual raster output including background filters and tint blending.
Future<Uint8List> _pixels(WidgetTester tester, GlobalKey captureKey) async {
  final boundary = tester.renderObject<RenderRepaintBoundary>(
    find.byKey(captureKey),
  );
  final pixels = await tester.runAsync(() async {
    final image = await boundary.toImage();
    try {
      final data = (await image.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!;
      return data.buffer.asUint8List(data.offsetInBytes, data.lengthInBytes);
    } finally {
      image.dispose();
    }
  });
  return pixels!;
}

/// Allows only rounding differences between shared and independent rendering.
void _expectSamePixels(Uint8List original, Uint8List grouped) {
  expect(grouped.length, original.length);
  var maxDifference = 0;
  for (var index = 0; index < original.length; index++) {
    maxDifference = math.max(
      maxDifference,
      (original[index] - grouped[index]).abs(),
    );
  }
  expect(maxDifference, lessThanOrEqualTo(2));
}

class _StripePainter extends CustomPainter {
  /// Creates a pattern with both fine and coarse detail for nested blur tests.
  const _StripePainter();

  /// Draws mixed stripe widths that reveal changes in the effective blur radius.
  @override
  void paint(Canvas canvas, Size size) {
    for (var x = 0.0; x < size.width; x += 4) {
      final white = (x / 4).toInt().isEven != (x / 40).toInt().isEven;
      canvas.drawRect(
        Rect.fromLTWH(x, 0, 4, size.height),
        Paint()..color = white ? Colors.white : Colors.black,
      );
    }
  }

  /// Retains the fixed source while the grouping structure changes.
  @override
  bool shouldRepaint(covariant _StripePainter oldDelegate) => false;
}
