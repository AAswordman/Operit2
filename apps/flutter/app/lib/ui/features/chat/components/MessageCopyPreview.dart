// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/markdown/MarkdownImageRenderer.dart';
import '../../../common/markdown/MarkdownInlineSpannable.dart';
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
    final l10n = AppLocalizations.of(context)!;
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
            Text(l10n.messageCopyTitle, style: theme.textTheme.titleMedium),
            const SizedBox(height: 12),
            SegmentedButton<MessageCopyFormat>(
              segments: <ButtonSegment<MessageCopyFormat>>[
                ButtonSegment<MessageCopyFormat>(
                  value: MessageCopyFormat.plainText,
                  label: Text(l10n.messageCopyPlainText),
                ),
                ButtonSegment<MessageCopyFormat>(
                  value: MessageCopyFormat.markdownSource,
                  label: Text(l10n.messageCopyMarkdownSource),
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
                              l10n.messageCopyPlainTextConversionFailed(
                                snapshot.error.toString(),
                              ),
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
                      ? l10n.messageCopyCopyPlainText
                      : l10n.messageCopyCopyMarkdownSource,
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
        SnackBar(
          content: Text(
            AppLocalizations.of(
              context,
            )!.messageCopyFailed(error.message ?? error.code),
          ),
        ),
      );
      return;
    }
    if (!context.mounted) {
      return;
    }
    ScaffoldMessenger.maybeOf(context)?.showSnackBar(
      SnackBar(
        content: Text(AppLocalizations.of(context)!.messageCopyCompleted),
      ),
    );
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
    completed = _applyCopyMarkdownEvent(state, event) || completed;
  }
  if (!completed) {
    state.eventBuilder.complete();
  }
  return _normalizeCopyWhitespace(
    _renderCopyBlocks(state.eventBuilder.toStableNodes(isStreaming: false)),
  );
}

/// Applies one runtime Markdown event to the shared AST builder.
bool _applyCopyMarkdownEvent(
  StreamMarkdownRendererState state,
  core_proxy.MarkdownStreamEvent event,
) {
  final parentBlockId = event.parentBlockId;
  if (parentBlockId != null) {
    state.eventBuilder.appendXmlMarkdownEvent(
      parentBlockId: parentBlockId,
      event: event,
    );
    return false;
  }
  switch (event.eventType) {
    case 'chunk':
      return false;
    case 'markdownBlockStart':
      final blockId = event.blockId;
      if (blockId == null) {
        throw StateError('markdownBlockStart missing blockId');
      }
      state.eventBuilder.startBlock(
        blockId: blockId,
        type: _copyMarkdownNodeType(event.nodeType),
        headerLevel: event.headerLevel,
      );
      return false;
    case 'markdownBlockChunk':
      final blockId = event.blockId;
      final value = event.value;
      if (blockId == null || value == null) {
        throw StateError('markdownBlockChunk missing blockId or value');
      }
      state.eventBuilder.appendBlock(blockId: blockId, content: value);
      return false;
    case 'markdownInlineStart':
      final blockId = event.blockId;
      final inlineId = event.inlineId;
      if (blockId == null || inlineId == null) {
        throw StateError('markdownInlineStart missing blockId or inlineId');
      }
      state.eventBuilder.startInline(
        blockId: blockId,
        inlineId: inlineId,
        type: _copyMarkdownNodeType(event.nodeType),
      );
      return false;
    case 'markdownInlineChunk':
      final blockId = event.blockId;
      final inlineId = event.inlineId;
      final value = event.value;
      if (blockId == null || inlineId == null || value == null) {
        throw StateError(
          'markdownInlineChunk missing blockId, inlineId, or value',
        );
      }
      state.eventBuilder.appendInline(
        blockId: blockId,
        inlineId: inlineId,
        content: value,
      );
      return false;
    case 'savepoint':
      final id = event.id;
      if (id == null) {
        throw StateError('savepoint missing id');
      }
      state.eventBuilder.savepoint(id);
      return false;
    case 'rollback':
      final id = event.id;
      if (id == null) {
        throw StateError('rollback missing id');
      }
      state.eventBuilder.rollback(id);
      return false;
    case 'completed':
      state.eventBuilder.complete();
      return true;
    default:
      throw StateError('Unknown Markdown copy event ${event.eventType}');
  }
}

/// Maps the runtime node label to the renderer's Markdown AST type.
MarkdownNodeType _copyMarkdownNodeType(String? label) {
  return switch (label) {
    null => MarkdownNodeType.plainText,
    'Header' => MarkdownNodeType.header,
    'BlockQuote' => MarkdownNodeType.blockQuote,
    'CodeBlock' => MarkdownNodeType.codeBlock,
    'OrderedList' => MarkdownNodeType.orderedList,
    'UnorderedList' => MarkdownNodeType.unorderedList,
    'HorizontalRule' => MarkdownNodeType.horizontalRule,
    'BlockLatex' => MarkdownNodeType.blockLatex,
    'Table' => MarkdownNodeType.table,
    'XmlBlock' => MarkdownNodeType.xmlBlock,
    'Image' => MarkdownNodeType.image,
    'Bold' => MarkdownNodeType.bold,
    'Italic' => MarkdownNodeType.italic,
    'InlineCode' => MarkdownNodeType.inlineCode,
    'Link' => MarkdownNodeType.link,
    'Strikethrough' => MarkdownNodeType.strikethrough,
    'Underline' => MarkdownNodeType.underline,
    'InlineLatex' => MarkdownNodeType.inlineLatex,
    'HtmlBreak' => MarkdownNodeType.htmlBreak,
    _ => throw StateError('Unknown Markdown node label $label'),
  };
}

/// Joins top-level AST nodes using readable block spacing.
String _renderCopyBlocks(List<MarkdownNodeStable> nodes) {
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
    buffer.write(_renderCopyBlock(contentNodes[index]));
  }
  return buffer.toString();
}

/// Renders one block AST node into direct-paste text.
String _renderCopyBlock(MarkdownNodeStable node) {
  switch (node.type) {
    case MarkdownNodeType.htmlBreak:
    case MarkdownNodeType.horizontalRule:
      return '';
    case MarkdownNodeType.codeBlock:
      return _renderCopyCodeBlock(node.content);
    case MarkdownNodeType.table:
      return _renderCopyTable(node.content);
    case MarkdownNodeType.blockLatex:
      return extractLatexContent(node.content.trim()).trim();
    case MarkdownNodeType.image:
      return extractMarkdownImageAlt(node.content.trim());
    case MarkdownNodeType.header:
      return _renderCopyInline(node).replaceFirst(RegExp(r'^#+\s*'), '');
    case MarkdownNodeType.unorderedList:
      return '• ${_renderCopyInline(node)}';
    case MarkdownNodeType.orderedList:
    case MarkdownNodeType.blockQuote:
    case MarkdownNodeType.xmlBlock:
    case MarkdownNodeType.plainText:
    case MarkdownNodeType.bold:
    case MarkdownNodeType.italic:
    case MarkdownNodeType.inlineCode:
    case MarkdownNodeType.link:
    case MarkdownNodeType.strikethrough:
    case MarkdownNodeType.underline:
    case MarkdownNodeType.inlineLatex:
      return _renderCopyInline(node);
  }
}

/// Renders inline AST nodes without Markdown decoration.
String _renderCopyInline(MarkdownNodeStable node) {
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
  final text = node.children.isNotEmpty
      ? node.children.map(_renderCopyInline).join()
      : parseInlineSegments(
          extractLinkText(node.content),
        ).map(_renderCopyInlineSegment).join();
  final url = extractLinkUrl(node.content);
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
String _renderCopyTable(String content) {
  final rows = <List<String>>[];
  for (final line in content.split('\n')) {
    final trimmed = line.trim();
    if (!RegExp(r'\|').hasMatch(trimmed)) {
      continue;
    }
    final withoutEdges = trimmed
        .replaceFirst(RegExp(r'^\|'), '')
        .replaceFirst(RegExp(r'\|$'), '');
    final cells = withoutEdges
        .split('|')
        .map((cell) {
          final segments = parseInlineSegments(cell.trim());
          return segments.map(_renderCopyInlineSegment).join();
        })
        .toList(growable: false);
    if (!_isCopyTableSeparator(cells)) {
      rows.add(cells);
    }
  }
  return rows.map((row) => row.join('\t')).join('\n');
}

/// Converts one inline Markdown segment into plain text.
String _renderCopyInlineSegment(MarkdownInlineSegment segment) {
  if (segment.nodeType == 'InlineLatex') {
    return extractLatexContent(segment.text.trim()).trim();
  }
  if (segment.nodeType == 'Link') {
    final text = segment.children.isEmpty
        ? extractLinkText(segment.text)
        : segment.children.map(_renderCopyInlineSegment).join();
    final url = extractLinkUrl(segment.text);
    return url.isEmpty ? text : '$text ($url)';
  }
  if (segment.children.isNotEmpty) {
    return segment.children.map(_renderCopyInlineSegment).join();
  }
  return resolveNestedInlineText(segment);
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
