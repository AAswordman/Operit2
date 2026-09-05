// ignore_for_file: file_names

import 'package:flutter/material.dart';

class PackageSliverList extends StatelessWidget {
  /// Creates a lazily rendered package sliver.
  const PackageSliverList({
    super.key,
    required this.itemCount,
    required this.itemBuilder,
  });

  final int itemCount;
  final IndexedWidgetBuilder itemBuilder;

  /// Builds a lazily rendered package sliver for expandable package cards.
  @override
  Widget build(BuildContext context) {
    return SliverList(
      delegate: SliverChildBuilderDelegate(itemBuilder, childCount: itemCount),
    );
  }
}
