// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/foundation.dart' show precisionErrorTolerance;
import 'package:flutter/gestures.dart';
import 'package:flutter/physics.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import 'package:operit2/ui/common/code_editor/CodeEditorSurface.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorScrollController.dart';

/// Scales text while optionally fitting its wrapping width to the viewport.
class CodeEditorZoomTransform extends SingleChildRenderObjectWidget {
  /// Creates a zoom boundary whose input coordinates match its painted text.
  const CodeEditorZoomTransform({
    super.key,
    required this.scale,
    required this.horizontalOffset,
    required this.wrapToViewport,
    this.contentWidth,
    required super.child,
  });

  final double scale;
  final double horizontalOffset;
  final bool wrapToViewport;

  /// Optional unscaled width for horizontally overflowing code documents.
  final double? contentWidth;

  /// Creates the shared layout, paint, hit-test, and semantics transform.
  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderTextZoomViewport(
        scale: scale,
        horizontalOffset: horizontalOffset,
        wrapToViewport: wrapToViewport,
        contentWidth: contentWidth,
      );

  /// Updates geometry in the rendering pipeline rather than after painting.
  @override
  void updateRenderObject(
    BuildContext context,
    covariant RenderObject renderObject,
  ) {
    (renderObject as _RenderTextZoomViewport).updateZoom(
      scale,
      horizontalOffset,
      wrapToViewport,
      contentWidth,
    );
  }
}

class _RenderTextZoomViewport extends RenderProxyBox {
  /// Creates a viewport with explicit wrapping and horizontal bounds.
  _RenderTextZoomViewport({
    required double scale,
    required double horizontalOffset,
    required bool wrapToViewport,
    required double? contentWidth,
  }) : _scale = scale,
       _horizontalOffset = horizontalOffset,
       _wrapToViewport = wrapToViewport,
       _contentWidth = contentWidth;

  double _scale;
  double _horizontalOffset;
  bool _wrapToViewport;
  double? _contentWidth;

  /// Shares one affine transform between painting, selection, and hit testing.
  Matrix4 get _transform {
    final extent = size.width - (_contentWidth ?? size.width) * _scale;
    final offset = _wrapToViewport
        ? 0.0
        : _horizontalOffset.clamp(extent < 0 ? extent : 0.0, 0.0);
    return Matrix4.diagonal3Values(_scale, _scale, 1)..setEntry(0, 3, offset);
  }

  /// Invalidates layout and paint together when the visible zoom changes.
  void updateZoom(
    double scale,
    double horizontalOffset,
    bool wrapToViewport,
    double? contentWidth,
  ) {
    if (_scale == scale &&
        _horizontalOffset == horizontalOffset &&
        _wrapToViewport == wrapToViewport &&
        _contentWidth == contentWidth) {
      return;
    }
    _scale = scale;
    _horizontalOffset = horizontalOffset;
    _wrapToViewport = wrapToViewport;
    _contentWidth = contentWidth;
    markNeedsLayout();
    markNeedsPaint();
    markNeedsSemanticsUpdate();
  }

  /// Fits wrapped editor lines to the actual visible width at every scale.
  @override
  void performLayout() {
    assert(constraints.hasBoundedWidth && constraints.hasBoundedHeight);
    size = constraints.biggest;
    child!.layout(
      BoxConstraints.tightFor(
        width: _wrapToViewport
            ? size.width / _scale
            : _contentWidth == null
            ? size.width
            : math.max(size.width / _scale, _contentWidth!),
        height: size.height / _scale,
      ),
      parentUsesSize: true,
    );
  }

  /// Paints the text and cursor using the exact transform used by input.
  @override
  void paint(PaintingContext context, Offset offset) {
    context.pushTransform(needsCompositing, offset, _transform, super.paint);
  }

  /// Maps selection and scroll gestures into the unscaled text viewport.
  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) {
    return result.addWithPaintTransform(
      transform: _transform,
      position: position,
      hitTest: (result, position) =>
          super.hitTestChildren(result, position: position),
    );
  }

  /// Exposes the same geometry to cursor overlays and accessibility clients.
  @override
  void applyPaintTransform(RenderBox child, Matrix4 transform) {
    transform.multiply(_transform);
  }
}

/// Shares one viewport's scrolling and gesture policy with its text or lazy list.
class CodeEditorViewportScope extends InheritedWidget {
  const CodeEditorViewportScope({
    super.key,
    required this.controller,
    required this.panEnabled,
    required this.pinching,
    required super.child,
  });

  final ScrollController controller;
  final bool panEnabled;
  final bool pinching;

  ScrollPhysics? get physics =>
      panEnabled || pinching ? const NeverScrollableScrollPhysics() : null;

  static CodeEditorViewportScope? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<CodeEditorViewportScope>();

  @override
  bool updateShouldNotify(CodeEditorViewportScope oldWidget) =>
      controller != oldWidget.controller ||
      pinching != oldWidget.pinching ||
      panEnabled != oldWidget.panEnabled;
}

/// The shared gesture, focal anchoring and layout path for editors and review.
class CodeEditorViewport extends StatefulWidget {
  const CodeEditorViewport({
    super.key,
    this.scale,
    this.onScaleChanged,
    this.scrollController,
    this.targetKey,
    this.identity,
    this.wrapToViewport = false,
    this.panEnabled = false,
    this.contentWidth,
    this.minScale = 0.6,
    this.maxScale = 2.5,
    this.chromeBuilder,
    this.contentBuilder,
    this.child,
  }) : assert(child != null || contentBuilder != null),
       assert(minScale > 0 && maxScale >= minScale);

  final double minScale;
  final double maxScale;
  final Widget Function(BuildContext context, double scale, Widget child)?
  chromeBuilder;
  final Widget Function(double scale)? contentBuilder;
  final double? scale;
  final ValueChanged<double>? onScaleChanged;
  final CodeEditorScrollController? scrollController;
  final GlobalKey? targetKey;
  final Object? identity;
  final bool wrapToViewport;
  final bool panEnabled;
  final double? contentWidth;
  final Widget? child;

  @override
  State<CodeEditorViewport> createState() => CodeEditorViewportState();
}

class CodeEditorViewportState extends State<CodeEditorViewport>
    with SingleTickerProviderStateMixin {
  CodeEditorScrollController? _ownedScroll;
  final _viewportKey = GlobalKey();
  final _contentKey = GlobalKey();
  final Map<int, Offset> _touches = {};
  late double _scale = (widget.scale ?? 1).clamp(
    widget.minScale,
    widget.maxScale,
  );
  double _horizontal = 0;
  double _startScale = 1;
  double? _startDistance;
  bool _trackpadPinching = false;
  Offset? _trackpadPan;
  bool _pointerEventsActive = true;
  RenderBox? _target;
  Offset? _sceneAnchor;
  TextPosition? _textAnchor;
  double _lineFraction = 0;
  Offset? _localFocalPoint;
  double _renderOriginY = 0;
  late final AnimationController _inertia;

  @override
  void initState() {
    super.initState();
    _inertia = AnimationController.unbounded(vsync: this)
      ..addListener(_flingTick);
  }

  Offset _flingDirection = Offset.zero;
  double _lastTravel = 0;

  CodeEditorScrollController get _scroll =>
      widget.scrollController ??
      (_ownedScroll ??= CodeEditorScrollController());
  RenderBox get _viewport =>
      _viewportKey.currentContext!.findRenderObject()! as RenderBox;
  bool get _pinching => _touches.length == 2 || _trackpadPinching;

  @override
  void didUpdateWidget(CodeEditorViewport oldWidget) {
    super.didUpdateWidget(oldWidget);
    _scale = (widget.scale ?? _scale).clamp(widget.minScale, widget.maxScale);
    if (widget.identity != oldWidget.identity) {
      _retirePointerSequence();
      _horizontal = 0;
      oldWidget.scrollController?.cancelZoomAnchor();
    }
  }

  /// Retires geometry and pointer ownership before the document or viewport leaves.
  void _retirePointerSequence() {
    _touches.clear();
    _startDistance = null;
    _trackpadPinching = false;
    _trackpadPan = null;
    _target = null;
    _sceneAnchor = null;
    _textAnchor = null;
    _localFocalPoint = null;
    widget.scrollController?.cancelZoomAnchor();
    _ownedScroll?.cancelZoomAnchor();
    _inertia.stop();
  }

  /// Ends gestures before cached Flutter hit tests can deliver events to a removed viewport.
  @override
  void deactivate() {
    _pointerEventsActive = false;
    _retirePointerSequence();
    super.deactivate();
  }

  /// Allows a reattached viewport to acquire a fresh pointer sequence.
  @override
  void activate() {
    super.activate();
    _pointerEventsActive = true;
  }

  /// Locates the active text renderer or lazy viewport through its render contract.
  RenderBox get _zoomRender {
    final matches = <RenderBox>[];
    void collect(RenderObject render) {
      if (render is RenderEditable || render is RenderAbstractViewport) {
        matches.add(render as RenderBox);
      } else {
        render.visitChildren(collect);
      }
    }

    collect(
      (widget.targetKey ?? _contentKey).currentContext!.findRenderObject()!,
    );
    return matches.single;
  }

  double _boundHorizontal(double offset, double scale) {
    if (widget.wrapToViewport) return 0;
    final extent =
        _viewport.size.width -
        (widget.contentWidth ?? _viewport.size.width) * scale;
    return offset.clamp(extent < 0 ? extent : 0.0, 0.0).toDouble();
  }

  double _anchorLineTop(RenderEditable render) =>
      render
          .getEndpointsForSelection(TextSelection.fromPosition(_textAnchor!))
          .single
          .point
          .dy -
      render.preferredLineHeight;

  /// Captures a source position for reflowing text, or a scene point for lazy rows.
  void _capture(Offset focalPoint) {
    _inertia.stop();
    _scroll.jumpTo(_scroll.offset);
    _horizontal = _boundHorizontal(_horizontal, _scale);
    final local = _viewport.globalToLocal(focalPoint);
    final render = _zoomRender;
    _target = render;
    final renderY = render.globalToLocal(focalPoint).dy;
    _localFocalPoint = local;
    _renderOriginY = local.dy - renderY * _scale;
    if (render is RenderCodeEditorText) {
      final editable = render;
      _textAnchor = editable.getPositionForPoint(focalPoint);
      _lineFraction =
          (renderY - _anchorLineTop(editable)) / editable.preferredLineHeight;
    }
    _sceneAnchor = Offset(
      (local.dx - _horizontal) / _scale,
      _scroll.offset + renderY,
    );
  }

  double _resolveZoomOffset() {
    final render = _target;
    final sceneY = render is RenderCodeEditorText
        ? _anchorLineTop(render) +
              _scroll.offset +
              _lineFraction * render.preferredLineHeight
        : _sceneAnchor!.dy;
    return sceneY - (_localFocalPoint!.dy - _renderOriginY) / _scale;
  }

  /// Commits the same editor anchoring transaction for every viewport consumer.
  void _zoom(double scale, Offset focalPoint) {
    final next = scale.clamp(widget.minScale, widget.maxScale);
    final local = _viewport.globalToLocal(focalPoint);
    if (next == _scale && _localFocalPoint == local) return;
    _localFocalPoint = local;
    _horizontal = _boundHorizontal(local.dx - _sceneAnchor!.dx * next, next);
    _scroll.anchorNextLayout(_resolveZoomOffset);
    if (_target case final RenderCodeEditorText render) {
      render.prepareAnchor(_textAnchor!);
    }
    _target!.markNeedsLayout();
    setState(() {
      _scale = next;
    });
    widget.onScaleChanged?.call(next);
  }

  /// Supports the editor's existing keyboard zoom actions.
  void zoomTo(double scale) {
    final center = _viewport.localToGlobal(_viewport.size.center(Offset.zero));
    _capture(center);
    _zoom(scale, center);
  }

  double get _touchDistance {
    final points = _touches.values.toList(growable: false);
    return (points[0] - points[1]).distance;
  }

  void _resetPinchBaseline() {
    _startScale = _scale;
    _startDistance = _touches.length == 2 ? _touchDistance : null;
    if (_touches.length == 2) {
      final points = _touches.values.toList(growable: false);
      _capture((points[0] + points[1]) / 2);
    }
  }

  void _pointerDown(PointerDownEvent event) {
    if (!_pointerEventsActive) return;
    _inertia.stop();
    if (event.kind != PointerDeviceKind.touch) return;
    final wasPinching = _pinching;
    _touches[event.pointer] = event.position;
    _resetPinchBaseline();
    if (wasPinching != _pinching) setState(() {});
  }

  void _pointerMove(PointerMoveEvent event) {
    if (!_touches.containsKey(event.pointer)) return;
    _touches[event.pointer] = event.position;
    final distance = _startDistance;
    if (_touches.length != 2 || distance == null || distance < 1) return;
    final points = _touches.values.toList(growable: false);
    _zoom(_startScale * _touchDistance / distance, (points[0] + points[1]) / 2);
  }

  void _pointerUp(PointerEvent event) {
    final wasPinching = _pinching;
    if (_touches.remove(event.pointer) == null) return;
    _resetPinchBaseline();
    if (wasPinching != _pinching) setState(() {});
  }

  /// Scrolls rows directly and rebuilds only when horizontal translation changes.
  void _pan(Offset delta) {
    if (!_pointerEventsActive || _pinching) return;
    _scroll.jumpTo(
      (_scroll.offset - delta.dy / _scale).clamp(
        0.0,
        _scroll.position.maxScrollExtent,
      ),
    );
    final horizontal = _boundHorizontal(_horizontal + delta.dx, _scale);
    if (!nearEqual(horizontal, _horizontal, precisionErrorTolerance)) {
      setState(() => _horizontal = horizontal);
    }
  }

  void _flingTick() {
    final travel = _inertia.value;
    _pan(_flingDirection * (travel - _lastTravel));
    _lastTravel = travel;
  }

  /// Starts trackpad scrolling without taking ownership from Flutter's drag.
  void _trackpadStart(PointerPanZoomStartEvent event) {
    if (!_pointerEventsActive) return;
    _inertia.stop();
    _startScale = _scale;
    _trackpadPan = Offset.zero;
  }

  /// Captures the current focal point only when a trackpad actually pinches.
  void _trackpadUpdate(PointerPanZoomUpdateEvent event) {
    final previousPan = _trackpadPan;
    if (!_pointerEventsActive || previousPan == null) return;
    _trackpadPan = event.pan;
    if (!_trackpadPinching) {
      if (event.scale == 1) return;
      _capture(event.position + previousPan);
      setState(() => _trackpadPinching = true);
    }
    _zoom(_startScale * event.scale, event.position + event.pan);
  }

  /// Releases pinch ownership while leaving ordinary trackpad pans native.
  void _trackpadEnd(PointerPanZoomEndEvent event) {
    _trackpadPan = null;
    if (_trackpadPinching) setState(() => _trackpadPinching = false);
  }

  @override
  void dispose() {
    _inertia.dispose();
    _ownedScroll?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    Widget content = ClipRect(
      key: _viewportKey,
      child: CodeEditorZoomTransform(
        scale: _scale,
        horizontalOffset: _horizontal,
        wrapToViewport: widget.wrapToViewport,
        contentWidth: widget.contentWidth,
        child: CodeEditorViewportScope(
          controller: _scroll,
          panEnabled: widget.panEnabled,
          pinching: _pinching,
          child: KeyedSubtree(
            key: _contentKey,
            child: widget.contentBuilder?.call(_scale) ?? widget.child!,
          ),
        ),
      ),
    );
    if (widget.panEnabled) {
      content = GestureDetector(
        behavior: HitTestBehavior.opaque,
        onPanUpdate: (details) => _pan(details.delta),
        onPanEnd: (details) {
          if (!_pointerEventsActive || _pinching || _touches.isNotEmpty) return;
          final velocity = details.velocity.pixelsPerSecond;
          if (velocity.distance < 50) return;
          _flingDirection = velocity / velocity.distance;
          _lastTravel = 0;
          _inertia.animateWith(FrictionSimulation(0.135, 0, velocity.distance));
        },
        child: content,
      );
    }
    final result = Listener(
      behavior: HitTestBehavior.opaque,
      onPointerDown: _pointerDown,
      onPointerMove: _pointerMove,
      onPointerUp: _pointerUp,
      onPointerCancel: _pointerUp,
      onPointerPanZoomStart: _trackpadStart,
      onPointerPanZoomUpdate: _trackpadUpdate,
      onPointerPanZoomEnd: _trackpadEnd,
      onPointerSignal: widget.panEnabled
          ? (event) {
              if (event is! PointerScrollEvent) return;
              GestureBinding.instance.pointerSignalResolver.register(event, (
                event,
              ) {
                if (!_pointerEventsActive) return;
                final scroll = event as PointerScrollEvent;
                _inertia.stop();
                if (HardwareKeyboard.instance.isControlPressed ||
                    HardwareKeyboard.instance.isMetaPressed) {
                  _capture(scroll.position);
                  _zoom(
                    _scale * math.exp(-scroll.scrollDelta.dy / 300),
                    scroll.position,
                  );
                } else {
                  _pan(
                    _viewport.globalToLocal(
                          scroll.position - scroll.scrollDelta,
                        ) -
                        _viewport.globalToLocal(scroll.position),
                  );
                }
              });
            }
          : null,
      child: content,
    );
    return widget.chromeBuilder?.call(context, _scale, result) ?? result;
  }
}
