import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/markdown/CanvasMonospaceCodeBlockBody.dart';
import 'package:operit2/ui/common/markdown/EnhancedCodeBlock.dart';

void main() {
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
