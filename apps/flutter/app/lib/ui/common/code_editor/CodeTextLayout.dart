// ignore_for_file: file_names

import 'dart:collection';
import 'dart:math' as math;
import 'dart:typed_data';
import 'dart:ui' show Brightness;

import 'package:flutter/rendering.dart';

import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxHighlighter.dart';
import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxLanguage.dart';
import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxPalette.dart';

/// Stores source offsets independently from cached, viewport-sized paragraphs.
class CodeTextLayout {
  /// Indexes the complete source without shaping any of its paragraphs.
  CodeTextLayout(
    String text, {
    CodeSyntaxLanguage language = CodeSyntaxLanguage.plainText,
    CodeSyntaxPalette palette = const CodeSyntaxPalette(Brightness.light),
  }) : _text = text,
       lines = _split(text),
       _syntaxPalette = palette {
    syntax.configure(language, lines);
  }

  String _text;
  List<CodeTextLine> lines;
  final CodeSyntaxHighlighter syntax = CodeSyntaxHighlighter();
  CodeSyntaxPalette _syntaxPalette;
  final LinkedHashMap<CodeTextLine, TextPainter> _paragraphs =
      LinkedHashMap<CodeTextLine, TextPainter>();
  Float64List _measuredHeights = Float64List(0);
  Uint32List _measuredCounts = Uint32List(0);
  double _measuredTotal = 0;
  int _measuredCount = 0;
  double _estimatedHeight = 0;
  double _width = -1;
  int _epoch = 0;
  double lineHeight = 0;
  TextStyle? _style;
  bool _fontsChanged = false;
  TextScaler? _textScaler;
  TextDirection? _direction;
  Locale? _locale;
  int debugParagraphLayouts = 0;
  int cacheCapacity = 128;

  /// Measures horizontal source extent with the same font metrics as editing.
  /// Used only for unwrapped custom bodies, cached by their owning editor.
  static double measureUnwrappedWidth(
    String text,
    TextStyle style,
    TextScaler scaler,
    TextDirection direction, {
    Locale? locale,
  }) {
    final layout = CodeTextLayout(text);
    layout.configure(double.infinity, style, scaler, direction, locale: locale);
    var width = 0.0;
    for (var i = 0; i < layout.lines.length; i++) {
      width = math.max(
        width,
        layout.paragraph(i, updateHeight: false).maxIntrinsicWidth,
      );
      if (i % 128 == 0) layout.trimCache({});
    }
    layout.dispose();
    return width.ceilToDouble();
  }

  /// Returns the number of native paragraphs retained by the viewport cache.
  int get cachedParagraphs => _paragraphs.length;

  /// Splits logical lines while preserving CRLF and UTF-16 document offsets.
  static List<CodeTextLine> _split(String text) {
    final result = <CodeTextLine>[];
    var start = 0;
    for (final match in RegExp(r'\r\n|\r|\n').allMatches(text)) {
      result.add(
        CodeTextLine(
          start,
          text.substring(start, match.start),
          text.substring(match.start, match.end),
        ),
      );
      start = match.end;
    }
    result.add(CodeTextLine(start, text.substring(start), ''));
    return result;
  }

  /// Reindexes only the edited line interval, retaining unchanged paragraph keys.
  void updateText(String text) {
    if (_text == text) return;
    var prefix = 0;
    final shorter = math.min(_text.length, text.length);
    while (prefix < shorter &&
        _text.codeUnitAt(prefix) == text.codeUnitAt(prefix)) {
      prefix++;
    }
    var oldEnd = _text.length;
    var newEnd = text.length;
    while (oldEnd > prefix &&
        newEnd > prefix &&
        _text.codeUnitAt(oldEnd - 1) == text.codeUnitAt(newEnd - 1)) {
      oldEnd--;
      newEnd--;
    }
    // Include both neighbors so edits inside CRLF pairs cannot split a separator.
    final first = lineAtOffset(math.max(0, prefix - 1));
    final last = lineAtOffset(math.min(_text.length, oldEnd + 1));
    final regionStart = lines[first].start;
    final hasSuffix = last + 1 < lines.length;
    final regionEnd = hasSuffix ? lines[last + 1].start : _text.length;
    final delta = text.length - _text.length;
    final replacement = _split(text.substring(regionStart, regionEnd + delta));
    if (hasSuffix) {
      assert(
        replacement.last.content.isEmpty && replacement.last.separator.isEmpty,
      );
      replacement.removeLast();
    }
    for (final line in replacement) {
      line.start += regionStart;
    }
    var unchangedStart = 0;
    while (first + unchangedStart <= last &&
        unchangedStart < replacement.length &&
        lines[first + unchangedStart].sameContent(
          replacement[unchangedStart],
        )) {
      replacement[unchangedStart] = lines[first + unchangedStart];
      unchangedStart++;
    }
    var unchangedOldEnd = last;
    var unchangedNewEnd = replacement.length - 1;
    while (unchangedOldEnd >= first + unchangedStart &&
        unchangedNewEnd >= unchangedStart &&
        lines[unchangedOldEnd].sameContent(replacement[unchangedNewEnd])) {
      final line = lines[unchangedOldEnd];
      line.start = replacement[unchangedNewEnd].start;
      replacement[unchangedNewEnd] = line;
      unchangedOldEnd--;
      unchangedNewEnd--;
    }
    for (var i = first + unchangedStart; i <= unchangedOldEnd; i++) {
      _paragraphs.remove(lines[i])?.dispose();
      syntax.evict(lines[i]);
    }
    for (var i = last + 1; i < lines.length; i++) {
      lines[i].start += delta;
    }
    lines.replaceRange(first, last + 1, replacement);
    _text = text;
    syntax.synchronize(
      lines,
      first: first,
      through: first + replacement.length,
      onInvalidated: _invalidateSyntaxLine,
    );
    _rebuildHeights();
  }

  /// Releases a cached paragraph whose multiline lexical context has changed.
  void _invalidateSyntaxLine(CodeSyntaxLine source) {
    final line = source as CodeTextLine;
    _paragraphs.remove(line)?.dispose();
    line.layoutWidth = -1;
  }

  /// Updates grammar or token colors without reshaping the complete document.
  bool configureSyntax(CodeSyntaxLanguage language, CodeSyntaxPalette palette) {
    final changed = syntax.configure(language, lines);
    if (!changed && _syntaxPalette == palette) return false;
    _syntaxPalette = palette;
    _disposeParagraphs();
    return true;
  }

  /// Starts a new wrapping epoch without laying out unseen source lines.
  void configure(
    double width,
    TextStyle style,
    TextScaler scaler,
    TextDirection direction, {
    Locale? locale,
  }) {
    final appearanceChanged =
        _fontsChanged ||
        _style != style ||
        _textScaler != scaler ||
        _direction != direction ||
        _locale != locale;
    if (!appearanceChanged && _width == width) return;
    _style = style;
    _fontsChanged = false;
    _textScaler = scaler;
    _direction = direction;
    _locale = locale;
    _width = width;
    if (appearanceChanged) {
      _disposeParagraphs();
      final sample = _createPainter(TextSpan(text: '', style: _style));
      sample.layout(maxWidth: width);
      lineHeight = sample.preferredLineHeight;
      sample.dispose();
    }
    _epoch++;
    _measuredHeights = Float64List(lines.length + 1);
    _measuredCounts = Uint32List(lines.length + 1);
    _measuredTotal = 0;
    _measuredCount = 0;
    _estimatedHeight = lineHeight;
  }

  /// Requests a fresh font epoch without discarding valid geometry query state.
  void invalidateFonts() {
    _fontsChanged = true;
  }

  /// Creates a paragraph with one uniform row strut, including empty lines.
  TextPainter _createPainter(InlineSpan text) => TextPainter(
    text: text,
    textDirection: _direction,
    locale: _locale,
    textScaler: _textScaler!,
    strutStyle: StrutStyle.fromTextStyle(_style!, forceStrutHeight: true),
    textWidthBasis: TextWidthBasis.parent,
  );

  /// Lays out one requested source line and updates its indexed row height.
  TextPainter paragraph(int index, {bool updateHeight = true}) {
    final line = lines[index];
    final painter =
        _paragraphs.remove(line) ??
        _createPainter(syntax.span(line, _style, _syntaxPalette));
    _paragraphs[line] = painter;
    if (line.layoutWidth != _width || line.epoch != _epoch) {
      painter.layout(maxWidth: _width);
      assert(() {
        debugParagraphLayouts++;
        return true;
      }());
      line.height = math.max(lineHeight, painter.height);
      line.epoch = _epoch;
      line.layoutWidth = _width;
    }
    if (updateHeight && line.heightEpoch != _epoch) {
      _recordHeight(index, line.height!);
      line.indexedHeight = line.height!;
      line.heightEpoch = _epoch;
    }
    return painter;
  }

  /// Releases paragraphs outside the bounded viewport working set.
  void trimCache(Set<CodeTextLine> retained) {
    final capacity = math.max(cacheCapacity, retained.length);
    final victims = _paragraphs.keys
        .where((line) => !retained.contains(line))
        .take(math.max(0, _paragraphs.length - capacity))
        .toList();
    for (final line in victims) {
      _paragraphs.remove(line)!.dispose();
      line.layoutWidth = -1;
    }
    syntax.trimCache(retained, capacity);
  }

  /// Rebuilds both measured-height Fenwick trees in linear time after a splice.
  void _rebuildHeights() {
    _measuredHeights = Float64List(lines.length + 1);
    _measuredCounts = Uint32List(lines.length + 1);
    _measuredTotal = 0;
    _measuredCount = 0;
    for (var i = 1; i <= lines.length; i++) {
      final line = lines[i - 1];
      if (line.heightEpoch == _epoch) {
        _measuredHeights[i] += line.indexedHeight;
        _measuredCounts[i]++;
        _measuredTotal += line.indexedHeight;
        _measuredCount++;
      }
      final parent = i + (i & -i);
      if (parent <= lines.length) {
        _measuredHeights[parent] += _measuredHeights[i];
        _measuredCounts[parent] += _measuredCounts[i];
      }
    }
    _estimatedHeight = _measuredCount == 0
        ? lineHeight
        : _measuredTotal / _measuredCount;
  }

  /// Records one exact row height and refines the mean for unvisited lines.
  void _recordHeight(int index, double height) {
    for (var i = index + 1; i < _measuredHeights.length; i += i & -i) {
      _measuredHeights[i] += height;
      _measuredCounts[i]++;
    }
    _measuredTotal += height;
    _measuredCount++;
    _estimatedHeight = _measuredTotal / _measuredCount;
  }

  /// Returns a source-line origin from exact heights plus the unvisited mean.
  double topOf(int index) {
    var measured = 0.0;
    var count = 0;
    for (var i = index; i > 0; i -= i & -i) {
      measured += _measuredHeights[i];
      count += _measuredCounts[i];
    }
    return measured + (index - count) * _estimatedHeight;
  }

  /// Returns the evolving scroll extent of the virtualized document.
  double get height => topOf(lines.length);

  /// Finds a source line with one logarithmic Fenwick prefix search.
  int lineAtY(double y) {
    var index = 0;
    var height = 0.0;
    var step = 1;
    while (step * 2 <= lines.length) {
      step *= 2;
    }
    for (; step > 0; step ~/= 2) {
      final next = index + step;
      if (next > lines.length) continue;
      final blockHeight =
          _measuredHeights[next] +
          ((next & -next) - _measuredCounts[next]) * _estimatedHeight;
      if (height + blockHeight <= y) {
        index = next;
        height += blockHeight;
      }
    }
    return math.min(index, lines.length - 1);
  }

  /// Resolves an absolute UTF-16 offset without consulting paragraph geometry.
  int lineAtOffset(int offset) {
    var low = 0;
    var high = lines.length - 1;
    while (low < high) {
      final middle = (low + high + 1) ~/ 2;
      if (lines[middle].start <= offset) {
        low = middle;
      } else {
        high = middle - 1;
      }
    }
    return low;
  }

  /// Disposes native paragraph resources before replacing appearance or owner.
  void _disposeParagraphs() {
    for (final entry in _paragraphs.entries) {
      entry.value.dispose();
      entry.key.layoutWidth = -1;
    }
    _paragraphs.clear();
  }

  /// Releases all native paragraphs owned by this document layout.
  void dispose() {
    _disposeParagraphs();
    syntax.dispose();
  }
}

/// Identifies one logical line across local edits and cache eviction.
class CodeTextLine extends CodeSyntaxLine {
  /// Records the source offset, content, and exact original line separator.
  CodeTextLine(this.start, String content, this.separator) : super(content);

  int start;
  final String separator;
  double? height;
  double layoutWidth = -1;
  int epoch = -1;
  int heightEpoch = -1;
  double indexedHeight = 0;

  /// Compares source content without using viewport or source-offset identity.
  bool sameContent(CodeTextLine other) =>
      content == other.content && separator == other.separator;
}
