// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/markdown/MarkdownImageRenderer.dart';
import '../../../common/markdown/MarkdownLatexBlock.dart';
import '../../../common/markdown/MarkdownNodeGrouper.dart';
import '../../../common/markdown/StreamMarkdownRendererState.dart';

enum MessageCopyFormat { plainText, markdownSource }

typedef MarkdownCopySplitter =
    Future<List<core_proxy.MarkdownStreamEvent>> Function(String content);

class MessageCopyPreviewSheet extends StatefulWidget {
  /// Creates a preview sheet for one message representation.
  const MessageCopyPreviewSheet({
    super.key,
    required this.markdownText,
    this.splitMarkdownContent,
  });

  final String markdownText;
  final MarkdownCopySplitter? splitMarkdownContent;

  /// Creates the state for the message copy preview sheet.
  @override
  State<MessageCopyPreviewSheet> createState() =>
      _MessageCopyPreviewSheetState();
}

class _MessageCopyPreviewSheetState extends State<MessageCopyPreviewSheet> {
  late final Future<String> _plainTextFuture;
  String? _plainText;
  MessageCopyFormat _format = MessageCopyFormat.plainText;

  /// Starts Markdown conversion before the preview is first rendered.
  @override
  void initState() {
    super.initState();
    final splitter = widget.splitMarkdownContent;
    _plainTextFuture = splitter == null
        ? Future<String>.error(
            StateError('Markdown copy splitter is not configured'),
          )
        : markdownToPlainTextForCopy(
            widget.markdownText,
            splitMarkdownContent: splitter,
          );
    _plainTextFuture.then(
      (value) {
        if (!mounted) {
          return;
        }
        setState(() {
          _plainText = value;
        });
      },
      onError: (Object error, StackTrace stackTrace) {
        return;
      },
    );
  }

  /// Builds the format selector, scrollable preview, and copy command.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final mediaQuery = MediaQuery.of(context);
    final copyText = _format == MessageCopyFormat.markdownSource
        ? widget.markdownText
        : _plainText;
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(20, 16, 20, 20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Text('复制消息', style: theme.textTheme.titleMedium),
            const SizedBox(height: 12),
            SegmentedButton<MessageCopyFormat>(
              segments: const <ButtonSegment<MessageCopyFormat>>[
                ButtonSegment<MessageCopyFormat>(
                  value: MessageCopyFormat.plainText,
                  label: Text('纯文本'),
                ),
                ButtonSegment<MessageCopyFormat>(
                  value: MessageCopyFormat.markdownSource,
                  label: Text('Markdown 源码'),
                ),
              ],
              selected: <MessageCopyFormat>{_format},
              onSelectionChanged: (selection) {
                setState(() {
                  _format = selection.single;
                });
              },
            ),
            const SizedBox(height: 12),
            ConstrainedBox(
              constraints: BoxConstraints(
                maxHeight: mediaQuery.size.height * 0.58,
                minHeight: 96,
              ),
              child: copyText == null
                  ? FutureBuilder<String>(
                      future: _plainTextFuture,
                      builder: (context, snapshot) {
                        if (snapshot.connectionState ==
                            ConnectionState.waiting) {
                          return const Center(
                            child: CircularProgressIndicator(),
                          );
                        }
                        if (snapshot.hasError) {
                          return Center(
                            child: Text(
                              '纯文本转换失败：${snapshot.error}',
                              style: TextStyle(color: theme.colorScheme.error),
                            ),
                          );
                        }
                        return _SelectableCopyText(text: snapshot.data!);
                      },
                    )
                  : _SelectableCopyText(text: copyText),
            ),
            const SizedBox(height: 8),
            Align(
              alignment: Alignment.centerRight,
              child: FilledButton.icon(
                onPressed: copyText == null
                    ? null
                    : () => _copyText(context, copyText),
                icon: const Icon(Icons.content_copy, size: 18),
                label: Text(
                  _format == MessageCopyFormat.plainText
                      ? '复制纯文本'
                      : '复制 Markdown 源码',
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  /// Copies the selected representation and reports completion in the sheet.
  Future<void> _copyText(BuildContext context, String text) async {
    try {
      await Clipboard.setData(ClipboardData(text: text));
    } on PlatformException catch (error) {
      if (!context.mounted) {
        return;
      }
      ScaffoldMessenger.maybeOf(context)?.showSnackBar(
        SnackBar(content: Text('复制失败：${error.message ?? error.code}')),
      );
      return;
    }
    if (!context.mounted) {
      return;
    }
    ScaffoldMessenger.maybeOf(
      context,
    )?.showSnackBar(const SnackBar(content: Text('消息已复制到剪贴板')));
  }
}

class _SelectableCopyText extends StatelessWidget {
  /// Creates a selectable, scrollable copy preview.
  const _SelectableCopyText({required this.text});

  final String text;

  /// Builds a scrollable text area with native selection support.
  @override
  Widget build(BuildContext context) {
    return SelectionArea(
      child: SingleChildScrollView(
        padding: const EdgeInsets.only(bottom: 8),
        child: SizedBox(
          width: double.infinity,
          child: Text(text, style: Theme.of(context).textTheme.bodyMedium),
        ),
      ),
    );
  }
}

/// Converts Markdown source into text suitable for direct pasting.
Future<String> markdownToPlainTextForCopy(
  String markdown, {
  required MarkdownCopySplitter splitMarkdownContent,
}) async {
  final events = await splitMarkdownContent(markdown);
  final state = StreamMarkdownRendererState();
  var completed = false;
  for (final event in events) {
    completed = state.applyEvent(event) == 'completed' || completed;
  }
  if (!completed) {
    state.eventBuilder.complete();
  }
  return _normalizeCopyWhitespace(
    await _renderCopyBlocks(
      state.eventBuilder.toStableNodes(isStreaming: false),
      splitMarkdownContent,
    ),
  );
}

/// Joins top-level AST nodes using readable block spacing.
Future<String> _renderCopyBlocks(
  List<MarkdownNodeStable> nodes,
  MarkdownCopySplitter split,
) async {
  final contentNodes = nodes
      .where((node) => node.type != MarkdownNodeType.htmlBreak)
      .toList(growable: false);
  final buffer = StringBuffer();
  for (var index = 0; index < contentNodes.length; index++) {
    if (index > 0) {
      final previousType = contentNodes[index - 1].type;
      final currentType = contentNodes[index].type;
      final adjacentLists =
          _isCopyListType(previousType) && _isCopyListType(currentType);
      buffer.write(adjacentLists ? '\n' : '\n\n');
    }
    buffer.write(await _renderCopyBlock(contentNodes[index], split));
  }
  return buffer.toString();
}

/// Renders one block AST node into direct-paste text.
Future<String> _renderCopyBlock(
  MarkdownNodeStable node,
  MarkdownCopySplitter split,
) async {
  switch (node.type) {
    case MarkdownNodeType.htmlBreak:
    case MarkdownNodeType.horizontalRule:
      return '';
    case MarkdownNodeType.codeBlock:
      return _renderCopyCodeBlock(node.content);
    case MarkdownNodeType.table:
      return _renderCopyTable(node.content, split);
    case MarkdownNodeType.blockLatex:
      return extractLatexContent(node.content.trim()).trim();
    case MarkdownNodeType.image:
      return extractMarkdownImageAlt(node.content.trim());
    case MarkdownNodeType.header:
      return _renderCopyInline(node);
    case MarkdownNodeType.unorderedList:
      return '• ${_renderCopyInline(node)}';
    case MarkdownNodeType.footnoteDefinition:
      return '[${node.sourcePrefix}] ${_renderCopyInline(node)}';
    case MarkdownNodeType.orderedList:
      return '${node.sourcePrefix.trim()} ${_renderCopyInline(node)}';
    case MarkdownNodeType.blockQuote:
      return markdownToPlainTextForCopy(
        node.content,
        splitMarkdownContent: split,
      );
    case MarkdownNodeType.xmlBlock:
    case MarkdownNodeType.plainText:
    case MarkdownNodeType.bold:
    case MarkdownNodeType.italic:
    case MarkdownNodeType.inlineCode:
    case MarkdownNodeType.link:
    case MarkdownNodeType.strikethrough:
    case MarkdownNodeType.underline:
    case MarkdownNodeType.inlineLatex:
    case MarkdownNodeType.footnoteReference:
      return _renderCopyInline(node);
  }
}

/// Renders inline AST nodes without Markdown decoration.
String _renderCopyInline(MarkdownNodeStable node) {
  if (node.type == MarkdownNodeType.footnoteReference) {
    return '[${node.content}]';
  }
  if (node.type == MarkdownNodeType.inlineLatex) {
    return extractLatexContent(node.content.trim()).trim();
  }
  if (node.type == MarkdownNodeType.link) {
    return _renderCopyLink(node);
  }
  if (node.type == MarkdownNodeType.htmlBreak) {
    return '\n';
  }
  if (node.children.isNotEmpty) {
    return node.children.map(_renderCopyInline).join();
  }
  return node.content;
}

/// Renders a Markdown link as visible text followed by its destination URL.
String _renderCopyLink(MarkdownNodeStable node) {
  final text = node.children.isEmpty
      ? node.content
      : node.children.map(_renderCopyInline).join();
  final url = node.linkUrl ?? '';
  return url.isEmpty ? text : '$text ($url)';
}

/// Renders a fenced code block without its fence markers.
String _renderCopyCodeBlock(String content) {
  final lines = content.trim().split('\n');
  final firstLine = lines.firstOrNull ?? '';
  final language = firstLine.startsWith('```')
      ? firstLine.substring(3).trim()
      : '';
  final codeLines = lines
      .skipWhile((line) => line.startsWith('```'))
      .toList(growable: true);
  while (codeLines.isNotEmpty && codeLines.last.trimRight().endsWith('```')) {
    codeLines.removeLast();
  }
  final code = codeLines.join('\n');
  return language.isEmpty ? code : '----$language-----\n$code';
}

/// Renders a Markdown table as tab-separated rows.
Future<String> _renderCopyTable(
  String content,
  MarkdownCopySplitter split,
) async {
  final rows = <List<String>>[];
  for (final line in content.split('\n')) {
    final trimmed = line.trim();
    if (!trimmed.contains('|')) {
      continue;
    }
    final start = trimmed.startsWith('|') ? 1 : 0;
    final end = trimmed.endsWith('|') ? trimmed.length - 1 : trimmed.length;
    final withoutEdges = trimmed.substring(start, end < start ? start : end);
    final rawCells = withoutEdges
        .split('|')
        .map((cell) => cell.trim())
        .toList();
    if (_isCopyTableSeparator(rawCells)) continue;
    final cells = await Future.wait(
      rawCells.map((cell) async {
        final nodes = markdownNodesFromEvents(await split(cell));
        return nodes.map(_renderCopyInline).join();
      }),
    );
    if (!_isCopyTableSeparator(cells)) {
      rows.add(cells);
    }
  }
  return rows.map((row) => row.join('\t')).join('\n');
}

/// Detects a Markdown table separator row using the parsed cell values.
bool _isCopyTableSeparator(List<String> cells) {
  return cells.isNotEmpty &&
      cells.every((cell) => RegExp(r'^:?-{3,}:?$').hasMatch(cell));
}

/// Reports whether an AST node is an ordered or unordered list item.
bool _isCopyListType(MarkdownNodeType type) {
  return type == MarkdownNodeType.orderedList ||
      type == MarkdownNodeType.unorderedList;
}

/// Collapses excessive blank lines while preserving intentional paragraph gaps.
String _normalizeCopyWhitespace(String text) {
  return text.replaceAll(RegExp(r'[ \t]*\n(?:[ \t]*\n)+[ \t]*'), '\n\n').trim();
}
