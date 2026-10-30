import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspacePanelSurface.dart';

void main() {
  for (final brightness in Brightness.values) {
    for (final glass in [false, true]) {
      for (final direction in TextDirection.values) {
        testWidgets(
          'workspace paints only its leading divider: $brightness, glass $glass, $direction',
          (tester) async {
            final captureKey = GlobalKey();
            await tester.pumpWidget(
              MaterialApp(
                theme: ThemeData(
                  colorScheme: ColorScheme.fromSeed(
                    seedColor: Colors.blue,
                    brightness: brightness,
                  ),
                ),
                home: Directionality(
                  textDirection: direction,
                  child: Center(
                    child: RepaintBoundary(
                      key: captureKey,
                      child: SizedBox(
                        width: 240,
                        height: 180,
                        child: WorkspacePanelSurface(
                          transparentSurface: glass,
                          child: const SizedBox.expand(),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            );
            await tester.pumpAndSettle();
            expect(tester.takeException(), isNull);

            // Read the actual painted edges. This catches the BorderDirectional
            // + borderRadius assertion and a reintroduced all-around frame.
            final boundary =
                captureKey.currentContext!.findRenderObject()
                    as RenderRepaintBoundary;
            final pixels = await tester.runAsync(() async {
              final image = await boundary.toImage(pixelRatio: 1);
              try {
                final bytes = await image.toByteData(
                  format: ui.ImageByteFormat.rawRgba,
                );
                return bytes!.buffer
                    .asUint32List(bytes.offsetInBytes, bytes.lengthInBytes ~/ 4)
                    .toList();
              } finally {
                image.dispose();
              }
            });
            int pixel(int x, int y) => pixels![y * 240 + x];
            final leading = direction == TextDirection.ltr ? 0 : 239;
            final trailing = direction == TextDirection.ltr ? 239 : 0;
            final interior = pixel(120, 90);
            expect(pixel(leading, 90), isNot(interior));
            expect(pixel(trailing, 90), interior);
            expect(pixel(120, 0), interior);
            expect(pixel(120, 179), interior);
          },
        );
      }
    }
  }
}
