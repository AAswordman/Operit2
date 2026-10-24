// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Exposes the production DSL renderer for isolated widget regression tests.
@visibleForTesting
Widget buildComposeDslLayoutForTest({
  required Map<String, Object?> node,
  required ComposeDslWebViewHostContext hostContext,
  required MarkdownContentSplitter splitMarkdownContent,
  required Future<Object?> Function(String actionId, String text) onTextInput,
  ToolPkgUiContentBuilder contentBuilder = _buildToolPkgUiBody,
  bool onLoadHandledByHost = false,
}) {
  final parsed = _composeProgramFixture(node);
  final renderer = _ComposeProgramExecutor(
    node: parsed,
    onAction: hostContext.dispatchAction,
    onTextInput: onTextInput,
    webViewHostContext: hostContext,
    splitMarkdownContent: splitMarkdownContent,
    onLoadHandledByHost: onLoadHandledByHost,
  );
  return Builder(
    builder: (context) => contentBuilder(
      context,
      ToolPkgUiContent._(
        body: renderer,
        sliver: _ComposeDslSliverContent(renderer: renderer),
      ),
    ),
  );
}

/// Applies the production retained store and renderer for isolated first-frame measurements.
@visibleForTesting
Widget buildComposeProgramLayoutForTest({
  required core_proxy.ToolPkgComposeDslTransaction update,
  required ComposeDslWebViewHostContext hostContext,
  required void Function(VoidCallback dispose) registerDispose,
  void Function(void Function(core_proxy.ToolPkgComposeDslTransaction) apply)?
  registerApply,
}) {
  final store = _ComposeProgramStore();
  store.apply(update);
  registerDispose(store.dispose);
  registerApply?.call(store.apply);
  /// Resolves host callback slots at execution time, including events queued before a reorder.
  Future<Object?> dispatch(String id, [Object? payload]) => hostContext.dispatchAction(id, payload);
  final resolvedHost = ComposeDslWebViewHostContext(
    clients: hostContext.clients,
    packageName: hostContext.packageName,
    routeInstanceId: hostContext.routeInstanceId,
    executionContextKey: hostContext.executionContextKey,
    dispatchAction: dispatch,
    runtimeOptionsProvider: hostContext.runtimeOptionsProvider,
    readFileBytes: hostContext.readFileBytes,
    onActionResultDelivered: hostContext.onActionResultDelivered,
  );
  return _ComposeProgramExecutor(
    node: store.root,
    onAction: dispatch,
    onTextInput: (id, text) async => null,
    webViewHostContext: resolvedHost,
    splitMarkdownContent: (_) async => [],
  );
}

class _ComposeProgramExecutor extends StatefulWidget {
  /// Creates the compose dsl renderer instance.
  const _ComposeProgramExecutor({
    super.key,
    required this.node,
    required this.onAction,
    required this.onTextInput,
    required this.webViewHostContext,
    required this.splitMarkdownContent,
    this.nodePath = 'root',
    this.modifierScope = _ComposeDslModifierScope.normal,
    this.embedDialog = false,
    this.dispatchOnLoad = true,
    this.onLoadHandledByHost = false,
  });

  final _ComposeProgramNode node;

  /// Resolves function for the Compose DSL renderer.
  final Future<Object?> Function(String actionId, [Object? payload]) onAction;

  /// Dispatches text edits through the page-wide ordered text queue.
  final Future<Object?> Function(String actionId, String text) onTextInput;
  final ComposeDslWebViewHostContext webViewHostContext;
  final MarkdownContentSplitter splitMarkdownContent;
  final String nodePath;
  final _ComposeDslModifierScope modifierScope;
  final bool embedDialog;
  final bool dispatchOnLoad;
  final bool onLoadHandledByHost;

  /// Creates the state that subscribes to nodes observed by this renderer.
  @override
  State<_ComposeProgramExecutor> createState() => _ComposeProgramExecutorState();
}

class _ComposeProgramExecutorState extends State<_ComposeProgramExecutor> {
  final Set<_ComposeProgramInstance> _dependencies = {};
  static int _mountSequence = 0;
  final int _mountId = ++_mountSequence;
  final Map<String, String> _eventTargets = {};
  Map<String, Object?>? _boundProps;
  Map<String, Object?>? _sourceProps;
  bool _hasDispatchedOnLoad = false;

  /// Resolves mount-owned event slots at dispatch time so queued callbacks observe current parameters.
  Future<Object?> _dispatchEvent(String id, [Object? payload]) {
    if (!id.startsWith('@program:')) return widget.onAction(id, payload);
    if (!mounted || node.isDisposed) return Future<Object?>.value();
    props;
    final target = _eventTargets[id];
    if (target == null) throw StateError('Unknown Compose event slot: $id');
    return widget.onAction(target, payload);
  }

  /// Binds callback addresses to this Flutter occurrence while sharing all callback-free values.
  Object? _bindEvents(Object? value, String path) {
    if (value is Map && value.containsKey('__actionId')) {
      final id = '@program:$_mountId:$path';
      _eventTargets[id] = value['__actionId'] as String;
      return <String, Object?>{'__actionId': id};
    }
    if (value is Map<String, Object?>) {
      Map<String, Object?>? changed;
      for (final entry in value.entries) {
        final next = _bindEvents(entry.value, '$path/${entry.key.length}:${entry.key}');
        if (!identical(next, entry.value)) { changed ??= Map.of(value); changed[entry.key] = next; }
      }
      return changed ?? value;
    }
    if (value is List) {
      List<Object?>? changed;
      for (var i = 0; i < value.length; i++) {
        final next = _bindEvents(value[i], '$path/$i');
        if (!identical(next, value[i])) { changed ??= List<Object?>.of(value); changed[i] = next; }
      }
      return changed ?? value;
    }
    return value;
  }

  /// Reads resolved instruction properties and preserves occurrence-local event addresses.
  Map<String, Object?> get props {
    final source = node.props;
    if (!identical(source, _sourceProps)) { _eventTargets.clear(); _boundProps = _bindEvents(source, '') as Map<String, Object?>; _sourceProps = source; }
    return _boundProps!;
  }
  bool _onLoadScheduled = false;

  /// Reads the current node handle owned by this widget.
  _ComposeProgramNode get node => widget.node;

  /// Reads the current action dispatcher without subscribing to a parent renderer.
  Future<Object?> Function(String actionId, [Object? payload]) get onAction =>
      _dispatchEvent;

  /// Reads the current ordered text dispatcher.
  Future<Object?> Function(String actionId, String text) get onTextInput =>
      (id, text) {
        props;
        final target = id.startsWith('@program:') ? _eventTargets[id] : id;
        if (!mounted || node.isDisposed || target == null) throw StateError('Compose text event mount has been disposed');
        return widget.onTextInput(target, text);
      };

  /// Reads the existing WebView execution context.
  ComposeDslWebViewHostContext get webViewHostContext =>
      widget.webViewHostContext;

  /// Reads the existing Markdown splitter.
  MarkdownContentSplitter get splitMarkdownContent =>
      widget.splitMarkdownContent;

  /// Uses the retained identity for node-owned resources and positional snapshot paths otherwise.
  String get nodePath => 'program-mount:$_mountId';

  /// Reads the parent's modifier scope.
  _ComposeDslModifierScope get modifierScope => widget.modifierScope;

  /// Reads the owning dialog's embedding policy.
  bool get embedDialog => widget.embedDialog;

  /// Requests a rebuild only after an observed node changes.
  void _invalidate() {
    if (mounted) setState(() {});
  }

  /// Captures reads from immediate and deferred node builders.
  Widget _trackBuild(Widget Function() build) {
    // Dialog routes and deferred layout builders can run after a retained commit
    // removes their source. Do not rebuild or resubscribe a retired node.
    if (!mounted || widget.node.isDisposed) return const SizedBox.shrink();
    final reads = <_ComposeProgramInstance>{};
    try {
      return _ComposeProgramReadScope.track(reads, build);
    } finally {
      for (final dependency in reads) {
        if (!dependency.isDisposed && _dependencies.add(dependency)) {
          dependency.addListener(_invalidate);
        }
      }
    }
  }

  /// Removes subscriptions from the preceding build before recording current reads.
  void _clearDependencies() {
    for (final dependency in _dependencies) {
      dependency.removeListener(_invalidate);
    }
    _dependencies.clear();
  }

  /// Releases every observed-node subscription on unmount.
  @override
  void dispose() {
    _clearDependencies();
    super.dispose();
  }

  /// Builds the widget for the current DSL state.
  @override
  Widget build(BuildContext context) {
    _clearDependencies();
    return _trackBuild(() {
      final content = _withModifier(
        context,
        _executeInstruction(context),
        props,
        onAction,
        nodeType: node.type,
        modifierScope: modifierScope,
      );
      _dispatchOnLoad();
      return content;
    });
  }

  /// Loads each mounted child once; the screen host owns the root's load and timing.
  void _dispatchOnLoad() {
    if (!widget.dispatchOnLoad ||
        widget.onLoadHandledByHost ||
        _hasDispatchedOnLoad ||
        _onLoadScheduled ||
        _actionId(props['onLoad']) == null) {
      return;
    }
    _onLoadScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _onLoadScheduled = false;
      if (!mounted ||
          node.isDisposed ||
          !widget.dispatchOnLoad ||
          widget.onLoadHandledByHost ||
          _hasDispatchedOnLoad) {
        return;
      }
      final actionId = _actionId(props['onLoad']);
      if (actionId == null) return;
      _hasDispatchedOnLoad = true;
      unawaited(onAction(actionId));
    });
  }

  /// Builds build node for the Compose DSL renderer.
  Widget _executeInstruction(BuildContext context) {
    final type = node.type;
    switch (type) {
      case 'GradientRule':
        return DecoratedBox(
          decoration: BoxDecoration(
            gradient: LinearGradient(
              colors: [
                _color(context, props['startColor']) ?? Colors.transparent,
                _color(context, props['endColor']) ?? Colors.transparent,
              ],
            ),
          ),
          child: const SizedBox.expand(),
        );
      case 'ActivityDots':
        return _ComposeActivityDots(props: props);
      case 'Draggable':
        return _ComposeDraggable(
          props: props,
          onAction: onAction,
          feedback: _optionalSlot('feedback'),
          childWhenDragging: _optionalSlot('childWhenDragging'),
          child: _decoratorChild(),
        );
      case 'DragTarget':
        return _ComposeDragTarget(
          props: props,
          onAction: onAction,
          child: _decoratorChild(),
        );
      case 'SwipeActions':
        final startId = _actionId(props['onStartAction']);
        final endId = _actionId(props['onEndAction']);
        return SwipeActions(
          actionThreshold: (_number(props['actionThreshold']) ?? 0.5)
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
        final items = (props['items'] as List<Object?>? ?? [])
            .map(_stringMap)
            .toList();
        return PopupMenuButton<int>(
          tooltip: _string(props['tooltip']),
          enabled: props['enabled'] != false,
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
            minWidth: _number(props['menuMinWidth']) ?? 118,
            maxWidth: _number(props['menuMaxWidth']) ?? 150,
          ),
          menuPadding: const EdgeInsets.all(4),
          onOpened: () => _ComposeHoverScope.menuOpenChanged(context, true),
          onCanceled: () => _ComposeHoverScope.menuOpenChanged(context, false),
          onSelected: (index) {
            _ComposeHoverScope.menuOpenChanged(context, false);
            final id = _actionId(props['onSelected']);
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
          color: _color(context, props['hoverBackground']),
          radius: _borderRadius(props['shape']) ?? BorderRadius.zero,
          child: _decoratorChild(),
        );
      case 'HoverOnly':
        return _ComposeHoverOnly(
          alwaysVisible: _bool(props['alwaysVisible']),
          child: _decoratorChild(),
        );
      case 'Column':
        return _ComposeFlex(
          direction: Axis.vertical,
          mainAxisSize: MainAxisSize.min,
          nodes: _slotNodes('content', useChildren: true),
          crossAxisAlignment: _crossAxis(props['horizontalAlignment']),
          mainAxisAlignment: _mainAxis(props['verticalArrangement']),
          spacing: _flexSpacing(
            props['verticalArrangement'],
            props['spacing'],
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
          builder: (context, constraints) => _trackBuild(() {
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
              crossAxisAlignment: _crossAxis(props['verticalAlignment']),
              mainAxisAlignment: _mainAxis(props['horizontalArrangement']),
              spacing: _flexSpacing(
                props['horizontalArrangement'],
                props['spacing'],
              ),
              children: children,
            );
          }),
        );
      case 'FlowRow':
        return Wrap(
          spacing: _flowSpacing(
            props['horizontalArrangement'],
            props['spacing'],
          ),
          runSpacing: _flowSpacing(
            props['verticalArrangement'],
            props['runSpacing'] ?? props['spacing'],
          ),
          alignment: _wrapAlignment(props['horizontalArrangement']),
          runAlignment: _wrapAlignment(props['verticalArrangement']),
          crossAxisAlignment: _wrapCrossAxis(
            props['itemVerticalAlignment'],
          ),
          children: _slotChildren('content', useChildren: true),
        );
      case 'LazyRow':
        return _lazyList(Axis.horizontal);
      case 'Box':
        return _ComposeBox(
          alignment: _alignment(props['contentAlignment']),
          nodes: _slotNodes('content', useChildren: true),
          children: _slotChildren(
            'content',
            useChildren: true,
            modifierScope: _ComposeDslModifierScope.box,
          ),
        );
      case 'Spacer':
        return SizedBox(
          width: _number(props['width']),
          height: _number(props['height']),
        );
      case 'Markdown':
        final colorScheme = Theme.of(context).colorScheme;
        return StreamMarkdownRenderer(
          content: _string(props['text']),
          isStreaming: false,
          textColor:
              _color(context, props['color']) ?? colorScheme.onSurface,
          backgroundColor:
              _color(context, props['backgroundColor']) ??
              Colors.transparent,
          splitMarkdownContent: splitMarkdownContent,
        );
      case 'Text':
      case 'BasicText':
        return Text(
          _string(props['text']),
          textAlign: switch (_string(props['textAlign'])) {
            'end' => TextAlign.end,
            'center' => TextAlign.center,
            'right' => TextAlign.right,
            'left' => TextAlign.left,
            'justify' => TextAlign.justify,
            _ => TextAlign.start,
          },
          maxLines: _int(props['maxLines']),
          overflow: _string(props['overflow']) == 'ellipsis'
              ? TextOverflow.ellipsis
              : null,
          softWrap: props['softWrap'] as bool?,
          style: _textStyle(context, props),
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
      case 'HorizontalPager':
        return _ComposePager(
          currentPage: _int(props['currentPage']) ?? 0,
          onPageChanged: (page) =>
              _invokeAction(props['onPageChanged'], page),
          children: _slotChildren('content', useChildren: true),
        );
      case 'InputDecorator':
        return InputDecorator(
          decoration: InputDecoration(
            labelText: _string(props['label']),
            enabled: _enabled(),
          ),
          child: _hasSlot('content')
              ? _slotInline('content')
              : _decoratorChild(),
        );
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
          _iconData(_string(props['name'])),
          size: _number(props['size']),
          color: _color(context, props['tint']),
        );
      case 'LinearProgressIndicator':
        final progress = _number(props['progress']);
        return LinearProgressIndicator(
          value: progress?.clamp(0, 1).toDouble(),
          color: _color(context, props['color']),
          backgroundColor: _color(context, props['trackColor']),
        );
      case 'LoadingIndicator':
        return M3LoadingIndicator(
          size: _number(props['size']) ?? 36,
          color: _color(context, props['color']),
        );
      case 'CircularProgressIndicator':
        return CircularProgressIndicator(
          value: _number(props['progress'])?.clamp(0, 1).toDouble(),
          strokeWidth: _number(props['strokeWidth']) ?? 4,
          color: _color(context, props['color']),
          backgroundColor: _color(context, props['trackColor']),
        );
      case 'Divider':
      case 'HorizontalDivider':
        return Divider(
          thickness: _number(props['thickness']),
          color: _color(context, props['color']),
        );
      case 'VerticalDivider':
        return VerticalDivider(
          thickness: _number(props['thickness']),
          color: _color(context, props['color']),
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
            alignment: _alignment(props['contentAlignment']),
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
          style: _textStyle(context, props) ?? const TextStyle(),
          child: _slotOrChildren('content'),
        );
      case 'SnackbarHost':
        return _slotOrChildren('content');
      case 'AiChat':
        return AIChatEmbed(
          clients: webViewHostContext.clients,
          chatId: props['chatId'] as String?,
          showInput: props['showInput'] as bool? ?? true,
        );
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
            props: props,
            nodePath: nodePath,
            webViewHostContext: webViewHostContext,
          ),
          props: props,
          onAction: onAction,
          hostContext: webViewHostContext.withActionDispatcher(onAction),
        );
      case 'Image':
      case 'AsyncImage':
        return _image(context);
      default:
        return _childrenColumn();
    }
  }
}
