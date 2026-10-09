// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Exposes the production DSL renderer for isolated widget regression tests.
@visibleForTesting
Widget buildComposeDslLayoutForTest({
  required Map<String, Object?> node,
  required ComposeDslWebViewHostContext hostContext,
  required MarkdownContentSplitter splitMarkdownContent,
  required Future<Object?> Function(String actionId, String text) onTextInput,
}) {
  final parsed = _ComposeDslNode.parse(node);
  if (parsed == null) {
    throw const FormatException('A valid Compose DSL test node is required');
  }
  return _ComposeDslRenderer(
    node: parsed,
    onAction: hostContext.dispatchAction,
    onTextInput: onTextInput,
    webViewHostContext: hostContext,
    splitMarkdownContent: splitMarkdownContent,
  );
}

class _ComposeDslRenderer extends StatelessWidget {
  /// Creates the compose dsl renderer instance.
  const _ComposeDslRenderer({
    super.key,
    required this.node,
    required this.onAction,
    required this.onTextInput,
    required this.webViewHostContext,
    required this.splitMarkdownContent,
    this.nodePath = 'root',
    this.modifierScope = _ComposeDslModifierScope.normal,
    this.embedDialog = false,
  });

  final _ComposeDslNode node;

  /// Resolves function for the Compose DSL renderer.
  final Future<Object?> Function(String actionId, [Object? payload]) onAction;

  /// Dispatches text edits through the page-wide ordered text queue.
  final Future<Object?> Function(String actionId, String text) onTextInput;
  final ComposeDslWebViewHostContext webViewHostContext;
  final MarkdownContentSplitter splitMarkdownContent;
  final String nodePath;
  final _ComposeDslModifierScope modifierScope;
  final bool embedDialog;

  /// Builds the widget for the current DSL state.
  @override
  Widget build(BuildContext context) {
    return _guardNodeBuild(
      () => _withModifier(
        context,
        _buildNode(context),
        node.props,
        onAction,
        nodeType: node.type,
        modifierScope: modifierScope,
      ),
    );
  }

  /// Contains invalid plugin nodes without installing a global Flutter error handler.
  Widget _guardNodeBuild(Widget Function() buildNode) {
    try {
      return buildNode();
    } catch (error, stackTrace) {
      ClientLogger.e(
        'event=compose_node_render_failed '
        'context=${webViewHostContext.executionContextKey} '
        'routeInstance=${webViewHostContext.routeInstanceId} '
        'node=$nodePath type=${node.type}',
        tag: 'ToolPkgUiLauncher',
        error: error,
        stackTrace: stackTrace,
      );
      return const SizedBox.shrink();
    }
  }

  /// Builds build node for the Compose DSL renderer.
  Widget _buildNode(BuildContext context) {
    final type = node.type;
    switch (type) {
      case 'GradientRule':
        return DecoratedBox(
          decoration: BoxDecoration(
            gradient: LinearGradient(
              colors: [
                _color(context, node.props['startColor']) ?? Colors.transparent,
                _color(context, node.props['endColor']) ?? Colors.transparent,
              ],
            ),
          ),
          child: const SizedBox.expand(),
        );
      case 'ActivityDots':
        return _ComposeActivityDots(props: node.props);
      case 'Draggable':
        return _ComposeDraggable(
          props: node.props,
          onAction: onAction,
          feedback: _optionalSlot('feedback'),
          childWhenDragging: _optionalSlot('childWhenDragging'),
          child: _decoratorChild(),
        );
      case 'DragTarget':
        return _ComposeDragTarget(
          props: node.props,
          onAction: onAction,
          child: _decoratorChild(),
        );
      case 'SwipeActions':
        final startId = _actionId(node.props['onStartAction']);
        final endId = _actionId(node.props['onEndAction']);
        return SwipeActions(
          actionThreshold: (_number(node.props['actionThreshold']) ?? 0.4)
              .clamp(0.05, 1),
          enabled: _enabled(),
          onStartAction: startId == null
              ? null
              : () => unawaited(onAction(startId)),
          onEndAction: endId == null ? null : () => unawaited(onAction(endId)),
          background:
              _optionalSlot('startBackground') ?? const SizedBox.shrink(),
          secondaryBackground:
              _optionalSlot('endBackground') ?? const SizedBox.shrink(),
          child: _decoratorChild(),
        );
      case 'PopupMenu':
        final scheme = Theme.of(context).colorScheme;
        final items = (node.props['items'] as List<Object?>? ?? [])
            .map(_stringMap)
            .toList();
        return PopupMenuButton<int>(
          tooltip: _string(node.props['tooltip']),
          enabled: node.props['enabled'] != false,
          padding: EdgeInsets.zero,
          borderRadius: BorderRadius.circular(6),
          color: Color.alphaBlend(
            scheme.surfaceContainerHighest.withValues(alpha: 0.75),
            scheme.surface,
          ),
          elevation: 6,
          shadowColor: Colors.black.withValues(alpha: 0.45),
          offset: const Offset(0, 4),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(10),
            side: BorderSide(
              color: scheme.outlineVariant.withValues(alpha: 0.4),
            ),
          ),
          constraints: BoxConstraints(
            minWidth: _number(node.props['menuMinWidth']) ?? 118,
            maxWidth: _number(node.props['menuMaxWidth']) ?? 150,
          ),
          menuPadding: const EdgeInsets.all(4),
          onOpened: () => _ComposeHoverScope.menuOpenChanged(context, true),
          onCanceled: () => _ComposeHoverScope.menuOpenChanged(context, false),
          onSelected: (index) {
            _ComposeHoverScope.menuOpenChanged(context, false);
            final id = _actionId(node.props['onSelected']);
            if (id != null) unawaited(onAction(id, index));
          },
          itemBuilder: (context) => [
            for (var index = 0; index < items.length; index++) ...[
              if (items[index]['dividerBefore'] == true)
                const PopupMenuDivider(height: 8),
              PopupMenuItem<int>(
                value: index,
                height: 32,
                padding: const EdgeInsets.symmetric(horizontal: 10),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      _iconData(_string(items[index]['icon'])),
                      size: 14.5,
                      color: items[index]['danger'] == true
                          ? scheme.error.withValues(alpha: 0.9)
                          : scheme.onSurfaceVariant.withValues(alpha: 0.85),
                    ),
                    const SizedBox(width: 9),
                    Flexible(
                      child: Text(
                        _string(items[index]['label']),
                        maxLines: 1,
                        style: TextStyle(
                          fontSize: 12.5,
                          fontWeight: FontWeight.w500,
                          letterSpacing: -0.1,
                          color: items[index]['danger'] == true
                              ? scheme.error.withValues(alpha: 0.9)
                              : scheme.onSurface.withValues(alpha: 0.92),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ],
          child: Center(child: _decoratorChild()),
        );
      case 'HoverRegion':
        return _ComposeHoverRegion(
          color: _color(context, node.props['hoverBackground']),
          radius: _borderRadius(node.props['shape']) ?? BorderRadius.zero,
          child: _decoratorChild(),
        );
      case 'HoverOnly':
        return _ComposeHoverOnly(
          alwaysVisible: _bool(node.props['alwaysVisible']),
          child: _decoratorChild(),
        );
      case 'Column':
        return _ComposeFlex(
          direction: Axis.vertical,
          mainAxisSize: MainAxisSize.min,
          nodes: _slotNodes('content', useChildren: true),
          crossAxisAlignment: _crossAxis(node.props['horizontalAlignment']),
          mainAxisAlignment: _mainAxis(node.props['verticalArrangement']),
          spacing: _flexSpacing(
            node.props['verticalArrangement'],
            node.props['spacing'],
          ),
          children: _slotChildren(
            'content',
            useChildren: true,
            modifierScope: _ComposeDslModifierScope.column,
          ),
        );
      case 'LazyColumn':
        return _lazyList(Axis.vertical);
      case 'Dialog':
      case 'AlertDialog':
        return _dialog(context, type == 'AlertDialog');
      case 'Row':
        final contentNodes = _slotNodes('content', useChildren: true);
        return LayoutBuilder(
          builder: (context, constraints) => _guardNodeBuild(() {
            final children = _buildRowChildren(
              contentNodes,
              pathPrefix: '$nodePath:content',
              boundedWidth: constraints.hasBoundedWidth,
            );
            return _ComposeFlex(
              direction: Axis.horizontal,
              nodes: contentNodes,
              mainAxisSize: _nodesRequireRowFlex(contentNodes)
                  ? MainAxisSize.max
                  : MainAxisSize.min,
              crossAxisAlignment: _crossAxis(node.props['verticalAlignment']),
              mainAxisAlignment: _mainAxis(node.props['horizontalArrangement']),
              spacing: _flexSpacing(
                node.props['horizontalArrangement'],
                node.props['spacing'],
              ),
              children: children,
            );
          }),
        );
      case 'FlowRow':
        return Wrap(
          spacing: _flowSpacing(
            node.props['horizontalArrangement'],
            node.props['spacing'],
          ),
          runSpacing: _flowSpacing(
            node.props['verticalArrangement'],
            node.props['runSpacing'] ?? node.props['spacing'],
          ),
          alignment: _wrapAlignment(node.props['horizontalArrangement']),
          runAlignment: _wrapAlignment(node.props['verticalArrangement']),
          crossAxisAlignment: _wrapCrossAxis(
            node.props['itemVerticalAlignment'],
          ),
          children: _slotChildren('content', useChildren: true),
        );
      case 'LazyRow':
        return _lazyList(Axis.horizontal);
      case 'Box':
        return _ComposeBox(
          alignment: _alignment(node.props['contentAlignment']),
          nodes: _slotNodes('content', useChildren: true),
          children: _slotChildren(
            'content',
            useChildren: true,
            modifierScope: _ComposeDslModifierScope.box,
          ),
        );
      case 'Spacer':
        return SizedBox(
          width: _number(node.props['width']),
          height: _number(node.props['height']),
        );
      case 'Markdown':
        final colorScheme = Theme.of(context).colorScheme;
        return StreamMarkdownRenderer(
          content: _string(node.props['text']),
          isStreaming: false,
          textColor:
              _color(context, node.props['color']) ?? colorScheme.onSurface,
          backgroundColor:
              _color(context, node.props['backgroundColor']) ??
              Colors.transparent,
          splitMarkdownContent: splitMarkdownContent,
        );
      case 'Text':
      case 'BasicText':
        return Text(
          _string(node.props['text']),
          maxLines: _int(node.props['maxLines']),
          overflow: _string(node.props['overflow']) == 'ellipsis'
              ? TextOverflow.ellipsis
              : null,
          softWrap: node.props['softWrap'] as bool?,
          style: _textStyle(context, node.props),
        );
      case 'Button':
      case 'ElevatedButton':
      case 'FilledTonalButton':
      case 'OutlinedButton':
      case 'TextButton':
        return _button(context, type);
      case 'FloatingActionButton':
      case 'SmallFloatingActionButton':
      case 'LargeFloatingActionButton':
      case 'ExtendedFloatingActionButton':
        return _floatingActionButton(context, type);
      case 'IconButton':
      case 'FilledIconButton':
      case 'FilledTonalIconButton':
      case 'OutlinedIconButton':
      case 'IconToggleButton':
      case 'FilledIconToggleButton':
      case 'FilledTonalIconToggleButton':
      case 'OutlinedIconToggleButton':
        return _iconButton(context, type);
      case 'TextField':
      case 'OutlinedTextField':
        return _textField(context, type);
      case 'Switch':
        return _switch(context);
      case 'Checkbox':
        return _checkbox();
      case 'RadioButton':
        return _radioButton();
      case 'Card':
      case 'ElevatedCard':
      case 'OutlinedCard':
        return _card(context, type);
      case 'Surface':
        return _surface(context);
      case 'MaterialTheme':
        return _slotOrChildren('content');
      case 'Icon':
        return Icon(
          _iconData(_string(node.props['name'])),
          size: _number(node.props['size']),
          color: _color(context, node.props['tint']),
        );
      case 'LinearProgressIndicator':
        final progress = _number(node.props['progress']);
        return LinearProgressIndicator(
          value: progress?.clamp(0, 1).toDouble(),
          color: _color(context, node.props['color']),
          backgroundColor: _color(context, node.props['trackColor']),
        );
      case 'LoadingIndicator':
        return M3LoadingIndicator(
          size: _number(node.props['size']) ?? 36,
          color: _color(context, node.props['color']),
        );
      case 'CircularProgressIndicator':
        return CircularProgressIndicator(
          value: _number(node.props['progress'])?.clamp(0, 1).toDouble(),
          strokeWidth: _number(node.props['strokeWidth']) ?? 4,
          color: _color(context, node.props['color']),
          backgroundColor: _color(context, node.props['trackColor']),
        );
      case 'Divider':
      case 'HorizontalDivider':
        return Divider(
          thickness: _number(node.props['thickness']),
          color: _color(context, node.props['color']),
        );
      case 'VerticalDivider':
        return VerticalDivider(
          thickness: _number(node.props['thickness']),
          color: _color(context, node.props['color']),
        );
      case 'AssistChip':
      case 'ElevatedAssistChip':
      case 'FilterChip':
      case 'ElevatedFilterChip':
      case 'SuggestionChip':
      case 'ElevatedSuggestionChip':
      case 'InputChip':
        return _chip(context, type);
      case 'Badge':
        return _badge(context);
      case 'BadgedBox':
        return _badgedBox(context);
      case 'Snackbar':
        return _snackbar(context);
      case 'ListItem':
        return _listItem(context);
      case 'NavigationBar':
      case 'ShortNavigationBar':
        return _navigationBar(context);
      case 'NavigationRail':
      case 'WideNavigationRail':
      case 'ModalWideNavigationRail':
        return _navigationRail(context);
      case 'NavigationRailItem':
      case 'WideNavigationRailItem':
      case 'ShortNavigationBarItem':
        return _navigationItemTile();
      case 'NavigationDrawerItem':
        return _navigationItemTile();
      case 'DismissibleDrawerSheet':
      case 'ModalDrawerSheet':
      case 'PermanentDrawerSheet':
        return _drawerSheet(context);
      case 'DismissibleNavigationDrawer':
      case 'ModalNavigationDrawer':
      case 'PermanentNavigationDrawer':
        return _navigationDrawer(context);
      case 'Tab':
      case 'LeadingIconTab':
        return _tabItem(context, leadingIcon: type == 'LeadingIconTab');
      case 'PrimaryTabRow':
      case 'SecondaryTabRow':
      case 'PrimaryScrollableTabRow':
      case 'SecondaryScrollableTabRow':
        return _tabRow(context, type);
      case 'Scaffold':
        return _scaffold(context);
      case 'BoxWithConstraints':
        return LayoutBuilder(
          builder: (context, constraints) => Stack(
            alignment: _alignment(node.props['contentAlignment']),
            children: _slotChildren(
              'content',
              useChildren: true,
              modifierScope: _ComposeDslModifierScope.box,
            ),
          ),
        );
      case 'SelectionContainer':
        return SelectionArea(child: _slotOrChildren('content'));
      case 'DisableSelection':
        return _slotOrChildren('content');
      case 'ProvideTextStyle':
        return DefaultTextStyle.merge(
          style: _textStyle(context, node.props) ?? const TextStyle(),
          child: _slotOrChildren('content'),
        );
      case 'SnackbarHost':
        return _slotOrChildren('content');
      case 'AiChat':
        return const AIChatEmbed();
      case 'AdaptiveSidePanel':
        return _adaptiveSidePanel();
      case 'PullToRefreshBox':
        return _pullToRefreshBox(context);
      case 'DropdownMenu':
        return _dropdownMenu(context);
      case 'TimePickerDialog':
        return _timePickerDialog(context);
      case 'VerticalDragHandle':
        return _verticalDragHandle(context);
      case 'Canvas':
        return _canvas(context);
      case 'WebView':
        return ComposeDslWebView(
          key: _webViewKey(
            props: node.props,
            nodePath: nodePath,
            webViewHostContext: webViewHostContext,
          ),
          props: node.props,
          onAction: onAction,
          hostContext: webViewHostContext,
        );
      case 'Image':
      case 'AsyncImage':
        return _image(context);
      default:
        return _childrenColumn();
    }
  }
}
