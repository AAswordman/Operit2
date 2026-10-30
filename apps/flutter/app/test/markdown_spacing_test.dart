import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/markdown/EnhancedTableBlock.dart';
import 'package:operit2/ui/common/markdown/MarkdownLatexBlock.dart';
import 'package:operit2/ui/common/markdown/MarkdownInlineSpannable.dart';
import 'package:operit2/ui/common/markdown/MarkdownNodeGrouper.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';

import 'support/semantic_markdown_fixture.dart';
import 'support/markdown_event_fixture.dart';

void main() {
  testWidgets('separate blocks retain the full configured line height', (
    tester,
  ) async {
    for (final type in [
      MarkdownNodeType.plainText,
      MarkdownNodeType.orderedList,
      MarkdownNodeType.unorderedList,
    ]) {
      await _pumpNode(
        tester,
        MarkdownNodeStable(
          type: type,
          content: '内容',
          isStreaming: false,
          sourcePrefix: '1. ',
        ),
      );
      final text = tester.widget<RichText>(find.text('内容', findRichText: true));
      final lineHeight = text.text.style!.fontSize! * markdownLineHeight;
      expect(
        tester.getSize(find.byType(CanvasMarkdownNodeRenderer)).height,
        greaterThanOrEqualTo(lineHeight.floorToDouble()),
        reason: '$type must retain line spacing at block boundaries',
      );
    }
  });

  for (final scale in [1.0, 1.8]) {
    testWidgets(
      'wrapped inline code retains body line height at scale $scale',
      (tester) async {
        const code =
            'workspace/very_long_path/another_directory/project/file.dart';
        await _pumpNode(
          tester,
          const MarkdownNodeStable(
            type: MarkdownNodeType.plainText,
            content: '正文\n$code\n结束',
            isStreaming: false,
            children: [
              MarkdownNodeStable(
                type: MarkdownNodeType.plainText,
                content: '正文\n',
                isStreaming: false,
              ),
              MarkdownNodeStable(
                type: MarkdownNodeType.inlineCode,
                content: code,
                isStreaming: false,
              ),
              MarkdownNodeStable(
                type: MarkdownNodeType.plainText,
                content: '\n结束',
                isStreaming: false,
              ),
            ],
          ),
          textScaler: TextScaler.linear(scale),
        );
        final finder = find.text('正文\n$code\n结束', findRichText: true);
        final rich = tester.widget<RichText>(finder);
        final painter = TextPainter(
          text: rich.text,
          textDirection: rich.textDirection ?? TextDirection.ltr,
          textScaler: rich.textScaler,
          textHeightBehavior: rich.textHeightBehavior,
          strutStyle: rich.strutStyle,
        )..layout(maxWidth: tester.getSize(finder).width);
        try {
          final lines = painter.computeLineMetrics();
          expect(lines.length, greaterThan(3));
          final minHeight =
              rich.textScaler.scale(rich.text.style!.fontSize!) *
              markdownLineHeight;
          for (final line in lines) {
            expect(
              line.height,
              greaterThanOrEqualTo(minHeight.floorToDouble()),
            );
          }
          expect(tester.takeException(), isNull);
        } finally {
          painter.dispose();
        }
      },
    );
  }

  testWidgets('table cells use the configured Markdown line height', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: EnhancedTableBlock(
            tableText: '| 标题 |\n| --- |\n| 内容 |',
            textColor: Colors.black,
            splitMarkdownContent: splitMarkdownEventFixture,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    for (final content in ['标题', '内容']) {
      final finder = find.text(content, findRichText: true);
      final rich = tester.widget<RichText>(finder);
      final minHeight = rich.text.style!.fontSize! * markdownLineHeight;
      expect(
        tester.getSize(finder).height,
        greaterThanOrEqualTo(minHeight.floorToDouble()),
      );
    }
    expect(tester.takeException(), isNull);
  });

  for (final type in [
    MarkdownNodeType.header,
    MarkdownNodeType.plainText,
    MarkdownNodeType.orderedList,
    MarkdownNodeType.unorderedList,
  ]) {
    testWidgets('$type ignores separator whitespace at block boundaries', (
      tester,
    ) async {
      Future<double> height(String text) async {
        await _pumpNode(
          tester,
          MarkdownNodeStable(
            type: type,
            content: text,
            isStreaming: false,
            headerLevel: 2,
            sourcePrefix: '1. ',
            children: [
              MarkdownNodeStable(
                type: MarkdownNodeType.bold,
                content: text,
                isStreaming: false,
              ),
            ],
          ),
        );
        expect(find.text('排版内容', findRichText: true), findsOneWidget);
        return tester.getSize(find.byType(CanvasMarkdownNodeRenderer)).height;
      }

      final clean = await height('排版内容');
      expect(await height('\n  排版内容\n\n'), closeTo(clean, .01));
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets(
    'blank lines collapse to one paragraph gap, soft breaks survive',
    (tester) async {
      Future<double> height(String content) async {
        await _pumpNode(
          tester,
          MarkdownNodeStable(
            type: MarkdownNodeType.plainText,
            content: content,
            isStreaming: false,
          ),
        );
        return tester.getSize(find.byType(CanvasMarkdownNodeRenderer)).height;
      }

      final single = await height('第一段');
      final paragraphs = await height('第一段\n\n第二段');
      expect(find.text('第一段', findRichText: true), findsOneWidget);
      expect(find.text('第二段', findRichText: true), findsOneWidget);
      expect(paragraphs, closeTo(single * 2 + 4, .01));
      expect(await height('\n第一段\n \n\n\n第二段\n\n'), closeTo(paragraphs, .01));

      await height('第一行\n第二行');
      expect(find.text('第一行\n第二行', findRichText: true), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('split paragraphs retain nested emphasis and working links', (
    tester,
  ) async {
    final opened = <String>[];
    await _pumpNode(
      tester,
      const MarkdownNodeStable(
        type: MarkdownNodeType.plainText,
        content: '\n第一段\n\n第二段\n',
        isStreaming: false,
        children: [
          MarkdownNodeStable(
            type: MarkdownNodeType.link,
            content: '\n第一段\n\n第二段\n',
            linkUrl: 'https://example.com',
            isStreaming: false,
            children: [
              MarkdownNodeStable(
                type: MarkdownNodeType.bold,
                content: '\n第一段\n\n第二段\n',
                isStreaming: false,
              ),
            ],
          ),
        ],
      ),
      onLinkClick: opened.add,
    );
    for (final text in ['第一段', '第二段']) {
      final paragraph = find.text(text, findRichText: true);
      expect(paragraph, findsOneWidget);
      final spans = tester.widget<RichText>(paragraph).text;
      final leaf = _leaves(spans).singleWhere((span) => span.text == text);
      expect(leaf.style?.fontWeight, FontWeight.w700);
      expect(leaf.recognizer, isNotNull);
      await tester.tapAt(tester.getTopLeft(paragraph) + const Offset(10, 7));
    }
    expect(opened, ['https://example.com', 'https://example.com']);
    expect(tester.takeException(), isNull);
  });

  testWidgets('trimming and paragraph slicing preserve inline math widgets', (
    tester,
  ) async {
    await _pumpNode(
      tester,
      const MarkdownNodeStable(
        type: MarkdownNodeType.plainText,
        content: '\n公式 x^2\n\n说明\n',
        isStreaming: false,
        children: [
          MarkdownNodeStable(
            type: MarkdownNodeType.plainText,
            content: '\n公式 ',
            isStreaming: false,
          ),
          MarkdownNodeStable(
            type: MarkdownNodeType.inlineLatex,
            content: 'x^2',
            isStreaming: false,
          ),
          MarkdownNodeStable(
            type: MarkdownNodeType.plainText,
            content: '\n\n说明\n',
            isStreaming: false,
          ),
        ],
      ),
    );
    expect(find.byType(MarkdownInlineLatex), findsOneWidget);
    expect(find.text('说明', findRichText: true), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('nonfinal body and list gaps follow the original renderer', (
    tester,
  ) async {
    for (final (type, gap) in [
      (MarkdownNodeType.plainText, 6.0),
      (MarkdownNodeType.orderedList, 2.0),
      (MarkdownNodeType.unorderedList, 2.0),
    ]) {
      final node = MarkdownNodeStable(
        type: type,
        content: '内容',
        isStreaming: false,
        sourcePrefix: '1. ',
      );
      await _pumpNode(tester, node);
      final lastHeight = tester
          .getSize(find.byType(CanvasMarkdownNodeRenderer))
          .height;
      await _pumpNode(tester, node, isLastNode: false);
      final withGap = tester
          .getSize(find.byType(CanvasMarkdownNodeRenderer))
          .height;
      expect(withGap - lastHeight, closeTo(gap, .01));
    }
  });

  for (final live in [false, true]) {
    testWidgets(
      'real Core heading/list events have no extra lines, live=$live',
      (tester) async {
        final source = semanticMarkdownFixtures[1]['source'] as String;
        final controller = StreamController<Object>.broadcast();
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: StreamMarkdownRenderer(
                content: source,
                isStreaming: live,
                contentStream: live ? controller.stream : null,
                textColor: Colors.black,
                backgroundColor: Colors.white,
                splitMarkdownContent: splitSemanticMarkdownFixture,
              ),
            ),
          ),
        );
        if (live) {
          for (final event in semanticMarkdownEvents(source)) {
            controller.add(event);
            await tester.pump(const Duration(milliseconds: 150));
          }
          await controller.close();
        } else {
          await controller.close();
        }
        await tester.pumpAndSettle();
        expect(find.text('标题', findRichText: true), findsOneWidget);
        expect(find.text('粗体', findRichText: true), findsOneWidget);
        for (final rich in tester.widgetList<RichText>(find.byType(RichText))) {
          final text = rich.text.toPlainText();
          if (text.trim().isNotEmpty) expect(text, text.trim());
        }
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets('blank nodes have no height and do not create a trailing gap', (
    tester,
  ) async {
    await _pumpNode(
      tester,
      const MarkdownNodeStable(
        type: MarkdownNodeType.plainText,
        content: '\n\n',
        isStreaming: false,
        children: [
          MarkdownNodeStable(
            type: MarkdownNodeType.plainText,
            content: '\n\n',
            isStreaming: false,
          ),
        ],
      ),
      isLastNode: false,
    );
    expect(tester.getSize(find.byType(CanvasMarkdownNodeRenderer)).height, 0);
  });
}

Future<void> _pumpNode(
  WidgetTester tester,
  MarkdownNodeStable node, {
  bool isLastNode = true,
  void Function(String)? onLinkClick,
  TextScaler textScaler = TextScaler.noScaling,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      builder: (context, child) => MediaQuery(
        data: MediaQuery.of(context).copyWith(textScaler: textScaler),
        child: child!,
      ),
      home: Scaffold(
        body: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 360,
            child: CanvasMarkdownNodeRenderer(
              nodeKey: 'static-node-spacing',
              node: node,
              textColor: Colors.black,
              backgroundColor: Colors.white,
              isLastNode: isLastNode,
              onLinkClick: onLinkClick,
              splitMarkdownContent: (_) async => [],
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Iterable<TextSpan> _leaves(InlineSpan span) sync* {
  if (span is TextSpan) {
    if (span.text?.isNotEmpty ?? false) yield span;
    for (final child in span.children ?? const <InlineSpan>[]) {
      yield* _leaves(child);
    }
  }
}
