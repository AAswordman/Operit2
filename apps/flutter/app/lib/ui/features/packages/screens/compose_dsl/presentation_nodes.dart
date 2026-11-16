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
      case 'CodeEditor':
        return CodeEditor(
          identity: props['key'] ?? props['filePath'],
          text: _string(props['text']),
          filePath: _string(props['filePath']),
          readOnly: true,
          wrapToViewport:
              props['wrapToViewport'] == null || _bool(props['wrapToViewport']),
          showLineNumbers:
              props['showLineNumbers'] == null ||
              _bool(props['showLineNumbers']),
          style: _inlineTextStyle(context, props).copyWith(
            fontFamilyFallback: const ['Cascadia Mono', 'monospace'],
            fontFamily: _string(props['fontFamily']).isEmpty
                ? 'monospace'
                : _string(props['fontFamily']),
          ),
          scale: _number(props['scale']),
          minScale: _number(props['minScale']) ?? 0.6,
          maxScale: _number(props['maxScale']) ?? 2.5,
          panEnabled: props['panEnabled'] == null || _bool(props['panEnabled']),
          contentWidth: _number(props['contentWidth']),
          contentColumns: [
            for (final raw in props['contentColumns'] as List? ?? const [])
              CodeEditorColumn(
                text: _string((raw as Map)['text']),
                fraction: _number(raw['fraction']) ?? 1,
                horizontalPadding: _number(raw['horizontalPadding']) ?? 0,
              ),
          ],
          contentBuilder:
              node.children.isEmpty && (node.slots['content']?.isEmpty ?? true)
              ? null
              : (context) => _decoratorChild(),
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
    final emphasis = props['emphasis'];
    final ranges = <CodeTextHighlight>[];
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
        ranges.add(
          CodeTextHighlight(
            start,
            end,
            TextStyle(
              backgroundColor: _color(context, range['color']),
              fontWeight: _fontWeight(_string(range['fontWeight'])),
            ),
          ),
        );
      }
    }
    final baseStyle = _inlineTextStyle(
      context,
      props,
    ).copyWith(fontFamilyFallback: const ['Cascadia Mono', 'monospace']);
    return Align(
      alignment: Alignment.centerLeft,
      widthFactor: 1,
      heightFactor: 1,
      child: CodeEditorText(
        text: text,
        filePath: _string(props['filePath']),
        style: baseStyle,
        highlights: ranges,
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
