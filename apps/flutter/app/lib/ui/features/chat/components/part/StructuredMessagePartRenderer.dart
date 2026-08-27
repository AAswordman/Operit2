// ignore_for_file: file_names

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../common/markdown/MarkdownNodeGrouper.dart';
import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../common/markdown/StreamMarkdownRendererState.dart';
import 'CustomXmlRenderer.dart';

class StructuredMessagePartRenderer extends StatefulWidget {
  const StructuredMessagePartRenderer({
    super.key,
    required this.parts,
    required this.textColor,
    required this.backgroundColor,
    required this.showThinkingProcess,
    this.rendererId,
    this.onLinkClick,
    this.onReady,
    this.initialThinkingExpanded = false,
    this.allowExpandedThinkingFullHeight = false,
    this.splitMarkdownContent,
  });

  final List<core_proxy.MessagePart> parts;
  final Color textColor;
  final Color backgroundColor;
  final bool showThinkingProcess;
  final String? rendererId;
  final void Function(String url)? onLinkClick;
  final VoidCallback? onReady;
  final bool initialThinkingExpanded;
  final bool allowExpandedThinkingFullHeight;
  final MarkdownContentSplitter? splitMarkdownContent;

  /// Creates readiness-tracking state for static Markdown parts.
  @override
  State<StructuredMessagePartRenderer> createState() =>
      _StructuredMessagePartRendererState();
}

class _StructuredMessagePartRendererState
    extends State<StructuredMessagePartRenderer> {
  late Set<String> _pendingMarkdownPartIds;
  var _readinessGeneration = 0;
  var _readinessScheduled = false;

  /// Initializes the pending static Markdown render set.
  @override
  void initState() {
    super.initState();
    _resetReadiness();
  }

  /// Resets readiness when the static Markdown part contents change.
  @override
  void didUpdateWidget(covariant StructuredMessagePartRenderer oldWidget) {
    super.didUpdateWidget(oldWidget);
    final previousContent = _markdownContentByPartId(oldWidget.parts);
    final nextContent = _markdownContentByPartId(widget.parts);
    if (!mapEquals(previousContent, nextContent)) {
      _resetReadiness();
      return;
    }
    if (oldWidget.onReady != widget.onReady &&
        _pendingMarkdownPartIds.isEmpty) {
      _readinessScheduled = false;
      _scheduleReadyNotification();
    }
  }

  /// Builds direct widgets for persisted semantic message parts.
  @override
  Widget build(BuildContext context) {
    final orderedParts = widget.parts.toList(growable: false)
      ..sort((left, right) => left.sequence.compareTo(right.sequence));
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        for (final part in orderedParts)
          KeyedSubtree(
            key: ValueKey<String>(part.partId),
            child: _partWidget(part),
          ),
      ],
    );
  }

  /// Creates the display widget matching one canonical message-part kind.
  Widget _partWidget(core_proxy.MessagePart part) {
    switch (part.kind) {
      case core_proxy.MessagePartKind.markdown:
        return StreamMarkdownRenderer(
          content: part.content,
          isStreaming: false,
          textColor: widget.textColor,
          backgroundColor: widget.backgroundColor,
          rendererId: '${widget.rendererId ?? 'message'}-${part.partId}',
          onLinkClick: widget.onLinkClick,
          onContentReady: () => _markPartReady(part.partId),
          splitMarkdownContent: widget.splitMarkdownContent!,
        );
      case core_proxy.MessagePartKind.thinking:
      case core_proxy.MessagePartKind.toolCall:
      case core_proxy.MessagePartKind.toolResult:
      case core_proxy.MessagePartKind.status:
        return _renderStructuredXmlPart(part);
    }
  }

  /// Renders one non-Markdown part through the original XML renderer.
  Widget _renderStructuredXmlPart(core_proxy.MessagePart part) {
    return CustomXmlRenderer(
      xmlContent: _structuredPartMarkup(part),
      isStreaming: false,
      textColor: widget.textColor,
      showThinkingProcess: widget.showThinkingProcess,
      initialThinkingExpanded: widget.initialThinkingExpanded,
      allowExpandedThinkingFullHeight: widget.allowExpandedThinkingFullHeight,
      splitMarkdownContent: widget.splitMarkdownContent!,
    );
  }

  /// Restarts readiness tracking for the current static Markdown parts.
  void _resetReadiness() {
    _readinessGeneration++;
    _readinessScheduled = false;
    _pendingMarkdownPartIds = _markdownContentByPartId(
      widget.parts,
    ).keys.toSet();
    if (_pendingMarkdownPartIds.isEmpty) {
      _scheduleReadyNotification();
    }
  }

  /// Marks one static Markdown part as painted and ready for display.
  void _markPartReady(String partId) {
    if (!_pendingMarkdownPartIds.remove(partId)) {
      return;
    }
    if (_pendingMarkdownPartIds.isEmpty) {
      _scheduleReadyNotification();
    }
  }

  /// Notifies the parent after every static Markdown part is ready.
  void _scheduleReadyNotification() {
    if (_readinessScheduled) {
      return;
    }
    _readinessScheduled = true;
    final generation = _readinessGeneration;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted ||
          generation != _readinessGeneration ||
          _pendingMarkdownPartIds.isNotEmpty) {
        return;
      }
      widget.onReady?.call();
    });
  }
}

/// Keeps completed live output visible until structured parts finish their first paint.
class StreamingStructuredMessageRenderer extends StatefulWidget {
  const StreamingStructuredMessageRenderer({
    super.key,
    required this.parts,
    required this.contentStream,
    required this.isStreaming,
    required this.textColor,
    required this.backgroundColor,
    required this.showThinkingProcess,
    required this.nodeGrouper,
    required this.streamState,
    this.rendererId,
    this.onLinkClick,
    this.initialThinkingExpanded = false,
    this.allowExpandedThinkingFullHeight = false,
    this.splitMarkdownContent,
  });

  final List<core_proxy.MessagePart> parts;
  final Stream<Object>? contentStream;
  final bool isStreaming;
  final Color textColor;
  final Color backgroundColor;
  final bool showThinkingProcess;
  final MarkdownNodeGrouper nodeGrouper;
  final StreamMarkdownRendererState streamState;
  final String? rendererId;
  final void Function(String url)? onLinkClick;
  final bool initialThinkingExpanded;
  final bool allowExpandedThinkingFullHeight;
  final MarkdownContentSplitter? splitMarkdownContent;

  /// Creates state that owns the live-to-structured visual handoff.
  @override
  State<StreamingStructuredMessageRenderer> createState() =>
      _StreamingStructuredMessageRendererState();
}

class _StreamingStructuredMessageRendererState
    extends State<StreamingStructuredMessageRenderer> {
  Stream<Object>? _retainedContentStream;

  /// Captures the initial live stream for uninterrupted rendering.
  @override
  void initState() {
    super.initState();
    _retainedContentStream = widget.contentStream;
  }

  /// Tracks new generation streams while retaining a completed stream for handoff.
  @override
  void didUpdateWidget(covariant StreamingStructuredMessageRenderer oldWidget) {
    super.didUpdateWidget(oldWidget);
    final nextStream = widget.contentStream;
    // Keep the active subscription stable while Flow replaces only the Dart wrapper.
    if (nextStream != null && _retainedContentStream == null) {
      _retainedContentStream = nextStream;
    }
  }

  /// Keeps the completed stream renderer mounted for a stable visual handoff.
  @override
  Widget build(BuildContext context) {
    final retainedStream = _retainedContentStream;
    final structuredParts = _buildStructuredParts(
      onReady: retainedStream == null ? null : _releaseRetainedContentStream,
    );
    if (retainedStream == null) {
      return structuredParts;
    }
    final liveMarkdown = _buildLiveMarkdown(retainedStream);
    final stackChildren = <Widget>[liveMarkdown];
    if (widget.contentStream == null && widget.parts.isNotEmpty) {
      stackChildren.add(
        Positioned.fill(
          child: IgnorePointer(
            child: Offstage(offstage: true, child: structuredParts),
          ),
        ),
      );
    }
    return Stack(children: stackChildren);
  }

  /// Builds the retained live Markdown renderer for a physical content stream.
  Widget _buildLiveMarkdown(Stream<Object> stream) {
    return KeyedSubtree(
      key: const ValueKey<String>('live-markdown'),
      child: StreamMarkdownRenderer(
        content: '',
        contentStream: stream,
        isStreaming: widget.isStreaming,
        textColor: widget.textColor,
        backgroundColor: widget.backgroundColor,
        nodeGrouper: widget.nodeGrouper,
        state: widget.streamState,
        onLinkClick: widget.onLinkClick,
        rendererId: widget.rendererId,
        showThinkingProcess: widget.showThinkingProcess,
        initialThinkingExpanded: widget.initialThinkingExpanded,
        allowExpandedThinkingFullHeight: widget.allowExpandedThinkingFullHeight,
        splitMarkdownContent: widget.splitMarkdownContent!,
      ),
    );
  }

  /// Builds the persisted message-part renderer used after stream completion.
  Widget _buildStructuredParts({VoidCallback? onReady}) {
    return KeyedSubtree(
      key: const ValueKey<String>('structured-parts'),
      child: StructuredMessagePartRenderer(
        parts: widget.parts,
        textColor: widget.textColor,
        backgroundColor: widget.backgroundColor,
        showThinkingProcess: widget.showThinkingProcess,
        rendererId: widget.rendererId,
        onLinkClick: widget.onLinkClick,
        onReady: onReady,
        initialThinkingExpanded: widget.initialThinkingExpanded,
        allowExpandedThinkingFullHeight: widget.allowExpandedThinkingFullHeight,
        splitMarkdownContent: widget.splitMarkdownContent,
      ),
    );
  }

  /// Releases the retained stream after persisted parts are ready to render.
  void _releaseRetainedContentStream() {
    if (!mounted || _retainedContentStream == null) {
      return;
    }
    setState(() {
      _retainedContentStream = null;
    });
  }
}

/// Returns the static Markdown content keyed by semantic part id.
Map<String, String> _markdownContentByPartId(
  List<core_proxy.MessagePart> parts,
) {
  return <String, String>{
    for (final part in parts)
      if (part.kind == core_proxy.MessagePartKind.markdown &&
          part.content.trim().isNotEmpty)
        part.partId: part.content,
  };
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
