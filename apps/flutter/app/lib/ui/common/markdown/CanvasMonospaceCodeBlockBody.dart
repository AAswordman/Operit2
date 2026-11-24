// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart' show SelectedContent;

import '../interactions/ChainedScrollController.dart';

import 'MarkdownCodeTypeface.dart';
import 'MarkdownSelectionArea.dart';

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
  late final _selection = _CodeSelectionDelegate(widget.lines);
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
      _selection.lines = widget.lines;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    _selection.dispose();
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
        selection: _selection,
      ),
    );
    final body = LayoutBuilder(
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
    );
    return MarkdownSelectionArea(
      child: SelectionContainer(delegate: _selection, child: body),
    );
  }
}

/// Reconstructs source between selected row endpoints, retaining empty lines.
class _CodeSelectionDelegate extends StaticSelectionContainerDelegate {
  /// Keeps source text independent of highlighted and virtualized paragraphs.
  _CodeSelectionDelegate(this.lines);

  List<String> lines;
  final rows = <int, StaticSelectionContainerDelegate>{};

  /// Copies source separators rather than concatenating paragraph fragments.
  @override
  SelectedContent? getSelectedContent() {
    final selected = [
      for (final index in rows.keys.toList()..sort())
        if (rows[index]!.getSelection() case final range?)
          (index: index, range: range),
    ];
    if (selected.isEmpty) return null;
    final first = selected.first;
    final last = selected.last;
    final start = math.min(first.range.startOffset, first.range.endOffset);
    final end = math.max(last.range.startOffset, last.range.endOffset);
    if (first.index == last.index) {
      return SelectedContent(
        plainText: lines[first.index].substring(start, end),
      );
    }
    return SelectedContent(
      plainText: [
        lines[first.index].substring(start),
        ...lines.sublist(first.index + 1, last.index),
        lines[last.index].substring(0, end),
      ].join('\n'),
    );
  }
}

class _CodeLine extends StatefulWidget {
  const _CodeLine({
    required this.lineNumber,
    required this.gutterWidth,
    required this.text,
    required this.span,
    required this.autoWrapEnabled,
    required this.selection,
  });

  final int lineNumber;
  final double gutterWidth;
  final String text;
  final InlineSpan? span;
  final bool autoWrapEnabled;
  final _CodeSelectionDelegate selection;

  /// Retains a row's source-selection endpoint across ordinary rebuilds.
  @override
  State<_CodeLine> createState() => _CodeLineState();
}

class _CodeLineState extends State<_CodeLine>
    with AutomaticKeepAliveClientMixin<_CodeLine> {
  final _selection = StaticSelectionContainerDelegate();

  /// Retains selected source endpoints when scrolling recycles visible rows.
  @override
  bool get wantKeepAlive => _selection.value.hasSelection;

  /// Updates row retention only when its selection geometry changes.
  void _selectionChanged() => updateKeepAlive();

  /// Registers source row identity alongside Flutter's selection geometry.
  @override
  void initState() {
    super.initState();
    _selection.addListener(_selectionChanged);
    widget.selection.rows[widget.lineNumber - 1] = _selection;
  }

  /// Rebinds recycled rows before the next selection query.
  @override
  void didUpdateWidget(covariant _CodeLine oldWidget) {
    super.didUpdateWidget(oldWidget);
    _unregister(oldWidget);
    widget.selection.rows[widget.lineNumber - 1] = _selection;
  }

  /// Prevents retiring wrap-mode rows from removing their replacements.
  void _unregister(_CodeLine row) {
    final index = row.lineNumber - 1;
    if (identical(row.selection.rows[index], _selection)) {
      row.selection.rows.remove(index);
    }
  }

  /// Releases only this row's registration when its viewport child retires.
  @override
  void dispose() {
    _unregister(widget);
    _selection.removeListener(_selectionChanged);
    _selection.dispose();
    super.dispose();
  }

  /// Keeps line numbers outside selection while registering the source row.
  @override
  Widget build(BuildContext context) {
    super.build(context);
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
      child: SelectionContainer(
        delegate: _selection,
        child: Text.rich(
          widget.span ?? TextSpan(text: widget.text),
          softWrap: widget.autoWrapEnabled,
          style: codeStyle,
        ),
      ),
    );
    final codeTextChild = widget.autoWrapEnabled
        ? Expanded(child: codeText)
        : ConstrainedBox(constraints: const BoxConstraints(), child: codeText);

    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        SelectionContainer.disabled(
          child: SizedBox(
            width: widget.gutterWidth,
            child: Padding(
              padding: const EdgeInsets.only(left: 8, right: 10),
              child: Text(
                '${widget.lineNumber}',
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
