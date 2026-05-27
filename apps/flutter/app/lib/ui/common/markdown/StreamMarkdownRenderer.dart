// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../core/chat/OperitChatRuntime.dart';
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
      return const SizedBox(height: 8);
    }
    if (type == 'HorizontalRule') {
      return Divider(color: textColor.withValues(alpha: 0.18));
    }
    if (type == 'CodeBlock' || type == 'Table') {
      return _CodeBlock(text: text, textColor: textColor);
    }
    if (block.children.isNotEmpty) {
      return Padding(
        padding: const EdgeInsets.only(bottom: 3),
        child: SelectableText.rich(
          TextSpan(
            children: [
              for (final child in block.children)
                TextSpan(
                  text: child.content.toString(),
                  style: _inlineStyle(context, child.nodeType, textColor),
                ),
            ],
          ),
          style: Theme.of(
            context,
          ).textTheme.bodyMedium?.copyWith(color: textColor, height: 1.45),
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

class _CodeBlock extends StatelessWidget {
  const _CodeBlock({required this.text, required this.textColor});

  final String text;
  final Color textColor;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(bottom: 8),
      padding: const EdgeInsets.fromLTRB(12, 10, 12, 10),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(8),
      ),
      child: SelectableText(
        text,
        style: theme.textTheme.bodySmall?.copyWith(
          color: textColor.withValues(alpha: 0.86),
          fontFamily: 'monospace',
          height: 1.35,
        ),
      ),
    );
  }
}

TextStyle? _inlineStyle(
  BuildContext context,
  String? nodeType,
  Color textColor,
) {
  final base = Theme.of(
    context,
  ).textTheme.bodyMedium?.copyWith(color: textColor, height: 1.45);
  switch (nodeType) {
    case 'Bold':
      return base?.copyWith(fontWeight: FontWeight.w700);
    case 'Italic':
      return base?.copyWith(fontStyle: FontStyle.italic);
    case 'Strikethrough':
      return base?.copyWith(decoration: TextDecoration.lineThrough);
    case 'Underline':
      return base?.copyWith(decoration: TextDecoration.underline);
    case 'InlineCode':
      return base?.copyWith(
        fontFamily: 'monospace',
        backgroundColor: Theme.of(
          context,
        ).colorScheme.surfaceContainerHighest.withValues(alpha: 0.55),
      );
  }
  return base;
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
  final openTagPattern = RegExp(
    r'<(think|thinking|search|status|tool|tool_result|html|mood|font|details|detail|meta)\b[^>]*>',
    caseSensitive: false,
  );
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

    final tag = match.group(1)!.toLowerCase();
    final closeTag = '</$tag>';
    final closeIndex = content.toLowerCase().indexOf(closeTag, end);
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
    final theme = Theme.of(context);
    final widgets = <Widget>[];
    final codeLines = <String>[];
    var inCode = false;

    void flushCode() {
      if (codeLines.isEmpty) {
        return;
      }
      widgets.add(
        Container(
          width: double.infinity,
          margin: const EdgeInsets.only(bottom: 8),
          padding: const EdgeInsets.fromLTRB(12, 10, 12, 10),
          decoration: BoxDecoration(
            color: theme.colorScheme.surfaceContainerHighest.withValues(
              alpha: 0.5,
            ),
            borderRadius: BorderRadius.circular(8),
          ),
          child: SelectableText(
            codeLines.join('\n'),
            style: theme.textTheme.bodySmall?.copyWith(
              color: textColor.withValues(alpha: 0.86),
              fontFamily: 'monospace',
              height: 1.35,
            ),
          ),
        ),
      );
      codeLines.clear();
    }

    for (final line in text.split('\n')) {
      final trimmed = line.trimRight();
      if (trimmed.startsWith('```')) {
        if (inCode) {
          flushCode();
        }
        inCode = !inCode;
      } else if (inCode) {
        codeLines.add(line);
      } else {
        widgets.add(_MarkdownLine(text: trimmed, color: textColor));
      }
    }
    flushCode();

    if (isStreaming) {
      widgets.add(
        const Padding(
          padding: EdgeInsets.only(top: 2),
          child: StreamingCursor(),
        ),
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: widgets,
    );
  }
}

class _MarkdownLine extends StatelessWidget {
  const _MarkdownLine({required this.text, required this.color});

  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) {
    if (text.isEmpty) {
      return const SizedBox(height: 8);
    }
    final theme = Theme.of(context);
    final headingLevel = _headingLevel(text);
    if (headingLevel > 0) {
      return Padding(
        padding: const EdgeInsets.only(top: 4, bottom: 6),
        child: SelectableText(
          text.substring(headingLevel + 1).trim(),
          style: theme.textTheme.titleSmall?.copyWith(
            color: color,
            fontWeight: FontWeight.w700,
          ),
        ),
      );
    }
    if (_isBulletLine(text)) {
      return Padding(
        padding: const EdgeInsets.only(bottom: 3),
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
              child: SelectableText(
                text.substring(2),
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: color,
                  height: 1.45,
                ),
              ),
            ),
          ],
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.only(bottom: 3),
      child: SelectableText(
        text,
        style: theme.textTheme.bodyMedium?.copyWith(color: color, height: 1.45),
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
  final match = RegExp(r'^(#{1,6})\s+').firstMatch(text);
  return match?.group(1)?.length ?? 0;
}

bool _isBulletLine(String text) {
  return text.startsWith('- ') || text.startsWith('* ');
}
