// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Renders material controls for Compose DSL nodes.
extension _ComposeMaterialControls on _ComposeProgramExecutorState {
  /// Builds button for the Compose DSL renderer.
  Widget _button(BuildContext context, String type) {
    final contentChildren = _slotChildren(
      'content',
      useChildren: true,
      modifierScope: _ComposeDslModifierScope.row,
    );
    final child = contentChildren.isNotEmpty
        ? Row(mainAxisSize: MainAxisSize.min, children: contentChildren)
        : Text(
            _string(props['text']).isEmpty
                ? type
                : _string(props['text']),
          );
    final onPressed = _enabled()
        ? () => _invokeAction(props['onClick'])
        : null;
    final style = _buttonStyle(context);
    switch (type) {
      case 'OutlinedButton':
        return OutlinedButton(onPressed: onPressed, style: style, child: child);
      case 'TextButton':
        return TextButton(onPressed: onPressed, style: style, child: child);
      case 'FilledTonalButton':
        return FilledButton.tonal(
          onPressed: onPressed,
          style: style,
          child: child,
        );
      default:
        return FilledButton(onPressed: onPressed, style: style, child: child);
    }
  }

  /// Builds card for the Compose DSL renderer.
  Widget _card(BuildContext context, String type) {
    final defaultRadius = type == 'Card' ? BorderRadius.circular(12) : null;
    final radius = _borderRadius(props['shape']) ?? defaultRadius;
    final borderSide =
        _borderSide(context, props['border']) ??
        (type == 'OutlinedCard'
            ? BorderSide(color: Theme.of(context).colorScheme.outlineVariant)
            : null);
    final child = _containerContent(
      context,
      contentColor: _colorWithAlpha(
        context,
        props['contentColor'],
        props['contentAlpha'],
      ),
      contentPadding: props['contentPadding'],
    );
    final themeSurface = props['themeSurface'];
    if (themeSurface != null) {
      final layer = switch (themeSurface) {
        'panel' => OperitGlassSurfaceLayer.panel,
        'card' => OperitGlassSurfaceLayer.card,
        'control' => OperitGlassSurfaceLayer.control,
        _ => throw FormatException(
          'Unknown theme surface layer: $themeSurface',
        ),
      };
      final color = _colorWithAlpha(
        context,
        props['containerColor'],
        props['containerAlpha'] ?? props['alpha'],
      );
      if (color == null) {
        throw const FormatException('Themed cards require a containerColor');
      }
      return OperitGlassSurface(
        color: color,
        layer: layer,
        borderRadius: radius ?? BorderRadius.zero,
        border: borderSide == null ? null : Border.fromBorderSide(borderSide),
        material: true,
        child: child,
      );
    }
    return Card(
      margin: EdgeInsets.zero,
      color: _colorWithAlpha(
        context,
        props['containerColor'],
        props['containerAlpha'] ?? props['alpha'],
      ),
      elevation:
          _number(props['elevation']) ?? (type == 'ElevatedCard' ? 3 : 1),
      shape: RoundedRectangleBorder(
        borderRadius: radius ?? BorderRadius.zero,
        side: borderSide ?? BorderSide.none,
      ),
      child: child,
    );
  }

  /// Builds surface for the Compose DSL renderer.
  Widget _surface(BuildContext context) {
    final radius = _borderRadius(props['shape']);
    final borderSide = _borderSide(context, props['border']);
    final rawShape = props['shape'];
    final kind = _normalizeToken(
      _string(
        rawShape is Map ? rawShape['type'] ?? rawShape['kind'] : rawShape,
      ),
    );
    final ShapeBorder? shape = switch (kind) {
      'pill' => StadiumBorder(side: borderSide ?? BorderSide.none),
      'circle' => CircleBorder(side: borderSide ?? BorderSide.none),
      _ =>
        borderSide == null && radius == null
            ? null
            : RoundedRectangleBorder(
                borderRadius: radius ?? BorderRadius.zero,
                side: borderSide ?? BorderSide.none,
              ),
    };
    final child = _surfaceContent(
      context,
      contentColor: _color(context, props['contentColor']),
      contentPadding: props['contentPadding'] ?? props['padding'],
    );
    final actionId = _actionId(props['onClick']);
    final surfaceChild = actionId == null
        ? child
        : InkWell(
            borderRadius: radius,
            onTap: _enabled()
                ? () => _invokeAction(props['onClick'])
                : null,
            child: child,
          );
    return Material(
      color:
          _colorWithAlpha(
            context,
            props['color'] ?? props['containerColor'],
            props['alpha'],
          ) ??
          Colors.transparent,
      elevation:
          _number(props['shadowElevation']) ??
          _number(props['tonalElevation']) ??
          0,
      shape: shape,
      borderRadius: shape == null ? radius : null,
      clipBehavior: shape == null ? Clip.none : Clip.antiAlias,
      child: surfaceChild,
    );
  }

  /// Builds container content for the Compose DSL renderer.
  Widget _containerContent(
    BuildContext context, {
    required Color? contentColor,
    required Object? contentPadding,
  }) {
    Widget child = _slotOrChildren('content');
    if (contentPadding != null) {
      child = Padding(
        padding: _edgeInsetsFromValue(contentPadding),
        child: child,
      );
    }
    return _withSlotColor(context, child, contentColor);
  }

  /// Builds surface content for the Compose DSL renderer.
  Widget _surfaceContent(
    BuildContext context, {
    required Color? contentColor,
    required Object? contentPadding,
  }) {
    Widget child = Stack(
      children: _slotChildren(
        'content',
        useChildren: true,
        modifierScope: _ComposeDslModifierScope.normal,
      ),
    );
    if (contentPadding != null) {
      child = Padding(
        padding: _edgeInsetsFromValue(contentPadding),
        child: child,
      );
    }
    return _withSlotColor(context, child, contentColor);
  }

  ButtonStyle? _buttonStyle(BuildContext context) {
    final containerColor = _color(context, props['containerColor']);
    final contentColor = _color(context, props['contentColor']);
    final disabledContainerColor = _color(
      context,
      props['disabledContainerColor'],
    );
    final disabledContentColor = _color(
      context,
      props['disabledContentColor'],
    );
    final radius = _borderRadius(props['shape']);
    final padding = props['contentPadding'];
    final borderSide = _borderSide(context, props['border']);
    if (containerColor == null &&
        contentColor == null &&
        disabledContainerColor == null &&
        disabledContentColor == null &&
        radius == null &&
        padding == null &&
        borderSide == null) {
      return null;
    }
    return ButtonStyle(
      backgroundColor: _buttonStateColor(
        enabled: containerColor,
        disabled: disabledContainerColor,
      ),
      foregroundColor: _buttonStateColor(
        enabled: contentColor,
        disabled: disabledContentColor,
      ),
      shape: radius == null
          ? null
          : WidgetStatePropertyAll<OutlinedBorder>(
              RoundedRectangleBorder(borderRadius: radius),
            ),
      side: borderSide == null
          ? null
          : WidgetStatePropertyAll<BorderSide>(borderSide),
      padding: padding == null
          ? null
          : WidgetStatePropertyAll<EdgeInsetsGeometry>(
              _edgeInsetsFromValue(padding),
            ),
    );
  }

  /// Builds chip for the Compose DSL renderer.
  Widget _chip(BuildContext context, String type) {
    final selected =
        _bool(props['selected']) || _bool(props['checked']);
    final label = _chipLabel(type);
    final onPressed = _enabled()
        ? () => _invokeAction(props['onClick'] ?? props['onSelected'])
        : null;
    final backgroundColor = _color(context, props['containerColor']);
    final selectedColor = _color(context, props['selectedContainerColor']);
    final contentColor = _color(context, props['contentColor']);
    final disabledColor = _color(context, props['disabledContainerColor']);
    final labelStyle = contentColor == null
        ? null
        : TextStyle(color: contentColor);
    final shape = _shapeBorder(
      props['shape'],
      defaultBorderRadius: BorderRadius.zero,
    );
    final elevation = type.startsWith('Elevated') ? 1.0 : 0.0;
    if (type.contains('Filter')) {
      return FilterChip(
        label: label,
        selected: selected,
        selectedColor: selectedColor,
        backgroundColor: backgroundColor,
        disabledColor: disabledColor,
        labelStyle: labelStyle,
        shape: shape,
        elevation: elevation,
        onSelected: _enabled()
            ? (_) => _invokeAction(
                props['onClick'] ?? props['onSelected'],
              )
            : null,
      );
    }
    if (type == 'InputChip') {
      final dismissAction = props['onDismiss'] ?? props['onDelete'];
      return InputChip(
        label: label,
        selected: selected,
        selectedColor: selectedColor,
        backgroundColor: backgroundColor,
        disabledColor: disabledColor,
        labelStyle: labelStyle,
        shape: shape,
        onPressed: onPressed,
        deleteIcon: _actionId(dismissAction) == null
            ? null
            : const Icon(Icons.close, size: 18),
        onDeleted: _actionId(dismissAction) == null
            ? null
            : () => _invokeAction(dismissAction),
      );
    }
    return ActionChip(
      label: label,
      backgroundColor: backgroundColor,
      disabledColor: disabledColor,
      labelStyle: labelStyle,
      shape: shape,
      elevation: elevation,
      onPressed: onPressed,
    );
  }

  /// Builds chip label for the Compose DSL renderer.
  Widget _chipLabel(String type) {
    final parts = <Widget>[];
    void addSlot(String name, {VoidCallback? onTap}) {
      if (_hasSlot(name)) {
        if (parts.isNotEmpty) {
          parts.add(const SizedBox(width: 6));
        }
        parts.add(_chipSlotPart(name, onTap: onTap));
      }
    }

    if (type == 'InputChip') {
      addSlot('avatar');
      addSlot('leadingIcon');
    } else if (type.contains('Suggestion')) {
      addSlot('icon');
    } else {
      addSlot('leadingIcon');
    }
    if (parts.isNotEmpty) {
      parts.add(const SizedBox(width: 6));
    }
    parts.add(
      _hasSlot('label')
          ? _chipSlotInline('label')
          : Text(
              _string(props['label']).isEmpty
                  ? type.replaceAll('Elevated', '')
                  : _string(props['label']),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
    );
    final dismissAction = props['onDismiss'] ?? props['onDelete'];
    if (type == 'InputChip' && _actionId(dismissAction) != null) {
      addSlot('trailingIcon', onTap: () => _invokeAction(dismissAction));
    } else {
      addSlot('trailingIcon');
    }
    return FittedBox(
      fit: BoxFit.scaleDown,
      alignment: Alignment.centerLeft,
      child: Row(mainAxisSize: MainAxisSize.min, children: parts),
    );
  }

  /// Builds chip slot part for the Compose DSL renderer.
  Widget _chipSlotPart(String name, {VoidCallback? onTap}) {
    Widget child = _chipSlotInline(name);
    if (onTap != null) {
      child = GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: onTap,
        child: child,
      );
    }
    return child;
  }

  /// Builds chip slot inline for the Compose DSL renderer.
  Widget _chipSlotInline(String name) {
    final slot = node.slots[name] ?? const <_ComposeProgramNode>[];
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: slot
          .asMap()
          .entries
          .map(
            (entry) =>
                _chipInlineChild(entry.value, '$nodePath:$name/${entry.key}'),
          )
          .toList(growable: false),
    );
  }

  /// Builds chip inline child for the Compose DSL renderer.
  Widget _chipInlineChild(_ComposeProgramNode child, String childPath) {
    if (child.type == 'Text') {
      return Text(
        _string(child.props['text']),
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      );
    }
    return _ComposeProgramExecutor(
      node: child,
      onAction: onAction,
      onTextInput: onTextInput,
      webViewHostContext: webViewHostContext,
      splitMarkdownContent: splitMarkdownContent,
      dispatchOnLoad: widget.dispatchOnLoad,
      nodePath: childPath,
    );
  }

  /// Builds icon button for the Compose DSL renderer.
  Widget _iconButton(BuildContext context, String type) {
    final toggle = type.contains('Toggle');
    final checked = _bool(props['checked']);
    final icon = _slotNodes('content', useChildren: true).isNotEmpty
        ? _iconButtonSlot('content', useChildren: true)
        : Icon(
            _iconData(
              _string(props['icon']).isEmpty
                  ? _string(props['name'])
                  : _string(props['icon']),
            ),
          );
    final selectedIcon = _hasSlot('selectedIcon')
        ? _iconButtonSlot('selectedIcon')
        : null;
    final onPressed = _enabled()
        ? () => _invokeAction(
            toggle ? props['onCheckedChange'] : props['onClick'],
            toggle ? !checked : null,
          )
        : null;
    final style = _iconButtonStyle(type);
    switch (type) {
      case 'FilledIconButton':
      case 'FilledIconToggleButton':
        return IconButton.filled(
          onPressed: onPressed,
          tooltip: props['tooltip'] as String?,
          isSelected: toggle ? checked : null,
          selectedIcon: selectedIcon,
          style: style,
          icon: icon,
        );
      case 'FilledTonalIconButton':
      case 'FilledTonalIconToggleButton':
        return IconButton.filledTonal(
          onPressed: onPressed,
          tooltip: props['tooltip'] as String?,
          isSelected: toggle ? checked : null,
          selectedIcon: selectedIcon,
          style: style,
          icon: icon,
        );
      case 'OutlinedIconButton':
      case 'OutlinedIconToggleButton':
        return IconButton.outlined(
          onPressed: onPressed,
          tooltip: props['tooltip'] as String?,
          isSelected: toggle ? checked : null,
          selectedIcon: selectedIcon,
          style: style,
          icon: icon,
        );
      default:
        return IconButton(
          onPressed: onPressed,
          tooltip: props['tooltip'] as String?,
          isSelected: toggle ? checked : null,
          selectedIcon: selectedIcon,
          style: style,
          icon: icon,
        );
    }
  }

  ButtonStyle? _iconButtonStyle(String type) {
    final hasExplicitSize =
        _number(props['width']) != null ||
        _number(props['height']) != null;
    final shape = _shapeBorder(
      props['shape'],
      defaultBorderRadius: type.contains('Outlined')
          ? BorderRadius.circular(12)
          : null,
    );
    if (!hasExplicitSize &&
        shape == null &&
        props['contentPadding'] == null) {
      return null;
    }
    return IconButton.styleFrom(
      shape: shape,
      minimumSize: hasExplicitSize ? Size.zero : null,
      padding: props['contentPadding'] != null
          ? _edgeInsetsFromValue(props['contentPadding'])
          : hasExplicitSize
          ? EdgeInsets.zero
          : null,
      tapTargetSize: hasExplicitSize ? MaterialTapTargetSize.shrinkWrap : null,
    );
  }

  /// Builds icon button slot for the Compose DSL renderer.
  Widget _iconButtonSlot(String name, {bool useChildren = false}) => Column(
    crossAxisAlignment: CrossAxisAlignment.center,
    mainAxisSize: MainAxisSize.min,
    children: _slotChildren(name, useChildren: useChildren),
  );

  /// Builds floating action button for the Compose DSL renderer.
  Widget _floatingActionButton(BuildContext context, String type) {
    final onPressed = _enabled()
        ? () => _invokeAction(props['onClick'])
        : null;
    final backgroundColor = _color(context, props['containerColor']);
    final foregroundColor = _color(context, props['contentColor']);
    final shape = _shapeBorder(
      props['shape'],
      defaultBorderRadius: BorderRadius.zero,
    );
    if (type == 'ExtendedFloatingActionButton') {
      return FloatingActionButton.extended(
        heroTag: null,
        onPressed: onPressed,
        backgroundColor: backgroundColor,
        foregroundColor: foregroundColor,
        shape: shape,
        icon: _hasSlot('icon') ? _slotRow('icon') : null,
        label: _slotRow('content', useChildren: true),
      );
    }
    final child = _hasSlot('content')
        ? _slotColumn('content')
        : Icon(_iconData(_string(props['icon'])));
    if (type == 'SmallFloatingActionButton') {
      return FloatingActionButton.small(
        heroTag: null,
        onPressed: onPressed,
        backgroundColor: backgroundColor,
        foregroundColor: foregroundColor,
        shape: shape,
        child: child,
      );
    }
    if (type == 'LargeFloatingActionButton') {
      return FloatingActionButton.large(
        heroTag: null,
        onPressed: onPressed,
        backgroundColor: backgroundColor,
        foregroundColor: foregroundColor,
        shape: shape,
        child: child,
      );
    }
    return FloatingActionButton(
      heroTag: null,
      onPressed: onPressed,
      backgroundColor: backgroundColor,
      foregroundColor: foregroundColor,
      shape: shape,
      child: child,
    );
  }

  /// Builds badge for the Compose DSL renderer.
  Widget _badge(BuildContext context) {
    final contentColor = _color(context, props['contentColor']);
    final label = _tintedSlotInline(
      context,
      'content',
      useChildren: true,
      color: contentColor,
    );
    return Badge(
      backgroundColor: _color(context, props['containerColor']),
      textColor: contentColor,
      label: label,
    );
  }

  /// Builds badged box for the Compose DSL renderer.
  Widget _badgedBox(BuildContext context) {
    return Badge(
      label: _hasSlot('badge') ? _slotColumn('badge') : null,
      child: _slotOrChildren('content'),
    );
  }

  /// Builds snackbar for the Compose DSL renderer.
  Widget _snackbar(BuildContext context) {
    final contentColor =
        _color(context, props['contentColor']) ??
        Theme.of(context).colorScheme.onInverseSurface;
    final actionContentColor =
        _color(context, props['actionContentColor']) ??
        Theme.of(context).colorScheme.inversePrimary;
    final dismissActionContentColor =
        _color(context, props['dismissActionContentColor']) ??
        actionContentColor;
    final content = _tintedSlotColumn(
      context,
      'content',
      useChildren: true,
      color: contentColor,
    );
    final actions = <Widget>[
      if (_hasSlot('action'))
        _tintedSlotInline(context, 'action', color: actionContentColor),
      if (_hasSlot('dismissAction'))
        _tintedSlotInline(
          context,
          'dismissAction',
          color: dismissActionContentColor,
        ),
    ];
    final actionOnNewLine = _bool(props['actionOnNewLine']);
    final shape =
        _shapeBorder(
              props['shape'],
              defaultBorderRadius: BorderRadius.zero,
            )
            as RoundedRectangleBorder?;
    final child = actionOnNewLine
        ? Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              content,
              if (actions.isNotEmpty) ...<Widget>[
                const SizedBox(height: 8),
                Row(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: actions,
                ),
              ],
            ],
          )
        : Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: <Widget>[
              Expanded(child: content),
              ...actions.map(
                (action) => Padding(
                  padding: const EdgeInsetsDirectional.only(start: 8),
                  child: action,
                ),
              ),
            ],
          );
    return Material(
      color:
          _color(context, props['containerColor']) ??
          Theme.of(context).colorScheme.inverseSurface,
      elevation: _number(props['elevation']) ?? 6,
      shape: shape,
      borderRadius: shape == null ? BorderRadius.zero : null,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
        child: child,
      ),
    );
  }

  /// Builds list item for the Compose DSL renderer.
  Widget _listItem(BuildContext context) {
    final leading = _hasSlot('leadingContent')
        ? Padding(
            padding: const EdgeInsetsDirectional.only(end: 16),
            child: _slotCompactColumn('leadingContent'),
          )
        : null;
    final trailing = _hasSlot('trailingContent')
        ? Padding(
            padding: const EdgeInsetsDirectional.only(start: 16),
            child: _slotCompactColumn('trailingContent'),
          )
        : null;
    final textColumn = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        if (_hasSlot('overlineContent')) _slotColumn('overlineContent'),
        _slotColumn('headlineContent'),
        if (_hasSlot('supportingContent')) _slotColumn('supportingContent'),
      ],
    );
    return Material(
      color: Colors.transparent,
      elevation: _number(props['shadowElevation']) ?? 0,
      surfaceTintColor: Theme.of(context).colorScheme.surfaceTint,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: <Widget>[
            ?leading,
            Expanded(child: textColumn),
            ?trailing,
          ],
        ),
      ),
    );
  }
}
