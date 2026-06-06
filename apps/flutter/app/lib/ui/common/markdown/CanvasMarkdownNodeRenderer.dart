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
    this.onLinkClick,
  });

  final String nodeKey;
  final MarkdownNodeStable node;
  final Color textColor;
  final Color backgroundColor;
  final bool isLastNode;
  final void Function(String url)? onLinkClick;

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
      onLinkClick: onLinkClick,
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
  final codeLines = lines
      .skipWhile((line) => line.startsWith('```'))
      .toList();
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
    this.onLinkClick,
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
  final void Function(String url)? onLinkClick;

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

    Widget wrapNode(Widget child) {
      if (widget.isStreaming) {
        return child;
      }
      return SelectionArea(child: child);
    }

    _RevealSegment directReveal() {
      return revealSegmentFor(_typewriterLineLength(widget.text));
    }
    switch (widget.nodeType) {
      case MarkdownNodeType.codeBlock:
        final parsed = _parseCodeBlock(widget.text);
        return wrapNode(
          EnhancedCodeBlock(code: parsed.code, language: parsed.language),
        );
      case MarkdownNodeType.table:
        return wrapNode(
          EnhancedTableBlock(tableText: widget.text, textColor: widget.textColor),
        );
      case MarkdownNodeType.blockQuote:
        return wrapNode(
          MarkdownBlockQuote(
            content: widget.text,
            textColor: widget.textColor,
            backgroundColor: widget.backgroundColor,
            isStreaming: widget.isStreaming,
          ),
        );
      case MarkdownNodeType.horizontalRule:
        return const MarkdownHorizontalRule();
      case MarkdownNodeType.xmlBlock:
        throw StateError('XML block must be rendered by CustomXmlRenderer');
      case MarkdownNodeType.image:
        return wrapNode(
          MarkdownImageRenderer(
            imageMarkdown: widget.text.trim(),
            textColor: widget.textColor,
          ),
        );
      case MarkdownNodeType.blockLatex:
        return wrapNode(
          MarkdownLatexBlock(content: widget.text, textColor: widget.textColor),
        );
      case MarkdownNodeType.htmlBreak:
        return const SizedBox(height: _markdownParagraphBreakHeight);
      case MarkdownNodeType.header:
        return wrapNode(
          _MarkdownHeading(
            textKey: '${widget.nodeKey}-text-0',
            text: widget.text,
            color: widget.textColor,
            reveal: directReveal(),
            children: widget.children,
            headerLevel: widget.headerLevel,
            isLastNode: widget.isLastNode,
            onLinkClick: widget.onLinkClick,
          ),
        );
      case MarkdownNodeType.orderedList:
      case MarkdownNodeType.unorderedList:
        return wrapNode(
          _MarkdownLine(
            textKey: '${widget.nodeKey}-text-0',
            text: widget.text,
            color: widget.textColor,
            reveal: directReveal(),
            children: widget.children,
            isLastNode: widget.isLastNode,
            onLinkClick: widget.onLinkClick,
          ),
        );
      case MarkdownNodeType.plainText:
        break;
      case MarkdownNodeType.bold:
      case MarkdownNodeType.italic:
      case MarkdownNodeType.inlineCode:
      case MarkdownNodeType.link:
      case MarkdownNodeType.strikethrough:
      case MarkdownNodeType.underline:
      case MarkdownNodeType.inlineLatex:
        return wrapNode(
          _MarkdownParagraph(
            textKey: '${widget.nodeKey}-text-0',
            text: widget.text,
            color: widget.textColor,
            reveal: directReveal(),
            children: widget.children,
            isLastNode: widget.isLastNode,
            onLinkClick: widget.onLinkClick,
          ),
        );
    }

    if (widget.children.isNotEmpty) {
      return wrapNode(
        _MarkdownParagraph(
          textKey: '${widget.nodeKey}-text-0',
          text: widget.text,
          color: widget.textColor,
          reveal: revealSegmentFor(widget.text.length),
          children: widget.children,
          isLastNode: widget.isLastNode,
          onLinkClick: widget.onLinkClick,
        ),
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
          onLinkClick: widget.onLinkClick,
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
            onLinkClick: widget.onLinkClick,
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
        isLastNode: isLastNode,
        onLinkClick: onLinkClick,
      );
    }
    if (_isBulletLine(text)) {
      final body = text.substring(2);
      final contentChildren = _childrenWithFirstContent(
        children,
        (value) => value.replaceFirst(RegExp(r'^\s*[-*+]\s+'), ''),
      );
      return Padding(
        padding: EdgeInsets.only(
          bottom: isLastNode ? 0 : _markdownLineBlockBottomPadding,
        ),
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
                children: contentChildren,
                onLinkClick: onLinkClick,
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
      final contentChildren = _childrenWithFirstContent(
        children,
        (value) => value.replaceFirst(RegExp(r'^\s*\d+\.\s*'), ''),
      );
      return Padding(
        padding: EdgeInsets.only(
          bottom: isLastNode ? 0 : _markdownLineBlockBottomPadding,
        ),
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
                children: contentChildren,
                onLinkClick: onLinkClick,
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
      padding: EdgeInsets.only(
        bottom: isLastNode ? 0 : _markdownLineBlockBottomPadding,
      ),
      child: _TypewriterMarkdownRichText(
        key: ValueKey<String>(textKey),
        text: text,
        color: color,
        revealLength: reveal.revealLength,
        showCursor: reveal.showCursor,
        children: children,
        onLinkClick: onLinkClick,
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
    final effectiveLevel = headerLevel ?? _determineHeaderLevel(text);
    final headingText = _markdownHeaderText(text);
    final style = _markdownHeaderStyle(theme, effectiveLevel)?.copyWith(
      color: color,
      fontWeight: FontWeight.w700,
      height: _markdownCanvasLineHeightMultiplier,
    );
    final topPadding = _markdownHeaderTopPadding(effectiveLevel);
    final bottomPadding = isLastNode
        ? 0.0
        : _markdownHeaderBottomPadding(effectiveLevel);
    final contentChildren = _childrenWithFirstContent(
      children,
      (value) => value.replaceFirst(RegExp(r'^\s*#+\s*'), ''),
    );

    return Padding(
      padding: EdgeInsets.only(top: topPadding, bottom: bottomPadding),
      child: _TypewriterMarkdownRichText(
        key: ValueKey<String>(textKey),
        text: headingText,
        color: color,
        revealLength: reveal.revealLength,
        showCursor: reveal.showCursor,
        children: contentChildren,
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

String _markdownHeaderText(String text) {
  return text.replaceFirst(RegExp(r'^\s*#+\s*'), '').trim();
}

List<MarkdownNodeStable> _childrenWithFirstContent(
  List<MarkdownNodeStable> children,
  String Function(String value) transformFirstContent,
) {
  if (children.isEmpty) {
    return children;
  }
  final updated = <MarkdownNodeStable>[...children];
  final first = updated.first;
  updated[0] = MarkdownNodeStable(
    type: first.type,
    content: transformFirstContent(first.content),
    isStreaming: first.isStreaming,
    stableKey: first.stableKey,
    children: first.children,
    headerLevel: first.headerLevel,
  );
  return updated;
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
        bottom: isLastNode ? 0 : _markdownLineBlockBottomPadding,
      ),
      child: _TypewriterMarkdownRichText(
        key: ValueKey<String>(textKey),
        text: text,
        color: color,
        revealLength: reveal.revealLength,
        showCursor: reveal.showCursor,
        children: children,
        onLinkClick: onLinkClick,
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
    this.children = const <MarkdownNodeStable>[],
    this.onLinkClick,
  });

  final String text;
  final Color color;
  final TextStyle? style;
  final double revealLength;
  final bool showCursor;
  final List<MarkdownNodeStable> children;
  final void Function(String url)? onLinkClick;

  @override
  Widget build(BuildContext context) {
    final span = buildMarkdownInlineSpannableFromText(
      context: context,
      text: text,
      textColor: color,
      baseStyle: style,
      onLinkClick: onLinkClick,
    );
    final richSpan = children.isNotEmpty
        ? buildMarkdownInlineSpannableFromMarkdownNodes(
            context: context,
            children: children,
            textColor: color,
            baseStyle: style,
            onLinkClick: onLinkClick,
          )
        : span;
    if (revealLength >= text.length && !showCursor) {
      return Text.rich(richSpan, style: style);
    }
    if (_containsWidgetSpan(richSpan)) {
      return Text.rich(richSpan, style: style);
    }
    return LayoutBuilder(
      builder: (context, constraints) {
        final textDirection = Directionality.of(context);
        final painter = TextPainter(
          text: richSpan,
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
                  span: richSpan,
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
