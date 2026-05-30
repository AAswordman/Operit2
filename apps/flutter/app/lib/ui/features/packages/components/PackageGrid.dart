// ignore_for_file: file_names

import 'package:flutter/material.dart';

class PackageGrid extends StatelessWidget {
  const PackageGrid({
    super.key,
    required this.itemCount,
    required this.itemBuilder,
    this.padding = const EdgeInsets.fromLTRB(16, 8, 16, 120),
  });

  final int itemCount;
  final IndexedWidgetBuilder itemBuilder;
  final EdgeInsetsGeometry padding;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        return GridView.builder(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: padding,
          gridDelegate: _delegateForWidth(constraints.maxWidth),
          itemCount: itemCount,
          itemBuilder: itemBuilder,
        );
      },
    );
  }
}

class PackageInlineGrid extends StatelessWidget {
  const PackageInlineGrid({
    super.key,
    required this.itemCount,
    required this.itemBuilder,
  });

  final int itemCount;
  final IndexedWidgetBuilder itemBuilder;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        return GridView.builder(
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          gridDelegate: _delegateForWidth(constraints.maxWidth),
          itemCount: itemCount,
          itemBuilder: itemBuilder,
        );
      },
    );
  }
}

SliverGridDelegateWithFixedCrossAxisCount _delegateForWidth(double width) {
  final columns = switch (width) {
    >= 1280 => 4,
    >= 900 => 3,
    >= 600 => 2,
    _ => 1,
  };
  return SliverGridDelegateWithFixedCrossAxisCount(
    crossAxisCount: columns,
    crossAxisSpacing: 10,
    mainAxisSpacing: 10,
    mainAxisExtent: width >= 600 ? 124 : 132,
  );
}
