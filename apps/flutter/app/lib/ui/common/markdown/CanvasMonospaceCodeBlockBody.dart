// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../interactions/ChainedScrollController.dart';

import 'MarkdownCodeTypeface.dart';

/// Builds and highlights only the code lines inside the capped viewport.
class CanvasMonospaceCodeBlockBody extends StatefulWidget {
  const CanvasMonospaceCodeBlockBody({
    super.key,
    required this.lines,
    required this.autoWrapEnabled,
    this.highlightLine,
  });

  final List<String> lines;
  final bool autoWrapEnabled;
  final InlineSpan Function(String line)? highlightLine;

  @override
  State<CanvasMonospaceCodeBlockBody> createState() =>
      _CanvasMonospaceCodeBlockBodyState();
}

class _CanvasMonospaceCodeBlockBodyState
    extends State<CanvasMonospaceCodeBlockBody> {
  final _controller = ChainedScrollController();
  int? _maxColumns;

  int _measureColumns() {
    var maxColumns = 0;
    for (final line in widget.lines) {
      var columns = 0;
      for (final rune in line.runes) {
        columns += rune == 9 ? 4 : (rune > 0x7f ? 2 : 1);
      }
      maxColumns = math.max(maxColumns, columns);
    }
    return maxColumns;
  }

  @override
  void didUpdateWidget(covariant CanvasMonospaceCodeBlockBody oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.lines, widget.lines)) {
      _maxColumns = null;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final digits = widget.lines.length.toString().length;
    final gutterWidth = (digits * 8.0 + 20.0).clamp(36.0, 64.0);
    final list = ListView.builder(
      controller: _controller,
      primary: false,
      shrinkWrap: true,
      padding: const EdgeInsets.symmetric(vertical: 6),
      itemCount: widget.lines.length,
      itemBuilder: (context, index) => _CodeLine(
        lineNumber: index + 1,
        gutterWidth: gutterWidth,
        text: widget.lines[index],
        span: widget.highlightLine?.call(widget.lines[index]),
        autoWrapEnabled: widget.autoWrapEnabled,
      ),
    );
    return SelectionArea(
      child: LayoutBuilder(
        builder: (context, constraints) {
          if (widget.autoWrapEnabled) return list;
          final measure = TextPainter(
            text: TextSpan(text: 'M', style: markdownCodeTextStyle(context)),
            textDirection: Directionality.of(context),
            textScaler: MediaQuery.textScalerOf(context),
          )..layout();
          final width = math.max(
            constraints.maxWidth,
            gutterWidth +
                (_maxColumns ??= _measureColumns()) * measure.width +
                12,
          );
          measure.dispose();
          return SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: SizedBox(width: width, child: list),
          );
        },
      ),
    );
  }
}

class _CodeLine extends StatelessWidget {
  const _CodeLine({
    required this.lineNumber,
    required this.gutterWidth,
    required this.text,
    required this.span,
    required this.autoWrapEnabled,
  });

  final int lineNumber;
  final double gutterWidth;
  final String text;
  final InlineSpan? span;
  final bool autoWrapEnabled;

  @override
  Widget build(BuildContext context) {
    final codeStyle = markdownCodeTextStyle(
      context,
      color: const Color(0xFFD4D4D4),
    );
    final lineNumberStyle =
        markdownCodeTextStyle(
          context,
          color: const Color(0xFF6E7681),
        )?.copyWith(
          fontFeatures: const <FontFeature>[FontFeature.tabularFigures()],
        );
    final codeText = Padding(
      padding: const EdgeInsets.only(right: 12),
      child: span == null
          ? Text(text, softWrap: autoWrapEnabled, style: codeStyle)
          : Text.rich(span!, softWrap: autoWrapEnabled, style: codeStyle),
    );
    final codeTextChild = autoWrapEnabled
        ? Expanded(child: codeText)
        : ConstrainedBox(constraints: const BoxConstraints(), child: codeText);

    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        SelectionContainer.disabled(
          child: SizedBox(
            width: gutterWidth,
            child: Padding(
              padding: const EdgeInsets.only(left: 8, right: 10),
              child: Text(
                '$lineNumber',
                textAlign: TextAlign.end,
                style: lineNumberStyle,
              ),
            ),
          ),
        ),
        codeTextChild,
      ],
    );
  }
}
