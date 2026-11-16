// ignore_for_file: file_names

import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'syntax/CodeSyntaxHighlighter.dart';
import 'syntax/CodeSyntaxLanguage.dart';
import 'syntax/CodeSyntaxPalette.dart';

export 'syntax/CodeSyntaxHighlighter.dart' show CodeTextHighlight;

/// Uses the editor's syntax spans in caller-decorated, lazily built source rows.
class CodeEditorText extends StatelessWidget {
  const CodeEditorText({
    super.key,
    required this.text,
    required this.style,
    this.filePath = '',
    this.highlights = const [],
    this.softWrap,
    this.maxLines,
  });
  final String text;
  final TextStyle style;
  final String filePath;
  final List<CodeTextHighlight> highlights;
  final bool? softWrap;
  final int? maxLines;

  @override
  Widget build(BuildContext context) {
    final syntax = CodeSyntaxHighlighter();
    final lines = text.split('\n').map(CodeSyntaxLine.new).toList();
    syntax.configure(CodeSyntaxLanguage.forPath(filePath), lines);
    final palette = CodeSyntaxPalette(Theme.of(context).brightness);
    var offset = 0;
    final spans = <InlineSpan>[];
    for (var i = 0; i < lines.length; i++) {
      final line = lines[i];
      spans.add(
        syntax.span(
          line,
          null,
          palette,
          highlights: [
            for (final range in highlights)
              if (range.end > offset &&
                  range.start < offset + line.content.length)
                CodeTextHighlight(
                  math.max(0, range.start - offset),
                  math.min(line.content.length, range.end - offset),
                  range.style,
                ),
          ],
        ),
      );
      if (i < lines.length - 1) spans.add(const TextSpan(text: '\n'));
      offset += line.content.length + 1;
    }
    syntax.dispose();
    return Text.rich(
      TextSpan(children: spans),
      style: style.copyWith(inherit: false),
      strutStyle: StrutStyle.fromTextStyle(style, forceStrutHeight: true),
      softWrap: softWrap,
      maxLines: maxLines,
    );
  }
}
