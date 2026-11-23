import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';

/// Preloads the shared optical shader before displaying a glass scene.
Future<void> prepareLiquidGlass() async {
  await _LiquidBackdrop.loadProgram();
}

/// Shares one live background texture between the glass surfaces in a scene.
class LiquidGlassBackdrop extends StatefulWidget {
  /// Observes real texture captures in debug tests without changing rendering.
  @visibleForTesting
  static VoidCallback? debugOnCapture;

  /// Creates the background capture and its foreground composition.
  const LiquidGlassBackdrop({
    super.key,
    required this.background,
    required this.child,
    this.fit = StackFit.expand,
  });

  final Widget background;
  final Widget child;
  final StackFit fit;

  /// Creates the owner of the scene's captured texture.
  @override
  State<LiquidGlassBackdrop> createState() => _LiquidGlassBackdropState();
}

class _LiquidGlassBackdropState extends State<LiquidGlassBackdrop>
    with WidgetsBindingObserver {
  final _backdrop = _LiquidBackdrop();
  late ui.FlutterView _view;

  /// Observes view metrics so moving between displays updates texture density.
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  /// Keeps source sampling at the view's physical pixel density.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _view = View.of(context);
    _backdrop.pixelRatio = _view.devicePixelRatio;
  }

  /// Refreshes source density when the owning view changes its pixel ratio.
  @override
  void didChangeMetrics() {
    _backdrop.pixelRatio = _view.devicePixelRatio;
  }

  /// Shares a single source capture across all lenses in each composed scene.
  @override
  Widget build(BuildContext context) {
    return _LiquidBackdropScope(
      backdrop: _backdrop,
      child: Stack(
        fit: widget.fit,
        children: <Widget>[
          Positioned.fill(
            child: _LiquidBackground(
              backdrop: _backdrop,
              child: widget.background,
            ),
          ),
          widget.child,
        ],
      ),
    );
  }

  /// Releases the scene's captured image when its owner leaves the tree.
  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _backdrop.dispose();
    super.dispose();
  }
}

/// Requests composition frames while an external background texture is playing.
class LiquidGlassSourceAnimation extends StatefulWidget {
  /// Connects texture playback to glass without repainting retained content.
  const LiquidGlassSourceAnimation({
    super.key,
    required this.active,
    required this.child,
  });

  final bool active;
  final Widget child;

  /// Creates the playback-scoped frame subscription.
  @override
  State<LiquidGlassSourceAnimation> createState() =>
      _LiquidGlassSourceAnimationState();
}

class _LiquidGlassSourceAnimationState extends State<LiquidGlassSourceAnimation>
    with SingleTickerProviderStateMixin {
  late final Ticker _ticker;
  _LiquidBackdrop? _backdrop;

  /// Creates a ticker governed by Flutter's normal visibility controls.
  @override
  void initState() {
    super.initState();
    _ticker = createTicker(_requestComposition);
  }

  /// Binds playback to the scene whose background contains this texture.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final scope = context
        .dependOnInheritedWidgetOfExactType<_LiquidBackdropScope>();
    if (scope == null) {
      throw FlutterError(
        'LiquidGlassSourceAnimation requires a LiquidGlassBackdrop ancestor.',
      );
    }
    if (_backdrop != scope.backdrop) {
      _backdrop?.removeListener(_syncTicker);
      _backdrop = scope.backdrop;
      _backdrop!.addListener(_syncTicker);
    }
    _syncTicker();
  }

  /// Starts or stops frame requests when the actual player changes state.
  @override
  void didUpdateWidget(LiquidGlassSourceAnimation oldWidget) {
    super.didUpdateWidget(oldWidget);
    _syncTicker();
  }

  /// Drives frames only for playing textures in scenes with optical surfaces.
  void _syncTicker() {
    final active = widget.active && _backdrop!.hasSurfaces;
    if (active == _ticker.isActive) return;
    if (active) {
      _ticker.start();
    } else {
      _ticker.stop();
    }
  }

  /// Updates composition without invalidating background or foreground paint.
  void _requestComposition(Duration elapsed) {
    _backdrop!.source!.invalidateTexture();
  }

  /// Preserves the player's layout and independently updated texture layer.
  @override
  Widget build(BuildContext context) => widget.child;

  /// Stops frame production before the player and scene are released.
  @override
  void dispose() {
    _backdrop?.removeListener(_syncTicker);
    _ticker.dispose();
    super.dispose();
  }
}

/// Renders the optical model ported from FletchMcKee/liquid 1.1.1.
class LiquidGlassSurface extends StatelessWidget {
  /// Creates a lens whose tint is independent of the sampled background alpha.
  const LiquidGlassSurface({
    super.key,
    required this.borderRadius,
    required this.child,
    this.tint = const Color(0x0AFFFFFF),
    this.refraction = 0.25,
    this.curve = 0.25,
    this.edge = 0.008,
    this.saturation = 1,
    this.dispersion = 0.025,
    this.contrast = 1,
  });

  final BorderRadius borderRadius;
  final Widget child;
  final Color tint;
  final double refraction;
  final double curve;
  final double edge;
  final double saturation;
  final double dispersion;
  final double contrast;

  /// Resolves the explicit scene source used by every lens in this subtree.
  @override
  Widget build(BuildContext context) {
    final scope = context
        .dependOnInheritedWidgetOfExactType<_LiquidBackdropScope>();
    if (scope == null) {
      throw FlutterError(
        'LiquidGlassSurface requires a LiquidGlassBackdrop ancestor.',
      );
    }
    return _LiquidGlassPaint(
      backdrop: scope.backdrop,
      surface: this,
      child: child,
    );
  }
}

class _LiquidBackdropScope extends InheritedWidget {
  /// Publishes the background owner without rebuilding on every captured frame.
  const _LiquidBackdropScope({required this.backdrop, required super.child});

  final _LiquidBackdrop backdrop;

  /// Rebinds lenses only when their background owner changes.
  @override
  bool updateShouldNotify(_LiquidBackdropScope oldWidget) =>
      backdrop != oldWidget.backdrop;
}

class _LiquidBackdrop extends ChangeNotifier {
  /// Reuses a program already prepared by the application or preview startup.
  _LiquidBackdrop() : program = _cachedProgram;

  static ui.FragmentProgram? _cachedProgram;

  /// Caches the compiled program while keeping asset futures local to the caller.
  static Future<ui.FragmentProgram> loadProgram() async {
    _cachedProgram ??= await ui.FragmentProgram.fromAsset(
      'packages/operit_ui/shaders/liquid_glass.frag',
    );
    return _cachedProgram!;
  }

  _RenderLiquidBackground? source;
  ui.FragmentProgram? program;
  ui.Image? image;
  bool _loading = false;
  bool _disposed = false;
  double _pixelRatio = 1;
  int _surfaceCount = 0;

  /// Reports whether the scene contains attached optical surfaces.
  bool get hasSurfaces => _surfaceCount > 0;

  /// Updates the density used by the next composed source texture.
  set pixelRatio(double value) {
    if (_pixelRatio == value) return;
    _pixelRatio = value;
    source?.invalidateTexture();
    notifyListeners();
  }

  /// Starts shader preparation and activates texture frames for the first lens.
  void addSurface(VoidCallback listener) {
    _surfaceCount++;
    super.addListener(listener);
    if (program == null && !_loading) {
      _loading = true;
      unawaited(_prepareProgram());
    }
    if (_surfaceCount == 1) {
      source?.invalidateTexture();
      notifyListeners();
    }
  }

  /// Stops texture frame requests when the final lens leaves the scene.
  void removeSurface(VoidCallback listener) {
    super.removeListener(listener);
    _surfaceCount--;
    if (_surfaceCount == 0 && !_disposed) {
      releaseImage();
      notifyListeners();
    }
  }

  /// Requests a paint when the asynchronously compiled shader becomes ready.
  Future<void> _prepareProgram() async {
    final loaded = await loadProgram();
    if (_disposed) return;
    program = loaded;
    source?.invalidateTexture();
    notifyListeners();
  }

  /// Releases the retained source when its scene or final lens goes away.
  void releaseImage() {
    image?.dispose();
    image = null;
  }

  /// Releases the scene without leaving any pending capture callbacks.
  @override
  void dispose() {
    _disposed = true;
    source = null;
    releaseImage();
    super.dispose();
  }
}

class _LiquidBackground extends SingleChildRenderObjectWidget {
  /// Creates a paint-tracked source boundary for the scene.
  const _LiquidBackground({required this.backdrop, required super.child});

  final _LiquidBackdrop backdrop;

  /// Creates the independent background layer.
  @override
  _RenderLiquidBackground createRenderObject(BuildContext context) =>
      _RenderLiquidBackground(backdrop);

  /// Keeps source ownership aligned with the current scene.
  @override
  void updateRenderObject(
    BuildContext context,
    _RenderLiquidBackground renderObject,
  ) {
    renderObject.backdrop = backdrop;
  }
}

class _RenderLiquidBackground extends RenderRepaintBoundary {
  /// Registers this boundary as the source of its scene.
  _RenderLiquidBackground(this._backdrop) {
    _backdrop.source = this;
  }

  _LiquidBackdrop _backdrop;

  /// Moves this boundary to a different scene owner.
  set backdrop(_LiquidBackdrop value) {
    if (value == _backdrop) return;
    _backdrop.releaseImage();
    _backdrop.source = null;
    _backdrop = value;
    value.source = this;
    markNeedsPaint();
  }

  /// Invalidates external texture frames and density without repainting content.
  void invalidateTexture() {
    (layer as _LiquidBackgroundLayer?)?.invalidateTexture();
    markNeedsCompositedLayerUpdate();
  }

  /// Lets Flutter's retained layer tree detect source and descendant changes.
  @override
  OffsetLayer updateCompositedLayer({
    required _LiquidBackgroundLayer? oldLayer,
  }) {
    final result = oldLayer ?? _LiquidBackgroundLayer();
    result.configure(_backdrop, Offset.zero & size);
    return result;
  }

  /// Removes this boundary from the scene before releasing its layer.
  @override
  void dispose() {
    if (_backdrop.source == this) {
      _backdrop.source = null;
      _backdrop.releaseImage();
    }
    super.dispose();
  }
}

class _LiquidBackgroundLayer extends OffsetLayer {
  _LiquidBackdrop? _backdrop;
  Rect _bounds = Rect.zero;
  double _pixelRatio = 0;
  bool _capturing = false;
  bool _hasLiveTexture = false;

  /// External texture pixels can change without any Dart layer invalidation.
  @override
  bool get alwaysNeedsAddToScene => _hasLiveTexture;

  /// Rebuilds the full-resolution capture when source ownership or size changes.
  void configure(_LiquidBackdrop backdrop, Rect bounds) {
    if (!identical(_backdrop, backdrop) ||
        _bounds != bounds ||
        _pixelRatio != backdrop._pixelRatio) {
      _backdrop = backdrop;
      _bounds = bounds;
      _pixelRatio = backdrop._pixelRatio;
      invalidateTexture();
    }
  }

  /// Marks externally updated textures dirty through the normal layer protocol.
  void invalidateTexture() {
    if (!alwaysNeedsAddToScene) markNeedsAddToScene();
  }

  /// Identifies native textures when the source layer tree actually changes.
  bool _containsLiveTexture(ContainerLayer parent) {
    for (
      Layer? child = parent.firstChild;
      child != null;
      child = child.nextSibling
    ) {
      if (child is TextureLayer && !child.freeze) return true;
      if (child is ContainerLayer && _containsLiveTexture(child)) return true;
    }
    return false;
  }

  /// Captures changed sources before lenses, retaining static sources otherwise.
  @override
  void addToScene(ui.SceneBuilder builder) {
    final backdrop = _backdrop!;
    if (!_capturing) _hasLiveTexture = _containsLiveTexture(this);
    if (!_capturing &&
        backdrop.hasSurfaces &&
        backdrop.program != null &&
        !_bounds.isEmpty) {
      _capturing = true;
      try {
        final image = toImageSync(_bounds, pixelRatio: _pixelRatio);
        backdrop.releaseImage();
        backdrop.image = image;
        assert(() {
          LiquidGlassBackdrop.debugOnCapture?.call();
          return true;
        }());
      } finally {
        _capturing = false;
      }
    }
    super.addToScene(builder);
  }
}

class _LiquidGlassPaint extends SingleChildRenderObjectWidget {
  /// Binds the lens description to a render object without per-lens captures.
  const _LiquidGlassPaint({
    required this.backdrop,
    required this.surface,
    required super.child,
  });

  final _LiquidBackdrop backdrop;
  final LiquidGlassSurface surface;

  /// Creates one shader instance for this surface's uniforms.
  @override
  _RenderLiquidGlass createRenderObject(BuildContext context) =>
      _RenderLiquidGlass(backdrop, surface);

  /// Updates material properties without changing the background texture owner.
  @override
  void updateRenderObject(
    BuildContext context,
    _RenderLiquidGlass renderObject,
  ) {
    renderObject
      ..backdrop = backdrop
      ..surface = surface;
  }
}

class _RenderLiquidGlass extends RenderProxyBox {
  /// Stores the scene and material until this lens attaches.
  _RenderLiquidGlass(this._backdrop, this._surface);

  _LiquidBackdrop _backdrop;
  LiquidGlassSurface _surface;
  ui.FragmentShader? _shader;
  ui.Image? _pictureImage;
  List<double>? _pictureValues;
  ui.Picture? _picture;

  /// Subscribes to a replacement scene and releases the old shader sampler.
  set backdrop(_LiquidBackdrop value) {
    if (value == _backdrop) return;
    if (attached) _backdrop.removeSurface(markNeedsPaint);
    _shader?.dispose();
    _shader = null;
    _releasePicture();
    _backdrop = value;
    if (attached) _backdrop.addSurface(markNeedsPaint);
    markNeedsPaint();
  }

  /// Updates the optical uniforms when the surface description changes.
  set surface(LiquidGlassSurface value) {
    _surface = value;
    markNeedsPaint();
  }

  /// Listens for new background frames while the surface is visible.
  @override
  void attach(PipelineOwner owner) {
    super.attach(owner);
    _backdrop.addSurface(markNeedsPaint);
  }

  /// Stops listening when this render object leaves the active pipeline.
  @override
  void detach() {
    _backdrop.removeSurface(markNeedsPaint);
    super.detach();
  }

  /// Retains foreground pictures while refreshing the optical layer each scene.
  @override
  bool get alwaysNeedsCompositing => true;

  /// Records foreground content independently from the live shader picture.
  @override
  void paint(PaintingContext context, Offset offset) {
    final opticalLayer = layer as _LiquidLensLayer? ?? _LiquidLensLayer(this);
    opticalLayer.offset = offset;
    layer = opticalLayer;
    context.pushLayer(opticalLayer, super.paint, offset);
  }

  /// Composes the lens before its retained foreground picture.
  void compose(ui.SceneBuilder builder, Offset offset) {
    final image = _backdrop.image;
    final program = _backdrop.program;
    final source = _backdrop.source;
    if (image != null && program != null && source != null && !size.isEmpty) {
      final shader = _shader ??= program.fragmentShader();
      final transform = getTransformTo(source);
      final origin = MatrixUtils.transformPoint(transform, Offset.zero);
      final xAxis =
          MatrixUtils.transformPoint(transform, const Offset(1, 0)) - origin;
      final yAxis =
          MatrixUtils.transformPoint(transform, const Offset(0, 1)) - origin;
      final rect = Offset.zero & size;
      final radii = _surface.borderRadius.toRRect(rect).scaleRadii();
      final minDimension = size.shortestSide;
      final values = <double>[
        size.width, size.height, offset.dx, offset.dy,
        // Match upstream's bottom-right, top-right, bottom-left, top-left order.
        (radii.brRadiusX / minDimension).clamp(0, 0.5),
        (radii.trRadiusX / minDimension).clamp(0, 0.5),
        (radii.blRadiusX / minDimension).clamp(0, 0.5),
        (radii.tlRadiusX / minDimension).clamp(0, 0.5),
        _surface.refraction, _surface.curve, _surface.edge, 0,
        _surface.tint.r, _surface.tint.g, _surface.tint.b, _surface.tint.a,
        _surface.saturation, _surface.dispersion, _surface.contrast, 0,
        origin.dx, origin.dy, source.size.width, source.size.height,
        xAxis.dx, xAxis.dy, yAxis.dx, yAxis.dy,
      ];
      if (!identical(_pictureImage, image) ||
          !listEquals(_pictureValues, values)) {
        for (var i = 0; i < values.length; i++) {
          shader.setFloat(i, values[i]);
        }
        shader.setImageSampler(0, image);
        final recorder = ui.PictureRecorder();
        Canvas(recorder).drawRect(offset & size, Paint()..shader = shader);
        _releasePicture();
        _picture = recorder.endRecording();
        _pictureImage = image;
        _pictureValues = values;
      }
      builder.addPicture(Offset.zero, _picture!);
    }
  }

  /// Releases recorded optical content when any sampler or uniform changes.
  void _releasePicture() {
    _picture?.dispose();
    _picture = null;
    _pictureImage = null;
    _pictureValues = null;
  }

  /// Releases this lens's shader and its reference to the sampled image.
  @override
  void dispose() {
    _releasePicture();
    _shader?.dispose();
    super.dispose();
  }
}

class _LiquidLensLayer extends ContainerLayer {
  /// Stores the lens whose transform and uniforms are evaluated at composition.
  _LiquidLensLayer(this.lens);

  final _RenderLiquidGlass lens;
  Offset offset = Offset.zero;

  /// Refreshes the shader even when its foreground picture remains retained.
  @override
  bool get alwaysNeedsAddToScene => true;

  /// Adds the current lens texture before its sharp foreground children.
  @override
  void addToScene(ui.SceneBuilder builder) {
    lens.compose(builder, offset);
    addChildrenToScene(builder);
  }
}
