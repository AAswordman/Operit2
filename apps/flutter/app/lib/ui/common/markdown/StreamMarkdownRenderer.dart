// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../util/ChatMarkupRegex.dart';
import 'EnhancedCodeBlock.dart';
import 'EnhancedTableBlock.dart';
import 'MarkdownNodeGrouper.dart';
import 'MarkdownBlockQuote.dart';
import 'MarkdownImageRenderer.dart';
import 'MarkdownInlineSpannable.dart';
import 'MarkdownLatexBlock.dart';
import '../../features/chat/components/part/CustomXmlRenderer.dart';

class StreamMarkdownRenderer extends StatelessWidget {
  const StreamMarkdownRenderer({
    super.key,
    required this.content,
    required this.isStreaming,
    required this.textColor,
    required this.backgroundColor,
    this.nodeGrouper = const NoopMarkdownNodeGrouper(),
  });

  final String content;
  final bool isStreaming;
  final Color textColor;
  final Color backgroundColor;
  final MarkdownNodeGrouper nodeGrouper;

  @override
  Widget build(BuildContext context) {
    final nodes = parseMarkdownNodes(content, isStreaming: isStreaming);
    final rendererId = 'flutter-markdown-${identityHashCode(this)}';
    final groupedItems = nodeGrouper.group(nodes, rendererId);

    Widget renderNodeAt(int index) {
      final node = nodes[index];
      if (node.type == MarkdownNodeType.xmlBlock) {
        return CustomXmlRenderer(
          xmlContent: node.content,
          isStreaming: node.isStreaming,
          textColor: textColor,
        );
      }
      return _MarkdownText(
        text: node.content,
        textColor: textColor,
        backgroundColor: backgroundColor,
        isStreaming: node.isStreaming,
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        for (final item in groupedItems)
          if (item is MarkdownSingleItem)
            renderNodeAt(item.index)
          else if (item is MarkdownGroupItem)
            nodeGrouper.renderGroup(
              group: item,
              nodes: nodes,
              rendererId: rendererId,
              isVisible: true,
              isLastNode: item.endIndexInclusive == nodes.length - 1,
              textColor: textColor,
              renderNodeAt: renderNodeAt,
            ),
      ],
    );
  }
}

const double _markdownParagraphBreakHeight = 4;
const double _markdownLineBlockBottomPadding = 3;
const double _markdownCanvasLineHeightMultiplier = 1.3;

List<MarkdownNodeStable> parseMarkdownNodes(
  String content, {
  required bool isStreaming,
}) {
  final nodes = <MarkdownNodeStable>[];
  final openTagPattern = ChatMarkupRegex.xmlBlockStartTag;
  var cursor = 0;

  while (cursor < content.length) {
    final sliced = content.substring(cursor);
    final match = openTagPattern.firstMatch(sliced);
    if (match == null) {
      _addTextNode(nodes, sliced);
      break;
    }

    final start = cursor + match.start;
    final end = cursor + match.end;
    if (start > cursor) {
      _addTextNode(nodes, content.substring(cursor, start));
    }

    final rawTag = match.group(1)!;
    final closeTag = '</$rawTag>';
    final closeIndex = content.toLowerCase().indexOf(
      closeTag.toLowerCase(),
      end,
    );
    final xmlEnd = closeIndex >= 0
        ? closeIndex + closeTag.length
        : content.length;
    final xmlContent = content.substring(start, xmlEnd);
    nodes.add(
      MarkdownNodeStable(
        type: MarkdownNodeType.xmlBlock,
        content: xmlContent,
        isStreaming: isStreaming && closeIndex < 0,
      ),
    );
    cursor = xmlEnd;
  }

  if (nodes.isNotEmpty && isStreaming) {
    final last = nodes.last;
    nodes[nodes.length - 1] = MarkdownNodeStable(
      type: last.type,
      content: last.content,
      isStreaming: true,
    );
  }
  return nodes;
}

void _addTextNode(List<MarkdownNodeStable> nodes, String text) {
  final cleaned = text.trim();
  if (cleaned.isNotEmpty) {
    nodes.add(
      MarkdownNodeStable(
        type: MarkdownNodeType.plainText,
        content: cleaned,
        isStreaming: false,
      ),
    );
  }
}

class _MarkdownText extends StatelessWidget {
  const _MarkdownText({
    required this.text,
    required this.textColor,
    required this.backgroundColor,
    required this.isStreaming,
  });

  final String text;
  final Color textColor;
  final Color backgroundColor;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    final widgets = <Widget>[];
    final codeLines = <String>[];
    final paragraphLines = <String>[];
    var inCode = false;
    var codeLanguage = '';
    final lines = text.split('\n');
    var index = 0;

    void flushCode() {
      if (codeLines.isEmpty) {
        return;
      }
      widgets.add(
        EnhancedCodeBlock(code: codeLines.join('\n'), language: codeLanguage),
      );
      codeLines.clear();
    }

    void flushParagraph() {
      if (paragraphLines.isEmpty) {
        return;
      }
      widgets.add(
        _MarkdownParagraph(text: paragraphLines.join('\n'), color: textColor),
      );
      paragraphLines.clear();
    }

    while (index < lines.length) {
      final line = lines[index];
      final trimmed = line.trimRight();
      if (trimmed.startsWith('```')) {
        if (inCode) {
          flushCode();
          codeLanguage = '';
        } else {
          flushParagraph();
          codeLanguage = trimmed.substring(3).trim();
        }
        inCode = !inCode;
      } else if (inCode) {
        codeLines.add(line);
      } else if (_isBlockLatexStart(trimmed)) {
        flushParagraph();
        final latexLines = <String>[trimmed];
        final start = trimmed.trimLeft();
        final singleLine = start.length > 2 && _isBlockLatexEnd(start, start);
        while (!singleLine && index + 1 < lines.length) {
          index++;
          final nextLine = lines[index].trimRight();
          latexLines.add(nextLine);
          if (_isBlockLatexEnd(start, nextLine.trimRight())) {
            break;
          }
        }
        widgets.add(
          MarkdownLatexBlock(
            content: latexLines.join('\n'),
            textColor: textColor,
          ),
        );
      } else if (_isTableStart(lines, index)) {
        flushParagraph();
        final tableLines = <String>[];
        while (index < lines.length && lines[index].trim().contains('|')) {
          tableLines.add(lines[index]);
          index++;
        }
        index--;
        widgets.add(
          EnhancedTableBlock(
            tableText: tableLines.join('\n'),
            textColor: textColor,
          ),
        );
      } else if (trimmed.trimLeft().startsWith('>')) {
        flushParagraph();
        final quoteLines = <String>[];
        while (index < lines.length &&
            lines[index].trimLeft().startsWith('>')) {
          quoteLines.add(lines[index]);
          index++;
        }
        index--;
        widgets.add(
          MarkdownBlockQuote(
            content: quoteLines.join('\n'),
            textColor: textColor,
            backgroundColor: backgroundColor,
            isStreaming: isStreaming,
          ),
        );
      } else if (isCompleteImageMarkdown(trimmed.trim())) {
        flushParagraph();
        widgets.add(
          MarkdownImageRenderer(
            imageMarkdown: trimmed.trim(),
            textColor: textColor,
          ),
        );
      } else if (_isHorizontalRule(trimmed)) {
        flushParagraph();
        widgets.add(const MarkdownHorizontalRule());
      } else if (trimmed.isEmpty) {
        flushParagraph();
        if (widgets.isNotEmpty) {
          widgets.add(const SizedBox(height: _markdownParagraphBreakHeight));
        }
      } else if (_headingLevel(trimmed) > 0 ||
          _isBulletLine(trimmed) ||
          _isOrderedLine(trimmed)) {
        flushParagraph();
        widgets.add(_MarkdownLine(text: trimmed, color: textColor));
      } else {
        paragraphLines.add(trimmed);
      }
      index++;
    }
    flushCode();
    flushParagraph();

    if (isStreaming) {
      widgets.add(
        const Padding(
          padding: EdgeInsets.only(top: 2),
          child: StreamingCursor(),
        ),
      );
    }
    final content = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: widgets,
    );
    if (isStreaming) {
      return content;
    }
    return SelectionArea(child: content);
  }
}

class _MarkdownLine extends StatelessWidget {
  const _MarkdownLine({required this.text, required this.color});

  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) {
    if (text.isEmpty) {
      return const SizedBox(height: _markdownParagraphBreakHeight);
    }
    final theme = Theme.of(context);
    final headingLevel = _headingLevel(text);
    if (headingLevel > 0) {
      return _MarkdownHeading(text: text, color: color);
    }
    if (_isBulletLine(text)) {
      return Padding(
        padding: const EdgeInsets.only(bottom: _markdownLineBlockBottomPadding),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.only(top: 8, right: 8),
              child: Container(
                width: 4,
                height: 4,
                decoration: BoxDecoration(
                  color: color.withValues(alpha: 0.7),
                  shape: BoxShape.circle,
                ),
              ),
            ),
            Expanded(
              child: Text.rich(
                buildMarkdownInlineSpannableFromText(
                  context: context,
                  text: text.substring(2),
                  textColor: color,
                ),
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: color,
                  height: 1.3,
                ),
              ),
            ),
          ],
        ),
      );
    }
    if (_isOrderedLine(text)) {
      final match = RegExp(r'^(\d+)\.\s*').firstMatch(text);
      final marker = match?.group(1) ?? '';
      final body = match == null ? text : text.substring(match.end);
      return Padding(
        padding: const EdgeInsets.only(bottom: _markdownLineBlockBottomPadding),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.only(right: 4),
              child: Text(
                '$marker.',
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: color,
                  fontWeight: FontWeight.w700,
                  height: 1.3,
                ),
              ),
            ),
            Expanded(
              child: Text.rich(
                buildMarkdownInlineSpannableFromText(
                  context: context,
                  text: body,
                  textColor: color,
                ),
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: color,
                  height: 1.3,
                ),
              ),
            ),
          ],
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.only(bottom: _markdownLineBlockBottomPadding),
      child: Text.rich(
        buildMarkdownInlineSpannableFromText(
          context: context,
          text: text,
          textColor: color,
        ),
        style: theme.textTheme.bodyMedium?.copyWith(color: color, height: 1.3),
      ),
    );
  }
}

class _MarkdownHeading extends StatelessWidget {
  const _MarkdownHeading({required this.text, required this.color});

  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final effectiveLevel = _determineHeaderLevel(text);
    final headingText = _markdownHeaderText(text);
    final style = _markdownHeaderStyle(theme, effectiveLevel)?.copyWith(
      color: color,
      fontWeight: FontWeight.w700,
      height: _markdownCanvasLineHeightMultiplier,
    );
    final topPadding = _markdownHeaderTopPadding(effectiveLevel);
    final bottomPadding = _markdownHeaderBottomPadding(effectiveLevel);

    return Padding(
      padding: EdgeInsets.only(top: topPadding, bottom: bottomPadding),
      child: Text.rich(
        buildMarkdownInlineSpannableFromText(
          context: context,
          text: headingText,
          textColor: color,
          baseStyle: style,
        ),
        style: style,
      ),
    );
  }
}

TextStyle? _markdownHeaderStyle(ThemeData theme, int level) {
  return switch (level) {
    1 => theme.textTheme.headlineMedium,
    2 => theme.textTheme.headlineSmall,
    3 => theme.textTheme.titleLarge,
    4 => theme.textTheme.titleMedium,
    5 => theme.textTheme.titleSmall,
    _ => theme.textTheme.bodyMedium,
  };
}

double _markdownHeaderTopPadding(int level) {
  return switch (level) {
    1 => 12,
    2 => 10,
    3 => 8,
    _ => 6,
  };
}

double _markdownHeaderBottomPadding(int level) {
  return switch (level) {
    1 || 2 => 4,
    _ => 2,
  };
}

String _markdownHeaderText(String text) {
  return text.replaceFirst(RegExp(r'^\s*#+\s*'), '').trim();
}

class _MarkdownParagraph extends StatelessWidget {
  const _MarkdownParagraph({required this.text, required this.color});

  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: _markdownLineBlockBottomPadding),
      child: Text.rich(
        buildMarkdownInlineSpannableFromText(
          context: context,
          text: text,
          textColor: color,
        ),
        style: theme.textTheme.bodyMedium?.copyWith(color: color, height: 1.3),
      ),
    );
  }
}

class StreamingCursor extends StatefulWidget {
  const StreamingCursor({super.key});

  @override
  State<StreamingCursor> createState() => _StreamingCursorState();
}

class _StreamingCursorState extends State<StreamingCursor>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 900),
    )..repeat(reverse: true);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final color = Theme.of(context).colorScheme.primary;
    return FadeTransition(
      opacity: Tween<double>(begin: 0.25, end: 0.85).animate(_controller),
      child: Container(
        width: 7,
        height: 16,
        decoration: BoxDecoration(
          color: color,
          borderRadius: BorderRadius.circular(2),
        ),
      ),
    );
  }
}

int _headingLevel(String text) {
  return _determineHeaderLevel(text);
}

int _determineHeaderLevel(String text) {
  final match = RegExp(r'^\s*(#{1,6})').firstMatch(text);
  return match?.group(1)?.length ?? 0;
}

bool _isBulletLine(String text) {
  return text.startsWith('- ') || text.startsWith('* ');
}

bool _isOrderedLine(String text) {
  return RegExp(r'^\d+\.\s+').hasMatch(text);
}

bool _isHorizontalRule(String text) {
  return RegExp(r'^\s{0,3}([-*_])(?:\s*\1){2,}\s*$').hasMatch(text);
}

bool _isTableStart(List<String> lines, int index) {
  if (index + 1 >= lines.length) {
    return false;
  }
  final current = lines[index].trim();
  final next = lines[index + 1].trim();
  return current.contains('|') &&
      next.contains('|') &&
      _isMarkdownTableSeparator(next);
}

bool _isMarkdownTableSeparator(String line) {
  final cells = line
      .replaceFirst(RegExp(r'^\|'), '')
      .replaceFirst(RegExp(r'\|$'), '')
      .split('|')
      .map((cell) => cell.trim());
  return cells.isNotEmpty &&
      cells.every((cell) => RegExp(r'^:?-{3,}:?$').hasMatch(cell));
}

bool _isBlockLatexStart(String line) {
  final trimmed = line.trimLeft();
  return trimmed.startsWith(r'$$') || trimmed.startsWith(r'\[');
}

bool _isBlockLatexEnd(String startLine, String line) {
  final trimmed = line.trimRight();
  if (startLine.startsWith(r'$$')) {
    return trimmed.endsWith(r'$$') && trimmed.length > 2;
  }
  return trimmed.endsWith(r'\]');
}

class MarkdownHorizontalRule extends StatelessWidget {
  const MarkdownHorizontalRule({super.key});

  @override
  Widget build(BuildContext context) {
    return Divider(
      height: 5,
      thickness: 1,
      color: Theme.of(context).colorScheme.outline.withValues(alpha: 0.5),
    );
  }
}
