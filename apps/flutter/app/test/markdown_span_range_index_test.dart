import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/markdown/MarkdownSpanRangeIndex.dart';

/// Verifies paragraph ranges preserve text, formatting, widgets and bounded traversal.
void main() {
  test('all UTF-16 ranges retain their exact text and inline widgets', () {
    const widget = WidgetSpan(child: SizedBox(width: 10, height: 10));
    const source = TextSpan(
      text: '前😀',
      children: [
        TextSpan(
          text: 'bold',
          style: TextStyle(fontWeight: FontWeight.bold),
          children: [TextSpan(text: '\n\n第二段')],
        ),
        widget,
        TextSpan(text: '尾'),
      ],
    );
    final index = MarkdownSpanRangeIndex(source);
    final text = source.toPlainText(includeSemanticsLabels: false);
    for (var start = 0; start <= text.length; start++) {
      for (var end = start; end <= text.length; end++) {
        expect(
          index.slice(start, end).toPlainText(includeSemanticsLabels: false),
          text.substring(start, end),
          reason: '$start..$end',
        );
      }
    }
    final widgetOffset = text.indexOf('\uFFFC');
    final selected = index.slice(widgetOffset, widgetOffset + 1) as TextSpan;
    expect(identical(selected.children!.single, widget), isTrue);
    expect(index.slice(0, text.length), same(source));
  });

  test(
    'partial ranges retain inherited styles and the original link owner',
    () {
      final recognizer = TapGestureRecognizer();
      addTearDown(recognizer.dispose);
      final source = TextSpan(
        style: const TextStyle(fontSize: 17),
        children: [
          const TextSpan(text: 'prefix\n\n'),
          TextSpan(
            style: const TextStyle(fontWeight: FontWeight.bold),
            children: [
              TextSpan(
                text: 'linked text',
                recognizer: recognizer,
                mouseCursor: SystemMouseCursors.click,
                locale: const Locale('zh'),
                spellOut: true,
              ),
            ],
          ),
          const TextSpan(text: '\n\nsuffix'),
        ],
      );
      final selected = MarkdownSpanRangeIndex(source).slice(10, 16) as TextSpan;
      expect(selected.toPlainText(), 'nked t');
      expect(selected.style, source.style);
      final bold = selected.children!.single as TextSpan;
      expect(bold.style!.fontWeight, FontWeight.bold);
      final link = bold.children!.single as TextSpan;
      expect(link.recognizer, same(recognizer));
      expect(link.mouseCursor, SystemMouseCursors.click);
      expect(link.locale, const Locale('zh'));
      expect(link.spellOut, isTrue);
    },
  );

  test('thousands of paragraph slices do not reread unrelated text spans', () {
    var textReads = 0;
    final leaves = <InlineSpan>[];
    for (var paragraph = 0; paragraph < 4000; paragraph++) {
      leaves.add(_CountedTextSpan('正文', () => textReads++));
      leaves.add(_CountedTextSpan('bold', () => textReads++));
      leaves.add(_CountedTextSpan('\n\n', () => textReads++));
    }
    final index = MarkdownSpanRangeIndex(TextSpan(children: leaves));
    for (var paragraph = 0; paragraph < 4000; paragraph++) {
      final span = index.slice(paragraph * 8, paragraph * 8 + 6);
      expect(span.toPlainText(), '正文bold');
    }
    // Indexing and returning 4,000 paragraphs must stay proportional to input.
    // The former full-tree scan read 48 million leaf strings for this fixture.
    expect(textReads, lessThan(leaves.length * 4));
  });
}

/// Counts actual leaf text reads so the complexity check needs no timing threshold.
class _CountedTextSpan extends TextSpan {
  /// Retains normal TextSpan behavior while recording traversal work.
  const _CountedTextSpan(String text, this.onRead) : super(text: text);

  final VoidCallback onRead;

  /// Observes every access to this leaf's text, including paragraph extraction.
  @override
  String? get text {
    onRead();
    return super.text;
  }
}
