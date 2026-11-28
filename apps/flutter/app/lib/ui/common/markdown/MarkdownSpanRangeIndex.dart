// ignore_for_file: file_names

import 'package:flutter/widgets.dart';

/// Indexes UTF-16 span boundaries once so paragraph slicing skips unrelated text.
class MarkdownSpanRangeIndex {
  /// Retains original spans, including inherited styles and interactive children.
  MarkdownSpanRangeIndex(InlineSpan span) : _root = _SpanRange.build(span, 0);

  final _SpanRange _root;

  /// Takes a text range without scanning siblings preceding or following it.
  InlineSpan slice(int start, int end) {
    RangeError.checkValidRange(start, end, _root.end);
    return _root.slice(start, end) ?? const TextSpan();
  }
}

/// Stores absolute text positions for a span and its ordered direct children.
class _SpanRange {
  /// Keeps one immutable subtree's positions for repeated paragraph queries.
  const _SpanRange(this.source, this.start, this.end, this.children);

  final InlineSpan source;
  final int start;
  final int end;
  final List<_SpanRange> children;

  /// Counts text once; inline widgets occupy Flutter's single placeholder unit.
  static _SpanRange build(InlineSpan source, int start) {
    var end = start;
    final children = <_SpanRange>[];
    if (source is WidgetSpan) {
      end++;
    } else if (source is TextSpan) {
      end += source.text?.length ?? 0;
      for (final child in source.children ?? const <InlineSpan>[]) {
        final range = build(child, end);
        children.add(range);
        end = range.end;
      }
    }
    return _SpanRange(source, start, end, children);
  }

  /// Reuses covered subtrees and copies only ancestors crossing a range boundary.
  InlineSpan? slice(int from, int to) {
    if (from >= end || to <= start || from == to) return null;
    if (from <= start && to >= end) return source;
    final span = source;
    if (span is! TextSpan) return null;
    final text = span.text ?? '';
    final textFrom = (from - start).clamp(0, text.length);
    final textTo = (to - start).clamp(textFrom, text.length);
    // Find the first overlapping child instead of walking every preceding one.
    var lower = 0;
    var upper = children.length;
    while (lower < upper) {
      final middle = (lower + upper) >> 1;
      if (children[middle].end <= from) {
        lower = middle + 1;
      } else {
        upper = middle;
      }
    }
    final selected = <InlineSpan>[];
    for (
      var index = lower;
      index < children.length && children[index].start < to;
      index++
    ) {
      final child = children[index].slice(from, to);
      if (child != null) selected.add(child);
    }
    if (textFrom == textTo && selected.isEmpty) return null;
    return TextSpan(
      text: textFrom == textTo ? null : text.substring(textFrom, textTo),
      children: selected,
      style: span.style,
      recognizer: span.recognizer,
      mouseCursor: span.mouseCursor,
      onEnter: span.onEnter,
      onExit: span.onExit,
      locale: span.locale,
      spellOut: span.spellOut,
    );
  }
}
