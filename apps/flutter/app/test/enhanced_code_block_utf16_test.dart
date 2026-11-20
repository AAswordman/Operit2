import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/markdown/CanvasMonospaceCodeBlockBody.dart';
import 'package:operit2/ui/common/markdown/EnhancedCodeBlock.dart';

/// Exercises code rendering, scrolling, and text selection.
void main() {
  testWidgets('code selection magnification has a bounded clip', (
    tester,
  ) async {
    const firstLine = 'alpha beta gamma delta';
    await tester.pumpWidget(
      MaterialApp(
        theme: ThemeData.dark(),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 360,
              height: 240,
              child: CanvasMonospaceCodeBlockBody(
                lines: [firstLine, ...List.filled(24, 'more code')],
                autoWrapEnabled: true,
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final firstLineFinder = find.text(firstLine);
    final paragraph = tester.renderObject<RenderParagraph>(
      find.descendant(of: firstLineFinder, matching: find.byType(RichText)),
    );
    final word = paragraph.getBoxesForSelection(
      const TextSelection(baseOffset: 6, extentOffset: 10),
    ).single.toRect();
    await tester.longPressAt(paragraph.localToGlobal(word.center));
    await tester.pumpAndSettle();
    expect(paragraph.selections.single, const TextSelection(
      baseOffset: 6,
      extentOffset: 10,
    ));

    final handle = await tester.startGesture(
      paragraph.localToGlobal(word.bottomRight + const Offset(3, 6)),
    );
    await handle.moveBy(const Offset(24, 0));
    await tester.pump();
    await handle.moveBy(const Offset(25, 0));
    await tester.pumpAndSettle();

    final layers = tester.layers;
    expect(layers.whereType<BackdropFilterLayer>(), hasLength(1));
    final widestPathClip = layers.whereType<ClipPathLayer>().fold<double>(
      0,
      (width, layer) => math.max(width, layer.clipPath!.getBounds().width),
    );
    expect(widestPathClip, lessThanOrEqualTo(360));
    expect(paragraph.selections.single.end, greaterThan(10));

    await handle.up();
    await tester.pumpAndSettle();
    expect(tester.layers.whereType<BackdropFilterLayer>(), isEmpty);
    expect(find.byType(AdaptiveTextSelectionToolbar), findsOneWidget);
    expect(tester.takeException(), isNull);
  }, variant: const TargetPlatformVariant({TargetPlatform.android}));

  testWidgets(
    'long code blocks mount only viewport lines and remain scrollable',
    (tester) async {
      final code = List.generate(
        5000,
        (index) => 'final value$index = $index;',
      ).join('\n');
      final stopwatch = Stopwatch()..start();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: EnhancedCodeBlock(code: code, language: 'dart'),
          ),
        ),
      );
      await tester.pumpAndSettle();
      debugPrint(
        '5000-line code block first layout: ${stopwatch.elapsedMilliseconds} ms',
      );
      final texts = find.descendant(
        of: find.byType(EnhancedCodeBlock),
        matching: find.byType(Text),
      );
      debugPrint(
        '5000-line code block mounted texts: ${texts.evaluate().length}',
      );
      expect(texts.evaluate().length, lessThan(150));
      expect(find.textContaining('value0 ='), findsOneWidget);
      expect(find.textContaining('value4999 ='), findsNothing);
      final list = find.descendant(
        of: find.byType(EnhancedCodeBlock),
        matching: find.byType(ListView),
      );
      await tester.scrollUntilVisible(
        find.textContaining('value4999 ='),
        3000,
        scrollable: find
            .descendant(of: list, matching: find.byType(Scrollable))
            .first,
        maxScrolls: 150,
      );
      expect(find.textContaining('value4999 ='), findsOneWidget);
      await tester.tap(find.byTooltip('Disable word wrap'));
      await tester.pumpAndSettle();
      expect(texts.evaluate().length, lessThan(150));
      expect(tester.takeException(), isNull);
      String? copied;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied = (call.arguments as Map)['text'] as String;
          }
          return null;
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );
      await tester.tap(find.byTooltip('Copy'));
      await tester.pumpAndSettle();
      expect(copied, code);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'code highlights only mounted lines and updates replaced source',
    (tester) async {
      var highlighted = 0;
      final lines = List.generate(5000, (index) => 'line $index');
      Future<void> render(List<String> source) => tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 420),
              child: CanvasMonospaceCodeBlockBody(
                lines: source,
                autoWrapEnabled: true,
                highlightLine: (line) {
                  highlighted++;
                  return TextSpan(text: line);
                },
              ),
            ),
          ),
        ),
      );
      await render(lines);
      expect(highlighted, lessThan(100));
      await render(['replacement']);
      expect(find.text('replacement'), findsOneWidget);
      expect(find.text('line 0'), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('code highlighting keeps supplementary characters in one span', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(body: EnhancedCodeBlock(code: 'print("你好😀")\n')),
      ),
    );

    expect(tester.takeException(), isNull);
    expect(find.textContaining('😀'), findsOneWidget);
    expect(find.textContaining('\uFFFD'), findsNothing);
  });

  testWidgets('code highlighting replaces an unpaired surrogate', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: EnhancedCodeBlock(code: 'x${String.fromCharCode(0xD800)}y'),
        ),
      ),
    );

    expect(tester.takeException(), isNull);
    expect(find.textContaining('x\uFFFDy'), findsOneWidget);
  });
}
