// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../common/markdown/MarkdownNodeGrouper.dart';
import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../common/markdown/StreamMarkdownRendererState.dart';

/// Renders live and completed assistant parts through one Markdown surface.
class StreamingStructuredMessageRenderer extends StatefulWidget {
  const StreamingStructuredMessageRenderer({
    super.key,
    required this.parts,
    required this.contentStream,
    required this.textColor,
    required this.backgroundColor,
    required this.showThinkingProcess,
    required this.nodeGrouper,
    this.mergeRender = const NoopMarkdownNodeMergeRender(),
    required this.streamState,
    this.rendererId,
    this.onLinkClick,
    this.initialThinkingExpanded = false,
    this.allowExpandedThinkingFullHeight = false,
    this.splitMarkdownContent,
  });

  final List<core_proxy.MessagePart> parts;
  final Stream<Object>? contentStream;
  final Color textColor;
  final Color backgroundColor;
  final bool showThinkingProcess;
  final MarkdownNodeGrouper nodeGrouper;
  final MarkdownNodeMergeRender mergeRender;
  final StreamMarkdownRendererState streamState;
  final String? rendererId;
  final void Function(String url)? onLinkClick;
  final bool initialThinkingExpanded;
  final bool allowExpandedThinkingFullHeight;
  final MarkdownContentSplitter? splitMarkdownContent;

  /// Creates state that keeps one physical stream attached to one renderer.
  @override
  State<StreamingStructuredMessageRenderer> createState() =>
      _StreamingStructuredMessageRendererState();
}

class _StreamingStructuredMessageRendererState
    extends State<StreamingStructuredMessageRenderer> {
  Stream<Object>? _retainedContentStream;
  bool _retainedContentStreamDone = false;

  /// Captures the initial live stream for uninterrupted rendering.
  @override
  void initState() {
    super.initState();
    _retainedContentStream = widget.contentStream;
    _retainedContentStreamDone = widget.contentStream == null;
  }

  /// Tracks new generation streams while keeping the active subscription stable.
  @override
  void didUpdateWidget(covariant StreamingStructuredMessageRenderer oldWidget) {
    super.didUpdateWidget(oldWidget);
    final nextStream = widget.contentStream;
    // Keep the active subscription stable while Flow replaces only the Dart wrapper.
    if (nextStream != null && _retainedContentStream == null) {
      _retainedContentStream = nextStream;
      _retainedContentStreamDone = false;
    }
  }

  /// Builds one Markdown renderer while retaining the stream-owned node tree.
  @override
  Widget build(BuildContext context) {
    final activeContentStream = _activeContentStream;
    return KeyedSubtree(
      key: const ValueKey<String>('assistant-markdown-surface'),
      child: StreamMarkdownRenderer(
        content: _assistantProtocolMarkupForParts(widget.parts),
        contentStream: activeContentStream,
        isStreaming: activeContentStream != null && !_retainedContentStreamDone,
        textColor: widget.textColor,
        backgroundColor: widget.backgroundColor,
        nodeGrouper: widget.nodeGrouper,
        mergeRender: widget.mergeRender,
        state: widget.streamState,
        onLinkClick: widget.onLinkClick,
        onStreamDone: _markRetainedContentStreamDone,
        rendererId: widget.rendererId,
        showThinkingProcess: widget.showThinkingProcess,
        initialThinkingExpanded: widget.initialThinkingExpanded,
        allowExpandedThinkingFullHeight: widget.allowExpandedThinkingFullHeight,
        splitMarkdownContent: widget.splitMarkdownContent!,
      ),
    );
  }

  /// Retains the physical stream so completed nodes survive parts publication.
  Stream<Object>? get _activeContentStream {
    return _retainedContentStream;
  }

  /// Marks the retained live stream as complete.
  void _markRetainedContentStreamDone() {
    if (_retainedContentStreamDone) {
      return;
    }
    setState(() {
      _retainedContentStreamDone = true;
    });
  }
}

/// Serializes canonical assistant parts into the renderer protocol source.
String _assistantProtocolMarkupForParts(List<core_proxy.MessagePart> parts) {
  final orderedParts = parts.toList(growable: false)
    ..sort((left, right) => left.sequence.compareTo(right.sequence));
  final markup = StringBuffer();
  for (final part in orderedParts) {
    switch (part.kind) {
      case core_proxy.MessagePartKind.markdown:
        markup.write(part.content);
        break;
      case core_proxy.MessagePartKind.thinking:
      case core_proxy.MessagePartKind.toolCall:
      case core_proxy.MessagePartKind.toolResult:
      case core_proxy.MessagePartKind.status:
        markup.write(_structuredPartMarkup(part));
        break;
    }
  }
  return markup.toString();
}

/// Serializes one canonical non-Markdown part for the established XML renderer.
String _structuredPartMarkup(core_proxy.MessagePart part) {
  final markup = StringBuffer();
  switch (part.kind) {
    case core_proxy.MessagePartKind.markdown:
      throw StateError('Markdown parts must use the Markdown renderer.');
    case core_proxy.MessagePartKind.thinking:
      markup
        ..write('<think>')
        ..write(part.content)
        ..write('</think>');
      break;
    case core_proxy.MessagePartKind.toolCall:
      final toolName = part.toolName;
      final toolCallId = part.toolCallId;
      if (toolName == null || toolCallId == null) {
        throw StateError('Tool-call parts require a name and call id.');
      }
      markup
        ..write('<tool name="')
        ..write(_escapeProtocolAttribute(toolName))
        ..write('" call_id="')
        ..write(_escapeProtocolAttribute(toolCallId))
        ..write('">');
      final parameterNames = part.attributes.keys.toList(growable: false)
        ..sort();
      for (final name in parameterNames) {
        markup
          ..write('<param name="')
          ..write(_escapeProtocolAttribute(name))
          ..write('">')
          ..write(part.attributes[name]!)
          ..write('</param>');
      }
      markup.write('</tool>');
      break;
    case core_proxy.MessagePartKind.toolResult:
      final toolName = part.toolName;
      if (toolName == null) {
        throw StateError('Tool-result parts require a tool name.');
      }
      markup
        ..write('<tool_result name="')
        ..write(_escapeProtocolAttribute(toolName))
        ..write('"');
      final toolCallId = part.toolCallId;
      if (toolCallId != null) {
        markup
          ..write(' call_id="')
          ..write(_escapeProtocolAttribute(toolCallId))
          ..write('"');
      }
      _writeProtocolAttributes(markup, part.attributes);
      markup
        ..write('><content>')
        ..write(part.content)
        ..write('</content></tool_result>');
      break;
    case core_proxy.MessagePartKind.status:
      markup.write('<status');
      _writeProtocolAttributes(markup, part.attributes);
      markup
        ..write('>')
        ..write(part.content)
        ..write('</status>');
      break;
  }
  return markup.toString();
}

/// Writes sorted XML-like attributes for a structured message part.
void _writeProtocolAttributes(
  StringBuffer markup,
  Map<String, String> attributes,
) {
  final names = attributes.keys.toList(growable: false)..sort();
  for (final name in names) {
    markup
      ..write(' ')
      ..write(name)
      ..write('="')
      ..write(_escapeProtocolAttribute(attributes[name]!))
      ..write('"');
  }
}

/// Escapes one XML-like attribute value before structured markup rendering.
String _escapeProtocolAttribute(String value) {
  return value
      .replaceAll('&', '&amp;')
      .replaceAll('"', '&quot;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;');
}
