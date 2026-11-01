import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test(
    'all bundled pet actions decode actual transparent WebP frames at source timing',
    () async {
      final root = Directory(
        '../../../plugins/packages/external/deepseek_dafeiyu/resources',
      );
      final metadata =
          jsonDecode(
                await File('${root.path}/animation-info.json').readAsString(),
              )
              as Map<String, dynamic>;
      final actions = metadata['actions'] as Map<String, dynamic>;
      for (final entry in actions.entries) {
        final bytes = await File(
          '${root.path}/${entry.key}.webp',
        ).readAsBytes();
        final codec = await ui.instantiateImageCodec(bytes);
        try {
          expect(codec.frameCount, entry.value['frames'], reason: entry.key);
          expect(codec.frameCount, greaterThan(200));
          expect(
            codec.repetitionCount,
            entry.value['loop'] == true ? -1 : 0,
            reason: entry.key,
          );
          var elapsedMs = 0;
          final samples = <int>{};
          for (var index = 0; index < codec.frameCount; index++) {
            final frame = await codec.getNextFrame();
            try {
              expect([
                frame.image.width,
                frame.image.height,
              ], metadata['canvas']);
              elapsedMs += frame.duration.inMilliseconds;
              if (index % 60 == 0) {
                final rgba = (await frame.image.toByteData(
                  format: ui.ImageByteFormat.rawRgba,
                ))!;
                final pixels = rgba.buffer.asUint8List(
                  rgba.offsetInBytes,
                  rgba.lengthInBytes,
                );
                expect(
                  pixels[3],
                  0,
                  reason: '${entry.key}: transparent canvas corner',
                );
                samples.add(Object.hashAll(pixels));
              }
            } finally {
              frame.image.dispose();
            }
          }
          expect(elapsedMs, entry.value['duration_ms'], reason: entry.key);
          expect(
            samples.length,
            greaterThanOrEqualTo(4),
            reason: '${entry.key}: distinct decoded action frames',
          );
        } finally {
          codec.dispose();
        }
      }
    },
  );
}
