// ignore_for_file: file_names

import 'dart:async';
import '../../../../core/application/PluginHotReload.dart';
import 'dart:convert';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import '../../../common/components/PageActivityMixin.dart';
import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/logging/ClientLogger.dart';
import '../../../../core/theme/PluginThemeSnapshot.dart';

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
import 'compose_dsl/fill_layout.dart';
import 'compose_dsl/lazy_viewport.dart';
import 'compose_dsl/action_scheduler.dart';

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
  /// Creates a routable or embedded screen using the existing Compose host.
  const ToolPkgUiLauncherScreen({
    super.key,
    required this.clients,
    required this.plugin,
    this.initialRouteId,
    this.embeddedScreenPath,
    this.showLauncherChrome = true,
    this.showLoadingIndicator = true,
    this.dialogTitle,
    this.initialState = const <String, Object?>{},
    this.initialMemo = const <String, Object?>{},
    this.initialModuleSpec,
    this.onActionResult,
  });

  final GeneratedCoreProxyClients clients;
  final core_proxy.ToolPkgContainerRuntime plugin;
  final String? initialRouteId;

  /// Identifies an embedded script independently of plugin-provided module metadata.
  final String? embeddedScreenPath;
  final bool showLauncherChrome;
  final bool showLoadingIndicator;

  /// Embeds the screen in the modal route owned by its caller.
  final String? dialogTitle;
  final Map<String, Object?> initialState;
  final Map<String, Object?> initialMemo;
  final Map<String, Object?>? initialModuleSpec;

  /// Delivers completed plugin actions to the owner of an embedded surface.
  final ValueChanged<Object?>? onActionResult;

  /// Creates the state that owns the plugin execution and render lifecycle.
  @override
  State<ToolPkgUiLauncherScreen> createState() =>
      _ToolPkgUiLauncherScreenState();
}

class _ToolPkgUiLauncherScreenState extends State<ToolPkgUiLauncherScreen> {
  static int _nextExecutionOwnerId = 0;
  static const String _logTag = 'ToolPkgUiLauncher';

  late final int _executionOwnerId = _nextExecutionOwnerId++;
  late final String _selectedRouteId = _initialRouteId();
  _ComposeDslRenderResult? _renderResult;
  String? _scriptScreenPath;
  ({String contextKey, String containerPackageName})? _activeExecutionContext;
  bool _loading = true;
  bool _loadedInitialRoute = false;
  int _routeLoadGeneration = 0;
  String _currentLanguageTag = 'en';
  ColorScheme? _themeScheme;
  String? _error;
  final _actionScheduler = ComposeDslActionScheduler();
  Future<void> _renderTail = Future<void>.value();
  final Set<StreamSubscription<String>> _detachedComposeEventSubscriptions = {};

  GeneratedApplicationPackageManagerCoreProxy get _packageManager =>
      widget.clients.application.packageManager();

  @override
  void initState() {
    super.initState();
    ComposeDslWebViewHostRegistry.ensureHostInteractionRegistered();
    PluginHotReload.revision.addListener(_reloadDevelopmentPackage);
  }

  /// Updates an embedded XML screen without destroying its tree or JS context.
  @override
  void didUpdateWidget(covariant ToolPkgUiLauncherScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!_composeInputEquals(oldWidget.initialState, widget.initialState) ||
        !_composeInputEquals(oldWidget.initialMemo, widget.initialMemo) ||
        (widget.embeddedScreenPath == null &&
            !_composeInputEquals(
              oldWidget.initialModuleSpec,
              widget.initialModuleSpec,
            ))) {
      unawaited(_loadRoute(updateInputs: true));
    }
  }

  /// Recreates the visible plugin document after runtime packages reload.
  void _reloadDevelopmentPackage() {
    setState(() {
      _renderResult = null;
      _loading = true;
      _activeExecutionContext = null;
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) unawaited(_loadRoute());
    });
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final scheme = Theme.of(context).colorScheme;
    final themeChanged = _themeScheme != scheme;
    _themeScheme = scheme;
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
    } else if (themeChanged) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted && !_loading) unawaited(_notifyThemeChanged());
      });
    }
  }

  /// Sends the current UI theme through the normal serialized plugin action lifecycle.
  Future<void> _notifyThemeChanged() async {
    final scheme = _themeScheme!;
    final uiModuleId = _selectedUiModuleId();
    final routeInstanceId = _selectedRouteInstanceId();
    final executionContextKey = _executionContextKey(
      uiModuleId: uiModuleId,
      routeInstanceId: routeInstanceId,
    );
    ClientLogger.i(
      'event=theme_dispatch_start package=${widget.plugin.packageName} '
      'context=$executionContextKey brightness=${scheme.brightness.name} '
      'primary=${scheme.primary.toARGB32().toRadixString(16)}',
      tag: _logTag,
    );
    try {
      await _dispatchAction(
        '__operit_theme_changed',
        pluginThemeSnapshot(scheme),
      );
      ClientLogger.i(
        'event=theme_dispatch_done package=${widget.plugin.packageName} '
        'context=$executionContextKey',
        tag: _logTag,
      );
    } catch (error, stackTrace) {
      ClientLogger.e(
        'event=theme_dispatch_failed package=${widget.plugin.packageName} '
        'context=$executionContextKey',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
      rethrow;
    }
  }

  String _initialRouteId() {
    final requested = widget.initialRouteId?.trim();
    if (requested != null && requested.isNotEmpty) {
      if (_embeddedScreenPath() != null) {
        return requested;
      }
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

  /// Serializes renders and invalidates superseded XML updates.
  Future<void> _loadRoute({bool updateInputs = false}) async {
    final generation = ++_routeLoadGeneration;
    final pending = _renderTail.then((_) async {
      await _loadRouteCore(generation, updateInputs: updateInputs);
    });
    _renderTail = pending;
    await pending;
  }

  /// Renders the latest input while preserving an already visible DSL tree.
  Future<void> _loadRouteCore(
    int routeLoadGeneration, {
    required bool updateInputs,
  }) async {
    if (!_isCurrentRouteLoad(routeLoadGeneration)) {
      return;
    }
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
      // Streaming input is an update, not a new page initialization.
      _loading = !updateInputs || _renderResult == null;
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
      final embeddedScreenPath = _embeddedScreenPath();
      final String? script;
      final String? screenPath;
      if (embeddedScreenPath != null) {
        script = await _packageManager.readToolPkgTextResource(
          packageNameOrSubpackageId: widget.plugin.packageName,
          resourcePath: embeddedScreenPath,
          preferEnabledContainer: true,
        );
        screenPath = embeddedScreenPath;
      } else {
        script = await _packageManager.getToolPkgComposeDslScript(
          containerPackageName: widget.plugin.packageName,
          uiModuleId: uiModuleId,
        );
        screenPath = await _packageManager.getToolPkgComposeDslScreenPath(
          containerPackageName: widget.plugin.packageName,
          uiModuleId: uiModuleId,
        );
      }
      if (!_isCurrentRouteLoad(routeLoadGeneration)) {
        return;
      }
      if (script == null || script.trim().isEmpty) {
        if (embeddedScreenPath != null) {
          throw StateError(
            'compose_dsl screen not found: '
            'package=${widget.plugin.packageName}, screen=$embeddedScreenPath',
          );
        }
        throw StateError(
          'compose_dsl script not found: '
          'package=${widget.plugin.packageName}, module=$uiModuleId',
        );
      }
      _scriptScreenPath = screenPath;
      final renderedTheme = _themeScheme;
      final raw = await _packageManager.executeToolPkgComposeDslScript(
        contextKey: executionContextKey,
        containerPackageName: widget.plugin.packageName,
        script: script,
        runtimeOptions: _runtimeOptions(
          uiModuleId: uiModuleId,
          routeInstanceId: routeInstanceId,
          executionContextKey: executionContextKey,
          updateInputs: updateInputs,
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
      if (_themeScheme != renderedTheme) {
        _scheduleThemeDispatchAfterRouteRender(routeLoadGeneration);
      }
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

  /// Dispatches a theme change after Flutter has bound controllers from the new tree.
  void _scheduleThemeDispatchAfterRouteRender(int routeLoadGeneration) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_isCurrentRouteLoad(routeLoadGeneration) && !_loading) {
        unawaited(_notifyThemeChanged());
      }
    });
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
    PluginHotReload.revision.removeListener(_reloadDevelopmentPackage);
    for (final subscription in _detachedComposeEventSubscriptions) {
      unawaited(subscription.cancel());
    }
    _detachedComposeEventSubscriptions.clear();
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

  /// Waits for text synchronization without serializing ordinary UI actions.
  Future<Object?> _dispatchAction(String actionId, [Object? payload]) {
    final routeGeneration = _routeLoadGeneration;
    return _actionScheduler.dispatchAction(() {
      if (!_isCurrentRouteLoad(routeGeneration)) return Future.value(null);
      return _dispatchActionCore(
        actionId,
        payload,
        reportAndSuppressErrors: true,
        notifyActionResult: true,
      );
    });
  }

  /// Preserves keystroke order across all text fields in this page.
  Future<Object?> _dispatchTextInput(String actionId, String text) {
    final routeGeneration = _routeLoadGeneration;
    return _actionScheduler.dispatchTextInput(() {
      if (!_isCurrentRouteLoad(routeGeneration)) return Future.value(null);
      return _dispatchActionCore(
        actionId,
        text,
        reportAndSuppressErrors: true,
        notifyActionResult: true,
      );
    });
  }

  /// Gives WebView actions the same text barrier while preserving error delivery.
  Future<Object?> _dispatchWebViewAction(String actionId, [Object? payload]) {
    final routeGeneration = _routeLoadGeneration;
    return _actionScheduler.dispatchAction(() {
      if (!_isCurrentRouteLoad(routeGeneration)) return Future.value(null);
      return _dispatchActionCore(
        actionId,
        payload,
        reportAndSuppressErrors: false,
        notifyActionResult: false,
      );
    });
  }

  /// Streams render updates and settles the individual action on completion.
  Future<Object?> _dispatchActionCore(
    String actionId,
    Object? payload, {
    required bool reportAndSuppressErrors,
    required bool notifyActionResult,
  }) async {
    final routeGeneration = _routeLoadGeneration;
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
      final rootActionId = _actionId(_renderResult?.tree.props['onLoad']);
      final keepDetachedEvents =
          widget.initialModuleSpec?['slot'] == 'above_input' &&
          rootActionId == actionId;
      final runtimeOptions = _runtimeOptions(
        uiModuleId: uiModuleId,
        routeInstanceId: routeInstanceId,
        executionContextKey: executionContextKey,
      )..['__operit_keep_compose_event_stream'] = keepDetachedEvents;
      final eventStream = _packageManager.dispatchToolPkgComposeDslActionEvents(
        contextKey: executionContextKey,
        containerPackageName: widget.plugin.packageName,
        actionId: actionId,
        payload: payload,
        runtimeOptions: runtimeOptions,
        envOverrides: const <String, String>{},
      );
      final completion = Completer<void>();
      late final StreamSubscription<String> subscription;
      subscription = eventStream.listen(
        (event) {
          if (!mounted) {
            if (!completion.isCompleted) {
              completion.complete();
            }
            unawaited(subscription.cancel());
            return;
          }
          final parsedEvent = _ParsedComposeDslActionEvent.parse(event);
          final phase = parsedEvent.phase;
          if (phase == 'intermediate' || phase == 'final') {
            latestActionResult = parsedEvent.actionResult;
            navigationCommands.addAll(parsedEvent.navigationCommands);
            final result = parsedEvent.renderResult;
            if (result == null) {
              return;
            }
            if (!mounted) {
              return;
            }
            setState(() {
              _renderResult = result;
              _error = null;
            });
          } else if (phase == 'error') {
            final errorText = parsedEvent.errorText;
            if (errorText == null) {
              if (!completion.isCompleted) {
                completion.completeError(
                  StateError('compose_dsl action error event missing error'),
                );
              }
              return;
            }
            if (!mounted) {
              return;
            }
            setState(() {
              _error = errorText;
            });
            if (!completion.isCompleted) {
              completion.completeError(StateError(errorText));
            }
          } else if (phase == 'complete') {
            if (!completion.isCompleted) {
              completion.complete();
            }
            if (!keepDetachedEvents) {
              unawaited(subscription.cancel());
            }
          }
        },
        onError: (Object error, StackTrace stackTrace) {
          if (!completion.isCompleted) {
            completion.completeError(error, stackTrace);
          }
        },
        onDone: () {
          if (!completion.isCompleted) {
            completion.completeError(
              StateError('compose_dsl action stream closed before completion'),
            );
          }
        },
      );
      if (keepDetachedEvents) {
        _detachedComposeEventSubscriptions.add(subscription);
      }
      await completion.future;
      _navigateCommands(navigationCommands);
      if (notifyActionResult) {
        _notifyActionResult(
          latestActionResult,
          routeGeneration: routeGeneration,
        );
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

  /// Delivers results only to the presentation that still owns this route load.
  void _notifyActionResult(Object? result, {required int routeGeneration}) {
    if (_isCurrentRouteLoad(routeGeneration)) {
      widget.onActionResult?.call(result);
    }
  }

  Map<String, Object?> _runtimeOptions({
    required String uiModuleId,
    required String routeInstanceId,
    required String executionContextKey,
    bool updateInputs = false,
  }) {
    return <String, Object?>{
      'packageName': widget.plugin.packageName,
      'theme': pluginThemeSnapshot(_themeScheme!),
      'containerPackageName': widget.plugin.packageName,
      'toolPkgId': widget.plugin.packageName,
      '__operit_ui_package_name': widget.plugin.packageName,
      '__operit_ui_toolpkg_id': widget.plugin.packageName,
      'uiModuleId': uiModuleId,
      '__operit_ui_module_id': uiModuleId,
      '__operit_toolpkg_runtime_kind': 'ui',
      'state': updateInputs
          ? <String, Object?>{...?_renderResult?.state, ...widget.initialState}
          : _renderResult?.state ?? widget.initialState,
      'memo': updateInputs
          ? <String, Object?>{...?_renderResult?.memo, ...widget.initialMemo}
          : _renderResult?.memo ?? widget.initialMemo,
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
    final embeddedScreenPath = _embeddedScreenPath();
    if (embeddedScreenPath != null) {
      return 'toolpkg_xml_render:$container:$embeddedScreenPath:$_executionOwnerId';
    }
    return 'toolpkg_compose_dsl:$container:$module:$route:$_executionOwnerId';
  }

  /// Reads the embedded screen path without consulting optional module metadata.
  String? _embeddedScreenPath() {
    final path = widget.embeddedScreenPath;
    if (path == null) return null;
    final normalized = path.trim();
    if (normalized.isEmpty) {
      throw ArgumentError.value(
        path,
        'embeddedScreenPath',
        'must not be blank',
      );
    }
    return normalized;
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
    ClientLogger.e(
      'event=compose_dispatch_failed phase=$phase '
      'package=${widget.plugin.packageName} route=$_selectedRouteId',
      tag: _logTag,
      error: error,
      stackTrace: stackTrace,
    );
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
    final routeGeneration = _routeLoadGeneration;
    final webViewHostContext = ComposeDslWebViewHostContext(
      packageName: widget.plugin.packageName,
      routeInstanceId: routeInstanceId,
      executionContextKey: executionContextKey,
      dispatchAction: _dispatchWebViewAction,
      onActionResultDelivered: (result) =>
          _notifyActionResult(result, routeGeneration: routeGeneration),
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
            showLoadingIndicator: widget.showLoadingIndicator,
            dialogTitle: widget.dialogTitle,
            onAction: _dispatchAction,
            onTextInput: _dispatchTextInput,
            webViewHostContext: webViewHostContext,
            splitMarkdownContent: (content) => widget
                .clients
                .chatRuntimeHolderMain
                .splitMarkdownContent(content: content),
          )
        : const _NoUiView();
    if (!widget.showLauncherChrome) {
      return content;
    }
    return Scaffold(
      appBar: AppBar(title: Text(toolPkgContainerDisplayName(widget.plugin))),
      body: SafeArea(child: content),
    );
  }

  bool _hasSelectedUi() {
    if (_embeddedScreenPath() != null) {
      return true;
    }
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
