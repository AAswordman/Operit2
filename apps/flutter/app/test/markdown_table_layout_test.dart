import 'package:flutter/material.dart';
import 'package:flutter_math_fork/flutter_math.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/common/markdown/EnhancedTableBlock.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies table sizing with indivisible math and wrapping prose.
void main() {
  testWidgets('long formulas fit their columns and remain scrollable', (
    tester,
  ) async {
    final formula = List.filled(24, r'\frac{x_i^2+y_i^2}{1+z_i^2}').join('+');
    await _pumpTable(
      tester,
      '| 公式 | 指数 |\n| --- | --- |\n| \$$formula\$ | ★★★★★ |',
    );

    final math = find.byType(Math);
    final mathBox = tester.renderObject<RenderBox>(math);
    final naturalWidth = mathBox.getMaxIntrinsicWidth(double.infinity);
    expect(naturalWidth, greaterThan(tableMaxColumnWidth));
    expect(mathBox.size.width, greaterThanOrEqualTo(naturalWidth - 0.01));
    expect(
      tester.getRect(math).right,
      lessThanOrEqualTo(
        tester.getRect(find.text('★★★★★', findRichText: true)).left,
      ),
    );

    final scrollable = tester.state<ScrollableState>(
      find.descendant(
        of: find.byType(EnhancedTableBlock),
        matching: find.byType(Scrollable),
      ),
    );
    expect(scrollable.position.maxScrollExtent, greaterThan(0));
    scrollable.position.jumpTo(scrollable.position.maxScrollExtent);
    await tester.pump();
    final viewport = tester.getRect(find.byType(SingleChildScrollView));
    final finalCell = tester.getRect(find.text('★★★★★', findRichText: true));
    expect(finalCell.left, greaterThanOrEqualTo(viewport.left));
    expect(finalCell.right, lessThanOrEqualTo(viewport.right));
    expect(tester.takeException(), isNull);
  });

  testWidgets('ordinary table text wraps at the preferred maximum width', (
    tester,
  ) async {
    final prose = List.filled(80, 'wrapping text').join(' ');
    await _pumpTable(tester, '| 标题 |\n| --- |\n| $prose |');

    expect(tester.getSize(find.byType(Table)).width, tableMaxColumnWidth);
    expect(
      tester.getSize(find.text(prose, findRichText: true)).height,
      greaterThan(40),
    );
    expect(tester.takeException(), isNull);
  });
}

/// Presents a narrow table using semantic cell events without a Core runtime.
Future<void> _pumpTable(WidgetTester tester, String table) async {
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: Scaffold(
        body: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 400,
            child: EnhancedTableBlock(
              tableText: table,
              textColor: Colors.black,
              splitMarkdownContent: _splitCell,
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

/// Supplies the semantic events for one plain-text or inline-math cell.
Future<List<MarkdownStreamEvent>> _splitCell(String source) async {
  final type = source.startsWith(r'$') ? 'InlineLatex' : null;
  return [
    for (final event in [
      'markdownBlockStart',
      'markdownInlineStart',
      'markdownInlineChunk',
      'markdownBlockEnd',
      'completed',
    ])
      MarkdownStreamEvent(
        chatId: 'test',
        eventType: event,
        value: event == 'markdownInlineChunk' ? source : null,
        id: null,
        blockId: 1,
        inlineId: 1,
        parentBlockId: null,
        nodeType: event == 'markdownBlockStart' ? null : type,
        headerLevel: null,
        xml: null,
      ),
  ];
}
