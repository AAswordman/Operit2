// ignore_for_file: file_names

import 'dart:async';
import 'dart:collection';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../interactions/MessagePressShield.dart';
import 'EnhancedCodeBlock.dart';
import 'EnhancedTableBlock.dart';
import 'MarkdownNodeGrouper.dart';
import 'MarkdownBlockQuote.dart';
import 'MarkdownImageRenderer.dart';
import 'MarkdownInlineSpannable.dart';
import 'MarkdownLatexBlock.dart';
import 'StreamMarkdownRendererState.dart';
import '../../features/chat/components/part/CustomXmlRenderer.dart';

part 'CanvasMarkdownNodeRenderer.dart';

typedef MarkdownContentSplitter =
    Future<List<core_proxy.MarkdownStreamEvent>> Function(String content);

class StreamMarkdownRenderer extends StatefulWidget {
  const StreamMarkdownRenderer({
    super.key,
    required this.content,
    required this.isStreaming,
    required this.textColor,
    required this.backgroundColor,
    this.nodeGrouper = const NoopMarkdownNodeGrouper(),
    this.contentStream,
    this.rendererId,
    this.state,
    this.onLinkClick,
    this.showThinkingProcess = true,
    this.initialThinkingExpanded = false,
    this.allowExpandedThinkingFullHeight = false,
    this.selectionRoot = true,
    this.onContentReady,
    this.onStreamDone,
    required this.splitMarkdownContent,
  });

  final String content;
  final bool isStreaming;
  final Color textColor;
  final Color backgroundColor;
  final MarkdownNodeGrouper nodeGrouper;
  final Stream<Object>? contentStream;
  final String? rendererId;
  final StreamMarkdownRendererState? state;
  final void Function(String url)? onLinkClick;
  final bool showThinkingProcess;
  final bool initialThinkingExpanded;
  final bool allowExpandedThinkingFullHeight;
  final bool selectionRoot;
  final VoidCallback? onContentReady;
  final VoidCallback? onStreamDone;
  final MarkdownContentSplitter splitMarkdownContent;

  @override
  State<StreamMarkdownRenderer> createState() => _StreamMarkdownRendererState();
}

const Duration _streamRenderInterval = Duration(milliseconds: 200);
const Duration _nodeFadeInDuration = Duration(milliseconds: 800);

class _StreamMarkdownRendererState extends State<StreamMarkdownRenderer> {
  StreamSubscription<Object>? _subscription;
  Timer? _renderTimer;
  late String _rendererId;
  late StreamMarkdownRendererState _rendererState;
  final Set<String> _scheduledVisibleNodeKeys = <String>{};
  bool _streamDone = false;
  bool _streamDoneNotified = false;
  int _startGeneration = 0;
  int _streamEventCount = 0;

  @override
  void initState() {
    super.initState();
    _rendererId = _computeRendererId();
    _rendererState = widget.state ?? StreamMarkdownRendererState();
    _startCurrentContent(reason: 'init');
  }

  @override
  void didUpdateWidget(covariant StreamMarkdownRenderer oldWidget) {
    super.didUpdateWidget(oldWidget);
    final nextRendererId = _computeRendererId();
    final stateChanged =
        widget.state != null && widget.state != oldWidget.state;
    if (stateChanged) {
      _rendererState = widget.state!;
    }
    final streamChanged = oldWidget.contentStream != widget.contentStream;
    final staticContentChanged =
        widget.contentStream == null && oldWidget.content != widget.content;
    if (nextRendererId != _rendererId ||
        stateChanged ||
        streamChanged ||
        staticContentChanged) {
      final reasons = <String>[
        if (nextRendererId != _rendererId) 'rendererId',
        if (stateChanged) 'state',
        if (streamChanged) 'stream',
        if (staticContentChanged) 'staticContent',
      ].join(',');
      if (_shouldLogMarkdownLifecycle(widget.contentStream, reasons)) {
        _logRendererTrace(
          'restart_request reason=$reasons '
          'oldRendererId=$_rendererId newRendererId=$nextRendererId '
          'oldStream=${_streamTraceId(oldWidget.contentStream)} '
          'newStream=${_streamTraceId(widget.contentStream)} '
          'chars=${_rendererState.collectedContent.length} '
          'nodes=${_rendererState.renderNodes.length} '
          'done=$_streamDone',
        );
      }
      _rendererId = nextRendererId;
      _startCurrentContent(reason: reasons);
    } else if (oldWidget.onContentReady != widget.onContentReady &&
        widget.contentStream == null &&
        _rendererState.streamParsingCompletedSuccessfully) {
      _notifyContentReady(_startGeneration);
    }
  }

  String _computeRendererId() {
    final explicitRendererId = widget.rendererId;
    if (explicitRendererId != null) {
      return explicitRendererId;
    }
    final stream = widget.contentStream;
    if (stream != null) {
      return 'renderer-${identityHashCode(stream)}';
    }
    return 'static-renderer-${widget.content.hashCode}';
  }

  /// Starts rendering the current static content or live content stream.
  void _startCurrentContent({required String reason}) {
    final generation = ++_startGeneration;
    _subscription?.cancel();
    _renderTimer?.cancel();
    _renderTimer = null;
    _scheduledVisibleNodeKeys.clear();
    _streamEventCount = 0;
    _streamDoneNotified = false;

    final stream = widget.contentStream;
    final shouldTrace = _shouldLogMarkdownLifecycle(stream, reason);
    if (stream == null &&
        _rendererState.streamParsingCompletedSuccessfully &&
        _rendererState.collectedContent.toString() == widget.content &&
        _rendererState.renderNodes.isNotEmpty) {
      if (shouldTrace) {
        _logRendererTrace(
          'static_reuse reason=$reason rendererId=$_rendererId '
          'chars=${widget.content.length} nodes=${_rendererState.renderNodes.length}',
        );
      }
      _rendererState.xmlNodeStreams.clear();
      _rendererState.xmlMarkdownEventStreams.clear();
      _streamDone = true;
      _notifyContentReady(generation);
      return;
    }

    if (shouldTrace) {
      _logRendererTrace(
        'reset reason=$reason rendererId=$_rendererId '
        'stream=${_streamTraceId(stream)} '
        'previousChars=${_rendererState.collectedContent.length} '
        'previousNodes=${_rendererState.renderNodes.length} '
        'wasDone=$_streamDone',
      );
    }
    _streamDone = stream == null;
    _rendererState.reset();
    if (stream == null) {
      final cachedNodes = _staticMarkdownNodeCache.get(widget.content);
      if (cachedNodes != null) {
        if (shouldTrace) {
          _logRendererTrace(
            'static_cache_hit rendererId=$_rendererId '
            'chars=${widget.content.length} nodes=${cachedNodes.length}',
          );
        }
        _streamDone = true;
        _rendererState.collectedContent.write(widget.content);
        _rendererState.streamParsingCompletedSuccessfully = true;
        _rendererState.renderNodes.addAll(cachedNodes);
        for (var index = 0; index < cachedNodes.length; index++) {
          _rendererState.nodeAnimationStates[_nodeKeyForIndex(
                _rendererId,
                index,
              )] =
              true;
        }
        _notifyContentReady(generation);
        return;
      }
      _loadStaticContent(widget.content, generation);
      return;
    }

    _synchronizeRenderNodes(isStreaming: true);
    _subscribe(stream);
  }

  Future<void> _loadStaticContent(String content, int generation) async {
    final events = await widget.splitMarkdownContent(content);
    if (!mounted || generation != _startGeneration) {
      return;
    }
    for (final event in events) {
      _applyMarkdownEvent(event);
    }
    _rendererState.eventBuilder.complete();
    _streamDone = true;
    _rendererState.streamParsingCompletedSuccessfully = true;
    _synchronizeRenderNodes(isStreaming: false);
    _staticMarkdownNodeCache.put(content, _rendererState.renderNodes);
    for (var index = 0; index < _rendererState.renderNodes.length; index++) {
      _rendererState.nodeAnimationStates[_nodeKeyForIndex(_rendererId, index)] =
          true;
    }
    setState(() {});
    _notifyContentReady(generation);
  }

  /// Reports static render readiness after its completed nodes have painted once.
  void _notifyContentReady(int generation) {
    final callback = widget.onContentReady;
    if (callback == null) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted ||
          generation != _startGeneration ||
          widget.contentStream != null) {
        return;
      }
      callback();
    });
  }

  void _subscribe(Stream<Object> stream) {
    _logRendererTrace(
      'subscribe rendererId=$_rendererId stream=${_streamTraceId(stream)}',
    );
    _subscription = stream.listen(
      (event) {
        _streamEventCount += 1;
        if (_shouldLogRenderStreamEvent(_streamEventCount)) {
          _logRendererTrace(
            'event rendererId=$_rendererId stream=${_streamTraceId(stream)} '
            'count=$_streamEventCount type=${event.runtimeType} '
            'chars=${_rendererState.collectedContent.length} '
            'nodes=${_rendererState.renderNodes.length}',
          );
        }
        _applyMarkdownEvent(event);
        _renderTimer ??= Timer(_streamRenderInterval, _flushRenderNodes);
      },
      onDone: () {
        _rendererState.eventBuilder.complete();
        _streamDone = true;
        _rendererState.streamParsingCompletedSuccessfully = true;
        _renderTimer?.cancel();
        _renderTimer = null;
        _logRendererTrace(
          'done rendererId=$_rendererId stream=${_streamTraceId(stream)} '
          'events=$_streamEventCount '
          'chars=${_rendererState.collectedContent.length} '
          'nodes=${_rendererState.renderNodes.length}',
        );
        _flushRenderNodes();
        _notifyStreamDone();
      },
      onError: (Object error, StackTrace stackTrace) {
        _rendererState.streamParsingCompletedSuccessfully = false;
        _logRendererTrace(
          'error rendererId=$_rendererId stream=${_streamTraceId(stream)} '
          'events=$_streamEventCount error=$error',
        );
      },
    );
  }

  /// Reports that the live Markdown stream has reached its terminal event.
  void _notifyStreamDone() {
    if (_streamDoneNotified) {
      return;
    }
    _streamDoneNotified = true;
    widget.onStreamDone?.call();
  }

  /// Returns a stable short label for one Dart stream object in logs.
  String _streamTraceId(Stream<Object>? stream) {
    return stream == null ? 'none' : identityHashCode(stream).toString();
  }

  /// Returns whether one render stream event should be logged.
  bool _shouldLogRenderStreamEvent(int count) {
    return count <= 5 || count % 256 == 0;
  }

  /// Emits one Markdown renderer lifecycle log entry.
  void _logRendererTrace(String message) {
    debugPrint('ChatRenderTrace markdown.$message');
  }

  void _flushRenderNodes() {
    _renderTimer = null;
    if (!mounted) {
      return;
    }
    setState(() => _synchronizeRenderNodes(isStreaming: !_streamDone));
  }

  void _synchronizeRenderNodes({required bool isStreaming}) {
    final nextNodes = _rendererState.eventBuilder.toStableNodes(
      isStreaming: isStreaming,
    );
    final nextKeys = <String>{
      for (var index = 0; index < nextNodes.length; index++)
        _nodeKeyForIndex(_rendererId, index),
    };
    final keysToReveal = <String>[];

    for (final key in nextKeys) {
      if (!_rendererState.nodeAnimationStates.containsKey(key)) {
        _rendererState.nodeAnimationStates[key] = false;
        keysToReveal.add(key);
      }
    }
    _rendererState.nodeAnimationStates.removeWhere(
      (key, value) => !nextKeys.contains(key),
    );
    _scheduledVisibleNodeKeys.removeWhere((key) => !nextKeys.contains(key));
    _rendererState.renderNodes
      ..clear()
      ..addAll(nextNodes);
    _scheduleNodeFadeIn(keysToReveal);
  }

  void _applyMarkdownEvent(Object event) {
    final normalized = _NormalizedMarkdownEvent.from(event);
    final traceEvent = _shouldLogMarkdownEventDetail(
      normalized,
      _streamEventCount,
    );
    if (traceEvent) {
      _logRendererTrace(
        'apply.begin rendererId=$_rendererId '
        'stream=${_streamTraceId(widget.contentStream)} '
        'count=$_streamEventCount ${normalized.traceSummary()} '
        'beforeChars=${_rendererState.collectedContent.length} '
        'beforeNodes=${_rendererState.renderNodes.length} done=$_streamDone',
      );
    }
    final parentBlockId = normalized.parentBlockId;
    if (parentBlockId != null) {
      _rendererState.eventBuilder.appendXmlMarkdownEvent(
        parentBlockId: parentBlockId,
        event: normalized.asLocalEvent(),
      );
      if (traceEvent) {
        _logRendererTrace(
          'apply.end rendererId=$_rendererId '
          'stream=${_streamTraceId(widget.contentStream)} '
          'count=$_streamEventCount ${normalized.traceSummary()} '
          'afterChars=${_rendererState.collectedContent.length} '
          'afterNodes=${_rendererState.renderNodes.length} done=$_streamDone',
        );
      }
      return;
    }
    switch (normalized.type) {
      case 'reset':
        _rendererState.reset();
        _scheduledVisibleNodeKeys.clear();
        _streamDone = false;
        break;
      case 'chunk':
        final value = normalized.value;
        if (value != null) {
          _rendererState.collectedContent.write(value);
        }
        break;
      case 'markdownBlockStart':
        final blockId = normalized.blockId;
        if (blockId == null) {
          throw StateError('markdownBlockStart missing blockId');
        }
        _rendererState.eventBuilder.startBlock(
          blockId: blockId,
          type: _nodeTypeFromLabel(normalized.nodeType),
          headerLevel: normalized.headerLevel,
        );
        break;
      case 'markdownBlockChunk':
        final blockId = normalized.blockId;
        final value = normalized.value;
        if (blockId == null || value == null) {
          throw StateError('markdownBlockChunk missing blockId or value');
        }
        _rendererState.eventBuilder.appendBlock(
          blockId: blockId,
          content: value,
        );
        break;
      case 'markdownInlineStart':
        final blockId = normalized.blockId;
        final inlineId = normalized.inlineId;
        if (blockId == null || inlineId == null) {
          throw StateError('markdownInlineStart missing blockId or inlineId');
        }
        _rendererState.eventBuilder.startInline(
          blockId: blockId,
          inlineId: inlineId,
          type: _nodeTypeFromLabel(normalized.nodeType),
        );
        break;
      case 'markdownInlineChunk':
        final blockId = normalized.blockId;
        final inlineId = normalized.inlineId;
        final value = normalized.value;
        if (blockId == null || inlineId == null || value == null) {
          throw StateError(
            'markdownInlineChunk missing blockId, inlineId, or value',
          );
        }
        _rendererState.eventBuilder.appendInline(
          blockId: blockId,
          inlineId: inlineId,
          content: value,
        );
        break;
      case 'completed':
        _rendererState.eventBuilder.complete();
        _streamDone = true;
        _rendererState.streamParsingCompletedSuccessfully = true;
        _notifyStreamDone();
        break;
      case 'savepoint':
        final id = normalized.id;
        if (id == null) {
          throw StateError('savepoint missing id');
        }
        _rendererState.eventBuilder.savepoint(id);
        break;
      case 'rollback':
        final id = normalized.id;
        if (id == null) {
          throw StateError('rollback missing id');
        }
        _rendererState.eventBuilder.rollback(id);
        break;
      default:
        throw StateError('Unknown markdown event type ${normalized.type}');
    }
    if (traceEvent) {
      _logRendererTrace(
        'apply.end rendererId=$_rendererId '
        'stream=${_streamTraceId(widget.contentStream)} '
        'count=$_streamEventCount ${normalized.traceSummary()} '
        'afterChars=${_rendererState.collectedContent.length} '
        'afterNodes=${_rendererState.renderNodes.length} done=$_streamDone',
      );
    }
  }

  void _scheduleNodeFadeIn(List<String> nodeKeys) {
    final unscheduledKeys = <String>[
      for (final key in nodeKeys)
        if (_scheduledVisibleNodeKeys.add(key)) key,
    ];
    if (unscheduledKeys.isEmpty) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      setState(() {
        for (final key in unscheduledKeys) {
          if (_rendererState.nodeAnimationStates.containsKey(key)) {
            _rendererState.nodeAnimationStates[key] = true;
          }
          _scheduledVisibleNodeKeys.remove(key);
        }
      });
    });
  }

  @override
  void dispose() {
    _renderTimer?.cancel();
    _subscription?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final content = _MarkdownNodeColumn(
      nodes: _rendererState.renderNodes,
      rendererId: _rendererId,
      textColor: widget.textColor,
      backgroundColor: widget.backgroundColor,
      nodeGrouper: widget.nodeGrouper,
      xmlNodeStreams: _rendererState.xmlNodeStreams,
      xmlMarkdownEventStreams: _rendererState.xmlMarkdownEventStreams,
      nodeAnimationStates: _rendererState.nodeAnimationStates,
      onLinkClick: widget.onLinkClick,
      showThinkingProcess: widget.showThinkingProcess,
      initialThinkingExpanded: widget.initialThinkingExpanded,
      allowExpandedThinkingFullHeight: widget.allowExpandedThinkingFullHeight,
      splitMarkdownContent: widget.splitMarkdownContent,
    );
    if (widget.isStreaming || !widget.selectionRoot) {
      return content;
    }
    return SelectionArea(child: content);
  }
}

/// Returns whether a lifecycle transition is useful enough to print.
bool _shouldLogMarkdownLifecycle(Stream<Object>? stream, String reason) {
  return stream != null || reason != 'init';
}

/// Returns whether a normalized Markdown event should print detailed state.
bool _shouldLogMarkdownEventDetail(
  _NormalizedMarkdownEvent event,
  int eventCount,
) {
  switch (event.type) {
    case 'reset':
    case 'completed':
    case 'savepoint':
    case 'rollback':
      return true;
    default:
      return eventCount > 0 && (eventCount <= 5 || eventCount % 256 == 0);
  }
}

const double _markdownParagraphBreakHeight = 4;
const double _markdownLineBlockBottomPadding = 3;
const double _markdownCanvasLineHeightMultiplier = 1.3;
final _StaticMarkdownNodeCache _staticMarkdownNodeCache =
    _StaticMarkdownNodeCache(maxEntries: 48);

class _StaticMarkdownNodeCache {
  _StaticMarkdownNodeCache({required this.maxEntries});

  final int maxEntries;
  final LinkedHashMap<String, List<MarkdownNodeStable>> _items =
      LinkedHashMap<String, List<MarkdownNodeStable>>();

  List<MarkdownNodeStable>? get(String content) {
    final value = _items.remove(content);
    if (value == null) {
      return null;
    }
    _items[content] = value;
    return value;
  }

  void put(String content, List<MarkdownNodeStable> nodes) {
    _items.remove(content);
    _items[content] = List<MarkdownNodeStable>.unmodifiable(nodes);
    while (_items.length > maxEntries) {
      _items.remove(_items.keys.first);
    }
  }
}

class _NormalizedMarkdownEvent {
  const _NormalizedMarkdownEvent({
    required this.type,
    required this.id,
    required this.value,
    required this.blockId,
    required this.inlineId,
    required this.parentBlockId,
    required this.nodeType,
    required this.headerLevel,
  });

  factory _NormalizedMarkdownEvent.from(Object event) {
    if (event is core_proxy.MarkdownStreamEvent) {
      return _NormalizedMarkdownEvent(
        type: event.eventType,
        id: event.id,
        value: event.value,
        blockId: event.blockId,
        inlineId: event.inlineId,
        parentBlockId: event.parentBlockId,
        nodeType: event.nodeType,
        headerLevel: event.headerLevel,
      );
    }
    if (event is _LocalMarkdownStreamEvent) {
      return _NormalizedMarkdownEvent(
        type: event.type,
        id: event.id,
        value: event.value,
        blockId: event.blockId,
        inlineId: event.inlineId,
        parentBlockId: event.parentBlockId,
        nodeType: event.nodeType,
        headerLevel: event.headerLevel,
      );
    }
    throw StateError('Unsupported markdown event ${event.runtimeType}');
  }

  _LocalMarkdownStreamEvent asLocalEvent() {
    return _LocalMarkdownStreamEvent(
      type: type,
      id: id,
      value: value,
      blockId: blockId,
      inlineId: inlineId,
      nodeType: nodeType,
      headerLevel: headerLevel,
    );
  }

  /// Returns a compact protocol-level description for render diagnostics.
  String traceSummary() {
    return 'eventType=$type valueLen=${value?.length ?? 0} '
        'id=${id ?? 'none'} blockId=${blockId ?? 'none'} '
        'inlineId=${inlineId ?? 'none'} '
        'parentBlockId=${parentBlockId ?? 'none'} '
        'nodeType=${nodeType ?? 'none'} headerLevel=${headerLevel ?? 'none'}';
  }

  final String type;
  final String? id;
  final String? value;
  final int? blockId;
  final int? inlineId;
  final int? parentBlockId;
  final String? nodeType;
  final int? headerLevel;
}

class _LocalMarkdownStreamEvent {
  const _LocalMarkdownStreamEvent({
    required this.type,
    required this.id,
    required this.value,
    required this.blockId,
    required this.inlineId,
    required this.nodeType,
    required this.headerLevel,
  });

  final String type;
  final String? id;
  final String? value;
  final int? blockId;
  final int? inlineId;
  final String? nodeType;
  final int? headerLevel;
  int? get parentBlockId => null;
}

MarkdownNodeType _nodeTypeFromLabel(String? label) {
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
    _ => throw StateError('Unknown markdown node type $label'),
  };
}

class _MarkdownNodeColumn extends StatefulWidget {
  const _MarkdownNodeColumn({
    required this.nodes,
    required this.rendererId,
    required this.textColor,
    required this.backgroundColor,
    required this.nodeGrouper,
    required this.xmlNodeStreams,
    required this.xmlMarkdownEventStreams,
    required this.showThinkingProcess,
    required this.initialThinkingExpanded,
    required this.allowExpandedThinkingFullHeight,
    required this.splitMarkdownContent,
    this.nodeAnimationStates,
    this.onLinkClick,
  });

  final List<MarkdownNodeStable> nodes;
  final String rendererId;
  final Color textColor;
  final Color backgroundColor;
  final MarkdownNodeGrouper nodeGrouper;
  final Map<int, Stream<String>> xmlNodeStreams;
  final Map<int, Stream<Object>> xmlMarkdownEventStreams;
  final bool showThinkingProcess;
  final bool initialThinkingExpanded;
  final bool allowExpandedThinkingFullHeight;
  final MarkdownContentSplitter splitMarkdownContent;
  final Map<String, bool>? nodeAnimationStates;
  final void Function(String url)? onLinkClick;

  @override
  State<_MarkdownNodeColumn> createState() => _MarkdownNodeColumnState();
}

class _MarkdownNodeColumnState extends State<_MarkdownNodeColumn> {
  final Map<String, _CachedSingleMarkdownNode> _singleNodeCache =
      <String, _CachedSingleMarkdownNode>{};
  final Map<String, _CachedMarkdownGroup> _groupCache =
      <String, _CachedMarkdownGroup>{};

  @override
  void didUpdateWidget(covariant _MarkdownNodeColumn oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.rendererId != widget.rendererId ||
        oldWidget.textColor != widget.textColor ||
        oldWidget.backgroundColor != widget.backgroundColor ||
        oldWidget.nodeGrouper != widget.nodeGrouper ||
        oldWidget.onLinkClick != widget.onLinkClick ||
        oldWidget.showThinkingProcess != widget.showThinkingProcess ||
        oldWidget.initialThinkingExpanded != widget.initialThinkingExpanded ||
        oldWidget.allowExpandedThinkingFullHeight !=
            widget.allowExpandedThinkingFullHeight) {
      _singleNodeCache.clear();
      _groupCache.clear();
    }
  }

  @override
  Widget build(BuildContext context) {
    final groupedItems = widget.nodeGrouper.group(
      widget.nodes,
      widget.rendererId,
    );
    final lastRenderableIndex = _lastRenderableNodeIndex(widget.nodes);
    final liveSingleKeys = <String>{};
    final liveGroupKeys = <String>{};

    bool isVisibleAt(int index) {
      final states = widget.nodeAnimationStates;
      if (states == null) {
        return true;
      }
      final key = _nodeKeyForIndex(widget.rendererId, index);
      return states.containsKey(key) ? states[key] == true : true;
    }

    Widget renderXmlContent({
      required String xmlContent,
      required bool isStreaming,
      required Color textColor,
      Stream<String>? xmlStream,
      Stream<Object>? xmlMarkdownEventStream,
      String? renderInstanceKey,
    }) {
      return CustomXmlRenderer(
        key: renderInstanceKey == null
            ? null
            : ValueKey<String>(renderInstanceKey),
        xmlContent: xmlContent,
        isStreaming: isStreaming,
        textColor: textColor,
        xmlStream: xmlStream,
        xmlMarkdownEventStream: xmlMarkdownEventStream,
        showThinkingProcess: widget.showThinkingProcess,
        initialThinkingExpanded: widget.initialThinkingExpanded,
        allowExpandedThinkingFullHeight: widget.allowExpandedThinkingFullHeight,
        splitMarkdownContent: widget.splitMarkdownContent,
      );
    }

    Widget renderNodeAt(int index) {
      final node = widget.nodes[index];
      if (node.type == MarkdownNodeType.xmlBlock) {
        return renderXmlContent(
          xmlContent: node.content,
          isStreaming: node.isStreaming,
          textColor: widget.textColor,
          xmlStream: widget.xmlNodeStreams[index],
          xmlMarkdownEventStream: widget.xmlMarkdownEventStreams[index],
        );
      }
      return CanvasMarkdownNodeRenderer(
        key: ValueKey<String>(_nodeKeyForIndex(widget.rendererId, index)),
        nodeKey: _nodeKeyForIndex(widget.rendererId, index),
        node: node,
        textColor: widget.textColor,
        backgroundColor: widget.backgroundColor,
        isLastNode: index == lastRenderableIndex,
        onLinkClick: widget.onLinkClick,
        splitMarkdownContent: widget.splitMarkdownContent,
      );
    }

    Widget renderAnimatedNodeAt(int index) {
      final node = widget.nodes[index];
      final cacheKey = _nodeKeyForIndex(widget.rendererId, index);
      liveSingleKeys.add(cacheKey);
      final isVisible = isVisibleAt(index);
      final isLastNode = index == lastRenderableIndex;
      final xmlStream = widget.xmlNodeStreams[index];
      final xmlMarkdownEventStream = widget.xmlMarkdownEventStreams[index];
      final cached = _singleNodeCache[cacheKey];
      if (cached != null &&
          cached.node == node &&
          cached.isVisible == isVisible &&
          cached.isLastNode == isLastNode &&
          identical(cached.xmlStream, xmlStream) &&
          identical(cached.xmlMarkdownEventStream, xmlMarkdownEventStream)) {
        return cached.widget;
      }
      late final Widget rendered;
      if (_canTypewriteNode(node.type)) {
        rendered = renderNodeAt(index);
      } else {
        rendered = _AnimatedMarkdownNode(
          isVisible: isVisible,
          child: renderNodeAt(index),
        );
      }
      _singleNodeCache[cacheKey] = _CachedSingleMarkdownNode(
        node: node,
        isVisible: isVisible,
        isLastNode: isLastNode,
        xmlStream: xmlStream,
        xmlMarkdownEventStream: xmlMarkdownEventStream,
        widget: rendered,
      );
      return rendered;
    }

    Widget renderGroupItem(MarkdownGroupItem group) {
      final cacheKey = 'group-${widget.rendererId}-${group.stableKey}';
      liveGroupKeys.add(cacheKey);
      final isVisible = isVisibleAt(group.startIndex);
      final isLastNode = group.endIndexInclusive == lastRenderableIndex;
      final slice = <MarkdownNodeStable>[
        for (
          var index = group.startIndex;
          index <= group.endIndexInclusive;
          index++
        )
          widget.nodes[index],
      ];
      final xmlStreams = <Stream<String>?>[
        for (
          var index = group.startIndex;
          index <= group.endIndexInclusive;
          index++
        )
          widget.xmlNodeStreams[index],
      ];
      final xmlMarkdownEventStreams = <Stream<Object>?>[
        for (
          var index = group.startIndex;
          index <= group.endIndexInclusive;
          index++
        )
          widget.xmlMarkdownEventStreams[index],
      ];
      final cached = _groupCache[cacheKey];
      if (cached != null &&
          cached.group.startIndex == group.startIndex &&
          cached.group.endIndexInclusive == group.endIndexInclusive &&
          cached.group.stableKey == group.stableKey &&
          cached.isVisible == isVisible &&
          cached.isLastNode == isLastNode &&
          _markdownNodeListEquals(cached.nodes, slice) &&
          _streamListIdentical(cached.xmlStreams, xmlStreams) &&
          _streamListIdentical(
            cached.xmlMarkdownEventStreams,
            xmlMarkdownEventStreams,
          )) {
        return cached.widget;
      }
      final rendered = widget.nodeGrouper.renderGroup(
        group: group,
        nodes: widget.nodes,
        rendererId: widget.rendererId,
        isVisible: isVisible,
        isLastNode: isLastNode,
        textColor: widget.textColor,
        xmlRenderer: renderXmlContent,
        xmlStreamResolver: (index) => widget.xmlNodeStreams[index],
        xmlMarkdownEventStreamResolver: (index) =>
            widget.xmlMarkdownEventStreams[index],
        onLinkClick: widget.onLinkClick,
        fillMaxWidth: true,
        textStyle: Theme.of(context).textTheme.bodyMedium!,
      );
      _groupCache[cacheKey] = _CachedMarkdownGroup(
        group: group,
        nodes: List<MarkdownNodeStable>.unmodifiable(slice),
        xmlStreams: List<Stream<String>?>.unmodifiable(xmlStreams),
        xmlMarkdownEventStreams: List<Stream<Object>?>.unmodifiable(
          xmlMarkdownEventStreams,
        ),
        isVisible: isVisible,
        isLastNode: isLastNode,
        widget: rendered,
      );
      return rendered;
    }

    final children = <Widget>[
      for (final item in groupedItems)
        if (item is MarkdownSingleItem)
          renderAnimatedNodeAt(item.index)
        else if (item is MarkdownGroupItem)
          renderGroupItem(item),
    ];

    _singleNodeCache.removeWhere((key, value) => !liveSingleKeys.contains(key));
    _groupCache.removeWhere((key, value) => !liveGroupKeys.contains(key));

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: children,
    );
  }
}

class _CachedSingleMarkdownNode {
  const _CachedSingleMarkdownNode({
    required this.node,
    required this.isVisible,
    required this.isLastNode,
    required this.xmlStream,
    required this.xmlMarkdownEventStream,
    required this.widget,
  });

  final MarkdownNodeStable node;
  final bool isVisible;
  final bool isLastNode;
  final Stream<String>? xmlStream;
  final Stream<Object>? xmlMarkdownEventStream;
  final Widget widget;
}

class _CachedMarkdownGroup {
  const _CachedMarkdownGroup({
    required this.group,
    required this.nodes,
    required this.xmlStreams,
    required this.xmlMarkdownEventStreams,
    required this.isVisible,
    required this.isLastNode,
    required this.widget,
  });

  final MarkdownGroupItem group;
  final List<MarkdownNodeStable> nodes;
  final List<Stream<String>?> xmlStreams;
  final List<Stream<Object>?> xmlMarkdownEventStreams;
  final bool isVisible;
  final bool isLastNode;
  final Widget widget;
}

bool _streamListIdentical<T>(List<Stream<T>?> left, List<Stream<T>?> right) {
  if (left.length != right.length) {
    return false;
  }
  for (var index = 0; index < left.length; index++) {
    if (!identical(left[index], right[index])) {
      return false;
    }
  }
  return true;
}

bool _markdownNodeListEquals(
  List<MarkdownNodeStable> left,
  List<MarkdownNodeStable> right,
) {
  if (identical(left, right)) {
    return true;
  }
  if (left.length != right.length) {
    return false;
  }
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) {
      return false;
    }
  }
  return true;
}

String _nodeKeyForIndex(String rendererId, int index) {
  if (rendererId.startsWith('static-')) {
    return 'static-node-$rendererId-$index';
  }
  return 'node-$rendererId-$index';
}

class _AnimatedMarkdownNode extends StatefulWidget {
  const _AnimatedMarkdownNode({required this.isVisible, required this.child});

  final bool isVisible;
  final Widget child;

  @override
  State<_AnimatedMarkdownNode> createState() => _AnimatedMarkdownNodeState();
}

class _AnimatedMarkdownNodeState extends State<_AnimatedMarkdownNode>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _opacity;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: _nodeFadeInDuration,
      value: widget.isVisible ? 1 : 0,
    );
    _opacity = CurvedAnimation(parent: _controller, curve: Curves.linear);
  }

  @override
  void didUpdateWidget(covariant _AnimatedMarkdownNode oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.isVisible == widget.isVisible) {
      return;
    }
    if (widget.isVisible) {
      _controller.forward();
    } else {
      _controller.reverse();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FadeTransition(
      opacity: _opacity,
      child: RepaintBoundary(child: widget.child),
    );
  }
}

/// Finds the final node that can own a visible streaming indicator.
int _lastRenderableNodeIndex(List<MarkdownNodeStable> nodes) {
  for (var index = nodes.length - 1; index >= 0; index--) {
    final node = nodes[index];
    if (node.content.trim().isNotEmpty || node.children.isNotEmpty) {
      return index;
    }
  }
  return -1;
}

bool _canTypewriteNode(MarkdownNodeType type) {
  return type == MarkdownNodeType.plainText ||
      type == MarkdownNodeType.header ||
      type == MarkdownNodeType.orderedList ||
      type == MarkdownNodeType.unorderedList;
}

int _typewriterTextLength(String text) {
  final lines = text.split('\n');
  final paragraphLines = <String>[];
  var inCode = false;
  var length = 0;
  var index = 0;

  void flushParagraph() {
    if (paragraphLines.isEmpty) {
      return;
    }
    length += paragraphLines.join('\n').length;
    paragraphLines.clear();
  }

  while (index < lines.length) {
    final line = lines[index];
    final trimmed = line.trimRight();
    if (trimmed.startsWith('```')) {
      if (!inCode) {
        flushParagraph();
      }
      inCode = !inCode;
    } else if (inCode) {
    } else if (_isBlockLatexStart(trimmed)) {
      flushParagraph();
      final start = trimmed.trimLeft();
      final singleLine = start.length > 2 && _isBlockLatexEnd(start, start);
      while (!singleLine && index + 1 < lines.length) {
        index++;
        final nextLine = lines[index].trimRight();
        if (_isBlockLatexEnd(start, nextLine.trimRight())) {
          break;
        }
      }
    } else if (_isTableStart(lines, index)) {
      flushParagraph();
      while (index < lines.length && lines[index].trim().contains('|')) {
        index++;
      }
      index--;
    } else if (trimmed.trimLeft().startsWith('>') ||
        isCompleteImageMarkdown(trimmed.trim()) ||
        _isHorizontalRule(trimmed) ||
        trimmed.isEmpty) {
      flushParagraph();
    } else if (_headingLevel(trimmed) > 0 ||
        _isBulletLine(trimmed) ||
        _isOrderedLine(trimmed)) {
      flushParagraph();
      length += _typewriterLineLength(trimmed);
    } else {
      paragraphLines.add(trimmed);
    }
    index++;
  }
  flushParagraph();
  return length;
}

int _typewriterLineLength(String text) {
  if (_headingLevel(text) > 0) {
    return _markdownHeaderText(text).length;
  }
  if (_isBulletLine(text)) {
    return text.substring(2).length;
  }
  if (_isOrderedLine(text)) {
    final match = RegExp(r'^(\d+)\.\s*').firstMatch(text);
    return match == null ? text.length : text.substring(match.end).length;
  }
  return text.length;
}
