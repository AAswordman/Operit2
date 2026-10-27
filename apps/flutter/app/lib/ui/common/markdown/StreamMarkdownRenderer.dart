// ignore_for_file: file_names

import 'dart:async';
import 'dart:collection';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import '../../../core/logging/ClientLogger.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../interactions/MessagePressShield.dart';
import 'EnhancedCodeBlock.dart';
import 'EnhancedTableBlock.dart';
import 'MarkdownNodeGrouper.dart';
import 'MarkdownTextReveal.dart';
import 'MarkdownBlockQuote.dart';
import 'MarkdownFootnotes.dart';
import 'MarkdownImageRenderer.dart';
import 'MarkdownInlineSpannable.dart';
import 'MarkdownLatexBlock.dart';
import 'StreamMarkdownRendererState.dart';
import '../../features/chat/components/part/CustomXmlRenderer.dart';
import 'Utf16Text.dart';

part 'CanvasMarkdownNodeRenderer.dart';

typedef MarkdownContentSplitter =
    Future<List<core_proxy.MarkdownStreamEvent>> Function(String content);

/// Reports whether a Markdown surface is preparing its first content layout.
class MarkdownLayoutNotification extends Notification {
  /// Identifies one surface so containing rows can track nested preparation.
  const MarkdownLayoutNotification({
    required this.source,
    required this.isPending,
  });

  final Object source;
  final bool isPending;
}

class StreamMarkdownRenderer extends StatefulWidget {
  const StreamMarkdownRenderer({
    super.key,
    required this.content,
    required this.isStreaming,
    required this.textColor,
    required this.backgroundColor,
    this.nodeGrouper = const NoopMarkdownNodeGrouper(),
    this.mergeRender = const NoopMarkdownNodeMergeRender(),
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
    this.onStreamError,
    required this.splitMarkdownContent,
  });

  final String content;
  final bool isStreaming;
  final Color textColor;
  final Color backgroundColor;
  final MarkdownNodeGrouper nodeGrouper;
  final MarkdownNodeMergeRender mergeRender;
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
  final VoidCallback? onStreamError;
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

  @override
  void initState() {
    super.initState();
    _rendererId = _computeRendererId();
    _rendererState = widget.state ?? StreamMarkdownRendererState();
    _startCurrentContent();
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
      _rendererId = nextRendererId;
      _startCurrentContent();
    } else if (oldWidget.onContentReady != widget.onContentReady &&
        widget.contentStream == null &&
        _rendererState.streamParsingCompletedSuccessfully) {
      _notifyContentReady(_startGeneration);
    }
  }

  /// Computes the stable identity used to preserve one renderer state.
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
  void _startCurrentContent() {
    final generation = ++_startGeneration;
    _subscription?.cancel();
    _renderTimer?.cancel();
    _renderTimer = null;
    _scheduledVisibleNodeKeys.clear();
    _streamDoneNotified = false;

    final stream = widget.contentStream;
    _streamDone = stream == null;
    if (stream == null) {
      final cachedNodes = _staticMarkdownNodeCache.get(widget.content);
      if (cachedNodes != null) {
        _rendererState.reset();
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
      _loadStaticContent(sanitizeUtf16(widget.content), generation).catchError((
        Object error,
        StackTrace stackTrace,
      ) {
        if (mounted && generation == _startGeneration) {
          _handleMarkdownError(error, stackTrace);
        }
      });
      return;
    }

    _rendererState.reset();
    _synchronizeRenderNodes(isStreaming: true);
    _subscribe(stream);
  }

  Future<void> _loadStaticContent(String content, int generation) async {
    final events = await widget.splitMarkdownContent(content);
    if (!mounted || generation != _startGeneration) {
      return;
    }
    // Keep the live tree visible until the persisted source has been parsed.
    _rendererState.reset();
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

  /// Subscribes to Markdown events and schedules incremental node syncs.
  void _subscribe(Stream<Object> stream) {
    _subscription = stream
        .map((event) {
          // Stream transformations deliver parser exceptions to this subscription's onError.
          _applyMarkdownEvent(event);
          return event;
        })
        .listen(
          (_) {
            if (!_streamDone) {
              _renderTimer ??= Timer(_streamRenderInterval, _flushRenderNodes);
            }
          },
          onDone: () {
            if (mounted) {
              setState(_completeLiveStreamRender);
            } else {
              _completeLiveStreamRender();
            }
            _notifyStreamDone();
          },
          onError: _handleMarkdownError,
        );
  }

  /// Terminates only the failed surface and retains its parsed nodes for inspection.
  void _handleMarkdownError(Object error, StackTrace stackTrace) {
    // Report failure before waiting for transport cancellation to finish.
    unawaited(_subscription?.cancel());
    ClientLogger.e(
      'Markdown parsing failed for $_rendererId',
      tag: 'StreamMarkdownRenderer',
      error: error,
      stackTrace: stackTrace,
    );
    _renderTimer?.cancel();
    _renderTimer = null;
    _streamDone = true;
    _rendererState.eventBuilder.complete();
    _rendererState.streamParsingCompletedSuccessfully = false;
    if (mounted) {
      setState(() => _synchronizeRenderNodes(isStreaming: false));
      widget.onStreamError?.call();
    }
  }

  /// Completes live parsing and synchronizes the final render nodes.
  void _completeLiveStreamRender() {
    if (!_streamDone) {
      _rendererState.eventBuilder.complete();
    }
    _streamDone = true;
    _rendererState.streamParsingCompletedSuccessfully = true;
    _renderTimer?.cancel();
    _renderTimer = null;
    _synchronizeRenderNodes(isStreaming: false);
  }

  /// Reports that the live Markdown stream has reached its terminal event.
  void _notifyStreamDone() {
    if (_streamDoneNotified) {
      return;
    }
    _streamDoneNotified = true;
    widget.onStreamDone?.call();
  }

  /// Synchronizes mutable Markdown nodes into visible render nodes.
  void _flushRenderNodes() {
    _renderTimer = null;
    if (!mounted) {
      return;
    }
    setState(() => _synchronizeRenderNodes(isStreaming: !_streamDone));
  }

  /// Rebuilds stable render nodes while preserving existing animation keys.
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

  /// Applies semantic events; only lifecycle effects belong to the widget.
  void _applyMarkdownEvent(Object event) {
    switch (_rendererState.applyEvent(event)) {
      case 'reset':
        _scheduledVisibleNodeKeys.clear();
        _streamDone = false;
        break;
      case 'completed':
        if (mounted) {
          setState(_completeLiveStreamRender);
        } else {
          _completeLiveStreamRender();
        }
        _notifyStreamDone();
        break;
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

  /// Releases the row reservation before this surface leaves its ancestors.
  @override
  void deactivate() {
    MarkdownLayoutNotification(
      source: this,
      isPending: false,
    ).dispatch(context);
    super.deactivate();
  }

  @override
  void dispose() {
    _renderTimer?.cancel();
    _subscription?.cancel();
    super.dispose();
  }

  /// Builds the Markdown node column inside a stable selection boundary.
  @override
  Widget build(BuildContext context) {
    MarkdownLayoutNotification(
      source: this,
      isPending:
          _rendererState.renderNodes.isEmpty &&
          !_rendererState.streamParsingCompletedSuccessfully,
    ).dispatch(context);
    final content = _MarkdownNodeColumn(
      nodes: _rendererState.renderNodes,
      rendererId: _rendererId,
      textColor: widget.textColor,
      backgroundColor: widget.backgroundColor,
      nodeGrouper: widget.nodeGrouper,
      mergeRender: widget.mergeRender,
      xmlNodeStreams: _rendererState.xmlNodeStreams,
      xmlMarkdownEventStreams: _rendererState.xmlMarkdownEventStreams,
      nodeAnimationStates: _rendererState.nodeAnimationStates,
      onLinkClick: widget.onLinkClick,
      showThinkingProcess: widget.showThinkingProcess,
      initialThinkingExpanded: widget.initialThinkingExpanded,
      allowExpandedThinkingFullHeight: widget.allowExpandedThinkingFullHeight,
      splitMarkdownContent: widget.splitMarkdownContent,
    );
    final inheritedFootnotes = MarkdownFootnoteScope.maybeOf(context);
    final scopedContent = MarkdownFootnoteScope(
      definitions: {
        ...?inheritedFootnotes?.definitions,
        for (final node in _rendererState.renderNodes)
          if (node.type == MarkdownNodeType.footnoteDefinition)
            node.sourcePrefix: node,
      },
      definitionBuilder: (definition) => CanvasMarkdownNodeRenderer(
        nodeKey: 'footnote-${definition.sourcePrefix}',
        node: definition,
        textColor: widget.textColor,
        backgroundColor: widget.backgroundColor,
        isLastNode: true,
        onLinkClick: widget.onLinkClick,
        splitMarkdownContent: widget.splitMarkdownContent,
      ),
      child: content,
    );
    if (!widget.selectionRoot) {
      return scopedContent;
    }
    return SelectionArea(child: scopedContent);
  }
}

const double _markdownParagraphBreakHeight = 4;
const double _markdownParagraphBottomPadding = 6;
const double _markdownListItemBottomPadding = 2;
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

class _MarkdownNodeColumn extends StatefulWidget {
  const _MarkdownNodeColumn({
    required this.nodes,
    required this.rendererId,
    required this.textColor,
    required this.backgroundColor,
    required this.nodeGrouper,
    required this.mergeRender,
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
  final MarkdownNodeMergeRender mergeRender;
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
        oldWidget.mergeRender != widget.mergeRender ||
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
        nestedMarkdownEventStream: widget.xmlMarkdownEventStreams[index],
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

    /// Renders one merge match using the start node's visibility.
    ///
    /// A later matching result must not replay fade-in: the call node already
    /// revealed this slot, so the merged row inherits that same visibility.
    Widget renderMergeMatch(
      MarkdownMergeMatch match,
      String renderInstanceKeyPrefix,
    ) {
      final mergeKey =
          'merge-${widget.rendererId}-$renderInstanceKeyPrefix-${match.stableKey}';
      return KeyedSubtree(
        key: ValueKey<String>(mergeKey),
        child: _AnimatedMarkdownNode(
          isVisible: isVisibleAt(match.startIndex),
          child: widget.mergeRender.renderMerge(
            match: match,
            nodes: widget.nodes,
            rendererId: widget.rendererId,
            textColor: widget.textColor,
            xmlRenderer: renderXmlContent,
            xmlStreamResolver: (index) => widget.xmlNodeStreams[index],
            xmlMarkdownEventStreamResolver: (index) =>
                widget.xmlMarkdownEventStreams[index],
            renderInstanceKey: mergeKey,
          ),
        ),
      );
    }

    /// Renders a source range after applying the configured merge strategy.
    List<Widget> renderMergedRange({
      required int startIndex,
      required int endIndexInclusive,
      required String renderInstanceKeyPrefix,
      required MarkdownNodePredicate shouldRenderNode,
    }) {
      final rendered = <Widget>[];
      var index = startIndex;
      while (index <= endIndexInclusive) {
        final match = widget.mergeRender.match(
          nodes: widget.nodes,
          startIndex: index,
          endIndexInclusive: endIndexInclusive,
        );
        if (match != null) {
          _validateMarkdownMergeMatch(match, index, endIndexInclusive);
          rendered.add(renderMergeMatch(match, renderInstanceKeyPrefix));
          index = match.endIndexInclusive + 1;
          continue;
        }
        if (shouldRenderNode(widget.nodes[index])) {
          rendered.add(renderAnimatedNodeAt(index));
        }
        index += 1;
      }
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
      final nodeVisibilities = <bool>[
        for (
          var index = group.startIndex;
          index <= group.endIndexInclusive;
          index++
        )
          isVisibleAt(index),
      ];
      final cached = _groupCache[cacheKey];
      if (cached != null &&
          cached.group.startIndex == group.startIndex &&
          cached.group.endIndexInclusive == group.endIndexInclusive &&
          cached.group.stableKey == group.stableKey &&
          cached.isVisible == isVisible &&
          cached.isLastNode == isLastNode &&
          _markdownNodeListEquals(cached.nodes, slice) &&
          _boolListEquals(cached.nodeVisibilities, nodeVisibilities) &&
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
        mergeRender: renderMergedRange,
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
        nodeVisibilities: List<bool>.unmodifiable(nodeVisibilities),
        widget: rendered,
      );
      return rendered;
    }

    final children = <Widget>[];
    var itemOffset = 0;
    while (itemOffset < groupedItems.length) {
      final item = groupedItems[itemOffset];
      if (item is MarkdownGroupItem) {
        children.add(renderGroupItem(item));
        itemOffset += 1;
        continue;
      }
      final single = item as MarkdownSingleItem;
      final rangeStart = single.index;
      var rangeEnd = rangeStart;
      itemOffset += 1;
      while (itemOffset < groupedItems.length) {
        final next = groupedItems[itemOffset];
        if (next is! MarkdownSingleItem || next.index != rangeEnd + 1) {
          break;
        }
        rangeEnd = next.index;
        itemOffset += 1;
      }
      children.addAll(
        renderMergedRange(
          startIndex: rangeStart,
          endIndexInclusive: rangeEnd,
          renderInstanceKeyPrefix: 'root-$rangeStart',
          shouldRenderNode: _renderEveryMarkdownNode,
        ),
      );
    }

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
    required this.nodeVisibilities,
    required this.widget,
  });

  final MarkdownGroupItem group;
  final List<MarkdownNodeStable> nodes;
  final List<Stream<String>?> xmlStreams;
  final List<Stream<Object>?> xmlMarkdownEventStreams;
  final bool isVisible;
  final bool isLastNode;
  final List<bool> nodeVisibilities;
  final Widget widget;
}

/// Compares cached per-node visibility snapshots for a rendered group.
bool _boolListEquals(List<bool> left, List<bool> right) {
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

/// Includes every node when rendering a non-grouped Markdown range.
bool _renderEveryMarkdownNode(MarkdownNodeStable node) {
  return true;
}

/// Validates that a merge strategy owns a forward range inside its request.
void _validateMarkdownMergeMatch(
  MarkdownMergeMatch match,
  int requestedStartIndex,
  int requestedEndIndexInclusive,
) {
  if (match.startIndex != requestedStartIndex ||
      match.endIndexInclusive < requestedStartIndex ||
      match.endIndexInclusive > requestedEndIndexInclusive) {
    throw StateError(
      'Invalid Markdown merge range ${match.startIndex}..'
      '${match.endIndexInclusive} for $requestedStartIndex..'
      '$requestedEndIndexInclusive.',
    );
  }
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
    if (_hasMarkdownNodeContent(node)) {
      return index;
    }
  }
  return -1;
}

/// Core may retain separator newlines as inline nodes after a visible block.
bool _hasMarkdownNodeContent(MarkdownNodeStable node) =>
    node.content.trim().isNotEmpty ||
    node.children.any(_hasMarkdownNodeContent);

bool _canTypewriteNode(MarkdownNodeType type) {
  return type == MarkdownNodeType.plainText ||
      type == MarkdownNodeType.header ||
      type == MarkdownNodeType.orderedList ||
      type == MarkdownNodeType.unorderedList ||
      type == MarkdownNodeType.footnoteDefinition;
}

int _markdownVisibleLength(String text, List<MarkdownNodeStable> children) {
  if (children.isEmpty) return text.characters.length;
  var length = 0;
  for (final child in children) {
    length +=
        (child.type == MarkdownNodeType.inlineLatex ||
            child.type == MarkdownNodeType.footnoteReference)
        ? 1
        : _markdownVisibleLength(child.content, child.children);
  }
  return length;
}
