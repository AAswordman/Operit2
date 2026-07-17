// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/concurrency/AppWorkers.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/AdaptiveSidePanel.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../common/icons/MaterialIconNameResolver.dart';
import '../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../chat/screens/AIChatScreen.dart';
import '../utils/PackageDisplayUtils.dart';
import 'ToolPkgComposeDslWebView.dart';

class ToolPkgUiLauncherScreen extends StatefulWidget {
  const ToolPkgUiLauncherScreen({
    super.key,
    required this.clients,
    required this.plugin,
    this.initialRouteId,
    this.showLauncherChrome = true,
  });

  final GeneratedCoreProxyClients clients;
  final core_proxy.ToolPkgContainerRuntime plugin;
  final String? initialRouteId;
  final bool showLauncherChrome;

  @override
  State<ToolPkgUiLauncherScreen> createState() =>
      _ToolPkgUiLauncherScreenState();
}

class _ToolPkgUiLauncherScreenState extends State<ToolPkgUiLauncherScreen> {
  static int _nextExecutionOwnerId = 0;

  late final int _executionOwnerId = _nextExecutionOwnerId++;
  late String _selectedRouteId = _initialRouteId();
  _ComposeDslRenderResult? _renderResult;
  String? _scriptScreenPath;
  ({String contextKey, String containerPackageName})? _activeExecutionContext;
  bool _loading = true;
  bool _loadedInitialRoute = false;
  int _routeLoadGeneration = 0;
  String _currentLanguageTag = 'en';
  String? _error;

  GeneratedApplicationPackageManagerCoreProxy get _packageManager =>
      widget.clients.application.packageManager();

  @override
  void initState() {
    super.initState();
    ComposeDslWebViewHostRegistry.ensureHostInteractionRegistered();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final languageTag = _resolveCurrentLanguage();
    final shouldLoadRoute =
        !_loadedInitialRoute || languageTag != _currentLanguageTag;
    _currentLanguageTag = languageTag;
    if (shouldLoadRoute) {
      _loadedInitialRoute = true;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) {
          _loadRoute();
        }
      });
    }
  }

  String _initialRouteId() {
    final requested = widget.initialRouteId?.trim();
    if (requested != null && requested.isNotEmpty) {
      final matched = widget.plugin.uiRoutes.any(
        (route) => route.routeId == requested || route.id == requested,
      );
      final moduleMatched = widget.plugin.uiModules.any(
        (module) => module.id == requested,
      );
      if (matched || moduleMatched) {
        return requested;
      }
    }
    if (widget.plugin.uiRoutes.isNotEmpty) {
      return widget.plugin.uiRoutes.first.routeId;
    }
    if (widget.plugin.uiModules.isNotEmpty) {
      return widget.plugin.uiModules.first.id;
    }
    return '';
  }

  Future<void> _loadRoute() async {
    if (!mounted) {
      return;
    }
    final routeLoadGeneration = ++_routeLoadGeneration;
    final uiModuleId = _selectedUiModuleId();
    final routeInstanceId = _selectedRouteInstanceId();
    final executionContextKey = _executionContextKey(
      uiModuleId: uiModuleId,
      routeInstanceId: routeInstanceId,
    );
    final executionContext = (
      contextKey: executionContextKey,
      containerPackageName: widget.plugin.packageName,
    );
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final previousExecutionContext = _activeExecutionContext;
      if (previousExecutionContext != null &&
          previousExecutionContext != executionContext) {
        _activeExecutionContext = null;
        await _releaseExecutionContext(previousExecutionContext);
        if (!_isCurrentRouteLoad(routeLoadGeneration)) {
          return;
        }
      }
      _activeExecutionContext = executionContext;
      final script = await _packageManager.getToolPkgComposeDslScript(
        containerPackageName: widget.plugin.packageName,
        uiModuleId: uiModuleId,
      );
      final screenPath = await _packageManager.getToolPkgComposeDslScreenPath(
        containerPackageName: widget.plugin.packageName,
        uiModuleId: uiModuleId,
      );
      if (!_isCurrentRouteLoad(routeLoadGeneration)) {
        return;
      }
      if (script == null || script.trim().isEmpty) {
        throw StateError(
          'compose_dsl script not found: package=${widget.plugin.packageName}, module=$uiModuleId',
        );
      }
      _scriptScreenPath = screenPath;
      final raw = await _packageManager.executeToolPkgComposeDslScript(
        contextKey: executionContextKey,
        containerPackageName: widget.plugin.packageName,
        script: script,
        runtimeOptions: _runtimeOptions(
          uiModuleId: uiModuleId,
          routeInstanceId: routeInstanceId,
          executionContextKey: executionContextKey,
        ),
        envOverrides: const <String, String>{},
      );
      if (!_isCurrentRouteLoad(routeLoadGeneration)) {
        return;
      }
      final result = await AppWorkers.run(
        () => _ComposeDslRenderResult.parse(raw),
        debugName: 'compose-dsl-render-parse',
      );
      if (!_isCurrentRouteLoad(routeLoadGeneration)) {
        return;
      }
      setState(() {
        _renderResult = result;
        _loading = false;
      });
    } catch (error, stackTrace) {
      if (!_isCurrentRouteLoad(routeLoadGeneration)) {
        return;
      }
      _printComposeError('render', error, stackTrace);
      setState(() {
        _error = error.toString();
        _loading = false;
      });
    }
  }

  bool _isCurrentRouteLoad(int routeLoadGeneration) {
    return mounted && routeLoadGeneration == _routeLoadGeneration;
  }

  /// Releases one page-owned ToolPkg execution context.
  Future<void> _releaseExecutionContext(
    ({String contextKey, String containerPackageName}) executionContext,
  ) async {
    await _packageManager.releaseToolPkgExecutionEngine(
      contextKey: executionContext.contextKey,
      containerPackageName: executionContext.containerPackageName,
    );
  }

  /// Releases the active ToolPkg context when this page leaves the widget tree.
  @override
  void dispose() {
    _routeLoadGeneration += 1;
    final executionContext = _activeExecutionContext;
    _activeExecutionContext = null;
    if (executionContext != null) {
      unawaited(
        _releaseExecutionContext(executionContext).catchError((
          Object error,
          StackTrace stackTrace,
        ) {
          _printComposeError('release', error, stackTrace);
        }),
      );
    }
    super.dispose();
  }

  Future<Object?> _dispatchAction(String actionId, [Object? payload]) {
    return _dispatchActionCore(
      actionId,
      payload,
      reportAndSuppressErrors: true,
    );
  }

  Future<Object?> _dispatchWebViewAction(String actionId, [Object? payload]) {
    return _dispatchActionCore(
      actionId,
      payload,
      reportAndSuppressErrors: false,
    );
  }

  Future<Object?> _dispatchActionCore(
    String actionId,
    Object? payload, {
    required bool reportAndSuppressErrors,
  }) async {
    final uiModuleId = _selectedUiModuleId();
    final routeInstanceId = _selectedRouteInstanceId();
    final executionContextKey = _executionContextKey(
      uiModuleId: uiModuleId,
      routeInstanceId: routeInstanceId,
    );
    Object? latestActionResult;
    try {
      final events = await _packageManager
          .dispatchToolPkgComposeDslActionEvents(
            contextKey: executionContextKey,
            containerPackageName: widget.plugin.packageName,
            actionId: actionId,
            payload: payload,
            runtimeOptions: _runtimeOptions(
              uiModuleId: uiModuleId,
              routeInstanceId: routeInstanceId,
              executionContextKey: executionContextKey,
            ),
            envOverrides: const <String, String>{},
          );
      for (final event in events) {
        if (!mounted) {
          return latestActionResult;
        }
        final parsedEvent = await AppWorkers.run(
          () => _ParsedComposeDslActionEvent.parse(event),
          debugName: 'compose-dsl-action-event-parse',
        );
        final phase = parsedEvent.phase;
        if (phase == 'intermediate' || phase == 'final') {
          latestActionResult = parsedEvent.actionResult;
          final result = parsedEvent.renderResult;
          if (result == null) {
            continue;
          }
          if (!mounted) {
            return latestActionResult;
          }
          setState(() {
            _renderResult = result;
            _error = null;
          });
        } else if (phase == 'error') {
          final errorText = parsedEvent.errorText;
          if (errorText == null) {
            throw StateError('compose_dsl action error event missing error');
          }
          if (!mounted) {
            return latestActionResult;
          }
          setState(() {
            _error = errorText;
          });
          throw StateError(errorText);
        } else if (phase == 'complete') {
          break;
        }
      }
      return latestActionResult;
    } catch (error, stackTrace) {
      if (!mounted) {
        return latestActionResult;
      }
      _printComposeError('action:$actionId', error, stackTrace);
      setState(() {
        _error = error.toString();
      });
      if (!reportAndSuppressErrors) {
        rethrow;
      }
      return null;
    }
  }

  Map<String, Object?> _runtimeOptions({
    required String uiModuleId,
    required String routeInstanceId,
    required String executionContextKey,
  }) {
    return <String, Object?>{
      'packageName': widget.plugin.packageName,
      'containerPackageName': widget.plugin.packageName,
      'toolPkgId': widget.plugin.packageName,
      '__operit_ui_package_name': widget.plugin.packageName,
      '__operit_ui_toolpkg_id': widget.plugin.packageName,
      'uiModuleId': uiModuleId,
      '__operit_ui_module_id': uiModuleId,
      '__operit_toolpkg_runtime_kind': 'ui',
      'state': _renderResult?.state ?? const <String, Object?>{},
      'memo': _renderResult?.memo ?? const <String, Object?>{},
      'routeInstanceId': routeInstanceId,
      '__operit_route_instance_id': routeInstanceId,
      'executionContextKey': executionContextKey,
      '__operit_compose_execution_context_key': executionContextKey,
      '__operit_package_lang': _currentLanguage(),
      '__operit_script_screen': _scriptScreenPath ?? '',
      'moduleSpec': _moduleSpec(uiModuleId),
    };
  }

  String _selectedUiModuleId() {
    for (final route in widget.plugin.uiRoutes) {
      if (route.routeId == _selectedRouteId || route.id == _selectedRouteId) {
        return route.id;
      }
    }
    for (final module in widget.plugin.uiModules) {
      if (module.id == _selectedRouteId) {
        return module.id;
      }
    }
    return _selectedRouteId;
  }

  String _selectedRouteInstanceId() {
    final uiModuleId = _selectedUiModuleId();
    for (final route in widget.plugin.uiRoutes) {
      if (route.routeId == _selectedRouteId || route.id == _selectedRouteId) {
        return 'screen:${widget.plugin.packageName}:$uiModuleId';
      }
    }
    return 'legacy:${widget.plugin.packageName}:$uiModuleId';
  }

  String _executionContextKey({
    required String uiModuleId,
    required String routeInstanceId,
  }) {
    final container = widget.plugin.packageName.trim().isEmpty
        ? 'default'
        : widget.plugin.packageName.trim();
    final module = uiModuleId.trim().isEmpty ? 'default' : uiModuleId.trim();
    final route = routeInstanceId.trim().isEmpty
        ? 'default'
        : routeInstanceId.trim();
    return 'toolpkg_compose_dsl:$container:$module:$route:$_executionOwnerId';
  }

  String _currentLanguage() {
    return _currentLanguageTag;
  }

  String _resolveCurrentLanguage() {
    final tag = Localizations.localeOf(context).toLanguageTag().trim();
    return tag.isEmpty ? 'en' : tag;
  }

  void _printComposeError(String phase, Object error, StackTrace stackTrace) {
    debugPrint(
      'ToolPkg compose_dsl $phase error: '
      'package=${widget.plugin.packageName}, '
      'route=$_selectedRouteId, '
      'error=$error',
    );
    debugPrintStack(stackTrace: stackTrace);
  }

  Map<String, Object?> _moduleSpec(String routeId) {
    for (final route in widget.plugin.uiRoutes) {
      if (route.routeId == routeId || route.id == routeId) {
        return <String, Object?>{
          'id': route.id,
          'routeId': route.routeId,
          'runtime': route.runtime,
          'screen': route.screen,
          'title': localizedText(route.title),
          'toolPkgId': widget.plugin.packageName,
          'keepAlive': route.keepAlive,
        };
      }
    }
    for (final module in widget.plugin.uiModules) {
      if (module.id == routeId) {
        return <String, Object?>{
          'id': module.id,
          'routeId': module.id,
          'runtime': module.runtime,
          'screen': module.screen,
          'title': localizedText(module.title),
          'toolPkgId': widget.plugin.packageName,
          'keepAlive': module.keepAlive,
        };
      }
    }
    return <String, Object?>{
      'routeId': routeId,
      'toolPkgId': widget.plugin.packageName,
    };
  }

  @override
  Widget build(BuildContext context) {
    final hasSelectedUi = _hasSelectedUi();
    final uiModuleId = _selectedUiModuleId();
    final routeInstanceId = _selectedRouteInstanceId();
    final executionContextKey = _executionContextKey(
      uiModuleId: uiModuleId,
      routeInstanceId: routeInstanceId,
    );
    final webViewHostContext = ComposeDslWebViewHostContext(
      routeInstanceId: routeInstanceId,
      executionContextKey: executionContextKey,
      dispatchAction: _dispatchWebViewAction,
      runtimeOptionsProvider: () => _runtimeOptions(
        uiModuleId: uiModuleId,
        routeInstanceId: routeInstanceId,
        executionContextKey: executionContextKey,
      ),
    );
    final content = hasSelectedUi
        ? _ComposeHost(
            key: ValueKey(_selectedRouteId),
            loading: _loading,
            error: _error,
            renderResult: _renderResult,
            onAction: _dispatchAction,
            webViewHostContext: webViewHostContext,
          )
        : const _NoUiView();
    if (!widget.showLauncherChrome) {
      return SizedBox.expand(child: content);
    }
    return Scaffold(
      appBar: AppBar(
        title: Text(toolPkgContainerDisplayName(widget.plugin)),
        actions: <Widget>[
          IconButton(
            tooltip: '刷新',
            onPressed: _loadRoute,
            icon: const Icon(Icons.refresh),
          ),
          IconButton(
            tooltip: '关闭',
            onPressed: () => Navigator.of(context).maybePop(),
            icon: const Icon(Icons.close),
          ),
        ],
      ),
      body: SafeArea(
        child: Row(
          children: <Widget>[
            SizedBox(width: 300, child: _navigationPane()),
            Expanded(child: content),
          ],
        ),
      ),
    );
  }

  Widget _navigationPane() {
    final colorScheme = Theme.of(context).colorScheme;
    final navigationEntries = _standaloneNavigationEntries();
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border(right: BorderSide(color: colorScheme.outlineVariant)),
      ),
      child: ListView(
        padding: const EdgeInsets.fromLTRB(12, 12, 12, 24),
        children: <Widget>[
          Text(
            '界面',
            style: Theme.of(
              context,
            ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700),
          ),
          const SizedBox(height: 8),
          for (final route in widget.plugin.uiRoutes)
            _RouteTile(
              route: route,
              selected: route.routeId == _selectedRouteId,
              onTap: () {
                setState(() {
                  _selectedRouteId = route.routeId;
                  _renderResult = null;
                });
                _loadRoute();
              },
            ),
          for (final module in _standaloneUiModules())
            _UiModuleTile(
              module: module,
              selected: module.id == _selectedRouteId,
              onTap: () {
                setState(() {
                  _selectedRouteId = module.id;
                  _renderResult = null;
                });
                _loadRoute();
              },
            ),
          if (navigationEntries.isNotEmpty) ...<Widget>[
            const SizedBox(height: 18),
            Text(
              '入口',
              style: Theme.of(
                context,
              ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 8),
            for (final entry in navigationEntries)
              _NavigationEntryTile(
                entry: entry,
                onTap: entry.routeId.trim().isEmpty
                    ? null
                    : () {
                        setState(() {
                          _selectedRouteId = entry.routeId;
                          _renderResult = null;
                        });
                        _loadRoute();
                      },
              ),
          ],
        ],
      ),
    );
  }

  bool _hasSelectedUi() {
    for (final route in widget.plugin.uiRoutes) {
      if (route.routeId == _selectedRouteId || route.id == _selectedRouteId) {
        return true;
      }
    }
    for (final module in widget.plugin.uiModules) {
      if (module.id == _selectedRouteId) {
        return true;
      }
    }
    return false;
  }

  List<core_proxy.ToolPkgUiModuleRuntime> _standaloneUiModules() {
    final routedModuleIds = <String>{
      for (final route in widget.plugin.uiRoutes) route.id.trim(),
    };
    return widget.plugin.uiModules
        .where((module) => !routedModuleIds.contains(module.id.trim()))
        .toList(growable: false);
  }

  List<core_proxy.ToolPkgNavigationEntryRuntime>
  _standaloneNavigationEntries() {
    final uiRouteIds = <String>{
      for (final route in widget.plugin.uiRoutes) route.id.trim(),
      for (final route in widget.plugin.uiRoutes) route.routeId.trim(),
      for (final module in widget.plugin.uiModules) module.id.trim(),
    }..remove('');
    return widget.plugin.navigationEntries
        .where((entry) {
          final routeId = entry.routeId.trim();
          return routeId.isEmpty || !uiRouteIds.contains(routeId);
        })
        .toList(growable: false);
  }
}

class _ComposeHost extends StatefulWidget {
  const _ComposeHost({
    super.key,
    required this.loading,
    required this.error,
    required this.renderResult,
    required this.onAction,
    required this.webViewHostContext,
  });

  final bool loading;
  final String? error;
  final _ComposeDslRenderResult? renderResult;
  final Future<Object?> Function(String actionId, [Object? payload]) onAction;
  final ComposeDslWebViewHostContext webViewHostContext;

  @override
  State<_ComposeHost> createState() => _ComposeHostState();
}

class _ComposeHostState extends State<_ComposeHost> {
  bool _hasDispatchedInitialOnLoad = false;

  @override
  void didUpdateWidget(covariant _ComposeHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.renderResult?.tree != widget.renderResult?.tree) {
      _dispatchRootOnLoad();
    }
  }

  @override
  Widget build(BuildContext context) {
    final tree = widget.renderResult?.tree;
    if (!widget.loading && widget.error == null && tree != null) {
      _dispatchRootOnLoad();
    }
    if (widget.loading) {
      return const M3LoadingPane();
    }
    if (widget.error != null) {
      return Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 620),
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Text(
              widget.error!,
              textAlign: TextAlign.center,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ),
        ),
      );
    }
    if (tree == null) {
      return const _NoUiView();
    }
    return _ComposeDslRenderer(
      node: tree,
      onAction: widget.onAction,
      webViewHostContext: widget.webViewHostContext,
    );
  }

  void _dispatchRootOnLoad() {
    if (_hasDispatchedInitialOnLoad || widget.loading || widget.error != null) {
      return;
    }
    final rootNode = widget.renderResult?.tree;
    if (rootNode == null) {
      return;
    }
    final rootOnLoadActionId = _actionId(rootNode.props['onLoad']);
    if (rootOnLoadActionId == null) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      if (!mounted || _hasDispatchedInitialOnLoad) {
        return;
      }
      final currentRootNode = widget.renderResult?.tree;
      if (currentRootNode == null) {
        return;
      }
      final currentRootOnLoadActionId = _actionId(
        currentRootNode.props['onLoad'],
      );
      if (currentRootOnLoadActionId != rootOnLoadActionId) {
        return;
      }
      _hasDispatchedInitialOnLoad = true;
      await widget.onAction(rootOnLoadActionId, null);
    });
  }
}

class _ComposeDslRenderer extends StatelessWidget {
  const _ComposeDslRenderer({
    required this.node,
    required this.onAction,
    required this.webViewHostContext,
    this.nodePath = 'root',
    this.modifierScope = _ComposeDslModifierScope.normal,
  });

  final _ComposeDslNode node;
  final Future<Object?> Function(String actionId, [Object? payload]) onAction;
  final ComposeDslWebViewHostContext webViewHostContext;
  final String nodePath;
  final _ComposeDslModifierScope modifierScope;

  @override
  Widget build(BuildContext context) {
    return _withModifier(
      context,
      _buildNode(context),
      node.props,
      onAction,
      nodeType: node.type,
      modifierScope: modifierScope,
    );
  }

  Widget _buildNode(BuildContext context) {
    final type = node.type;
    switch (type) {
      case 'Column':
        return Column(
          crossAxisAlignment: _crossAxis(node.props['horizontalAlignment']),
          mainAxisAlignment: _mainAxis(node.props['verticalArrangement']),
          children: _slotChildren(
            'content',
            useChildren: true,
            modifierScope: _ComposeDslModifierScope.column,
          ),
        );
      case 'LazyColumn':
        return SingleChildScrollView(
          child: Column(
            crossAxisAlignment: _crossAxis(node.props['horizontalAlignment']),
            mainAxisAlignment: _mainAxis(node.props['verticalArrangement']),
            children: _slotChildren(
              'content',
              useChildren: true,
              modifierScope: _ComposeDslModifierScope.column,
            ),
          ),
        );
      case 'Row':
        final contentNodes = _slotNodes('content', useChildren: true);
        final children = _buildNodeWidgets(
          contentNodes,
          pathPrefix: '$nodePath:content',
          modifierScope: _ComposeDslModifierScope.row,
        );
        return Row(
          mainAxisSize: _nodesRequireRowFlex(contentNodes)
              ? MainAxisSize.max
              : MainAxisSize.min,
          crossAxisAlignment: _crossAxis(node.props['verticalAlignment']),
          mainAxisAlignment: _mainAxis(node.props['horizontalArrangement']),
          children: children,
        );
      case 'LazyRow':
        return SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: Row(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: _crossAxis(node.props['verticalAlignment']),
            mainAxisAlignment: _mainAxis(node.props['horizontalArrangement']),
            children: _slotChildren('content', useChildren: true),
          ),
        );
      case 'Box':
        return Stack(
          alignment: _alignment(node.props['contentAlignment']),
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

  Widget _button(BuildContext context, String type) {
    final contentChildren = _slotChildren(
      'content',
      useChildren: true,
      modifierScope: _ComposeDslModifierScope.row,
    );
    final child = contentChildren.isNotEmpty
        ? Row(mainAxisSize: MainAxisSize.min, children: contentChildren)
        : Text(
            _string(node.props['text']).isEmpty
                ? type
                : _string(node.props['text']),
          );
    final onPressed = _enabled()
        ? () => _invokeAction(node.props['onClick'])
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

  Widget _card(BuildContext context, String type) {
    final defaultRadius = type == 'Card' ? BorderRadius.circular(12) : null;
    final radius = _borderRadius(node.props['shape']) ?? defaultRadius;
    final borderSide =
        _borderSide(context, node.props['border']) ??
        (type == 'OutlinedCard'
            ? BorderSide(color: Theme.of(context).colorScheme.outlineVariant)
            : null);
    final child = _containerContent(
      context,
      contentColor: _colorWithAlpha(
        context,
        node.props['contentColor'],
        node.props['contentAlpha'],
      ),
      contentPadding: node.props['contentPadding'],
    );
    return Card(
      color: _colorWithAlpha(
        context,
        node.props['containerColor'],
        node.props['containerAlpha'] ?? node.props['alpha'],
      ),
      elevation:
          _number(node.props['elevation']) ?? (type == 'ElevatedCard' ? 3 : 1),
      shape: RoundedRectangleBorder(
        borderRadius: radius ?? BorderRadius.zero,
        side: borderSide ?? BorderSide.none,
      ),
      child: child,
    );
  }

  Widget _surface(BuildContext context) {
    final radius = _borderRadius(node.props['shape']);
    final borderSide = _borderSide(context, node.props['border']);
    final shape = borderSide == null && radius == null
        ? null
        : RoundedRectangleBorder(
            borderRadius: radius ?? BorderRadius.zero,
            side: borderSide ?? BorderSide.none,
          );
    final child = _surfaceContent(
      context,
      contentColor: _color(context, node.props['contentColor']),
      contentPadding: node.props['contentPadding'] ?? node.props['padding'],
    );
    final actionId = _actionId(node.props['onClick']);
    final surfaceChild = actionId == null
        ? child
        : InkWell(
            borderRadius: radius,
            onTap: _enabled()
                ? () => _invokeAction(node.props['onClick'])
                : null,
            child: child,
          );
    return Material(
      color:
          _colorWithAlpha(
            context,
            node.props['color'] ?? node.props['containerColor'],
            node.props['alpha'],
          ) ??
          Colors.transparent,
      elevation:
          _number(node.props['shadowElevation']) ??
          _number(node.props['tonalElevation']) ??
          0,
      shape: shape,
      borderRadius: shape == null ? radius : null,
      child: surfaceChild,
    );
  }

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
    final containerColor = _color(context, node.props['containerColor']);
    final contentColor = _color(context, node.props['contentColor']);
    final disabledContainerColor = _color(
      context,
      node.props['disabledContainerColor'],
    );
    final disabledContentColor = _color(
      context,
      node.props['disabledContentColor'],
    );
    final radius = _borderRadius(node.props['shape']);
    final padding = node.props['contentPadding'];
    final borderSide = _borderSide(context, node.props['border']);
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

  Widget _chip(BuildContext context, String type) {
    final selected =
        _bool(node.props['selected']) || _bool(node.props['checked']);
    final label = _chipLabel(type);
    final onPressed = _enabled()
        ? () => _invokeAction(node.props['onClick'] ?? node.props['onSelected'])
        : null;
    final backgroundColor = _color(context, node.props['containerColor']);
    final selectedColor = _color(context, node.props['selectedContainerColor']);
    final contentColor = _color(context, node.props['contentColor']);
    final disabledColor = _color(context, node.props['disabledContainerColor']);
    final labelStyle = contentColor == null
        ? null
        : TextStyle(color: contentColor);
    final shape = _shapeBorder(
      node.props['shape'],
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
                node.props['onClick'] ?? node.props['onSelected'],
              )
            : null,
      );
    }
    if (type == 'InputChip') {
      final dismissAction = node.props['onDismiss'] ?? node.props['onDelete'];
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
              _string(node.props['label']).isEmpty
                  ? type.replaceAll('Elevated', '')
                  : _string(node.props['label']),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
    );
    final dismissAction = node.props['onDismiss'] ?? node.props['onDelete'];
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

  Widget _chipSlotInline(String name) {
    final slot = node.slots[name] ?? const <_ComposeDslNode>[];
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

  Widget _chipInlineChild(_ComposeDslNode child, String childPath) {
    if (child.type == 'Text') {
      return Text(
        _string(child.props['text']),
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      );
    }
    return _ComposeDslRenderer(
      node: child,
      onAction: onAction,
      webViewHostContext: webViewHostContext,
      nodePath: childPath,
    );
  }

  Widget _iconButton(BuildContext context, String type) {
    final toggle = type.contains('Toggle');
    final checked = _bool(node.props['checked']);
    final icon = _hasSlot('content')
        ? _iconButtonSlot('content')
        : Icon(
            _iconData(
              _string(node.props['icon']).isEmpty
                  ? _string(node.props['name'])
                  : _string(node.props['icon']),
            ),
          );
    final selectedIcon = _hasSlot('selectedIcon')
        ? _iconButtonSlot('selectedIcon')
        : null;
    final onPressed = _enabled()
        ? () => _invokeAction(
            toggle ? node.props['onCheckedChange'] : node.props['onClick'],
            toggle ? !checked : null,
          )
        : null;
    final style = _iconButtonStyle(type);
    switch (type) {
      case 'FilledIconButton':
      case 'FilledIconToggleButton':
        return IconButton.filled(
          onPressed: onPressed,
          isSelected: toggle ? checked : null,
          selectedIcon: selectedIcon,
          style: style,
          icon: icon,
        );
      case 'FilledTonalIconButton':
      case 'FilledTonalIconToggleButton':
        return IconButton.filledTonal(
          onPressed: onPressed,
          isSelected: toggle ? checked : null,
          selectedIcon: selectedIcon,
          style: style,
          icon: icon,
        );
      case 'OutlinedIconButton':
      case 'OutlinedIconToggleButton':
        return IconButton.outlined(
          onPressed: onPressed,
          isSelected: toggle ? checked : null,
          selectedIcon: selectedIcon,
          style: style,
          icon: icon,
        );
      default:
        return IconButton(
          onPressed: onPressed,
          isSelected: toggle ? checked : null,
          selectedIcon: selectedIcon,
          icon: icon,
        );
    }
  }

  ButtonStyle? _iconButtonStyle(String type) {
    if (type == 'IconButton' || type == 'IconToggleButton') {
      return null;
    }
    final shape = _shapeBorder(
      node.props['shape'],
      defaultBorderRadius: type.contains('Outlined')
          ? BorderRadius.circular(12)
          : null,
    );
    return shape == null ? null : IconButton.styleFrom(shape: shape);
  }

  Widget _iconButtonSlot(String name) => Column(
    crossAxisAlignment: CrossAxisAlignment.center,
    mainAxisSize: MainAxisSize.min,
    children: _slotChildren(name),
  );

  Widget _floatingActionButton(BuildContext context, String type) {
    final onPressed = _enabled()
        ? () => _invokeAction(node.props['onClick'])
        : null;
    final backgroundColor = _color(context, node.props['containerColor']);
    final foregroundColor = _color(context, node.props['contentColor']);
    final shape = _shapeBorder(
      node.props['shape'],
      defaultBorderRadius: BorderRadius.zero,
    );
    if (type == 'ExtendedFloatingActionButton') {
      return FloatingActionButton.extended(
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
        : Icon(_iconData(_string(node.props['icon'])));
    if (type == 'SmallFloatingActionButton') {
      return FloatingActionButton.small(
        onPressed: onPressed,
        backgroundColor: backgroundColor,
        foregroundColor: foregroundColor,
        shape: shape,
        child: child,
      );
    }
    if (type == 'LargeFloatingActionButton') {
      return FloatingActionButton.large(
        onPressed: onPressed,
        backgroundColor: backgroundColor,
        foregroundColor: foregroundColor,
        shape: shape,
        child: child,
      );
    }
    return FloatingActionButton(
      onPressed: onPressed,
      backgroundColor: backgroundColor,
      foregroundColor: foregroundColor,
      shape: shape,
      child: child,
    );
  }

  Widget _badge(BuildContext context) {
    final contentColor = _color(context, node.props['contentColor']);
    final label = _tintedSlotInline(
      context,
      'content',
      useChildren: true,
      color: contentColor,
    );
    return Badge(
      backgroundColor: _color(context, node.props['containerColor']),
      textColor: contentColor,
      label: label,
    );
  }

  Widget _badgedBox(BuildContext context) {
    return Badge(
      label: _hasSlot('badge') ? _slotColumn('badge') : null,
      child: _slotOrChildren('content'),
    );
  }

  Widget _snackbar(BuildContext context) {
    final contentColor =
        _color(context, node.props['contentColor']) ??
        Theme.of(context).colorScheme.onInverseSurface;
    final actionContentColor =
        _color(context, node.props['actionContentColor']) ??
        Theme.of(context).colorScheme.inversePrimary;
    final dismissActionContentColor =
        _color(context, node.props['dismissActionContentColor']) ??
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
    final actionOnNewLine = _bool(node.props['actionOnNewLine']);
    final shape =
        _shapeBorder(
              node.props['shape'],
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
          _color(context, node.props['containerColor']) ??
          Theme.of(context).colorScheme.inverseSurface,
      elevation: _number(node.props['elevation']) ?? 6,
      shape: shape,
      borderRadius: shape == null ? BorderRadius.zero : null,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
        child: child,
      ),
    );
  }

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
      elevation: _number(node.props['shadowElevation']) ?? 0,
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

  Widget _navigationBar(BuildContext context) {
    final items = _navigationContentNodes()
        .where(
          (child) =>
              child.type == 'NavigationBarItem' ||
              child.type == 'ShortNavigationBarItem',
        )
        .toList(growable: false);
    final destinations = items
        .map(_navigationDestination)
        .toList(growable: false);
    final selectedIndex = _selectedIndex(items, node.props['selectedIndex']);
    return NavigationBar(
      selectedIndex: selectedIndex,
      backgroundColor: _color(context, node.props['containerColor']),
      elevation: _number(node.props['tonalElevation']),
      destinations: destinations.isEmpty
          ? const <Widget>[
              NavigationDestination(icon: Icon(Icons.widgets), label: ''),
            ]
          : destinations,
      onDestinationSelected: (index) {
        if (index >= 0 && index < items.length) {
          final item = items[index];
          if (item.props['enabled'] != false) {
            _invokeAction(item.props['onClick']);
          }
        }
      },
    );
  }

  NavigationDestination _navigationDestination(_ComposeDslNode item) {
    return NavigationDestination(
      icon: _navigationItemIcon(item, selected: false),
      selectedIcon: _hasSlotFrom(item, 'selectedIcon')
          ? _navigationItemIcon(item, selected: true)
          : null,
      label: _plainTextFrom(item, 'label') ?? _string(item.props['label']),
    );
  }

  Widget _navigationRail(BuildContext context) {
    final items = _navigationContentNodes()
        .where(
          (child) =>
              child.type == 'NavigationRailItem' ||
              child.type == 'WideNavigationRailItem',
        )
        .toList(growable: false);
    final header = _slotChildren('header');
    return SizedBox(
      height: _navigationRailHeight(items, header),
      child: NavigationRail(
        selectedIndex: _selectedIndex(items, node.props['selectedIndex']),
        backgroundColor: _color(context, node.props['containerColor']),
        leading: header.isEmpty
            ? null
            : Column(mainAxisSize: MainAxisSize.min, children: header),
        labelType: items.any((item) => _bool(item.props['alwaysShowLabel']))
            ? NavigationRailLabelType.all
            : NavigationRailLabelType.selected,
        destinations: items.isEmpty
            ? const <NavigationRailDestination>[
                NavigationRailDestination(
                  icon: Icon(Icons.widgets),
                  label: Text(''),
                ),
              ]
            : items
                  .map(
                    (item) => NavigationRailDestination(
                      icon: _navigationItemIcon(item, selected: false),
                      selectedIcon: _hasSlotFrom(item, 'selectedIcon')
                          ? _navigationItemIcon(item, selected: true)
                          : null,
                      label: Text(
                        _plainTextFrom(item, 'label') ??
                            _string(item.props['label']),
                      ),
                    ),
                  )
                  .toList(growable: false),
        onDestinationSelected: (index) {
          if (index >= 0 && index < items.length) {
            final item = items[index];
            if (item.props['enabled'] != false) {
              _invokeAction(item.props['onClick']);
            }
          }
        },
      ),
    );
  }

  Widget _navigationItemTile() {
    final selected = _bool(node.props['selected']);
    final leading = _navigationItemIcon(
      node,
      selected: selected,
      includeBadge: false,
    );
    return ListTile(
      selected: selected,
      leading: leading,
      title: _slotOrText('label'),
      trailing: _hasSlot('badge') ? _slotInline('badge') : null,
      enabled: _enabled(),
      shape: RoundedRectangleBorder(
        borderRadius: _borderRadius(node.props['shape']) ?? BorderRadius.zero,
      ),
      onTap: _enabled() ? () => _invokeAction(node.props['onClick']) : null,
    );
  }

  Widget _navigationItemIcon(
    _ComposeDslNode item, {
    required bool selected,
    bool includeBadge = true,
  }) {
    final icon =
        (selected ? _slotFrom(item, 'selectedIcon') : null) ??
        _slotFrom(item, 'icon') ??
        const Icon(Icons.circle_outlined);
    if (!includeBadge || !_hasSlotFrom(item, 'badge')) {
      return icon;
    }
    return Badge(label: _slotFrom(item, 'badge'), child: icon);
  }

  List<_ComposeDslNode> _navigationContentNodes() {
    final content = node.slots['content'];
    return content != null && content.isNotEmpty ? content : node.children;
  }

  double _navigationRailHeight(
    List<_ComposeDslNode> items,
    List<Widget> header,
  ) {
    final explicit = _number(node.props['height']);
    if (explicit != null && explicit > 0) {
      return explicit;
    }
    return math.max(
      120,
      72.0 * math.max(1, items.length) + 48.0 * header.length,
    );
  }

  Widget _scaffold(BuildContext context) {
    final topBar = _slotChildren('topBar');
    final bottomBar = _slotChildren('bottomBar');
    final snackbarHost = _slotChildren('snackbarHost');
    final contentColor = _color(context, node.props['contentColor']);
    final content = contentColor == null
        ? _slotOrChildren('content')
        : IconTheme.merge(
            data: IconThemeData(color: contentColor),
            child: DefaultTextStyle.merge(
              style: TextStyle(color: contentColor),
              child: _slotOrChildren('content'),
            ),
          );
    return Material(
      color:
          _color(context, node.props['containerColor']) ??
          Theme.of(context).colorScheme.surface,
      child: Stack(
        children: <Widget>[
          Column(
            children: <Widget>[
              ...topBar,
              Expanded(child: content),
              ...bottomBar,
            ],
          ),
          if (_hasSlot('floatingActionButton'))
            Positioned(
              right: 16,
              bottom: bottomBar.isEmpty ? 16 : 88,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.end,
                children: _slotChildren('floatingActionButton'),
              ),
            ),
          if (snackbarHost.isNotEmpty)
            Positioned(
              left: 16,
              right: 16,
              bottom: 16,
              child: _slotColumn('snackbarHost'),
            ),
        ],
      ),
    );
  }

  Widget _drawerSheet(BuildContext context) {
    return Material(
      color:
          _color(context, node.props['drawerContainerColor']) ??
          Theme.of(context).colorScheme.surface,
      elevation: _number(node.props['drawerTonalElevation']) ?? 0,
      child: SafeArea(child: _slotOrChildren('content')),
    );
  }

  Widget _navigationDrawer(BuildContext context) {
    final drawer = _slotChildren('drawerContent');
    final content = _slotOrChildren('content');
    if (drawer.isEmpty) {
      return content;
    }
    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        SizedBox(
          width: _number(node.props['drawerWidth']) ?? 304,
          child: drawer.first,
        ),
        Expanded(child: content),
      ],
    );
  }

  Widget _tabItem(BuildContext context, {required bool leadingIcon}) {
    final selected = _bool(node.props['selected']);
    final selectedContentColor = _color(
      context,
      node.props['selectedContentColor'],
    );
    final unselectedContentColor = _color(
      context,
      node.props['unselectedContentColor'],
    );
    final contentColor = selected
        ? selectedContentColor
        : unselectedContentColor;
    final effectiveColor =
        contentColor ??
        (selected
            ? Theme.of(context).colorScheme.primary
            : Theme.of(context).colorScheme.onSurfaceVariant);
    return InkWell(
      onTap: _enabled() ? () => _invokeAction(node.props['onClick']) : null,
      child: IconTheme.merge(
        data: IconThemeData(color: effectiveColor),
        child: DefaultTextStyle.merge(
          style: TextStyle(color: effectiveColor),
          child: Padding(
            padding: _edgeInsetsFromValue(
              node.props['contentPadding'] ??
                  (leadingIcon
                      ? const <Object?>[16, 12]
                      : const <Object?>[18, 14]),
            ),
            child: leadingIcon ? _leadingIconTabContent() : _tabContent(),
          ),
        ),
      ),
    );
  }

  Widget _tabContent() {
    final content = _slotChildren('content', useChildren: true);
    if (content.isNotEmpty) {
      return Column(mainAxisSize: MainAxisSize.min, children: content);
    }
    final text = _plainSlotText('text') ?? _string(node.props['text']);
    return Text(text);
  }

  Widget _leadingIconTabContent() {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        if (_hasSlot('icon')) ...<Widget>[
          _slotInline('icon'),
          const SizedBox(width: 8),
        ],
        if (_hasSlot('text'))
          _slotInline('text')
        else
          Text(_string(node.props['text'])),
      ],
    );
  }

  Widget _tabRow(BuildContext context, String type) {
    final contentColor = _color(context, node.props['contentColor']);
    final tabWidgets = _slotChildren('tabs');
    Widget tabs = Row(mainAxisSize: MainAxisSize.min, children: tabWidgets);
    if (type.contains('Scrollable')) {
      final edgePadding = _number(node.props['edgePadding']) ?? 0;
      tabs = SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Padding(
          padding: EdgeInsets.symmetric(horizontal: edgePadding),
          child: tabs,
        ),
      );
    }
    final tabBand = Stack(
      alignment: Alignment.bottomCenter,
      children: <Widget>[
        tabs,
        if (_hasSlot('indicator')) _slotInline('indicator'),
      ],
    );
    return Material(
      color:
          _color(context, node.props['containerColor']) ??
          Theme.of(context).colorScheme.surface,
      child: DefaultTextStyle.merge(
        style: TextStyle(color: contentColor),
        child: IconTheme.merge(
          data: IconThemeData(color: contentColor),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              tabBand,
              if (_hasSlot('divider'))
                _slotColumn('divider')
              else
                Divider(
                  height: 1,
                  thickness: 1,
                  color: Theme.of(context).colorScheme.outlineVariant,
                ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _pullToRefreshBox(BuildContext context) {
    final refreshing = _bool(node.props['isRefreshing']);
    final content = RefreshIndicator(
      onRefresh: () async {
        final actionId = _actionId(node.props['onRefresh']);
        if (actionId != null) {
          await onAction(actionId);
        }
      },
      child: SingleChildScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        child: _slotOrChildren('content'),
      ),
    );
    if (!refreshing || !_hasSlot('indicator')) {
      return content;
    }
    return Stack(
      alignment: _alignment(node.props['contentAlignment']),
      children: <Widget>[
        content,
        Positioned(left: 0, top: 0, right: 0, child: _slotColumn('indicator')),
      ],
    );
  }

  Widget _dropdownMenu(BuildContext context) {
    final items = _slotChildren('content', useChildren: true);
    final label = _plainSlotText('label') ?? _string(node.props['label']);
    final anchor = _dropdownMenuAnchor(label);
    if (_bool(node.props['expanded'])) {
      final offset = _number(node.props['offset']) ?? 0;
      final selectActionId = _actionId(node.props['onClick']);
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          anchor,
          SizedBox(height: offset),
          Material(
            color:
                _color(context, node.props['containerColor']) ??
                Theme.of(context).colorScheme.surface,
            elevation: _number(node.props['tonalElevation']) ?? 8,
            shape: RoundedRectangleBorder(
              borderRadius:
                  _borderRadius(node.props['shape']) ??
                  BorderRadius.circular(4),
            ),
            child: ConstrainedBox(
              constraints: BoxConstraints(
                minWidth: _number(node.props['menuMinWidth']) ?? 160,
                maxWidth: _number(node.props['menuMaxWidth']) ?? 360,
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  for (var index = 0; index < items.length; index += 1)
                    selectActionId == null
                        ? items[index]
                        : InkWell(
                            onTap: () => onAction(selectActionId, index),
                            child: items[index],
                          ),
                ],
              ),
            ),
          ),
        ],
      );
    }
    return PopupMenuButton<int>(
      enabled: _enabled(),
      tooltip: label.isEmpty ? null : label,
      itemBuilder: (context) => <PopupMenuEntry<int>>[
        for (var index = 0; index < items.length; index += 1)
          PopupMenuItem<int>(value: index, child: items[index]),
      ],
      onSelected: (index) => _invokeAction(node.props['onClick'], index),
      child: anchor,
    );
  }

  Widget _dropdownMenuAnchor(String label) {
    final width = _number(node.props['width']) ?? 200;
    return SizedBox(
      width: width,
      child: InputDecorator(
        decoration: InputDecoration(
          labelText: label.isEmpty ? null : label,
          border: const OutlineInputBorder(),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Expanded(
              child: Text(
                _string(node.props['text'] ?? node.props['selectedText']),
                overflow: TextOverflow.ellipsis,
              ),
            ),
            const Icon(Icons.arrow_drop_down),
          ],
        ),
      ),
    );
  }

  Widget _timePickerDialog(BuildContext context) {
    final dialog = AlertDialog(
      title: _hasSlot('title') ? _slotColumn('title') : null,
      content: _slotOrChildren('content'),
      actions: <Widget>[
        ..._slotChildren('modeToggleButton'),
        ..._slotChildren('dismissButton'),
        ..._slotChildren('confirmButton'),
      ],
      backgroundColor: _color(context, node.props['containerColor']),
      shape: _borderRadius(node.props['shape']) == null
          ? null
          : RoundedRectangleBorder(
              borderRadius: _borderRadius(node.props['shape'])!,
            ),
    );
    final dismissActionId = _actionId(node.props['onDismissRequest']);
    if (dismissActionId == null) {
      return dialog;
    }
    return Focus(
      autofocus: true,
      onKeyEvent: (node, event) {
        if (event is KeyDownEvent &&
            event.logicalKey == LogicalKeyboardKey.escape) {
          onAction(dismissActionId);
          return KeyEventResult.handled;
        }
        return KeyEventResult.ignored;
      },
      child: dialog,
    );
  }

  Widget _verticalDragHandle(BuildContext context) {
    final color =
        _color(context, node.props['color']) ??
        Theme.of(context).colorScheme.outlineVariant;
    return Center(
      child: Container(
        width: _number(node.props['width']) ?? 4,
        height: _number(node.props['height']) ?? 36,
        decoration: BoxDecoration(
          color: color,
          borderRadius: BorderRadius.circular(999),
        ),
      ),
    );
  }

  Widget _switch(BuildContext context) {
    final actionId = _actionId(node.props['onCheckedChange']);
    final checked = _bool(node.props['checked']);
    final enabled = _enabled() && actionId != null;
    final checkedThumbColor = _color(context, node.props['checkedThumbColor']);
    final checkedTrackColor = _color(context, node.props['checkedTrackColor']);
    final uncheckedThumbColor = _color(
      context,
      node.props['uncheckedThumbColor'],
    );
    final uncheckedTrackColor = _color(
      context,
      node.props['uncheckedTrackColor'],
    );
    final control = Switch(
      value: checked,
      onChanged: enabled ? (value) => _invokeAction(actionId, value) : null,
      thumbColor: _stateColor(
        checked: checkedThumbColor,
        unchecked: uncheckedThumbColor,
      ),
      trackColor: _stateColor(
        checked: checkedTrackColor,
        unchecked: uncheckedTrackColor,
      ),
    );
    if (!_hasSlot('thumbContent')) {
      return control;
    }
    return SizedBox(
      width: 58,
      height: 36,
      child: Stack(
        alignment: Alignment.center,
        children: <Widget>[
          control,
          AnimatedAlign(
            duration: const Duration(milliseconds: 120),
            curve: Curves.easeOut,
            alignment: checked ? Alignment.centerRight : Alignment.centerLeft,
            child: IgnorePointer(
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 8),
                child: SizedBox(
                  width: 18,
                  height: 18,
                  child: FittedBox(child: _slotInline('thumbContent')),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _checkbox() {
    final actionId = _actionId(node.props['onCheckedChange']);
    return Checkbox(
      value: _bool(node.props['checked']),
      onChanged: _enabled() && actionId != null
          ? (value) => _invokeAction(actionId, value)
          : null,
    );
  }

  Widget _radioButton() {
    final selected = _bool(node.props['selected']);
    return RadioGroup<bool>(
      groupValue: selected,
      onChanged: (_) => _invokeAction(node.props['onClick']),
      child: Radio<bool>(value: true, enabled: _enabled()),
    );
  }

  Widget _textField(BuildContext context, String type) {
    final labelSlot = _hasSlot('label') ? _slotColumn('label') : null;
    final placeholderSlot = _hasSlot('placeholder')
        ? _slotColumn('placeholder')
        : null;
    final supportingSlot = _hasSlot('supportingText')
        ? _slotColumn('supportingText')
        : null;
    final isSingleLine =
        node.props['singleLine'] == true || node.props['isPassword'] == true;
    return _ComposeTextField(
      identity: _string(node.props['key']).trim().isEmpty
          ? nodePath
          : _string(node.props['key']).trim(),
      value: _string(node.props['value']),
      enabled: _enabled() && _actionId(node.props['onValueChange']) != null,
      readOnly: node.props['readOnly'] == true,
      obscureText: node.props['isPassword'] == true,
      singleLine: isSingleLine,
      minLines: isSingleLine ? null : (_int(node.props['minLines']) ?? 1),
      maxLines: isSingleLine ? 1 : _int(node.props['maxLines']),
      keyboardType: _textInputType(node.props['keyboardType']),
      textInputAction: _textInputAction(
        node.props['imeAction'] ?? node.props['keyboardAction'],
      ),
      isError: node.props['isError'] == true,
      textStyle: _textFieldStyle(context, node.props['style']),
      labelText: labelSlot == null ? _plainSlotText('label') : null,
      label: labelSlot,
      hintText: placeholderSlot == null ? _plainSlotText('placeholder') : null,
      hint: placeholderSlot,
      prefixIcon: _hasSlot('leadingIcon') ? _slotColumn('leadingIcon') : null,
      suffixIcon: _hasSlot('trailingIcon') ? _slotColumn('trailingIcon') : null,
      prefix: _hasSlot('prefix') ? _slotColumn('prefix') : null,
      suffix: _hasSlot('suffix') ? _slotColumn('suffix') : null,
      supportingText: supportingSlot,
      helperText: supportingSlot == null
          ? _plainSlotText('supportingText')
          : null,
      border: type == 'TextField'
          ? const OutlineInputBorder()
          : const OutlineInputBorder(),
      onChanged: (value) => _invokeAction(node.props['onValueChange'], value),
    );
  }

  Widget _canvas(BuildContext context) {
    final canvas = CustomPaint(
      painter: _ComposeCanvasPainter(
        commands: _canvasCommands(node.props['commands']),
        colorScheme: Theme.of(context).colorScheme,
        textTheme: Theme.of(context).textTheme,
      ),
    );
    final actionId = _actionId(node.props['onSizeChanged']);
    if (actionId == null) {
      return canvas;
    }
    return _SizeReportingBox(
      onSizeChanged: (size) {
        onAction(actionId, <String, Object?>{
          'width': size.width,
          'height': size.height,
        });
      },
      child: canvas,
    );
  }

  /// Builds a plugin-controlled responsive trailing panel around the primary slot.
  Widget _adaptiveSidePanel() {
    final openChangedActionId = _requiredActionId(
      node.props['onOpenChanged'],
      'onOpenChanged',
    );
    return AdaptiveSidePanel(
      open: node.props['open'] == true,
      onOpenChanged: (open) {
        unawaited(onAction(openChangedActionId, open));
      },
      breakpoint: _number(node.props['breakpoint']) ?? 600,
      defaultWidth: _number(node.props['defaultWidth']) ?? 360,
      minWidth: _number(node.props['minWidth']) ?? 280,
      minContentWidth: _number(node.props['minContentWidth']) ?? 320,
      panel: _adaptiveSidePanelSlot('side'),
      child: _adaptiveSidePanelSlot('content', useChildren: true),
    );
  }

  /// Renders one adaptive side-panel slot while preserving its available bounds.
  Widget _adaptiveSidePanelSlot(String name, {bool useChildren = false}) {
    final nodes = _slotNodes(name, useChildren: useChildren);
    if (nodes.length != 1) {
      throw StateError('AdaptiveSidePanel.$name requires exactly one child');
    }
    return _ComposeDslRenderer(
      node: nodes.single,
      onAction: onAction,
      webViewHostContext: webViewHostContext,
      nodePath: '$nodePath:$name/0',
    );
  }

  bool _enabled() => node.props['enabled'] != false;

  List<Widget> _children({
    _ComposeDslModifierScope modifierScope = _ComposeDslModifierScope.normal,
  }) => _buildNodeWidgets(
    node.children,
    pathPrefix: nodePath,
    modifierScope: modifierScope,
  );

  Widget _childrenColumn() => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    mainAxisSize: MainAxisSize.min,
    children: _children(modifierScope: _ComposeDslModifierScope.column),
  );

  Widget _slotRow(String name, {bool useChildren = false}) => Row(
    mainAxisSize: MainAxisSize.min,
    children: _slotChildren(
      name,
      useChildren: useChildren,
      modifierScope: _ComposeDslModifierScope.row,
    ),
  );

  Widget _slotOrText(String name) {
    final slot = node.slots[name];
    if (slot != null && slot.isNotEmpty) {
      return _ComposeDslRenderer(
        node: slot.first,
        onAction: onAction,
        webViewHostContext: webViewHostContext,
        nodePath: '$nodePath:$name/0',
      );
    }
    final text = _string(node.props[name]);
    return Text(text.isEmpty ? name : text);
  }

  Widget _slotOrChildren(String name) {
    final slotChildren = _slotChildren(
      name,
      modifierScope: _ComposeDslModifierScope.column,
    );
    if (slotChildren.isNotEmpty) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: slotChildren,
      );
    }
    return _childrenColumn();
  }

  List<Widget> _slotChildren(
    String name, {
    bool useChildren = false,
    _ComposeDslModifierScope modifierScope = _ComposeDslModifierScope.normal,
  }) => _buildNodeWidgets(
    _slotNodes(name, useChildren: useChildren),
    pathPrefix: '$nodePath:$name',
    modifierScope: modifierScope,
  );

  List<_ComposeDslNode> _slotNodes(String name, {bool useChildren = false}) {
    final slot = node.slots[name];
    return slot != null && slot.isNotEmpty
        ? slot
        : (useChildren ? node.children : const <_ComposeDslNode>[]);
  }

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
            node: entry.value,
            onAction: onAction,
            webViewHostContext: webViewHostContext,
            nodePath: '$pathPrefix/${entry.key}',
            modifierScope: modifierScope,
          ),
        )
        .toList(growable: false);
  }

  bool _nodesRequireRowFlex(List<_ComposeDslNode> nodes) {
    return nodes.any((child) => _rowFlexSpec(child.props) != null);
  }

  Widget _slotColumn(String name) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    mainAxisSize: MainAxisSize.min,
    children: _slotChildren(
      name,
      modifierScope: _ComposeDslModifierScope.column,
    ),
  );

  Widget _slotCompactColumn(String name) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    mainAxisSize: MainAxisSize.min,
    children: _slotChildren(
      name,
      modifierScope: _ComposeDslModifierScope.column,
    ),
  );

  Widget _slotInline(String name) => Row(
    mainAxisSize: MainAxisSize.min,
    children: _slotChildren(name, modifierScope: _ComposeDslModifierScope.row),
  );

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

  Widget _tintedSlotColumn(
    BuildContext context,
    String name, {
    bool useChildren = false,
    Color? color,
  }) {
    return _withSlotColor(
      context,
      Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
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

  bool _hasSlot(String name) => (node.slots[name]?.isNotEmpty ?? false);

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
      webViewHostContext: webViewHostContext,
      nodePath: '$nodePath:$name/0',
    );
  }

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

  void _invokeAction(Object? rawAction, [Object? payload]) {
    final actionId = _actionId(rawAction);
    if (actionId == null) {
      return;
    }
    onAction(actionId, payload);
  }

  int _selectedIndex(List<_ComposeDslNode> items, Object? rawIndex) {
    final explicit = _int(rawIndex);
    if (explicit != null && explicit >= 0 && explicit < items.length) {
      return explicit;
    }
    final selected = items.indexWhere((item) => _bool(item.props['selected']));
    return selected < 0 ? 0 : selected;
  }

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

  Widget _image(BuildContext context) {
    final alpha = (_number(node.props['alpha']) ?? 1).clamp(0, 1).toDouble();
    final fit = _boxFit(node.props['contentScale']);
    final description = _string(node.props['contentDescription']).trim();
    Widget child;
    final source = _imageSource().trim();
    if (source.isNotEmpty) {
      child = Image.network(source, fit: fit);
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

class _ComposeTextField extends StatefulWidget {
  const _ComposeTextField({
    required this.identity,
    required this.value,
    required this.enabled,
    required this.readOnly,
    required this.obscureText,
    required this.singleLine,
    required this.minLines,
    required this.maxLines,
    required this.keyboardType,
    required this.textInputAction,
    required this.isError,
    required this.textStyle,
    required this.labelText,
    required this.label,
    required this.hintText,
    required this.hint,
    required this.prefixIcon,
    required this.suffixIcon,
    required this.prefix,
    required this.suffix,
    required this.supportingText,
    required this.helperText,
    required this.border,
    required this.onChanged,
  });

  final String identity;
  final String value;
  final bool enabled;
  final bool readOnly;
  final bool obscureText;
  final bool singleLine;
  final int? minLines;
  final int? maxLines;
  final TextInputType? keyboardType;
  final TextInputAction? textInputAction;
  final bool isError;
  final TextStyle? textStyle;
  final String? labelText;
  final Widget? label;
  final String? hintText;
  final Widget? hint;
  final Widget? prefixIcon;
  final Widget? suffixIcon;
  final Widget? prefix;
  final Widget? suffix;
  final Widget? supportingText;
  final String? helperText;
  final InputBorder border;
  final ValueChanged<String> onChanged;

  @override
  State<_ComposeTextField> createState() => _ComposeTextFieldState();
}

class _ComposeTextFieldState extends State<_ComposeTextField> {
  late TextEditingController _controller;
  late FocusNode _focusNode;
  late String _lastAppliedExternalValue;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.value);
    _controller.selection = TextSelection.collapsed(
      offset: widget.value.length,
    );
    _focusNode = FocusNode();
    _lastAppliedExternalValue = widget.value;
  }

  @override
  void didUpdateWidget(_ComposeTextField oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.identity != oldWidget.identity) {
      _controller.dispose();
      _focusNode.dispose();
      _controller = TextEditingController(text: widget.value);
      _controller.selection = TextSelection.collapsed(
        offset: widget.value.length,
      );
      _focusNode = FocusNode();
      _lastAppliedExternalValue = widget.value;
      return;
    }
    if (widget.value == _controller.text) {
      _lastAppliedExternalValue = widget.value;
      return;
    }
    final externalValueChanged = widget.value != _lastAppliedExternalValue;
    if (_focusNode.hasFocus && !externalValueChanged) {
      return;
    }
    final selection = _controller.selection;
    final start = selection.start.clamp(0, widget.value.length);
    final end = selection.end.clamp(0, widget.value.length);
    _controller.value = TextEditingValue(
      text: widget.value,
      selection: TextSelection(baseOffset: start, extentOffset: end),
    );
    _lastAppliedExternalValue = widget.value;
  }

  @override
  void dispose() {
    _controller.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: _controller,
      focusNode: _focusNode,
      enabled: widget.enabled,
      readOnly: widget.readOnly,
      obscureText: widget.obscureText,
      maxLines: widget.maxLines,
      minLines: widget.minLines,
      keyboardType: widget.keyboardType,
      textInputAction: widget.textInputAction,
      style: widget.textStyle,
      decoration: InputDecoration(
        labelText: widget.labelText,
        label: widget.label,
        hintText: widget.hintText,
        hint: widget.hint,
        errorText: widget.isError ? '' : null,
        prefixIcon: widget.prefixIcon,
        suffixIcon: widget.suffixIcon,
        prefix: widget.prefix,
        suffix: widget.suffix,
        helperText: widget.helperText,
        helper: widget.supportingText,
        border: widget.border,
      ),
      onChanged: (value) {
        widget.onChanged(value);
      },
    );
  }
}

class _ComposeCanvasPainter extends CustomPainter {
  const _ComposeCanvasPainter({
    required this.commands,
    required this.colorScheme,
    required this.textTheme,
  });

  final List<Map<String, Object?>> commands;
  final ColorScheme colorScheme;
  final TextTheme textTheme;

  @override
  void paint(Canvas canvas, Size size) {
    for (final command in commands) {
      final type = _normalizeToken(
        _string(command['type'] ?? command['command']),
      );
      switch (type) {
        case 'line':
        case 'drawline':
          _drawLine(canvas, size, command);
          break;
        case 'rect':
        case 'drawrect':
          _drawRect(canvas, size, command);
          break;
        case 'roundrect':
        case 'drawroundrect':
          _drawRoundRect(canvas, size, command);
          break;
        case 'circle':
        case 'drawcircle':
          _drawCircle(canvas, size, command);
          break;
        case 'text':
        case 'drawtext':
          _drawText(canvas, size, command);
          break;
        case 'icon':
        case 'drawicon':
          _drawIcon(canvas, size, command);
          break;
        case 'path':
        case 'drawpath':
          _drawPath(canvas, size, command);
          break;
      }
    }
  }

  void _drawLine(Canvas canvas, Size size, Map<String, Object?> command) {
    final paint = _paint(command, stroke: true);
    canvas.drawLine(
      Offset(
        _canvasNumber(command['x1'], size.width),
        _canvasNumber(command['y1'], size.height),
      ),
      Offset(
        _canvasNumber(command['x2'], size.width),
        _canvasNumber(command['y2'], size.height),
      ),
      paint,
    );
  }

  void _drawRect(Canvas canvas, Size size, Map<String, Object?> command) {
    final rect = Rect.fromLTWH(
      _canvasNumber(command['x'], size.width),
      _canvasNumber(command['y'], size.height),
      _canvasNumber(command['width'], size.width),
      _canvasNumber(command['height'], size.height),
    );
    canvas.drawRect(rect, _paint(command, stroke: _isStroke(command)));
  }

  void _drawRoundRect(Canvas canvas, Size size, Map<String, Object?> command) {
    final rect = Rect.fromLTWH(
      _canvasNumber(command['x'], size.width),
      _canvasNumber(command['y'], size.height),
      _canvasNumber(command['width'], size.width),
      _canvasNumber(command['height'], size.height),
    );
    final radius = _canvasNumber(
      command['radius'] ?? command['cornerRadius'],
      size.shortestSide,
    );
    canvas.drawRRect(
      RRect.fromRectAndRadius(rect, Radius.circular(radius)),
      _paint(command, stroke: _isStroke(command)),
    );
  }

  void _drawCircle(Canvas canvas, Size size, Map<String, Object?> command) {
    canvas.drawCircle(
      Offset(
        _canvasNumber(command['cx'] ?? command['x'], size.width),
        _canvasNumber(command['cy'] ?? command['y'], size.height),
      ),
      _canvasNumber(command['radius'] ?? command['r'], size.shortestSide),
      _paint(command, stroke: _isStroke(command)),
    );
  }

  void _drawText(Canvas canvas, Size size, Map<String, Object?> command) {
    final text = _string(command['text']);
    if (text.isEmpty) {
      return;
    }
    final painter =
        TextPainter(
          text: TextSpan(
            text: text,
            style: textTheme.bodyMedium!.copyWith(
              color: _canvasColor(command['color']) ?? colorScheme.onSurface,
              fontWeight: _fontWeight(_string(command['fontWeight'])),
            ),
          ),
          textScaler: TextScaler.linear(
            _canvasNumber(command['fontSize'], size.shortestSide, base: 14) /
                textTheme.bodyMedium!.fontSize!,
          ),
          maxLines: _int(command['maxLines']),
          textDirection: TextDirection.ltr,
        )..layout(
          minWidth: _canvasNumber(command['minWidth'], size.width),
          maxWidth: _canvasNumber(
            command['maxWidth'],
            size.width,
            base: size.width <= 0 ? double.infinity : size.width,
          ),
        );
    painter.paint(
      canvas,
      Offset(
        _canvasNumber(command['x'], size.width),
        _canvasNumber(command['y'], size.height),
      ),
    );
  }

  void _drawIcon(Canvas canvas, Size size, Map<String, Object?> command) {
    final icon = _iconData(_string(command['name'] ?? command['icon']));
    final fontSize = _canvasNumber(
      command['size'],
      size.shortestSide,
      base: 24,
    );
    final painter = TextPainter(
      text: TextSpan(
        text: String.fromCharCode(icon.codePoint),
        style: TextStyle(
          fontFamily: icon.fontFamily,
          color:
              _canvasColor(command['color'] ?? command['tint']) ??
              colorScheme.onSurface,
        ),
      ),
      textScaler: TextScaler.linear(fontSize / textTheme.bodyMedium!.fontSize!),
      textDirection: TextDirection.ltr,
    )..layout();
    painter.paint(
      canvas,
      Offset(
        _canvasNumber(command['x'], size.width),
        _canvasNumber(command['y'], size.height),
      ),
    );
  }

  void _drawPath(Canvas canvas, Size size, Map<String, Object?> command) {
    final ops = command['path'];
    if (ops is! List<Object?>) {
      return;
    }
    final path = Path();
    for (final rawOp in ops.whereType<Map<Object?, Object?>>()) {
      final op = _string(rawOp['op'] ?? rawOp['type'] ?? rawOp['command']);
      switch (_normalizeToken(op)) {
        case 'moveto':
          path.moveTo(
            _canvasNumber(rawOp['x'], size.width),
            _canvasNumber(rawOp['y'], size.height),
          );
          break;
        case 'lineto':
          path.lineTo(
            _canvasNumber(rawOp['x'], size.width),
            _canvasNumber(rawOp['y'], size.height),
          );
          break;
        case 'quadto':
          path.quadraticBezierTo(
            _canvasNumber(rawOp['x1'], size.width),
            _canvasNumber(rawOp['y1'], size.height),
            _canvasNumber(rawOp['x2'], size.width),
            _canvasNumber(rawOp['y2'], size.height),
          );
          break;
        case 'cubicto':
          path.cubicTo(
            _canvasNumber(rawOp['x1'], size.width),
            _canvasNumber(rawOp['y1'], size.height),
            _canvasNumber(rawOp['x2'], size.width),
            _canvasNumber(rawOp['y2'], size.height),
            _canvasNumber(rawOp['x3'], size.width),
            _canvasNumber(rawOp['y3'], size.height),
          );
          break;
        case 'close':
          path.close();
          break;
      }
    }
    canvas.drawPath(path, _paint(command, stroke: _isStroke(command)));
  }

  Paint _paint(Map<String, Object?> command, {required bool stroke}) {
    return Paint()
      ..isAntiAlias = true
      ..style = stroke ? PaintingStyle.stroke : PaintingStyle.fill
      ..strokeWidth = _number(command['strokeWidth']) ?? 1
      ..color =
          _canvasColor(command['color'] ?? command['brush']) ??
          colorScheme.primary;
  }

  bool _isStroke(Map<String, Object?> command) =>
      _normalizeToken(_string(command['style'])) == 'stroke' ||
      _number(command['strokeWidth']) != null;

  Color? _canvasColor(Object? raw) {
    if (raw is Map<Object?, Object?>) {
      final colors = raw['colors'];
      if (colors is List<Object?> && colors.isNotEmpty) {
        return _canvasColor(colors.first);
      }
      final token = raw['__colorToken']?.toString();
      final tokenColor = _colorToken(colorScheme, token ?? '');
      final alpha = _number(raw['alpha']);
      return alpha == null ? tokenColor : tokenColor?.withValues(alpha: alpha);
    }
    if (raw is String && raw.trim().startsWith('#')) {
      final hex = raw.trim().substring(1);
      final parsed = int.tryParse(hex.length == 6 ? 'ff$hex' : hex, radix: 16);
      return parsed == null ? null : Color(parsed);
    }
    return _colorToken(colorScheme, _string(raw));
  }

  double _canvasNumber(Object? raw, double axis, {double base = 0}) {
    if (raw is Map<Object?, Object?>) {
      final value = _number(raw['value']) ?? base;
      final unit = _string(raw['unit']).toLowerCase();
      return unit == 'fraction' ? value * axis : value;
    }
    return _number(raw) ?? base;
  }

  @override
  bool shouldRepaint(covariant _ComposeCanvasPainter oldDelegate) =>
      oldDelegate.commands != commands ||
      oldDelegate.colorScheme != colorScheme ||
      oldDelegate.textTheme != textTheme;
}

class _SizeReportingBox extends StatefulWidget {
  const _SizeReportingBox({required this.child, required this.onSizeChanged});

  final Widget child;
  final ValueChanged<Size> onSizeChanged;

  @override
  State<_SizeReportingBox> createState() => _SizeReportingBoxState();
}

class _SizeReportingBoxState extends State<_SizeReportingBox> {
  Size? _lastSize;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _reportSize());
  }

  @override
  void didUpdateWidget(covariant _SizeReportingBox oldWidget) {
    super.didUpdateWidget(oldWidget);
    WidgetsBinding.instance.addPostFrameCallback((_) => _reportSize());
  }

  void _reportSize() {
    if (!mounted) {
      return;
    }
    final size = context.size;
    if (size == null || size == _lastSize) {
      return;
    }
    _lastSize = size;
    widget.onSizeChanged(size);
  }

  @override
  Widget build(BuildContext context) => widget.child;
}

class _ParsedComposeDslActionEvent {
  const _ParsedComposeDslActionEvent({
    required this.phase,
    required this.renderResult,
    required this.actionResult,
    required this.errorText,
  });

  final String? phase;
  final _ComposeDslRenderResult? renderResult;
  final Object? actionResult;
  final String? errorText;

  static _ParsedComposeDslActionEvent parse(String event) {
    final decoded = jsonDecode(event) as Map<String, Object?>;
    final phase = decoded['phase']?.toString().trim();
    if (phase == 'intermediate' || phase == 'final') {
      final raw = decoded['result'];
      if (raw is! String) {
        throw StateError('compose_dsl action result event missing result');
      }
      final result = _ComposeDslRenderResult.tryParse(raw);
      return _ParsedComposeDslActionEvent(
        phase: phase,
        renderResult: result,
        actionResult: _ComposeDslRenderResult.actionResultOf(raw),
        errorText: null,
      );
    }
    if (phase == 'error') {
      return _ParsedComposeDslActionEvent(
        phase: phase,
        renderResult: null,
        actionResult: null,
        errorText: decoded['error']?.toString(),
      );
    }
    return _ParsedComposeDslActionEvent(
      phase: phase,
      renderResult: null,
      actionResult: null,
      errorText: null,
    );
  }
}

class _ComposeDslRenderResult {
  const _ComposeDslRenderResult({
    required this.tree,
    required this.state,
    required this.memo,
    required this.actionResult,
  });

  final _ComposeDslNode tree;
  final Map<String, Object?> state;
  final Map<String, Object?> memo;
  final Object? actionResult;

  static _ComposeDslRenderResult parse(String? raw) {
    final result = tryParse(raw);
    if (result != null) {
      return result;
    }
    throw FormatException(
      'compose_dsl result is invalid: ${_rawResultSummary(raw)}',
    );
  }

  static _ComposeDslRenderResult? tryParse(String? raw) {
    final value = _rootObject(raw);
    if (value == null) {
      return null;
    }
    final success = value['success'];
    if (success == false) {
      throw Exception((value['message'] ?? 'compose_dsl failed').toString());
    }
    final tree = _ComposeDslNode.parse(value['tree']);
    if (tree == null) {
      return null;
    }
    return _ComposeDslRenderResult(
      tree: tree,
      state: _stringMap(value['state']),
      memo: _stringMap(value['memo']),
      actionResult: _plainJsonValue(value['actionResult']),
    );
  }

  static Object? actionResultOf(String? raw) {
    final value = _rootObject(raw);
    if (value == null) {
      return null;
    }
    final success = value['success'];
    if (success == false) {
      throw Exception((value['message'] ?? 'compose_dsl failed').toString());
    }
    return _plainJsonValue(value['actionResult']);
  }

  static Map<Object?, Object?>? _rootObject(String? raw) {
    Object? value = raw;
    for (var i = 0; i < 3; i += 1) {
      if (value is String) {
        final trimmed = value.trim();
        if (trimmed.isEmpty) {
          break;
        }
        value = jsonDecode(trimmed);
      }
    }
    if (value is Map<Object?, Object?>) {
      return value;
    }
    return null;
  }

  static String _rawResultSummary(Object? raw) {
    final text = raw?.toString().trim();
    if (text == null || text.isEmpty) {
      return '<empty>';
    }
    const maxLength = 1200;
    if (text.length <= maxLength) {
      return text;
    }
    return '${text.substring(0, maxLength)}...';
  }
}

Object? _plainJsonValue(Object? raw) {
  if (raw is! String) {
    return raw;
  }
  final trimmed = raw.trim();
  if (trimmed.isEmpty) {
    return null;
  }
  try {
    return jsonDecode(trimmed);
  } catch (_) {
    return raw;
  }
}

class _ComposeDslNode {
  const _ComposeDslNode({
    required this.type,
    required this.props,
    required this.children,
    required this.slots,
  });

  final String type;
  final Map<String, Object?> props;
  final List<_ComposeDslNode> children;
  final Map<String, List<_ComposeDslNode>> slots;

  static _ComposeDslNode? parse(Object? raw) {
    if (raw is! Map<Object?, Object?>) {
      return null;
    }
    final type = (raw['type'] ?? '').toString().trim();
    if (type.isEmpty) {
      return null;
    }
    return _ComposeDslNode(
      type: type,
      props: _stringMap(raw['props']),
      children: _nodeList(raw['children']),
      slots: _slotMap(raw['slots']),
    );
  }
}

class _RouteTile extends StatelessWidget {
  const _RouteTile({
    required this.route,
    required this.selected,
    required this.onTap,
  });

  final core_proxy.ToolPkgUiRouteRuntime route;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      selected: selected,
      selectedTileColor: Theme.of(
        context,
      ).colorScheme.secondaryContainer.withValues(alpha: 0.55),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      leading: const Icon(Icons.dashboard_customize_outlined),
      title: Text(
        localizedText(route.title).isEmpty
            ? route.id
            : localizedText(route.title),
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: Text(
        route.routeId,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      onTap: onTap,
    );
  }
}

class _UiModuleTile extends StatelessWidget {
  const _UiModuleTile({
    required this.module,
    required this.selected,
    required this.onTap,
  });

  final core_proxy.ToolPkgUiModuleRuntime module;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      selected: selected,
      selectedTileColor: Theme.of(
        context,
      ).colorScheme.secondaryContainer.withValues(alpha: 0.55),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      leading: const Icon(Icons.tune_outlined),
      title: Text(
        localizedText(module.title).isEmpty
            ? module.id
            : localizedText(module.title),
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: Text(module.id, maxLines: 1, overflow: TextOverflow.ellipsis),
      onTap: onTap,
    );
  }
}

class _NavigationEntryTile extends StatelessWidget {
  const _NavigationEntryTile({required this.entry, required this.onTap});

  final core_proxy.ToolPkgNavigationEntryRuntime entry;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      leading: const Icon(Icons.open_in_new_outlined),
      title: Text(
        localizedText(entry.title).isEmpty
            ? entry.id
            : localizedText(entry.title),
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: Text(
        entry.routeId.trim().isEmpty ? entry.id : entry.routeId,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      trailing: onTap == null ? const Icon(Icons.bolt_outlined) : null,
      onTap: onTap,
    );
  }
}

class _NoUiView extends StatelessWidget {
  const _NoUiView();

  @override
  Widget build(BuildContext context) {
    return const Center(child: Icon(Icons.extension_off_outlined, size: 42));
  }
}

enum _ComposeDslModifierScope { normal, row, column, box }

class _RowFlexSpec {
  const _RowFlexSpec({required this.weight, required this.fill});

  final double weight;
  final bool fill;
}

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

bool _hasModifierOp(Object? rawModifier, String token) =>
    _modifierOpByToken(rawModifier, token) != null;

int _flexForWeight(double weight) {
  final scaled = (weight * 1000).round();
  return scaled < 1 ? 1 : scaled;
}

Widget _withModifier(
  BuildContext context,
  Widget child,
  Map<String, Object?> props,
  Future<Object?> Function(String actionId, [Object? payload]) onAction, {
  required String nodeType,
  required _ComposeDslModifierScope modifierScope,
}) {
  final ops = _modifierOps(props['modifier']);
  Widget current = child;
  current = _withDirectModifierProps(
    context,
    current,
    props,
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
      case 'fillMaxWidth':
      case 'fillmaxwidth':
        if (modifierScope != _ComposeDslModifierScope.row) {
          current = SizedBox(width: double.infinity, child: current);
        }
        break;
      case 'fillMaxHeight':
      case 'fillmaxheight':
        current = SizedBox(height: double.infinity, child: current);
        break;
      case 'fillMaxSize':
      case 'fillmaxsize':
        if (modifierScope == _ComposeDslModifierScope.row) {
          current = SizedBox(height: double.infinity, child: current);
        } else {
          current = SizedBox.expand(child: current);
        }
        break;
      case 'width':
      case 'requiredWidth':
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
        current = DecoratedBox(
          decoration: BoxDecoration(
            color: _color(context, args.firstOrNull),
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
  return current;
}

Widget _withDirectModifierProps(
  BuildContext context,
  Widget child,
  Map<String, Object?> props, {
  required String nodeType,
  required _ComposeDslModifierScope modifierScope,
}) {
  Widget current = child;
  final width = _number(props['width']);
  final height = _number(props['height']);
  if (width != null || height != null) {
    current = SizedBox(width: width, height: height, child: current);
  }
  if (_bool(props['fillMaxSize'])) {
    if (modifierScope == _ComposeDslModifierScope.row) {
      current = SizedBox(height: double.infinity, child: current);
    } else {
      current = SizedBox.expand(child: current);
    }
  } else if (_bool(props['fillMaxWidth'])) {
    if (modifierScope != _ComposeDslModifierScope.row) {
      current = SizedBox(width: double.infinity, child: current);
    }
  } else if (_bool(props['fillMaxHeight'])) {
    current = SizedBox(height: double.infinity, child: current);
  }
  final padding = props['padding'] ?? props['contentPadding'];
  if (padding != null) {
    current = Padding(padding: _edgeInsetsFromValue(padding), child: current);
  }
  final background = props['backgroundColor'] ?? props['background'];
  if (background != null) {
    current = DecoratedBox(
      decoration: BoxDecoration(
        color: _color(context, background),
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

Key _webViewKey({
  required Map<String, Object?> props,
  required String nodePath,
  required ComposeDslWebViewHostContext webViewHostContext,
}) {
  final explicitKey = _string(props['key']).trim();
  final controller = props['controller'];
  final controllerKey = controller is Map<Object?, Object?>
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

EdgeInsets _edgeInsets(List<Object?> args) {
  if (args.isEmpty) {
    return EdgeInsets.zero;
  }
  final first = args.first;
  if (first is Map<Object?, Object?>) {
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

EdgeInsets _edgeInsetsFromValue(Object? raw) {
  if (raw is List<Object?>) {
    return _edgeInsets(raw);
  }
  return _edgeInsets(<Object?>[raw]);
}

List<Map<String, Object?>> _modifierOps(Object? raw) {
  if (raw is Map<Object?, Object?> && raw['__modifierOps'] is List<Object?>) {
    return (raw['__modifierOps'] as List<Object?>)
        .whereType<Map<Object?, Object?>>()
        .map(_stringMap)
        .toList(growable: false);
  }
  return const <Map<String, Object?>>[];
}

BoxConstraints _axisConstraints(
  List<Object?> args, {
  required bool horizontal,
}) {
  final first = args.firstOrNull;
  double? min;
  double? max;
  if (first is Map<Object?, Object?>) {
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

BoxConstraints _sizeConstraints(List<Object?> args) {
  final first = args.firstOrNull;
  if (first is Map<Object?, Object?>) {
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

Offset _offset(List<Object?> args) {
  final first = args.firstOrNull;
  if (first is Map<Object?, Object?>) {
    return Offset(_number(first['x']) ?? 0, _number(first['y']) ?? 0);
  }
  return Offset(
    _number(first) ?? 0,
    _number(args.length > 1 ? args[1] : null) ?? 0,
  );
}

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
  );
}

TextStyle? _textFieldStyle(BuildContext context, Object? raw) {
  if (raw is! Map<Object?, Object?>) {
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

TextStyle _scaledTextStyle(TextStyle style, double? size) {
  if (size == null) {
    return style;
  }
  return style.apply(fontSizeFactor: size / style.fontSize!);
}

FontWeight? _fontWeight(String value) {
  return switch (value.toLowerCase()) {
    'bold' || 'w700' || '700' => FontWeight.w700,
    'semibold' || 'w600' || '600' => FontWeight.w600,
    'medium' || 'w500' || '500' => FontWeight.w500,
    'light' || 'w300' || '300' => FontWeight.w300,
    _ => null,
  };
}

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

CrossAxisAlignment _crossAxis(Object? raw) {
  return switch (_string(raw)) {
    'center' ||
    'centerHorizontally' ||
    'centerVertically' => CrossAxisAlignment.center,
    'end' || 'right' || 'bottom' => CrossAxisAlignment.end,
    _ => CrossAxisAlignment.start,
  };
}

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

BorderRadius? _borderRadius(Object? raw) {
  if (raw is Map<Object?, Object?>) {
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

OutlinedBorder? _shapeBorder(Object? raw, {BorderRadius? defaultBorderRadius}) {
  final radius = _borderRadius(raw) ?? defaultBorderRadius;
  return radius == null ? null : RoundedRectangleBorder(borderRadius: radius);
}

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
  if (raw is Map<Object?, Object?>) {
    final token = raw['__colorToken']?.toString();
    final color = _colorToken(colorScheme, token ?? '');
    final alpha = _number(raw['alpha']);
    return alpha == null ? color : color?.withValues(alpha: alpha);
  }
  return null;
}

Color? _colorWithAlpha(BuildContext context, Object? raw, Object? alphaRaw) {
  final color = _color(context, raw);
  final alpha = _number(alphaRaw);
  return color == null || alpha == null
      ? color
      : color.withValues(alpha: alpha.clamp(0, 1).toDouble());
}

BorderSide? _borderSide(BuildContext context, Object? raw) {
  if (raw is! Map<Object?, Object?>) {
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

IconData _iconData(String name) {
  final alias = switch (name) {
    'add' || 'plus' => Icons.add,
    'close' => Icons.close,
    'check' => Icons.check,
    'settings' => Icons.settings,
    'search' => Icons.search,
    'delete' => Icons.delete_outline,
    'edit' => Icons.edit_outlined,
    'refresh' => Icons.refresh,
    'download' => Icons.download,
    'upload' => Icons.upload,
    'save' => Icons.save_outlined,
    'home' => Icons.home_outlined,
    'info' => Icons.info_outline,
    'warning' => Icons.warning_amber_outlined,
    'person' || 'account' => Icons.person_outline,
    'folder' => Icons.folder_outlined,
    'file' => Icons.insert_drive_file_outlined,
    'play' => Icons.play_arrow,
    'pause' => Icons.pause,
    'stop' => Icons.stop,
    'menu' => Icons.menu,
    'more' || 'moreVert' => Icons.more_vert,
    'arrowBack' || 'back' => Icons.arrow_back,
    'arrowForward' || 'forward' => Icons.arrow_forward,
    _ => null,
  };
  return alias ??
      MaterialIconNameResolver.resolveOrDefault(name, Icons.widgets_outlined);
}

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

String? _actionId(Object? raw) {
  if (raw is Map<Object?, Object?>) {
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

Map<String, Object?> _stringMap(Object? raw) {
  if (raw is Map<Object?, Object?>) {
    return raw.map((key, value) => MapEntry(key.toString(), value));
  }
  return <String, Object?>{};
}

List<Map<String, Object?>> _canvasCommands(Object? raw) {
  if (raw is! List<Object?>) {
    return const <Map<String, Object?>>[];
  }
  return raw
      .whereType<Map<Object?, Object?>>()
      .map(_stringMap)
      .toList(growable: false);
}

List<_ComposeDslNode> _nodeList(Object? raw) {
  if (raw is List<Object?>) {
    return raw
        .map(_ComposeDslNode.parse)
        .whereType<_ComposeDslNode>()
        .toList(growable: false);
  }
  return const <_ComposeDslNode>[];
}

Map<String, List<_ComposeDslNode>> _slotMap(Object? raw) {
  if (raw is Map<Object?, Object?>) {
    return raw.map((key, value) => MapEntry(key.toString(), _nodeList(value)));
  }
  return const <String, List<_ComposeDslNode>>{};
}

String _string(Object? raw) => raw?.toString() ?? '';

bool _bool(Object? raw) {
  if (raw is bool) {
    return raw;
  }
  final text = raw?.toString().trim().toLowerCase();
  return text == 'true' || text == '1' || text == 'yes';
}

String _normalizeToken(String value) =>
    value.replaceAll(RegExp(r'[^A-Za-z0-9]'), '').toLowerCase();

double? _number(Object? raw) {
  if (raw is num) {
    return raw.toDouble();
  }
  if (raw is Map<Object?, Object?> && raw['value'] is num) {
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

  Object? elementAtOrNull(int index) =>
      index < 0 || index >= length ? null : this[index];
}
