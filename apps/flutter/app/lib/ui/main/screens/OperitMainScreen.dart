// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../components/AppContent.dart';
import '../layout/NavigationLayoutMetrics.dart';
import '../layout/PhoneLayout.dart';
import '../MainLayoutController.dart';
import '../TopBarController.dart';
import '../layout/TabletLayout.dart';
import '../navigation/AppNavigationModels.dart';
import '../navigation/AppRouteCatalog.dart';
import 'OperitScreens.dart';

class OperitMainScreen extends StatefulWidget {
  const OperitMainScreen({super.key});

  @override
  State<OperitMainScreen> createState() => _OperitMainScreenState();
}

class _OperitMainScreenState extends State<OperitMainScreen> {
  static const int _backPressedIntervalMs = 2000;
  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  );

  late AppNavigationModel _navigationModel;
  late final AppRouterState _routerState;
  late final TopBarController _topBarController;
  late final MainLayoutController _mainLayoutController;
  List<core_proxy.ToolPkgUiRoute> _toolPkgUiRoutes =
      const <core_proxy.ToolPkgUiRoute>[];
  List<core_proxy.ToolPkgNavigationEntry> _toolPkgNavigationEntries =
      const <core_proxy.ToolPkgNavigationEntry>[];
  bool _drawerOpen = false;
  bool _isTabletSidebarExpanded = false;
  bool _isNavigatingBack = false;
  bool _requestedInitialToolPkgNavigationRefresh = false;
  int _backPressedTime = 0;

  @override
  void initState() {
    super.initState();
    _topBarController = TopBarController();
    _mainLayoutController = MainLayoutController();
    _routerState = AppRouterState(AppRouteCatalog.initialEntry());
    AppRouterGateway.install(handler: _navigateToRoute, reset: _resetToRoute);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _navigationModel = AppRouteCatalog.build(
      context,
      toolPkgUiRoutes: _toolPkgUiRoutes,
      toolPkgNavigationEntries: _toolPkgNavigationEntries,
    );
    AppRouteDiscoveryGateway.install(() => _navigationModel.routes);
    if (!_requestedInitialToolPkgNavigationRefresh) {
      _requestedInitialToolPkgNavigationRefresh = true;
      _refreshToolPkgNavigationModel();
    }
  }

  @override
  void dispose() {
    AppRouterGateway.clear();
    AppRouteDiscoveryGateway.clear();
    _routerState.dispose();
    _topBarController.dispose();
    _mainLayoutController.dispose();
    super.dispose();
  }

  void _navigateToRoute(
    String routeId,
    Map<String, Object?> args,
    RouteEntrySource source,
  ) {
    final routeSpec = _navigationModel.routesById[routeId];
    if (routeSpec == null) {
      throw StateError('Unknown routeId: $routeId');
    }
    _isNavigatingBack = false;
    if (!_shouldPreserveTopBarTitle(routeId, args, source)) {
      _topBarController.clear();
    }
    _mainLayoutController.clear();
    _routerState.navigate(
      routeId: routeId,
      args: args,
      source: source,
      routeSpec: routeSpec,
    );
  }

  void _resetToRoute(
    String routeId,
    Map<String, Object?> args,
    RouteEntrySource source,
  ) {
    if (!_navigationModel.routesById.containsKey(routeId)) {
      throw StateError('Unknown routeId: $routeId');
    }
    _isNavigatingBack = false;
    if (!_shouldPreserveTopBarTitle(routeId, args, source)) {
      _topBarController.clear();
    }
    _mainLayoutController.clear();
    _routerState.resetTo(
      RouteEntry(routeId: routeId, args: args, source: source),
    );
  }

  bool _shouldPreserveTopBarTitle(
    String nextRouteId,
    Map<String, Object?> nextArgs,
    RouteEntrySource nextSource,
  ) {
    final currentScreen = AppRouteCatalog.resolveScreen(
      _navigationModel,
      _routerState.currentEntry,
    );
    final nextScreen = AppRouteCatalog.resolveScreen(
      _navigationModel,
      RouteEntry(routeId: nextRouteId, args: nextArgs, source: nextSource),
    );
    return currentScreen.preserveTopBarTitleWhenReplacingWith(nextScreen);
  }

  Future<void> _refreshToolPkgNavigationModel() async {
    final useEnglish = _useEnglishForToolPkgText(context);
    final packageManager = _clients.permissionsPackToolPackageManager;
    final results = await Future.wait<Object>(<Future<Object>>[
      packageManager.getToolPkgUiRoutes(
        runtime: 'compose_dsl',
        useEnglish: useEnglish,
      ),
      packageManager.getToolPkgNavigationEntries(useEnglish: useEnglish),
    ]);
    if (!mounted) {
      return;
    }
    setState(() {
      _toolPkgUiRoutes = results[0] as List<core_proxy.ToolPkgUiRoute>;
      _toolPkgNavigationEntries =
          results[1] as List<core_proxy.ToolPkgNavigationEntry>;
      _navigationModel = AppRouteCatalog.build(
        context,
        toolPkgUiRoutes: _toolPkgUiRoutes,
        toolPkgNavigationEntries: _toolPkgNavigationEntries,
      );
      AppRouteDiscoveryGateway.install(() => _navigationModel.routes);
    });
  }

  void _navigateToNavigationEntry(NavigationEntrySpec entry) {
    final action = entry.action;
    if (action != null) {
      final ownerPackageName = entry.ownerPackageName;
      if (ownerPackageName == null) {
        return;
      }
      unawaited(
        _runToolPkgNavigationEntryAction(
          entry: entry,
          action: action,
          ownerPackageName: ownerPackageName,
        ),
      );
      return;
    }
    final currentRouteEntry = _routerState.currentEntry;
    if (currentRouteEntry.routeId == entry.routeId &&
        mapEquals(currentRouteEntry.args, entry.routeArgs)) {
      return;
    }
    setState(() {
      _drawerOpen = false;
      _isNavigatingBack = false;
    });
    _resetToRoute(entry.routeId, entry.routeArgs, RouteEntrySource.drawer);
  }

  Future<void> _runToolPkgNavigationEntryAction({
    required NavigationEntrySpec entry,
    required NavigationEntryActionSpec action,
    required String ownerPackageName,
  }) async {
    try {
      await _clients.permissionsPackToolPackageManager
          .runToolPkgNavigationEntryAction(
            containerPackageName: ownerPackageName,
            entryId: entry.entryId,
            functionName: action.functionName,
            inlineFunctionSource: action.functionSource,
            eventPayload: <String, Object?>{
              'entryId': entry.entryId,
              'routeId': entry.routeId,
              'surface': _toolPkgNavigationSurfaceName(entry.surface),
              'title': entry.title,
              'description': entry.description,
            },
          );
    } catch (error, stackTrace) {
      debugPrint(
        'ToolPkg navigation action failed: entryId=${entry.entryId}, '
        'package=$ownerPackageName, error=$error\n$stackTrace',
      );
    }
  }

  void _activateConversationRoute() {
    final entry = _navigationModel.navigationEntriesById['main.ai_chat'];
    if (entry == null) {
      throw StateError('Unknown navigation entry: main.ai_chat');
    }
    setState(() {
      _drawerOpen = false;
      _isNavigatingBack = false;
    });
    _resetToRoute(entry.routeId, <String, Object?>{
      'conversationActivatedAt': DateTime.now().microsecondsSinceEpoch,
    }, RouteEntrySource.drawer);
  }

  void _goBack() {
    _isNavigatingBack = true;
    _topBarController.clear();
    _mainLayoutController.clear();
    _routerState.pop();
  }

  void _resetToConversationFromBack() {
    final entry = _navigationModel.navigationEntriesById['main.ai_chat'];
    if (entry == null) {
      throw StateError('Unknown navigation entry: main.ai_chat');
    }
    _isNavigatingBack = true;
    _topBarController.clear();
    _mainLayoutController.clear();
    _routerState.resetTo(
      RouteEntry(
        routeId: entry.routeId,
        args: <String, Object?>{
          'conversationActivatedAt': DateTime.now().microsecondsSinceEpoch,
        },
        source: RouteEntrySource.defaultSource,
      ),
    );
  }

  void _handleExitBackPress() {
    final currentTime = DateTime.now().millisecondsSinceEpoch;
    if (currentTime - _backPressedTime > _backPressedIntervalMs) {
      _backPressedTime = currentTime;
      final messenger = ScaffoldMessenger.of(context);
      messenger.hideCurrentSnackBar();
      messenger.showSnackBar(
        const SnackBar(
          content: Text('再按一次退出应用'),
          duration: Duration(milliseconds: _backPressedIntervalMs),
          behavior: SnackBarBehavior.floating,
        ),
      );
    } else {
      SystemNavigator.pop();
    }
  }

  void _handleSystemBack(OperitScreen currentScreen) {
    if (_drawerOpen) {
      setState(() {
        _drawerOpen = false;
      });
      return;
    }
    if (_routerState.canPop) {
      _goBack();
      return;
    }
    if (currentScreen is! AiChatScreenRoute) {
      _resetToConversationFromBack();
      return;
    }
    _handleExitBackPress();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _routerState,
      builder: (context, _) {
        final currentRouteEntry = _routerState.currentEntry;
        final currentScreen = AppRouteCatalog.resolveScreen(
          _navigationModel,
          currentRouteEntry,
        );
        final pluginSidebarEntries = _navigationModel.navigationEntries
            .where(
              (entry) => entry.surface == NavigationSurface.mainSidebarPlugins,
            )
            .toList(growable: false);
        final currentRouteTitle =
            _navigationModel.routesById[currentRouteEntry.routeId]!.title ??
            currentScreen.title ??
            '';
        final mediaQuery = MediaQuery.of(context);
        final useTabletLayout = useTabletLayoutForWidth(mediaQuery.size.width);
        final content = AppContent(
          routerState: _routerState,
          currentScreen: currentScreen,
          currentRouteEntry: currentRouteEntry,
          currentRouteTitle: currentRouteTitle,
          useTabletLayout: useTabletLayout,
          isTabletSidebarExpanded: _isTabletSidebarExpanded,
          canGoBack: _routerState.canPop,
          enableNavigationAnimation: true,
          isNavigatingBack: _isNavigatingBack,
          topBarController: _topBarController,
          onGoBack: _goBack,
          onNavigationButtonPressed: () {
            if (useTabletLayout) {
              setState(() {
                _isTabletSidebarExpanded = !_isTabletSidebarExpanded;
              });
            } else {
              setState(() {
                _drawerOpen = true;
              });
            }
          },
        );

        return MainLayoutScope(
          controller: _mainLayoutController,
          child: TopBarScope(
            controller: _topBarController,
            child: PopScope(
              canPop:
                  defaultTargetPlatform != TargetPlatform.android &&
                  !_drawerOpen &&
                  !_routerState.canPop,
              onPopInvokedWithResult: (didPop, result) {
                if (didPop) {
                  return;
                }
                _handleSystemBack(currentScreen);
              },
              child: Scaffold(
                body: useTabletLayout
                    ? TabletLayout(
                        content: content,
                        navigationEntries: _navigationModel.navigationEntries,
                        pluginSidebarEntries: pluginSidebarEntries,
                        selectedRouteId: currentRouteEntry.routeId,
                        isTabletSidebarExpanded: _isTabletSidebarExpanded,
                        tabletSidebarWidth: 280,
                        collapsedTabletSidebarWidth: 56,
                        onNavigationEntrySelected: _navigateToNavigationEntry,
                        onConversationActivated: _activateConversationRoute,
                      )
                    : PhoneLayout(
                        content: content,
                        navigationEntries: _navigationModel.navigationEntries,
                        pluginSidebarEntries: pluginSidebarEntries,
                        selectedRouteId: currentRouteEntry.routeId,
                        drawerWidth: mediaQuery.size.width * 0.75,
                        drawerOpen: _drawerOpen,
                        enableNavigationAnimation: true,
                        onOpenDrawer: () {
                          _refreshToolPkgNavigationModel();
                          setState(() {
                            _drawerOpen = true;
                          });
                        },
                        onCloseDrawer: () {
                          setState(() {
                            _drawerOpen = false;
                          });
                        },
                        onNavigationEntrySelected: _navigateToNavigationEntry,
                        onConversationActivated: _activateConversationRoute,
                      ),
              ),
            ),
          ),
        );
      },
    );
  }
}

bool _useEnglishForToolPkgText(BuildContext context) {
  return Localizations.localeOf(context).languageCode.toLowerCase() != 'zh';
}

String _toolPkgNavigationSurfaceName(NavigationSurface surface) {
  return switch (surface) {
    NavigationSurface.mainSidebarAi => 'main_sidebar_ai',
    NavigationSurface.mainSidebarTools => 'main_sidebar_tools',
    NavigationSurface.mainSidebarPlugins => 'main_sidebar_plugins',
    NavigationSurface.mainSidebarSystem => 'main_sidebar_system',
    NavigationSurface.toolbox => 'toolbox',
  };
}
