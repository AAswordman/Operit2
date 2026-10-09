// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Retains Flutter flex sizing while honoring Compose child alignment and z-index.
class _ComposeFlex extends Flex {
  /// Creates a scoped row or column using Flutter's existing flex parent data.
  const _ComposeFlex({
    required super.direction,
    required this.nodes,
    required super.children,
    super.mainAxisSize,
    super.mainAxisAlignment,
    super.crossAxisAlignment,
    super.spacing,
  });

  final List<_ComposeDslNode> nodes;

  /// Creates a flex layout with per-child Compose drawing metadata.
  @override
  RenderFlex createRenderObject(BuildContext context) => _ComposeRenderFlex(
    nodes: nodes,
    direction: direction,
    mainAxisSize: mainAxisSize,
    mainAxisAlignment: mainAxisAlignment,
    crossAxisAlignment: crossAxisAlignment,
    textDirection: Directionality.of(context),
    spacing: spacing,
  );

  /// Updates both Flutter flex properties and Compose child metadata.
  @override
  void updateRenderObject(
    BuildContext context,
    _ComposeRenderFlex renderObject,
  ) {
    super.updateRenderObject(context, renderObject);
    renderObject.nodes = nodes;
    renderObject.markNeedsLayout();
  }
}

class _ComposeRenderFlex extends RenderFlex {
  /// Configures standard flex measurement with plugin child metadata.
  _ComposeRenderFlex({
    required List<_ComposeDslNode> nodes,
    required super.direction,
    required super.mainAxisSize,
    required super.mainAxisAlignment,
    required super.crossAxisAlignment,
    required super.textDirection,
    super.spacing,
  }) : _nodes = nodes {
    for (final node in _nodes) { node.addListener(_nodeChanged); }
  }

  List<_ComposeDslNode> _nodes;
  /// Exposes child metadata used by layout and paint.
  List<_ComposeDslNode> get nodes => _nodes;

  /// Rebinds child metadata listeners when the flex membership changes.
  set nodes(List<_ComposeDslNode> value) {
    for (final node in _nodes) { node.removeListener(_nodeChanged); }
    _nodes = value;
    for (final node in _nodes) { node.addListener(_nodeChanged); }
    markNeedsLayout();
  }

  /// Invalidates layout and drawing when retained child metadata changes.
  void _nodeChanged() { markNeedsLayout(); markNeedsPaint(); }

  /// Releases retained child subscriptions with the render object.
  @override
  void dispose() {
    for (final node in _nodes) { node.removeListener(_nodeChanged); }
    super.dispose();
  }

  /// Applies explicit child alignment after Flutter distributes flex space.
  @override
  void performLayout() {
    super.performLayout();
    var child = firstChild;
    var index = 0;
    while (child != null) {
      final props = nodes[index].props;
      final args =
          _modifierOpByToken(props['modifier'], 'align')?['args']
              as List<Object?>?;
      final rawAlign = props['align'] ?? args?.firstOrNull;
      if (rawAlign != null) {
        final token = _normalizeToken(_string(rawAlign));
        var fraction = switch (token) {
          'center' || 'centerhorizontally' || 'centervertically' => 0.5,
          'end' || 'right' || 'bottom' => 1.0,
          'start' || 'left' || 'top' => 0.0,
          _ => throw FormatException(
            'Invalid scoped Compose alignment: $rawAlign',
          ),
        };
        if (direction == Axis.vertical &&
            textDirection == TextDirection.rtl &&
            (token == 'start' || token == 'end')) {
          fraction = 1 - fraction;
        }
        final data = child.parentData! as FlexParentData;
        data.offset = direction == Axis.horizontal
            ? Offset(
                data.offset.dx,
                (size.height - child.size.height) * fraction,
              )
            : Offset(
                (size.width - child.size.width) * fraction,
                data.offset.dy,
              );
      }
      child = childAfter(child);
      index++;
    }
  }

  /// Sorts draw order without disturbing measured positions or keyed elements.
  List<RenderBox> _paintOrder() {
    final entries = <({RenderBox child, double z, int index})>[];
    var child = firstChild;
    var index = 0;
    while (child != null) {
      final props = nodes[index].props;
      final args =
          _modifierOpByToken(props['modifier'], 'zindex')?['args']
              as List<Object?>?;
      entries.add((
        child: child,
        z: _number(props['zIndex'] ?? args?.firstOrNull) ?? 0,
        index: index,
      ));
      child = childAfter(child);
      index++;
    }
    entries.sort((a, b) {
      final order = a.z.compareTo(b.z);
      return order == 0 ? a.index.compareTo(b.index) : order;
    });
    return entries.map((entry) => entry.child).toList(growable: false);
  }

  /// Paints children in stable ascending z-index order.
  @override
  void paint(PaintingContext context, Offset offset) {
    for (final child in _paintOrder()) {
      context.paintChild(
        child,
        offset + (child.parentData! as FlexParentData).offset,
      );
    }
  }

  /// Routes input to the topmost overlapping child first.
  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) {
    for (final child in _paintOrder().reversed) {
      if (result.addWithPaintOffset(
        offset: (child.parentData! as FlexParentData).offset,
        position: position,
        hitTest: (result, transformed) =>
            child.hitTest(result, position: transformed),
      )) {
        return true;
      }
    }
    return false;
  }
}
