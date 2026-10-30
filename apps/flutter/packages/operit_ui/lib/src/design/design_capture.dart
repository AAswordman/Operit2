/// Opt-in, offline capture of the live render tree for the Figma design bridge.
library;

import 'dart:async';
import 'dart:convert';
import 'dart:developer' as developer;
import 'dart:ui' as ui;

import 'design_scenarios.dart';

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';

part 'design_runtime.dart';

/// Enabled explicitly in debug/profile builds; absent from ordinary app flows.
const operitDesignBridgeEnabled =
    bool.fromEnvironment('OPERIT_DESIGN_BRIDGE') && !kReleaseMode;

/// Identifies the owning tool or plugin without requiring a render boundary.
class OperitDesignSource extends InheritedWidget {
  const OperitDesignSource({
    super.key,
    required this.source,
    required super.child,
  });
  final Map<String, Object?> source;

  @override
  bool updateShouldNotify(OperitDesignSource oldWidget) => false;
}

/// A WebView can provide editable DOM layers in its own logical coordinates.
typedef OperitDesignSurfaceReader = Future<Map<String, Object?>> Function();
typedef OperitDesignSurfaceAction =
    Future<Object?> Function(Map<String, Object?> request);

/// Keeps platform surfaces addressable only for the lifetime of their widgets.
class OperitDesignSurfaces {
  static final Map<Element, OperitDesignSurfaceReader> _readers = {};
  static final Map<Element, OperitDesignSurfaceAction> _actions = {};

  static void register(
    BuildContext context,
    OperitDesignSurfaceReader reader, {
    OperitDesignSurfaceAction? onRuntimeRequest,
  }) {
    if (operitDesignBridgeEnabled) {
      _readers[context as Element] = reader;
      if (onRuntimeRequest != null) _actions[context] = onRuntimeRequest;
    }
  }

  static void unregister(BuildContext context) {
    _readers.remove(context);
    _actions.remove(context);
  }
}

/// Marks only the platform content, preserving Flutter overlays as siblings.
class OperitDesignSurface extends StatefulWidget {
  const OperitDesignSurface({
    super.key,
    required this.readScene,
    this.onRuntimeRequest,
    required this.child,
  });
  final OperitDesignSurfaceReader readScene;
  final OperitDesignSurfaceAction? onRuntimeRequest;
  final Widget child;
  @override
  State<OperitDesignSurface> createState() => _OperitDesignSurfaceState();
}

class _OperitDesignSurfaceState extends State<OperitDesignSurface> {
  @override
  void initState() {
    super.initState();
    _register();
  }

  void _register() {
    OperitDesignSurfaces.unregister(context);
    OperitDesignSurfaces.register(
      context,
      () => widget.readScene(),
      onRuntimeRequest: widget.onRuntimeRequest == null
          ? null
          : (request) => widget.onRuntimeRequest!(request),
    );
  }

  @override
  void didUpdateWidget(OperitDesignSurface oldWidget) {
    super.didUpdateWidget(oldWidget);
    _register();
  }

  @override
  void dispose() {
    OperitDesignSurfaces.unregister(context);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.child;
}

/// Wraps an application (including overlays) in a captureable root.
class OperitDesignCaptureScope extends StatefulWidget {
  const OperitDesignCaptureScope({
    super.key,
    required this.child,
    this.name = 'Operit',
  });
  final Widget child;
  final String name;

  @override
  State<OperitDesignCaptureScope> createState() =>
      _OperitDesignCaptureScopeState();
}

class _OperitDesignCaptureScopeState extends State<OperitDesignCaptureScope> {
  static final Map<String, _OperitDesignCaptureScopeState> _scopes = {};
  static bool _registered = false;
  final _boundary = GlobalKey();
  OperitDesignRuntime? _runtimeValue;
  OperitDesignRuntime get _runtime => _runtimeValue ??= OperitDesignRuntime(
    root: () =>
        _boundary.currentContext!.findRenderObject()! as RenderRepaintBoundary,
    name: widget.name,
  );

  @override
  void initState() {
    super.initState();
    if (operitDesignBridgeEnabled) {
      _scopes[widget.name] = this;
      if (!_registered) {
        _registered = true;
        developer.registerExtension('ext.operit.design.runtime', (
          method,
          args,
        ) async {
          try {
            final scope = args['scope'] == null
                ? _scopes.values.first
                : _scopes[args['scope']];
            if (scope == null) throw StateError('Unknown design runtime scope');
            final request = jsonDecode(args['request'] ?? '{}');
            if (request is! Map<String, dynamic>) {
              throw const FormatException('Runtime request must be an object');
            }
            final response = await scope._runtime.request(request);
            return developer.ServiceExtensionResponse.result(
              jsonEncode(response),
            );
          } catch (error, stack) {
            return developer.ServiceExtensionResponse.error(
              developer.ServiceExtensionResponse.extensionError,
              '$error\n$stack',
            );
          }
        });
        developer.registerExtension('ext.operit.design.capture', (
          method,
          args,
        ) async {
          try {
            if (args['catalog'] == 'true') {
              return developer.ServiceExtensionResponse.result(
                jsonEncode({'scenarios': OperitDesignScenarios.list()}),
              );
            }
            if (args['restore'] == 'true') {
              await OperitDesignScenarios.restore();
              return developer.ServiceExtensionResponse.result(
                jsonEncode({'restored': true}),
              );
            }
            if (args['list'] == 'true') {
              return developer.ServiceExtensionResponse.result(
                jsonEncode({'scopes': _scopes.keys.toList()}),
              );
            }
            final scope = args['scope'] == null
                ? _scopes.values.first
                : _scopes[args['scope']];
            if (scope == null) throw StateError('Unknown design capture scope');
            final scenario = args['scenario'];
            if (scenario != null) await OperitDesignScenarios.open(scenario);
            final settleMs = int.tryParse(args['settleMs'] ?? '0') ?? 0;
            if (settleMs < 0 || settleMs > 10000) {
              throw ArgumentError('settleMs must be between 0 and 10000');
            }
            if (settleMs > 0) {
              await Future<void>.delayed(Duration(milliseconds: settleMs));
            }
            WidgetsBinding.instance.scheduleFrame();
            await WidgetsBinding.instance.endOfFrame;
            final boundary =
                scope._boundary.currentContext!.findRenderObject()!
                    as RenderRepaintBoundary;
            final document = await captureOperitDesign(
              boundary,
              name: args['name'] ?? scope.widget.name,
            );
            return developer.ServiceExtensionResponse.result(
              jsonEncode({'document': document}),
            );
          } catch (error, stack) {
            return developer.ServiceExtensionResponse.error(
              developer.ServiceExtensionResponse.extensionError,
              '$error\n$stack',
            );
          }
        });
      }
    }
  }

  @override
  void dispose() {
    _runtimeValue?.dispose();
    if (identical(_scopes[widget.name], this)) _scopes.remove(widget.name);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => operitDesignBridgeEnabled
      ? RepaintBoundary(key: _boundary, child: widget.child)
      : widget.child;
}

/// Captures visible content; lazy, collapsed and offscreen states are captured
/// separately after navigating/expanding/scrolling them in the running app.
@visibleForTesting
Future<Map<String, Object?>> captureOperitDesign(
  RenderRepaintBoundary root, {
  String name = 'Operit',
  bool includeReference = true,
  OperitDesignRuntime? runtime,
}) async {
  if (root.debugNeedsPaint) {
    throw StateError('Wait for a painted frame before capturing');
  }
  final capture = _DesignCapture(root, runtime: runtime);
  try {
    return await capture.run(name, includeReference: includeReference);
  } finally {
    capture.reference?.dispose();
  }
}

class _DesignCapture {
  _DesignCapture(this.root, {this.runtime}) {
    if (runtime != null) {
      warnings
        ..clear()
        ..add('界面持续跟随真实应用；当前仅支持运行端声明的控件属性。')
        ..add('未挂载或视口外的内容将在实际导航、展开或滚动后进入界面。');
    }
  }
  final RenderRepaintBoundary root;
  final OperitDesignRuntime? runtime;
  final List<Map<String, Object?>> assets = [];
  final Set<String> warnings = {
    '仅采集当前可见状态。请分别展开工具结果、切换插件页面并滚动采集；未挂载的懒加载内容不会凭空生成。',
    'Figma 布局与文字修改保留为带来源的设计交接；应用行为仍由 Flutter / 插件代码实现。',
  };
  ui.Image? reference;
  int _nextId = 0;
  int _bytes = 0;
  final Map<ui.Image, String> _images = {};
  final Set<Element> _capturedSurfaces = {};

  String color(Color color) {
    final value = color.toARGB32();
    return '#${(value & 0xffffff).toRadixString(16).padLeft(6, '0')}${(value >> 24).toRadixString(16).padLeft(2, '0')}'
        .toUpperCase();
  }

  Future<String> image(ui.Image image) async {
    final existing = _images[image];
    if (existing != null) return existing;
    final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
    if (bytes == null) throw StateError('Unable to encode design asset');
    _bytes += bytes.lengthInBytes;
    if (_bytes > 40 * 1024 * 1024) {
      throw StateError(
        'Capture exceeds 40 MiB of image assets; capture a smaller viewport',
      );
    }
    final path = 'assets/images/capture_${assets.length}.png';
    assets.add({
      'path': path,
      'mediaType': 'image/png',
      'base64': base64Encode(
        bytes.buffer.asUint8List(bytes.offsetInBytes, bytes.lengthInBytes),
      ),
    });
    _images[image] = path;
    return path;
  }

  Rect bounds(RenderObject object) => MatrixUtils.transformRect(
    object.getTransformTo(root),
    object.paintBounds,
  );

  Element? creator(RenderObject object) {
    final creator = object.debugCreator;
    return creator is DebugCreator ? creator.element : null;
  }

  Map<String, Object?> source(RenderObject object) {
    final result = <String, Object?>{
      'renderObject': object.runtimeType.toString(),
    };
    final element = creator(object);
    if (element == null) return result;
    final chain = <String>[];
    final surfaces = <String>{};
    void inspect(Element e) {
      final name = e.widget.runtimeType.toString();
      if (RegExp(r'(Screen|Dialog|Sheet|Panel)$').hasMatch(name)) {
        surfaces.add(name);
      }
      if (chain.length < 16) chain.add(e.widget.runtimeType.toString());
      if (e.widget.key != null && !result.containsKey('widgetKey')) {
        result['widgetKey'] = e.widget.key.toString();
      }
      if (e.widget case OperitDesignSource(:final source)) {
        for (final entry in source.entries) {
          result.putIfAbsent(entry.key, () => entry.value);
        }
      }
    }

    inspect(element);
    element.visitAncestorElements((e) {
      inspect(e);
      return true;
    });
    result['widget'] = element.widget.runtimeType.toString();
    result['ancestors'] = chain;
    if (surfaces.isNotEmpty) result['surfaces'] = surfaces.toList();
    return result;
  }

  Future<Map<String, Object?>> run(
    String name, {
    required bool includeReference,
  }) async {
    if (root.size.isEmpty || root.size.width * root.size.height > 16000000) {
      throw StateError(
        'Capture viewport must contain at most 16 million pixels',
      );
    }
    reference = await root.toImage(pixelRatio: 1);
    final viewport = Offset.zero & root.size;
    final node = await walk(root, viewport, Offset.zero, 0);
    final scenes = <Map<String, Object?>>[node!];
    node['name'] = name;
    if (includeReference) {
      scenes.add({
        'id': 'reference',
        'name': '原始渲染参考（锁定）',
        'type': 'image',
        'x': root.size.width + 80,
        'y': 0,
        'width': root.size.width,
        'height': root.size.height,
        'asset': await image(reference!),
        'source': {'reference': true},
      });
    }
    return {
      'format': 'operit-design',
      'version': 1,
      'name': name,
      'origin': 'flutter-runtime',
      'assets': assets,
      'scenes': scenes,
      'warnings': warnings.toList(),
    };
  }

  Future<String> raster(Rect rect) async {
    final clipped = rect.intersect(Offset.zero & root.size);
    final recorder = ui.PictureRecorder();
    final canvas = Canvas(recorder);
    canvas.drawImageRect(
      reference!,
      clipped,
      Offset.zero & clipped.size,
      Paint(),
    );
    final picture = recorder.endRecording();
    final cropped = await picture.toImage(
      clipped.width.ceil().clamp(1, 16384),
      clipped.height.ceil().clamp(1, 16384),
    );
    picture.dispose();
    try {
      return await image(cropped);
    } finally {
      _images.remove(cropped);
      cropped.dispose();
    }
  }

  List<Map<String, Object?>> textRuns(InlineSpan span) {
    final runs = <Map<String, Object?>>[];
    int offset = 0;
    void visit(InlineSpan current, TextStyle inherited) {
      final style = inherited.merge(current.style);
      if (current is TextSpan) {
        final text = current.text ?? '';
        if (text.isNotEmpty) {
          runs.add({
            'start': offset,
            'end': offset + text.length,
            if (style.color != null) 'color': color(style.color!),
            if (style.fontSize != null) 'fontSize': style.fontSize,
            if (style.fontFamily != null) 'fontFamily': style.fontFamily,
            if (style.fontWeight != null) 'fontWeight': style.fontWeight!.value,
            if (style.fontStyle == FontStyle.italic) 'italic': true,
          });
          offset += text.length;
        }
        for (final child in current.children ?? <InlineSpan>[]) {
          visit(child, style);
        }
      } else {
        offset++;
      }
    }

    visit(span, const TextStyle(fontSize: 14, color: Colors.black));
    return runs;
  }

  Future<Map<String, Object?>?> walk(
    RenderObject object,
    Rect clip,
    Offset parentOrigin,
    int depth,
  ) async {
    if (!object.attached || depth > 512 || _nextId >= 10000) {
      warnings.add('渲染树超过采集深度或 10000 图层限制；请缩小采集范围。');
      return null;
    }
    if (object is RenderOffstage && object.offstage) return null;
    if (object is RenderOpacity && object.opacity == 0) return null;
    if (object is RenderSliver &&
        (object.geometry == null || !object.geometry!.visible)) {
      return null;
    }
    final rect = bounds(object);
    if (!rect.isFinite || rect.isEmpty || !rect.overlaps(clip)) return null;
    final metadata = source(object);
    if (runtime != null) metadata.addAll(runtime!._describe(object));
    final node = <String, Object?>{
      'id': 'render-${_nextId++}',
      'name': metadata['widget'] ?? object.runtimeType.toString(),
      'type': 'frame',
      'x': rect.left - parentOrigin.dx,
      'y': rect.top - parentOrigin.dy,
      'width': rect.width,
      'height': rect.height,
      'source': metadata,
    };
    if (runtime != null && object is RenderPadding) {
      final padding = object.padding.resolve(
        object.textDirection ?? TextDirection.ltr,
      );
      node.addAll({
        'paddingLeft': padding.left,
        'paddingRight': padding.right,
        'paddingTop': padding.top,
        'paddingBottom': padding.bottom,
      });
    }
    if (object is RenderOpacity) node['opacity'] = object.opacity;
    if (object is RenderAnimatedOpacity) node['opacity'] = object.opacity.value;
    final matrix = object.getTransformTo(root);
    if (matrix.storage[1].abs() > 0.001 || matrix.storage[4].abs() > 0.001) {
      warnings.add('旋转／斜切内容按包围框导出，参考图保留原始效果。');
    }
    // Match the exact platform subtree, so progress/error overlays remain
    // editable Flutter siblings. This also works without debugCreator.
    {
      Element? owner;
      for (final entry in OperitDesignSurfaces._readers.keys) {
        if (entry.mounted && identical(entry.findRenderObject(), object)) {
          owner = entry;
          break;
        }
      }
      if (owner != null && !_capturedSurfaces.contains(owner)) {
        _capturedSurfaces.add(owner);
        try {
          final payload = await OperitDesignSurfaces._readers[owner]!().timeout(
            const Duration(seconds: 15),
          );
          final surface = owner.findRenderObject();
          final surfaceRect = surface == null ? rect : bounds(surface);
          final dom = Map<String, Object?>.from(payload['scene'] as Map);
          final cssWidth = (dom['width'] as num).toDouble();
          final cssHeight = (dom['height'] as num).toDouble();
          if (cssWidth <= 0 || cssHeight <= 0) {
            throw StateError('WebView viewport is empty');
          }
          final sx = surfaceRect.width / cssWidth,
              sy = surfaceRect.height / cssHeight;
          final textScale = sx < sy ? sx : sy;
          void scaleDom(Map<String, Object?> n) {
            for (final key in ['x', 'width', 'paddingLeft', 'paddingRight']) {
              if (n[key] is num) n[key] = (n[key] as num) * sx;
            }
            for (final key in ['y', 'height', 'paddingTop', 'paddingBottom']) {
              if (n[key] is num) n[key] = (n[key] as num) * sy;
            }
            for (final key in [
              'fontSize',
              'lineHeight',
              'radius',
              'strokeWidth',
            ]) {
              if (n[key] is num) n[key] = (n[key] as num) * textScale;
            }
            for (final child in n['children'] as List? ?? []) {
              scaleDom(child as Map<String, Object?>);
            }
          }

          scaleDom(dom);
          dom['x'] = surfaceRect.left - parentOrigin.dx;
          dom['y'] = surfaceRect.top - parentOrigin.dy;
          dom['width'] = surfaceRect.width;
          dom['height'] = surfaceRect.height;
          final prefix = 'web-${node['id']}-';
          final pathMap = <String, String>{};
          for (final raw in payload['assets'] as List? ?? []) {
            final asset = Map<String, Object?>.from(raw as Map);
            _bytes += ((asset['base64'] as String).length * 3 / 4).ceil();
            if (_bytes > 40 * 1024 * 1024) {
              throw StateError('WebView assets exceed capture budget');
            }
            final old = asset['path'] as String;
            final path = 'assets/images/$prefix${assets.length}.png';
            pathMap[old] = path;
            asset['path'] = path;
            assets.add(asset);
          }
          void remap(Map<String, Object?> n) {
            n['id'] = '$prefix${n['id']}';
            final webSource = Map<String, Object?>.from(
              n['source'] as Map? ?? {},
            );
            final ownerSource = {...metadata}
              ..removeWhere((key, _) => key.startsWith('runtime'));
            n['source'] = {
              ...ownerSource,
              ...webSource,
              if (runtime != null &&
                  OperitDesignSurfaces._actions.containsKey(owner))
                ...runtime!._describeSurface(owner!, {
                  ...ownerSource,
                  ...webSource,
                }),
            };
            if (n['asset'] is String) {
              n['asset'] = pathMap[n['asset']] ?? n['asset'];
            }
            for (final child in n['children'] as List? ?? []) {
              remap(child as Map<String, Object?>);
            }
          }

          remap(dom);
          for (final warning in payload['warnings'] as List? ?? []) {
            warnings.add(warning.toString());
          }
          return dom;
        } catch (error) {
          warnings.add('WebView DOM 采集失败：$error；保留区域参考图。');
          return await rasterNode(node, rect, 'webview-unavailable');
        }
      }
    }
    if (object is RenderParagraph) {
      final style = object.text.style;
      final plain = object.text.toPlainText();
      if ((style?.fontFamily ?? '').toLowerCase().contains('materialicons')) {
        return rasterNode(node, rect, 'icon-font');
      }
      node.addAll({
        'type': 'text',
        'text': plain,
        'fontSize': style?.fontSize ?? 14,
        'fontWeight': style?.fontWeight?.value ?? 400,
        'fill': color(style?.color ?? Colors.black),
        'fontFamily': style?.fontFamily ?? 'Noto Sans SC',
        'runs': textRuns(object.text),
        'textAlign': switch (object.textAlign) {
          TextAlign.center => 'CENTER',
          TextAlign.right || TextAlign.end => 'RIGHT',
          TextAlign.justify => 'JUSTIFIED',
          _ => 'LEFT',
        },
      });
      final inlineChildren = <RenderObject>[];
      object.visitChildren(inlineChildren.add);
      if (inlineChildren.isNotEmpty) {
        node['text'] = plain.replaceAll('\uFFFC', ' ');
        final textNode = <String, Object?>{
          ...node,
          'id': '${node['id']}-text',
          'x': 0,
          'y': 0,
        };
        node.remove('text');
        node.remove('runs');
        node.remove('fill');
        node['type'] = 'frame';
        final contents = <Map<String, Object?>>[textNode];
        for (final child in inlineChildren) {
          final exported = await walk(
            child,
            clip.intersect(rect),
            rect.topLeft,
            depth + 1,
          );
          if (exported != null) contents.add(exported);
        }
        node['children'] = contents;
      }
      return node;
    }
    if (object is RenderEditable) {
      node.addAll({
        'type': 'text',
        'text': object.obscureText
            ? '••••••'
            : object.text?.toPlainText() ?? '',
        'fontSize': object.text?.style?.fontSize ?? 14,
        'fill': color(object.text?.style?.color ?? Colors.black),
      });
      return node;
    }
    if (object is RenderImage && object.image != null) {
      node.addAll({
        'type': 'image',
        'asset': await image(object.image!),
        'imageFit': object.fit == BoxFit.contain ? 'FIT' : 'FILL',
      });
      if (object.centerSlice != null || object.color != null) {
        warnings.add('九宫格图片与图片滤色保留原素材；实际合成效果见参考图。');
      }
      return node;
    }
    if (object is RenderDecoratedBox && object.decoration is BoxDecoration) {
      final decoration = object.decoration as BoxDecoration;

      if (decoration.color != null) node['fill'] = color(decoration.color!);
      node['radius'] =
          decoration.borderRadius?.resolve(TextDirection.ltr).topLeft.x ?? 0;
      if (decoration.shape == BoxShape.circle) {
        node['radius'] = object.size.shortestSide / 2;
      }
      final border = decoration.border;
      if (border is Border && border.isUniform) {
        node['stroke'] = color(border.top.color);
        node['strokeWidth'] = border.top.width;
      }
    }
    if (object is RenderPhysicalModel) {
      node['fill'] = color(object.color);
      node['radius'] = object.borderRadius?.topLeft.x ?? 0;
      node['clipsContent'] = object.clipBehavior != Clip.none;
    }
    if (object is RenderClipRect ||
        object is RenderClipRRect ||
        object is RenderClipPath) {
      node['clipsContent'] = true;
    }
    if (object is RenderClipRRect) {
      node['radius'] = object.borderRadius.resolve(TextDirection.ltr).topLeft.x;
    }

    if (object.runtimeType.toString().contains('PlatformView') ||
        object.runtimeType.toString().contains('TextureBox')) {
      return rasterNode(node, rect, 'platform-view');
    }
    final children = <RenderObject>[];
    // Navigator's theater retains covered routes in visitChildren, but only
    // its onstage children are painted. Do not expose covered controls as live.
    if (object.runtimeType.toString() == '_RenderTheater') {
      object.visitChildrenForSemantics(children.add);
    } else {
      object.visitChildren(children.add);
    }
    final result = <Map<String, Object?>>[];
    Map<String, Object?>? foreground;
    if (object is RenderCustomPaint) {
      if (object.painter != null) {
        result.add(
          await paintedLayer(
            object,
            rect,
            (canvas) => object.painter!.paint(canvas, object.size),
            'custom-paint',
          ),
        );
      }
      if (object.foregroundPainter != null) {
        foreground = await paintedLayer(
          object,
          rect,
          (canvas) => object.foregroundPainter!.paint(canvas, object.size),
          'custom-paint',
        );
      }
    }
    if (object is RenderDecoratedBox && object.decoration is BoxDecoration) {
      final decoration = object.decoration as BoxDecoration;
      if (decoration.gradient != null || decoration.image != null) {
        final painter = decoration.createBoxPainter();
        try {
          final layer = await paintedLayer(
            object,
            rect,
            (canvas) => painter.paint(
              canvas,
              Offset.zero,
              ImageConfiguration(
                size: object.size,
                textDirection: TextDirection.ltr,
              ),
            ),
            'complex-decoration',
          );
          node.remove('fill');
          node.remove('stroke');
          if (object.position == DecorationPosition.foreground) {
            foreground = layer;
          } else {
            result.add(layer);
          }
        } finally {
          painter.dispose();
        }
      }
    }
    for (final child in children) {
      final localClip = object.describeApproximatePaintClip(child);
      final childClip = localClip == null
          ? clip
          : clip.intersect(
              MatrixUtils.transformRect(object.getTransformTo(root), localClip),
            );
      final exported = await walk(child, childClip, rect.topLeft, depth + 1);
      if (exported != null) result.add(exported);
    }
    if (foreground != null) result.add(foreground);
    if (object is RenderDecoratedBox &&
        object.position == DecorationPosition.foreground &&
        node['fill'] != null &&
        result.isNotEmpty) {
      // Preserve foreground paint order as a final independent layer.
      result.add({
        'id': 'render-${_nextId++}',
        'name': '前景装饰',
        'type': 'rectangle',
        'x': 0,
        'y': 0,
        'width': rect.width,
        'height': rect.height,
        'fill': node.remove('fill'),
        'radius': node['radius'] ?? 0,
      });
    }
    node['children'] = result;
    if (result.isEmpty &&
        node['fill'] == null &&
        node['stroke'] == null &&
        object != root) {
      return null;
    }
    return node;
  }

  Future<Map<String, Object?>> paintedLayer(
    RenderBox object,
    Rect rect,
    void Function(Canvas) paint,
    String reason,
  ) async {
    final recorder = ui.PictureRecorder();
    final canvas = Canvas(recorder);
    canvas.translate(-object.paintBounds.left, -object.paintBounds.top);
    paint(canvas);
    final picture = recorder.endRecording();
    final bitmap = await picture.toImage(
      object.paintBounds.width.ceil().clamp(1, 8192),
      object.paintBounds.height.ceil().clamp(1, 8192),
    );
    picture.dispose();
    try {
      warnings.add('装饰／自定义画布作为独立图片保留，内部子组件仍为可编辑图层。');
      return {
        'id': 'render-${_nextId++}',
        'name': '装饰绘制 / ${object.runtimeType}',
        'type': 'image',
        'x': 0,
        'y': 0,
        'width': rect.width,
        'height': rect.height,
        'asset': await image(bitmap),
        'source': {...source(object), 'rasterReason': reason},
      };
    } finally {
      _images.remove(bitmap);
      bitmap.dispose();
    }
  }

  Future<Map<String, Object?>> rasterNode(
    Map<String, Object?> node,
    Rect rect,
    String reason,
  ) async {
    final clipped = rect.intersect(Offset.zero & root.size);
    node.addAll({
      'type': 'image',
      'asset': await raster(rect),
      'width': clipped.width,
      'height': clipped.height,
    });
    node['x'] = (node['x'] as num) + clipped.left - rect.left;
    node['y'] = (node['y'] as num) + clipped.top - rect.top;
    (node['source'] as Map<String, Object?>)['rasterReason'] = reason;
    warnings.add('部分图层以独立图片保留（$reason），可移动、缩放和替换；内部路径／shader 不会伪装成可编辑文字。');
    return node;
  }
}
