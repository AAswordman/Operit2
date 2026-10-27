// ignore_for_file: file_names

import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

class _RevealParentData extends ContainerBoxParentData<RenderBox> {}

/// Reveals a fully laid-out paragraph without changing its text or semantics on animation frames.
class MarkdownTextReveal extends MultiChildRenderObjectWidget {
  MarkdownTextReveal({
    super.key,
    required this.animation,
    required this.text,
    required this.revealOffset,
    required this.nextRevealOffset,
    required this.showCursor,
    required Widget paragraph,
    required Widget cursor,
  }) : super(
         children: [
           paragraph,
           ExcludeSemantics(child: cursor),
         ],
       );

  final Animation<double> animation;
  final String text;
  final int revealOffset;
  final int? nextRevealOffset;
  final bool showCursor;

  @override
  RenderObject createRenderObject(BuildContext context) =>
      RenderMarkdownTextReveal(
        animation: animation,
        text: text,
        revealOffset: revealOffset,
        nextRevealOffset: nextRevealOffset,
        showCursor: showCursor,
      );

  @override
  void updateRenderObject(
    BuildContext context,
    RenderMarkdownTextReveal renderObject,
  ) {
    renderObject.update(
      animation,
      text,
      revealOffset,
      nextRevealOffset,
      showCursor,
    );
  }
}

/// Keeps paragraph layout, hit testing and selection independent of reveal progress.
class RenderMarkdownTextReveal extends RenderBox
    with
        ContainerRenderObjectMixin<
          RenderBox,
          ContainerBoxParentData<RenderBox>
        > {
  RenderMarkdownTextReveal({
    required Animation<double> animation,
    required String text,
    required int revealOffset,
    required int? nextRevealOffset,
    required bool showCursor,
  }) : _animation = animation,
       _text = text,
       _revealOffset = revealOffset,
       _nextRevealOffset = nextRevealOffset,
       _showCursor = showCursor {
    _updateCharacterOffsets();
  }

  Animation<double> _animation;
  String _text;
  int _revealOffset;
  int? _nextRevealOffset;
  bool _showCursor;
  List<int> _characterOffsets = const [];
  RenderParagraph? _paragraph;

  void _updateCharacterOffsets() {
    var offset = 0;
    _characterOffsets = [
      0,
      for (final character in _text.characters) offset += character.length,
    ];
  }

  void update(
    Animation<double> animation,
    String text,
    int revealOffset,
    int? nextRevealOffset,
    bool showCursor,
  ) {
    if (!identical(animation, _animation)) {
      if (attached) _animation.removeListener(markNeedsPaint);
      _animation = animation;
      if (attached) _animation.addListener(markNeedsPaint);
    }
    if (text != _text) {
      _text = text;
      _updateCharacterOffsets();
    }
    _revealOffset = revealOffset;
    _nextRevealOffset = nextRevealOffset;
    _showCursor = showCursor;
    markNeedsPaint();
  }

  @override
  void setupParentData(RenderBox child) {
    if (child.parentData is! ContainerBoxParentData<RenderBox>) {
      child.parentData = _RevealParentData();
    }
  }

  @override
  void attach(PipelineOwner owner) {
    super.attach(owner);
    _animation.addListener(markNeedsPaint);
  }

  @override
  void detach() {
    _animation.removeListener(markNeedsPaint);
    super.detach();
  }

  @override
  Size computeDryLayout(BoxConstraints constraints) =>
      constraints.constrain(firstChild!.getDryLayout(constraints));

  @override
  double? computeDistanceToActualBaseline(TextBaseline baseline) =>
      firstChild!.getDistanceToActualBaseline(baseline);

  @override
  void performLayout() {
    firstChild!.layout(constraints, parentUsesSize: true);
    lastChild!.layout(const BoxConstraints.tightFor(width: 7, height: 16));
    size = constraints.constrain(firstChild!.size);
    _paragraph = _findParagraph(firstChild!);
  }

  static RenderParagraph? _findParagraph(RenderObject object) {
    if (object is RenderParagraph) return object;
    RenderParagraph? result;
    object.visitChildren((child) => result ??= _findParagraph(child));
    return result;
  }

  double get _progress => (_animation.value - _revealOffset)
      .clamp(0, _characterOffsets.length - 1)
      .toDouble();

  bool get _ownsCursor =>
      _showCursor &&
      (_revealOffset == 0 || _animation.value >= _revealOffset) &&
      (_nextRevealOffset == null || _animation.value < _nextRevealOffset!);

  Path _rangePath(int start, int end) {
    final path = Path();
    final paragraph = _paragraph;
    if (paragraph == null || start == end) return path;
    for (final box in paragraph.getBoxesForSelection(
      TextSelection(baseOffset: start, extentOffset: end),
      boxHeightStyle: ui.BoxHeightStyle.max,
    )) {
      path.addRect(box.toRect().inflate(0.5));
    }
    return path;
  }

  Offset get _cursorOffset {
    final paragraph = _paragraph;
    if (paragraph == null) return Offset.zero;
    final progress = _progress;
    final index = progress.floor();
    final next = (index + 1).clamp(0, _characterOffsets.length - 1);
    final position = TextPosition(offset: _characterOffsets[index]);
    final from = paragraph.getOffsetForCaret(position, Rect.zero);
    final to = paragraph.getOffsetForCaret(
      TextPosition(offset: _characterOffsets[next]),
      Rect.zero,
    );
    final caret = Offset.lerp(from, to, progress - index)!;
    final lineHeight = paragraph.getFullHeightForCaret(position);
    return caret + Offset(0, (lineHeight - lastChild!.size.height) / 2);
  }

  @override
  void paint(PaintingContext context, Offset offset) {
    final progress = _progress;
    final complete = progress >= _characterOffsets.length - 1;
    if (complete || _paragraph == null) {
      context.paintChild(firstChild!, offset);
    } else {
      final index = progress.floor();
      if (index > 0) {
        context.pushClipPath(
          needsCompositing,
          offset,
          Offset.zero & size,
          _rangePath(0, _characterOffsets[index]),
          (context, offset) => context.paintChild(firstChild!, offset),
        );
      }
      final alpha = ((progress - index) * 255).round();
      if (alpha > 0) {
        context.pushClipPath(
          needsCompositing,
          offset,
          Offset.zero & size,
          _rangePath(_characterOffsets[index], _characterOffsets[index + 1]),
          (context, offset) => context.pushOpacity(
            offset,
            alpha,
            (context, offset) => context.paintChild(firstChild!, offset),
          ),
        );
      }
    }
    if (_ownsCursor) context.paintChild(lastChild!, offset + _cursorOffset);
  }

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) {
    if (_progress < _characterOffsets.length - 1 &&
        !_rangePath(
          0,
          _characterOffsets[_progress.floor()],
        ).contains(position)) {
      return false;
    }
    return firstChild!.hitTest(result, position: position);
  }

  @override
  void applyPaintTransform(RenderBox child, Matrix4 transform) {
    if (child == lastChild) {
      final offset = _cursorOffset;
      transform.translateByDouble(offset.dx, offset.dy, 0, 1);
    }
  }

  @override
  void visitChildrenForSemantics(RenderObjectVisitor visitor) =>
      visitor(firstChild!);
}
