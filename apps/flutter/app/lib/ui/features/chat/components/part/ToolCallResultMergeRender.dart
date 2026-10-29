// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../util/ChatMarkupRegex.dart';
import '../../../../common/markdown/MarkdownNodeGrouper.dart';
import 'ToolDisplayComponents.dart';

/// Merges ordered tool calls with any results that have arrived so far.
class ToolCallResultMergeRender extends MarkdownNodeMergeRender {
  const ToolCallResultMergeRender();

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
    var lastToolIndex = startIndex;
    for (var index = startIndex; index <= endIndexInclusive; index += 1) {
      final node = nodes[index];
      if (_isToolMergeSeparator(node)) {
        continue;
      }
      final parsed = _parseToolMergeNode(node);
      if (!readingResults && parsed?.kind == _ToolMergeNodeKind.call) {
        callNames.add(parsed!.matchToolName);
        lastToolIndex = index;
        continue;
      }
      if (parsed?.kind == _ToolMergeNodeKind.result &&
          resultCount < callNames.length &&
          parsed!.matchToolName == callNames[resultCount]) {
        readingResults = true;
        resultCount += 1;
        lastToolIndex = index;
        continue;
      }
      break;
    }
    return MarkdownMergeMatch(
      startIndex: startIndex,
      endIndexInclusive: lastToolIndex,
      stableKey: 'tool-calls-$startIndex',
    );
  }

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
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        for (var index = 0; index < calls.length; index += 1)
          MergedToolCallResultRow(
            key: ValueKey<String>(
              '$renderInstanceKey-${calls[index].node.callId ?? calls[index].index}',
            ),
            toolName: calls[index].node.toolName,
            callId: calls[index].node.callId,
            display:
                calls[index].node.display ??
                (index < results.length ? results[index].node.display : null),
            resultData: index < results.length
                ? results[index].node.resultData
                : null,
            parameterMarkup: calls[index].node.parameterMarkup,
            resultText: index < results.length
                ? results[index].node.resultText
                : '',
            isSuccess: index < results.length
                ? results[index].node.isSuccess
                : null,
            textColor: textColor,
            isStreaming:
                index >= results.length ||
                (nodes[results[index].index].isStreaming &&
                    nodes[results[index].index].xmlIsClosed != true),
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
    this.callId,
    this.display,
    this.resultData,
  });

  final _ToolMergeNodeKind kind;
  final String toolName;
  final String matchToolName;
  final String parameterMarkup;
  final String resultText;
  final bool isSuccess;
  final String? callId;
  final ToolDisplayDeclaration? display;
  final Map<String, dynamic>? resultData;
}

class _IndexedToolMergeNode {
  const _IndexedToolMergeNode({required this.index, required this.node});

  final int index;
  final _ToolMergeNode node;
}

/// Adapts one call and its current result to the shared native tool row.
class MergedToolCallResultRow extends StatelessWidget {
  const MergedToolCallResultRow({
    super.key,
    required this.toolName,
    required this.parameterMarkup,
    required this.resultText,
    required this.isSuccess,
    required this.textColor,
    required this.isStreaming,
    this.callId,
    this.display,
    this.resultData,
  });

  final String toolName;
  final String? callId;
  final ToolDisplayDeclaration? display;
  final Map<String, dynamic>? resultData;
  final String parameterMarkup;
  final String resultText;
  final bool? isSuccess;
  final Color textColor;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    return ToolCallRow(
      presentation: ToolPresentation.fromCall(
        toolName,
        parameterMarkup,
        callId: callId,
        result: resultText,
        isSuccess: isSuccess,
        display: display,
        resultData: resultData,
      ),
      isStreaming: isStreaming,
    );
  }
}

/// Returns the most recent tool call that still lacks a matched result.
String? resolvePendingToolName(Iterable<MarkdownNodeStable> nodes) {
  final pendingCalls = <_ToolMergeNode>[];
  for (final node in nodes) {
    final parsed = _parseToolMergeNode(node);
    if (parsed?.kind == _ToolMergeNodeKind.call) {
      pendingCalls.add(parsed!);
      continue;
    }
    if (parsed?.kind == _ToolMergeNodeKind.result) {
      final result = parsed!;
      final matchIndex = pendingCalls.indexWhere(
        (call) => call.matchToolName == result.matchToolName,
      );
      if (matchIndex != -1) {
        pendingCalls.removeAt(matchIndex);
      }
    }
  }
  return pendingCalls.isEmpty ? null : pendingCalls.last.toolName;
}

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
  final callId = node.xmlAttributes?['call_id']?.trim();
  if (normalizedTag == 'tool') {
    final matchToolName = _resolveProxyTargetToolName(toolName, node);
    if (matchToolName == null) {
      return null;
    }
    return _ToolMergeNode(
      kind: _ToolMergeNodeKind.call,
      toolName: toolName,
      callId: callId,
      matchToolName: matchToolName,
      display: toolDisplayFromAttributes(node.xmlAttributes),
      parameterMarkup: _parameterMarkup(node.xmlChildren),
      resultText: '',
      isSuccess: false,
    );
  }
  final status = node.xmlAttributes?['status']?.trim().toLowerCase();
  return _ToolMergeNode(
    kind: _ToolMergeNodeKind.result,
    toolName: toolName,
    callId: callId,
    matchToolName: toolName,
    parameterMarkup: '',
    display: toolDisplayFromAttributes(node.xmlAttributes),
    resultData: decodeToolMetadata(node.xmlAttributes?['result_data']),
    resultText: _resultText(node),
    isSuccess: status == null || status.isEmpty || status == 'success',
  );
}

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
  return toolName;
}

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

String _resultText(MarkdownNodeStable node) {
  for (final child in node.xmlChildren) {
    if (child.tagName == 'content') return child.body.trim();
  }
  return node.xmlBody?.trim() ?? '';
}

String _escapeXmlAttribute(String value) {
  return value
      .replaceAll('&', '&amp;')
      .replaceAll('"', '&quot;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;');
}

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
