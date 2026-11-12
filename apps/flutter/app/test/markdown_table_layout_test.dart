import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_math_fork/flutter_math.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/common/markdown/EnhancedTableBlock.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

import 'support/semantic_markdown_fixture.dart';

/// Verifies table sizing with indivisible math and wrapping prose.
void main() {
  testWidgets('does not reuse empty cell results for incoming table content', (
    tester,
  ) async {
    await _pumpTable(tester, '| | | |\n|---|---|---|\n| | | |');
    final ready = Completer<void>();
    await _pumpTable(
      tester,
      '| 项目 | 计算 | 金额 |\n|---|---|---|\n| 官网 | 5 × 2000 | 10000 |',
      split: (source) async {
        await ready.future;
        return _splitCell(source);
      },
    );
    expect(find.text('项目', findRichText: true), findsOneWidget);
    expect(find.text('官网', findRichText: true), findsOneWidget);
    ready.complete();
    await tester.pumpAndSettle();
    expect(find.text('项目', findRichText: true), findsOneWidget);
    expect(find.text('10000', findRichText: true), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('keeps table cell text after real Core events resolve', (
    tester,
  ) async {
    await _pumpTable(
      tester,
      '| 名称 | 数量 |\n| --- | ---: |\n| 苹果 | **2** |',
      split: splitSemanticMarkdownFixture,
    );
    for (final text in ['名称', '数量', '苹果', '2']) {
      expect(find.text(text, findRichText: true), findsOneWidget);
    }
    expect(tester.takeException(), isNull);
  });

  testWidgets('late cell parsing cannot replace a newer source', (
    tester,
  ) async {
    final previous = Completer<List<MarkdownStreamEvent>>();
    final current = Completer<List<MarkdownStreamEvent>>();
    await _pumpTable(tester, '| 旧标题 |\n|---|', split: (_) => previous.future);
    await _pumpTable(tester, '| 新标题 |\n|---|', split: (_) => current.future);
    expect(find.text('新标题', findRichText: true), findsOneWidget);
    expect(find.text('旧标题', findRichText: true), findsNothing);
    previous.complete(await _splitCell('旧标题'));
    await tester.pumpAndSettle();
    expect(find.text('新标题', findRichText: true), findsOneWidget);
    current.complete(await _splitCell('新标题'));
    await tester.pumpAndSettle();
    expect(find.text('新标题', findRichText: true), findsOneWidget);
    expect(find.text('旧标题', findRichText: true), findsNothing);
    expect(tester.takeException(), isNull);
  });

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
Future<void> _pumpTable(
  WidgetTester tester,
  String table, {
  Future<List<MarkdownStreamEvent>> Function(String)? split,
}) async {
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
              splitMarkdownContent: split ?? _splitCell,
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
