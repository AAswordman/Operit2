// ignore_for_file: file_names

import 'package:flutter/widgets.dart';

/// Applies zoom anchoring during scroll layout, before any text can be painted.
class CodeEditorScrollController extends ScrollController {
  /// Creates a scroll controller with a one-layout zoom correction request.
  CodeEditorScrollController();

  double Function()? _resolveZoomOffset;
  bool _virtualLayout = false;

  /// Keeps the source anchor active until virtual height discovery has settled.
  void beginVirtualLayout() {
    _virtualLayout = true;
  }

  /// Completes the same-frame virtual layout and releases its source anchor.
  void endVirtualLayout() {
    _virtualLayout = false;
    _resolveZoomOffset = null;
  }

  /// Replaces the pending correction with the latest focal-point geometry.
  void anchorNextLayout(double Function() resolveOffset) {
    _resolveZoomOffset = resolveOffset;
  }

  /// Cancels geometry belonging to a document or presentation being replaced.
  void cancelZoomAnchor() {
    _resolveZoomOffset = null;
  }

  /// Creates a position that corrects zoom offsets in the layout transaction.
  @override
  ScrollPosition createScrollPosition(
    ScrollPhysics physics,
    ScrollContext context,
    ScrollPosition? oldPosition,
  ) => _WorkspaceZoomScrollPosition(
    controller: this,
    physics: physics,
    context: context,
    oldPosition: oldPosition,
    initialPixels: initialScrollOffset,
    keepScrollOffset: keepScrollOffset,
  );
}

class _WorkspaceZoomScrollPosition extends ScrollPositionWithSingleContext {
  /// Shares pending zoom geometry across scroll-position recreation.
  _WorkspaceZoomScrollPosition({
    required this.controller,
    required super.physics,
    required super.context,
    required super.oldPosition,
    required super.initialPixels,
    required super.keepScrollOffset,
  });

  final CodeEditorScrollController controller;
  bool _correctingZoom = false;

  /// Commits a focal-point correction as soon as new viewport metrics exist.
  @override
  bool applyContentDimensions(double minScrollExtent, double maxScrollExtent) {
    final resolve = controller._resolveZoomOffset;
    if (!controller._virtualLayout) controller._resolveZoomOffset = null;
    final before = pixels;
    _correctingZoom = resolve != null;
    if (resolve != null) {
      correctPixels(resolve().clamp(minScrollExtent, maxScrollExtent));
    }
    try {
      final accepted = super.applyContentDimensions(
        minScrollExtent,
        maxScrollExtent,
      );
      // Scroll viewports must recompute their child paint offsets in this same
      // layout pass. RenderEditable reads the corrected offset directly.
      return accepted && before == pixels;
    } finally {
      _correctingZoom = false;
    }
  }

  /// Keeps ordinary dimension physics from overwriting the zoom transaction.
  @override
  bool correctForNewDimensions(
    ScrollMetrics oldPosition,
    ScrollMetrics newPosition,
  ) {
    if (_correctingZoom) return true;
    return super.correctForNewDimensions(oldPosition, newPosition);
  }
}
