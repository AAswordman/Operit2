// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Renders renderer slots for Compose DSL nodes.
extension _ComposeRendererSlots on _ComposeDslRendererState {
  /// Evaluates enabled for the Compose DSL renderer.
  bool _enabled() => node.props['enabled'] != false;

  /// Resolves children for the Compose DSL renderer.
  List<Widget> _children({
    _ComposeDslModifierScope modifierScope = _ComposeDslModifierScope.normal,
  }) => _buildNodeWidgets(
    node.children,
    pathPrefix: nodePath,
    modifierScope: modifierScope,
  );

  /// A native decorator must not reinterpret its child's row weight as column flex.
  Widget _decoratorChild() =>
      node.children.length == 1 ? _children().single : _childrenColumn();

  /// Renders a single decorator slot without imposing a Row/Column flex scope.
  Widget? _optionalSlot(String name) {
    final nodes = node.slots[name];
    if (nodes == null || nodes.isEmpty) return null;
    final widgets = _buildNodeWidgets(nodes, pathPrefix: '$nodePath:$name');
    return widgets.length == 1
        ? widgets.single
        : Column(mainAxisSize: MainAxisSize.min, children: widgets);
  }

  /// Keeps Compose content start-aligned without forcing a cross-axis size.
  Widget _childrenColumn() => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    mainAxisSize: MainAxisSize.min,
    children: _children(modifierScope: _ComposeDslModifierScope.column),
  );

  /// Builds slot row for the Compose DSL renderer.
  Widget _slotRow(String name, {bool useChildren = false}) => Row(
    mainAxisSize: MainAxisSize.min,
    children: _slotChildren(
      name,
      useChildren: useChildren,
      modifierScope: _ComposeDslModifierScope.row,
    ),
  );

  /// Builds slot or text for the Compose DSL renderer.
  Widget _slotOrText(String name) {
    final slot = node.slots[name];
    if (slot != null && slot.isNotEmpty) {
      return _ComposeDslRenderer(
        node: slot.first,
        onAction: onAction,
        onTextInput: onTextInput,
        webViewHostContext: webViewHostContext,
        splitMarkdownContent: splitMarkdownContent,
        nodePath: '$nodePath:$name/0',
      );
    }
    final text = _string(node.props[name]);
    return Text(text.isEmpty ? name : text);
  }

  /// Lays out slot content with the same start alignment as direct children.
  Widget _slotOrChildren(String name) {
    final slotChildren = _slotChildren(
      name,
      modifierScope: _ComposeDslModifierScope.column,
    );
    if (slotChildren.isNotEmpty) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: slotChildren,
      );
    }
    return _childrenColumn();
  }

  /// Resolves slot children for the Compose DSL renderer.
  List<Widget> _slotChildren(
    String name, {
    bool useChildren = false,
    _ComposeDslModifierScope modifierScope = _ComposeDslModifierScope.normal,
  }) => _buildNodeWidgets(
    _slotNodes(name, useChildren: useChildren),
    pathPrefix: '$nodePath:$name',
    modifierScope: modifierScope,
  );

  /// Resolves slot nodes for the Compose DSL renderer.
  List<_ComposeDslNode> _slotNodes(String name, {bool useChildren = false}) {
    final slot = node.slots[name];
    return slot != null && slot.isNotEmpty
        ? slot
        : (useChildren ? node.children : const <_ComposeDslNode>[]);
  }

  /// Resolves build node widgets for the Compose DSL renderer.
  List<Widget> _buildNodeWidgets(
    List<_ComposeDslNode> nodes, {
    required String pathPrefix,
    _ComposeDslModifierScope modifierScope = _ComposeDslModifierScope.normal,
  }) {
    return nodes
        .asMap()
        .entries
        .map(
          (entry) => _ComposeDslRenderer(
            key: entry.value.props['key'] == null
                ? null
                : ValueKey(entry.value.props['key']),
            node: entry.value,
            onAction: onAction,
            onTextInput: onTextInput,
            webViewHostContext: webViewHostContext,
            splitMarkdownContent: splitMarkdownContent,
            nodePath: '$pathPrefix/${entry.key}',
            modifierScope: modifierScope,
          ),
        )
        .toList(growable: false);
  }

  /// Applies implicit text flex only to bounded rows without explicit weights.
  List<Widget> _buildRowChildren(
    List<_ComposeDslNode> nodes, {
    required String pathPrefix,
    required bool boundedWidth,
  }) {
    final widgets = _buildNodeWidgets(
      nodes,
      pathPrefix: pathPrefix,
      modifierScope: _ComposeDslModifierScope.row,
    );
    if (!boundedWidth || _nodesRequireRowFlex(nodes)) {
      return widgets;
    }
    return nodes
        .asMap()
        .entries
        .map((entry) {
          final child = entry.value;
          if ((child.type == 'Text' || child.type == 'BasicText') &&
              _rowFlexSpec(child.props) == null) {
            return Flexible(fit: FlexFit.loose, child: widgets[entry.key]);
          }
          return widgets[entry.key];
        })
        .toList(growable: false);
  }

  /// Evaluates nodes require row flex for the Compose DSL renderer.
  bool _nodesRequireRowFlex(List<_ComposeDslNode> nodes) {
    return nodes.any((child) => _rowFlexSpec(child.props) != null);
  }

  /// Lets slot children request their own widths through explicit modifiers.
  Widget _slotColumn(String name) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    mainAxisSize: MainAxisSize.min,
    children: _slotChildren(
      name,
      modifierScope: _ComposeDslModifierScope.column,
    ),
  );

  /// Builds slot compact column for the Compose DSL renderer.
  Widget _slotCompactColumn(String name) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    mainAxisSize: MainAxisSize.min,
    children: _slotChildren(
      name,
      modifierScope: _ComposeDslModifierScope.column,
    ),
  );

  /// Builds slot inline for the Compose DSL renderer.
  Widget _slotInline(String name) => Row(
    mainAxisSize: MainAxisSize.min,
    children: _slotChildren(name, modifierScope: _ComposeDslModifierScope.row),
  );

  /// Builds tinted slot inline for the Compose DSL renderer.
  Widget _tintedSlotInline(
    BuildContext context,
    String name, {
    bool useChildren = false,
    Color? color,
  }) {
    return _withSlotColor(
      context,
      Row(
        mainAxisSize: MainAxisSize.min,
        children: _slotChildren(
          name,
          useChildren: useChildren,
          modifierScope: _ComposeDslModifierScope.row,
        ),
      ),
      color,
    );
  }

  /// Applies slot color without introducing implicit cross-axis stretching.
  Widget _tintedSlotColumn(
    BuildContext context,
    String name, {
    bool useChildren = false,
    Color? color,
  }) {
    return _withSlotColor(
      context,
      Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: _slotChildren(
          name,
          useChildren: useChildren,
          modifierScope: _ComposeDslModifierScope.column,
        ),
      ),
      color,
    );
  }

  /// Builds with slot color for the Compose DSL renderer.
  Widget _withSlotColor(BuildContext context, Widget child, Color? color) {
    if (color == null) {
      return child;
    }
    return DefaultTextStyle.merge(
      style: TextStyle(color: color),
      child: IconTheme.merge(
        data: IconThemeData(color: color),
        child: child,
      ),
    );
  }

  /// Evaluates has slot for the Compose DSL renderer.
  bool _hasSlot(String name) => (node.slots[name]?.isNotEmpty ?? false);

  /// Evaluates has slot from for the Compose DSL renderer.
  bool _hasSlotFrom(_ComposeDslNode source, String name) =>
      source.slots[name]?.isNotEmpty ?? false;

  Widget? _slotFrom(_ComposeDslNode source, String name) {
    final slot = source.slots[name];
    if (slot == null || slot.isEmpty) {
      return null;
    }
    return _ComposeDslRenderer(
      node: slot.first,
      onAction: onAction,
      onTextInput: onTextInput,
      webViewHostContext: webViewHostContext,
      splitMarkdownContent: splitMarkdownContent,
      nodePath: '$nodePath:$name/0',
    );
  }

  /// Resolves plain text from for the Compose DSL renderer.
  String? _plainTextFrom(_ComposeDslNode source, String name) {
    final value = source.props[name];
    if (value is String && value.trim().isNotEmpty) {
      return value;
    }
    final slot = source.slots[name];
    if (slot != null && slot.length == 1 && slot.first.type == 'Text') {
      return _string(slot.first.props['text']);
    }
    return null;
  }

  /// Resolves plain slot text for the Compose DSL renderer.
  String? _plainSlotText(String name) {
    final value = node.props[name];
    if (value is String) {
      return value;
    }
    final slot = node.slots[name];
    if (slot != null && slot.length == 1 && slot.first.type == 'Text') {
      return _string(slot.first.props['text']);
    }
    return null;
  }

  /// Resolves invoke action for the Compose DSL renderer.
  void _invokeAction(Object? rawAction, [Object? payload]) {
    final actionId = _actionId(rawAction);
    if (actionId == null) {
      return;
    }
    onAction(actionId, payload);
  }

  /// Resolves selected index for the Compose DSL renderer.
  int _selectedIndex(List<_ComposeDslNode> items, Object? rawIndex) {
    final explicit = _int(rawIndex);
    if (explicit != null && explicit >= 0 && explicit < items.length) {
      return explicit;
    }
    final selected = items.indexWhere((item) => _bool(item.props['selected']));
    return selected < 0 ? 0 : selected;
  }

  /// Resolves image source for the Compose DSL renderer.
  String _imageSource() {
    final raw =
        node.props['model'] ??
        node.props['data'] ??
        node.props['url'] ??
        node.props['uri'] ??
        node.props['path'] ??
        node.props['fileUri'] ??
        node.props['src'];
    if (raw is Map<Object?, Object?>) {
      return _string(
        raw['url'] ?? raw['uri'] ?? raw['path'] ?? raw['fileUri'] ?? raw['src'],
      );
    }
    return _string(raw);
  }

  /// Builds image for the Compose DSL renderer.
  Widget _image(BuildContext context) {
    final alpha = (_number(node.props['alpha']) ?? 1).clamp(0, 1).toDouble();
    final fit = _boxFit(node.props['contentScale']);
    final description = _string(node.props['contentDescription']).trim();
    Widget child;
    final source = _imageSource().trim();
    if (source.isNotEmpty) {
      child = _ComposeStableImage(source: source, fit: fit);
    } else {
      final iconName = _string(
        node.props['name'] ?? node.props['icon'] ?? 'info',
      );
      child = Icon(
        _iconData(iconName),
        color: _color(context, node.props['tint']),
        size: _number(node.props['size']),
      );
    }
    if (alpha < 1) {
      child = Opacity(opacity: alpha, child: child);
    }
    if (description.isNotEmpty) {
      child = Semantics(label: description, image: true, child: child);
    }
    return child;
  }
}
