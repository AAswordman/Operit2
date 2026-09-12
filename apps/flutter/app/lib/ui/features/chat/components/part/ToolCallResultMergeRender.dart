// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../../util/ChatMarkupRegex.dart';
import '../../../../common/components/OperitDialog.dart';
import '../../../../common/interactions/MessagePressShield.dart';
import '../../../../common/markdown/MarkdownNodeGrouper.dart';
import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import 'ToolDisplayComponents.dart';
import 'XmlCanvasSummaryComponents.dart';

/// Merges ordered tool calls and equally ordered, name-matched results.
class ToolCallResultMergeRender extends MarkdownNodeMergeRender {
  const ToolCallResultMergeRender();

  /// Matches either one call/result pair or one batched calls/results sequence.
  @override
  MarkdownMergeMatch? match({
    required List<MarkdownNodeStable> nodes,
    required int startIndex,
    required int endIndexInclusive,
  }) {
    if (startIndex < 0 ||
        startIndex >= nodes.length ||
        endIndexInclusive >= nodes.length ||
        startIndex > endIndexInclusive) {
      throw RangeError.range(startIndex, 0, endIndexInclusive, 'startIndex');
    }
    final first = _parseToolMergeNode(nodes[startIndex]);
    if (first?.kind != _ToolMergeNodeKind.call) {
      return null;
    }

    final callNames = <String>[];
    var resultCount = 0;
    var readingResults = false;
    for (var index = startIndex; index <= endIndexInclusive; index += 1) {
      final node = nodes[index];
      if (_isToolMergeSeparator(node)) {
        continue;
      }
      final parsed = _parseToolMergeNode(node);
      if (!readingResults && parsed?.kind == _ToolMergeNodeKind.call) {
        callNames.add(parsed!.matchToolName);
        continue;
      }
      if (parsed?.kind == _ToolMergeNodeKind.result) {
        readingResults = true;
        if (resultCount >= callNames.length ||
            parsed!.matchToolName != callNames[resultCount]) {
          return null;
        }
        resultCount += 1;
        if (resultCount == callNames.length) {
          return MarkdownMergeMatch(
            startIndex: startIndex,
            endIndexInclusive: index,
            stableKey: 'tool-results-$startIndex-$index',
          );
        }
        continue;
      }
      return null;
    }
    return null;
  }

  /// Renders every matched call/result pair as one compact row.
  @override
  Widget renderMerge({
    required MarkdownMergeMatch match,
    required List<MarkdownNodeStable> nodes,
    required String rendererId,
    required Color textColor,
    required MarkdownXmlRenderer xmlRenderer,
    required Stream<String>? Function(int index) xmlStreamResolver,
    required Stream<Object>? Function(int index) xmlMarkdownEventStreamResolver,
    required String renderInstanceKey,
  }) {
    final calls = <_IndexedToolMergeNode>[];
    final results = <_IndexedToolMergeNode>[];
    for (
      var index = match.startIndex;
      index <= match.endIndexInclusive;
      index += 1
    ) {
      final parsed = _parseToolMergeNode(nodes[index]);
      switch (parsed?.kind) {
        case _ToolMergeNodeKind.call:
          calls.add(_IndexedToolMergeNode(index: index, node: parsed!));
          break;
        case _ToolMergeNodeKind.result:
          results.add(_IndexedToolMergeNode(index: index, node: parsed!));
          break;
        case null:
          break;
      }
    }
    if (calls.length != results.length) {
      throw StateError('A tool merge match must contain complete pairs.');
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        for (var index = 0; index < calls.length; index += 1)
          MergedToolCallResultRow(
            key: ValueKey<String>(
              '$renderInstanceKey-${calls[index].index}-${results[index].index}',
            ),
            toolName: calls[index].node.toolName,
            parameterMarkup: calls[index].node.parameterMarkup,
            resultText: results[index].node.resultText,
            isSuccess: results[index].node.isSuccess,
            textColor: textColor,
            isStreaming: nodes[results[index].index].isStreaming,
          ),
      ],
    );
  }
}

enum _ToolMergeNodeKind { call, result }

class _ToolMergeNode {
  const _ToolMergeNode({
    required this.kind,
    required this.toolName,
    required this.matchToolName,
    required this.parameterMarkup,
    required this.resultText,
    required this.isSuccess,
  });

  final _ToolMergeNodeKind kind;
  final String toolName;
  final String matchToolName;
  final String parameterMarkup;
  final String resultText;
  final bool isSuccess;
}

class _IndexedToolMergeNode {
  const _IndexedToolMergeNode({required this.index, required this.node});

  final int index;
  final _ToolMergeNode node;
}

/// Displays one completed tool invocation as a compact merged row.
class MergedToolCallResultRow extends StatelessWidget {
  const MergedToolCallResultRow({
    super.key,
    required this.toolName,
    required this.parameterMarkup,
    required this.resultText,
    required this.isSuccess,
    required this.textColor,
    required this.isStreaming,
  });

  final String toolName;
  final String parameterMarkup;
  final String resultText;
  final bool isSuccess;
  final Color textColor;
  final bool isStreaming;

  /// Builds one completion row with details available through one interaction.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final display = normalizeToolDisplayForStrictProxy(
      toolName,
      parameterMarkup,
    );
    final requestSummary = buildParamsHeadPreview(display.params);

    /// Opens the complete request and result for this merged row.
    void openDetail() {
      _showMergedToolDetailDialog(
        context,
        toolName: display.toolName,
        parameterMarkup: display.params,
        resultText: resultText,
        isSuccess: isSuccess,
      );
    }

    return CanvasToolSummaryRow(
      toolName: display.toolName,
      summary: requestSummary,
      semanticDescription: buildToolSemanticDescription(
        display.toolName,
        display.params,
        useByteSummary: false,
      ),
      leadingIcon: getToolIcon(display.toolName),
      titleColor: theme.colorScheme.primary,
      summaryColor: textColor.withValues(alpha: 0.7),
      onClick: openDetail,
      trailing: _MergedToolResultActions(
        isSuccess: isSuccess,
        resultText: resultText,
        isStreaming: isStreaming,
      ),
    );
  }
}

class _MergedToolResultActions extends StatelessWidget {
  const _MergedToolResultActions({
    required this.isSuccess,
    required this.resultText,
    required this.isStreaming,
  });

  final bool isSuccess;
  final String resultText;
  final bool isStreaming;

  /// Builds fixed-height result actions without changing the tool row geometry.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final statusColor = isSuccess
        ? theme.colorScheme.primary
        : theme.colorScheme.error;
    return SizedBox(
      height: 20,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(
            isSuccess ? Icons.check : Icons.close,
            size: 15,
            color: statusColor,
          ),
          if (resultText.isNotEmpty) ...<Widget>[
            const SizedBox(width: 4),
            MessagePressShieldRegion(
              child: Tooltip(
                message: 'Copy result',
                child: InkResponse(
                  onTap: () {
                    unawaited(_copyMergedToolResult(resultText));
                  },
                  radius: 14,
                  child: Icon(
                    Icons.content_copy,
                    size: 14,
                    color: theme.colorScheme.primary.withValues(alpha: 0.6),
                  ),
                ),
              ),
            ),
          ],
          if (isStreaming) ...<Widget>[
            const SizedBox(width: 6),
            const StreamingCursor(),
          ],
        ],
      ),
    );
  }
}

/// Opens the full call and result content for one merged row.
void _showMergedToolDetailDialog(
  BuildContext context, {
  required String toolName,
  required String parameterMarkup,
  required String resultText,
  required bool isSuccess,
}) {
  showDialog<void>(
    context: context,
    builder: (dialogContext) {
      final theme = Theme.of(dialogContext);
      return OperitDialogScaffold(
        title: toolName,
        icon: Icon(getToolIcon(toolName), size: 20),
        maxWidth: 720,
        actions: <Widget>[
          FilledButton(
            onPressed: () {
              Navigator.of(dialogContext).pop();
            },
            child: const Text('Close'),
          ),
        ],
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxHeight: 420),
          child: SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                Text('Call parameters', style: theme.textTheme.labelMedium),
                const SizedBox(height: 6),
                _MergedToolDetailSurface(
                  text: parameterMarkup,
                  color: theme.colorScheme.surfaceContainerHighest,
                ),
                const SizedBox(height: 12),
                Row(
                  children: <Widget>[
                    Icon(
                      isSuccess ? Icons.check : Icons.close,
                      size: 16,
                      color: isSuccess
                          ? theme.colorScheme.primary
                          : theme.colorScheme.error,
                    ),
                    const SizedBox(width: 6),
                    Text('Result', style: theme.textTheme.labelMedium),
                    const Spacer(),
                    if (resultText.isNotEmpty)
                      IconButton(
                        onPressed: () {
                          unawaited(_copyMergedToolResult(resultText));
                        },
                        icon: const Icon(Icons.content_copy, size: 18),
                      ),
                  ],
                ),
                const SizedBox(height: 6),
                _MergedToolDetailSurface(
                  text: resultText,
                  color: isSuccess
                      ? theme.colorScheme.surfaceContainerHighest
                      : theme.colorScheme.errorContainer,
                ),
              ],
            ),
          ),
        ),
      );
    },
  );
}

class _MergedToolDetailSurface extends StatelessWidget {
  const _MergedToolDetailSurface({required this.text, required this.color});

  final String text;
  final Color color;

  /// Builds one selectable detail surface in the merged tool dialog.
  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.35),
        borderRadius: BorderRadius.circular(6),
      ),
      child: SelectableText(text, style: Theme.of(context).textTheme.bodySmall),
    );
  }
}

/// Copies the merged result while keeping platform failures observable.
Future<void> _copyMergedToolResult(String resultText) async {
  try {
    await Clipboard.setData(ClipboardData(text: resultText));
  } on PlatformException catch (error, stackTrace) {
    debugPrint('Failed to copy merged tool result: $error\n$stackTrace');
  }
}

/// Parses one complete tool-like XML node for deterministic merge matching.
_ToolMergeNode? _parseToolMergeNode(MarkdownNodeStable node) {
  if (node.type != MarkdownNodeType.xmlBlock) {
    return null;
  }
  final normalizedTag = ChatMarkupRegex.normalizeToolLikeTagName(
    node.xmlTagName,
  );
  if (normalizedTag != 'tool' && normalizedTag != 'tool_result') {
    return null;
  }
  final toolName = node.xmlAttributes?['name']?.trim();
  if (toolName == null || toolName.isEmpty) {
    return null;
  }
  if (normalizedTag == 'tool') {
    final matchToolName = _resolveProxyTargetToolName(toolName, node);
    if (matchToolName == null) {
      return null;
    }
    return _ToolMergeNode(
      kind: _ToolMergeNodeKind.call,
      toolName: toolName,
      matchToolName: matchToolName,
      parameterMarkup: _parameterMarkup(node.xmlChildren),
      resultText: '',
      isSuccess: false,
    );
  }
  final status = node.xmlAttributes?['status']?.trim().toLowerCase();
  return _ToolMergeNode(
    kind: _ToolMergeNodeKind.result,
    toolName: toolName,
    matchToolName: toolName,
    parameterMarkup: '',
    resultText: _resultText(node),
    isSuccess: status == null || status.isEmpty || status == 'success',
  );
}

/// Resolves proxy wrappers to the concrete target named by their parameters.
String? _resolveProxyTargetToolName(String toolName, MarkdownNodeStable node) {
  if (toolName != 'proxy' && toolName != 'package_proxy') {
    return toolName;
  }
  for (final parameter in node.xmlChildren) {
    if (parameter.tagName?.toLowerCase() == 'param' &&
        parameter.attributes?['name']?.trim() == 'tool_name') {
      final targetToolName = parameter.body.trim();
      return targetToolName.isEmpty ? null : targetToolName;
    }
  }
  return null;
}

/// Serializes Rust-decoded parameter children for the existing tool display.
String _parameterMarkup(List<MarkdownXmlChildStable> children) {
  return children
      .where((child) => child.tagName != null)
      .map((child) {
        final attributes =
            child.attributes?.entries
                .map(
                  (entry) =>
                      ' ${entry.key}="${_escapeXmlAttribute(entry.value)}"',
                )
                .join() ??
            '';
        final tag = child.tagName!;
        return '<$tag$attributes>${child.body}</$tag>';
      })
      .join()
      .trim();
}

/// Returns readable result text from direct child metadata without reparsing XML.
String _resultText(MarkdownNodeStable node) {
  if (node.xmlChildren.isEmpty) {
    return node.xmlBody?.trim() ?? '';
  }
  return node.xmlChildren.map((child) => child.body).join().trim();
}

/// Escapes an attribute value reconstructed from stream metadata.
String _escapeXmlAttribute(String value) {
  return value
      .replaceAll('&', '&amp;')
      .replaceAll('"', '&quot;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;');
}

/// Identifies formatting-only nodes permitted inside one tool batch.
bool _isToolMergeSeparator(MarkdownNodeStable node) {
  if (node.type == MarkdownNodeType.plainText) {
    return node.content.trim().isEmpty;
  }
  if (node.type == MarkdownNodeType.htmlBreak) {
    return true;
  }
  if (node.type != MarkdownNodeType.xmlBlock) {
    return false;
  }
  return node.xmlTagName?.toLowerCase() == 'meta';
}
