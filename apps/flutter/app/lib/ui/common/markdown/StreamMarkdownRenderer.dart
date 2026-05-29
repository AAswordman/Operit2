// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../util/ChatMarkupRegex.dart';
import 'EnhancedCodeBlock.dart';
import 'EnhancedTableBlock.dart';
import 'MarkdownNodeGrouper.dart';
import 'MarkdownBlockQuote.dart';
import 'MarkdownImageRenderer.dart';
import 'MarkdownInlineSpannable.dart';
import 'MarkdownLatexBlock.dart';
import '../../features/chat/components/part/CustomXmlRenderer.dart';

class StreamMarkdownRenderer extends StatelessWidget {
  const StreamMarkdownRenderer({
    super.key,
    required this.content,
    required this.isStreaming,
    required this.textColor,
    required this.backgroundColor,
    this.nodeGrouper = const NoopMarkdownNodeGrouper(),
    this.contentStream,
    this.rendererId,
  });

  final String content;
  final bool isStreaming;
  final Color textColor;
  final Color backgroundColor;
  final MarkdownNodeGrouper nodeGrouper;
  final Stream<String>? contentStream;
  final String? rendererId;

  @override
  Widget build(BuildContext context) {
    final stream = contentStream;
    if (stream != null) {
      return _StreamingMarkdownRenderer(
        initialContent: content,
        contentStream: stream,
        textColor: textColor,
        backgroundColor: backgroundColor,
        nodeGrouper: nodeGrouper,
        rendererId: rendererId,
      );
    }
    final nodes = parseMarkdownNodes(content, isStreaming: isStreaming);
    return _MarkdownNodeColumn(
      nodes: nodes,
      rendererId: rendererId ?? 'flutter-markdown-static',
      textColor: textColor,
      backgroundColor: backgroundColor,
      nodeGrouper: nodeGrouper,
    );
  }
}

const Duration _streamRenderInterval = Duration(milliseconds: 200);
const Duration _nodeFadeInDuration = Duration(milliseconds: 800);

class _StreamingMarkdownRenderer extends StatefulWidget {
  const _StreamingMarkdownRenderer({
    required this.initialContent,
    required this.contentStream,
    required this.textColor,
    required this.backgroundColor,
    required this.nodeGrouper,
    required this.rendererId,
  });

  final String initialContent;
  final Stream<String> contentStream;
  final Color textColor;
  final Color backgroundColor;
  final MarkdownNodeGrouper nodeGrouper;
  final String? rendererId;

  @override
  State<_StreamingMarkdownRenderer> createState() =>
      _StreamingMarkdownRendererState();
}

class _StreamingMarkdownRendererState
    extends State<_StreamingMarkdownRenderer> {
  StreamSubscription<String>? _subscription;
  Timer? _renderTimer;
  late final String _rendererId;
  late _StreamingMarkdownParser _parser;
  List<MarkdownNodeStable> _renderNodes = const <MarkdownNodeStable>[];
  final Map<String, bool> _nodeAnimationStates = <String, bool>{};
  final Set<String> _scheduledVisibleNodeKeys = <String>{};
  bool _streamDone = false;

  @override
  void initState() {
    super.initState();
    _rendererId =
        widget.rendererId ??
        'flutter-stream-markdown-${identityHashCode(this)}';
    _parser = _StreamingMarkdownParser();
    _parser.addChunk(widget.initialContent);
    _synchronizeRenderNodes();
    _subscribe();
  }

  @override
  void didUpdateWidget(covariant _StreamingMarkdownRenderer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.contentStream != widget.contentStream) {
      _subscription?.cancel();
      _renderTimer?.cancel();
      _renderTimer = null;
      _streamDone = false;
      _parser = _StreamingMarkdownParser();
      _parser.addChunk(widget.initialContent);
      _synchronizeRenderNodes();
      _subscribe();
    }
  }

  void _subscribe() {
    _subscription = widget.contentStream.listen(
      (chunk) {
        _parser.addChunk(chunk);
        _renderTimer ??= Timer(_streamRenderInterval, _flushRenderNodes);
      },
      onDone: () {
        _streamDone = true;
        _renderTimer?.cancel();
        _renderTimer = null;
        _flushRenderNodes();
      },
    );
  }

  void _flushRenderNodes() {
    _renderTimer = null;
    if (!mounted) {
      return;
    }
    setState(_synchronizeRenderNodes);
  }

  void _synchronizeRenderNodes() {
    final nextNodes = _parser.toStableNodes(isStreaming: !_streamDone);
    final nextKeys = <String>{
      for (final node in nextNodes) _nodeKeyForStable(_rendererId, node),
    };
    final keysToReveal = <String>[];

    for (final key in nextKeys) {
      if (!_nodeAnimationStates.containsKey(key)) {
        _nodeAnimationStates[key] = false;
        keysToReveal.add(key);
      }
    }
    _nodeAnimationStates.removeWhere((key, value) => !nextKeys.contains(key));
    _scheduledVisibleNodeKeys.removeWhere((key) => !nextKeys.contains(key));
    _renderNodes = nextNodes;
    _scheduleNodeFadeIn(keysToReveal);
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
          if (_nodeAnimationStates.containsKey(key)) {
            _nodeAnimationStates[key] = true;
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
    return _MarkdownNodeColumn(
      nodes: _renderNodes,
      rendererId: _rendererId,
      textColor: widget.textColor,
      backgroundColor: widget.backgroundColor,
      nodeGrouper: widget.nodeGrouper,
      nodeAnimationStates: _nodeAnimationStates,
    );
  }
}

const double _markdownParagraphBreakHeight = 4;
const double _markdownLineBlockBottomPadding = 3;
const double _markdownCanvasLineHeightMultiplier = 1.3;
const int _maxConsecutiveRenderedNewlines = 2;

class _MarkdownNodeColumn extends StatelessWidget {
  const _MarkdownNodeColumn({
    required this.nodes,
    required this.rendererId,
    required this.textColor,
    required this.backgroundColor,
    required this.nodeGrouper,
    this.nodeAnimationStates,
  });

  final List<MarkdownNodeStable> nodes;
  final String rendererId;
  final Color textColor;
  final Color backgroundColor;
  final MarkdownNodeGrouper nodeGrouper;
  final Map<String, bool>? nodeAnimationStates;

  @override
  Widget build(BuildContext context) {
    final groupedItems = nodeGrouper.group(nodes, rendererId);
    final lastRenderableIndex = _lastTypewriterNodeIndex(nodes);

    bool isVisibleAt(int index) {
      final states = nodeAnimationStates;
      if (states == null) {
        return true;
      }
      final node = nodes[index];
      final key = _nodeKeyForStable(rendererId, node);
      return states.containsKey(key) ? states[key] == true : true;
    }

    Widget renderNodeAt(int index) {
      final node = nodes[index];
      if (node.type == MarkdownNodeType.xmlBlock) {
        return CustomXmlRenderer(
          xmlContent: node.content,
          isStreaming: node.isStreaming,
          textColor: textColor,
        );
      }
      return _MarkdownText(
        key: ValueKey<String>(_nodeKeyForStable(rendererId, node)),
        nodeKey: _nodeKeyForStable(rendererId, node),
        text: node.content,
        textColor: textColor,
        backgroundColor: backgroundColor,
        isStreaming: node.isStreaming,
        isLastNode: index == lastRenderableIndex,
        nodeType: node.type,
      );
    }

    Widget renderAnimatedNodeAt(int index) {
      final node = nodes[index];
      if (_canTypewriteNode(node.type)) {
        return renderNodeAt(index);
      }
      return _AnimatedMarkdownNode(
        isVisible: isVisibleAt(index),
        child: renderNodeAt(index),
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        for (final item in groupedItems)
          if (item is MarkdownSingleItem)
            renderAnimatedNodeAt(item.index)
          else if (item is MarkdownGroupItem)
            nodeGrouper.renderGroup(
              group: item,
              nodes: nodes,
              rendererId: rendererId,
              isVisible: isVisibleAt(item.startIndex),
              isLastNode: item.endIndexInclusive == lastRenderableIndex,
              textColor: textColor,
              renderNodeAt: renderNodeAt,
            ),
      ],
    );
  }
}

String _nodeKeyForStable(String rendererId, MarkdownNodeStable node) {
  return '$rendererId-${node.stableKey}';
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

int _lastTypewriterNodeIndex(List<MarkdownNodeStable> nodes) {
  for (var index = nodes.length - 1; index >= 0; index--) {
    if (_canTypewriteNode(nodes[index].type) &&
        nodes[index].content.isNotEmpty) {
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

class _MutableMarkdownNode {
  _MutableMarkdownNode({
    required this.type,
    required this.stableKey,
    String initialContent = '',
  }) : content = StringBuffer(initialContent);

  final MarkdownNodeType type;
  final String stableKey;
  final StringBuffer content;
}

class _PendingLineBreakState {
  const _PendingLineBreakState({
    this.count = 0,
    this.lastWasCarriageReturn = false,
  });

  final int count;
  final bool lastWasCarriageReturn;
}

class _StreamingMarkdownParser {
  final List<_MutableMarkdownNode> _nodes = <_MutableMarkdownNode>[];
  final StringBuffer _tagCandidate = StringBuffer();
  _MutableMarkdownNode? _activeTextNode;
  _MutableMarkdownNode? _activeXmlNode;
  String? _activeXmlCloseTag;
  _PendingLineBreakState _pendingLineBreakState =
      const _PendingLineBreakState();
  int _nextNodeId = 0;

  void addChunk(String chunk) {
    for (var index = 0; index < chunk.length; index++) {
      _appendChar(chunk[index]);
    }
  }

  List<MarkdownNodeStable> toStableNodes({required bool isStreaming}) {
    final out = <MarkdownNodeStable>[
      for (final node in _nodes)
        MarkdownNodeStable(
          type: node.type,
          content: node.content.toString(),
          isStreaming: false,
          stableKey: node.stableKey,
        ),
    ];
    if (isStreaming && out.isNotEmpty) {
      final last = out.last;
      out[out.length - 1] = MarkdownNodeStable(
        type: last.type,
        content: last.content,
        isStreaming: true,
        stableKey: last.stableKey,
      );
    }
    return out;
  }

  void _appendChar(String char) {
    final xmlNode = _activeXmlNode;
    if (xmlNode != null) {
      xmlNode.content.write(char);
      final closeTag = _activeXmlCloseTag;
      if (closeTag != null &&
          xmlNode.content.toString().toLowerCase().endsWith(closeTag)) {
        _activeXmlNode = null;
        _activeXmlCloseTag = null;
      }
      return;
    }

    if (_tagCandidate.isNotEmpty || char == '<') {
      _appendTagCandidate(char);
      return;
    }

    _appendPlainChar(char);
  }

  void _appendTagCandidate(String char) {
    _tagCandidate.write(char);
    final candidate = _tagCandidate.toString();
    if (!candidate.contains('>')) {
      return;
    }

    final match = ChatMarkupRegex.xmlBlockStartTag.firstMatch(candidate);
    if (match != null && match.start == 0 && match.end == candidate.length) {
      final rawTag = match.group(1)!;
      final xmlNode = _newNode(MarkdownNodeType.xmlBlock);
      xmlNode.content.write(candidate);
      _activeXmlNode = xmlNode;
      _activeXmlCloseTag = '</${rawTag.toLowerCase()}>';
      if (candidate.trimRight().endsWith('/>')) {
        _activeXmlNode = null;
        _activeXmlCloseTag = null;
      }
    } else {
      for (var index = 0; index < candidate.length; index++) {
        _appendPlainChar(candidate[index]);
      }
    }
    _tagCandidate.clear();
  }

  void _appendPlainChar(String char) {
    if (char == '\n' || char == '\r') {
      _pendingLineBreakState = _accumulatePendingLineBreak(
        _pendingLineBreakState,
        char,
      );
      return;
    }

    final node = _activeTextNode ?? _newNode(_classifyTextNodeType(char));
    _activeTextNode = node;
    final pendingCount = _pendingLineBreakState.count
        .clamp(0, _maxConsecutiveRenderedNewlines)
        .toInt();
    if (pendingCount > 0 && node.content.isNotEmpty) {
      for (var index = 0; index < pendingCount; index++) {
        node.content.write('\n');
      }
    }
    _pendingLineBreakState = const _PendingLineBreakState();
    node.content.write(char);
  }

  _PendingLineBreakState _accumulatePendingLineBreak(
    _PendingLineBreakState state,
    String char,
  ) {
    final normalizedCount = state.count
        .clamp(0, _maxConsecutiveRenderedNewlines)
        .toInt();
    if (char == '\n' && state.lastWasCarriageReturn && normalizedCount > 0) {
      return _PendingLineBreakState(count: normalizedCount);
    }
    return _PendingLineBreakState(
      count: (normalizedCount + 1)
          .clamp(0, _maxConsecutiveRenderedNewlines)
          .toInt(),
      lastWasCarriageReturn: char == '\r',
    );
  }

  _MutableMarkdownNode _newNode(MarkdownNodeType type) {
    final node = _MutableMarkdownNode(
      type: type,
      stableKey: 'node-${_nextNodeId++}',
    );
    _nodes.add(node);
    if (type == MarkdownNodeType.xmlBlock) {
      _activeTextNode = null;
      _pendingLineBreakState = const _PendingLineBreakState();
    }
    return node;
  }
}

MarkdownNodeType _classifyTextNodeType(String firstChar) {
  if (firstChar == '#') {
    return MarkdownNodeType.header;
  }
  if (firstChar == '-' || firstChar == '*' || firstChar == '+') {
    return MarkdownNodeType.unorderedList;
  }
  if (RegExp(r'\d').hasMatch(firstChar)) {
    return MarkdownNodeType.orderedList;
  }
  return MarkdownNodeType.plainText;
}

List<MarkdownNodeStable> parseMarkdownNodes(
  String content, {
  required bool isStreaming,
}) {
  final nodes = <MarkdownNodeStable>[];
  final openTagPattern = ChatMarkupRegex.xmlBlockStartTag;
  var cursor = 0;

  while (cursor < content.length) {
    final sliced = content.substring(cursor);
    final match = openTagPattern.firstMatch(sliced);
    if (match == null) {
      _addTextNode(nodes, sliced);
      break;
    }

    final start = cursor + match.start;
    final end = cursor + match.end;
    if (start > cursor) {
      _addTextNode(nodes, content.substring(cursor, start));
    }

    final rawTag = match.group(1)!;
    final closeTag = '</$rawTag>';
    final closeIndex = content.toLowerCase().indexOf(
      closeTag.toLowerCase(),
      end,
    );
    final xmlEnd = closeIndex >= 0
        ? closeIndex + closeTag.length
        : content.length;
    final xmlContent = content.substring(start, xmlEnd);
    nodes.add(
      MarkdownNodeStable(
        type: MarkdownNodeType.xmlBlock,
        content: xmlContent,
        isStreaming: isStreaming && closeIndex < 0,
        stableKey: 'node-${nodes.length}',
      ),
    );
    cursor = xmlEnd;
  }

  if (nodes.isNotEmpty && isStreaming) {
    final last = nodes.last;
    nodes[nodes.length - 1] = MarkdownNodeStable(
      type: last.type,
      content: last.content,
      isStreaming: true,
      stableKey: last.stableKey,
    );
  }
  return nodes;
}

void _addTextNode(List<MarkdownNodeStable> nodes, String text) {
  final cleaned = _normalizeRenderedLineBreaks(text);
  if (cleaned.trim().isEmpty) {
    return;
  }
  nodes.add(
    MarkdownNodeStable(
      type: _classifyStableTextNodeType(cleaned),
      content: cleaned,
      isStreaming: false,
      stableKey: 'node-${nodes.length}',
    ),
  );
}

String _normalizeRenderedLineBreaks(String text) {
  final buffer = StringBuffer();
  var state = const _PendingLineBreakState();
  for (var index = 0; index < text.length; index++) {
    final char = text[index];
    if (char == '\n' || char == '\r') {
      final normalizedCount = state.count
          .clamp(0, _maxConsecutiveRenderedNewlines)
          .toInt();
      if (char == '\n' && state.lastWasCarriageReturn && normalizedCount > 0) {
        state = _PendingLineBreakState(count: normalizedCount);
      } else {
        state = _PendingLineBreakState(
          count: (normalizedCount + 1)
              .clamp(0, _maxConsecutiveRenderedNewlines)
              .toInt(),
          lastWasCarriageReturn: char == '\r',
        );
      }
      continue;
    }
    if (state.count > 0 && buffer.isNotEmpty) {
      for (var lineIndex = 0; lineIndex < state.count; lineIndex++) {
        buffer.write('\n');
      }
    }
    state = const _PendingLineBreakState();
    buffer.write(char);
  }
  return buffer.toString();
}

MarkdownNodeType _classifyStableTextNodeType(String text) {
  final firstLine = text.split('\n').first.trimLeft();
  if (_headingLevel(firstLine) > 0) {
    return MarkdownNodeType.header;
  }
  if (_isOrderedLine(firstLine)) {
    return MarkdownNodeType.orderedList;
  }
  if (_isBulletLine(firstLine)) {
    return MarkdownNodeType.unorderedList;
  }
  return MarkdownNodeType.plainText;
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

class _MarkdownText extends StatefulWidget {
  const _MarkdownText({
    super.key,
    required this.nodeKey,
    required this.text,
    required this.textColor,
    required this.backgroundColor,
    required this.isStreaming,
    required this.isLastNode,
    required this.nodeType,
  });

  final String nodeKey;
  final String text;
  final Color textColor;
  final Color backgroundColor;
  final bool isStreaming;
  final bool isLastNode;
  final MarkdownNodeType nodeType;

  @override
  State<_MarkdownText> createState() => _MarkdownTextState();
}

class _MarkdownTextState extends State<_MarkdownText>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late Animation<double> _revealAnimation;
  double _revealValue = 0;
  int _targetLength = 0;

  bool get _enableTypewriter =>
      widget.isStreaming &&
      widget.isLastNode &&
      _canTypewriteNode(widget.nodeType);

  @override
  void initState() {
    super.initState();
    _controller =
        AnimationController(vsync: this, duration: _streamRenderInterval)
          ..addListener(() {
            setState(() {
              _revealValue = _revealAnimation.value;
            });
          });
    _targetLength = _typewriterTextLength(widget.text);
    _revealAnimation = AlwaysStoppedAnimation<double>(
      _enableTypewriter ? 0 : _targetLength.toDouble(),
    );
    _revealValue = _revealAnimation.value;
    if (_enableTypewriter && _targetLength > 0) {
      _animateRevealTo(_targetLength);
    }
  }

  @override
  void didUpdateWidget(covariant _MarkdownText oldWidget) {
    super.didUpdateWidget(oldWidget);
    final targetLength = _typewriterTextLength(widget.text);
    if (!_enableTypewriter) {
      _controller.stop();
      _targetLength = targetLength;
      _revealValue = targetLength.toDouble();
      _revealAnimation = AlwaysStoppedAnimation<double>(_revealValue);
      return;
    }
    if (targetLength < _revealValue) {
      _controller.stop();
      _targetLength = targetLength;
      _revealValue = targetLength.toDouble();
      _revealAnimation = AlwaysStoppedAnimation<double>(_revealValue);
      return;
    }
    if (targetLength == _targetLength) {
      return;
    }
    _targetLength = targetLength;
    _animateRevealTo(targetLength);
  }

  void _animateRevealTo(int targetLength) {
    final current = _revealValue.clamp(0, targetLength).toDouble();
    _revealAnimation = Tween<double>(
      begin: current,
      end: targetLength.toDouble(),
    ).animate(CurvedAnimation(parent: _controller, curve: Curves.linear));
    _controller.forward(from: 0);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final widgets = <Widget>[];
    final codeLines = <String>[];
    final paragraphLines = <String>[];
    var inCode = false;
    var codeLanguage = '';
    final lines = widget.text.split('\n');
    var index = 0;
    var textBlockIndex = 0;
    var typewriterOffset = 0;
    final enableTypewriter = _enableTypewriter;

    _RevealSegment revealSegmentFor(int length) {
      if (!enableTypewriter) {
        return _RevealSegment(
          revealLength: length.toDouble(),
          showCursor: false,
        );
      }
      final segmentStart = typewriterOffset;
      final segmentEnd = segmentStart + length;
      final localReveal = (_revealValue - typewriterOffset)
          .clamp(0, length)
          .toDouble();
      typewriterOffset += length;
      return _RevealSegment(
        revealLength: localReveal,
        showCursor:
            length > 0 &&
            _revealValue >= segmentStart &&
            (_revealValue < segmentEnd || segmentEnd == _targetLength),
      );
    }

    void flushCode() {
      if (codeLines.isEmpty) {
        return;
      }
      widgets.add(
        EnhancedCodeBlock(code: codeLines.join('\n'), language: codeLanguage),
      );
      codeLines.clear();
    }

    void flushParagraph() {
      if (paragraphLines.isEmpty) {
        return;
      }
      widgets.add(
        _MarkdownParagraph(
          textKey: '${widget.nodeKey}-text-${textBlockIndex++}',
          text: paragraphLines.join('\n'),
          color: widget.textColor,
          reveal: revealSegmentFor(paragraphLines.join('\n').length),
        ),
      );
      paragraphLines.clear();
    }

    while (index < lines.length) {
      final line = lines[index];
      final trimmed = line.trimRight();
      if (trimmed.startsWith('```')) {
        if (inCode) {
          flushCode();
          codeLanguage = '';
        } else {
          flushParagraph();
          codeLanguage = trimmed.substring(3).trim();
        }
        inCode = !inCode;
      } else if (inCode) {
        codeLines.add(line);
      } else if (_isBlockLatexStart(trimmed)) {
        flushParagraph();
        final latexLines = <String>[trimmed];
        final start = trimmed.trimLeft();
        final singleLine = start.length > 2 && _isBlockLatexEnd(start, start);
        while (!singleLine && index + 1 < lines.length) {
          index++;
          final nextLine = lines[index].trimRight();
          latexLines.add(nextLine);
          if (_isBlockLatexEnd(start, nextLine.trimRight())) {
            break;
          }
        }
        widgets.add(
          MarkdownLatexBlock(
            content: latexLines.join('\n'),
            textColor: widget.textColor,
          ),
        );
      } else if (_isTableStart(lines, index)) {
        flushParagraph();
        final tableLines = <String>[];
        while (index < lines.length && lines[index].trim().contains('|')) {
          tableLines.add(lines[index]);
          index++;
        }
        index--;
        widgets.add(
          EnhancedTableBlock(
            tableText: tableLines.join('\n'),
            textColor: widget.textColor,
          ),
        );
      } else if (trimmed.trimLeft().startsWith('>')) {
        flushParagraph();
        final quoteLines = <String>[];
        while (index < lines.length &&
            lines[index].trimLeft().startsWith('>')) {
          quoteLines.add(lines[index]);
          index++;
        }
        index--;
        widgets.add(
          MarkdownBlockQuote(
            content: quoteLines.join('\n'),
            textColor: widget.textColor,
            backgroundColor: widget.backgroundColor,
            isStreaming: widget.isStreaming,
          ),
        );
      } else if (isCompleteImageMarkdown(trimmed.trim())) {
        flushParagraph();
        widgets.add(
          MarkdownImageRenderer(
            imageMarkdown: trimmed.trim(),
            textColor: widget.textColor,
          ),
        );
      } else if (_isHorizontalRule(trimmed)) {
        flushParagraph();
        widgets.add(const MarkdownHorizontalRule());
      } else if (trimmed.isEmpty) {
        flushParagraph();
        if (widgets.isNotEmpty) {
          widgets.add(const SizedBox(height: _markdownParagraphBreakHeight));
        }
      } else if (_headingLevel(trimmed) > 0 ||
          _isBulletLine(trimmed) ||
          _isOrderedLine(trimmed)) {
        flushParagraph();
        widgets.add(
          _MarkdownLine(
            textKey: '${widget.nodeKey}-text-${textBlockIndex++}',
            text: trimmed,
            color: widget.textColor,
            reveal: revealSegmentFor(_typewriterLineLength(trimmed)),
          ),
        );
      } else {
        paragraphLines.add(trimmed);
      }
      index++;
    }
    flushCode();
    flushParagraph();

    if (widget.isStreaming && !enableTypewriter) {
      widgets.add(
        const Padding(
          padding: EdgeInsets.only(top: 2),
          child: StreamingCursor(),
        ),
      );
    }
    final content = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: widgets,
    );
    if (widget.isStreaming) {
      return content;
    }
    return SelectionArea(child: content);
  }
}

class _MarkdownLine extends StatelessWidget {
  const _MarkdownLine({
    required this.textKey,
    required this.text,
    required this.color,
    required this.reveal,
  });

  final String textKey;
  final String text;
  final Color color;
  final _RevealSegment reveal;

  @override
  Widget build(BuildContext context) {
    if (text.isEmpty) {
      return const SizedBox(height: _markdownParagraphBreakHeight);
    }
    final theme = Theme.of(context);
    final headingLevel = _headingLevel(text);
    if (headingLevel > 0) {
      return _MarkdownHeading(
        textKey: textKey,
        text: text,
        color: color,
        reveal: reveal,
      );
    }
    if (_isBulletLine(text)) {
      final body = text.substring(2);
      return Padding(
        padding: const EdgeInsets.only(bottom: _markdownLineBlockBottomPadding),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.only(top: 8, right: 8),
              child: Container(
                width: 4,
                height: 4,
                decoration: BoxDecoration(
                  color: color.withValues(alpha: 0.7),
                  shape: BoxShape.circle,
                ),
              ),
            ),
            Expanded(
              child: _TypewriterMarkdownRichText(
                key: ValueKey<String>('$textKey-body'),
                text: body,
                color: color,
                revealLength: reveal.revealLength,
                showCursor: reveal.showCursor,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: color,
                  height: 1.3,
                ),
              ),
            ),
          ],
        ),
      );
    }
    if (_isOrderedLine(text)) {
      final match = RegExp(r'^(\d+)\.\s*').firstMatch(text);
      final marker = match?.group(1) ?? '';
      final body = match == null ? text : text.substring(match.end);
      return Padding(
        padding: const EdgeInsets.only(bottom: _markdownLineBlockBottomPadding),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.only(right: 4),
              child: Text(
                '$marker.',
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: color,
                  fontWeight: FontWeight.w700,
                  height: 1.3,
                ),
              ),
            ),
            Expanded(
              child: _TypewriterMarkdownRichText(
                key: ValueKey<String>('$textKey-body'),
                text: body,
                color: color,
                revealLength: reveal.revealLength,
                showCursor: reveal.showCursor,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: color,
                  height: 1.3,
                ),
              ),
            ),
          ],
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.only(bottom: _markdownLineBlockBottomPadding),
      child: _TypewriterMarkdownRichText(
        key: ValueKey<String>(textKey),
        text: text,
        color: color,
        revealLength: reveal.revealLength,
        showCursor: reveal.showCursor,
        style: theme.textTheme.bodyMedium?.copyWith(color: color, height: 1.3),
      ),
    );
  }
}

class _MarkdownHeading extends StatelessWidget {
  const _MarkdownHeading({
    required this.textKey,
    required this.text,
    required this.color,
    required this.reveal,
  });

  final String textKey;
  final String text;
  final Color color;
  final _RevealSegment reveal;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final effectiveLevel = _determineHeaderLevel(text);
    final headingText = _markdownHeaderText(text);
    final style = _markdownHeaderStyle(theme, effectiveLevel)?.copyWith(
      color: color,
      fontWeight: FontWeight.w700,
      height: _markdownCanvasLineHeightMultiplier,
    );
    final topPadding = _markdownHeaderTopPadding(effectiveLevel);
    final bottomPadding = _markdownHeaderBottomPadding(effectiveLevel);

    return Padding(
      padding: EdgeInsets.only(top: topPadding, bottom: bottomPadding),
      child: _TypewriterMarkdownRichText(
        key: ValueKey<String>(textKey),
        text: headingText,
        color: color,
        revealLength: reveal.revealLength,
        showCursor: reveal.showCursor,
        style: style,
      ),
    );
  }
}

TextStyle? _markdownHeaderStyle(ThemeData theme, int level) {
  return switch (level) {
    1 => theme.textTheme.headlineMedium,
    2 => theme.textTheme.headlineSmall,
    3 => theme.textTheme.titleLarge,
    4 => theme.textTheme.titleMedium,
    5 => theme.textTheme.titleSmall,
    _ => theme.textTheme.bodyMedium,
  };
}

double _markdownHeaderTopPadding(int level) {
  return switch (level) {
    1 => 12,
    2 => 10,
    3 => 8,
    _ => 6,
  };
}

double _markdownHeaderBottomPadding(int level) {
  return switch (level) {
    1 || 2 => 4,
    _ => 2,
  };
}

String _markdownHeaderText(String text) {
  return text.replaceFirst(RegExp(r'^\s*#+\s*'), '').trim();
}

class _MarkdownParagraph extends StatelessWidget {
  const _MarkdownParagraph({
    required this.textKey,
    required this.text,
    required this.color,
    required this.reveal,
  });

  final String textKey;
  final String text;
  final Color color;
  final _RevealSegment reveal;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: _markdownLineBlockBottomPadding),
      child: _TypewriterMarkdownRichText(
        key: ValueKey<String>(textKey),
        text: text,
        color: color,
        revealLength: reveal.revealLength,
        showCursor: reveal.showCursor,
        style: theme.textTheme.bodyMedium?.copyWith(color: color, height: 1.3),
      ),
    );
  }
}

class _TypewriterMarkdownRichText extends StatelessWidget {
  const _TypewriterMarkdownRichText({
    super.key,
    required this.text,
    required this.color,
    required this.style,
    required this.revealLength,
    required this.showCursor,
  });

  final String text;
  final Color color;
  final TextStyle? style;
  final double revealLength;
  final bool showCursor;

  @override
  Widget build(BuildContext context) {
    final span = buildMarkdownInlineSpannableFromText(
      context: context,
      text: text,
      textColor: color,
      baseStyle: style,
    );
    if (revealLength >= text.length && !showCursor) {
      return Text.rich(span, style: style);
    }
    if (_containsWidgetSpan(span)) {
      return Text.rich(span, style: style);
    }
    return LayoutBuilder(
      builder: (context, constraints) {
        final textDirection = Directionality.of(context);
        final painter = TextPainter(
          text: span,
          textDirection: textDirection,
          textScaler: MediaQuery.textScalerOf(context),
        )..layout(maxWidth: constraints.maxWidth);
        final cursorPosition = showCursor
            ? _cursorPositionForReveal(painter, revealLength)
            : null;
        return SizedBox(
          width: constraints.maxWidth,
          height: painter.height,
          child: Stack(
            clipBehavior: Clip.none,
            children: <Widget>[
              CustomPaint(
                size: Size(constraints.maxWidth, painter.height),
                painter: _RevealTextPainter(
                  span: span,
                  textDirection: textDirection,
                  textScaler: MediaQuery.textScalerOf(context),
                  maxWidth: constraints.maxWidth,
                  revealLength: revealLength,
                ),
              ),
              if (cursorPosition != null)
                Positioned(
                  left: cursorPosition.dx,
                  top: cursorPosition.dy,
                  child: const StreamingCursor(),
                ),
            ],
          ),
        );
      },
    );
  }
}

class _RevealSegment {
  const _RevealSegment({required this.revealLength, required this.showCursor});

  final double revealLength;
  final bool showCursor;
}

Offset _cursorPositionForReveal(TextPainter painter, double revealLength) {
  final textLength = painter.plainText.length;
  if (textLength == 0) {
    return Offset.zero;
  }
  final target = revealLength.clamp(0, textLength).toDouble();
  final baseLen = target.floor().clamp(0, textLength).toInt();
  final partial = (target - baseLen).clamp(0, 1).toDouble();

  if (baseLen < textLength) {
    final boxes = painter.getBoxesForSelection(
      TextSelection(baseOffset: baseLen, extentOffset: baseLen + 1),
    );
    if (boxes.isNotEmpty) {
      final rect = boxes.first.toRect();
      final top = rect.top + ((rect.height - 16) / 2).clamp(0, rect.height);
      return Offset(rect.left + rect.width * partial, top);
    }
  }

  final offset = (baseLen - 1).clamp(0, textLength - 1).toInt();
  final boxes = painter.getBoxesForSelection(
    TextSelection(baseOffset: offset, extentOffset: offset + 1),
  );
  if (boxes.isEmpty) {
    return Offset.zero;
  }
  final rect = boxes.last.toRect();
  final top = rect.top + ((rect.height - 16) / 2).clamp(0, rect.height);
  return Offset(rect.right, top);
}

bool _containsWidgetSpan(InlineSpan span) {
  if (span is WidgetSpan) {
    return true;
  }
  if (span is TextSpan) {
    final children = span.children;
    if (children == null) {
      return false;
    }
    return children.any(_containsWidgetSpan);
  }
  return false;
}

class _RevealTextPainter extends CustomPainter {
  const _RevealTextPainter({
    required this.span,
    required this.textDirection,
    required this.textScaler,
    required this.maxWidth,
    required this.revealLength,
  });

  final InlineSpan span;
  final TextDirection textDirection;
  final TextScaler textScaler;
  final double maxWidth;
  final double revealLength;

  @override
  void paint(Canvas canvas, Size size) {
    final painter = TextPainter(
      text: span,
      textDirection: textDirection,
      textScaler: textScaler,
    )..layout(maxWidth: maxWidth);
    final textLength = painter.plainText.length;
    if (textLength == 0) {
      return;
    }
    final target = revealLength.clamp(0, textLength).toDouble();
    final baseLen = target.floor().clamp(0, textLength).toInt();
    final partial = (target - baseLen).clamp(0, 1).toDouble();

    _paintSelectionRange(canvas, painter, 0, baseLen);
    if (baseLen < textLength && partial > 0) {
      _paintPartialCharacter(canvas, painter, baseLen, partial);
    }
  }

  void _paintSelectionRange(
    Canvas canvas,
    TextPainter painter,
    int start,
    int end,
  ) {
    if (end <= start) {
      return;
    }
    final boxes = painter.getBoxesForSelection(
      TextSelection(baseOffset: start, extentOffset: end),
    );
    for (final box in boxes) {
      final rect = box.toRect();
      canvas.save();
      canvas.clipRect(rect);
      painter.paint(canvas, Offset.zero);
      canvas.restore();
    }
  }

  void _paintPartialCharacter(
    Canvas canvas,
    TextPainter painter,
    int offset,
    double partial,
  ) {
    final boxes = painter.getBoxesForSelection(
      TextSelection(baseOffset: offset, extentOffset: offset + 1),
    );
    for (final box in boxes) {
      final rect = box.toRect();
      final left = rect.left;
      final right = rect.left + rect.width * partial;
      if (right <= left) {
        continue;
      }
      canvas.save();
      canvas.clipRect(Rect.fromLTRB(left, rect.top, right, rect.bottom));
      canvas.saveLayer(
        Rect.fromLTRB(left, rect.top, right, rect.bottom),
        Paint()..color = Color.fromRGBO(255, 255, 255, partial),
      );
      painter.paint(canvas, Offset.zero);
      canvas.restore();
      canvas.restore();
    }
  }

  @override
  bool shouldRepaint(covariant _RevealTextPainter oldDelegate) {
    return oldDelegate.span != span ||
        oldDelegate.textDirection != textDirection ||
        oldDelegate.textScaler != textScaler ||
        oldDelegate.maxWidth != maxWidth ||
        oldDelegate.revealLength != revealLength;
  }
}

class StreamingCursor extends StatefulWidget {
  const StreamingCursor({super.key});

  @override
  State<StreamingCursor> createState() => _StreamingCursorState();
}

class _StreamingCursorState extends State<StreamingCursor>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 900),
    )..repeat(reverse: true);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final color = Theme.of(context).colorScheme.primary;
    return Align(
      alignment: Alignment.centerLeft,
      widthFactor: 1,
      heightFactor: 1,
      child: FadeTransition(
        opacity: Tween<double>(begin: 0.25, end: 0.85).animate(_controller),
        child: SizedBox(
          width: 7,
          height: 16,
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: color,
              borderRadius: BorderRadius.circular(2),
            ),
          ),
        ),
      ),
    );
  }
}

int _headingLevel(String text) {
  return _determineHeaderLevel(text);
}

int _determineHeaderLevel(String text) {
  final match = RegExp(r'^\s*(#{1,6})').firstMatch(text);
  return match?.group(1)?.length ?? 0;
}

bool _isBulletLine(String text) {
  return text.startsWith('- ') || text.startsWith('* ');
}

bool _isOrderedLine(String text) {
  return RegExp(r'^\d+\.\s+').hasMatch(text);
}

bool _isHorizontalRule(String text) {
  return RegExp(r'^\s{0,3}([-*_])(?:\s*\1){2,}\s*$').hasMatch(text);
}

bool _isTableStart(List<String> lines, int index) {
  if (index + 1 >= lines.length) {
    return false;
  }
  final current = lines[index].trim();
  final next = lines[index + 1].trim();
  return current.contains('|') &&
      next.contains('|') &&
      _isMarkdownTableSeparator(next);
}

bool _isMarkdownTableSeparator(String line) {
  final cells = line
      .replaceFirst(RegExp(r'^\|'), '')
      .replaceFirst(RegExp(r'\|$'), '')
      .split('|')
      .map((cell) => cell.trim());
  return cells.isNotEmpty &&
      cells.every((cell) => RegExp(r'^:?-{3,}:?$').hasMatch(cell));
}

bool _isBlockLatexStart(String line) {
  final trimmed = line.trimLeft();
  return trimmed.startsWith(r'$$') || trimmed.startsWith(r'\[');
}

bool _isBlockLatexEnd(String startLine, String line) {
  final trimmed = line.trimRight();
  if (startLine.startsWith(r'$$')) {
    return trimmed.endsWith(r'$$') && trimmed.length > 2;
  }
  return trimmed.endsWith(r'\]');
}

class MarkdownHorizontalRule extends StatelessWidget {
  const MarkdownHorizontalRule({super.key});

  @override
  Widget build(BuildContext context) {
    return Divider(
      height: 5,
      thickness: 1,
      color: Theme.of(context).colorScheme.outline.withValues(alpha: 0.5),
    );
  }
}
