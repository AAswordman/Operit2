import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:xterm/src/ui/line_cache.dart';
import 'package:xterm/src/ui/painter.dart';
import 'package:xterm/xterm.dart';

/// Returns the retained picture without relying on compositor cache heuristics.
ui.Picture pictureOf(OffsetLayer layer) =>
    (layer.firstChild! as PictureLayer).picture!;

/// Builds a painter with the application's smallest supported terminal scale.
TerminalPainter createPainter() => TerminalPainter(
  theme: TerminalThemes.defaultTheme,
  textStyle: const TerminalStyle(fontSize: 7.2),
  textScaler: TextScaler.noScaling,
);

/// Exercises cache reuse, mutation detection, and actual compositing lifecycle.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'equal font configurations preserve cache identity and font features',
    () {
      final first = TerminalStyle.fromTextStyle(
        const TextStyle(
          fontFamily: 'monospace',
          fontFamilyFallback: ['Mono'],
          fontSize: 8,
          fontFeatures: [FontFeature.tabularFigures()],
          letterSpacing: 0.2,
        ),
      );
      final second = first.copyWith(fontFamilyFallback: ['Mono']);
      expect(first, second);
      expect(first.hashCode, second.hashCode);
      expect(second.toTextStyle().fontFeatures, [
        const FontFeature.tabularFigures(),
      ]);
      expect(second.toTextStyle().letterSpacing, 0.2);
      expect(first.copyWith(fontSize: 9), isNot(first));
      final painter = TerminalPainter(
        theme: TerminalThemes.defaultTheme,
        textStyle: first,
        textScaler: TextScaler.noScaling,
      );
      addTearDown(painter.dispose);
      final measuredCell = painter.cellSize;
      painter.textStyle = second;
      expect(identical(painter.textStyle, first), isTrue);
      expect(identical(painter.cellSize, measuredCell), isTrue);
    },
  );

  test(
    'dense viewport retains unchanged row pictures and updates only edited rows',
    () {
      final terminal = Terminal(maxLines: 1000)..resize(180, 80);
      terminal.write(List.filled(70, 'x' * 170).join('\r\n'));
      final painter = createPainter();
      final cache = TerminalLineCache();
      addTearDown(cache.clear);
      addTearDown(painter.dispose);
      final lines = [for (var i = 0; i < 80; i++) terminal.buffer.lines[i]];
      cache.beginFrame();
      final pictures = [
        for (final line in lines) pictureOf(cache.layerFor(line, painter)),
      ];
      cache.endFrame();
      terminal.write('\x1b[10;1HZ');
      cache.beginFrame();
      final next = [
        for (final line in lines) pictureOf(cache.layerFor(line, painter)),
      ];
      cache.endFrame();
      expect(
        [
          for (var i = 0; i < 80; i++)
            if (!identical(pictures[i], next[i])) i,
        ],
        [9],
      );
      expect(cache.length, 80);
      cache.beginFrame();
      cache.layerFor(lines.last, painter);
      cache.endFrame();
      expect(cache.length, 1);
    },
  );

  test(
    'direct cell edits, row resizing, and scrolling invalidate exact rows',
    () {
      final terminal = Terminal()
        ..resize(40, 4)
        ..write('first\r\nsecond');
      final line = terminal.buffer.lines[0];
      final painter = createPainter();
      final cache = TerminalLineCache();
      addTearDown(cache.clear);
      addTearDown(painter.dispose);
      cache.beginFrame();
      final original = pictureOf(cache.layerFor(line, painter));
      cache.endFrame();
      line.data[3] = 'Z'.codeUnitAt(0) | (1 << CellContent.widthShift);
      cache.beginFrame();
      final edited = pictureOf(cache.layerFor(line, painter));
      expect(identical(original, edited), isFalse);
      cache.endFrame();
      line.resize(20);
      cache.beginFrame();
      expect(
        identical(edited, pictureOf(cache.layerFor(line, painter))),
        isFalse,
      );
      cache.endFrame();
      cache.clear();
      expect(cache.length, 0);
    },
  );

  test(
    'background spans preserve wide cells, inverse and underlined spaces',
    () async {
      final terminal = Terminal()..resize(40, 4);
      terminal.write('\x1b[44m中 A\x1b[41m B\x1b[7m C\x1b[0m\x1b[4m \x1b[0m');
      final painter = createPainter();
      addTearDown(painter.dispose);
      final line = terminal.buffer.lines[0];
      final width = (painter.cellSize.width * 40 + 2).ceil();
      final height = (painter.cellSize.height * 2).ceil();
      final images = <ui.Image>[];
      for (final batched in [false, true]) {
        final recorder = ui.PictureRecorder();
        final canvas = Canvas(recorder);
        if (batched) {
          painter.paintLine(canvas, Offset.zero, line);
        } else {
          final cell = CellData.empty();
          // Establish the background plane before foreground glyphs, including
          // glyphs that reach a neighbouring cell's fractional pixel boundary.
          for (var column = 0; column < line.length; column++) {
            line.getCellData(column, cell);
            painter.paintCellBackground(
              canvas,
              Offset(column * painter.cellSize.width, 0),
              cell,
            );
            if (line.getWidth(column) == 2) column++;
          }
          for (var column = 0; column < line.length; column++) {
            line.getCellData(column, cell);
            painter.paintCellForeground(
              canvas,
              Offset(column * painter.cellSize.width, 0),
              cell,
            );
            if (line.getWidth(column) == 2) column++;
          }
        }
        final picture = recorder.endRecording();
        images.add(await picture.toImage(width, height));
        picture.dispose();
      }
      for (final image in images) {
        addTearDown(image.dispose);
      }
      expect(
        (await images[1].toByteData())!.buffer.asUint8List(),
        (await images[0].toByteData())!.buffer.asUint8List(),
      );
    },
  );

  testWidgets(
    'dense terminal survives output, scrolling, style rebuild and disposal',
    (tester) async {
      final terminal = Terminal(maxLines: 1000);
      final scroll = ScrollController();
      addTearDown(scroll.dispose);
      await tester.pumpWidget(
        MaterialApp(
          home: TerminalView(
            terminal,
            scrollController: scroll,
            textStyle: const TerminalStyle(fontSize: 7.2),
          ),
        ),
      );
      terminal.write(
        List.filled(200, '\x1b[44mhello 中 123\x1b[0m').join('\r\n'),
      );
      await tester.pump();
      terminal.write('\rupdated');
      await tester.pump();
      scroll.jumpTo(0);
      await tester.pump();
      await tester.pumpWidget(
        MaterialApp(
          home: TerminalView(
            terminal,
            scrollController: scroll,
            textStyle: TerminalStyle.fromTextStyle(
              const TextStyle(fontSize: 7.2),
            ),
          ),
        ),
      );
      await tester.pump();
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );
}
