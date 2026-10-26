import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:webview_platform_interface/webview_platform_interface.dart';

import 'linux_webview_controller.dart';
import 'linux_webview_creation_params.dart';

class LinuxWebViewWidget extends PlatformWebViewWidget {
  LinuxWebViewWidget(PlatformWebViewWidgetCreationParams params)
    : super.implementation(
        params is LinuxWebViewWidgetCreationParams
            ? params
            : LinuxWebViewWidgetCreationParams.fromPlatformWebViewWidgetCreationParams(
                params,
              ),
      );

  @override
  Widget build(BuildContext context) {
    final LinuxWebViewController controller =
        params.controller as LinuxWebViewController;
    return _LinuxPlatformWebView(
      controller: controller,
      gestureRecognizers: params.gestureRecognizers,
      key: params.key,
    );
  }
}

class _LinuxPlatformWebView extends StatefulWidget {
  const _LinuxPlatformWebView({
    super.key,
    required this.controller,
    required this.gestureRecognizers,
  });

  final LinuxWebViewController controller;
  final Set<Factory<OneSequenceGestureRecognizer>> gestureRecognizers;

  @override
  State<_LinuxPlatformWebView> createState() => _LinuxPlatformWebViewState();
}

class _LinuxViewGeometry {
  const _LinuxViewGeometry({required this.frame, required this.clip});

  static const _LinuxViewGeometry hidden = _LinuxViewGeometry(
    frame: Rect.zero,
    clip: Rect.zero,
  );

  final Rect frame;
  final Rect clip;

  bool get isVisible =>
      _isUsableRect(frame) && _isUsableRect(clip) && frame.overlaps(clip);

  static bool _isUsableRect(Rect rect) =>
      rect.left.isFinite &&
      rect.top.isFinite &&
      rect.right.isFinite &&
      rect.bottom.isFinite &&
      rect.width > 0 &&
      rect.height > 0;

  @override
  bool operator ==(Object other) =>
      other is _LinuxViewGeometry && other.frame == frame && other.clip == clip;

  @override
  int get hashCode => Object.hash(frame, clip);
}

class _LinuxPlatformWebViewState extends State<_LinuxPlatformWebView>
    with WidgetsBindingObserver {
  _LinuxViewGeometry _lastGeometry = _LinuxViewGeometry.hidden;
  bool _geometryVisible = false;
  bool _applicationVisible = true;
  bool _visibilityCheckScheduled = false;
  int? _nativeViewId;
  _LinuxInputController? _inputController;
  int _mouseButtons = 0;
  final FocusNode _focusNode = FocusNode(debugLabel: 'Linux WebView');
  final GlobalKey _geometryKey = GlobalKey();

  @override
  void initState() {
    super.initState();
    unawaited(_initializeComposition(widget.controller));
    WidgetsBinding.instance.addObserver(this);
    _applicationVisible = _isApplicationVisible(
      WidgetsBinding.instance.lifecycleState,
    );
    _scheduleVisibilityCheck();
  }

  @override
  void didChangeMetrics() {
    super.didChangeMetrics();
    _markGeometryNeedsUpdate();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    super.didChangeAppLifecycleState(state);
    final bool applicationVisible = _isApplicationVisible(state);
    if (_applicationVisible == applicationVisible) {
      return;
    }
    _applicationVisible = applicationVisible;
    if (applicationVisible) {
      // Keep the native view hidden until Flutter paints its current geometry.
      // The window may have moved or resized while the app was suspended.
      _geometryVisible = false;
      _markGeometryNeedsUpdate();
      _scheduleVisibilityCheck();
    } else {
      _syncFrame(widget.controller);
    }
  }

  @override
  void didUpdateWidget(_LinuxPlatformWebView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (identical(oldWidget.controller, widget.controller)) {
      return;
    }

    _nativeViewId = null;
    _inputController = null;
    _mouseButtons = 0;
    unawaited(_initializeComposition(widget.controller));
    final _LinuxViewGeometry currentGeometry = _lastGeometry;
    _setFrameSafely(
      oldWidget.controller,
      _LinuxViewGeometry.hidden,
      visible: false,
    );
    _setFrameSafely(
      widget.controller,
      _geometryVisible && _applicationVisible
          ? currentGeometry
          : _LinuxViewGeometry.hidden,
      visible: _geometryVisible && _applicationVisible,
    );
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _focusNode.dispose();
    _setFrameSafely(
      widget.controller,
      _LinuxViewGeometry.hidden,
      visible: false,
    );
    super.dispose();
  }

  void _pushGeometry(_LinuxViewGeometry geometry, {required bool visible}) {
    _lastGeometry = geometry;
    _geometryVisible = visible;
    _syncFrame(widget.controller);
    _scheduleVisibilityCheck();
  }

  void _syncFrame(LinuxWebViewController controller) {
    final bool visible = _geometryVisible && _applicationVisible;
    _setFrameSafely(
      controller,
      visible ? _lastGeometry : _LinuxViewGeometry.hidden,
      visible: visible,
    );
  }

  void _markGeometryNeedsUpdate() {
    if (!mounted) {
      return;
    }
    _geometryKey.currentContext?.findRenderObject()?.markNeedsPaint();
    // A lifecycle transition does not guarantee another Flutter frame. Make
    // sure the native view is restored without waiting for unrelated UI work.
    WidgetsBinding.instance.ensureVisualUpdate();
  }

  void _scheduleVisibilityCheck() {
    if (_visibilityCheckScheduled || !_applicationVisible) {
      return;
    }
    _visibilityCheckScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((Duration _) {
      _visibilityCheckScheduled = false;
      if (!mounted || !_applicationVisible) {
        return;
      }
      final RenderObject? renderObject = _geometryKey.currentContext
          ?.findRenderObject();
      if (renderObject is _LinuxGeometryRenderBox) {
        renderObject.syncGeometry();
      }
      _scheduleVisibilityCheck();
    });
  }

  static bool _isApplicationVisible(AppLifecycleState? state) {
    return state == null ||
        state == AppLifecycleState.resumed ||
        state == AppLifecycleState.inactive;
  }

  void _setFrameSafely(
    LinuxWebViewController controller,
    _LinuxViewGeometry geometry, {
    required bool visible,
  }) {
    unawaited(() async {
      try {
        await controller.setFrame(
          geometry.frame,
          clipRect: geometry.clip,
          visible: visible,
        );
      } on StateError catch (error) {
        if (!controller.isDisposed) {
          debugPrint(
            'webview_all_linux: failed to update the WebView frame: $error',
          );
        }
      } catch (error) {
        debugPrint(
          'webview_all_linux: failed to update the WebView frame: $error',
        );
      }
    }());
  }

  void _handleGeometryChanged(_LinuxViewGeometry geometry) {
    final bool visible = geometry.isVisible;
    if (_geometryVisible != visible || geometry != _lastGeometry) {
      _pushGeometry(
        visible ? geometry : _LinuxViewGeometry.hidden,
        visible: visible,
      );
    }
  }

  /// Connects the native controller identifier to the engine platform layer.
  Future<void> _initializeComposition(LinuxWebViewController controller) async {
    final int id;
    try {
      id = await controller.nativeViewIdentifier;
    } on StateError {
      if (controller.isDisposed) {
        return;
      }
      rethrow;
    }
    if (mounted && identical(widget.controller, controller)) {
      setState(() {
        _nativeViewId = id;
        _inputController = _LinuxInputController(
          viewId: id,
          forward: _forwardPointer,
          releaseFocus: controller.releaseFocus,
        );
      });
    }
  }

  /// Reports asynchronous native input errors without hiding failed operations.
  void _dispatchInput(Future<void> operation) {
    final LinuxWebViewController controller = widget.controller;
    unawaited(
      operation.catchError((Object error, StackTrace stack) {
        if (!controller.isDisposed) {
          FlutterError.reportError(
            FlutterErrorDetails(
              exception: error,
              stack: stack,
              library: 'webview_all_linux',
              context: ErrorDescription('while forwarding WebView input'),
            ),
          );
        }
      }),
    );
  }

  /// Encodes keyboard and mouse state using GDK modifier constants.
  int _modifiers(int buttons) {
    final HardwareKeyboard keyboard = HardwareKeyboard.instance;
    return (keyboard.isShiftPressed ? 1 : 0) |
        (keyboard.isControlPressed ? 4 : 0) |
        (keyboard.isAltPressed ? 8 : 0) |
        (keyboard.isMetaPressed ? 1 << 28 : 0) |
        (buttons & kPrimaryMouseButton != 0 ? 1 << 8 : 0) |
        (buttons & kMiddleMouseButton != 0 ? 1 << 9 : 0) |
        (buttons & kSecondaryMouseButton != 0 ? 1 << 10 : 0);
  }

  /// Sends a single event using coordinates local to the full WebKit layout.
  void _sendPointer(
    PointerEvent event,
    LinuxWebViewPointerEventType type, {
    int button = 0,
    int? buttons,
    Offset scrollDelta = Offset.zero,
  }) {
    _dispatchInput(
      widget.controller.dispatchPointerEvent(
        type: type,
        position: event.localPosition,
        pointer: event.pointer,
        time: event.timeStamp.inMilliseconds,
        modifiers: _modifiers(buttons ?? event.buttons),
        button: button,
        scrollDelta: scrollDelta,
      ),
    );
  }

  /// Routes browser pointer events only after Flutter accepts the hit test.
  void _forwardPointer(PointerEvent event) {
    if (_nativeViewId == null || widget.controller.isDisposed) {
      return;
    }
    if (event is PointerDownEvent) {
      _focusNode.requestFocus();
    }
    if (event.kind == PointerDeviceKind.touch) {
      final LinuxWebViewPointerEventType type = switch (event) {
        PointerDownEvent() => LinuxWebViewPointerEventType.touchBegin,
        PointerMoveEvent() => LinuxWebViewPointerEventType.touchUpdate,
        PointerUpEvent() => LinuxWebViewPointerEventType.touchEnd,
        PointerCancelEvent() => LinuxWebViewPointerEventType.touchCancel,
        _ => LinuxWebViewPointerEventType.motion,
      };
      _sendPointer(event, type);
      return;
    }
    if (event is PointerEnterEvent) {
      _sendPointer(event, LinuxWebViewPointerEventType.enter);
      return;
    }
    if (event is PointerExitEvent) {
      _sendPointer(event, LinuxWebViewPointerEventType.leave);
      return;
    }
    const Map<int, int> nativeButtons = <int, int>{
      kPrimaryMouseButton: 1,
      kSecondaryMouseButton: 3,
      kMiddleMouseButton: 2,
      kBackMouseButton: 8,
      kForwardMouseButton: 9,
    };
    final int buttons = event is PointerCancelEvent ? 0 : event.buttons;
    for (final MapEntry<int, int> entry in nativeButtons.entries) {
      if ((_mouseButtons ^ buttons) & entry.key == 0) {
        continue;
      }
      _sendPointer(
        event,
        buttons & entry.key != 0
            ? LinuxWebViewPointerEventType.buttonPress
            : LinuxWebViewPointerEventType.buttonRelease,
        button: entry.value,
        buttons: _mouseButtons,
      );
    }
    _mouseButtons = buttons;
    if (event is PointerMoveEvent || event is PointerHoverEvent) {
      _sendPointer(event, LinuxWebViewPointerEventType.motion);
    }
  }

  /// Resolves wheel ownership through Flutter before scrolling the browser.
  void _forwardSignal(PointerSignalEvent event) {
    if (event is PointerScrollEvent && _nativeViewId != null) {
      GestureBinding.instance.pointerSignalResolver.register(event, (event) {
        final PointerScrollEvent scroll = event as PointerScrollEvent;
        _sendPointer(
          scroll,
          LinuxWebViewPointerEventType.scroll,
          scrollDelta: scroll.scrollDelta,
        );
      });
    }
  }

  /// Releases native keyboard focus when a Flutter route takes focus.
  void _handleFocusChanged(bool focused) {
    if (!focused && !widget.controller.isDisposed) {
      _dispatchInput(widget.controller.releaseFocus());
    }
  }

  /// Builds the engine platform layer and the Flutter-owned input surface.
  @override
  Widget build(BuildContext context) {
    return Focus(
      focusNode: _focusNode,
      onFocusChange: _handleFocusChanged,
      child: MouseRegion(
        onEnter: _forwardPointer,
        onExit: _forwardPointer,
        child: Listener(
          behavior: HitTestBehavior.opaque,
          onPointerSignal: _forwardSignal,
          child: _LinuxGeometryObserver(
            key: _geometryKey,
            nativeViewId: _nativeViewId,
            onGeometryChanged: _handleGeometryChanged,
            onDetached: () =>
                _pushGeometry(_LinuxViewGeometry.hidden, visible: false),
            child: _inputController == null
                ? const SizedBox.expand()
                : _LinuxInputSurface(
                    key: ValueKey(_nativeViewId),
                    controller: _inputController!,
                    gestureRecognizers: widget.gestureRecognizers,
                  ),
          ),
        ),
      ),
    );
  }
}

/// Adapts native WebKit input to Flutter's platform-view gesture arbitration.
class _LinuxInputController extends PlatformViewController {
  /// Retains the native identity while the widget owns the browser lifecycle.
  _LinuxInputController({
    required this.viewId,
    required this.forward,
    required this.releaseFocus,
  });

  @override
  final int viewId;
  final ValueChanged<PointerEvent> forward;
  final Future<void> Function() releaseFocus;

  /// Forwards only pointer sequences accepted by Flutter's gesture arena.
  @override
  Future<void> dispatchPointerEvent(PointerEvent event) async => forward(event);

  /// Returns native focus when Flutter requests it through the platform API.
  @override
  Future<void> clearFocus() => releaseFocus();

  /// Leaves browser disposal to its owning WebView controller.
  @override
  Future<void> dispose() async {}
}

/// Uses Flutter's standard gesture recognizers with the clipped scene marker.
class _LinuxInputSurface extends PlatformViewSurface {
  /// Configures gesture competition with scrollable and draggable ancestors.
  const _LinuxInputSurface({
    super.key,
    required super.controller,
    required super.gestureRecognizers,
  }) : super(hitTestBehavior: PlatformViewHitTestBehavior.opaque);

  /// Creates the standard platform input surface without a duplicate layer.
  @override
  PlatformViewRenderBox createRenderObject(BuildContext context) =>
      _LinuxInputRenderBox(
        controller: controller,
        gestureRecognizers: gestureRecognizers,
      );
}

/// Keeps Flutter's platform-view input and semantics behavior in one place.
class _LinuxInputRenderBox extends PlatformViewRenderBox {
  /// Initializes Flutter's platform-view gesture arena participant.
  _LinuxInputRenderBox({
    required super.controller,
    required super.gestureRecognizers,
  }) : super(hitTestBehavior: PlatformViewHitTestBehavior.opaque);

  /// The enclosing geometry observer emits the clipped platform-view marker.
  @override
  void paint(PaintingContext context, Offset offset) {}
}

class _LinuxGeometryObserver extends SingleChildRenderObjectWidget {
  const _LinuxGeometryObserver({
    super.key,
    required this.nativeViewId,
    required this.onGeometryChanged,
    required this.onDetached,
    required Widget child,
  }) : super(child: child);

  final int? nativeViewId;
  final ValueChanged<_LinuxViewGeometry> onGeometryChanged;
  final VoidCallback onDetached;

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _LinuxGeometryRenderBox(
      nativeViewId: nativeViewId,
      onGeometryChanged: onGeometryChanged,
      onDetached: onDetached,
    );
  }

  @override
  void updateRenderObject(
    BuildContext context,
    covariant _LinuxGeometryRenderBox renderObject,
  ) {
    renderObject
      ..nativeViewId = nativeViewId
      ..onGeometryChanged = onGeometryChanged
      ..onDetached = onDetached;
  }
}

class _LinuxGeometryRenderBox extends RenderProxyBox {
  _LinuxGeometryRenderBox({
    required int? nativeViewId,
    required ValueChanged<_LinuxViewGeometry> onGeometryChanged,
    required VoidCallback onDetached,
  }) : _nativeViewId = nativeViewId,
       _onGeometryChanged = onGeometryChanged,
       _onDetached = onDetached;

  ValueChanged<_LinuxViewGeometry> _onGeometryChanged;
  VoidCallback _onDetached;
  bool _reportedUnsupportedTransform = false;
  bool _reportedUnsupportedClip = false;
  int? _nativeViewId;
  _LinuxViewGeometry _paintGeometry = _LinuxViewGeometry.hidden;

  /// Makes the platform-view marker participate in the composited scene.
  @override
  bool get alwaysNeedsCompositing => _nativeViewId != null;

  /// Invalidates the scene when a native controller is attached or replaced.
  set nativeViewId(int? value) {
    if (_nativeViewId == value) {
      return;
    }
    _nativeViewId = value;
    markNeedsCompositingBitsUpdate();
    markNeedsPaint();
  }

  bool get isEffectivelyPainted {
    RenderObject child = this;
    RenderObject? ancestor = parent;
    while (ancestor != null) {
      if (!ancestor.paintsChild(child)) {
        return false;
      }
      child = ancestor;
      ancestor = ancestor.parent;
    }
    return true;
  }

  RenderView? get _renderView {
    RenderObject root = this;
    RenderObject? ancestor = parent;
    while (ancestor != null) {
      root = ancestor;
      ancestor = ancestor.parent;
    }
    return root is RenderView ? root : null;
  }

  set onGeometryChanged(ValueChanged<_LinuxViewGeometry> value) {
    _onGeometryChanged = value;
  }

  set onDetached(VoidCallback value) {
    _onDetached = value;
  }

  @override
  void detach() {
    _onDetached();
    super.detach();
  }

  /// Recomputes the on-screen rect of this box and reports it.
  ///
  /// Called from [paint] and from each already-scheduled Flutter frame so the
  /// native overlay keeps tracking composited movement during sliver scrolling
  /// and can detect when a previously clipped child re-enters the viewport.
  void syncGeometry() {
    _paintGeometry = _LinuxViewGeometry.hidden;
    if (!attached) {
      _onGeometryChanged(_LinuxViewGeometry.hidden);
      return;
    }
    if (!isEffectivelyPainted) {
      _onGeometryChanged(_LinuxViewGeometry.hidden);
      return;
    }
    final RenderView? renderView = _renderView;
    final RenderBox? renderRoot = renderView?.child;
    if (renderView == null || renderRoot == null) {
      _onGeometryChanged(_LinuxViewGeometry.hidden);
      return;
    }
    // GTK and RenderView layout both use logical pixels. Targeting the
    // RenderView child deliberately excludes RenderView's root DPR transform.
    final Matrix4 transform = getTransformTo(renderRoot);
    final Offset topLeft = MatrixUtils.transformPoint(transform, Offset.zero);
    final Offset topRight = MatrixUtils.transformPoint(
      transform,
      Offset(size.width, 0),
    );
    final Offset bottomLeft = MatrixUtils.transformPoint(
      transform,
      Offset(0, size.height),
    );
    final Offset bottomRight = MatrixUtils.transformPoint(
      transform,
      Offset(size.width, size.height),
    );
    const double epsilon = 0.000001;
    final bool translationOnly =
        (topLeft.dy - topRight.dy).abs() <= epsilon &&
        (topLeft.dx - bottomLeft.dx).abs() <= epsilon &&
        (topRight.dx - bottomRight.dx).abs() <= epsilon &&
        (bottomLeft.dy - bottomRight.dy).abs() <= epsilon &&
        ((topRight.dx - topLeft.dx) - size.width).abs() <= epsilon &&
        ((bottomLeft.dy - topLeft.dy) - size.height).abs() <= epsilon;
    if (!translationOnly) {
      if (!_reportedUnsupportedTransform) {
        debugPrint(
          'webview_all_linux: scaled, rotated, skewed, perspective, and '
          'mirrored WebViewWidget transforms cannot be represented by a '
          'native GTK overlay; the native WebView was hidden.',
        );
        _reportedUnsupportedTransform = true;
      }
      _onGeometryChanged(_LinuxViewGeometry.hidden);
      return;
    }

    final Rect frame = Rect.fromPoints(topLeft, bottomRight);
    Rect clip = frame.intersect(Offset.zero & renderView.size);

    RenderObject child = this;
    RenderObject? ancestor = parent;
    while (ancestor != null && ancestor != renderView) {
      final Rect? localClip = ancestor.describeApproximatePaintClip(child);
      if (localClip != null) {
        if (_hasNonRectangularClip(ancestor)) {
          _reportUnsupportedClip();
          _onGeometryChanged(_LinuxViewGeometry.hidden);
          return;
        }
        final Rect? transformedClip = _transformAxisAlignedRect(
          ancestor.getTransformTo(renderRoot),
          localClip,
        );
        if (transformedClip == null) {
          _reportUnsupportedClip();
          _onGeometryChanged(_LinuxViewGeometry.hidden);
          return;
        }
        clip = clip.intersect(transformedClip);
        if (!_LinuxViewGeometry._isUsableRect(clip)) {
          _onGeometryChanged(_LinuxViewGeometry.hidden);
          return;
        }
      }
      if (identical(ancestor, renderRoot)) {
        break;
      }
      child = ancestor;
      ancestor = ancestor.parent;
    }

    _paintGeometry = _LinuxViewGeometry(frame: frame, clip: clip);
    _onGeometryChanged(_paintGeometry);
  }

  bool _hasNonRectangularClip(RenderObject ancestor) {
    if (ancestor is RenderClipRRect ||
        ancestor is RenderClipRSuperellipse ||
        ancestor is RenderClipOval ||
        ancestor is RenderClipPath ||
        ancestor is RenderPhysicalShape) {
      return true;
    }
    if (ancestor is RenderPhysicalModel) {
      return ancestor.shape != BoxShape.rectangle ||
          (ancestor.borderRadius != null &&
              ancestor.borderRadius != BorderRadius.zero);
    }
    return false;
  }

  Rect? _transformAxisAlignedRect(Matrix4 transform, Rect rect) {
    final List<Offset> corners = <Offset>[
      MatrixUtils.transformPoint(transform, rect.topLeft),
      MatrixUtils.transformPoint(transform, rect.topRight),
      MatrixUtils.transformPoint(transform, rect.bottomRight),
      MatrixUtils.transformPoint(transform, rect.bottomLeft),
    ];
    if (corners.any(
      (Offset point) => !point.dx.isFinite || !point.dy.isFinite,
    )) {
      return null;
    }

    const double epsilon = 0.000001;
    bool horizontal(Offset first, Offset second) =>
        (first.dy - second.dy).abs() <= epsilon;
    bool vertical(Offset first, Offset second) =>
        (first.dx - second.dx).abs() <= epsilon;

    final bool horizontalFirst =
        horizontal(corners[0], corners[1]) &&
        vertical(corners[1], corners[2]) &&
        horizontal(corners[2], corners[3]) &&
        vertical(corners[3], corners[0]);
    final bool verticalFirst =
        vertical(corners[0], corners[1]) &&
        horizontal(corners[1], corners[2]) &&
        vertical(corners[2], corners[3]) &&
        horizontal(corners[3], corners[0]);
    if (!horizontalFirst && !verticalFirst) {
      return null;
    }

    return MatrixUtils.transformRect(transform, rect);
  }

  void _reportUnsupportedClip() {
    if (_reportedUnsupportedClip) {
      return;
    }
    debugPrint(
      'webview_all_linux: non-rectangular or transformed Flutter clips cannot '
      'be represented by a native GTK overlay; the native WebView was hidden.',
    );
    _reportedUnsupportedClip = true;
  }

  @override
  void paint(PaintingContext context, Offset offset) {
    super.paint(context, offset);
    syncGeometry();
    final int? nativeViewId = _nativeViewId;
    if (nativeViewId != null && _paintGeometry.isVisible) {
      final Rect viewport = _paintGeometry.clip.shift(
        offset - _paintGeometry.frame.topLeft,
      );
      // Platform-view preroll takes placement from the scene transform stack.
      // Keep the marker at its local origin and translate its containing layer.
      final OffsetLayer placement = OffsetLayer(offset: viewport.topLeft);
      placement.append(
        PlatformViewLayer(
          rect: Offset.zero & viewport.size,
          viewId: nativeViewId,
        ),
      );
      context.addLayer(placement);
    }
  }
}
