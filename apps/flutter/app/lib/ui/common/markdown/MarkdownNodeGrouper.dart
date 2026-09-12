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

/// Stores one direct XML child decoded by the Rust Markdown stream.
class MarkdownXmlChildStable {
  const MarkdownXmlChildStable({
    required this.index,
    required this.tagName,
    required this.attributes,
    required this.body,
    required this.isClosed,
  });

  final int index;
  final String? tagName;
  final Map<String, String>? attributes;
  final String body;
  final bool isClosed;
}

typedef MarkdownNodePredicate = bool Function(MarkdownNodeStable node);

typedef MarkdownNodeRangeMergeRender =
    List<Widget> Function({
      required int startIndex,
      required int endIndexInclusive,
      required String renderInstanceKeyPrefix,
      required MarkdownNodePredicate shouldRenderNode,
    });

/// Describes one contiguous source range owned by a merge renderer.
class MarkdownMergeMatch {
  const MarkdownMergeMatch({
    required this.startIndex,
    required this.endIndexInclusive,
    required this.stableKey,
  });

  final int startIndex;
  final int endIndexInclusive;
  final String stableKey;
}

/// Matches and renders logical rows composed from multiple Markdown nodes.
abstract class MarkdownNodeMergeRender {
  const MarkdownNodeMergeRender();

  /// Matches one merge range beginning at the requested source index.
  MarkdownMergeMatch? match({
    required List<MarkdownNodeStable> nodes,
    required int startIndex,
    required int endIndexInclusive,
  });

  /// Renders one previously matched logical node range.
  Widget renderMerge({
    required MarkdownMergeMatch match,
    required List<MarkdownNodeStable> nodes,
    required String rendererId,
    required Color textColor,
    required MarkdownXmlRenderer xmlRenderer,
    required Stream<String>? Function(int index) xmlStreamResolver,
    required Stream<Object>? Function(int index) xmlMarkdownEventStreamResolver,
    required String renderInstanceKey,
  });
}

/// Leaves every Markdown node as an independent render item.
class NoopMarkdownNodeMergeRender extends MarkdownNodeMergeRender {
  const NoopMarkdownNodeMergeRender();

  /// Reports that no source range should be merged.
  @override
  MarkdownMergeMatch? match({
    required List<MarkdownNodeStable> nodes,
    required int startIndex,
    required int endIndexInclusive,
  }) {
    return null;
  }

  /// Rejects rendering because this strategy never produces a match.
  @override
  Widget renderMerge({
    required MarkdownMergeMatch match,
    required List<MarkdownNodeStable> nodes,
    required String rendererId,
    required Color textColor,
    required MarkdownXmlRenderer xmlRenderer,
    required Stream<String>? Function(int index) xmlStreamResolver,
    required Stream<Object>? Function(int index) xmlMarkdownEventStreamResolver,
    required String renderInstanceKey,
  }) {
    throw StateError('NoopMarkdownNodeMergeRender cannot render a match.');
  }
}

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
    this.xmlTagName,
    this.xmlAttributes,
    this.xmlBody,
    this.xmlChildren = const <MarkdownXmlChildStable>[],
    this.xmlIsClosed,
  });

  final MarkdownNodeType type;
  final String content;
  final bool isStreaming;
  final String stableKey;
  final List<MarkdownNodeStable> children;
  final int? headerLevel;
  final String? xmlTagName;
  final Map<String, String>? xmlAttributes;
  final String? xmlBody;
  final List<MarkdownXmlChildStable> xmlChildren;
  final bool? xmlIsClosed;

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        other is MarkdownNodeStable &&
            other.type == type &&
            other.content == content &&
            other.isStreaming == isStreaming &&
            other.stableKey == stableKey &&
            other.headerLevel == headerLevel &&
            other.xmlTagName == xmlTagName &&
            _mapEquals(other.xmlAttributes, xmlAttributes) &&
            other.xmlBody == xmlBody &&
            _xmlChildListEquals(other.xmlChildren, xmlChildren) &&
            other.xmlIsClosed == xmlIsClosed &&
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
      xmlTagName,
      xmlAttributes == null ? null : Object.hashAll(xmlAttributes!.entries),
      xmlBody,
      Object.hashAll(
        xmlChildren.map(
          (child) => Object.hash(
            child.index,
            child.tagName,
            child.attributes == null
                ? null
                : Object.hashAll(child.attributes!.entries),
            child.body,
            child.isClosed,
          ),
        ),
      ),
      xmlIsClosed,
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

/// Compares two nullable XML attribute maps by key and value.
bool _mapEquals(Map<String, String>? left, Map<String, String>? right) {
  if (identical(left, right)) {
    return true;
  }
  if (left == null || right == null || left.length != right.length) {
    return false;
  }
  for (final entry in left.entries) {
    if (right[entry.key] != entry.value) {
      return false;
    }
  }
  return true;
}

/// Compares direct XML child metadata in source order.
bool _xmlChildListEquals(
  List<MarkdownXmlChildStable> left,
  List<MarkdownXmlChildStable> right,
) {
  if (left.length != right.length) {
    return false;
  }
  for (var index = 0; index < left.length; index++) {
    final a = left[index];
    final b = right[index];
    if (a.index != b.index ||
        a.tagName != b.tagName ||
        !_mapEquals(a.attributes, b.attributes) ||
        a.body != b.body ||
        a.isClosed != b.isClosed) {
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
    required MarkdownNodeRangeMergeRender mergeRender,
    required Stream<String>? Function(int index) xmlStreamResolver,
    required Stream<Object>? Function(int index) xmlMarkdownEventStreamResolver,
    required void Function(String url)? onLinkClick,
    required bool fillMaxWidth,
    required TextStyle textStyle,
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
    required MarkdownNodeRangeMergeRender mergeRender,
    required Stream<String>? Function(int index) xmlStreamResolver,
    required Stream<Object>? Function(int index) xmlMarkdownEventStreamResolver,
    required void Function(String url)? onLinkClick,
    required bool fillMaxWidth,
    required TextStyle textStyle,
  }) {
    return const SizedBox.shrink();
  }
}
