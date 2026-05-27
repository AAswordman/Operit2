// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../core/chat/OperitChatRuntime.dart';
import '../../../util/ChatMarkupRegex.dart';
import 'EnhancedCodeBlock.dart';
import 'EnhancedTableBlock.dart';
import 'MarkdownBlockQuote.dart';
import 'MarkdownImageRenderer.dart';
import 'MarkdownInlineSpannable.dart';
import 'MarkdownLatexBlock.dart';
import '../../features/chat/components/part/CustomXmlRenderer.dart';
import '../../features/chat/components/part/ThinkToolsXmlNodeGrouper.dart';

class StreamMarkdownRenderer extends StatelessWidget {
  const StreamMarkdownRenderer({
    super.key,
    required this.content,
    required this.isStreaming,
    this.streamState,
    required this.textColor,
    required this.backgroundColor,
  });

  final String content;
  final bool isStreaming;
  final ChatMarkdownStreamState? streamState;
  final Color textColor;
  final Color backgroundColor;

  @override
  Widget build(BuildContext context) {
    final state = streamState;
    if (state != null && state.blocks.isNotEmpty) {
      return _GroupedMarkdown(
        streamState: state,
        isStreaming: isStreaming,
        textColor: textColor,
        backgroundColor: backgroundColor,
      );
    }

    final nodes = _parseMarkdownNodes(content, isStreaming: isStreaming);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        for (final node in nodes)
          if (node.xmlContent != null)
            CustomXmlRenderer(
              xmlContent: node.xmlContent!,
              isStreaming: node.isStreaming,
              textColor: textColor,
            )
          else
            _MarkdownText(
              text: node.text,
              textColor: textColor,
              backgroundColor: backgroundColor,
              isStreaming: node.isStreaming,
            ),
      ],
    );
  }
}

const double _markdownParagraphBreakHeight = 4;
const double _markdownLineBlockBottomPadding = 3;
const double _markdownCanvasLineHeightMultiplier = 1.3;

class _GroupedMarkdown extends StatelessWidget {
  const _GroupedMarkdown({
    required this.streamState,
    required this.isStreaming,
    required this.textColor,
    required this.backgroundColor,
  });

  final ChatMarkdownStreamState streamState;
  final bool isStreaming;
  final Color textColor;
  final Color backgroundColor;

  @override
  Widget build(BuildContext context) {
    final groupedItems = groupThinkToolsXmlNodes(streamState.blocks);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        for (final item in groupedItems)
          if (item is MarkdownSingleItem)
            _GroupedMarkdownBlock(
              block: streamState.blocks[item.index],
              isStreaming: isStreaming,
              textColor: textColor,
              backgroundColor: backgroundColor,
            )
          else if (item is MarkdownGroupItem)
            _GroupedMarkdownGroup(
              item: item,
              blocks: streamState.blocks,
              isStreaming: isStreaming,
              textColor: textColor,
              backgroundColor: backgroundColor,
            ),
        if (isStreaming)
          const Padding(
            padding: EdgeInsets.only(top: 2),
            child: StreamingCursor(),
          ),
      ],
    );
  }
}

class _GroupedMarkdownGroup extends StatefulWidget {
  const _GroupedMarkdownGroup({
    required this.item,
    required this.blocks,
    required this.isStreaming,
    required this.textColor,
    required this.backgroundColor,
  });

  final MarkdownGroupItem item;
  final List<ChatMarkdownBlockNode> blocks;
  final bool isStreaming;
  final Color textColor;
  final Color backgroundColor;

  @override
  State<_GroupedMarkdownGroup> createState() => _GroupedMarkdownGroupState();
}

class _GroupedMarkdownGroupState extends State<_GroupedMarkdownGroup> {
  bool expanded = true;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final end = widget.item.endIndexInclusive < widget.blocks.length
        ? widget.item.endIndexInclusive
        : widget.blocks.length - 1;
    final slice = widget.blocks.sublist(widget.item.startIndex, end + 1);
    final toolCount = slice.where((block) {
      return block.nodeType == 'XmlBlock' &&
          extractXmlTagName(block.content.toString()) == 'tool';
    }).length;
    final title = widget.item.stableKey.startsWith('tools-only-')
        ? 'Tools ($toolCount)'
        : 'Thinking & tools ($toolCount)';

    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          InkWell(
            onTap: () {
              setState(() {
                expanded = !expanded;
              });
            },
            borderRadius: BorderRadius.circular(6),
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: Row(
                children: <Widget>[
                  AnimatedRotation(
                    turns: expanded ? 0.25 : 0,
                    duration: const Duration(milliseconds: 300),
                    child: Icon(
                      Icons.keyboard_arrow_right,
                      size: 18,
                      color: widget.textColor.withValues(alpha: 0.7),
                    ),
                  ),
                  const SizedBox(width: 6),
                  Text(
                    title,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: widget.textColor.withValues(alpha: 0.7),
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ],
              ),
            ),
          ),
          AnimatedCrossFade(
            firstChild: const SizedBox.shrink(),
            secondChild: Padding(
              padding: const EdgeInsets.only(left: 24, top: 4, bottom: 8),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  for (final block in slice)
                    _GroupedMarkdownBlock(
                      block: block,
                      isStreaming: widget.isStreaming,
                      textColor: widget.textColor,
                      backgroundColor: widget.backgroundColor,
                    ),
                ],
              ),
            ),
            crossFadeState: expanded
                ? CrossFadeState.showSecond
                : CrossFadeState.showFirst,
            duration: const Duration(milliseconds: 200),
          ),
        ],
      ),
    );
  }
}

class _GroupedMarkdownBlock extends StatelessWidget {
  const _GroupedMarkdownBlock({
    required this.block,
    required this.isStreaming,
    required this.textColor,
    required this.backgroundColor,
  });

  final ChatMarkdownBlockNode block;
  final bool isStreaming;
  final Color textColor;
  final Color backgroundColor;

  @override
  Widget build(BuildContext context) {
    final type = block.nodeType;
    final text = block.content.toString();
    if (type == 'XmlBlock') {
      return CustomXmlRenderer(
        xmlContent: text,
        isStreaming: isStreaming,
        textColor: textColor,
      );
    }
    if (type == 'HtmlBreak') {
      return const SizedBox(height: _markdownParagraphBreakHeight);
    }
    if (type == 'HorizontalRule') {
      return const MarkdownHorizontalRule();
    }
    if (type == 'BlockQuote') {
      return MarkdownBlockQuote(
        content: text,
        textColor: textColor,
        backgroundColor: backgroundColor,
        isStreaming: isStreaming,
      );
    }
    if (type == 'BlockLatex') {
      return MarkdownLatexBlock(content: text, textColor: textColor);
    }
    if (type == 'Image') {
      return MarkdownImageRenderer(imageMarkdown: text, textColor: textColor);
    }
    if (type == 'Table') {
      return EnhancedTableBlock(tableText: text, textColor: textColor);
    }
    if (type == 'CodeBlock') {
      final parsed = _parseCodeBlock(text);
      return EnhancedCodeBlock(code: parsed.code, language: parsed.language);
    }
    if (type == 'Header') {
      return _MarkdownHeading(
        text: text,
        color: textColor,
        level: block.headerLevel,
      );
    }
    if (block.children.isNotEmpty) {
      return Padding(
        padding: const EdgeInsets.only(bottom: _markdownLineBlockBottomPadding),
        child: Text.rich(
          buildMarkdownInlineSpannableFromChildren(
            context: context,
            textColor: textColor,
            children: <MarkdownInlineSegment>[
              for (final child in block.children)
                MarkdownInlineSegment(
                  text: child.content.toString(),
                  nodeType: child.nodeType,
                ),
            ],
          ),
          style: Theme.of(
            context,
          ).textTheme.bodyMedium?.copyWith(color: textColor, height: 1.3),
        ),
      );
    }
    return _MarkdownText(
      text: text,
      textColor: textColor,
      backgroundColor: backgroundColor,
      isStreaming: false,
    );
  }
}

class _MarkdownNode {
  const _MarkdownNode.text(this.text, {required this.isStreaming})
    : xmlContent = null;

  const _MarkdownNode.xml(this.xmlContent, {required this.isStreaming})
    : text = '';

  final String text;
  final String? xmlContent;
  final bool isStreaming;
}

List<_MarkdownNode> _parseMarkdownNodes(
  String content, {
  required bool isStreaming,
}) {
  final nodes = <_MarkdownNode>[];
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
      _MarkdownNode.xml(xmlContent, isStreaming: isStreaming && closeIndex < 0),
    );
    cursor = xmlEnd;
  }

  if (nodes.isNotEmpty && isStreaming) {
    final last = nodes.last;
    nodes[nodes.length - 1] = last.xmlContent == null
        ? _MarkdownNode.text(last.text, isStreaming: true)
        : _MarkdownNode.xml(last.xmlContent, isStreaming: true);
  }
  return nodes;
}

void _addTextNode(List<_MarkdownNode> nodes, String text) {
  final cleaned = text.trim();
  if (cleaned.isNotEmpty) {
    nodes.add(_MarkdownNode.text(cleaned, isStreaming: false));
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
  const _MarkdownHeading({required this.text, required this.color, this.level});

  final String text;
  final Color color;
  final int? level;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final effectiveLevel = level ?? _determineHeaderLevel(text);
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

_ParsedCodeBlock _parseCodeBlock(String text) {
  final lines = text.trim().split('\n');
  if (lines.isNotEmpty && lines.first.trimLeft().startsWith('```')) {
    final language = lines.first.trimLeft().substring(3).trim();
    final body = lines
        .skip(1)
        .takeWhile((line) => !line.trimRight().endsWith('```'))
        .join('\n');
    return _ParsedCodeBlock(code: body, language: language);
  }
  return _ParsedCodeBlock(code: text, language: '');
}

class _ParsedCodeBlock {
  const _ParsedCodeBlock({required this.code, required this.language});

  final String code;
  final String language;
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
