// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Reports actual painted bounds, including changes caused by ancestor scrolling.
class _PositionReportingBox extends SingleChildRenderObjectWidget {
  /// Creates a geometry observer without changing child layout constraints.
  const _PositionReportingBox({required super.child, required this.onPosition});

  final ValueChanged<Rect> onPosition;

  /// Creates the geometry-aware render proxy.
  @override
  RenderObject createRenderObject(BuildContext context) =>
      _PositionReportingRenderBox(onPosition);

  /// Refreshes the callback when plugin action identifiers change.
  @override
  void updateRenderObject(
    BuildContext context,
    _PositionReportingRenderBox renderObject,
  ) {
    renderObject.onPosition = onPosition;
  }
}

class _PositionReportingRenderBox extends RenderProxyBox {
  /// Stores the callback for distinct painted geometry snapshots.
  _PositionReportingRenderBox(this.onPosition);

  ValueChanged<Rect> onPosition;
  Rect? _lastBounds;

  /// Reports bounds after painting so callbacks cannot mutate the layout in progress.
  @override
  void paint(PaintingContext context, Offset offset) {
    super.paint(context, offset);
    final bounds = localToGlobal(Offset.zero) & size;
    if (bounds == _lastBounds) return;
    _lastBounds = bounds;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (attached) onPosition(bounds);
    });
  }
}

class _SizeReportingBox extends SingleChildRenderObjectWidget {
  /// Observes layout changes even when the retained DSL widget is not rebuilt.
  const _SizeReportingBox({required super.child, required this.onSizeChanged});

  final ValueChanged<Size> onSizeChanged;

  /// Creates a size observer that preserves its child's incoming constraints.
  @override
  RenderObject createRenderObject(BuildContext context) =>
      _SizeReportingRenderBox(onSizeChanged);

  /// Uses the current action callback after plugin recomposition.
  @override
  void updateRenderObject(
    BuildContext context,
    _SizeReportingRenderBox renderObject,
  ) {
    renderObject.onSizeChanged = onSizeChanged;
  }
}

class _SizeReportingRenderBox extends RenderProxyBox {
  /// Retains the last delivered size across layout and callback updates.
  _SizeReportingRenderBox(this.onSizeChanged);

  ValueChanged<Size> onSizeChanged;
  Size? _lastReportedSize;
  bool _reportScheduled = false;

  /// Coalesces real geometry changes into one notification after the current frame.
  @override
  void performLayout() {
    super.performLayout();
    if (size == _lastReportedSize || _reportScheduled) return;
    _reportScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _reportScheduled = false;
      if (!attached || !hasSize || size == _lastReportedSize) return;
      _lastReportedSize = size;
      onSizeChanged(size);
    });
  }
}
