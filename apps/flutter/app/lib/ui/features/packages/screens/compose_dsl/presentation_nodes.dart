// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Renders generic native presentation primitives, without plugin business logic.
extension _ComposePresentationNodes on _ComposeProgramExecutorState {
  /// Preserves native typography, scrolling, tooltips and Material interactions.
  Widget _presentationNode(BuildContext context, String type) {
    switch (type) {
      case 'CodeText':
        return _codeText(context);
      case 'RichText':
        final spans = props['spans'];
        if (spans is! List) {
          throw const FormatException('RichText.spans must be an array');
        }
        return Text.rich(
          TextSpan(
            children: [
              for (final raw in spans)
                if (raw is Map)
                  TextSpan(
                    text: _string(raw['text']),
                    style: _inlineTextStyle(context, _stringMap(raw)),
                  )
                else
                  throw const FormatException('RichText spans must be objects'),
            ],
          ),
          style: _inlineTextStyle(context, props),
          maxLines: _int(props['maxLines']),
          softWrap: props['softWrap'] as bool?,
          overflow: _string(props['overflow']) == 'ellipsis'
              ? TextOverflow.ellipsis
              : TextOverflow.clip,
        );
      case 'Tooltip':
        return Tooltip(
          message: _string(props['message']),
          waitDuration: Duration(milliseconds: _int(props['waitMillis']) ?? 0),
          child: _decoratorChild(),
        );
      case 'ScrollArea':
        return _ComposeScrollArea(
          axis: _string(props['axis']) == 'horizontal'
              ? Axis.horizontal
              : Axis.vertical,
          thumbVisibility: _bool(props['thumbVisibility']),
          trackVisibility: _bool(props['trackVisibility']),
          child: _decoratorChild(),
        );
      case 'MenuAnchor':
        final entries = node.slots['content'] ?? node.children;
        final metadata = props['items'] as List?;
        return PopupMenuButton<int>(
          tooltip: _string(props['tooltip']),
          padding: EdgeInsets.zero,
          onSelected: (index) => _invokeAction(props['onSelect'], index),
          itemBuilder: (context) => [
            for (var index = 0; index < entries.length; index++)
              if (metadata != null &&
                  _bool(_stringMap(metadata[index])['divider']))
                PopupMenuDivider(
                  height: _number(_stringMap(metadata[index])['height']) ?? 6,
                )
              else
                PopupMenuItem<int>(
                  value: index,
                  enabled:
                      metadata == null ||
                      _stringMap(metadata[index])['enabled'] != false,
                  height:
                      (metadata == null
                          ? null
                          : _number(_stringMap(metadata[index])['height'])) ??
                      _number(props['itemHeight']) ??
                      32,
                  child: _chipInlineChild(
                    entries[index],
                    '$nodePath:menu/$index',
                  ),
                ),
          ],
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: _slotChildren('anchor'),
          ),
        );
      case 'InkWell':
        return Material(
          color: Colors.transparent,
          child: InkWell(
            onTap: props['onClick'] == null
                ? null
                : () => _invokeAction(props['onClick']),
            borderRadius: _borderRadius(props['shape']),
            child: _decoratorChild(),
          ),
        );
      case 'Svg':
        return SvgPicture.string(
          _string(props['source']),
          width: _number(props['width']),
          height: _number(props['height']),
          colorFilter: props['tint'] == null
              ? null
              : ColorFilter.mode(
                  _color(context, props['tint'])!,
                  BlendMode.srcIn,
                ),
        );
      default:
        throw FormatException('Unknown presentation node: $type');
    }
  }

  /// Applies a generic syntax grammar and caller-owned intervals to literal text.
  Widget _codeText(BuildContext context) {
    final text = _string(props['text']);
    if (text.isEmpty) return const SizedBox();
    final language = WorkspaceSyntaxLanguage.forPath(
      _string(props['filePath']),
    );
    final tokens = <WorkspaceSyntaxToken>[];
    if (language != WorkspaceSyntaxLanguage.plainText &&
        language != WorkspaceSyntaxLanguage.diff) {
      final lexer = WorkspaceSyntaxLexer(language);
      var state = WorkspaceSyntaxState.empty;
      var offset = 0;
      for (final line in text.split('\n')) {
        final result = lexer.scan(line, state, tokens: true);
        for (final token in result.tokens) {
          tokens.add(
            WorkspaceSyntaxToken(
              offset + token.start,
              offset + token.end,
              token.kind,
            ),
          );
        }
        state = result.state;
        offset += line.length + 1;
      }
    }
    final palette = WorkspaceSyntaxPalette(Theme.of(context).brightness);
    // Source intervals are ordered; walk them once for whole-file previews.
    var tokenIndex = 0;
    Color? syntaxColor(int start, int end) {
      while (tokenIndex < tokens.length && tokens[tokenIndex].end <= start) {
        tokenIndex++;
      }
      if (tokenIndex == tokens.length) return null;
      final token = tokens[tokenIndex];
      return token.start <= start && token.end >= end
          ? palette.color(token.kind)
          : null;
    }

    final emphasis = props['emphasis'];
    final ranges = <({int start, int end, Map<String, dynamic> style})>[];
    if (emphasis is List) {
      for (final raw in emphasis) {
        if (raw is! Map)
          throw const FormatException('CodeText emphasis must be objects');
        final range = _stringMap(raw);
        final start = _int(range['start']);
        final end = _int(range['end']);
        if (start == null ||
            end == null ||
            start < 0 ||
            end < start ||
            end > text.length) {
          throw const FormatException(
            'CodeText emphasis is outside the source text',
          );
        }
        ranges.add((start: start, end: end, style: range));
      }
    }
    final cuts = <int>{
      0,
      text.length,
      for (final token in tokens) ...[token.start, token.end],
      for (final range in ranges) ...[range.start, range.end],
    }.toList()..sort();
    final baseStyle = _inlineTextStyle(
      context,
      props,
    ).copyWith(fontFamilyFallback: const ['Cascadia Mono', 'monospace']);
    return Align(
      alignment: Alignment.centerLeft,
      child: Text.rich(
        TextSpan(
          style: baseStyle,
          children: [
            for (var index = 0; index < cuts.length - 1; index++)
              if (cuts[index] < cuts[index + 1])
                TextSpan(
                  text: text.substring(cuts[index], cuts[index + 1]),
                  style: baseStyle.copyWith(
                    color: syntaxColor(cuts[index], cuts[index + 1]),
                    backgroundColor: ranges
                        .where(
                          (range) =>
                              range.start <= cuts[index] &&
                              range.end >= cuts[index + 1],
                        )
                        .map((range) => _color(context, range.style['color']))
                        .firstOrNull,
                    fontWeight: ranges
                        .where(
                          (range) =>
                              range.start <= cuts[index] &&
                              range.end >= cuts[index + 1],
                        )
                        .map(
                          (range) =>
                              _fontWeight(_string(range.style['fontWeight'])),
                        )
                        .firstOrNull,
                  ),
                ),
          ],
        ),
        maxLines: _int(props['maxLines']),
        softWrap: props['softWrap'] as bool?,
      ),
    );
  }
}

/// Owns an explicit scroll controller so nested areas never share a primary one.
class _ComposeScrollArea extends StatefulWidget {
  /// Creates one bounded scroll viewport and its matching native scrollbar.
  const _ComposeScrollArea({
    required this.axis,
    required this.thumbVisibility,
    required this.trackVisibility,
    required this.child,
  });

  final Axis axis;
  final bool thumbVisibility;
  final bool trackVisibility;
  final Widget child;

  /// Keeps native scrolling state alive across plugin program updates.
  @override
  State<_ComposeScrollArea> createState() => _ComposeScrollAreaState();
}

class _ComposeScrollAreaState extends State<_ComposeScrollArea> {
  final _controller = ScrollController();

  /// Releases the exact controller used by the scrollbar and its viewport.
  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  /// Uses native scrolling and scrollbar hit testing on every platform.
  @override
  Widget build(BuildContext context) => Scrollbar(
    controller: _controller,
    thumbVisibility: widget.thumbVisibility,
    trackVisibility: widget.trackVisibility,
    child: SingleChildScrollView(
      controller: _controller,
      primary: false,
      scrollDirection: widget.axis,
      child: widget.child,
    ),
  );
}
