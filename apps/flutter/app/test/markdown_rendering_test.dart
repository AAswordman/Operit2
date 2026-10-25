import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/markdown/MarkdownInlineSpannable.dart';
import 'package:operit2/ui/common/markdown/MarkdownNodeGrouper.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRendererState.dart';

import 'support/semantic_markdown_fixture.dart';

void main() {
  testWidgets('paints nested styles directly from Core semantic events', (
    tester,
  ) async {
    final gestures = MarkdownLinkGestureOwner();
    addTearDown(gestures.dispose);
    final nodes = markdownNodesFromEvents(
      semanticMarkdownEvents(semanticMarkdownFixtures[0]['source'] as String),
    );
    late TextSpan span;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) {
            span = buildMarkdownInlineSpannableFromMarkdownNodes(
              context: context,
              children: nodes,
              textColor: Colors.black,
              baseStyle: const TextStyle(fontWeight: FontWeight.w400),
              gestures: gestures,
            );
            return Text.rich(span);
          },
        ),
      ),
    );
    final leaves = _leaves(span);
    for (final leaf in leaves.where((leaf) => leaf.text!.trim().isNotEmpty)) {
      expect(leaf.style?.fontWeight, FontWeight.w700, reason: leaf.text);
    }
    expect(
      leaves.singleWhere((leaf) => leaf.text == '斜体').style?.fontStyle,
      FontStyle.italic,
    );
    expect(
      leaves.singleWhere((leaf) => leaf.text == '链接').recognizer,
      isNotNull,
    );
    expect(span.toPlainText(), '粗体 斜体 代码 链接 https://example.org');
  });

  for (final live in [false, true]) {
    testWidgets('renders Core lists, headings and emphasis for live=$live', (
      tester,
    ) async {
      final source = semanticMarkdownFixtures[1]['source'] as String;
      final events = semanticMarkdownEvents(source);
      final controller = StreamController<Object>.broadcast();
      await tester.pumpWidget(
        _surface(source, live ? controller.stream : null),
      );
      if (live) {
        for (final event in events.take(events.length - 1)) {
          controller.add(event);
          await tester.pump(const Duration(milliseconds: 150));
        }
        expect(_bullets(tester), hasLength(3));
        controller.add(events.last);
        await tester.pump();
        await controller.close();
      }
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(_bullets(tester), hasLength(3));
      expect(find.text('3)'), findsOneWidget);
      expect(
        find.textContaining('- 保留正文符号', findRichText: true),
        findsOneWidget,
      );
      final spans = tester
          .widgetList<RichText>(find.byType(RichText))
          .expand((widget) => _leaves(widget.text));
      final bold = spans.singleWhere((span) => span.text == '粗体');
      final both = spans.singleWhere((span) => span.text == '粗斜');
      expect(bold.style?.fontWeight, FontWeight.w700);
      expect(both.style?.fontWeight, FontWeight.w700);
      expect(both.style?.fontStyle, FontStyle.italic);
      final first = tester.getTopLeft(find.text('粗体', findRichText: true));
      final indented = tester.getTopLeft(find.text('缩进项', findRichText: true));
      expect(indented.dx, greaterThan(first.dx));
      await tester.pumpWidget(const SizedBox());
      if (!live) await controller.close();
    });

    testWidgets(
      'routes nested quote events without reparsing live text for live=$live',
      (tester) async {
        final source = semanticMarkdownFixtures[2]['source'] as String;
        final controller = StreamController<Object>.broadcast();
        var splits = 0;
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: StreamMarkdownRenderer(
                content: source,
                isStreaming: live,
                contentStream: live ? controller.stream : null,
                textColor: Colors.black,
                backgroundColor: Colors.white,
                splitMarkdownContent: (source) {
                  splits++;
                  return splitSemanticMarkdownFixture(source);
                },
              ),
            ),
          ),
        );
        if (live) {
          for (final event in semanticMarkdownEvents(
            source,
          ).where((event) => event.eventType != 'completed')) {
            controller.add(event);
            await tester.pump(const Duration(milliseconds: 150));
          }
          expect(splits, 0);
          expect(_bullets(tester), hasLength(1));
          expect(find.textContaining('嵌套引用', findRichText: true), findsWidgets);
          for (final event in semanticMarkdownEvents(
            source,
          ).where((event) => event.eventType == 'completed')) {
            controller.add(event);
          }
          await tester.pump();
          await controller.close();
        }
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        expect(_bullets(tester), hasLength(1));
        expect(find.textContaining('嵌套引用', findRichText: true), findsWidgets);
        await tester.pumpWidget(const SizedBox());
        if (!live) await controller.close();
      },
    );
  }

  test('reuses completed node snapshots while new chunks extend the tail', () {
    final state = StreamMarkdownRendererState();
    final builder = state.eventBuilder;
    builder.startBlock(
      blockId: 1,
      type: MarkdownNodeType.plainText,
      headerLevel: null,
    );
    builder.startInline(
      blockId: 1,
      inlineId: 1,
      type: MarkdownNodeType.plainText,
    );
    builder.appendInline(blockId: 1, inlineId: 1, content: 'history');
    builder.startBlock(
      blockId: 2,
      type: MarkdownNodeType.plainText,
      headerLevel: null,
    );
    builder.startInline(
      blockId: 2,
      inlineId: 1,
      type: MarkdownNodeType.plainText,
    );
    builder.appendInline(blockId: 2, inlineId: 1, content: 'tail');
    final before = builder.toStableNodes(isStreaming: true);
    builder.appendInline(blockId: 2, inlineId: 1, content: ' extended');
    final after = builder.toStableNodes(isStreaming: true);
    expect(identical(before.first, after.first), isTrue);
    expect(identical(before.last, after.last), isFalse);
    expect(after.last.content, 'tail extended');
  });

  test('restores nested style stacks and parent caches after rollback', () {
    final builder = StreamMarkdownRendererState().eventBuilder;
    builder.startBlock(
      blockId: 1,
      type: MarkdownNodeType.plainText,
      headerLevel: null,
    );
    builder.startInlineStyle(
      blockId: 1,
      inlineId: 1,
      type: MarkdownNodeType.bold,
    );
    builder.startInline(
      blockId: 1,
      inlineId: 2,
      type: MarkdownNodeType.plainText,
    );
    builder.appendInline(blockId: 1, inlineId: 2, content: 'before');
    builder.savepoint('saved');
    builder.startInlineStyle(
      blockId: 1,
      inlineId: 3,
      type: MarkdownNodeType.italic,
    );
    builder.startInline(
      blockId: 1,
      inlineId: 4,
      type: MarkdownNodeType.plainText,
    );
    builder.appendInline(blockId: 1, inlineId: 4, content: 'discard');
    builder.toStableNodes(isStreaming: true);
    builder.rollback('saved');
    builder.appendInline(blockId: 1, inlineId: 2, content: ' after');
    builder.endInlineStyle(blockId: 1, inlineId: 1);
    builder.complete();
    final block = builder.toStableNodes(isStreaming: false).single;
    expect(block.content, 'before after');
    expect(block.children.single.type, MarkdownNodeType.bold);
    expect(block.children.single.children.single.content, 'before after');
  });
}

Widget _surface(String source, Stream<Object>? stream) => MaterialApp(
  home: Scaffold(
    body: StreamMarkdownRenderer(
      content: source,
      isStreaming: stream != null,
      contentStream: stream,
      textColor: Colors.black,
      backgroundColor: Colors.white,
      splitMarkdownContent: splitSemanticMarkdownFixture,
    ),
  ),
);

List<TextSpan> _leaves(InlineSpan span) => [
  if (span is TextSpan) ...[
    if (span.text?.isNotEmpty ?? false) span,
    for (final child in span.children ?? <InlineSpan>[]) ..._leaves(child),
  ],
];

List<Container> _bullets(WidgetTester tester) => tester
    .widgetList<Container>(find.byType(Container))
    .where(
      (widget) =>
          widget.decoration is BoxDecoration &&
          (widget.decoration as BoxDecoration).shape == BoxShape.circle,
    )
    .toList();
