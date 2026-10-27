import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/markdown/MarkdownFootnotes.dart';
import 'package:operit2/ui/common/markdown/MarkdownNodeGrouper.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRendererState.dart';

import 'support/semantic_markdown_fixture.dart';

String _source(String prefix) =>
    semanticMarkdownFixtures.singleWhere(
          (fixture) => (fixture['source'] as String).startsWith(prefix),
        )['source']
        as String;

Widget _renderer(
  String source, {
  Stream<Object>? stream,
  void Function(String)? onLinkClick,
}) => StreamMarkdownRenderer(
  content: source,
  isStreaming: stream != null,
  contentStream: stream,
  textColor: Colors.black,
  backgroundColor: Colors.white,
  splitMarkdownContent: splitSemanticMarkdownFixture,
  onLinkClick: onLinkClick,
);

Widget _surface(
  String source, {
  Stream<Object>? stream,
  void Function(String)? onLinkClick,
}) => MaterialApp(
  home: Scaffold(
    body: SingleChildScrollView(
      child: _renderer(source, stream: stream, onLinkClick: onLinkClick),
    ),
  ),
);

Iterable<TextSpan> _leaves(InlineSpan span) sync* {
  if (span is! TextSpan) return;
  if (span.text?.isNotEmpty == true) yield span;
  for (final child in span.children ?? const <InlineSpan>[]) {
    yield* _leaves(child);
  }
}

void main() {
  test('Core definitions expose labels and formatted bodies separately', () {
    final nodes = markdownNodesFromEvents(
      semanticMarkdownEvents(_source('正文[^1]')),
    );
    final definitions = nodes
        .where((node) => node.type == MarkdownNodeType.footnoteDefinition)
        .toList();
    expect(definitions.map((node) => node.sourcePrefix), ['1', 'note']);
    expect(definitions.first.content, '第一条脚注与链接');
    expect(definitions.last.content, contains('续行内容'));
    expect(
      definitions.first.children.any(
        (node) => node.type == MarkdownNodeType.bold,
      ),
      isTrue,
    );
    expect(
      definitions.first.children.any(
        (node) => node.type == MarkdownNodeType.link,
      ),
      isTrue,
    );
  });

  for (final live in [false, true]) {
    testWidgets(
      'footnotes show superscripts and open formatted notes, live=$live',
      (tester) async {
        final source = _source('正文[^1]');
        final opened = <String>[];
        final controller = StreamController<Object>.broadcast();
        await tester.pumpWidget(
          _surface(
            source,
            stream: live ? controller.stream : null,
            onLinkClick: opened.add,
          ),
        );
        if (live) {
          final events = semanticMarkdownEvents(source);
          final definitionIndex = events.indexWhere(
            (event) =>
                event.eventType == 'markdownBlockStart' &&
                event.nodeType == 'FootnoteDefinition',
          );
          for (final event in events.take(definitionIndex)) {
            controller.add(event);
          }
          await tester.pump(const Duration(milliseconds: 250));
          await tester.pump(const Duration(seconds: 2));
          // The cached paragraph must resolve a definition that arrives later.
          expect(find.text('[^1]'), findsNWidgets(2));
          for (final event in events.skip(definitionIndex)) {
            controller.add(event);
          }
          await controller.close();
        } else {
          await controller.close();
        }
        await tester.pumpAndSettle();
        expect(find.byType(MarkdownFootnoteReference), findsNWidgets(4));
        expect(find.text('[^1]'), findsNothing);
        expect(find.text('[^missing]'), findsOneWidget);
        final firstReference = find.byType(MarkdownFootnoteReference).first;
        final marker = find.descendant(
          of: firstReference,
          matching: find.text('[1]'),
        );
        final style = tester.widget<Text>(marker).style!;
        expect(style.fontSize, lessThan(14));
        await tester.tap(marker);
        await tester.pumpAndSettle();
        expect(find.byType(AlertDialog), findsOneWidget);
        final richTexts = tester.widgetList<RichText>(
          find.descendant(
            of: find.byType(AlertDialog),
            matching: find.byType(RichText),
          ),
        );
        final leaves = richTexts.expand((rich) => _leaves(rich.text)).toList();
        expect(
          leaves.singleWhere((span) => span.text == '脚注').style?.fontWeight,
          FontWeight.w700,
        );
        final link = leaves.singleWhere((span) => span.text == '链接');
        (link.recognizer as TapGestureRecognizer).onTap!();
        expect(opened, ['https://example.com']);
        await tester.tap(
          find.descendant(
            of: find.byType(AlertDialog),
            matching: find.byType(TextButton),
          ),
        );
        await tester.pumpAndSettle();
        expect(find.byType(AlertDialog), findsNothing);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox());
      },
    );
  }

  testWidgets(
    'adjacent named notes stay separate and literal syntax stays literal',
    (tester) async {
      await tester.pumpWidget(_surface(_source('相邻[^a]')));
      await tester.pumpAndSettle();
      final references = tester.widgetList<MarkdownFootnoteReference>(
        find.byType(MarkdownFootnoteReference),
      );
      expect(references.map((reference) => reference.label), ['a', 'b']);
      expect(find.text('[a]'), findsNWidgets(2));
      expect(find.text('[b]'), findsNWidgets(2));
      final text = tester
          .widgetList<RichText>(find.byType(RichText))
          .map((rich) => rich.text.toPlainText())
          .join();
      expect(text, contains('[^code]'));
      expect(text, contains('[^escaped]'));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('an empty definition still resolves its reference', (
    tester,
  ) async {
    await tester.pumpWidget(_surface(_source('空脚注[^empty]')));
    await tester.pumpAndSettle();
    final reference = find.byType(MarkdownFootnoteReference);
    expect(reference, findsOneWidget);
    expect(
      find.descendant(of: reference, matching: find.text('[empty]')),
      findsOneWidget,
    );
    await tester.tap(
      find.descendant(of: reference, matching: find.text('[empty]')),
    );
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
  testWidgets('nested quotes resolve footnotes from their document', (
    tester,
  ) async {
    await tester.pumpWidget(_surface(_source('> 引用里的脚注')));
    await tester.pumpAndSettle();
    final reference = find.byType(MarkdownFootnoteReference);
    expect(reference, findsOneWidget);
    final marker = find.descendant(
      of: reference,
      matching: find.text('[quote]'),
    );
    expect(marker, findsOneWidget);
    await tester.tap(marker);
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsOneWidget);
    expect(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.text('引用注释', findRichText: true),
      ),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('separate messages do not share footnote definitions', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: Column(
              children: [
                _renderer(_source('正文[^1]')),
                _renderer(_source('独立文档[^1]')),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('[^1]'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('footnotes fit a narrow viewport with enlarged text', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(320, 640));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      MaterialApp(
        home: MediaQuery(
          data: const MediaQueryData(
            size: Size(320, 640),
            textScaler: TextScaler.linear(1.5),
          ),
          child: Scaffold(
            body: SingleChildScrollView(child: _renderer(_source('正文[^1]'))),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(MarkdownFootnoteReference), findsNWidgets(4));
    expect(tester.takeException(), isNull);
  });
}
