// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../common/components/OperitDialog.dart';
import 'ParamVisualizer.dart';
import 'ToolDisplayComponents.dart';

class ContentDetailDialog extends StatefulWidget {
  const ContentDetailDialog({
    super.key,
    required this.title,
    required this.content,
    required this.icon,
    required this.onDismiss,
    this.isDiffContent = false,
  });

  final String title;
  final String content;
  final IconData icon;
  final VoidCallback onDismiss;
  final bool isDiffContent;

  @override
  State<ContentDetailDialog> createState() => _ContentDetailDialogState();
}

class _ContentDetailDialogState extends State<ContentDetailDialog> {
  bool _splitView = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isXmlContent = widget.content.trim().startsWith('<');
    return OperitDialogScaffold(
      title: widget.title,
      icon: Icon(widget.icon, size: 20),
      maxWidth: _splitView ? 960 : 720,
      actions: <Widget>[
        if (widget.isDiffContent)
          TextButton.icon(
            onPressed: () => setState(() => _splitView = !_splitView),
            icon: Icon(
              _splitView ? Icons.view_agenda : Icons.view_column,
              size: 16,
            ),
            label: Text(_splitView ? 'Unified' : 'Split'),
          ),
        FilledButton(onPressed: widget.onDismiss, child: const Text('Close')),
      ],
      child: Container(
        width: double.infinity,
        constraints: const BoxConstraints(minHeight: 50, maxHeight: 480),
        padding: const EdgeInsets.all(8),
        decoration: BoxDecoration(
          color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
          borderRadius: BorderRadius.circular(8),
        ),
        child: SingleChildScrollView(
          child: widget.isDiffContent
              ? _splitView
                    ? SplitDiffContent(lines: widget.content.lines())
                    : DiffContentWidget(lines: widget.content.lines())
              : isXmlContent
              ? ParamVisualizer(xmlContent: widget.content)
              : CodeContentWithLineNumbers(
                  lines: widget.content.lines(),
                  textColor: theme.colorScheme.onSurface,
                ),
        ),
      ),
    );
  }
}

class DiffContentWidget extends StatelessWidget {
  const DiffContentWidget({super.key, required this.lines});

  final List<String> lines;

  @override
  Widget build(BuildContext context) {
    final rows = buildDiffRows(lines);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        for (final row in rows)
          if (row.kind == DiffRowKind.hunk)
            Container(
              width: double.infinity,
              color: row.background(context),
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 3),
              child: Text(
                row.text,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  fontFamily: 'monospace',
                  fontSize: 11,
                  fontWeight: FontWeight.w600,
                  color: row.foreground(context),
                ),
              ),
            )
          else
            Container(
              width: double.infinity,
              color: row.background(context),
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 1),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  SizedBox(
                    width: 30,
                    child: Text(
                      row.oldLine != null ? '${row.oldLine}' : '',
                      textAlign: TextAlign.right,
                      style: _lineNumberStyle(context),
                    ),
                  ),
                  const SizedBox(width: 6),
                  SizedBox(
                    width: 30,
                    child: Text(
                      row.newLine != null ? '${row.newLine}' : '',
                      textAlign: TextAlign.right,
                      style: _lineNumberStyle(context),
                    ),
                  ),
                  const SizedBox(width: 6),
                  SizedBox(
                    width: 12,
                    child: Text(
                      switch (row.kind) {
                        DiffRowKind.add => '+',
                        DiffRowKind.remove => '-',
                        _ => ' ',
                      },
                      textAlign: TextAlign.center,
                      style: TextStyle(
                        fontFamily: 'monospace',
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                        color: row.foreground(context),
                      ),
                    ),
                  ),
                  const SizedBox(width: 6),
                  Expanded(
                    child: Text(
                      row.text,
                      softWrap: true,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        fontFamily: 'monospace',
                        color: row.foreground(context),
                      ),
                    ),
                  ),
                ],
              ),
            ),
      ],
    );
  }

  TextStyle? _lineNumberStyle(BuildContext context) {
    return Theme.of(context).textTheme.bodySmall?.copyWith(
      fontFamily: 'monospace',
      fontSize: 11,
      color: Theme.of(context).colorScheme.outline,
    );
  }
}

class SplitDiffContent extends StatelessWidget {
  const SplitDiffContent({super.key, required this.lines});

  final List<String> lines;

  @override
  Widget build(BuildContext context) {
    final rows = buildDiffRows(lines);
    final theme = Theme.of(context);
    return Column(
      children: <Widget>[
        for (final row in rows)
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Expanded(
                child: Container(
                  color: row.kind == DiffRowKind.remove ? row.background(context) : null,
                  padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                  child: Text(
                    row.kind == DiffRowKind.add ? '' : row.text,
                    style: theme.textTheme.bodySmall?.copyWith(
                      fontFamily: 'monospace',
                      color: row.kind == DiffRowKind.remove ? row.foreground(context) : null,
                    ),
                  ),
                ),
              ),
              Expanded(
                child: Container(
                  color: row.kind == DiffRowKind.add ? row.background(context) : null,
                  padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                  child: Text(
                    row.kind == DiffRowKind.remove ? '' : row.text,
                    style: theme.textTheme.bodySmall?.copyWith(
                      fontFamily: 'monospace',
                      color: row.kind == DiffRowKind.add ? row.foreground(context) : null,
                    ),
                  ),
                ),
              ),
            ],
          ),
      ],
    );
  }
}

enum DiffRowKind { context, add, remove, hunk }

class DiffRow {
  const DiffRow({
    required this.kind,
    required this.text,
    this.oldLine,
    this.newLine,
  });

  final String text;
  final int? oldLine;
  final int? newLine;
  final DiffRowKind kind;

  Color background(BuildContext context) {
    final colors = DiffColors.of(context);
    return switch (kind) {
      DiffRowKind.add => colors.addBackground,
      DiffRowKind.remove => colors.removeBackground,
      DiffRowKind.hunk => colors.hunkBackground,
      DiffRowKind.context => Colors.transparent,
    };
  }

  Color foreground(BuildContext context) {
    final colors = DiffColors.of(context);
    return switch (kind) {
      DiffRowKind.add => colors.addForeground,
      DiffRowKind.remove => colors.removeForeground,
      DiffRowKind.hunk => colors.hunkForeground,
      DiffRowKind.context => colors.contextForeground,
    };
  }
}

List<DiffRow> buildDiffRows(List<String> lines) {
  var oldLine = 1;
  var newLine = 1;
  final rows = <DiffRow>[];
  for (final line in lines) {
    if (line.startsWith('--- ') || line.startsWith('+++ ')) {
      continue;
    }
    if (line.startsWith('@@')) {
      final parsed = parseHunk(line);
      oldLine = parsed?.$1 ?? oldLine;
      newLine = parsed?.$2 ?? newLine;
      rows.add(
        DiffRow(
          kind: DiffRowKind.hunk,
          text: line,
        ),
      );
    } else if (line.startsWith('+')) {
      rows.add(
        DiffRow(
          kind: DiffRowKind.add,
          text: line.substring(1),
          newLine: newLine++,
        ),
      );
    } else if (line.startsWith('-')) {
      rows.add(
        DiffRow(
          kind: DiffRowKind.remove,
          text: line.substring(1),
          oldLine: oldLine++,
        ),
      );
    } else {
      rows.add(
        DiffRow(
          kind: DiffRowKind.context,
          text: line.startsWith(' ') ? line.substring(1) : line,
          oldLine: oldLine++,
          newLine: newLine++,
        ),
      );
    }
  }
  return rows;
}

(int, int)? parseHunk(String line) {
  final match = RegExp(r'@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@').firstMatch(line);
  if (match == null) {
    return null;
  }
  return (
    int.tryParse(match.group(1) ?? '1') ?? 1,
    int.tryParse(match.group(2) ?? '1') ?? 1,
  );
}

class DiffColors {
  const DiffColors({
    required this.addBackground,
    required this.addForeground,
    required this.removeBackground,
    required this.removeForeground,
    required this.hunkBackground,
    required this.hunkForeground,
    required this.contextForeground,
  });

  static DiffColors of(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final scheme = Theme.of(context).colorScheme;

    if (isDark) {
      return DiffColors(
        addBackground: const Color(0xFF22C55E).withValues(alpha: 0.18),
        addForeground: const Color(0xFF4ADE80),
        removeBackground: const Color(0xFFEF4444).withValues(alpha: 0.18),
        removeForeground: const Color(0xFFF87171),
        hunkBackground: const Color(0xFF0284C7).withValues(alpha: 0.15),
        hunkForeground: const Color(0xFF38BDF8),
        contextForeground: scheme.onSurfaceVariant.withValues(alpha: 0.9),
      );
    } else {
      return DiffColors(
        addBackground: const Color(0xFFDCFCE7),
        addForeground: const Color(0xFF15803D),
        removeBackground: const Color(0xFFFEE2E2),
        removeForeground: const Color(0xFFB91C1C),
        hunkBackground: const Color(0xFFE0F2FE),
        hunkForeground: const Color(0xFF0369A1),
        contextForeground: scheme.onSurfaceVariant,
      );
    }
  }

  final Color addBackground;
  final Color addForeground;
  final Color removeBackground;
  final Color removeForeground;
  final Color hunkBackground;
  final Color hunkForeground;
  final Color contextForeground;
}

extension _DialogStringLines on String {
  List<String> lines() => split('\n');
}
