// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../theme/OperitTheme.dart';
import 'MarkdownInlineSpannable.dart';

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
  });

  final String tableText;
  final Color textColor;

  @override
  Widget build(BuildContext context) {
    final rows = _parseTableRows(tableText);
    if (rows.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = Theme.of(context);
    final transparentSurface = OperitTheme.of(
      context,
    ).themePreferenceSnapshot.transparentSurfaceEnabled;
    final outline = theme.colorScheme.outline.withValues(
      alpha: transparentSurface ? 0.32 : 0.5,
    );
    final grid = theme.colorScheme.outline.withValues(
      alpha: transparentSurface ? 0.22 : 0.35,
    );
    final headerColor = theme.colorScheme.surfaceContainerHighest.withValues(
      alpha: transparentSurface ? 0.12 : 0.3,
    );
    final bodyColor = transparentSurface
        ? Colors.transparent
        : theme.colorScheme.surface;
    final maxColumns = rows.fold<int>(
      0,
      (value, row) => row.length > value ? row.length : value,
    );
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: tableOuterVerticalPadding),
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border.all(color: outline, width: tableBorderWidth),
          borderRadius: BorderRadius.circular(tableCornerRadius),
        ),
        child: ClipRRect(
          borderRadius: BorderRadius.circular(tableCornerRadius),
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
                              child: Text.rich(
                                buildMarkdownInlineSpannableFromText(
                                  context: context,
                                  text: columnIndex < rows[rowIndex].length
                                      ? rows[rowIndex][columnIndex]
                                      : '',
                                  textColor: textColor,
                                ),
                                style: theme.textTheme.bodySmall?.copyWith(
                                  color: textColor,
                                  height: tableLineHeight,
                                  fontWeight: rowIndex == 0
                                      ? FontWeight.w700
                                      : null,
                                ),
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
    );
  }
}

List<List<String>> _parseTableRows(String tableText) {
  final rows = <List<String>>[];
  for (final line in tableText.split('\n')) {
    final trimmed = line.trim();
    if (!trimmed.contains('|')) {
      continue;
    }
    final withoutEdges = trimmed
        .replaceFirst(RegExp(r'^\|'), '')
        .replaceFirst(RegExp(r'\|$'), '');
    final cells = withoutEdges.split('|').map((cell) => cell.trim()).toList();
    if (_isSeparatorRow(cells)) {
      continue;
    }
    rows.add(cells);
  }
  return rows;
}

bool _isSeparatorRow(List<String> cells) {
  return cells.isNotEmpty &&
      cells.every((cell) => RegExp(r'^:?-{3,}:?$').hasMatch(cell));
}
