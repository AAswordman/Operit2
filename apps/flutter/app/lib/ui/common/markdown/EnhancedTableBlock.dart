// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../theme/OperitTheme.dart';
import 'MarkdownInlineSpannable.dart';
import 'MarkdownNodeGrouper.dart';
import 'StreamMarkdownRenderer.dart';
import 'StreamMarkdownRendererState.dart';

const double tableMinColumnWidth = 80;
const double tableMaxColumnWidth = 320;
const double tableCellHorizontalPadding = 8;
const double tableCellVerticalPadding = 8;
const double tableOuterVerticalPadding = 8;
const double tableCornerRadius = 4;
const double tableBorderWidth = 1;
const double tableGridWidth = 0.5;
const double tableLineHeight = 1.3;

class EnhancedTableBlock extends StatelessWidget {
  const EnhancedTableBlock({
    super.key,
    required this.tableText,
    required this.textColor,
    required this.splitMarkdownContent,
  });

  final String tableText;
  final Color textColor;
  final MarkdownContentSplitter splitMarkdownContent;

  @override
  Widget build(BuildContext context) {
    final parsed = _parseTable(tableText);
    final rows = parsed.rows;
    final alignments = parsed.alignments;
    if (rows.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = Theme.of(context);
    final isDark = theme.brightness == Brightness.dark;
    final transparentSurface = OperitTheme.of(
      context,
    ).themePreferenceSnapshot.transparentSurfaceEnabled;
    final outline = theme.colorScheme.outline.withValues(
      alpha: transparentSurface ? 0.32 : 0.5,
    );
    final grid = theme.colorScheme.outline.withValues(
      alpha: transparentSurface ? 0.22 : 0.35,
    );
    final headerColor = isDark
        ? theme.colorScheme.onSurface.withValues(
            alpha: transparentSurface ? 0.06 : 0.08,
          )
        : theme.colorScheme.surfaceContainerHighest.withValues(
            alpha: transparentSurface ? 0.12 : 0.3,
          );
    final bodyColor = transparentSurface || isDark
        ? Colors.transparent
        : theme.colorScheme.surface;
    final bodyTextStyle = theme.textTheme.bodySmall?.copyWith(
      color: textColor,
      height: tableLineHeight,
    );
    final headerTextStyle = bodyTextStyle?.copyWith(
      fontWeight: FontWeight.w700,
    );
    final maxColumns = rows.fold<int>(
      0,
      (value, row) => row.length > value ? row.length : value,
    );
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: tableOuterVerticalPadding),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(tableCornerRadius),
        child: DecoratedBox(
          position: DecorationPosition.foreground,
          decoration: BoxDecoration(
            border: Border.all(color: outline, width: tableBorderWidth),
            borderRadius: BorderRadius.circular(tableCornerRadius),
          ),
          child: Padding(
            padding: const EdgeInsets.all(tableBorderWidth),
            child: SelectionArea(
              child: SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                physics: const BouncingScrollPhysics(),
                child: Table(
                  defaultColumnWidth: const IntrinsicColumnWidth(),
                  border: TableBorder(
                    horizontalInside: BorderSide(
                      color: grid,
                      width: tableGridWidth,
                    ),
                    verticalInside: BorderSide(
                      color: grid,
                      width: tableGridWidth,
                    ),
                  ),
                  children: <TableRow>[
                    for (var rowIndex = 0; rowIndex < rows.length; rowIndex++)
                      TableRow(
                        decoration: BoxDecoration(
                          color: rowIndex == 0 ? headerColor : bodyColor,
                        ),
                        children: <Widget>[
                          for (
                            var columnIndex = 0;
                            columnIndex < maxColumns;
                            columnIndex++
                          )
                            ConstrainedBox(
                              constraints: const BoxConstraints(
                                minWidth: tableMinColumnWidth,
                                maxWidth: tableMaxColumnWidth,
                              ),
                              child: Padding(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: tableCellHorizontalPadding,
                                  vertical: tableCellVerticalPadding,
                                ),
                                child: _MarkdownTableCellText(
                                  splitMarkdownContent: splitMarkdownContent,
                                  text: columnIndex < rows[rowIndex].length
                                      ? rows[rowIndex][columnIndex]
                                      : '',
                                  textColor: textColor,
                                  style: rowIndex == 0
                                      ? headerTextStyle
                                      : bodyTextStyle,
                                  textAlign: columnIndex < alignments.length
                                      ? alignments[columnIndex]
                                      : TextAlign.left,
                                ),
                              ),
                            ),
                        ],
                      ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

({List<List<String>> rows, List<TextAlign> alignments}) _parseTable(
  String tableText,
) {
  final rows = <List<String>>[];
  var alignments = const <TextAlign>[];
  for (final line in tableText.split('\n')) {
    final trimmed = line.trim();
    if (!trimmed.contains('|')) {
      continue;
    }
    final start = trimmed.startsWith('|') ? 1 : 0;
    final end = trimmed.endsWith('|') ? trimmed.length - 1 : trimmed.length;
    final withoutEdges = trimmed.substring(start, end < start ? start : end);
    final cells = withoutEdges.split('|').map((cell) => cell.trim()).toList();
    if (_isSeparatorRow(cells)) {
      if (alignments.isEmpty) {
        alignments = cells.map(_parseCellAlignment).toList(growable: false);
      }
      continue;
    }
    rows.add(cells);
  }
  return (rows: rows, alignments: alignments);
}

bool _isSeparatorRow(List<String> cells) {
  return cells.isNotEmpty &&
      cells.every((cell) {
        var start = cell.startsWith(':') ? 1 : 0;
        final end = cell.endsWith(':') ? cell.length - 1 : cell.length;
        if (start >= end) return false;
        for (; start < end; start++) {
          if (cell.codeUnitAt(start) != 45) return false;
        }
        return true;
      });
}

class _MarkdownTableCellText extends StatefulWidget {
  const _MarkdownTableCellText({
    required this.text,
    required this.textColor,
    required this.style,
    required this.textAlign,
    required this.splitMarkdownContent,
  });

  final String text;
  final Color textColor;
  final TextStyle? style;
  final TextAlign textAlign;
  final MarkdownContentSplitter splitMarkdownContent;

  @override
  State<_MarkdownTableCellText> createState() => _MarkdownTableCellTextState();
}

class _MarkdownTableCellTextState extends State<_MarkdownTableCellText> {
  final MarkdownLinkGestureOwner _gestures = MarkdownLinkGestureOwner();
  late Future<List<MarkdownNodeStable>> _nodes;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(covariant _MarkdownTableCellText oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.text != widget.text) _load();
  }

  void _load() {
    _nodes = widget.text.isEmpty
        ? Future.value(const [])
        : widget
              .splitMarkdownContent(widget.text)
              .then(markdownNodesFromEvents);
  }

  @override
  void dispose() {
    _gestures.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => FutureBuilder<List<MarkdownNodeStable>>(
    future: _nodes,
    builder: (context, snapshot) {
      _gestures.beginFrame();
      try {
        final nodes = snapshot.data;
        return Text.rich(
          nodes == null
              ? TextSpan(text: widget.text)
              : buildMarkdownInlineSpannableFromMarkdownNodes(
                  context: context,
                  children: nodes,
                  textColor: widget.textColor,
                  baseStyle: widget.style,
                  gestures: _gestures,
                ),
          textAlign: widget.textAlign,
          style: widget.style,
        );
      } finally {
        _gestures.endFrame();
      }
    },
  );
}

TextAlign _parseCellAlignment(String separatorCell) {
  final startsWithColon = separatorCell.startsWith(':');
  final endsWithColon = separatorCell.endsWith(':');
  if (startsWithColon && endsWithColon) {
    return TextAlign.center;
  }
  if (endsWithColon) {
    return TextAlign.right;
  }
  return TextAlign.left;
}
