// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import '../../../../core/link/CoreLinkProtocol.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/AdaptiveSidePanel.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../common/icons/MaterialIconNameResolver.dart';
import '../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../main/navigation/AppNavigationModels.dart';
import '../../chat/screens/AIChatScreen.dart';
import '../utils/PackageDisplayUtils.dart';
import 'ToolPkgComposeDslWebView.dart';

part 'compose_dsl/compose_host.dart';
part 'compose_dsl/dialog_host.dart';
part 'compose_dsl/lazy_list.dart';
part 'compose_dsl/gesture_region.dart';
part 'compose_dsl/text_field.dart';
part 'compose_dsl/canvas_painter.dart';
part 'compose_dsl/size_reporting.dart';
part 'compose_dsl/render_models.dart';
part 'compose_dsl/modifiers.dart';
part 'compose_dsl/value_parsers.dart';
part 'compose_dsl/renderer.dart';
part 'compose_dsl/box_layout.dart';
part 'compose_dsl/flex_layout.dart';
part 'compose_dsl/desktop_widget.dart';
part 'compose_dsl/material_controls.dart';
part 'compose_dsl/navigation_nodes.dart';
part 'compose_dsl/interactive_nodes.dart';
part 'compose_dsl/renderer_slots.dart';

class ToolPkgUiLauncherScreen extends StatefulWidget {
  const ToolPkgUiLauncherScreen({
    super.key,
    required this.clients,
    required this.plugin,
    this.initialRouteId,
    this.showLauncherChrome = true,
    this.initialState = const <String, Object?>{},
    this.initialMemo = const <String, Object?>{},
    this.initialModuleSpec,
  });

  final GeneratedCoreProxyClients clients;
  final core_proxy.ToolPkgContainerRuntime plugin;
  final String? initialRouteId;
  final bool showLauncherChrome;
  final Map<String, Object?> initialState;
  final Map<String, Object?> initialMemo;
  final Map<String, Object?>? initialModuleSpec;

  @override
  State<ToolPkgUiLauncherScreen> createState() =>
      _ToolPkgUiLauncherScreenState();
}

class _ToolPkgUiLauncherScreenState extends State<ToolPkgUiLauncherScreen> {
  static int _nextExecutionOwnerId = 0;

  late final int _executionOwnerId = _nextExecutionOwnerId++;
  late final String _selectedRouteId = _initialRouteId();
  _ComposeDslRenderResult? _renderResult;
  String? _scriptScreenPath;
  ({String contextKey, String containerPackageName})? _activeExecutionContext;
  bool _loading = true;
  bool _loadedInitialRoute = false;
  int _routeLoadGeneration = 0;
  String _currentLanguageTag = 'en';
  String? _error;
  Future<Object?> _actionTail = Future<Object?>.value();

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
      if (_activeExecutionContext != executionContext) {
        _activeExecutionContext = executionContext;
        await _acquireExecutionContext(executionContext);
        if (!_isCurrentRouteLoad(routeLoadGeneration)) {
          if (_activeExecutionContext != executionContext) {
            await _releaseExecutionContext(executionContext);
          }
          return;
        }
      }
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
      final result = _ComposeDslRenderResult.parse(raw);
      if (!_isCurrentRouteLoad(routeLoadGeneration)) {
        return;
      }
      setState(() {
        _renderResult = result;
        _loading = false;
      });
      _navigateCommands(_ComposeDslRenderResult.navigationCommandsOf(raw));
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

  /// Acquires one page-owned ToolPkg execution context.
  Future<void> _acquireExecutionContext(
    ({String contextKey, String containerPackageName}) executionContext,
  ) async {
    await _packageManager.acquireToolPkgExecutionEngine(
      contextKey: executionContext.contextKey,
      containerPackageName: executionContext.containerPackageName,
    );
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

  /// Preserves edit order and makes later button actions observe settled input.
  Future<Object?> _dispatchAction(String actionId, [Object? payload]) {
    final routeGeneration = _routeLoadGeneration;
    _actionTail = _actionTail.then((_) {
      if (!mounted || routeGeneration != _routeLoadGeneration) return null;
      return _dispatchActionCore(
        actionId,
        payload,
        reportAndSuppressErrors: true,
      );
    });
    return _actionTail;
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
    final navigationCommands =
        <({String routeId, Map<String, Object?> args})>[];
    try {
      await for (final event
          in _packageManager.dispatchToolPkgComposeDslActionEvents(
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
          )) {
        if (!mounted) {
          return latestActionResult;
        }
        final parsedEvent = _ParsedComposeDslActionEvent.parse(event);
        final phase = parsedEvent.phase;
        if (phase == 'intermediate' || phase == 'final') {
          latestActionResult = parsedEvent.actionResult;
          navigationCommands.addAll(parsedEvent.navigationCommands);
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
      _navigateCommands(navigationCommands);
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
      'state': _renderResult?.state ?? widget.initialState,
      'memo': _renderResult?.memo ?? widget.initialMemo,
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

  /// Applies navigation side effects after their render or action completes.
  void _navigateCommands(
    List<({String routeId, Map<String, Object?> args})> commands,
  ) {
    for (final command in commands) {
      AppRouterGateway.navigate(
        routeId: command.routeId,
        args: command.args,
        source: RouteEntrySource.script,
      );
    }
  }

  void _printComposeError(String phase, Object error, StackTrace stackTrace) {
    final diagnostic = error is CoreLinkError
        ? error.toDiagnosticString()
        : error.toString();
    debugPrint(
      'ToolPkg compose_dsl $phase error: '
      'package=${widget.plugin.packageName}, '
      'route=$_selectedRouteId, '
      'error=$diagnostic',
    );
    debugPrintStack(stackTrace: stackTrace);
  }

  Map<String, Object?> _moduleSpec(String routeId) {
    final initialModuleSpec = widget.initialModuleSpec;
    if (initialModuleSpec != null) {
      return initialModuleSpec;
    }
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
            splitMarkdownContent: (content) => widget
                .clients
                .chatRuntimeHolderMain
                .splitMarkdownContent(content: content),
          )
        : const _NoUiView();
    if (!widget.showLauncherChrome) {
      return SizedBox.expand(child: content);
    }
    return Scaffold(
      appBar: AppBar(title: Text(toolPkgContainerDisplayName(widget.plugin))),
      body: SafeArea(child: content),
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
}
