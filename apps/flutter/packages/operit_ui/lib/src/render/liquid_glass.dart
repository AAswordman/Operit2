import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';

/// Preloads the shared optical shader before displaying a glass scene.
Future<void> prepareLiquidGlass() async {
  await _LiquidBackdrop.loadProgram();
}

/// Shares one live background texture between the glass surfaces in a scene.
class LiquidGlassBackdrop extends StatefulWidget {
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
      child: _LiquidScene(
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
    _backdrop!.source!.markNeedsCompositedLayerUpdate();
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
    if (_surfaceCount == 1) notifyListeners();
  }

  /// Stops texture frame requests when the final lens leaves the scene.
  void removeSurface(VoidCallback listener) {
    super.removeListener(listener);
    _surfaceCount--;
    if (_surfaceCount == 0 && !_disposed) notifyListeners();
  }

  /// Requests a paint when the asynchronously compiled shader becomes ready.
  Future<void> _prepareProgram() async {
    final loaded = await loadProgram();
    if (_disposed) return;
    program = loaded;
    notifyListeners();
  }

  /// Releases the scene without leaving any pending capture callbacks.
  @override
  void dispose() {
    _disposed = true;
    source = null;
    super.dispose();
  }
}

class _LiquidScene extends SingleChildRenderObjectWidget {
  /// Wraps the scene in a composition layer shared by every lens.
  const _LiquidScene({required this.backdrop, required super.child});

  final _LiquidBackdrop backdrop;

  /// Creates the render object that owns the composition layer.
  @override
  _RenderLiquidScene createRenderObject(BuildContext context) =>
      _RenderLiquidScene(backdrop);

  /// Keeps the composition layer aligned with its source owner.
  @override
  void updateRenderObject(
    BuildContext context,
    _RenderLiquidScene renderObject,
  ) {
    renderObject.backdrop = backdrop;
  }
}

class _RenderLiquidScene extends RenderProxyBox {
  /// Stores the source owner until this scene paints.
  _RenderLiquidScene(this.backdrop);

  _LiquidBackdrop backdrop;

  /// Composes source textures after all repaint boundaries have painted.
  @override
  bool get alwaysNeedsCompositing => true;

  /// Inserts one shared texture capture before composing the scene's lenses.
  @override
  void paint(PaintingContext context, Offset offset) {
    final sceneLayer = layer as _LiquidSceneLayer? ?? _LiquidSceneLayer();
    sceneLayer.backdrop = backdrop;
    layer = sceneLayer;
    context.pushLayer(sceneLayer, super.paint, offset);
  }
}

class _LiquidSceneLayer extends ContainerLayer {
  late _LiquidBackdrop backdrop;

  /// Samples on every submitted scene, including retained paint and textures.
  @override
  bool get alwaysNeedsAddToScene => true;

  /// Captures a fully painted source once and shares it within this composition.
  @override
  void addToScene(ui.SceneBuilder builder) {
    final source = backdrop.source!;
    if (backdrop.hasSurfaces &&
        backdrop.program != null &&
        !source.size.isEmpty) {
      final image = source.captureTexture(backdrop._pixelRatio);
      backdrop.image = image;
      try {
        addChildrenToScene(builder);
      } finally {
        backdrop.image = null;
        image.dispose();
      }
    } else {
      addChildrenToScene(builder);
    }
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
    _backdrop.source = null;
    _backdrop = value;
    value.source = this;
    markNeedsPaint();
  }

  /// Samples the completed layer during scene composition on the raster thread.
  ui.Image captureTexture(double pixelRatio) {
    assert(!debugNeedsPaint);
    return (layer! as OffsetLayer).toImageSync(
      Offset.zero & size,
      pixelRatio: pixelRatio,
    );
  }

  /// Removes this boundary from the scene before releasing its layer.
  @override
  void dispose() {
    if (_backdrop.source == this) _backdrop.source = null;
    super.dispose();
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

  /// Subscribes to a replacement scene and releases the old shader sampler.
  set backdrop(_LiquidBackdrop value) {
    if (value == _backdrop) return;
    if (attached) _backdrop.removeSurface(markNeedsPaint);
    _shader?.dispose();
    _shader = null;
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
      for (var i = 0; i < values.length; i++) {
        shader.setFloat(i, values[i]);
      }
      shader.setImageSampler(0, image);
      final recorder = ui.PictureRecorder();
      Canvas(recorder).drawRect(offset & size, Paint()..shader = shader);
      final picture = recorder.endRecording();
      builder.addPicture(Offset.zero, picture);
      picture.dispose();
    }
  }

  /// Releases this lens's shader and its reference to the sampled image.
  @override
  void dispose() {
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
