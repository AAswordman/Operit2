// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import 'file_preview/syntax/WorkspaceSyntaxLanguage.dart';
import 'file_preview/syntax/WorkspaceSyntaxLexer.dart';
import 'file_preview/syntax/WorkspaceSyntaxPalette.dart';
import 'file_preview/syntax/WorkspaceSyntaxToken.dart';

enum InlineDiffKind { context, add, remove }

/// A changed character range inside one diff line, used for word-level emphasis.
class DiffSegment {
  const DiffSegment({required this.start, required this.end});

  final int start;
  final int end;
}

class InlineDiffRow {
  const InlineDiffRow({
    required this.lineNumber,
    required this.text,
    required this.kind,
    this.oldLineNumber,
    this.newLineNumber,
    this.segments = const <DiffSegment>[],
  });

  final int lineNumber;
  final String text;
  final InlineDiffKind kind;
  final int? oldLineNumber;
  final int? newLineNumber;
  final List<DiffSegment> segments;
}

/// One `@@` hunk with its source ranges and parsed rows.
class UnifiedHunk {
  const UnifiedHunk({
    required this.header,
    required this.oldStart,
    required this.oldCount,
    required this.newStart,
    required this.newCount,
    required this.rows,
  });

  final String header;
  final int oldStart;
  final int oldCount;
  final int newStart;
  final int newCount;
  final List<InlineDiffRow> rows;

  int get oldEndExclusive => oldStart + oldCount;

  int get newEndExclusive => newStart + newCount;
}

/// Parsed unified diff: hunks plus aggregate statistics.
class WorkspaceDiffDocument {
  const WorkspaceDiffDocument({required this.hunks});

  final List<UnifiedHunk> hunks;

  int get additions => hunks.fold(
    0,
    (total, hunk) =>
        total +
        hunk.rows.where((row) => row.kind == InlineDiffKind.add).length,
  );

  int get deletions => hunks.fold(
    0,
    (total, hunk) =>
        total +
        hunk.rows.where((row) => row.kind == InlineDiffKind.remove).length,
  );

  int get maxLineLength => hunks.fold(
    0,
    (maxLength, hunk) => hunk.rows.fold(
      maxLength,
      (inner, row) => math.max(inner, _visualWidth(row.text)),
    ),
  );

  static int _visualWidth(String text) {
    var width = 0;
    for (final rune in text.runes) {
      if (rune == 0x09) {
        width += 4;
      } else if (rune > 0x7F) {
        width += 2;
      } else {
        width += 1;
      }
    }
    return width;
  }

  static WorkspaceDiffDocument parse(
    String diff, {
    bool allowImplicitHunks = false,
  }) {
    final hunks = <UnifiedHunk>[];
    final headerPattern = RegExp(
      r'^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@.*',
    );
    String? header;
    var oldStart = 0;
    var oldCount = 0;
    var newStart = 0;
    var newCount = 0;
    var oldLine = 1;
    var newLine = 1;
    List<InlineDiffRow>? rows;

    void flush() {
      final current = rows;
      final currentHeader = header;
      if (current != null && current.isNotEmpty && currentHeader != null) {
        hunks.add(
          UnifiedHunk(
            header: currentHeader,
            oldStart: oldStart,
            oldCount: oldCount,
            newStart: newStart,
            newCount: newCount,
            rows: _withIntraLineSegments(current),
          ),
        );
      }
    }

    for (final raw in diff.split('\n')) {
      final line = raw.endsWith('\r') ? raw.substring(0, raw.length - 1) : raw;
      final match = headerPattern.firstMatch(line);
      if (match != null) {
        flush();
        header = line;
        oldStart = int.parse(match.group(1)!);
        oldCount = int.parse(match.group(2) ?? '1');
        newStart = int.parse(match.group(3)!);
        newCount = int.parse(match.group(4) ?? '1');
        oldLine = oldStart;
        newLine = newStart;
        rows = <InlineDiffRow>[];
        continue;
      }
      // Headerless fragments (chat tool payloads) become one implicit hunk so
      // stray +/- lines stay visible without weakening workspace parsing.
      if (allowImplicitHunks && rows == null && _isImplicitDiffLine(line)) {
        header = '@@';
        oldStart = 1;
        oldCount = 0;
        newStart = 1;
        newCount = 0;
        oldLine = 1;
        newLine = 1;
        rows = <InlineDiffRow>[];
      }
      final current = rows;
      if (current == null || line.startsWith('\\')) {
        continue;
      }
      if (line.startsWith('+')) {
        current.add(
          InlineDiffRow(
            lineNumber: newLine,
            newLineNumber: newLine,
            text: line.substring(1),
            kind: InlineDiffKind.add,
          ),
        );
        newLine += 1;
      } else if (line.startsWith('-')) {
        current.add(
          InlineDiffRow(
            lineNumber: oldLine,
            oldLineNumber: oldLine,
            text: line.substring(1),
            kind: InlineDiffKind.remove,
          ),
        );
        oldLine += 1;
      } else if (line.startsWith(' ')) {
        current.add(
          InlineDiffRow(
            lineNumber: newLine,
            oldLineNumber: oldLine,
            newLineNumber: newLine,
            text: line.substring(1),
            kind: InlineDiffKind.context,
          ),
        );
        oldLine += 1;
        newLine += 1;
      }
    }
    flush();
    return WorkspaceDiffDocument(hunks: hunks);
  }

  static bool _isImplicitDiffLine(String line) {
    if (line.startsWith('+++') || line.startsWith('---')) return false;
    if (line.startsWith('diff ') || line.startsWith('index ')) return false;
    return line.startsWith('+') || line.startsWith('-');
  }
}

/// Pairs adjacent removed/added rows inside one hunk and computes the
/// changed character ranges for word-level highlighting.
List<InlineDiffRow> _withIntraLineSegments(List<InlineDiffRow> rows) {
  final result = List<InlineDiffRow>.of(rows);
  var index = 0;
  while (index < result.length) {
    if (result[index].kind != InlineDiffKind.remove) {
      index += 1;
      continue;
    }
    final removeStart = index;
    while (index < result.length &&
        result[index].kind == InlineDiffKind.remove) {
      index += 1;
    }
    final removeEnd = index;
    final addStart = index;
    while (index < result.length && result[index].kind == InlineDiffKind.add) {
      index += 1;
    }
    final addEnd = index;
    final pairCount = math.min(removeEnd - removeStart, addEnd - addStart);
    for (var pair = 0; pair < pairCount; pair += 1) {
      final removeIndex = removeStart + pair;
      final addIndex = addStart + pair;
      final removed = result[removeIndex];
      final added = result[addIndex];
      final (removedSegments, addedSegments) = _changedRanges(
        removed.text,
        added.text,
      );
      result[removeIndex] = InlineDiffRow(
        lineNumber: removed.lineNumber,
        text: removed.text,
        kind: removed.kind,
        oldLineNumber: removed.oldLineNumber,
        newLineNumber: removed.newLineNumber,
        segments: removedSegments,
      );
      result[addIndex] = InlineDiffRow(
        lineNumber: added.lineNumber,
        text: added.text,
        kind: added.kind,
        oldLineNumber: added.oldLineNumber,
        newLineNumber: added.newLineNumber,
        segments: addedSegments,
      );
    }
  }
  return result;
}

/// Finds the changed middle range after trimming the common prefix/suffix.
(List<DiffSegment>, List<DiffSegment>) _changedRanges(
  String oldText,
  String newText,
) {
  final shared = math.min(oldText.length, newText.length);
  var prefix = 0;
  while (prefix < shared && oldText[prefix] == newText[prefix]) {
    prefix += 1;
  }
  var suffix = 0;
  while (suffix < shared - prefix &&
      oldText[oldText.length - 1 - suffix] ==
          newText[newText.length - 1 - suffix]) {
    suffix += 1;
  }
  final oldEnd = oldText.length - suffix;
  final newEnd = newText.length - suffix;
  return (
    oldEnd > prefix
        ? <DiffSegment>[DiffSegment(start: prefix, end: oldEnd)]
        : const <DiffSegment>[],
    newEnd > prefix
        ? <DiffSegment>[DiffSegment(start: prefix, end: newEnd)]
        : const <DiffSegment>[],
  );
}

List<List<InlineDiffRow>> parseUnifiedHunks(String diff) {
  return WorkspaceDiffDocument.parse(
    diff,
  ).hunks.map((hunk) => hunk.rows).toList(growable: false);
}

List<String> parseUnifiedHunkHeaders(String diff) {
  return WorkspaceDiffDocument.parse(
    diff,
  ).hunks.map((hunk) => hunk.header).toList(growable: false);
}

({int additions, int deletions}) unifiedDiffStats(String diff) {
  final document = WorkspaceDiffDocument.parse(diff);
  return (additions: document.additions, deletions: document.deletions);
}

/// Unified (single column) diff with foldable gaps between hunks.
class WorkspaceInlineDiff extends StatefulWidget {
  const WorkspaceInlineDiff({
    super.key,
    required this.diff,
    this.filePath,
    this.wrapLines = false,
    this.fileLines,
    this.showFullFile = false,
    this.embedded = false,
    this.embeddedMaxHeight = 240,
    this.allowImplicitHunks = false,
  });

  final String diff;
  final String? filePath;
  final bool wrapLines;

  /// Current text of the changed file, used to fill the unchanged regions that
  /// the diff payload omits. When null those regions stay collapsed.
  final List<String>? fileLines;

  /// When true every unchanged region renders as real lines instead of a bar.
  final bool showFullFile;

  /// Chat embedding: intrinsic height up to [embeddedMaxHeight] with its own
  /// vertical scrolling, never relying on parent viewport height.
  final bool embedded;

  final double embeddedMaxHeight;

  /// Accepts headerless +/- fragments as one implicit hunk (chat payloads).
  final bool allowImplicitHunks;

  @override
  State<WorkspaceInlineDiff> createState() => _WorkspaceInlineDiffState();
}

class _WorkspaceInlineDiffState extends State<WorkspaceInlineDiff> {
  late WorkspaceDiffDocument _document = WorkspaceDiffDocument.parse(
    widget.diff,
    allowImplicitHunks: widget.allowImplicitHunks,
  );
  late WorkspaceSyntaxLexer? _lexer = _resolveLexer(widget.filePath);
  final ScrollController _embeddedScrollController = ScrollController();

  @override
  void dispose() {
    _embeddedScrollController.dispose();
    super.dispose();
  }

  /// Gaps the user expanded by tapping, keyed by their first new-file line so
  /// the choice survives a rebuild.
  final Set<int> _expandedGaps = <int>{};

  static WorkspaceSyntaxLexer? _resolveLexer(String? filePath) {
    if (filePath == null || filePath.isEmpty) return null;
    final language = WorkspaceSyntaxLanguage.forPath(filePath);
    if (language == WorkspaceSyntaxLanguage.plainText ||
        language == WorkspaceSyntaxLanguage.diff) {
      return null;
    }
    return WorkspaceSyntaxLexer(language);
  }

  @override
  void didUpdateWidget(WorkspaceInlineDiff oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.diff != widget.diff) {
      _document = WorkspaceDiffDocument.parse(
        widget.diff,
        allowImplicitHunks: widget.allowImplicitHunks,
      );
      _expandedGaps.clear();
    }
    if (oldWidget.filePath != widget.filePath) {
      _lexer = _resolveLexer(widget.filePath);
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_document.hunks.isEmpty) {
      return WorkspaceDiffEmptyState(
        message: '没有可显示的行级变更',
        compact: widget.embedded,
      );
    }
    final palette = WorkspaceSyntaxPalette(Theme.of(context).brightness);
    final items = _unifiedItems(
      _document,
      fileLines: widget.fileLines,
      expandedGaps: widget.showFullFile ? null : _expandedGaps,
    );
    Widget list = ListView.builder(
      controller: widget.embedded ? _embeddedScrollController : null,
      shrinkWrap: widget.embedded,
      physics: widget.embedded ? const ClampingScrollPhysics() : null,
      padding: widget.embedded
          ? const EdgeInsets.symmetric(vertical: 4)
          : const EdgeInsets.fromLTRB(0, 4, 0, 24),
      itemCount: items.length,
      itemBuilder: (context, index) {
        final item = items[index];
        return switch (item) {
          _GapItem(:final count, :final newStartLine, :final expandable) =>
            _UnchangedGapBar(
              count: count,
              expandable: expandable,
              onExpand: newStartLine == null
                  ? null
                  : () => setState(() => _expandedGaps.add(newStartLine)),
            ),
          _HunkItem(:final header) => widget.embedded
              ? const SizedBox.shrink()
              : _HunkHeaderBar(header: header),
          _LineItem(:final row) => _UnifiedDiffLine(
            row: row,
            lexer: _lexer,
            palette: palette,
            wrapLines: widget.wrapLines,
            embedded: widget.embedded,
          ),
          _PairItem() => throw StateError('unreachable'),
        };
      },
    );
    if (widget.embedded) {
      list = ConstrainedBox(
        constraints: BoxConstraints(maxHeight: widget.embeddedMaxHeight),
        child: Scrollbar(
          controller: _embeddedScrollController,
          child: list,
        ),
      );
    }
    return _DiffScrollSurface(
      maxLineLength: _document.maxLineLength,
      gutterWidth: _unifiedGutterWidth,
      wrapLines: widget.wrapLines,
      embedded: widget.embedded,
      child: list,
    );
  }
}

/// Side-by-side diff; removed and added lines are aligned per pair.
class WorkspaceSplitDiff extends StatefulWidget {
  const WorkspaceSplitDiff({
    super.key,
    required this.diff,
    this.filePath,
    this.wrapLines = false,
    this.fileLines,
    this.showFullFile = false,
  });

  final String diff;
  final String? filePath;
  final bool wrapLines;

  /// Current text of the changed file, used to fill the unchanged regions that
  /// the diff payload omits. When null those regions stay collapsed.
  final List<String>? fileLines;

  /// When true every unchanged region renders as real lines instead of a bar.
  final bool showFullFile;

  @override
  State<WorkspaceSplitDiff> createState() => _WorkspaceSplitDiffState();
}

class _WorkspaceSplitDiffState extends State<WorkspaceSplitDiff> {
  late WorkspaceDiffDocument _document = WorkspaceDiffDocument.parse(
    widget.diff,
  );
  late WorkspaceSyntaxLexer? _lexer = _resolveLexer(widget.filePath);

  final Set<int> _expandedGaps = <int>{};

  static WorkspaceSyntaxLexer? _resolveLexer(String? filePath) {
    if (filePath == null || filePath.isEmpty) return null;
    final language = WorkspaceSyntaxLanguage.forPath(filePath);
    if (language == WorkspaceSyntaxLanguage.plainText ||
        language == WorkspaceSyntaxLanguage.diff) {
      return null;
    }
    return WorkspaceSyntaxLexer(language);
  }

  @override
  void didUpdateWidget(WorkspaceSplitDiff oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.diff != widget.diff) {
      _document = WorkspaceDiffDocument.parse(widget.diff);
      _expandedGaps.clear();
    }
    if (oldWidget.filePath != widget.filePath) {
      _lexer = _resolveLexer(widget.filePath);
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_document.hunks.isEmpty) {
      return const WorkspaceDiffEmptyState(message: '没有可显示的行级变更');
    }
    final palette = WorkspaceSyntaxPalette(Theme.of(context).brightness);
    final items = _splitItems(
      _document,
      fileLines: widget.fileLines,
      expandedGaps: widget.showFullFile ? null : _expandedGaps,
    );
    return _DiffScrollSurface(
      maxLineLength: _document.maxLineLength,
      gutterWidth: _splitGutterWidth,
      split: true,
      wrapLines: widget.wrapLines,
      child: ListView.builder(
        padding: const EdgeInsets.fromLTRB(0, 4, 0, 24),
        itemCount: items.length,
        itemBuilder: (context, index) {
          final item = items[index];
          return switch (item) {
            _GapItem(:final count, :final newStartLine, :final expandable) =>
              _UnchangedGapBar(
                count: count,
                expandable: expandable,
                onExpand: newStartLine == null
                    ? null
                    : () => setState(() => _expandedGaps.add(newStartLine)),
              ),
            _PairItem(:final left, :final right) => _SplitDiffRow(
                left: left,
                right: right,
                lexer: _lexer,
                palette: palette,
                wrapLines: widget.wrapLines,
              ),
            _HunkItem() => throw StateError('unreachable'),
            _LineItem() => throw StateError('unreachable'),
          };
        },
      ),
    );
  }
}

const double _unifiedGutterWidth = 3 + 42 + 8;
const double _splitGutterWidth = 3 + 42 + 6;
const double _kDiffLineHeight = 22.0;

typedef _DiffColorSet = ({
  Color background,
  Color accent,
  Color foreground,
  Color highlight,
  Color lineNumber,
  Color lineNumberBackground,
});

sealed class _DiffItem {
  const _DiffItem();
}

class _GapItem extends _DiffItem {
  const _GapItem({
    required this.count,
    this.oldStartLine,
    this.newStartLine,
    this.expandable = false,
  });
  final int count;
  final int? oldStartLine;
  final int? newStartLine;
  final bool expandable;
}

class _HunkItem extends _DiffItem {
  const _HunkItem(this.header);
  final String header;
}

class _LineItem extends _DiffItem {
  const _LineItem(this.row);
  final InlineDiffRow row;
}

class _PairItem extends _DiffItem {
  const _PairItem({required this.left, required this.right});
  final InlineDiffRow? left;
  final InlineDiffRow? right;
}

List<_DiffItem> _unifiedItems(
  WorkspaceDiffDocument document, {
  List<String>? fileLines,
  Set<int>? expandedGaps,
}) {
  final items = <_DiffItem>[];
  var previousOldEnd = 1;
  var previousNewEnd = 1;
  var first = true;
  for (final hunk in document.hunks) {
    final gap = first ? hunk.oldStart - 1 : hunk.oldStart - previousOldEnd;
    final oldStartLine = first ? 1 : previousOldEnd;
    final newStartLine = first ? 1 : previousNewEnd;
    if (gap > 0) {
      final expanded = fileLines != null &&
          (expandedGaps == null || expandedGaps.contains(newStartLine));
      final resolved = expanded
          ? _resolveGapRows(fileLines, oldStartLine, newStartLine, gap)
          : null;
      if (resolved != null && resolved.isNotEmpty) {
        for (final row in resolved) {
          items.add(_LineItem(row));
        }
      } else {
        items.add(
          _GapItem(
            count: gap,
            oldStartLine: oldStartLine,
            newStartLine: newStartLine,
            expandable: fileLines != null,
          ),
        );
      }
    }
    first = false;
    previousOldEnd = hunk.oldEndExclusive;
    previousNewEnd = hunk.newEndExclusive;
    // Implicit headerless fragments (chat payloads) have no @@ bar to show.
    if (hunk.header != '@@') {
      items.add(_HunkItem(hunk.header));
    }
    for (final row in hunk.rows) {
      items.add(_LineItem(row));
    }
  }
  return items;
}

/// Resolves the unchanged region between hunks into real rows using the current
/// file text. Returns null when the text does not cover the whole gap, so the
/// caller keeps the compact placeholder instead of showing partial garbage.
List<InlineDiffRow>? _resolveGapRows(
  List<String> fileLines,
  int oldStartLine,
  int newStartLine,
  int count,
) {
  if (count <= 0) return const <InlineDiffRow>[];
  final rows = <InlineDiffRow>[];
  for (var offset = 0; offset < count; offset += 1) {
    final oldLine = oldStartLine + offset;
    final newLine = newStartLine + offset;
    final index = newLine - 1;
    if (index < 0 || index >= fileLines.length) return null;
    rows.add(
      InlineDiffRow(
        lineNumber: newLine,
        text: fileLines[index],
        kind: InlineDiffKind.context,
        oldLineNumber: oldLine,
        newLineNumber: newLine,
      ),
    );
  }
  return rows;
}

List<_DiffItem> _splitItems(
  WorkspaceDiffDocument document, {
  List<String>? fileLines,
  Set<int>? expandedGaps,
}) {
  final items = <_DiffItem>[];
  var previousOldEnd = 1;
  var previousNewEnd = 1;
  var first = true;
  for (final hunk in document.hunks) {
    final gap = first ? hunk.oldStart - 1 : hunk.oldStart - previousOldEnd;
    final oldStartLine = first ? 1 : previousOldEnd;
    final newStartLine = first ? 1 : previousNewEnd;
    if (gap > 0) {
      final expanded = fileLines != null &&
          (expandedGaps == null || expandedGaps.contains(newStartLine));
      final resolved = expanded
          ? _resolveGapRows(fileLines, oldStartLine, newStartLine, gap)
          : null;
      if (resolved != null && resolved.isNotEmpty) {
        for (final row in resolved) {
          items.add(_PairItem(left: row, right: row));
        }
      } else {
        items.add(
          _GapItem(
            count: gap,
            oldStartLine: oldStartLine,
            newStartLine: newStartLine,
            expandable: fileLines != null,
          ),
        );
      }
    }
    first = false;
    previousOldEnd = hunk.oldEndExclusive;
    previousNewEnd = hunk.newEndExclusive;
    final rows = hunk.rows;
    var index = 0;
    while (index < rows.length) {
      final row = rows[index];
      if (row.kind == InlineDiffKind.context) {
        items.add(_PairItem(left: row, right: row));
        index += 1;
        continue;
      }
      final removeStart = index;
      while (index < rows.length && rows[index].kind == InlineDiffKind.remove) {
        index += 1;
      }
      final removeEnd = index;
      final addStart = index;
      while (index < rows.length && rows[index].kind == InlineDiffKind.add) {
        index += 1;
      }
      final addEnd = index;
      final removeCount = removeEnd - removeStart;
      final addCount = addEnd - addStart;
      final pairCount = math.max(removeCount, addCount);
      for (var pair = 0; pair < pairCount; pair += 1) {
        items.add(
          _PairItem(
            left: pair < removeCount ? rows[removeStart + pair] : null,
            right: pair < addCount ? rows[addStart + pair] : null,
          ),
        );
      }
    }
  }
  return items;
}

/// Horizontal scroll surface shared by both layouts so long lines are never
/// clipped.
class _DiffScrollSurface extends StatefulWidget {
  const _DiffScrollSurface({
    required this.child,
    required this.maxLineLength,
    required this.gutterWidth,
    this.split = false,
    this.wrapLines = false,
    this.embedded = false,
  });

  final Widget child;
  final int maxLineLength;
  final double gutterWidth;
  final bool split;
  final bool wrapLines;
  final bool embedded;

  @override
  State<_DiffScrollSurface> createState() => _DiffScrollSurfaceState();
}

class _DiffScrollSurfaceState extends State<_DiffScrollSurface> {
  final ScrollController _horizontalController = ScrollController();

  @override
  void dispose() {
    _horizontalController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (widget.wrapLines) {
      return widget.child;
    }

    const charWidth = 12.0 * 0.65;
    return LayoutBuilder(
      builder: (context, constraints) {
        final textWidth = widget.maxLineLength * charWidth + 48;
        final contentWidth = widget.split
            ? math.max(constraints.maxWidth, 2 * (widget.gutterWidth + textWidth))
            : math.max(constraints.maxWidth, widget.gutterWidth + textWidth);
        // No overflow means no scrollbar: an always-painted track would show
        // as a dark strip under short diffs.
        if (contentWidth <= constraints.maxWidth) {
          return widget.child;
        }
        return Scrollbar(
          controller: _horizontalController,
          thumbVisibility: !widget.embedded,
          trackVisibility: !widget.embedded,
          child: SingleChildScrollView(
            controller: _horizontalController,
            scrollDirection: Axis.horizontal,
            child: SizedBox(
              width: contentWidth,
              // Embedded (chat) usage sizes to the content height; the
              // workspace viewport supplies an explicit height instead.
              height: widget.embedded ? null : constraints.maxHeight,
              child: widget.child,
            ),
          ),
        );
      },
    );
  }
}

/// Bar shown between hunks for unchanged regions whose content is not part
/// of the diff payload. Becomes tappable when the caller can supply the file
/// text, so one gap can be expanded without switching the whole view.
class _UnchangedGapBar extends StatelessWidget {
  const _UnchangedGapBar({
    required this.count,
    this.expandable = false,
    this.onExpand,
  });

  final int count;
  final bool expandable;
  final VoidCallback? onExpand;

  bool get _tappable => expandable && onExpand != null;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final content = Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        children: <Widget>[
          Expanded(
            child: Divider(
              height: 1,
              color: scheme.outlineVariant.withValues(alpha: 0.4),
            ),
          ),
          Container(
            margin: const EdgeInsets.symmetric(horizontal: 10),
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 3),
            decoration: BoxDecoration(
              color: scheme.surfaceContainerHighest.withValues(alpha: 0.7),
              borderRadius: BorderRadius.circular(20),
              border: Border.all(
                color: scheme.outlineVariant.withValues(alpha: 0.4),
              ),
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Icon(
                  _tappable ? Icons.unfold_more : Icons.unfold_less,
                  size: 12,
                  color: _tappable ? scheme.primary : scheme.outline,
                ),
                const SizedBox(width: 5),
                Text(
                  '$count 行未更改',
                  style: TextStyle(
                    color: _tappable ? scheme.primary : scheme.outline,
                    fontSize: 10.5,
                  ),
                ),
              ],
            ),
          ),
          Expanded(
            child: Divider(
              height: 1,
              color: scheme.outlineVariant.withValues(alpha: 0.4),
            ),
          ),
        ],
      ),
    );
    if (!_tappable) return content;
    return InkWell(
      onTap: onExpand,
      child: Tooltip(
        message: '展开这 $count 行未更改内容',
        child: content,
      ),
    );
  }
}

class _UnifiedDiffLine extends StatelessWidget {
  const _UnifiedDiffLine({
    required this.row,
    this.lexer,
    this.palette,
    this.wrapLines = false,
    this.embedded = false,
  });

  final InlineDiffRow row;
  final WorkspaceSyntaxLexer? lexer;
  final WorkspaceSyntaxPalette? palette;
  final bool wrapLines;
  final bool embedded;

  @override
  Widget build(BuildContext context) {
    final colors = _diffColors(context, row.kind);
    if (wrapLines) {
      return Container(
        constraints: const BoxConstraints(minHeight: _kDiffLineHeight),
        decoration: BoxDecoration(
          color: row.kind != InlineDiffKind.context ? colors.background : null,
          border: embedded
              ? null
              : Border(left: BorderSide(color: colors.accent, width: 3)),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            _LineNumber(
              value: switch (row.kind) {
                InlineDiffKind.add => row.newLineNumber,
                InlineDiffKind.remove => row.oldLineNumber,
                InlineDiffKind.context => row.newLineNumber,
              },
              color: colors.lineNumber,
              background: colors.lineNumberBackground,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: 3),
                child: _DiffLineText(
                  row: row,
                  colors: colors,
                  lexer: lexer,
                  palette: palette,
                  wrapLines: true,
                ),
              ),
            ),
          ],
        ),
      );
    }
    return SizedBox(
      height: _kDiffLineHeight,
      child: Stack(
        clipBehavior: Clip.none,
        children: <Widget>[
          if (row.kind != InlineDiffKind.context)
            Positioned(
              left: 0,
              right: 0,
              top: 0,
              bottom: -1,
              child: ColoredBox(color: colors.background),
            ),
          Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: <Widget>[
              if (!embedded)
                Container(
                  width: 3,
                  height: double.infinity,
                  color: colors.accent,
                ),
              _LineNumber(
                value: switch (row.kind) {
                  InlineDiffKind.add => row.newLineNumber,
                  InlineDiffKind.remove => row.oldLineNumber,
                  InlineDiffKind.context => row.newLineNumber,
                },
                color: colors.lineNumber,
                background: embedded ? Colors.transparent : colors.lineNumberBackground,
                embedded: embedded,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: _DiffLineText(
                  row: row,
                  colors: colors,
                  lexer: lexer,
                  palette: palette,
                  wrapLines: false,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _SplitDiffRow extends StatelessWidget {
  const _SplitDiffRow({
    required this.left,
    required this.right,
    this.lexer,
    this.palette,
    this.wrapLines = false,
  });

  final InlineDiffRow? left;
  final InlineDiffRow? right;
  final WorkspaceSyntaxLexer? lexer;
  final WorkspaceSyntaxPalette? palette;
  final bool wrapLines;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    if (wrapLines) {
      return Container(
        constraints: const BoxConstraints(minHeight: _kDiffLineHeight),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Expanded(
              child: _SplitDiffCell(
                row: left,
                added: false,
                lexer: lexer,
                palette: palette,
                wrapLines: true,
              ),
            ),
            Container(
              width: 1,
              color: scheme.outlineVariant.withValues(alpha: 0.32),
            ),
            Expanded(
              child: _SplitDiffCell(
                row: right,
                added: true,
                lexer: lexer,
                palette: palette,
                wrapLines: true,
              ),
            ),
          ],
        ),
      );
    }
    return SizedBox(
      height: _kDiffLineHeight,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Expanded(
            child: _SplitDiffCell(
              row: left,
              added: false,
              lexer: lexer,
              palette: palette,
              wrapLines: false,
            ),
          ),
          Container(
            width: 1,
            color: scheme.outlineVariant.withValues(alpha: 0.32),
          ),
          Expanded(
            child: _SplitDiffCell(
              row: right,
              added: true,
              lexer: lexer,
              palette: palette,
              wrapLines: false,
            ),
          ),
        ],
      ),
    );
  }
}

class _SplitDiffCell extends StatelessWidget {
  const _SplitDiffCell({
    required this.row,
    required this.added,
    this.lexer,
    this.palette,
    this.wrapLines = false,
  });

  final InlineDiffRow? row;
  final bool added;
  final WorkspaceSyntaxLexer? lexer;
  final WorkspaceSyntaxPalette? palette;
  final bool wrapLines;

  @override
  Widget build(BuildContext context) {
    final row = this.row;
    if (row == null) {
      final theme = Theme.of(context);
      final emptyBg = Color.alphaBlend(
        theme.colorScheme.surfaceContainerHighest.withValues(
          alpha: theme.brightness == Brightness.dark ? 0.20 : 0.12,
        ),
        theme.colorScheme.surface,
      );
      if (wrapLines) {
        return Container(
          constraints: const BoxConstraints(minHeight: _kDiffLineHeight),
          color: emptyBg,
        );
      }
      return SizedBox(
        height: _kDiffLineHeight,
        child: ColoredBox(
          color: emptyBg,
          child: const SizedBox.expand(),
        ),
      );
    }
    final colors = _diffColors(context, row.kind);
    if (wrapLines) {
      return Container(
        constraints: const BoxConstraints(minHeight: _kDiffLineHeight),
        decoration: BoxDecoration(
          color: row.kind != InlineDiffKind.context ? colors.background : null,
          border: Border(left: BorderSide(color: colors.accent, width: 3)),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            _LineNumber(
              value: added ? row.newLineNumber : row.oldLineNumber,
              color: colors.lineNumber,
              background: colors.lineNumberBackground,
            ),
            const SizedBox(width: 6),
            Expanded(
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: 3),
                child: _DiffLineText(
                  row: row,
                  colors: colors,
                  lexer: lexer,
                  palette: palette,
                  wrapLines: true,
                ),
              ),
            ),
          ],
        ),
      );
    }
    return SizedBox(
      height: _kDiffLineHeight,
      child: Stack(
        clipBehavior: Clip.none,
        children: <Widget>[
          if (row.kind != InlineDiffKind.context)
            Positioned(
              left: 0,
              right: 0,
              top: 0,
              bottom: -1,
              child: ColoredBox(color: colors.background),
            ),
          Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: <Widget>[
              Container(
                width: 3,
                height: double.infinity,
                color: colors.accent,
              ),
              _LineNumber(
                value: added ? row.newLineNumber : row.oldLineNumber,
                color: colors.lineNumber,
                background: colors.lineNumberBackground,
              ),
              const SizedBox(width: 6),
              Expanded(
                child: _DiffLineText(
                  row: row,
                  colors: colors,
                  lexer: lexer,
                  palette: palette,
                  wrapLines: false,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

/// Line content; renders syntax highlighted tokens and word-level diff segments.
class _DiffLineText extends StatelessWidget {
  const _DiffLineText({
    required this.row,
    required this.colors,
    this.lexer,
    this.palette,
    this.wrapLines = false,
  });

  final InlineDiffRow row;
  final _DiffColorSet colors;
  final WorkspaceSyntaxLexer? lexer;
  final WorkspaceSyntaxPalette? palette;
  final bool wrapLines;

  @override
  Widget build(BuildContext context) {
    final baseStyle = TextStyle(
      color: colors.foreground,
      fontFamily: 'Consolas',
      fontFamilyFallback: const <String>['Cascadia Mono', 'monospace'],
      fontSize: 12,
      height: 1.0,
    );
    final text = row.text;
    if (text.isEmpty) {
      return const SizedBox();
    }

    final segments = row.segments;
    final tokens =
        lexer?.scan(text, WorkspaceSyntaxState.empty, tokens: true).tokens ??
        const <WorkspaceSyntaxToken>[];

    if (tokens.isEmpty && segments.isEmpty) {
      return Align(
        alignment: Alignment.centerLeft,
        child: Text(
          text,
          maxLines: wrapLines ? null : 1,
          softWrap: wrapLines,
          style: baseStyle,
        ),
      );
    }

    final cutPoints = <int>{0, text.length};
    for (final seg in segments) {
      cutPoints.add(seg.start.clamp(0, text.length));
      cutPoints.add(seg.end.clamp(0, text.length));
    }
    for (final tok in tokens) {
      cutPoints.add(tok.start.clamp(0, text.length));
      cutPoints.add(tok.end.clamp(0, text.length));
    }
    final sortedCuts = cutPoints.toList()..sort();

    final spans = <TextSpan>[];
    var tokenIndex = 0;
    for (var i = 0; i < sortedCuts.length - 1; i++) {
      final start = sortedCuts[i];
      final end = sortedCuts[i + 1];
      if (start >= end) continue;

      while (tokenIndex < tokens.length && tokens[tokenIndex].end <= start) {
        tokenIndex++;
      }

      WorkspaceSyntaxTokenKind? tokenKind;
      if (tokenIndex < tokens.length) {
        final tok = tokens[tokenIndex];
        if (tok.start <= start && end <= tok.end) {
          tokenKind = tok.kind;
        }
      }

      final inSegment = segments.any((s) => s.start <= start && end <= s.end);
      final tokenColor = (palette != null && tokenKind != null)
          ? palette!.color(tokenKind)
          : colors.foreground;

      final spanStyle = baseStyle.copyWith(
        color: tokenColor,
        backgroundColor: inSegment ? colors.highlight : null,
        fontWeight: inSegment ? FontWeight.w700 : null,
      );

      spans.add(TextSpan(text: text.substring(start, end), style: spanStyle));
    }

    return Align(
      alignment: Alignment.centerLeft,
      child: Text.rich(
        TextSpan(children: spans, style: baseStyle),
        maxLines: wrapLines ? null : 1,
        softWrap: wrapLines,
      ),
    );
  }
}

class _HunkHeaderBar extends StatelessWidget {
  const _HunkHeaderBar({required this.header});

  final String header;

  @override
  Widget build(BuildContext context) {
    final colors = _diffColors(context, InlineDiffKind.context);
    final scheme = Theme.of(context).colorScheme;
    return Container(
      width: double.infinity,
      height: _kDiffLineHeight,
      color: colors.lineNumberBackground,
      padding: const EdgeInsets.only(left: 53),
      alignment: Alignment.centerLeft,
      child: Text(
        header,
        maxLines: 1,
        overflow: TextOverflow.clip,
        style: TextStyle(
          color: scheme.outline.withValues(alpha: 0.9),
          fontFamily: 'Consolas',
          fontFamilyFallback: const <String>['Cascadia Mono', 'monospace'],
          fontSize: 11,
          height: 1.0,
        ),
      ),
    );
  }
}

class _LineNumber extends StatelessWidget {
  const _LineNumber({
    required this.value,
    this.color,
    this.background,
    this.embedded = false,
  });

  final int? value;
  final Color? color;
  final Color? background;
  final bool embedded;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: embedded ? 36 : 42,
      color: background,
      child: Padding(
        padding: EdgeInsets.only(right: embedded ? 12 : 8),
        child: Align(
          alignment: Alignment.centerRight,
          child: Text(
            value?.toString() ?? '',
            textAlign: TextAlign.right,
            style: TextStyle(
              color: color ??
                  Theme.of(context).colorScheme.outline.withValues(alpha: 0.75),
              fontFamily: 'Consolas',
              fontFamilyFallback: const <String>['Cascadia Mono', 'monospace'],
              fontSize: 11,
              height: 1.0,
            ),
          ),
        ),
      ),
    );
  }
}

_DiffColorSet _diffColors(BuildContext context, InlineDiffKind kind) {
  final scheme = Theme.of(context).colorScheme;
  final dark = scheme.brightness == Brightness.dark;
  final surface = scheme.surface;
  final add = dark ? const Color(0xFF3FB950) : const Color(0xFF1A7F37);
  final remove = dark ? const Color(0xFFF85149) : const Color(0xFFCF222E);
  // OpenCode-style tints: the whole line gets a soft color wash while the
  // line-number gutter carries a slightly stronger tint of the same hue.
  final lineWash = dark ? 0.20 : 0.12;
  final gutterWash = dark ? 0.10 : 0.07;
  final contextGutter = Color.alphaBlend(
    scheme.onSurface.withValues(alpha: dark ? 0.055 : 0.045),
    surface,
  );
  return switch (kind) {
    InlineDiffKind.add => (
      background: Color.alphaBlend(
        add.withValues(alpha: lineWash),
        surface,
      ),
      accent: add,
      foreground: scheme.onSurface,
      highlight: add.withValues(alpha: dark ? 0.40 : 0.32),
      lineNumber: add.withValues(alpha: dark ? 0.85 : 0.90),
      lineNumberBackground: Color.alphaBlend(
        add.withValues(alpha: gutterWash),
        surface,
      ),
    ),
    InlineDiffKind.remove => (
      background: Color.alphaBlend(
        remove.withValues(alpha: lineWash),
        surface,
      ),
      accent: remove,
      foreground: scheme.onSurface,
      highlight: remove.withValues(alpha: dark ? 0.40 : 0.32),
      lineNumber: remove.withValues(alpha: dark ? 0.85 : 0.90),
      lineNumberBackground: Color.alphaBlend(
        remove.withValues(alpha: gutterWash),
        surface,
      ),
    ),
    InlineDiffKind.context => (
      background: Colors.transparent,
      accent: Colors.transparent,
      foreground: scheme.onSurface.withValues(alpha: 0.82),
      highlight: Colors.transparent,
      lineNumber: scheme.outline.withValues(alpha: 0.65),
      lineNumberBackground: contextGutter,
    ),
  };
}

class WorkspaceDiffEmptyState extends StatelessWidget {
  const WorkspaceDiffEmptyState({
    super.key,
    required this.message,
    this.title = '尚无文件变更',
    this.subtitle,
    this.compact = false,
  });

  final String message;
  final String title;
  final String? subtitle;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    if (compact) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 6, horizontal: 8),
        child: Text(
          message,
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
          ),
        ),
      );
    }
    final isDark = theme.brightness == Brightness.dark;
    final sub = subtitle ?? message;

    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Container(
            width: 32,
            height: 32,
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(6),
              border: Border.all(
                color: theme.colorScheme.outline.withValues(
                  alpha: isDark ? 0.35 : 0.50,
                ),
                width: 1.2,
              ),
            ),
            child: Icon(
              Icons.add,
              size: 16,
              color: theme.colorScheme.outline.withValues(
                alpha: isDark ? 0.65 : 0.80,
              ),
            ),
          ),
          const SizedBox(height: 12),
          Text(
            title,
            style: TextStyle(
              fontSize: 13,
              fontWeight: FontWeight.w600,
              color: theme.colorScheme.onSurface.withValues(
                alpha: isDark ? 0.88 : 0.92,
              ),
            ),
          ),
          const SizedBox(height: 4),
          Text(
            sub,
            style: TextStyle(
              fontSize: 11.5,
              color: theme.colorScheme.outline.withValues(
                alpha: isDark ? 0.65 : 0.80,
              ),
            ),
          ),
        ],
      ),
    );
  }
}