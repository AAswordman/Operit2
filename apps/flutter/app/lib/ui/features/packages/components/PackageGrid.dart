// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

class PackageSliverList extends StatelessWidget {
  /// Creates a lazily rendered responsive package grid sliver.
  const PackageSliverList({
    super.key,
    required this.itemCount,
    required this.itemBuilder,
    this.maxItemWidth = 420,
    this.minItemWidth = 300,
    this.horizontalSpacing = 12,
  }) : assert(itemCount >= 0),
       assert(minItemWidth > 0),
       assert(maxItemWidth >= minItemWidth),
       assert(horizontalSpacing >= 0);

  final int itemCount;
  final IndexedWidgetBuilder itemBuilder;

  /// The target maximum width used to choose the number of columns.
  final double maxItemWidth;
  final double minItemWidth;
  final double horizontalSpacing;

  /// Builds a lazily rendered package grid for expandable package cards.
  @override
  Widget build(BuildContext context) {
    double? layoutWidth;
    late Widget grid;
    return SliverLayoutBuilder(
      builder: (context, constraints) {
        // Scroll offsets change sliver constraints without changing the grid.
        // A new parent build creates a fresh cache for updated data and builders.
        if (layoutWidth != constraints.crossAxisExtent) {
          layoutWidth = constraints.crossAxisExtent;
          grid = _buildRows(layoutWidth!);
        }
        return grid;
      },
    );
  }

  /// Builds full-width variable-height rows with isolated card repaint boundaries.
  Widget _buildRows(double width) {
    final columnCount = _columnCountForWidth(width);
    final rowCount = _rowCountForItems(itemCount, columnCount);
    return SliverList(
      delegate: SliverChildBuilderDelegate(
        (context, rowIndex) {
          final firstIndex = rowIndex * columnCount;
          final visibleCount = math.min(columnCount, itemCount - firstIndex);
          return Row(
            mainAxisAlignment: MainAxisAlignment.start,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              for (var offset = 0; offset < visibleCount; offset++) ...<Widget>[
                if (offset > 0) SizedBox(width: horizontalSpacing),
                Expanded(
                  child: RepaintBoundary(
                    child: itemBuilder(context, firstIndex + offset),
                  ),
                ),
              ],
            ],
          );
        },
        childCount: rowCount,
        addAutomaticKeepAlives: false,
        addRepaintBoundaries: false,
      ),
    );
  }

  /// Chooses columns using the target width and minimum multi-column card width.
  int _columnCountForWidth(double width) {
    var count = math.max(
      1,
      ((width + horizontalSpacing) / (maxItemWidth + horizontalSpacing)).ceil(),
    );
    while (count > 1) {
      final candidateWidth = (width - horizontalSpacing * (count - 1)) / count;
      if (candidateWidth >= minItemWidth) {
        return count;
      }
      count -= 1;
    }
    return 1;
  }

  /// Calculates the number of rows for the item and column counts.
  int _rowCountForItems(int items, int columns) {
    return (items + columns - 1) ~/ columns;
  }
}
