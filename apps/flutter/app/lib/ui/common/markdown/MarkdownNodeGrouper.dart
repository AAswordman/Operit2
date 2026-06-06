// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

typedef MarkdownXmlRenderer =
    Widget Function({
      required String xmlContent,
      required bool isStreaming,
      required Color textColor,
      Stream<String>? xmlStream,
      Stream<Object>? xmlMarkdownEventStream,
      String? renderInstanceKey,
    });

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

enum MarkdownNodeType {
  plainText,
  header,
  blockQuote,
  codeBlock,
  orderedList,
  unorderedList,
  horizontalRule,
  blockLatex,
  table,
  xmlBlock,
  image,
  bold,
  italic,
  inlineCode,
  link,
  strikethrough,
  underline,
  inlineLatex,
  htmlBreak,
}

class MarkdownNodeStable {
  const MarkdownNodeStable({
    required this.type,
    required this.content,
    required this.isStreaming,
    this.stableKey = '',
    this.children = const <MarkdownNodeStable>[],
    this.headerLevel,
  });

  final MarkdownNodeType type;
  final String content;
  final bool isStreaming;
  final String stableKey;
  final List<MarkdownNodeStable> children;
  final int? headerLevel;

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        other is MarkdownNodeStable &&
            other.type == type &&
            other.content == content &&
            other.isStreaming == isStreaming &&
            other.stableKey == stableKey &&
            other.headerLevel == headerLevel &&
            _listEquals(other.children, children);
  }

  @override
  int get hashCode {
    return Object.hash(
      type,
      content,
      isStreaming,
      stableKey,
      headerLevel,
      Object.hashAll(children),
    );
  }
}

bool _listEquals<T>(List<T> left, List<T> right) {
  if (identical(left, right)) {
    return true;
  }
  if (left.length != right.length) {
    return false;
  }
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) {
      return false;
    }
  }
  return true;
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
    required MarkdownXmlRenderer xmlRenderer,
    required Stream<String>? Function(int index) xmlStreamResolver,
    required Stream<Object>? Function(int index) xmlMarkdownEventStreamResolver,
    required void Function(String url)? onLinkClick,
    required bool fillMaxWidth,
    required double fontSize,
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
    required MarkdownXmlRenderer xmlRenderer,
    required Stream<String>? Function(int index) xmlStreamResolver,
    required Stream<Object>? Function(int index) xmlMarkdownEventStreamResolver,
    required void Function(String url)? onLinkClick,
    required bool fillMaxWidth,
    required double fontSize,
  }) {
    return const SizedBox.shrink();
  }
}
