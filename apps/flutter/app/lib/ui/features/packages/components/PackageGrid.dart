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
  final double maxItemWidth;
  final double minItemWidth;
  final double horizontalSpacing;

  /// Builds a lazily rendered package grid for expandable package cards.
  @override
  Widget build(BuildContext context) {
    return SliverLayoutBuilder(
      builder: (context, constraints) {
        final columnCount = _columnCountForWidth(constraints.crossAxisExtent);
        final rowCount = _rowCountForItems(itemCount, columnCount);
        final naturalItemWidth =
            (constraints.crossAxisExtent -
                horizontalSpacing * (columnCount - 1)) /
            columnCount;
        final itemWidth = math.min(maxItemWidth, naturalItemWidth).toDouble();
        return SliverList(
          delegate: SliverChildBuilderDelegate((context, rowIndex) {
            final firstIndex = rowIndex * columnCount;
            final visibleCount = math.min(columnCount, itemCount - firstIndex);
            return Row(
              mainAxisAlignment: MainAxisAlignment.start,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                for (
                  var offset = 0;
                  offset < visibleCount;
                  offset++
                ) ...<Widget>[
                  if (offset > 0) SizedBox(width: horizontalSpacing),
                  SizedBox(
                    width: itemWidth,
                    child: itemBuilder(context, firstIndex + offset),
                  ),
                ],
              ],
            );
          }, childCount: rowCount),
        );
      },
    );
  }

  /// Calculates the number of columns that respect the item width bounds.
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
