// ignore_for_file: file_names

import 'dart:collection';
import 'dart:math' as math;

import 'package:flutter/painting.dart';

import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxLanguage.dart';
import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxLexer.dart';
import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxPalette.dart';
import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxToken.dart';

/// Attaches lightweight lexical state to a persistent source-line identity.
class CodeSyntaxLine {
  /// Preserves source text independently from evictable visible token intervals.
  CodeSyntaxLine(this.content);

  final String content;
  CodeSyntaxState? syntaxStart;
  CodeSyntaxState? syntaxEnd;
}

/// Indexes multiline context while caching colored tokens only for viewed lines.
class CodeSyntaxHighlighter {
  /// Starts in the explicit plain-text format without doing source-wide lexing.
  CodeSyntaxHighlighter();

  CodeSyntaxLanguage _language = CodeSyntaxLanguage.plainText;
  final LinkedHashMap<CodeSyntaxLine, List<CodeSyntaxToken>> _tokens =
      LinkedHashMap();
  static final Map<CodeSyntaxLanguage, CodeSyntaxLexer> _lexers = {};
  int debugStateScans = 0;
  int debugTokenScans = 0;

  /// Exposes the selected grammar to render inspection and regression tests.
  CodeSyntaxLanguage get language => _language;

  /// Reports only retained tokenized lines, never total indexed source lines.
  int get cachedLines => _tokens.length;

  /// Resolves one shared compiled lexer for the explicitly selected grammar.
  CodeSyntaxLexer get _lexer =>
      _lexers.putIfAbsent(_language, () => CodeSyntaxLexer(_language));

  /// Reindexes lexical boundaries when a file's declared language changes.
  bool configure(CodeSyntaxLanguage language, List<CodeSyntaxLine> lines) {
    if (_language == language) return false;
    _language = language;
    _tokens.clear();
    for (final line in lines) {
      line.syntaxStart = null;
      line.syntaxEnd = null;
    }
    synchronize(lines, through: lines.length);
    return true;
  }

  /// Propagates edited lexical state only until the unchanged suffix converges.
  void synchronize(
    List<CodeSyntaxLine> lines, {
    int first = 0,
    required int through,
    void Function(CodeSyntaxLine line)? onInvalidated,
  }) {
    if (_language == CodeSyntaxLanguage.plainText || first >= lines.length) {
      return;
    }
    var state = first == 0
        ? CodeSyntaxState.empty
        : lines[first - 1].syntaxEnd!;
    for (var i = first; i < lines.length; i++) {
      final line = lines[i];
      if (i >= through && line.syntaxStart == state) break;
      if (line.syntaxStart != state || line.syntaxEnd == null) {
        _tokens.remove(line);
        onInvalidated?.call(line);
      }
      line.syntaxStart = state;
      state = _lexer.scan(line.content, state, tokens: false).state;
      line.syntaxEnd = state;
      assert(() {
        debugStateScans++;
        return true;
      }());
    }
  }

  /// Removes a deleted line's tokens without discarding unrelated cached lines.
  void evict(CodeSyntaxLine line) => _tokens.remove(line);

  /// Generates tokens only when their paragraph is requested for rendering.
  List<CodeSyntaxToken> tokens(CodeSyntaxLine line) {
    if (_language == CodeSyntaxLanguage.plainText) return const [];
    final cached = _tokens.remove(line);
    if (cached != null) {
      _tokens[line] = cached;
      return cached;
    }
    final result = _lexer.scan(line.content, line.syntaxStart!);
    assert(
      result.state == line.syntaxEnd,
      'State indexing and visible lexing must agree',
    );
    assert(() {
      debugTokenScans++;
      return true;
    }());
    _tokens[line] = result.tokens;
    return result.tokens;
  }

  /// Applies syntax and optional caller-owned emphasis without changing source.
  TextSpan span(
    CodeSyntaxLine line,
    TextStyle? style,
    CodeSyntaxPalette palette, {
    List<CodeTextHighlight> highlights = const [],
  }) {
    final intervals = tokens(line);
    final cuts = <int>{
      0,
      line.content.length,
      for (final token in intervals) ...[token.start, token.end],
      for (final highlight in highlights) ...[highlight.start, highlight.end],
    }.toList()..sort();
    var tokenIndex = 0;
    final children = <InlineSpan>[];
    for (var i = 0; i < cuts.length - 1; i++) {
      final start = cuts[i], end = cuts[i + 1];
      if (start == end) continue;
      while (tokenIndex < intervals.length &&
          intervals[tokenIndex].end <= start) {
        tokenIndex++;
      }
      TextStyle? paint =
          tokenIndex < intervals.length && intervals[tokenIndex].start <= start
          ? palette.style(intervals[tokenIndex].kind)
          : null;
      for (final highlight in highlights) {
        if (highlight.start <= start && highlight.end >= end) {
          paint = (paint ?? const TextStyle()).merge(highlight.style);
        }
      }
      children.add(
        TextSpan(text: line.content.substring(start, end), style: paint),
      );
    }
    return TextSpan(style: style, children: children);
  }

  /// Caps visible token storage independently from lightweight multiline states.
  void trimCache(Set<CodeSyntaxLine> retained, int capacity) {
    final limit = math.max(capacity, retained.length);
    final victims = _tokens.keys
        .where((line) => !retained.contains(line))
        .take(math.max(0, _tokens.length - limit))
        .toList();
    for (final line in victims) {
      _tokens.remove(line);
    }
  }

  /// Releases all colored intervals when the owning document is disposed.
  void dispose() => _tokens.clear();
}

/// A UTF-16 interval layered over the editor's syntax colors.
class CodeTextHighlight {
  const CodeTextHighlight(this.start, this.end, this.style)
    : assert(start >= 0 && end >= start);
  final int start;
  final int end;
  final TextStyle style;
}
