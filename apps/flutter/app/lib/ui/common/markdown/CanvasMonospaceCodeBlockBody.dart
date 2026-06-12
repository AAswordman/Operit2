// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'MarkdownCodeTypeface.dart';

class CanvasMonospaceCodeBlockBody extends StatelessWidget {
  const CanvasMonospaceCodeBlockBody({
    super.key,
    required this.lines,
    required this.autoWrapEnabled,
    this.highlightedLines,
  });

  final List<String> lines;
  final bool autoWrapEnabled;
  final List<InlineSpan>? highlightedLines;

  @override
  Widget build(BuildContext context) {
    return SelectionArea(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          for (var index = 0; index < lines.length; index++)
            _CodeLine(
              lineNumber: index + 1,
              text: lines[index],
              span:
                  highlightedLines == null || index >= highlightedLines!.length
                  ? null
                  : highlightedLines![index],
              autoWrapEnabled: autoWrapEnabled,
            ),
        ],
      ),
    );
  }
}

class _CodeLine extends StatelessWidget {
  const _CodeLine({
    required this.lineNumber,
    required this.text,
    required this.span,
    required this.autoWrapEnabled,
  });

  final int lineNumber;
  final String text;
  final InlineSpan? span;
  final bool autoWrapEnabled;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        SelectionContainer.disabled(
          child: SizedBox(
            width: 40,
            child: Padding(
              padding: const EdgeInsets.only(right: 8),
              child: Text(
                '$lineNumber',
                textAlign: TextAlign.end,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: const Color(0xFF858585),
                  fontFamily: markdownCodeFontFamily,
                ),
              ),
            ),
          ),
        ),
        ConstrainedBox(
          constraints: autoWrapEnabled
              ? const BoxConstraints(maxWidth: 680)
              : const BoxConstraints(),
          child: span == null
              ? Text(
                  text,
                  style: markdownCodeTextStyle(
                    context,
                    color: const Color(0xFFD4D4D4),
                  ),
                )
              : Text.rich(
                  span!,
                  style: markdownCodeTextStyle(
                    context,
                    color: const Color(0xFFD4D4D4),
                  ),
                ),
        ),
      ],
    );
  }
}
