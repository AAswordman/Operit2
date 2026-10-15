import 'dart:math' as math;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

/// Keeps Flutter list behavior while supplying Compose-aware viewport measurement.
class ComposeDslLazyListView extends ListView {
  /// Creates a lazy list whose cross axis is measured from its instantiated items.
  const ComposeDslLazyListView({
    super.key,
    required super.controller,
    required super.scrollDirection,
    required super.reverse,
    required super.shrinkWrap,
    required super.childrenDelegate,
    super.itemExtent,
    super.cacheExtent,
  }) : super.custom(padding: EdgeInsets.zero);

  /// Uses one content-measuring viewport for both list axes and sizing modes.
  @override
  Widget buildViewport(
    BuildContext context,
    ViewportOffset offset,
    AxisDirection axisDirection,
    List<Widget> slivers,
  ) => _ComposeLazyViewport(
    axisDirection: axisDirection,
    offset: offset,
    slivers: slivers,
    shrinkWrap: shrinkWrap,
    clipBehavior: clipBehavior,
    paintOrder: paintOrder,
  );
}

/// Reuses Flutter's sliver element and scrolling protocols for natural cross sizing.
class _ComposeLazyViewport extends ShrinkWrappingViewport {
  /// Creates a viewport with explicit main-axis sizing and content-driven cross sizing.
  const _ComposeLazyViewport({
    required super.axisDirection,
    required super.offset,
    required super.slivers,
    required this.shrinkWrap,
    required super.clipBehavior,
    required super.paintOrder,
  });

  final bool shrinkWrap;

  /// Creates the sliver viewport without requiring a bounded cross axis.
  @override
  _RenderComposeLazyViewport createRenderObject(BuildContext context) =>
      _RenderComposeLazyViewport(
        axisDirection: axisDirection,
        crossAxisDirection: Viewport.getDefaultCrossAxisDirection(
          context,
          axisDirection,
        ),
        offset: offset,
        shrinkWrap: shrinkWrap,
        clipBehavior: clipBehavior,
        paintOrder: paintOrder,
      );

  /// Preserves the existing scroll position and render children during updates.
  @override
  void updateRenderObject(
    BuildContext context,
    _RenderComposeLazyViewport renderObject,
  ) {
    super.updateRenderObject(context, renderObject);
    renderObject.shrinkWrap = shrinkWrap;
  }
}

/// Measures real cached items before assigning their final cross-axis extent.
class _RenderComposeLazyViewport extends RenderShrinkWrappingViewport {
  /// Retains Flutter's sliver painting, hit testing, semantics, and reveal geometry.
  _RenderComposeLazyViewport({
    required super.axisDirection,
    required super.crossAxisDirection,
    required super.offset,
    required super.clipBehavior,
    required super.paintOrder,
    required bool shrinkWrap,
  }) : _shrinkWrap = shrinkWrap;

  bool _shrinkWrap;
  double _availableCrossAxisExtent = 0;
  double _contentScrollExtent = 0;
  double _contentPaintExtent = 0;
  bool _visualOverflow = false;

  /// Exposes the parent's actual cross-axis limit to item measurement.
  double get availableCrossAxisExtent => _availableCrossAxisExtent;

  /// Reports whether the main axis wraps its contents.
  bool get shrinkWrap => _shrinkWrap;

  /// Invalidates geometry when the main-axis sizing policy changes.
  set shrinkWrap(bool value) {
    if (_shrinkWrap == value) return;
    _shrinkWrap = value;
    markNeedsLayout();
  }

  /// Measures only instantiated items, then aligns them in the measured viewport.
  @override
  void performLayout() {
    final (mainLimit, crossLimit) = switch (axis) {
      Axis.vertical => (constraints.maxHeight, constraints.maxWidth),
      Axis.horizontal => (constraints.maxWidth, constraints.maxHeight),
    };
    if (!shrinkWrap && !mainLimit.isFinite) {
      throw FlutterError(
        'A non-shrink-wrapping Compose list requires a bounded main axis.',
      );
    }
    if (_availableCrossAxisExtent != crossLimit) {
      // Item measurement also depends on this limit, beyond its sliver box
      // constraints. Invalidate through Flutter's layout callback protocol:
      // direct descendant mutation during performLayout is forbidden.
      invokeLayoutCallback<BoxConstraints>((_) {
        _availableCrossAxisExtent = crossLimit;
        _visitComposeListItems(
          this,
          (item) => item.markNeedsLayout(),
          includeKeptAlive: true,
        );
      });
    }
    var crossExtent = _constrainCrossAxis(0);
    while (true) {
      final correction = _layoutSlivers(mainLimit, crossExtent);
      if (correction != 0) {
        offset.correctBy(correction);
        continue;
      }
      var measuredCrossExtent = 0.0;
      _visitComposeListItems(this, (item) {
        measuredCrossExtent = math.max(
          measuredCrossExtent,
          item.naturalCrossAxisExtent,
        );
      });
      final nextCrossExtent = _constrainCrossAxis(measuredCrossExtent);
      if (nextCrossExtent != crossExtent) {
        crossExtent = nextCrossExtent;
        continue;
      }
      final mainExtent = shrinkWrap
          ? switch (axis) {
              Axis.vertical => constraints.constrainHeight(_contentPaintExtent),
              Axis.horizontal => constraints.constrainWidth(
                _contentPaintExtent,
              ),
            }
          : mainLimit;
      final acceptedViewport = offset.applyViewportDimension(mainExtent);
      final acceptedContent = offset.applyContentDimensions(
        0,
        math.max(0, _contentScrollExtent - mainExtent),
      );
      if (acceptedViewport && acceptedContent) {
        size = switch (axis) {
          Axis.vertical => Size(crossExtent, mainExtent),
          Axis.horizontal => Size(mainExtent, crossExtent),
        };
        return;
      }
    }
  }

  /// Applies parent minimums and maximums to the measured content extent.
  double _constrainCrossAxis(double extent) => switch (axis) {
    Axis.vertical =>
      constraints.hasBoundedWidth
          ? constraints.maxWidth
          : constraints.constrainWidth(extent),
    Axis.horizontal =>
      constraints.hasBoundedHeight
          ? constraints.maxHeight
          : constraints.constrainHeight(extent),
  };

  /// Uses Flutter's sliver layout sequence without eagerly building distant items.
  double _layoutSlivers(double mainExtent, double crossExtent) {
    _contentScrollExtent = 0;
    _contentPaintExtent = 0;
    final pixels = offset.pixels;
    _visualOverflow = pixels < 0;
    final cache = cacheExtent!;
    return layoutChildSequence(
      child: firstChild,
      scrollOffset: math.max(0, pixels),
      overlap: math.min(0, pixels),
      layoutOffset: math.max(0, -pixels),
      remainingPaintExtent: mainExtent + math.min(0, pixels),
      mainAxisExtent: mainExtent,
      crossAxisExtent: crossExtent,
      growthDirection: GrowthDirection.forward,
      advance: childAfter,
      remainingCacheExtent: mainExtent + 2 * cache,
      cacheOrigin: -cache,
    );
  }

  /// Collects scroll geometry from the same slivers that were actually laid out.
  @override
  void updateOutOfBandData(
    GrowthDirection growthDirection,
    SliverGeometry childLayoutGeometry,
  ) {
    assert(growthDirection == GrowthDirection.forward);
    _contentScrollExtent += childLayoutGeometry.scrollExtent;
    _contentPaintExtent += childLayoutGeometry.maxPaintExtent;
    _visualOverflow |= childLayoutGeometry.hasVisualOverflow;
  }

  /// Supplies the overflow state used by Flutter's viewport clipping.
  @override
  bool get hasVisualOverflow => _visualOverflow;

  /// Includes the actual cache region in the inherited accessibility protocol.
  @override
  Rect? describeSemanticsClip(RenderSliver? child) {
    if (child != null &&
        child.ensureSemantics &&
        !(child.geometry!.visible || child.geometry!.cacheExtent > 0)) {
      return null;
    }
    final bounds = semanticBounds;
    final cache = cacheExtent!;
    return switch (axis) {
      Axis.vertical => Rect.fromLTRB(
        bounds.left,
        bounds.top - cache,
        bounds.right,
        bounds.bottom + cache,
      ),
      Axis.horizontal => Rect.fromLTRB(
        bounds.left - cache,
        bounds.top,
        bounds.right + cache,
        bounds.bottom,
      ),
    };
  }
}

/// Visits active items for measurement, optionally including retained items for invalidation.
void _visitComposeListItems(
  RenderObject root,
  void Function(_RenderComposeListItem item) visitor, {
  bool includeKeptAlive = false,
}) {
  if (root is _RenderComposeListItem) {
    visitor(root);
    return;
  }
  if (root is RenderSliverMultiBoxAdaptor && !includeKeptAlive) {
    var child = root.firstChild;
    while (child != null) {
      _visitComposeListItems(child, visitor);
      child = root.childAfter(child);
    }
    return;
  }
  root.visitChildren(
    (child) => _visitComposeListItems(
      child,
      visitor,
      includeKeptAlive: includeKeptAlive,
    ),
  );
}

/// Measures an item naturally while retaining Flutter's required sliver box size.
class ComposeDslLazyListItem extends SingleChildRenderObjectWidget {
  /// Wraps the actual item once, preserving its state and stable list key.
  const ComposeDslLazyListItem({
    super.key,
    required this.axis,
    required this.alignment,
    required Widget super.child,
  });

  final Axis axis;
  final Alignment alignment;

  /// Creates the natural-size measuring and alignment adapter.
  @override
  RenderAligningShiftedBox createRenderObject(BuildContext context) =>
      _RenderComposeListItem(axis: axis, alignment: alignment);

  /// Updates measurement and alignment without replacing the item subtree.
  @override
  void updateRenderObject(
    BuildContext context,
    RenderAligningShiftedBox renderObject,
  ) {
    (renderObject as _RenderComposeListItem)
      ..axis = axis
      ..alignment = alignment;
  }
}

/// Separates a child's natural cross size from its final sliver cross constraint.
class _RenderComposeListItem extends RenderAligningShiftedBox {
  /// Stores the list axis and delegates painting and hit testing to Flutter.
  _RenderComposeListItem({required Axis axis, required super.alignment})
    : _axis = axis,
      super(textDirection: null);

  Axis _axis;
  double naturalCrossAxisExtent = 0;

  /// Reports the direction used to loosen item measurement.
  Axis get axis => _axis;

  /// Invalidates natural measurement when the list direction changes.
  set axis(Axis value) {
    if (_axis == value) return;
    _axis = value;
    markNeedsLayout();
  }

  /// Requires the measuring viewport that owns this item and its sliver.
  _RenderComposeLazyViewport get _viewport {
    for (var ancestor = parent; ancestor != null; ancestor = ancestor.parent) {
      if (ancestor is _RenderComposeLazyViewport) return ancestor;
    }
    throw StateError(
      'A Compose list item must belong to a Compose lazy viewport.',
    );
  }

  /// Preserves main-axis constraints and exposes the real parent cross-axis limit.
  BoxConstraints _naturalConstraints(BoxConstraints incoming) => switch (axis) {
    Axis.vertical => BoxConstraints(
      maxWidth: _viewport.availableCrossAxisExtent,
      minHeight: incoming.minHeight,
      maxHeight: incoming.maxHeight,
    ),
    Axis.horizontal => BoxConstraints(
      minWidth: incoming.minWidth,
      maxWidth: incoming.maxWidth,
      maxHeight: _viewport.availableCrossAxisExtent,
    ),
  };

  /// Reports the sliver-compatible dry size using the real measurement constraints.
  @override
  Size computeDryLayout(BoxConstraints constraints) => constraints.constrain(
    child!.getDryLayout(_naturalConstraints(constraints)),
  );

  /// Measures the baseline with the same natural constraints and alignment offset.
  @override
  double? computeDryBaseline(
    BoxConstraints constraints,
    TextBaseline baseline,
  ) {
    final natural = _naturalConstraints(constraints);
    final baselineOffset = child!.getDryBaseline(natural, baseline);
    if (baselineOffset == null) return null;
    final childSize = child!.getDryLayout(natural);
    final parentSize = constraints.constrain(childSize);
    return baselineOffset +
        resolvedAlignment
            .alongOffset(
              Offset(
                parentSize.width - childSize.width,
                parentSize.height - childSize.height,
              ),
            )
            .dy;
  }

  /// Measures the actual child once per constraint change and aligns its real box.
  @override
  void performLayout() {
    child!.layout(_naturalConstraints(constraints), parentUsesSize: true);
    naturalCrossAxisExtent = switch (axis) {
      Axis.vertical => child!.size.width,
      Axis.horizontal => child!.size.height,
    };
    size = constraints.constrain(child!.size);
    alignChild();
  }
}
