// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

enum _ComposeDslModifierScope { normal, row, column, box }

class _RowFlexSpec {
  /// Creates the row flex spec instance.
  const _RowFlexSpec({required this.weight, required this.fill});

  final double weight;
  final bool fill;
}

/// Resolves row flex spec for the Compose DSL renderer.
_RowFlexSpec? _rowFlexSpec(Map<String, Object?> props) {
  final explicitWeight = _number(props['weight']);
  if (explicitWeight != null && explicitWeight > 0) {
    return _RowFlexSpec(
      weight: explicitWeight,
      fill: _boolOrDefault(props['weightFill'], true),
    );
  }

  final weightOp = _modifierOpByToken(props['modifier'], 'weight');
  if (weightOp != null) {
    final args = weightOp['args'] is List<Object?>
        ? weightOp['args'] as List<Object?>
        : const <Object?>[];
    final weight = _number(args.firstOrNull);
    if (weight != null && weight > 0) {
      return _RowFlexSpec(
        weight: weight,
        fill: _boolOrDefault(args.elementAtOrNull(1), true),
      );
    }
  }

  if (_bool(props['fillMaxWidth']) ||
      _bool(props['fillMaxSize']) ||
      _hasModifierOp(props['modifier'], 'fillmaxwidth') ||
      _hasModifierOp(props['modifier'], 'fillmaxsize')) {
    return const _RowFlexSpec(weight: 1, fill: true);
  }

  return null;
}

/// Evaluates bool or default for the Compose DSL renderer.
bool _boolOrDefault(Object? raw, bool defaultValue) =>
    raw == null ? defaultValue : _bool(raw);

Map<String, Object?>? _modifierOpByToken(Object? rawModifier, String token) {
  final normalizedToken = _normalizeToken(token);
  for (final op in _modifierOps(rawModifier)) {
    if (_normalizeToken((op['name'] ?? '').toString()) == normalizedToken) {
      return op;
    }
  }
  return null;
}

/// Evaluates has modifier op for the Compose DSL renderer.
bool _hasModifierOp(Object? rawModifier, String token) =>
    _modifierOpByToken(rawModifier, token) != null;

/// Resolves flex for weight for the Compose DSL renderer.
int _flexForWeight(double weight) {
  final scaled = (weight * 1000).round();
  return scaled < 1 ? 1 : scaled;
}

/// Builds with modifier for the Compose DSL renderer.
Widget _withModifier(
  BuildContext context,
  Widget child,
  Map<String, Object?> props,

  /// Resolves function for the Compose DSL renderer.
  Future<Object?> Function(String actionId, [Object? payload]) onAction, {
  required String nodeType,
  required _ComposeDslModifierScope modifierScope,
}) {
  final ops = _modifierOps(props['modifier']);
  final dragOp = _modifierOpByToken(props['modifier'], 'draggestures');
  final transformOp = _modifierOpByToken(
    props['modifier'],
    'transformgestures',
  );
  final combinedMotion = dragOp != null && transformOp != null;
  Widget current = child;
  current = _withDirectModifierProps(
    context,
    current,
    props,
    onAction,
    nodeType: nodeType,
    modifierScope: modifierScope,
  );
  for (final op in ops.reversed) {
    final name = _normalizeToken((op['name'] ?? '').toString());
    final args = op['args'] is List<Object?>
        ? op['args'] as List<Object?>
        : const <Object?>[];
    switch (name) {
      case 'padding':
        current = Padding(padding: _edgeInsets(args), child: current);
        break;
      case 'imepadding':
        final insets = MediaQuery.viewInsetsOf(context);
        current = Padding(
          padding: insets,
          child: MediaQuery.removeViewInsets(
            context: context,
            removeLeft: true,
            removeTop: true,
            removeRight: true,
            removeBottom: true,
            child: current,
          ),
        );
        break;
      case 'statusbarspadding':
      case 'navigationbarspadding':
      case 'systembarspadding':
      case 'safedrawingpadding':
        current = SafeArea(
          top: name != 'navigationbarspadding',
          bottom: name != 'statusbarspadding',
          left: name != 'statusbarspadding',
          right: name != 'statusbarspadding',
          child: current,
        );
        break;
      case 'defaultminsize':
        final values = _stringMap(args.firstOrNull);
        current = ConstrainedBox(
          constraints: BoxConstraints(
            minWidth:
                _number(
                  values.isEmpty ? args.firstOrNull : values['minWidth'],
                ) ??
                0,
            minHeight:
                _number(
                  values.isEmpty
                      ? args.elementAtOrNull(1)
                      : values['minHeight'],
                ) ??
                0,
          ),
          child: current,
        );
        break;
      case 'wrapcontentwidth':
      case 'wrapcontentheight':
      case 'wrapcontentsize':
        final values = _stringMap(args.firstOrNull);
        final align = values.isEmpty ? args.firstOrNull : values['align'];
        final alignment = align == null
            ? Alignment.center
            : switch (name) {
                'wrapcontentwidth' => switch (_normalizeToken(_string(align))) {
                  'center' || 'centerhorizontally' => Alignment.center,
                  'start' || 'left' => Alignment.centerLeft,
                  'end' || 'right' => Alignment.centerRight,
                  _ => throw FormatException(
                    'Invalid horizontal alignment: $align',
                  ),
                },
                'wrapcontentheight' => switch (_normalizeToken(
                  _string(align),
                )) {
                  'center' || 'centervertically' => Alignment.center,
                  'top' || 'start' => Alignment.topCenter,
                  'bottom' || 'end' => Alignment.bottomCenter,
                  _ => throw FormatException(
                    'Invalid vertical alignment: $align',
                  ),
                },
                _ => _alignment(align),
              };
        final unbounded = _bool(
          values.isEmpty ? args.elementAtOrNull(1) : values['unbounded'],
        );
        if (unbounded) {
          current = UnconstrainedBox(
            alignment: alignment,
            constrainedAxis: switch (name) {
              'wrapcontentwidth' => Axis.vertical,
              'wrapcontentheight' => Axis.horizontal,
              _ => null,
            },
            child: current,
          );
        }
        current = Align(
          alignment: alignment,
          widthFactor: name != 'wrapcontentheight' ? 1 : null,
          heightFactor: name != 'wrapcontentwidth' ? 1 : null,
          child: current,
        );
        break;
      case 'onsizechanged':
        final actionId = _requiredActionId(args.firstOrNull, 'onSizeChanged');
        current = _SizeReportingBox(
          onSizeChanged: (size) => onAction(actionId, <String, Object?>{
            'width': size.width,
            'height': size.height,
          }),
          child: current,
        );
        break;
      case 'ongloballypositioned':
        final actionId = _requiredActionId(
          args.firstOrNull,
          'onGloballyPositioned',
        );
        current = _PositionReportingBox(
          onPosition: (bounds) {
            final root = context
                .findAncestorStateOfType<_ComposeHostState>()
                ?.context
                .findRenderObject();
            if (root is! RenderBox) {
              throw StateError('Compose root is not laid out');
            }
            final rootOrigin = root.localToGlobal(Offset.zero);
            onAction(actionId, <String, Object?>{
              'rootX': bounds.left - rootOrigin.dx,
              'rootY': bounds.top - rootOrigin.dy,
              'windowX': bounds.left,
              'windowY': bounds.top,
              'width': bounds.width,
              'height': bounds.height,
            });
          },
          child: current,
        );
        break;
      case 'combinedclickable':
      case 'tapgestures':
      case 'draggestures':
      case 'transformgestures':
        if (combinedMotion && name == 'transformgestures') break;
        if (combinedMotion && name == 'draggestures') {
          final transformArgs = transformOp['args'] as List<Object?>;
          current = _ComposeGestureRegion(
            kind: 'motiongestures',
            options: <String, Object?>{
              ..._stringMap(args.firstOrNull),
              ..._stringMap(transformArgs.firstOrNull),
            },
            onAction: onAction,
            child: current,
          );
          break;
        }
        current = _ComposeGestureRegion(
          kind: name,
          options: _stringMap(args.firstOrNull),
          onAction: onAction,
          child: current,
        );
        break;
      case 'fillMaxWidth':
      case 'fillmaxwidth':
      case 'fillMaxHeight':
      case 'fillmaxheight':
      case 'fillMaxSize':
      case 'fillmaxsize':
        current = ComposeDslFill(
          fillWidth:
              name != 'fillmaxheight' &&
              modifierScope != _ComposeDslModifierScope.row,
          fillHeight: name != 'fillmaxwidth',
          fraction: math.max(0, _number(args.firstOrNull) ?? 1),
          child: current,
        );
        break;
      case 'width':
      case 'requiredWidth':
      case 'requiredwidth':
        current = SizedBox(width: _number(args.firstOrNull), child: current);
        break;
      case 'height':
      case 'requiredHeight':
      case 'requiredheight':
        current = SizedBox(height: _number(args.firstOrNull), child: current);
        break;
      case 'size':
      case 'requiredSize':
      case 'requiredsize':
        current = SizedBox(
          width: _number(args.firstOrNull),
          height: _number(args.length > 1 ? args[1] : args.firstOrNull),
          child: current,
        );
        break;
      case 'widthin':
      case 'requiredwidthin':
        current = ConstrainedBox(
          constraints: _axisConstraints(args, horizontal: true),
          child: current,
        );
        break;
      case 'heightin':
      case 'requiredheightin':
        current = ConstrainedBox(
          constraints: _axisConstraints(args, horizontal: false),
          child: current,
        );
        break;
      case 'sizein':
      case 'requiredsizein':
        current = ConstrainedBox(
          constraints: _sizeConstraints(args),
          child: current,
        );
        break;
      case 'aspectratio':
        final ratio = _number(args.firstOrNull);
        if (ratio != null && ratio > 0) {
          current = AspectRatio(aspectRatio: ratio, child: current);
        }
        break;
      case 'alpha':
        current = Opacity(
          opacity: (_number(args.firstOrNull) ?? 1).clamp(0, 1),
          child: current,
        );
        break;
      case 'rotate':
        current = Transform.rotate(
          angle: ((_number(args.firstOrNull) ?? 0) * math.pi) / 180,
          child: current,
        );
        break;
      case 'scale':
        current = Transform.scale(
          scale: _number(args.firstOrNull) ?? 1,
          child: current,
        );
        break;
      case 'offset':
        final offset = _offset(args);
        current = Transform.translate(offset: offset, child: current);
        break;
      case 'background':
        final brush = _stringMap(args.firstOrNull);
        current = DecoratedBox(
          decoration: BoxDecoration(
            color: brush['colors'] == null
                ? _color(context, args.firstOrNull)
                : null,
            gradient: brush['colors'] == null
                ? null
                : _backgroundGradient(context, brush),
            borderRadius: _borderRadius(args.length > 1 ? args[1] : null),
          ),
          child: current,
        );
        break;
      case 'border':
        current = DecoratedBox(
          decoration: BoxDecoration(
            border: Border.all(
              width: _number(args.firstOrNull) ?? 1,
              color:
                  _color(context, args.length > 1 ? args[1] : null) ??
                  Theme.of(context).colorScheme.outline,
            ),
            borderRadius: _borderRadius(args.length > 2 ? args[2] : null),
          ),
          child: current,
        );
        break;
      case 'clip':
        current = ClipRRect(
          borderRadius: _borderRadius(args.firstOrNull) ?? BorderRadius.zero,
          child: current,
        );
        break;
      case 'cliptobounds':
        current = ClipRect(child: current);
        break;
      case 'shadow':
        current = DecoratedBox(
          decoration: BoxDecoration(
            boxShadow: <BoxShadow>[
              BoxShadow(
                blurRadius: (_number(args.firstOrNull) ?? 0) * 2,
                spreadRadius: 0,
                color: Colors.black.withValues(alpha: 0.22),
              ),
            ],
            borderRadius: _borderRadius(args.length > 1 ? args[1] : null),
          ),
          child: current,
        );
        break;
      case 'clickable':
        final actionId = _actionId(args.firstOrNull);
        if (actionId != null) {
          current = InkWell(onTap: () => onAction(actionId), child: current);
        }
        break;
    }
  }
  if (modifierScope == _ComposeDslModifierScope.row) {
    final rowFlexSpec = _rowFlexSpec(props);
    if (rowFlexSpec != null) {
      current = Flexible(
        flex: _flexForWeight(rowFlexSpec.weight),
        fit: rowFlexSpec.fill ? FlexFit.tight : FlexFit.loose,
        child: current,
      );
    }
  }
  if (modifierScope == _ComposeDslModifierScope.column) {
    final weightOp = _modifierOpByToken(props['modifier'], 'weight');
    final weightArgs = weightOp?['args'] as List<Object?>?;
    final weight = _number(props['weight'] ?? weightArgs?.firstOrNull);
    if (weight != null && weight > 0) {
      current = Flexible(
        flex: _flexForWeight(weight),
        fit:
            _boolOrDefault(
              props['weightFill'] ?? weightArgs?.elementAtOrNull(1),
              true,
            )
            ? FlexFit.tight
            : FlexFit.loose,
        child: current,
      );
    }
  }
  return current;
}

/// Builds with direct modifier props for the Compose DSL renderer.
Widget _withDirectModifierProps(
  BuildContext context,
  Widget child,
  Map<String, Object?> props,
  Future<Object?> Function(String, [Object?]) onAction, {
  required String nodeType,
  required _ComposeDslModifierScope modifierScope,
}) {
  Widget current = child;
  // Box owns its visual transition so intermediate sizes are not clamped by
  // an outer SizedBox. All geometry, colors and timing come from the plugin.
  final boxVisuals =
      nodeType == 'Box' &&
      (props['animation'] is Map || props['hoverStyle'] is Map);
  final visualProps = boxVisuals && _ComposeHoverScope.of(context)
      ? <String, Object?>{...props, ..._stringMap(props['hoverStyle'])}
      : props;
  final width = _number(visualProps['width']);
  final height = _number(visualProps['height']);
  if (boxVisuals) {
    final animation = _stringMap(props['animation']);
    final durationMillis = _int(animation['durationMillis']) ?? 0;
    final decoration = BoxDecoration(
      color: _color(
        context,
        visualProps['backgroundColor'] ?? visualProps['background'],
      ),
      gradient: visualProps['backgroundBrush'] == null
          ? null
          : _backgroundGradient(
              context,
              _stringMap(visualProps['backgroundBrush']),
            ),
      borderRadius: _borderRadius(
        visualProps['backgroundShape'] ?? visualProps['shape'],
      ),
    );
    current = AnimatedContainer(
      duration: Duration(milliseconds: durationMillis < 0 ? 0 : durationMillis),
      curve: switch (_normalizeToken(_string(animation['curve']))) {
        'easeoutcubic' => Curves.easeOutCubic,
        'easeinout' => Curves.easeInOut,
        'easeout' => Curves.easeOut,
        _ => Curves.linear,
      },
      width: width,
      height: height,
      decoration: decoration,
      child: current,
    );
  } else if (width != null || height != null) {
    current = SizedBox(width: width, height: height, child: current);
  }
  if (_bool(props['fillMaxSize'])) {
    current = ComposeDslFill(
      fillWidth: modifierScope != _ComposeDslModifierScope.row,
      fillHeight: true,
      child: current,
    );
  } else if (_bool(props['fillMaxHeight'])) {
    current = ComposeDslFill(
      fillWidth: false,
      fillHeight: true,
      child: current,
    );
  } else if (_bool(props['fillMaxWidth'])) {
    current = ComposeDslFill(
      fillWidth: modifierScope != _ComposeDslModifierScope.row,
      fillHeight: false,
      child: current,
    );
  }
  final padding = _commonPaddingFromProps(props);
  if (padding != null) {
    current = Padding(padding: padding, child: current);
  }
  // Put row ink above its decoration and inside its full padded hit region.
  // Wrapping a decorated row afterwards hides the splash behind the background.
  if (nodeType == 'Row' && _actionId(props['onClick']) != null) {
    final clipArgs =
        _modifierOpByToken(props['modifier'], 'clip')?['args']
            as List<Object?>?;
    current = _ComposeGestureRegion(
      kind: 'combinedclickable',
      options: <String, Object?>{'onClick': props['onClick']},
      borderRadius:
          _borderRadius(props['backgroundShape'] ?? props['shape']) ??
          _borderRadius(clipArgs?.firstOrNull) ??
          BorderRadius.zero,
      onAction: onAction,
      child: current,
    );
  }
  final background = props['backgroundColor'] ?? props['background'];
  final brush = props['backgroundBrush'];
  if (!boxVisuals && (background != null || brush != null)) {
    current = DecoratedBox(
      decoration: BoxDecoration(
        color: brush == null ? _color(context, background) : null,
        gradient: brush == null
            ? null
            : _backgroundGradient(context, _stringMap(brush)),
        borderRadius: _borderRadius(props['backgroundShape'] ?? props['shape']),
      ),
      child: current,
    );
  }
  final alpha = _number(props['alpha']);
  if (alpha != null && !_nodeOwnsDirectAlpha(nodeType)) {
    current = Opacity(opacity: alpha.clamp(0, 1), child: current);
  }
  return current;
}

/// Resolves the vertical gradient brush supported by the shared DSL contract.
Gradient _backgroundGradient(BuildContext context, Map<String, Object?> brush) {
  if (_normalizeToken(_string(brush['type'])) != 'verticalgradient') {
    throw FormatException('Unsupported Compose brush: ${brush['type']}');
  }
  final colors = (brush['colors'] as List<Object?>)
      .map((value) {
        final color = _color(context, value);
        if (color == null) {
          throw FormatException('Invalid Compose brush color: $value');
        }
        return color;
      })
      .toList(growable: false);
  if (colors.length < 2) {
    throw const FormatException('A gradient requires at least two colors');
  }
  return LinearGradient(
    begin: Alignment.topCenter,
    end: Alignment.bottomCenter,
    colors: colors,
  );
}

/// Evaluates node owns direct alpha for the Compose DSL renderer.
bool _nodeOwnsDirectAlpha(String nodeType) {
  return switch (nodeType) {
    'Card' ||
    'ElevatedCard' ||
    'OutlinedCard' ||
    'Surface' ||
    'Image' ||
    'AsyncImage' => true,
    _ => false,
  };
}
