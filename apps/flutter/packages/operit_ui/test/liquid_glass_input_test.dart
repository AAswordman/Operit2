import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';

/// Verifies vertical source alignment at real composer positions and densities.
void main() {
  for (final style in <String>[
    ThemePreferenceDefaults.INPUT_STYLE_AGENT,
    ThemePreferenceDefaults.INPUT_STYLE_CLASSIC,
  ]) {
    for (final ratio in <double>[1, 3]) {
      testWidgets('$style samples beneath a bottom composer at DPR $ratio', (
        tester,
      ) async {
        tester.view.physicalSize = Size(400 * ratio, 800 * ratio);
        tester.view.devicePixelRatio = ratio;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        await tester.runAsync(prepareLiquidGlass);
        final sceneKey = GlobalKey();
        final inputKey = GlobalKey();
        final bottom = ValueNotifier<double>(24);
        addTearDown(bottom.dispose);
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: MediaQuery(
              data: const MediaQueryData(size: Size(400, 800)),
              child: Center(
                child: RepaintBoundary(
                  key: sceneKey,
                  child: SizedBox(
                    width: 320,
                    height: 720,
                    child: LiquidGlassBackdrop(
                      background: CustomPaint(painter: _SourceCoordinates()),
                      child: ValueListenableBuilder<double>(
                        valueListenable: bottom,
                        child: RepaintBoundary(
                          child: ThemeComposerCard(
                            inputStyle: style,
                            floating: true,
                            transparentSurface: true,
                            child: SizedBox(key: inputKey, height: 60),
                          ),
                        ),
                        builder: (context, value, child) => Stack(
                          children: <Widget>[
                            Positioned(
                              left: 8,
                              right: 8,
                              bottom: value,
                              child: child!,
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        for (final offset in <double>[600, 24, 260, 24]) {
          bottom.value = offset;
          await tester.pump();
          final scene = tester.renderObject<RenderRepaintBoundary>(
            find.byKey(sceneKey),
          );
          final input = tester.renderObject<RenderBox>(find.byKey(inputKey));
          final center = input.localToGlobal(
            input.size.center(Offset.zero),
            ancestor: scene,
          );
          final pixels = await tester.runAsync(() async {
            final image = await scene.toImage();
            final bytes = await image.toByteData();
            image.dispose();
            return bytes!.buffer.asUint8List();
          });
          final index = (center.dy.floor() * 320 + center.dx.floor()) * 4;
          const tint = 9 / 255;
          final expectedGreen =
              (center.dy.floor() + 0.5) / 720 * 255 * (1 - tint) + 9;
          expect(
            pixels![index + 1],
            closeTo(expectedGreen, 2),
            reason: 'bottom=$offset center=$center',
          );
          expect(tester.takeException(), isNull);
        }
      });
    }
  }
}

class _SourceCoordinates extends CustomPainter {
  /// Encodes vertical source coordinates in green to expose inverted sampling.
  @override
  void paint(Canvas canvas, Size size) {
    for (var y = 0; y < size.height; y++) {
      canvas.drawRect(
        Rect.fromLTWH(0, y.toDouble(), size.width, 1),
        Paint()
          ..color = Color.fromARGB(
            255,
            0,
            ((y + 0.5) / size.height * 255).round(),
            0,
          ),
      );
    }
  }

  /// Keeps the diagnostic source static while the composer moves.
  @override
  bool shouldRepaint(_SourceCoordinates oldDelegate) => false;
}
