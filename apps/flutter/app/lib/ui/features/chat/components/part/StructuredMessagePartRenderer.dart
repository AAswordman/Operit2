// ignore_for_file: file_names

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../common/markdown/MarkdownNodeGrouper.dart';
import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../common/markdown/StreamMarkdownRendererState.dart';
import 'ToolDisplayComponents.dart';
import 'ToolResultDisplay.dart';

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
  });

  final List<core_proxy.MessagePart> parts;
  final Color textColor;
  final Color backgroundColor;
  final bool showThinkingProcess;
  final String? rendererId;
  final void Function(String url)? onLinkClick;
  final VoidCallback? onReady;

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
        );
      case core_proxy.MessagePartKind.thinking:
        return _ThinkingPart(
          content: part.content,
          textColor: widget.textColor,
          visible: widget.showThinkingProcess,
        );
      case core_proxy.MessagePartKind.toolCall:
        final display = normalizeStructuredToolDisplay(
          part.toolName!,
          part.attributes,
        );
        return DetailedToolDisplay(
          toolName: display.toolName,
          params: display.params,
          textColor: widget.textColor,
          isStreaming: false,
        );
      case core_proxy.MessagePartKind.toolResult:
        return ToolResultDisplay(
          toolName: part.toolName!,
          result: part.content,
          isSuccess: part.attributes['status'] == 'success',
          isStreaming: false,
        );
      case core_proxy.MessagePartKind.status:
        return StreamMarkdownRenderer(
          content: part.content,
          isStreaming: false,
          textColor: widget.textColor.withValues(alpha: 0.7),
          backgroundColor: widget.backgroundColor,
          rendererId: '${widget.rendererId ?? 'message'}-${part.partId}',
          onLinkClick: widget.onLinkClick,
          onContentReady: () => _markPartReady(part.partId),
        );
    }
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

  /// Creates state that owns the live-to-structured visual handoff.
  @override
  State<StreamingStructuredMessageRenderer> createState() =>
      _StreamingStructuredMessageRendererState();
}

class _StreamingStructuredMessageRendererState
    extends State<StreamingStructuredMessageRenderer> {
  Stream<Object>? _retainedContentStream;
  late bool _structuredReady;

  /// Captures the initial live stream for uninterrupted rendering.
  @override
  void initState() {
    super.initState();
    _retainedContentStream = widget.contentStream;
    _structuredReady = widget.contentStream == null;
  }

  /// Tracks new generation streams while retaining a completed stream for handoff.
  @override
  void didUpdateWidget(covariant StreamingStructuredMessageRenderer oldWidget) {
    super.didUpdateWidget(oldWidget);
    final nextStream = widget.contentStream;
    if (nextStream != null && !identical(nextStream, _retainedContentStream)) {
      _retainedContentStream = nextStream;
      _structuredReady = false;
    }
  }

  /// Builds a stable overlay for the live output and prepared structured parts.
  @override
  Widget build(BuildContext context) {
    final streamIsActive = widget.contentStream != null;
    final showRetainedStream =
        streamIsActive || (!_structuredReady && _retainedContentStream != null);
    final buildStructuredParts = !streamIsActive;

    return Stack(
      alignment: Alignment.topLeft,
      clipBehavior: Clip.none,
      children: <Widget>[
        if (showRetainedStream)
          KeyedSubtree(
            key: const ValueKey<String>('live-markdown'),
            child: StreamMarkdownRenderer(
              content: '',
              contentStream: _retainedContentStream,
              isStreaming: widget.isStreaming,
              textColor: widget.textColor,
              backgroundColor: widget.backgroundColor,
              nodeGrouper: widget.nodeGrouper,
              state: widget.streamState,
              onLinkClick: widget.onLinkClick,
              rendererId: widget.rendererId,
              showThinkingProcess: widget.showThinkingProcess,
              initialThinkingExpanded: widget.initialThinkingExpanded,
              allowExpandedThinkingFullHeight:
                  widget.allowExpandedThinkingFullHeight,
            ),
          ),
        if (buildStructuredParts)
          IgnorePointer(
            key: const ValueKey<String>('structured-parts'),
            ignoring: !_structuredReady,
            child: Opacity(
              opacity: _structuredReady ? 1 : 0,
              child: StructuredMessagePartRenderer(
                parts: widget.parts,
                textColor: widget.textColor,
                backgroundColor: widget.backgroundColor,
                showThinkingProcess: widget.showThinkingProcess,
                rendererId: widget.rendererId,
                onLinkClick: widget.onLinkClick,
                onReady: _markStructuredReady,
              ),
            ),
          ),
      ],
    );
  }

  /// Completes the handoff after structured parts have painted once.
  void _markStructuredReady() {
    if (!mounted || widget.contentStream != null || _structuredReady) {
      return;
    }
    setState(() {
      _structuredReady = true;
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
      if (part.kind == core_proxy.MessagePartKind.markdown ||
          part.kind == core_proxy.MessagePartKind.status)
        part.partId: part.content,
  };
}

class _ThinkingPart extends StatefulWidget {
  const _ThinkingPart({
    required this.content,
    required this.textColor,
    required this.visible,
  });

  final String content;
  final Color textColor;
  final bool visible;

  /// Creates mutable state for the thinking disclosure.
  @override
  State<_ThinkingPart> createState() => _ThinkingPartState();
}

class _ThinkingPartState extends State<_ThinkingPart> {
  /// Builds the independent thinking-message disclosure.
  @override
  Widget build(BuildContext context) {
    if (!widget.visible) {
      return const SizedBox.shrink();
    }
    return ExpansionTile(
      tilePadding: EdgeInsets.zero,
      childrenPadding: const EdgeInsets.only(bottom: 8),
      title: Text(
        'Thinking',
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
          color: widget.textColor.withValues(alpha: 0.75),
        ),
      ),
      children: <Widget>[
        StreamMarkdownRenderer(
          content: widget.content,
          isStreaming: false,
          textColor: widget.textColor.withValues(alpha: 0.85),
          backgroundColor: Colors.transparent,
          selectionRoot: false,
        ),
      ],
    );
  }
}
