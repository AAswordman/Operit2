part of 'design_capture.dart';

/// A live design session. Events reach the production Flutter widget tree and
/// visual edits are reapplied after rebuilds without replacing widget state.
/// Only installed by the explicit, non-release design bridge.
class OperitDesignRuntime {
  OperitDesignRuntime({required this.root, this.name = 'Operit'}) {
    if (!operitDesignBridgeEnabled) {
      throw StateError(
        'OPERIT_DESIGN_BRIDGE must be enabled in a non-release build',
      );
    }
    _sessions.add(this);
    if (!_frameCallbackInstalled) {
      _frameCallbackInstalled = true;
      WidgetsBinding.instance.addPersistentFrameCallback((_) {
        for (final session in _sessions.toList()) {
          session._synchronize();
        }
      });
    }
  }

  final RenderRepaintBoundary Function() root;
  final String name;
  static final _sessions = <OperitDesignRuntime>{};
  static bool _frameCallbackInstalled = false;
  final _ids = Expando<String>();
  final _targets = <String, RenderObject>{};
  final _edits = <String, Map<String, Object?>>{};
  final _webTargets =
      <String, ({Element owner, String domTarget, String selector})>{};
  final _webEdits = <String, Map<String, Object?>>{};
  final _applied = <RenderObject, _RuntimePropertyState>{};
  int _nextId = 0;
  int _revision = 0;
  int _pointer = 20000;
  bool _disposed = false;
  Future<void> _queue = Future<void>.value();

  void dispose() {
    _restoreAll();
    _sessions.remove(this);
    _targets.clear();
    _webTargets.clear();
    _webEdits.clear();
    _edits.clear();
    _disposed = true;
  }

  void _checkOpen() {
    if (_disposed) throw StateError('Design runtime session is closed');
  }

  String _id(RenderObject object) => _ids[object] ??= 'live-${_nextId++}';

  /// Explicit owner IDs survive remounts. Without one, edits deliberately stay
  /// scoped to this render object's lifetime rather than guessing by text/index.
  String _selector(RenderObject object) {
    final creator = object.debugCreator;
    if (creator is! DebugCreator) return _id(object);
    final path = <String>[];
    Element current = creator.element;
    Map<String, Object?>? owner;
    while (true) {
      if (current.widget case OperitDesignSource(:final source)) {
        if (source['designId'] != null || source['composeId'] != null) {
          owner = source;
          break;
        }
      }
      Element? parent;
      current.visitAncestorElements((e) {
        parent = e;
        return false;
      });
      if (parent == null) break;
      final key = current.widget.key;
      int sibling = 0;
      if (key == null) {
        var found = false;
        parent!.visitChildren((e) {
          if (identical(e, current)) found = true;
          if (!found && e.widget.runtimeType == current.widget.runtimeType) {
            sibling++;
          }
        });
      }
      path.add('${current.widget.runtimeType}:${key ?? sibling}');
      current = parent!;
    }
    if (owner == null) return _id(object);
    return jsonEncode([
      owner['package'],
      owner['route'],
      owner['designId'] ?? owner['composeId'],
      ...path.reversed,
    ]);
  }

  List<String> _properties(RenderObject object) => switch (object) {
    RenderParagraph() => [
      if (object.text is TextSpan && (object.text as TextSpan).children == null)
        'text',
      'fontSize',
      'fill',
    ],
    RenderDecoratedBox(decoration: BoxDecoration()) => [
      if ((object.decoration as BoxDecoration).gradient == null &&
          (object.decoration as BoxDecoration).image == null)
        'fill',
      if ((object.decoration as BoxDecoration).shape == BoxShape.rectangle)
        'radius',
    ],
    RenderPhysicalModel() => [
      'fill',
      if (object.shape == BoxShape.rectangle) 'radius',
    ],
    RenderPadding() => [
      'paddingLeft',
      'paddingRight',
      'paddingTop',
      'paddingBottom',
    ],
    RenderConstrainedBox() => ['width', 'height'],
    RenderOpacity() => ['opacity'],
    _ => [],
  };

  Map<String, Object?> _describe(RenderObject object) {
    final id = _id(object);
    _targets[id] = object;
    final selector = _selector(object);
    return {
      'runtimeTarget': id,
      'runtimeSelector': selector,
      'runtimeProperties': _properties(object),
      'runtimePersistence': selector == id ? 'element' : 'source',
      if (object is RenderEditable) 'runtimeInput': !object.readOnly,
      if (object is RenderEditable) 'runtimeObscure': object.obscureText,
    };
  }

  Map<String, Object?> _describeSurface(
    Element owner,
    Map<String, Object?> source,
  ) {
    final domTarget = source['domRuntimeTarget'];
    final surface = owner.findRenderObject();
    if (domTarget is! String || surface == null) return {};
    final target = 'web:${_id(surface)}:$domTarget';
    final explicit = source['designId'];
    final selector = jsonEncode([
      'web',
      source['package'],
      source['route'],
      source['composeId'],
      source['url'],
      explicit ?? domTarget,
      source['domRuntimeKind'],
      source['domRuntimeTextIndex'],
    ]);
    _webTargets[target] = (
      owner: owner,
      domTarget: domTarget,
      selector: selector,
    );
    return {
      'runtimeTarget': target,
      'runtimeSelector': selector,
      'runtimeProperties': source['domRuntimeProperties'] ?? [],
      'runtimePersistence': source['domRuntimePersistence'] ?? 'element',
      'runtimeInput': source['domRuntimeInput'] == true,
      'runtimeObscure': source['domRuntimeObscure'] == true,
    };
  }

  Future<void> _surfaceRequest(
    String target,
    Map<String, Object?> request,
  ) async {
    final web = _webTargets[target];
    if (web == null || !web.owner.mounted) {
      throw StateError('Stale WebView target');
    }
    final action = OperitDesignSurfaces._actions[web.owner];
    if (action == null) throw StateError('WebView runtime is unavailable');
    final result = await action({...request, 'target': web.domTarget});
    if (result is! Map || result['ok'] != true) {
      throw StateError('WebView did not acknowledge the runtime request');
    }
    if (request['operation'] == 'edit') {
      _webEdits[web.selector] = {
        ...?_webEdits[web.selector],
        ...Map<String, Object?>.from(request['properties'] as Map),
      };
    } else if (request['operation'] == 'clear') {
      _webEdits.remove(web.selector);
    }
  }

  Future<void> dispatchEvent(Map<String, Object?> event) async {
    _checkOpen();
    final target = event['target'];
    if (target is String && _webTargets.containsKey(target)) {
      await _surfaceRequest(target, {'operation': 'event', 'event': event});
    } else {
      dispatch(event);
    }
  }

  Future<Map<String, Object?>> snapshot() async {
    _checkOpen();
    _targets.clear();
    _webTargets.clear();
    final document = await captureOperitDesign(
      root(),
      name: name,
      includeReference: false,
      runtime: this,
    );
    return {
      'protocol': 'operit-design-runtime',
      'version': 1,
      'revision': ++_revision,
      'document': document,
      'edits': _exportEdits(),
    };
  }

  List<Map<String, Object?>> _exportEdits() => [
    for (final entry in _edits.entries)
      {
        'selector': entry.key,
        'properties': {...entry.value},
      },
    for (final entry in _webEdits.entries)
      {
        'selector': entry.key,
        'properties': {...entry.value},
        'kind': 'webview',
      },
  ];

  /// Serialized VM requests prevent input/edit/snapshot races. Every input is
  /// followed by real layout and paint, rather than selecting a canned scene.
  Future<Map<String, Object?>> request(Map<String, dynamic> request) {
    final result = Completer<Map<String, Object?>>();
    _queue = _queue.then((_) async {
      try {
        _checkOpen();
        switch (request['operation']) {
          case 'snapshot':
            break;
          case 'event':
            await dispatchEvent(
              Map<String, Object?>.from(request['event'] as Map),
            );
          case 'edit':
            final target = request['target'] as String;
            final properties = Map<String, Object?>.from(
              request['properties'] as Map,
            );
            if (_webTargets.containsKey(target)) {
              await _surfaceRequest(target, {
                'operation': 'edit',
                'properties': properties,
              });
            } else {
              edit(target, properties);
            }
          case 'clear':
            final target = request['target'] as String?;
            if (target != null && _webTargets.containsKey(target)) {
              await _surfaceRequest(target, {'operation': 'clear'});
            } else {
              clear(target);
              if (target == null) {
                final owners = _webTargets.values
                    .map((web) => web.owner)
                    .toSet();
                for (final owner in owners) {
                  await OperitDesignSurfaces._actions[owner]?.call({
                    'operation': 'clear',
                  });
                }
                _webEdits.clear();
              }
            }
          default:
            throw ArgumentError('Unknown runtime operation');
        }
        WidgetsBinding.instance.scheduleFrame();
        await WidgetsBinding.instance.endOfFrame;
        // A visual override after the frame may have requested a second layout.
        if (root().debugNeedsPaint || root().debugNeedsLayout) {
          WidgetsBinding.instance.scheduleFrame();
          await WidgetsBinding.instance.endOfFrame;
        }
        result.complete(await snapshot());
      } catch (error, stack) {
        result.completeError(error, stack);
      }
    });
    return result.future;
  }

  RenderObject _target(String id) {
    _checkOpen();
    final object = _targets[id];
    if (object == null || !object.attached) {
      throw StateError('Stale runtime target: $id');
    }
    RenderObject? ancestor = object;
    while (ancestor != null && !identical(ancestor, root())) {
      if (ancestor is RenderOffstage && ancestor.offstage) {
        throw StateError('Runtime target is hidden');
      }
      ancestor = ancestor.parent;
    }
    if (ancestor == null) {
      throw StateError('Runtime target belongs to another scope');
    }
    return object;
  }

  /// Dispatches real pointer, scroll and text-editing events. Text input uses
  /// EditableTextState's user-edit path so controllers, formatters and onChanged
  /// participate. No source callback is guessed from a captured label.
  void dispatch(Map<String, Object?> event) {
    _checkOpen();
    final object = _target(event['target'] as String);
    final local = Offset(
      _finite(event['x'] ?? object.paintBounds.width / 2, 'x'),
      _finite(event['y'] ?? object.paintBounds.height / 2, 'y'),
    );
    if (!object.paintBounds.contains(local)) {
      throw ArgumentError('Pointer is outside the target');
    }
    final position = MatrixUtils.transformPoint(
      object.getTransformTo(null),
      local,
    );
    final viewId = object.owner?.rootNode is RenderView
        ? (object.owner!.rootNode as RenderView).flutterView.viewId
        : View.of((object.debugCreator as DebugCreator).element).viewId;
    switch (event['type']) {
      case 'tap':
        final pointer = ++_pointer;
        GestureBinding.instance.handlePointerEvent(
          PointerDownEvent(
            pointer: pointer,
            position: position,
            viewId: viewId,
          ),
        );
        GestureBinding.instance.handlePointerEvent(
          PointerUpEvent(pointer: pointer, position: position, viewId: viewId),
        );
      case 'scroll':
        GestureBinding.instance.handlePointerEvent(
          PointerScrollEvent(
            position: position,
            viewId: viewId,
            scrollDelta: Offset(
              _finite(event['dx'] ?? 0, 'dx'),
              _finite(event['dy'] ?? 0, 'dy'),
            ),
          ),
        );
      case 'input':
        final creator = object.debugCreator;
        if (object is! RenderEditable ||
            object.readOnly ||
            creator is! DebugCreator) {
          throw ArgumentError('Target is not a writable Flutter input');
        }
        final state = creator.element
            .findAncestorStateOfType<EditableTextState>();
        if (state == null) throw StateError('EditableTextState is unavailable');
        final text = event['text'];
        if (text is! String || text.length > 65536) {
          throw ArgumentError('Invalid input text');
        }
        state.widget.focusNode.requestFocus();
        state.userUpdateTextEditingValue(
          TextEditingValue(
            text: text,
            selection: TextSelection.collapsed(offset: text.length),
          ),
          SelectionChangedCause.keyboard,
        );
      default:
        throw ArgumentError('Unsupported runtime event: ${event['type']}');
    }
  }

  void edit(String target, Map<String, Object?> properties) {
    final object = _target(target);
    final allowed = _properties(object);
    if (properties.isEmpty ||
        properties.keys.any((key) => !allowed.contains(key))) {
      throw ArgumentError('Supported properties: ${allowed.join(', ')}');
    }
    for (final entry in properties.entries) {
      if (entry.key == 'text') {
        if (entry.value is! String || (entry.value as String).length > 65536) {
          throw ArgumentError('Invalid design text');
        }
      } else if (entry.key == 'fill') {
        _runtimeColor(entry.value);
      } else {
        final value = _finite(entry.value, entry.key);
        if (value < 0 || value > (entry.key == 'opacity' ? 1 : 10000)) {
          throw ArgumentError('Invalid ${entry.key}');
        }
      }
    }
    final selector = _selector(object);
    _edits[selector] = {...?_edits[selector], ...properties};
    _synchronize();
    WidgetsBinding.instance.scheduleFrame();
  }

  void clear([String? target]) {
    _checkOpen();
    if (target == null) {
      _edits.clear();
    } else {
      _edits.remove(_selector(_target(target)));
    }
    _synchronize();
    WidgetsBinding.instance.scheduleFrame();
  }

  void _restoreAll() {
    for (final entry in _applied.entries) {
      if (entry.key.attached) entry.value.restore();
    }
    _applied.clear();
  }

  void _synchronize() {
    if (_disposed) return;
    if (_edits.isEmpty) {
      _restoreAll();
      return;
    }
    RenderRepaintBoundary boundary;
    try {
      boundary = root();
    } catch (_) {
      return;
    }
    if (!boundary.attached) return;
    final visited = <RenderObject>{};
    var changed = false;
    void visit(RenderObject object) {
      visited.add(object);
      final patch = _edits[_selector(object)];
      if (patch != null && _properties(object).isNotEmpty) {
        final state = _applied.putIfAbsent(
          object,
          () => _RuntimePropertyState(object),
        );
        changed = state.apply(patch) || changed;
      } else {
        _applied.remove(object)?.restore();
      }
      object.visitChildren(visit);
    }

    visit(boundary);
    _applied.removeWhere((object, _) => !visited.contains(object));
    // Persistent callbacks run after WidgetsBinding has painted. Layout changes
    // here require an explicit subsequent frame, including newly mounted nodes.
    if (changed) WidgetsBinding.instance.scheduleFrame();
  }
}

double _finite(Object? value, String name) {
  if (value is! num || !value.isFinite) throw ArgumentError('Invalid $name');
  return value.toDouble();
}

Color _runtimeColor(Object? value) {
  if (value is! String || !RegExp(r'^#[a-fA-F0-9]{8}$').hasMatch(value)) {
    throw ArgumentError('fill must be #RRGGBBAA');
  }
  final rgba = int.parse(value.substring(1), radix: 16);
  return Color(((rgba & 255) << 24) | (rgba >> 8));
}

class _RuntimePropertyState {
  _RuntimePropertyState(this.object) : base = _read(object);
  final RenderObject object;
  Object? base;
  Object? applied;

  static Object? _read(RenderObject o) => switch (o) {
    RenderParagraph() => o.text,
    RenderDecoratedBox() => o.decoration,
    RenderPhysicalModel() => (o.color, o.borderRadius),
    RenderPadding() => o.padding,
    RenderConstrainedBox() => o.additionalConstraints,
    RenderOpacity() => o.opacity,
    _ => null,
  };

  void _write(Object? value) {
    switch (object) {
      case RenderParagraph o:
        o.text = value! as InlineSpan;
      case RenderDecoratedBox o:
        o.decoration = value! as Decoration;
      case RenderPhysicalModel o:
        final v = value! as (Color, BorderRadius?);
        o.color = v.$1;
        o.borderRadius = v.$2;
      case RenderPadding o:
        o.padding = value! as EdgeInsetsGeometry;
      case RenderConstrainedBox o:
        o.additionalConstraints = value! as BoxConstraints;
      case RenderOpacity o:
        o.opacity = value! as double;
    }
  }

  bool apply(Map<String, Object?> patch) {
    final current = _read(object);
    if (applied != null && current != applied) base = current;
    final next = _patched(base, patch);
    final changed = current != next;
    if (changed) _write(next);
    applied = next;
    return changed;
  }

  void restore() {
    if (applied != null && _read(object) == applied) _write(base);
  }

  Object? _patched(Object? original, Map<String, Object?> p) {
    final fill = p.containsKey('fill') ? _runtimeColor(p['fill']) : null;
    final radius = (p['radius'] as num?)?.toDouble();
    switch (object) {
      case RenderParagraph():
        return _span(original! as InlineSpan, p);
      case RenderDecoratedBox():
        final decoration = original! as BoxDecoration;
        return decoration.copyWith(
          color: fill,
          borderRadius: radius == null ? null : BorderRadius.circular(radius),
        );
      case RenderPhysicalModel():
        final value = original! as (Color, BorderRadius?);
        return (
          fill ?? value.$1,
          radius == null ? value.$2 : BorderRadius.circular(radius),
        );
      case RenderPadding o:
        final value = (original! as EdgeInsetsGeometry).resolve(
          o.textDirection ?? TextDirection.ltr,
        );
        return value.copyWith(
          left: (p['paddingLeft'] as num?)?.toDouble(),
          right: (p['paddingRight'] as num?)?.toDouble(),
          top: (p['paddingTop'] as num?)?.toDouble(),
          bottom: (p['paddingBottom'] as num?)?.toDouble(),
        );
      case RenderConstrainedBox():
        final value = original! as BoxConstraints;
        final width = (p['width'] as num?)?.toDouble(),
            height = (p['height'] as num?)?.toDouble();
        return value.copyWith(
          minWidth: width,
          maxWidth: width,
          minHeight: height,
          maxHeight: height,
        );
      case RenderOpacity():
        return (p['opacity'] as num?)?.toDouble() ?? original;
      default:
        return original;
    }
  }

  InlineSpan _span(InlineSpan original, Map<String, Object?> p) {
    if (original is! TextSpan) return original;
    final fill = p.containsKey('fill') ? _runtimeColor(p['fill']) : null;
    return TextSpan(
      text: p['text'] as String? ?? original.text,
      children: original.children
          ?.map((child) => _span(child, {...p}..remove('text')))
          .toList(),
      style: (original.style ?? const TextStyle()).copyWith(
        fontSize: (p['fontSize'] as num?)?.toDouble(),
        color: fill,
      ),
      recognizer: original.recognizer,
      mouseCursor: original.mouseCursor,
      onEnter: original.onEnter,
      onExit: original.onExit,
      semanticsLabel: p.containsKey('text') ? null : original.semanticsLabel,
      semanticsIdentifier: original.semanticsIdentifier,
      locale: original.locale,
      spellOut: original.spellOut,
    );
  }
}
