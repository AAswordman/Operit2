// ignore_for_file: file_names

import 'dart:async';

import 'MarkdownNodeGrouper.dart';

class StreamMarkdownRendererState {
  final List<MutableMarkdownNode> nodes = <MutableMarkdownNode>[];
  final List<MarkdownNodeStable> renderNodes = <MarkdownNodeStable>[];
  final Map<int, Stream<String>> xmlNodeStreams = <int, Stream<String>>{};
  final Map<int, Stream<Object>> xmlMarkdownEventStreams =
      <int, Stream<Object>>{};
  late final MarkdownEventNodeBuilder eventBuilder = MarkdownEventNodeBuilder(
    nodes: nodes,
    xmlNodeStreams: xmlNodeStreams,
    xmlMarkdownEventStreams: xmlMarkdownEventStreams,
  );
  final Map<String, bool> nodeAnimationStates = <String, bool>{};
  final Map<int, (int, MarkdownNodeStable)> conversionCache =
      <int, (int, MarkdownNodeStable)>{};
  final StringBuffer collectedContent = StringBuffer();
  bool streamParsingCompletedSuccessfully = false;
  String rendererId = '';

  void updateRendererId(String id) {
    rendererId = id;
  }

  void reset() {
    eventBuilder.reset();
    renderNodes.clear();
    nodeAnimationStates.clear();
    conversionCache.clear();
    xmlNodeStreams.clear();
    xmlMarkdownEventStreams.clear();
    collectedContent.clear();
    streamParsingCompletedSuccessfully = false;
  }
}

class MutableMarkdownNode {
  MutableMarkdownNode({
    required this.type,
    required this.stableKey,
    this.headerLevel,
  });

  MarkdownNodeType type;
  final String stableKey;
  final int? headerLevel;
  final StringBuffer content = StringBuffer();
  final List<MutableMarkdownNode> children = <MutableMarkdownNode>[];
  bool isStreaming = false;

  MutableMarkdownNode copy() {
    final node = MutableMarkdownNode(
      type: type,
      stableKey: stableKey,
      headerLevel: headerLevel,
    );
    node.content.write(content.toString());
    node.children.addAll(<MutableMarkdownNode>[
      for (final child in children) child.copy(),
    ]);
    node.isStreaming = isStreaming;
    return node;
  }

  MarkdownNodeStable toStable() {
    return MarkdownNodeStable(
      type: type,
      content: content.toString(),
      isStreaming: isStreaming,
      stableKey: stableKey,
      headerLevel: headerLevel,
      children: <MarkdownNodeStable>[
        for (final child in children) child.toStable(),
      ],
    );
  }
}

class MarkdownEventNodeBuilder {
  MarkdownEventNodeBuilder({
    required this.nodes,
    required this.xmlNodeStreams,
    required this.xmlMarkdownEventStreams,
  });

  final List<MutableMarkdownNode> nodes;
  final Map<int, Stream<String>> xmlNodeStreams;
  final Map<int, Stream<Object>> xmlMarkdownEventStreams;
  final Map<int, MutableMarkdownNode> _blocks = <int, MutableMarkdownNode>{};
  final Map<String, MutableMarkdownNode> _inlines =
      <String, MutableMarkdownNode>{};
  final Map<String, _PendingInlineStart> _pendingInlines =
      <String, _PendingInlineStart>{};
  final Set<int> _htmlBreakBlocks = <int>{};
  final Set<int> _mergedBlocks = <int>{};
  final Set<int> _latexBlocks = <int>{};
  final Set<String> _latexInlines = <String>{};
  final Map<int, _PendingLineBreakState> _pendingInlineBreaks =
      <int, _PendingLineBreakState>{};
  final Map<int, int> _pendingBlockBreaks = <int, int>{};
  final Map<int, _ReplayStringStream> _xmlControllers =
      <int, _ReplayStringStream>{};
  final Map<int, _ReplayObjectStream> _xmlMarkdownControllers =
      <int, _ReplayObjectStream>{};
  final Map<String, _MarkdownEventNodeBuilderSnapshot> _savepoints =
      <String, _MarkdownEventNodeBuilderSnapshot>{};
  int _pendingHtmlBreakCount = 0;

  void reset() {
    for (final controller in _xmlControllers.values) {
      controller.close();
    }
    for (final controller in _xmlMarkdownControllers.values) {
      controller.close();
    }
    nodes.clear();
    _blocks.clear();
    _inlines.clear();
    _pendingInlines.clear();
    _htmlBreakBlocks.clear();
    _mergedBlocks.clear();
    _latexBlocks.clear();
    _latexInlines.clear();
    _pendingInlineBreaks.clear();
    _pendingBlockBreaks.clear();
    _xmlControllers.clear();
    _xmlMarkdownControllers.clear();
    xmlNodeStreams.clear();
    xmlMarkdownEventStreams.clear();
    _savepoints.clear();
    _pendingHtmlBreakCount = 0;
  }

  void savepoint(String id) {
    _savepoints[id] = _MarkdownEventNodeBuilderSnapshot.capture(this);
  }

  void rollback(String id) {
    final snapshot = _savepoints[id];
    if (snapshot == null) {
      return;
    }
    snapshot.restore(this);
  }

  void startBlock({
    required int blockId,
    required MarkdownNodeType type,
    required int? headerLevel,
  }) {
    _finalizeOpenInlineNodes();
    _finalizeOpenLatexBlocks(exceptBlockId: blockId);

    if (type == MarkdownNodeType.htmlBreak) {
      if (_canMergeWithHtmlBreak(nodes.isEmpty ? null : nodes.last)) {
        _pendingHtmlBreakCount = (_pendingHtmlBreakCount + 1).clamp(
          0,
          _maxConsecutiveRenderedNewlines,
        );
      } else {
        final node = _htmlBreakNode(blockId);
        nodes.add(node);
        _blocks[blockId] = node;
      }
      _htmlBreakBlocks.add(blockId);
      return;
    }

    if (type == MarkdownNodeType.horizontalRule &&
        _pendingHtmlBreakCount > 0) {
      _appendHtmlBreakNodes(_pendingHtmlBreakCount);
      _pendingHtmlBreakCount = 0;
    }

    final nodeType = type == MarkdownNodeType.blockLatex
        ? MarkdownNodeType.plainText
        : type;
    final mergeWithPrevious = _pendingHtmlBreakCount > 0 &&
        nodeType == MarkdownNodeType.plainText &&
        _canMergeWithHtmlBreak(nodes.isEmpty ? null : nodes.last);
    if (_pendingHtmlBreakCount > 0 && !mergeWithPrevious) {
      _appendHtmlBreakNodes(_pendingHtmlBreakCount);
      _pendingHtmlBreakCount = 0;
    }

    if (mergeWithPrevious) {
      final node = nodes.last;
      _blocks[blockId] = node;
      _mergedBlocks.add(blockId);
      _pendingBlockBreaks[blockId] = _pendingHtmlBreakCount;
      if (type == MarkdownNodeType.blockLatex) {
        _latexBlocks.add(blockId);
      }
      _pendingHtmlBreakCount = 0;
      return;
    }

    final node = MutableMarkdownNode(
      type: nodeType,
      stableKey: 'block-$blockId',
      headerLevel: headerLevel,
    );
    nodes.add(node);
    _blocks[blockId] = node;
    if (type == MarkdownNodeType.blockLatex) {
      _latexBlocks.add(blockId);
    }
    if (type == MarkdownNodeType.xmlBlock) {
      final controller = _ReplayStringStream();
      final markdownController = _ReplayObjectStream();
      _xmlControllers[blockId] = controller;
      _xmlMarkdownControllers[blockId] = markdownController;
      xmlNodeStreams[nodes.length - 1] = controller.stream;
      xmlMarkdownEventStreams[nodes.length - 1] = markdownController.stream;
    }
    _pendingHtmlBreakCount = 0;
  }

  void appendBlock({
    required int blockId,
    required String content,
  }) {
    if (_htmlBreakBlocks.contains(blockId)) {
      return;
    }
    final node = _blocks[blockId];
    if (node == null) {
      throw StateError('Missing markdown block $blockId');
    }
    node.content.write(content);
    _xmlControllers[blockId]?.add(content);
  }

  void appendXmlMarkdownEvent({
    required int parentBlockId,
    required Object event,
  }) {
    final controller = _xmlMarkdownControllers[parentBlockId];
    if (controller == null) {
      throw StateError('Missing XML markdown stream for block $parentBlockId');
    }
    controller.add(event);
  }

  void startInline({
    required int blockId,
    required int inlineId,
    required MarkdownNodeType type,
  }) {
    if (_htmlBreakBlocks.contains(blockId)) {
      return;
    }
    final block = _blocks[blockId];
    if (block == null) {
      throw StateError('Missing markdown block $blockId for inline $inlineId');
    }
    _finalizeInlineForBlock(blockId);
    final key = _inlineKey(blockId, inlineId);
    final nodeType = type == MarkdownNodeType.inlineLatex
        ? MarkdownNodeType.plainText
        : type;
    _pendingInlines[key] = _PendingInlineStart(
      blockId: blockId,
      inlineId: inlineId,
      originalType: type,
      type: nodeType,
    );
    if (type == MarkdownNodeType.inlineLatex) {
      _latexInlines.add(key);
    }
  }

  void appendInline({
    required int blockId,
    required int inlineId,
    required String content,
  }) {
    if (_htmlBreakBlocks.contains(blockId)) {
      return;
    }
    final block = _blocks[blockId];
    if (block == null) {
      throw StateError('Missing markdown block $blockId for inline $inlineId');
    }
    final key = _inlineKey(blockId, inlineId);
    final pending = _pendingInlines[key];
    if (pending == null) {
      throw StateError('Missing markdown inline start $blockId/$inlineId');
    }
    var state = _pendingInlineBreaks[blockId] ??
        _PendingLineBreakState(count: _pendingBlockBreaks.remove(blockId) ?? 0);
    MutableMarkdownNode? node = _inlines[key];
    for (final codePoint in content.runes) {
      final char = String.fromCharCode(codePoint);
      if (char == '\n' || char == '\r') {
        state = _accumulatePendingLineBreak(state, char);
        continue;
      }
      node ??= _createInlineNode(block, pending, key);
      if (state.count > 0) {
        _appendPendingLineBreaks(block, node, state.count);
        state = const _PendingLineBreakState();
      }
      block.content.write(char);
      node.content.write(char);
    }
    if (state.count > 0) {
      _pendingInlineBreaks[blockId] = state;
    } else {
      _pendingInlineBreaks.remove(blockId);
    }
  }

  void complete() {
    _finalizeOpenInlineNodes();
    _finalizeOpenLatexBlocks();
    for (final controller in _xmlControllers.values) {
      controller.close();
    }
    for (final controller in _xmlMarkdownControllers.values) {
      controller.close();
    }
    _xmlControllers.clear();
    _xmlMarkdownControllers.clear();
  }

  List<MarkdownNodeStable> toStableNodes({required bool isStreaming}) {
    for (final node in nodes) {
      node.isStreaming = false;
    }
    if (isStreaming && nodes.isNotEmpty) {
      nodes.last.isStreaming = true;
    }
    return <MarkdownNodeStable>[for (final node in nodes) node.toStable()];
  }

  String _inlineKey(int blockId, int inlineId) => '$blockId/$inlineId';

  MutableMarkdownNode _createInlineNode(
    MutableMarkdownNode block,
    _PendingInlineStart pending,
    String key,
  ) {
    final existing = _mergedBlocks.contains(pending.blockId) &&
            pending.type == MarkdownNodeType.plainText &&
            block.children.isNotEmpty &&
            block.children.last.type == MarkdownNodeType.plainText
        ? block.children.last
        : null;
    final node = existing ??
        MutableMarkdownNode(
          type: pending.type,
          stableKey: 'block-${pending.blockId}-inline-${pending.inlineId}',
        );
    if (existing == null) {
      block.children.add(node);
    }
    _inlines[key] = node;
    return node;
  }

  void _finalizeOpenInlineNodes() {
    final blockIds = <int>{..._pendingInlines.values.map((item) => item.blockId)};
    for (final blockId in blockIds) {
      _finalizeInlineForBlock(blockId);
    }
  }

  void _finalizeInlineForBlock(int blockId) {
    final keys = <String>[
      for (final entry in _pendingInlines.entries)
        if (entry.value.blockId == blockId) entry.key,
    ];
    for (final key in keys) {
      final pending = _pendingInlines.remove(key);
      final node = _inlines[key];
      if (pending == null || node == null) {
        continue;
      }
      if (_latexInlines.remove(key)) {
        node.type = MarkdownNodeType.inlineLatex;
      }
      if (pending.originalType == MarkdownNodeType.plainText &&
          _trimAll(node.content.toString()).isEmpty) {
        final block = _blocks[blockId];
        block?.children.remove(node);
        _inlines.remove(key);
      }
    }
  }

  void _finalizeOpenLatexBlocks({int? exceptBlockId}) {
    final ids = <int>[
      for (final id in _latexBlocks)
        if (id != exceptBlockId) id,
    ];
    for (final id in ids) {
      _blocks[id]?.type = MarkdownNodeType.blockLatex;
      _latexBlocks.remove(id);
    }
  }

  bool _canMergeWithHtmlBreak(MutableMarkdownNode? node) {
    return node?.type == MarkdownNodeType.plainText;
  }

  MutableMarkdownNode _htmlBreakNode(int blockId) {
    final node = MutableMarkdownNode(
      type: MarkdownNodeType.htmlBreak,
      stableKey: 'block-$blockId',
    );
    node.content.write('\n');
    return node;
  }

  void _appendHtmlBreakNodes(int count) {
    final normalized = count.clamp(0, _maxConsecutiveRenderedNewlines);
    for (var index = 0; index < normalized; index++) {
      final blockId = -nodes.length - index - 1;
      nodes.add(_htmlBreakNode(blockId));
    }
  }

  void _appendPendingLineBreaks(
    MutableMarkdownNode block,
    MutableMarkdownNode inline,
    int count,
  ) {
    if (block.content.isEmpty) {
      return;
    }
    final normalized = count.clamp(0, _maxConsecutiveRenderedNewlines);
    for (var index = 0; index < normalized; index++) {
      block.content.write('\n');
      inline.content.write('\n');
    }
  }

  _PendingLineBreakState _accumulatePendingLineBreak(
    _PendingLineBreakState state,
    String char,
  ) {
    final normalizedCount = state.count.clamp(
      0,
      _maxConsecutiveRenderedNewlines,
    );
    if (char == '\n' && state.lastWasCarriageReturn && normalizedCount > 0) {
      return _PendingLineBreakState(
        count: normalizedCount,
        lastWasCarriageReturn: false,
      );
    }
    return _PendingLineBreakState(
      count: (normalizedCount + 1).clamp(0, _maxConsecutiveRenderedNewlines),
      lastWasCarriageReturn: char == '\r',
    );
  }

  MutableMarkdownNode? _nodeByKey(String stableKey) {
    MutableMarkdownNode? visit(MutableMarkdownNode node) {
      if (node.stableKey == stableKey) {
        return node;
      }
      for (final child in node.children) {
        final found = visit(child);
        if (found != null) {
          return found;
        }
      }
      return null;
    }

    for (final node in nodes) {
      final found = visit(node);
      if (found != null) {
        return found;
      }
    }
    return null;
  }
}

const int _maxConsecutiveRenderedNewlines = 2;

class _ReplayStringStream {
  _ReplayStringStream();

  _ReplayStringStream.fromHistory(Iterable<String> history) {
    _history.addAll(history);
  }

  final List<String> _history = <String>[];
  final StreamController<String> _controller =
      StreamController<String>.broadcast();
  bool _closed = false;
  late final Stream<String> stream = _buildStream();

  List<String> get history => _history;

  Stream<String> _buildStream() {
    return Stream<String>.multi((controller) {
      for (final chunk in _history) {
        controller.add(chunk);
      }
      final subscription = _controller.stream.listen(
        controller.add,
        onError: controller.addError,
        onDone: controller.close,
      );
      controller.onCancel = subscription.cancel;
    }, isBroadcast: true);
  }

  void add(String chunk) {
    if (_closed) {
      return;
    }
    _history.add(chunk);
    _controller.add(chunk);
  }

  void close() {
    if (_closed) {
      return;
    }
    _closed = true;
    _controller.close();
  }
}

class _ReplayObjectStream {
  _ReplayObjectStream();

  _ReplayObjectStream.fromHistory(Iterable<Object> history) {
    _history.addAll(history);
  }

  final List<Object> _history = <Object>[];
  final StreamController<Object> _controller =
      StreamController<Object>.broadcast();
  bool _closed = false;
  late final Stream<Object> stream = _buildStream();

  List<Object> get history => _history;

  Stream<Object> _buildStream() {
    return Stream<Object>.multi((controller) {
      for (final event in _history) {
        controller.add(event);
      }
      final subscription = _controller.stream.listen(
        controller.add,
        onError: controller.addError,
        onDone: controller.close,
      );
      controller.onCancel = subscription.cancel;
    }, isBroadcast: true);
  }

  void add(Object event) {
    if (_closed) {
      return;
    }
    _history.add(event);
    _controller.add(event);
  }

  void close() {
    if (_closed) {
      return;
    }
    _closed = true;
    _controller.close();
  }
}

class _PendingLineBreakState {
  const _PendingLineBreakState({
    this.count = 0,
    this.lastWasCarriageReturn = false,
  });

  final int count;
  final bool lastWasCarriageReturn;
}

class _PendingInlineStart {
  const _PendingInlineStart({
    required this.blockId,
    required this.inlineId,
    required this.originalType,
    required this.type,
  });

  final int blockId;
  final int inlineId;
  final MarkdownNodeType originalType;
  final MarkdownNodeType type;
}

class _MarkdownEventNodeBuilderSnapshot {
  _MarkdownEventNodeBuilderSnapshot({
    required this.nodes,
    required this.blockKeys,
    required this.inlineKeys,
    required this.pendingInlines,
    required this.htmlBreakBlocks,
    required this.mergedBlocks,
    required this.latexBlocks,
    required this.latexInlines,
    required this.pendingInlineBreaks,
    required this.pendingBlockBreaks,
    required this.xmlControllerHistories,
    required this.xmlMarkdownControllerHistories,
    required this.xmlStreamIndexes,
    required this.xmlMarkdownStreamIndexes,
    required this.pendingHtmlBreakCount,
  });

  factory _MarkdownEventNodeBuilderSnapshot.capture(
    MarkdownEventNodeBuilder builder,
  ) {
    return _MarkdownEventNodeBuilderSnapshot(
      nodes: <MutableMarkdownNode>[
        for (final node in builder.nodes) node.copy(),
      ],
      blockKeys: <int, String>{
        for (final entry in builder._blocks.entries)
          entry.key: entry.value.stableKey,
      },
      inlineKeys: <String, String>{
        for (final entry in builder._inlines.entries)
          entry.key: entry.value.stableKey,
      },
      pendingInlines: <String, _PendingInlineStart>{
        ...builder._pendingInlines,
      },
      htmlBreakBlocks: <int>{...builder._htmlBreakBlocks},
      mergedBlocks: <int>{...builder._mergedBlocks},
      latexBlocks: <int>{...builder._latexBlocks},
      latexInlines: <String>{...builder._latexInlines},
      pendingInlineBreaks: <int, _PendingLineBreakState>{
        ...builder._pendingInlineBreaks,
      },
      pendingBlockBreaks: <int, int>{...builder._pendingBlockBreaks},
      xmlControllerHistories: <int, List<String>>{
        for (final entry in builder._xmlControllers.entries)
          entry.key: <String>[...entry.value.history],
      },
      xmlMarkdownControllerHistories: <int, List<Object>>{
        for (final entry in builder._xmlMarkdownControllers.entries)
          entry.key: <Object>[...entry.value.history],
      },
      xmlStreamIndexes: <int, int>{
        for (final streamEntry in builder.xmlNodeStreams.entries)
          for (final controllerEntry in builder._xmlControllers.entries)
            if (identical(streamEntry.value, controllerEntry.value.stream))
              controllerEntry.key: streamEntry.key,
      },
      xmlMarkdownStreamIndexes: <int, int>{
        for (final streamEntry in builder.xmlMarkdownEventStreams.entries)
          for (final controllerEntry
              in builder._xmlMarkdownControllers.entries)
            if (identical(streamEntry.value, controllerEntry.value.stream))
              controllerEntry.key: streamEntry.key,
      },
      pendingHtmlBreakCount: builder._pendingHtmlBreakCount,
    );
  }

  final List<MutableMarkdownNode> nodes;
  final Map<int, String> blockKeys;
  final Map<String, String> inlineKeys;
  final Map<String, _PendingInlineStart> pendingInlines;
  final Set<int> htmlBreakBlocks;
  final Set<int> mergedBlocks;
  final Set<int> latexBlocks;
  final Set<String> latexInlines;
  final Map<int, _PendingLineBreakState> pendingInlineBreaks;
  final Map<int, int> pendingBlockBreaks;
  final Map<int, List<String>> xmlControllerHistories;
  final Map<int, List<Object>> xmlMarkdownControllerHistories;
  final Map<int, int> xmlStreamIndexes;
  final Map<int, int> xmlMarkdownStreamIndexes;
  final int pendingHtmlBreakCount;

  void restore(MarkdownEventNodeBuilder builder) {
    builder.nodes
      ..clear()
      ..addAll(<MutableMarkdownNode>[
        for (final node in nodes) node.copy(),
      ]);
    builder._blocks
      ..clear()
      ..addEntries(<MapEntry<int, MutableMarkdownNode>>[
        for (final entry in blockKeys.entries)
          if (builder._nodeByKey(entry.value) != null)
            MapEntry<int, MutableMarkdownNode>(
              entry.key,
              builder._nodeByKey(entry.value)!,
            ),
      ]);
    builder._inlines
      ..clear()
      ..addEntries(<MapEntry<String, MutableMarkdownNode>>[
        for (final entry in inlineKeys.entries)
          if (builder._nodeByKey(entry.value) != null)
            MapEntry<String, MutableMarkdownNode>(
              entry.key,
              builder._nodeByKey(entry.value)!,
            ),
      ]);
    builder._pendingInlines
      ..clear()
      ..addAll(pendingInlines);
    builder._htmlBreakBlocks
      ..clear()
      ..addAll(htmlBreakBlocks);
    builder._mergedBlocks
      ..clear()
      ..addAll(mergedBlocks);
    builder._latexBlocks
      ..clear()
      ..addAll(latexBlocks);
    builder._latexInlines
      ..clear()
      ..addAll(latexInlines);
    builder._pendingInlineBreaks
      ..clear()
      ..addAll(pendingInlineBreaks);
    builder._pendingBlockBreaks
      ..clear()
      ..addAll(pendingBlockBreaks);
    for (final controller in builder._xmlControllers.values) {
      controller.close();
    }
    for (final controller in builder._xmlMarkdownControllers.values) {
      controller.close();
    }
    builder._xmlControllers.clear();
    builder._xmlMarkdownControllers.clear();
    builder.xmlNodeStreams.clear();
    builder.xmlMarkdownEventStreams.clear();
    for (final entry in xmlControllerHistories.entries) {
      final controller = _ReplayStringStream.fromHistory(entry.value);
      builder._xmlControllers[entry.key] = controller;
      final streamIndex = xmlStreamIndexes[entry.key];
      if (streamIndex != null) {
        builder.xmlNodeStreams[streamIndex] = controller.stream;
      }
    }
    for (final entry in xmlMarkdownControllerHistories.entries) {
      final controller = _ReplayObjectStream.fromHistory(entry.value);
      builder._xmlMarkdownControllers[entry.key] = controller;
      final streamIndex = xmlMarkdownStreamIndexes[entry.key];
      if (streamIndex != null) {
        builder.xmlMarkdownEventStreams[streamIndex] = controller.stream;
      }
    }
    builder._pendingHtmlBreakCount = pendingHtmlBreakCount;
  }
}

String _trimAll(String value) {
  return value.trim().replaceAll(RegExp(r'\n{3,}'), '\n\n');
}
