// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Resolves web view key for the Compose DSL renderer.
Key _webViewKey({
  required Map<String, Object?> props,
  required String nodePath,
  required ComposeDslWebViewHostContext webViewHostContext,
}) {
  final explicitKey = _string(props['key']).trim();
  final controller = props['controller'];
  final controllerKey = controller is Map
      ? _string(controller['key']).trim()
      : '';
  final identity = explicitKey.isNotEmpty
      ? explicitKey
      : controllerKey.isNotEmpty
      ? controllerKey
      : nodePath;
  return ValueKey<String>(
    'compose_webview:${webViewHostContext.executionContextKey}:$identity',
  );
}

/// Resolves edge insets for the Compose DSL renderer.
EdgeInsets _edgeInsets(List<Object?> args) {
  if (args.isEmpty) {
    return EdgeInsets.zero;
  }
  final first = args.first;
  if (first is Map) {
    final all = _number(first['all']);
    return EdgeInsets.only(
      left:
          _number(first['start']) ??
          _number(first['left']) ??
          _number(first['horizontal']) ??
          all ??
          0,
      top: _number(first['top']) ?? _number(first['vertical']) ?? all ?? 0,
      right:
          _number(first['end']) ??
          _number(first['right']) ??
          _number(first['horizontal']) ??
          all ??
          0,
      bottom:
          _number(first['bottom']) ?? _number(first['vertical']) ?? all ?? 0,
    );
  }
  if (args.length >= 4) {
    return EdgeInsets.fromLTRB(
      _number(args[0]) ?? 0,
      _number(args[1]) ?? 0,
      _number(args[2]) ?? 0,
      _number(args[3]) ?? 0,
    );
  }
  if (args.length >= 2) {
    return EdgeInsets.symmetric(
      horizontal: _number(args[0]) ?? 0,
      vertical: _number(args[1]) ?? 0,
    );
  }
  return EdgeInsets.all(_number(first) ?? 0);
}

/// Resolves edge insets from value for the Compose DSL renderer.
EdgeInsets _edgeInsetsFromValue(Object? raw) {
  if (raw is List<Object?>) {
    return _edgeInsets(raw);
  }
  return _edgeInsets(<Object?>[raw]);
}

/// Resolves Compose padding props, matching the Kotlin common padding spec.
EdgeInsets? _commonPaddingFromProps(Map<String, Object?> props) {
  if (props['padding'] != null) {
    return _edgeInsetsFromValue(props['padding']);
  }
  final start = _number(props['paddingStart']);
  final top = _number(props['paddingTop']);
  final end = _number(props['paddingEnd']);
  final bottom = _number(props['paddingBottom']);
  if (start != null || top != null || end != null || bottom != null) {
    return EdgeInsets.only(
      left: start ?? 0,
      top: top ?? 0,
      right: end ?? 0,
      bottom: bottom ?? 0,
    );
  }
  final horizontal = _number(props['paddingHorizontal']);
  final vertical = _number(props['paddingVertical']);
  if (horizontal != null || vertical != null) {
    return EdgeInsets.symmetric(
      horizontal: horizontal ?? 0,
      vertical: vertical ?? 0,
    );
  }
  return null;
}

/// Preserves fixed gaps in aligned groups; distributed arrangements own their gaps.
double _flexSpacing(Object? arrangement, Object? spacing) {
  final token = _normalizeToken(_string(arrangement));
  if (token == 'spacebetween' ||
      token == 'spacearound' ||
      token == 'spaceevenly') {
    return 0;
  }
  return _number(spacing) ?? 0;
}

/// Resolves a flow-line gap while preserving distributed arrangement behavior.
double _flowSpacing(Object? arrangement, Object? spacing) =>
    _flexSpacing(arrangement, spacing);

/// Resolves a Compose arrangement for Flutter's flow layout.
WrapAlignment _wrapAlignment(Object? raw) {
  final token = _string(raw);
  if (token.isEmpty) {
    return WrapAlignment.start;
  }
  return switch (token) {
    'start' => WrapAlignment.start,
    'center' => WrapAlignment.center,
    'end' => WrapAlignment.end,
    'spaceBetween' => WrapAlignment.spaceBetween,
    'spaceAround' => WrapAlignment.spaceAround,
    'spaceEvenly' => WrapAlignment.spaceEvenly,
    _ => throw FormatException('Invalid FlowRow arrangement: $raw'),
  };
}

/// Resolves a Compose item alignment for Flutter's flow layout.
WrapCrossAlignment _wrapCrossAxis(Object? raw) {
  final token = _string(raw);
  if (token.isEmpty) {
    return WrapCrossAlignment.start;
  }
  return switch (token) {
    'start' => WrapCrossAlignment.start,
    'center' => WrapCrossAlignment.center,
    'end' => WrapCrossAlignment.end,
    _ => throw FormatException('Invalid FlowRow item alignment: $raw'),
  };
}

/// Resolves modifier ops for the Compose DSL renderer.
List<Map<String, Object?>> _modifierOps(Object? raw) {
  if (raw is! Map || raw['__modifierOps'] is! List) {
    return const <Map<String, Object?>>[];
  }
  return (raw['__modifierOps'] as List)
      .whereType<Map>()
      .map(_stringMap)
      .toList(growable: false);
}

/// Resolves axis constraints for the Compose DSL renderer.
BoxConstraints _axisConstraints(
  List<Object?> args, {
  required bool horizontal,
}) {
  final first = args.firstOrNull;
  double? min;
  double? max;
  if (first is Map) {
    min = _number(first['min'] ?? first['minWidth'] ?? first['minHeight']);
    max = _number(first['max'] ?? first['maxWidth'] ?? first['maxHeight']);
  } else {
    min = _number(first);
    max = _number(args.length > 1 ? args[1] : null);
  }
  return horizontal
      ? BoxConstraints(minWidth: min ?? 0, maxWidth: max ?? double.infinity)
      : BoxConstraints(minHeight: min ?? 0, maxHeight: max ?? double.infinity);
}

/// Resolves size constraints for the Compose DSL renderer.
BoxConstraints _sizeConstraints(List<Object?> args) {
  final first = args.firstOrNull;
  if (first is Map) {
    return BoxConstraints(
      minWidth: _number(first['minWidth']) ?? 0,
      minHeight: _number(first['minHeight']) ?? 0,
      maxWidth: _number(first['maxWidth']) ?? double.infinity,
      maxHeight: _number(first['maxHeight']) ?? double.infinity,
    );
  }
  final minWidth = _number(args.elementAtOrNull(0)) ?? 0;
  final minHeight = _number(args.elementAtOrNull(1)) ?? minWidth;
  final maxWidth = _number(args.elementAtOrNull(2)) ?? double.infinity;
  final maxHeight = _number(args.elementAtOrNull(3)) ?? maxWidth;
  return BoxConstraints(
    minWidth: minWidth,
    minHeight: minHeight,
    maxWidth: maxWidth,
    maxHeight: maxHeight,
  );
}

/// Resolves offset for the Compose DSL renderer.
Offset _offset(List<Object?> args) {
  final first = args.firstOrNull;
  if (first is Map) {
    return Offset(_number(first['x']) ?? 0, _number(first['y']) ?? 0);
  }
  return Offset(
    _number(first) ?? 0,
    _number(args.length > 1 ? args[1] : null) ?? 0,
  );
}

/// Resolves text style for the Compose DSL renderer.
TextStyle? _textStyle(BuildContext context, Map<String, Object?> props) {
  final theme = Theme.of(context).textTheme;
  final style = switch (_string(props['style'])) {
    'headlineSmall' => theme.headlineSmall,
    'headlineMedium' => theme.headlineMedium,
    'titleLarge' => theme.titleLarge,
    'titleMedium' => theme.titleMedium,
    'titleSmall' => theme.titleSmall,
    'bodyLarge' => theme.bodyLarge,
    'bodySmall' => theme.bodySmall,
    'labelLarge' => theme.labelLarge,
    'labelMedium' => theme.labelMedium,
    'labelSmall' => theme.labelSmall,
    _ => theme.bodyMedium,
  };
  return _scaledTextStyle(style!, _number(props['fontSize'])).copyWith(
    color: _color(context, props['color']),
    fontWeight: _fontWeight(_string(props['fontWeight'])),
    letterSpacing: _number(props['letterSpacing']),
  );
}

/// Resolves text field style for the Compose DSL renderer.
TextStyle? _textFieldStyle(BuildContext context, Object? raw) {
  if (raw is! Map) {
    return null;
  }
  final style = Theme.of(context).textTheme.bodyMedium!;
  return _scaledTextStyle(style, _number(raw['fontSize'])).copyWith(
    color: _color(context, raw['color']),
    fontWeight: _fontWeight(_string(raw['fontWeight'])),
    fontFamily: _string(raw['fontFamily']).trim().isEmpty
        ? null
        : _string(raw['fontFamily']).trim(),
  );
}

/// Resolves scaled text style for the Compose DSL renderer.
TextStyle _scaledTextStyle(TextStyle style, double? size) {
  if (size == null) {
    return style;
  }
  return style.apply(fontSizeFactor: size / style.fontSize!);
}

/// Resolves font weight for the Compose DSL renderer.
FontWeight? _fontWeight(String value) {
  return switch (value.toLowerCase()) {
    'bold' || 'w700' || '700' => FontWeight.w700,
    'semibold' || 'w600' || '600' => FontWeight.w600,
    'medium' || 'w500' || '500' => FontWeight.w500,
    'light' || 'w300' || '300' => FontWeight.w300,
    _ => null,
  };
}

/// Resolves main axis for the Compose DSL renderer.
MainAxisAlignment _mainAxis(Object? raw) {
  return switch (_string(raw)) {
    'center' => MainAxisAlignment.center,
    'end' => MainAxisAlignment.end,
    'spaceBetween' => MainAxisAlignment.spaceBetween,
    'spaceAround' => MainAxisAlignment.spaceAround,
    'spaceEvenly' => MainAxisAlignment.spaceEvenly,
    _ => MainAxisAlignment.start,
  };
}

/// Resolves cross axis for the Compose DSL renderer.
CrossAxisAlignment _crossAxis(Object? raw) {
  return switch (_string(raw)) {
    'center' ||
    'centerHorizontally' ||
    'centerVertically' => CrossAxisAlignment.center,
    'end' || 'right' || 'bottom' => CrossAxisAlignment.end,
    _ => CrossAxisAlignment.start,
  };
}

/// Resolves alignment for the Compose DSL renderer.
Alignment _alignment(Object? raw) {
  return switch (_string(raw)) {
    'center' => Alignment.center,
    'topCenter' || 'centerTop' => Alignment.topCenter,
    'topEnd' || 'endTop' => Alignment.topRight,
    'centerEnd' || 'endCenter' => Alignment.centerRight,
    'bottomEnd' || 'endBottom' => Alignment.bottomRight,
    'bottomCenter' || 'centerBottom' => Alignment.bottomCenter,
    'bottomStart' || 'startBottom' => Alignment.bottomLeft,
    'centerStart' || 'startCenter' => Alignment.centerLeft,
    _ => Alignment.topLeft,
  };
}

/// Resolves border radius for the Compose DSL renderer.
BorderRadius? _borderRadius(Object? raw) {
  if (raw is Map) {
    final kind = _string(raw['kind'] ?? raw['type']).toLowerCase();
    if (kind == 'circle' || kind == 'pill') {
      return BorderRadius.circular(9999);
    }
    final radius =
        _number(raw['radius']) ??
        _number(raw['all']) ??
        _number(raw['cornerRadius']);
    if (radius != null) {
      return BorderRadius.circular(radius);
    }
  }
  final token = _string(raw).toLowerCase();
  if (token == 'circle' || token == 'pill') {
    return BorderRadius.circular(9999);
  }
  final number = _number(raw);
  return number == null ? null : BorderRadius.circular(number);
}

/// Resolves shape border for the Compose DSL renderer.
OutlinedBorder? _shapeBorder(Object? raw, {BorderRadius? defaultBorderRadius}) {
  final radius = _borderRadius(raw) ?? defaultBorderRadius;
  return radius == null ? null : RoundedRectangleBorder(borderRadius: radius);
}

/// Resolves color for the Compose DSL renderer.
Color? _color(BuildContext context, Object? raw) {
  final colorScheme = Theme.of(context).colorScheme;
  if (raw is String && raw.trim().isNotEmpty) {
    final value = raw.trim();
    if (value.startsWith('#')) {
      final hex = value.substring(1);
      final parsed = int.tryParse(hex.length == 6 ? 'ff$hex' : hex, radix: 16);
      return parsed == null ? null : Color(parsed);
    }
    return _colorToken(colorScheme, value);
  }
  if (raw is Map) {
    final token = raw['__colorToken']?.toString();
    final color = _colorToken(colorScheme, token ?? '');
    final alpha = _number(raw['alpha']);
    return alpha == null ? color : color?.withValues(alpha: alpha);
  }
  return null;
}

/// Resolves color with alpha for the Compose DSL renderer.
Color? _colorWithAlpha(BuildContext context, Object? raw, Object? alphaRaw) {
  final color = _color(context, raw);
  final alpha = _number(alphaRaw);
  return color == null || alpha == null
      ? color
      : color.withValues(alpha: alpha.clamp(0, 1).toDouble());
}

/// Resolves border side for the Compose DSL renderer.
BorderSide? _borderSide(BuildContext context, Object? raw) {
  if (raw is! Map) {
    return null;
  }
  final width = _number(raw['width']) ?? 1;
  final color =
      _colorWithAlpha(context, raw['color'], raw['alpha']) ??
      Theme.of(context).colorScheme.outline;
  return BorderSide(width: width, color: color);
}

WidgetStateProperty<Color?>? _stateColor({
  required Color? checked,
  required Color? unchecked,
}) {
  if (checked == null && unchecked == null) {
    return null;
  }
  return WidgetStateProperty.resolveWith((states) {
    if (states.contains(WidgetState.selected)) {
      return checked;
    }
    return unchecked;
  });
}

WidgetStateProperty<Color?>? _buttonStateColor({
  required Color? enabled,
  required Color? disabled,
}) {
  if (enabled == null && disabled == null) {
    return null;
  }
  return WidgetStateProperty.resolveWith((states) {
    if (states.contains(WidgetState.disabled) && disabled != null) {
      return disabled;
    }
    return enabled;
  });
}

/// Resolves color token for the Compose DSL renderer.
Color? _colorToken(ColorScheme scheme, String token) {
  return switch (token) {
    'primary' => scheme.primary,
    'onPrimary' => scheme.onPrimary,
    'primaryContainer' => scheme.primaryContainer,
    'onPrimaryContainer' => scheme.onPrimaryContainer,
    'secondary' => scheme.secondary,
    'onSecondary' => scheme.onSecondary,
    'secondaryContainer' => scheme.secondaryContainer,
    'onSecondaryContainer' => scheme.onSecondaryContainer,
    'tertiary' => scheme.tertiary,
    'onTertiary' => scheme.onTertiary,
    'tertiaryContainer' => scheme.tertiaryContainer,
    'onTertiaryContainer' => scheme.onTertiaryContainer,
    'surface' => scheme.surface,
    'onSurface' => scheme.onSurface,
    'surfaceDim' => scheme.surfaceDim,
    'surfaceBright' => scheme.surfaceBright,
    'surfaceContainerLowest' => scheme.surfaceContainerLowest,
    'surfaceContainerLow' => scheme.surfaceContainerLow,
    'surfaceContainer' => scheme.surfaceContainer,
    'surfaceContainerHigh' => scheme.surfaceContainerHigh,
    'surfaceContainerHighest' => scheme.surfaceContainerHighest,
    'shadow' => scheme.shadow,
    'primaryFixed' => scheme.primaryFixed,
    'primaryFixedDim' => scheme.primaryFixedDim,
    'onPrimaryFixed' => scheme.onPrimaryFixed,
    'onPrimaryFixedVariant' => scheme.onPrimaryFixedVariant,
    'secondaryFixed' => scheme.secondaryFixed,
    'secondaryFixedDim' => scheme.secondaryFixedDim,
    'onSecondaryFixed' => scheme.onSecondaryFixed,
    'onSecondaryFixedVariant' => scheme.onSecondaryFixedVariant,
    'tertiaryFixed' => scheme.tertiaryFixed,
    'tertiaryFixedDim' => scheme.tertiaryFixedDim,
    'onTertiaryFixed' => scheme.onTertiaryFixed,
    'onTertiaryFixedVariant' => scheme.onTertiaryFixedVariant,
    'surfaceVariant' => scheme.surfaceContainerHighest,
    'onSurfaceVariant' => scheme.onSurfaceVariant,
    'background' => scheme.surface,
    'onBackground' => scheme.onSurface,
    'error' => scheme.error,
    'onError' => scheme.onError,
    'errorContainer' => scheme.errorContainer,
    'onErrorContainer' => scheme.onErrorContainer,
    'outline' => scheme.outline,
    'outlineVariant' => scheme.outlineVariant,
    'inverseSurface' => scheme.inverseSurface,
    'inverseOnSurface' => scheme.onInverseSurface,
    'inversePrimary' => scheme.inversePrimary,
    'surfaceTint' => scheme.surfaceTint,
    'scrim' => scheme.scrim,
    _ => null,
  };
}

/// Resolves icon data for the Compose DSL renderer.
IconData _iconData(String name) {
  return MaterialIconNameResolver.resolve(name);
}

/// Resolves box fit for the Compose DSL renderer.
BoxFit _boxFit(Object? raw) {
  return switch (_string(raw)) {
    'fit' || 'fitWidth' => BoxFit.fitWidth,
    'fitHeight' => BoxFit.fitHeight,
    'inside' => BoxFit.contain,
    'crop' || 'cover' => BoxFit.cover,
    'fillBounds' || 'fill' => BoxFit.fill,
    'none' => BoxFit.none,
    _ => BoxFit.contain,
  };
}

/// Resolves text input type for the Compose DSL renderer.
TextInputType? _textInputType(Object? raw) {
  return switch (_normalizeToken(_string(raw))) {
    'text' => TextInputType.text,
    'multiline' => TextInputType.multiline,
    'number' => TextInputType.number,
    'decimal' => const TextInputType.numberWithOptions(decimal: true),
    'signednumber' => const TextInputType.numberWithOptions(signed: true),
    'phone' || 'telephone' => TextInputType.phone,
    'datetime' || 'date' || 'time' => TextInputType.datetime,
    'email' || 'emailaddress' => TextInputType.emailAddress,
    'url' || 'uri' => TextInputType.url,
    'name' => TextInputType.name,
    'address' || 'streetaddress' => TextInputType.streetAddress,
    'password' || 'visiblepassword' => TextInputType.visiblePassword,
    'none' => TextInputType.none,
    _ => null,
  };
}

/// Resolves text input action for the Compose DSL renderer.
TextInputAction? _textInputAction(Object? raw) {
  return switch (_normalizeToken(_string(raw))) {
    'none' => TextInputAction.none,
    'unspecified' || 'default' => TextInputAction.unspecified,
    'done' => TextInputAction.done,
    'go' => TextInputAction.go,
    'search' => TextInputAction.search,
    'send' => TextInputAction.send,
    'next' => TextInputAction.next,
    'previous' => TextInputAction.previous,
    'continueaction' || 'continue' => TextInputAction.continueAction,
    'join' => TextInputAction.join,
    'route' => TextInputAction.route,
    'emergencycall' => TextInputAction.emergencyCall,
    'newline' => TextInputAction.newline,
    _ => null,
  };
}

/// Resolves action id for the Compose DSL renderer.
String? _actionId(Object? raw) {
  if (raw is Map) {
    final value = raw['__actionId'] ?? raw['actionId'];
    final actionId = value?.toString().trim();
    return actionId == null || actionId.isEmpty ? null : actionId;
  }
  final text = raw?.toString().trim();
  return text == null || text.isEmpty ? null : text;
}

/// Reads one required Compose action identifier from a serialized node property.
String _requiredActionId(Object? raw, String propertyName) {
  final actionId = _actionId(raw);
  if (actionId == null) {
    throw StateError('$propertyName requires a Compose action');
  }
  return actionId;
}

/// Resolves string map for the Compose DSL renderer.
Map<String, Object?> _stringMap(Object? raw) {
  if (raw is Map) {
    return raw.map((key, value) => MapEntry(key.toString(), value as Object?));
  }
  return <String, Object?>{};
}

/// Resolves canvas commands for the Compose DSL renderer.
List<Map<String, Object?>> _canvasCommands(Object? raw) {
  if (raw is! List) {
    return const <Map<String, Object?>>[];
  }
  return raw.whereType<Map>().map(_stringMap).toList(growable: false);
}

/// Resolves node list for the Compose DSL renderer.
List<_ComposeDslNode> _nodeList(Object? raw) {
  if (raw is List) {
    return raw
        .map(_ComposeDslNode.parse)
        .whereType<_ComposeDslNode>()
        .toList(growable: false);
  }
  return const <_ComposeDslNode>[];
}

/// Resolves slot map for the Compose DSL renderer.
Map<String, List<_ComposeDslNode>> _slotMap(Object? raw) {
  if (raw is Map) {
    return raw.map((key, value) => MapEntry(key.toString(), _nodeList(value)));
  }
  return const <String, List<_ComposeDslNode>>{};
}

/// Resolves string for the Compose DSL renderer.
String _string(Object? raw) => raw?.toString() ?? '';

/// Evaluates bool for the Compose DSL renderer.
bool _bool(Object? raw) {
  if (raw is bool) {
    return raw;
  }
  final text = raw?.toString().trim().toLowerCase();
  return text == 'true' || text == '1' || text == 'yes';
}

/// Resolves normalize token for the Compose DSL renderer.
String _normalizeToken(String value) =>
    value.replaceAll(RegExp(r'[^A-Za-z0-9]'), '').toLowerCase();

double? _number(Object? raw) {
  if (raw is num) {
    return raw.toDouble();
  }
  if (raw is Map && raw['value'] is num) {
    return (raw['value'] as num).toDouble();
  }
  return double.tryParse(raw?.toString() ?? '');
}

int? _int(Object? raw) {
  if (raw is int) {
    return raw;
  }
  if (raw is num) {
    return raw.toInt();
  }
  return int.tryParse(raw?.toString() ?? '');
}

extension _FirstOrNull on List<Object?> {
  Object? get firstOrNull => isEmpty ? null : first;

  /// Resolves element at or null for the Compose DSL renderer.
  Object? elementAtOrNull(int index) =>
      index < 0 || index >= length ? null : this[index];
}
