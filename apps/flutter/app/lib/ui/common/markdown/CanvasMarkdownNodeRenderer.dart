// ignore_for_file: file_names

part of 'StreamMarkdownRenderer.dart';

class CanvasMarkdownNodeRenderer extends StatelessWidget {
  const CanvasMarkdownNodeRenderer({
    super.key,
    required this.nodeKey,
    required this.node,
    required this.textColor,
    required this.backgroundColor,
    required this.isLastNode,
    required this.splitMarkdownContent,
    this.onLinkClick,
    this.nestedMarkdownEventStream,
  });

  final String nodeKey;
  final MarkdownNodeStable node;
  final Color textColor;
  final Color backgroundColor;
  final bool isLastNode;
  final MarkdownContentSplitter splitMarkdownContent;
  final void Function(String url)? onLinkClick;
  final Stream<Object>? nestedMarkdownEventStream;

  /// Builds one stable Markdown node without repainting preceding nodes.
  @override
  Widget build(BuildContext context) {
    return _MarkdownText(
      nodeKey: nodeKey,
      text: node.content,
      textColor: textColor,
      backgroundColor: backgroundColor,
      isStreaming: node.isStreaming,
      isLastNode: isLastNode,
      nodeType: node.type,
      children: node.children,
      headerLevel: node.headerLevel,
      sourcePrefix: node.sourcePrefix,
      nestedMarkdownEventStream: nestedMarkdownEventStream,
      onLinkClick: onLinkClick,
      splitMarkdownContent: splitMarkdownContent,
    );
  }
}

class _ParsedCodeBlock {
  const _ParsedCodeBlock({required this.code, required this.language});

  final String code;
  final String language;
}

_ParsedCodeBlock _parseCodeBlock(String text) {
  final lines = text.trim().split('\n');
  final firstLine = lines.isEmpty ? '' : lines.first;
  final language = firstLine.startsWith('```')
      ? firstLine.substring(3).trim()
      : '';
  final codeLines = lines.skipWhile((line) => line.startsWith('```')).toList();
  while (codeLines.isNotEmpty && codeLines.last.trimRight().endsWith('```')) {
    codeLines.removeLast();
  }
  return _ParsedCodeBlock(code: codeLines.join('\n'), language: language);
}

class _MarkdownText extends StatefulWidget {
  const _MarkdownText({
    required this.nodeKey,
    required this.text,
    required this.textColor,
    required this.backgroundColor,
    required this.isStreaming,
    required this.isLastNode,
    required this.nodeType,
    required this.children,
    required this.headerLevel,
    required this.sourcePrefix,
    required this.splitMarkdownContent,
    this.onLinkClick,
    this.nestedMarkdownEventStream,
  });

  final String nodeKey;
  final String text;
  final Color textColor;
  final Color backgroundColor;
  final bool isStreaming;
  final bool isLastNode;
  final MarkdownNodeType nodeType;
  final List<MarkdownNodeStable> children;
  final int? headerLevel;
  final String sourcePrefix;
  final MarkdownContentSplitter splitMarkdownContent;
  final void Function(String url)? onLinkClick;
  final Stream<Object>? nestedMarkdownEventStream;

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
      !widget.nodeKey.startsWith('static-node-') &&
      widget.isLastNode &&
      (widget.text.isNotEmpty || widget.children.isNotEmpty) &&
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
    _targetLength = _markdownVisibleLength(widget.text, widget.children);
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
    final targetLength = _markdownVisibleLength(widget.text, widget.children);
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
    final reveal = _RevealSegment(
      revealLength: _enableTypewriter ? _revealValue : _targetLength.toDouble(),
      showCursor: _enableTypewriter,
    );
    if (widget.nodeType == MarkdownNodeType.plainText &&
        widget.text.trim().isEmpty &&
        !widget.children.any(_hasMarkdownNodeContent)) {
      return const SizedBox.shrink();
    }
    switch (widget.nodeType) {
      case MarkdownNodeType.codeBlock:
        final parsed = _parseCodeBlock(widget.text);
        return EnhancedCodeBlock(code: parsed.code, language: parsed.language);
      case MarkdownNodeType.table:
        return EnhancedTableBlock(
          tableText: widget.text,
          textColor: widget.textColor,
          splitMarkdownContent: widget.splitMarkdownContent,
        );
      case MarkdownNodeType.blockQuote:
        return MarkdownBlockQuote(
          child: StreamMarkdownRenderer(
            content: widget.text,
            isStreaming: widget.isStreaming,
            contentStream: widget.nestedMarkdownEventStream,
            rendererId: '${widget.nodeKey}-quote',
            textColor: widget.textColor,
            backgroundColor: widget.backgroundColor,
            selectionRoot: false,
            splitMarkdownContent: widget.splitMarkdownContent,
            onLinkClick: widget.onLinkClick,
          ),
        );
      case MarkdownNodeType.horizontalRule:
        return const MarkdownHorizontalRule();
      case MarkdownNodeType.xmlBlock:
        throw StateError('XML block must be rendered by CustomXmlRenderer');
      case MarkdownNodeType.image:
        return MarkdownImageRenderer(
          imageMarkdown: widget.text.trim(),
          textColor: widget.textColor,
          onOpenLink: widget.onLinkClick,
        );
      case MarkdownNodeType.blockLatex:
        return MarkdownLatexBlock(
          content: widget.text,
          textColor: widget.textColor,
        );
      case MarkdownNodeType.htmlBreak:
        return const SizedBox(height: _markdownParagraphBreakHeight);
      case MarkdownNodeType.header:
        return _MarkdownHeading(
          textKey: '${widget.nodeKey}-text-0',
          text: widget.text,
          color: widget.textColor,
          reveal: reveal,
          children: widget.children,
          headerLevel: widget.headerLevel ?? 1,
          isLastNode: widget.isLastNode,
          onLinkClick: widget.onLinkClick,
        );
      case MarkdownNodeType.orderedList:
      case MarkdownNodeType.unorderedList:
        return _MarkdownLine(
          textKey: '${widget.nodeKey}-text-0',
          text: widget.text,
          color: widget.textColor,
          reveal: reveal,
          children: widget.children,
          listType: widget.nodeType,
          sourcePrefix: widget.sourcePrefix,
          isLastNode: widget.isLastNode,
          onLinkClick: widget.onLinkClick,
        );
      default:
        return _MarkdownParagraph(
          textKey: '${widget.nodeKey}-text-0',
          text: widget.text,
          color: widget.textColor,
          reveal: reveal,
          children: widget.children,
          isLastNode: widget.isLastNode,
          onLinkClick: widget.onLinkClick,
        );
    }
  }
}

class _MarkdownLine extends StatelessWidget {
  const _MarkdownLine({
    required this.textKey,
    required this.text,
    required this.color,
    required this.reveal,
    required this.listType,
    required this.sourcePrefix,
    this.children = const [],
    this.isLastNode = false,
    this.onLinkClick,
  });
  final String textKey;
  final String text;
  final Color color;
  final _RevealSegment reveal;
  final MarkdownNodeType listType;
  final String sourcePrefix;
  final List<MarkdownNodeStable> children;
  final bool isLastNode;
  final void Function(String url)? onLinkClick;

  @override
  Widget build(BuildContext context) {
    final style = Theme.of(
      context,
    ).textTheme.bodyMedium?.copyWith(color: color, height: 1.3);
    var indent = 0.0;
    for (final char in sourcePrefix.codeUnits) {
      if (char == 0x20) {
        indent += 8;
      } else if (char == 0x09) {
        indent += 32;
      } else {
        break;
      }
    }
    return Padding(
      padding: EdgeInsets.only(
        left: indent,
        bottom: isLastNode ? 0 : _markdownListItemBottomPadding,
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (listType == MarkdownNodeType.unorderedList)
            Padding(
              padding: const EdgeInsets.only(top: 8, right: 8),
              child: Container(
                width: 4,
                height: 4,
                decoration: BoxDecoration(
                  color: color.withValues(alpha: .7),
                  shape: BoxShape.circle,
                ),
              ),
            )
          else
            Padding(
              padding: const EdgeInsets.only(right: 4),
              child: Text(
                sourcePrefix.trim(),
                style: style,
                textHeightBehavior: _markdownTextHeightBehavior,
              ),
            ),
          Expanded(
            child: _TypewriterMarkdownRichText(
              key: ValueKey<String>('$textKey-body'),
              text: text,
              color: color,
              revealLength: reveal.revealLength,
              showCursor: reveal.showCursor,
              children: children,
              onLinkClick: onLinkClick,
              style: style,
            ),
          ),
        ],
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
    this.children = const <MarkdownNodeStable>[],
    this.headerLevel,
    this.isLastNode = false,
    this.onLinkClick,
  });

  final String textKey;
  final String text;
  final Color color;
  final _RevealSegment reveal;
  final List<MarkdownNodeStable> children;
  final int? headerLevel;
  final bool isLastNode;
  final void Function(String url)? onLinkClick;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final effectiveLevel = headerLevel ?? 1;
    final style = _markdownHeaderStyle(theme, effectiveLevel)?.copyWith(
      color: color,
      fontWeight: FontWeight.w700,
      height: _markdownCanvasLineHeightMultiplier,
    );
    final topPadding = _markdownHeaderTopPadding(effectiveLevel);
    final bottomPadding = isLastNode
        ? 0.0
        : _markdownHeaderBottomPadding(effectiveLevel);

    return Padding(
      padding: EdgeInsets.only(top: topPadding, bottom: bottomPadding),
      child: _TypewriterMarkdownRichText(
        key: ValueKey<String>(textKey),
        text: text,
        color: color,
        revealLength: reveal.revealLength,
        showCursor: reveal.showCursor,
        children: children,
        onLinkClick: onLinkClick,
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

class _MarkdownParagraph extends StatelessWidget {
  const _MarkdownParagraph({
    required this.textKey,
    required this.text,
    required this.color,
    required this.reveal,
    this.children = const <MarkdownNodeStable>[],
    this.isLastNode = false,
    this.onLinkClick,
  });

  final String textKey;
  final String text;
  final Color color;
  final _RevealSegment reveal;
  final List<MarkdownNodeStable> children;
  final bool isLastNode;
  final void Function(String url)? onLinkClick;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: EdgeInsets.only(
        bottom: isLastNode ? 0 : _markdownParagraphBottomPadding,
      ),
      child: _TypewriterMarkdownRichText(
        key: ValueKey<String>(textKey),
        text: text,
        color: color,
        revealLength: reveal.revealLength,
        showCursor: reveal.showCursor,
        children: children,
        onLinkClick: onLinkClick,
        style: theme.textTheme.bodyMedium?.copyWith(
          color: color,
          height: _markdownCanvasLineHeightMultiplier,
        ),
        splitParagraphs: true,
      ),
    );
  }
}

class _TypewriterMarkdownRichText extends StatefulWidget {
  const _TypewriterMarkdownRichText({
    super.key,
    required this.text,
    required this.color,
    required this.style,
    required this.revealLength,
    required this.showCursor,
    this.children = const <MarkdownNodeStable>[],
    this.onLinkClick,
    this.splitParagraphs = false,
  });

  final String text;
  final Color color;
  final TextStyle? style;
  final bool splitParagraphs;
  final double revealLength;
  final bool showCursor;
  final List<MarkdownNodeStable> children;
  final void Function(String url)? onLinkClick;

  @override
  State<_TypewriterMarkdownRichText> createState() =>
      _TypewriterMarkdownRichTextState();
}

class _TypewriterMarkdownRichTextState
    extends State<_TypewriterMarkdownRichText> {
  final MarkdownLinkGestureOwner _gestures = MarkdownLinkGestureOwner();

  InlineSpan? _span;
  List<_MarkdownTextParagraph> _paragraphs = const [];
  bool? _spanSplitParagraphs;
  List<MarkdownNodeStable>? _spanNodes;
  String? _spanText;
  TextStyle? _spanStyle;
  Color? _spanColor;
  ThemeData? _spanTheme;
  void Function(String)? _spanOnLinkClick;

  @override
  void dispose() {
    _gestures.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    if (_span == null ||
        !identical(_spanNodes, widget.children) ||
        _spanText != widget.text ||
        _spanStyle != widget.style ||
        _spanColor != widget.color ||
        _spanTheme != theme ||
        _spanOnLinkClick != widget.onLinkClick ||
        _spanSplitParagraphs != widget.splitParagraphs) {
      _gestures.beginFrame();
      try {
        _span = widget.children.isEmpty
            ? TextSpan(text: widget.text, style: widget.style)
            : buildMarkdownInlineSpannableFromMarkdownNodes(
                context: context,
                children: widget.children,
                textColor: widget.color,
                baseStyle: widget.style,
                onLinkClick: widget.onLinkClick,
                gestures: _gestures,
              );
        _paragraphs = _markdownTextParagraphs(
          _span!,
          splitParagraphs: widget.splitParagraphs,
        );
        _spanSplitParagraphs = widget.splitParagraphs;
        _spanNodes = widget.children;
        _spanText = widget.text;
        _spanStyle = widget.style;
        _spanColor = widget.color;
        _spanTheme = theme;
        _spanOnLinkClick = widget.onLinkClick;
      } finally {
        _gestures.endFrame();
      }
    }
    if (_paragraphs.isEmpty) return const SizedBox.shrink();
    final pressShield = MessagePressShield.maybeOf(context);

    /// Wraps interactive spans so message dragging does not steal link taps.
    Widget wrapWithInteractiveShield(
      Widget child,
      double maxWidth,
      InlineSpan revealSpan,
    ) {
      if (pressShield == null || !_containsInteractiveSpan(revealSpan)) {
        return child;
      }
      return Listener(
        behavior: HitTestBehavior.translucent,
        onPointerDown: (event) {
          final painter = TextPainter(
            text: revealSpan,
            textDirection: Directionality.of(context),
            textScaler: MediaQuery.textScalerOf(context),
            textHeightBehavior: _markdownTextHeightBehavior,
          )..layout(maxWidth: maxWidth);
          try {
            if (_isInteractiveSpanHit(
              span: revealSpan,
              painter: painter,
              position: event.localPosition,
            )) {
              pressShield.shieldPointer(event.pointer);
            }
          } finally {
            painter.dispose();
          }
        },
        onPointerUp: (event) => pressShield.unshieldPointer(event.pointer),
        onPointerCancel: (event) => pressShield.unshieldPointer(event.pointer),
        child: child,
      );
    }

    Widget buildParagraph(int index) {
      final paragraph = _paragraphs[index];
      final isLast = index == _paragraphs.length - 1;
      final revealSpan = _revealedTypewriterSpan(
        source: paragraph.span,
        revealLength: widget.revealLength - paragraph.revealOffset,
        showCursor:
            widget.showCursor &&
            (index == 0 || widget.revealLength >= paragraph.revealOffset) &&
            (isLast ||
                widget.revealLength < _paragraphs[index + 1].revealOffset),
        baseStyle: widget.style,
      );
      return LayoutBuilder(
        builder: (context, constraints) => wrapWithInteractiveShield(
          Text.rich(
            revealSpan,
            style: widget.style,
            textHeightBehavior: _markdownTextHeightBehavior,
          ),
          constraints.maxWidth,
          revealSpan,
        ),
      );
    }

    if (_paragraphs.length == 1) return buildParagraph(0);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        for (var index = 0; index < _paragraphs.length; index++) ...[
          if (index > 0) const SizedBox(height: _markdownParagraphBreakHeight),
          buildParagraph(index),
        ],
      ],
    );
  }
}

// Android's StaticLayout excludes font padding and applies line spacing only
// between lines. Keep Flutter's first/last line from adding extra block gaps.
const _markdownTextHeightBehavior = TextHeightBehavior(
  applyHeightToFirstAscent: false,
  applyHeightToLastDescent: false,
);

class _MarkdownTextParagraph {
  const _MarkdownTextParagraph(this.span, this.revealOffset);

  final InlineSpan span;
  final int revealOffset;
}

/// Trims block boundary newlines and replaces internal blank lines with gaps.
/// Slice the existing span tree so emphasis, links, and inline math survive.
List<_MarkdownTextParagraph> _markdownTextParagraphs(
  InlineSpan span, {
  required bool splitParagraphs,
}) {
  final text = span.toPlainText(includeSemanticsLabels: false);
  final start = text.length - text.trimLeft().length;
  final end = text.trimRight().length;
  if (start >= end) return const [];
  final paragraphs = <_MarkdownTextParagraph>[];
  var previousEnd = 0;
  var revealOffset = 0;
  void append(int from, int to) {
    if (from >= to) return;
    revealOffset += text.substring(previousEnd, from).characters.length;
    paragraphs.add(
      _MarkdownTextParagraph(
        from == 0 && to == text.length
            ? span
            : _sliceMarkdownSpan(span, from, to),
        revealOffset,
      ),
    );
    revealOffset += text.substring(from, to).characters.length;
    previousEnd = to;
  }

  var paragraphStart = start;
  if (splitParagraphs) {
    for (final separator in RegExp(
      r'\r?\n[ \t\r]*\n(?:[ \t\r]*\n)*',
    ).allMatches(text.substring(start, end))) {
      append(paragraphStart, start + separator.start);
      paragraphStart = start + separator.end;
    }
  }
  append(paragraphStart, end);
  return paragraphs;
}

/// Takes a UTF-16 range without flattening inherited styles or recognizers.
InlineSpan _sliceMarkdownSpan(InlineSpan source, int start, int end) {
  var offset = 0;
  InlineSpan? slice(InlineSpan span) {
    if (span is WidgetSpan) {
      final position = offset++;
      return position >= start && position < end ? span : null;
    }
    if (span is! TextSpan) return null;
    final text = span.text ?? '';
    final textStart = offset;
    offset += text.length;
    final from = (start - textStart).clamp(0, text.length);
    final to = (end - textStart).clamp(from, text.length);
    final children = <InlineSpan>[
      for (final child in span.children ?? const <InlineSpan>[]) ?slice(child),
    ];
    if (from == to && children.isEmpty) return null;
    return TextSpan(
      text: from == to ? null : text.substring(from, to),
      children: children,
      style: span.style,
      recognizer: span.recognizer,
      mouseCursor: span.mouseCursor,
      onEnter: span.onEnter,
      onExit: span.onExit,
      locale: span.locale,
      spellOut: span.spellOut,
    );
  }

  return slice(source) ?? const TextSpan();
}

class _RevealSegment {
  const _RevealSegment({required this.revealLength, required this.showCursor});

  final double revealLength;
  final bool showCursor;
}

/// Tracks reveal progress while reconstructing one RichText span tree.
class _InlineRevealState {
  _InlineRevealState({required this.revealLength, required this.showCursor});

  final double revealLength;
  final bool showCursor;
  var consumedLength = 0;
  var cursorInserted = false;
}

/// Builds a RichText span tree that preserves layout while revealing content.
TextSpan _revealedTypewriterSpan({
  required InlineSpan source,
  required double revealLength,
  required bool showCursor,
  required TextStyle? baseStyle,
}) {
  final plainTextLength = source.toPlainText().characters.length;
  final state = _InlineRevealState(
    revealLength: revealLength.clamp(0, plainTextLength).toDouble(),
    showCursor: showCursor,
  );
  final spans = <InlineSpan>[];
  _appendRevealedInlineSpan(
    spans: spans,
    source: source,
    state: state,
    inheritedStyle: baseStyle,
  );
  _appendRevealCursor(spans, state);
  return TextSpan(children: spans);
}

/// Appends one revealed inline span while preserving source ordering.
void _appendRevealedInlineSpan({
  required List<InlineSpan> spans,
  required InlineSpan source,
  required _InlineRevealState state,
  required TextStyle? inheritedStyle,
  GestureRecognizer? inheritedRecognizer,
}) {
  if (source is TextSpan) {
    final effectiveStyle = _mergeTextStyles(inheritedStyle, source.style);
    final effectiveRecognizer = source.recognizer ?? inheritedRecognizer;
    final text = source.text;
    if (text != null && text.isNotEmpty) {
      _appendRevealedText(
        spans: spans,
        text: text,
        style: effectiveStyle,
        recognizer: effectiveRecognizer,
        state: state,
      );
    }
    final children = source.children;
    if (children != null) {
      for (final child in children) {
        _appendRevealedInlineSpan(
          spans: spans,
          source: child,
          state: state,
          inheritedStyle: effectiveStyle,
          inheritedRecognizer: effectiveRecognizer,
        );
      }
    }
    return;
  }
  if (source is WidgetSpan) {
    _appendRevealedWidgetSpan(spans: spans, source: source, state: state);
    return;
  }
  spans.add(source);
}

/// Appends revealed, partial, and hidden text spans for one source string.
void _appendRevealedText({
  required List<InlineSpan> spans,
  required String text,
  required TextStyle? style,
  required GestureRecognizer? recognizer,
  required _InlineRevealState state,
}) {
  final segmentStart = state.consumedLength;
  final textCharacters = text.characters;
  final textLength = textCharacters.length;
  final segmentEnd = segmentStart + textLength;
  final revealLength = state.revealLength;
  if (revealLength >= segmentEnd) {
    spans.add(TextSpan(text: text, style: style, recognizer: recognizer));
    state.consumedLength = segmentEnd;
    _appendRevealCursor(spans, state);
    return;
  }
  if (revealLength <= segmentStart) {
    _appendRevealCursor(spans, state);
    spans.add(TextSpan(text: text, style: _hiddenTextStyle(style)));
    state.consumedLength = segmentEnd;
    return;
  }

  final visibleLength = (revealLength.floor() - segmentStart)
      .clamp(0, textLength)
      .toInt();
  final partialAmount = (revealLength - revealLength.floor())
      .clamp(0, 1)
      .toDouble();
  if (visibleLength > 0) {
    spans.add(
      TextSpan(
        text: textCharacters.take(visibleLength).toString(),
        style: style,
        recognizer: recognizer,
      ),
    );
  }
  var hiddenStart = visibleLength;
  if (partialAmount > 0 && visibleLength < textLength) {
    spans.add(
      TextSpan(
        text: textCharacters.skip(visibleLength).take(1).toString(),
        style: _partialTextStyle(style, partialAmount),
      ),
    );
    hiddenStart = visibleLength + 1;
  }
  state.consumedLength = segmentStart + hiddenStart;
  _appendRevealCursor(spans, state);
  if (hiddenStart < textLength) {
    spans.add(
      TextSpan(
        text: textCharacters.skip(hiddenStart).toString(),
        style: _hiddenTextStyle(style),
      ),
    );
  }
  state.consumedLength = segmentEnd;
}

/// Appends one WidgetSpan with reveal opacity while preserving its placeholder.
void _appendRevealedWidgetSpan({
  required List<InlineSpan> spans,
  required WidgetSpan source,
  required _InlineRevealState state,
}) {
  final segmentStart = state.consumedLength;
  final revealAmount = (state.revealLength - segmentStart)
      .clamp(0, 1)
      .toDouble();
  _appendRevealCursor(spans, state);
  spans.add(
    WidgetSpan(
      alignment: source.alignment,
      baseline: source.baseline,
      style: source.style,
      child: Opacity(opacity: revealAmount, child: source.child),
    ),
  );
  state.consumedLength = segmentStart + 1;
  _appendRevealCursor(spans, state);
}

/// Appends the streaming cursor at the current reveal boundary.
void _appendRevealCursor(List<InlineSpan> spans, _InlineRevealState state) {
  if (!state.showCursor ||
      state.cursorInserted ||
      state.revealLength > state.consumedLength) {
    return;
  }
  state.cursorInserted = true;
  spans.add(_streamingCursorSpan());
}

/// Builds an inline cursor with finite layout constraints.
WidgetSpan _streamingCursorSpan() {
  return const WidgetSpan(
    alignment: PlaceholderAlignment.middle,
    child: SizedBox(width: 7, height: 16, child: StreamingCursor()),
  );
}

/// Merges nested TextSpan styles for reveal-specific spans.
TextStyle? _mergeTextStyles(TextStyle? inheritedStyle, TextStyle? style) {
  if (inheritedStyle == null) {
    return style;
  }
  if (style == null) {
    return inheritedStyle;
  }
  return inheritedStyle.merge(style);
}

/// Returns a style that keeps layout metrics while hiding glyph paint.
TextStyle? _hiddenTextStyle(TextStyle? style) {
  return _textStyleWithRevealOpacity(style, 0);
}

/// Returns a style that paints a partially revealed glyph.
TextStyle? _partialTextStyle(TextStyle? style, double amount) {
  return _textStyleWithRevealOpacity(style, amount);
}

/// Returns a style with adjusted text visibility and preserved metrics.
TextStyle? _textStyleWithRevealOpacity(TextStyle? style, double amount) {
  final opacity = amount.clamp(0, 1).toDouble();
  final source = style ?? const TextStyle();
  final color = source.color ?? Colors.transparent;
  return source.copyWith(
    color: color.withValues(alpha: opacity),
    backgroundColor: opacity == 0 ? Colors.transparent : source.backgroundColor,
    decorationColor: opacity == 0 ? Colors.transparent : source.decorationColor,
    shadows: opacity == 0 ? const <Shadow>[] : source.shadows,
  );
}

/// Reports whether a span tree has an active gesture recognizer.
bool _containsInteractiveSpan(InlineSpan span) {
  if (span is TextSpan) {
    if (span.recognizer != null) {
      return true;
    }
    final children = span.children;
    if (children == null) {
      return false;
    }
    return children.any(_containsInteractiveSpan);
  }
  if (span is WidgetSpan) {
    return false;
  }
  return false;
}

/// Reports whether a pointer position lands on an interactive text span.
bool _isInteractiveSpanHit({
  required InlineSpan span,
  required TextPainter painter,
  required Offset position,
}) {
  final ui.GlyphInfo? glyph = painter.getClosestGlyphForOffset(position);
  if (glyph == null || !glyph.graphemeClusterLayoutBounds.contains(position)) {
    return false;
  }
  final hitSpan = span.getSpanForPosition(
    TextPosition(offset: glyph.graphemeClusterCodeUnitRange.start),
  );
  return hitSpan is TextSpan && hitSpan.recognizer != null;
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
