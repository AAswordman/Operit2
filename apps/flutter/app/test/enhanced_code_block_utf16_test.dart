import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/markdown/EnhancedCodeBlock.dart';

void main() {
  testWidgets(
    'code highlighting keeps supplementary characters in one span',
    (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(body: EnhancedCodeBlock(code: 'print("你好😀")\n')),
        ),
      );

      expect(tester.takeException(), isNull);
      expect(find.textContaining('😀'), findsOneWidget);
      expect(find.textContaining('\uFFFD'), findsNothing);
    },
  );

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
