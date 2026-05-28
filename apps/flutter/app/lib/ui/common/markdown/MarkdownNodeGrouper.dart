// ignore_for_file: file_names

import 'package:flutter/material.dart';

sealed class MarkdownGroupedItem {
  const MarkdownGroupedItem();
}

class MarkdownSingleItem extends MarkdownGroupedItem {
  const MarkdownSingleItem(this.index);

  final int index;
}

class MarkdownGroupItem extends MarkdownGroupedItem {
  const MarkdownGroupItem({
    required this.startIndex,
    required this.endIndexInclusive,
    required this.stableKey,
  });

  final int startIndex;
  final int endIndexInclusive;
  final String stableKey;
}

enum MarkdownNodeType { plainText, xmlBlock }

class MarkdownNodeStable {
  const MarkdownNodeStable({
    required this.type,
    required this.content,
    required this.isStreaming,
  });

  final MarkdownNodeType type;
  final String content;
  final bool isStreaming;
}

abstract class MarkdownNodeGrouper {
  const MarkdownNodeGrouper();

  List<MarkdownGroupedItem> group(
    List<MarkdownNodeStable> nodes,
    String rendererId,
  );

  Widget renderGroup({
    required MarkdownGroupItem group,
    required List<MarkdownNodeStable> nodes,
    required String rendererId,
    required bool isVisible,
    required bool isLastNode,
    required Color textColor,
    required Widget Function(int index) renderNodeAt,
  });
}

class NoopMarkdownNodeGrouper extends MarkdownNodeGrouper {
  const NoopMarkdownNodeGrouper();

  @override
  List<MarkdownGroupedItem> group(
    List<MarkdownNodeStable> nodes,
    String rendererId,
  ) {
    return <MarkdownGroupedItem>[
      for (var i = 0; i < nodes.length; i++) MarkdownSingleItem(i),
    ];
  }

  @override
  Widget renderGroup({
    required MarkdownGroupItem group,
    required List<MarkdownNodeStable> nodes,
    required String rendererId,
    required bool isVisible,
    required bool isLastNode,
    required Color textColor,
    required Widget Function(int index) renderNodeAt,
  }) {
    return const SizedBox.shrink();
  }
}
