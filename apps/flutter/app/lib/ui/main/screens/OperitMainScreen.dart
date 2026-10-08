// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:desktop_widgets/desktop_widgets.dart';

import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/notifications/NotificationActivationService.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../features/chat/viewmodel/ChatSelectionTransition.dart';
import '../components/AppContent.dart';
import '../components/DrawerConversationState.dart';
import '../layout/NavigationLayoutMetrics.dart';
import '../layout/PhoneLayout.dart';
import '../MainLayoutController.dart';
import '../TopBarController.dart';
import '../layout/TabletLayout.dart';
import '../layout/SidebarDockController.dart';
import '../navigation/AppNavigationModels.dart';
import '../navigation/AppRouteCatalog.dart';
import '../navigation/ToolPkgCatalogChangeBus.dart';
import 'OperitScreens.dart';
import '../../features/packages/screens/ToolPkgUiLauncherScreen.dart';

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
  late final SidebarDockController _sidebarDockController;
  List<core_proxy.ToolPkgUiRoute> _toolPkgUiRoutes =
      const <core_proxy.ToolPkgUiRoute>[];
  List<core_proxy.ToolPkgNavigationEntry> _toolPkgNavigationEntries =
      const <core_proxy.ToolPkgNavigationEntry>[];
  late final ValueNotifier<DrawerConversationState> _drawerConversationState;
  StreamSubscription<List<core_proxy.ChatHistoryListItem>>?
  _drawerHistoriesSubscription;
  StreamSubscription<String?>? _drawerCurrentChatSubscription;
  StreamSubscription<List<String>>? _drawerActiveStreamingChatIdsSubscription;
  late final ValueNotifier<bool> _drawerOpenState;
  bool _isTabletSidebarExpanded = false;
  bool _isNavigatingBack = false;
  bool _requestedInitialToolPkgNavigationRefresh = false;
  StreamSubscription<void>? _toolPkgCatalogChangeSubscription;
  int _backPressedTime = 0;

  /// Initializes navigation services and drawer data subscriptions.
  @override
  void initState() {
    super.initState();
    _topBarController = TopBarController();
    _mainLayoutController = MainLayoutController();
    _sidebarDockController = SidebarDockController();
    _routerState = AppRouterState(AppRouteCatalog.initialEntry());
    _drawerConversationState = ValueNotifier<DrawerConversationState>(
      const DrawerConversationState(),
    );
    _drawerOpenState = ValueNotifier<bool>(false);
    AppRouterGateway.install(handler: _navigateToRoute, reset: _resetToRoute);
    NotificationActivationService.instance.installChatHandler(
      _activateNotificationChat,
    );
    _toolPkgCatalogChangeSubscription = ToolPkgCatalogChangeBus.listen(() {
      if (mounted) {
        unawaited(_refreshToolPkgNavigationModel());
      }
    });
    unawaited(_loadSidebarDockPreferences());
    unawaited(_initializeDrawerData());
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        unawaited(_initializeDesktopWidgets());
      }
    });
  }

  /// Supplies application content and navigation callbacks to the unified widget plugin.
  Future<void> _initializeDesktopWidgets() async {
    try {
      await DesktopWidgets.setActionHandler(_handleDesktopWidgetAction);
      if (!mounted) return;
      await DesktopWidgets.setSnapshotRenderer((payload, size) async {
        final instanceId = payload['instanceId'] as String;
        final frame = await ToolPkgDesktopWidgetFrame.load(
          clients: _clients,
          definition: core_proxy.ToolPkgDesktopWidget.fromJson(
            (payload['definition'] as Map).cast<String, Object?>(),
          ),
          instanceId: instanceId,
          useEnglish: Localizations.localeOf(context).languageCode == 'en',
        );
        if (!mounted) throw StateError('The widget content owner is closed');
        final png = await DesktopWidgetRasterizer.render(
          context: context,
          size: size,
          child: frame.buildContent(
            clients: _clients,
            instanceId: instanceId,
            onOpenRoute: (package, route) async {
              await _handleDesktopWidgetAction(
                MethodCall('openDesktopWidgetRoute', {
                  'packageName': package,
                  'routeId': route,
                }),
              );
            },
          ),
        );
        return DesktopWidgetSnapshot(
          png: png,
          description: frame.definition.title,
          action: 'openDesktopWidgetRoute',
          actionArguments: {
            'packageName': frame.definition.containerPackageName,
            'routeId': frame.definition.routeId,
          },
        );
      });
    } catch (error) {
      if (!mounted) rethrow;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(error.toString())));
    }
  }

  /// Resolves widget clicks through the existing route catalog and navigation gateway.
  Future<Object?> _handleDesktopWidgetAction(MethodCall call) async {
    try {
      if (call.method != 'openDesktopWidgetRoute') {
        throw UnsupportedError('Unknown widget action: ${call.method}');
      }
      if (!mounted) throw StateError('The widget navigation owner is closed');
      final args = (call.arguments as Map).cast<String, Object?>();
      final package = args['packageName'] as String;
      final route = args['routeId'] as String;
      await _refreshToolPkgNavigationModel();
      if (!mounted) throw StateError('The widget navigation owner is closed');
      final spec = _navigationModel.routesById[route];
      if (spec == null ||
          spec.ownerPackageName != package ||
          spec.runtime != RouteRuntime.toolPkgComposeDsl) {
        throw StateError('Widget route is not registered for $package: $route');
      }
      Navigator.of(context).popUntil((route) => route.isFirst);
      AppRouterGateway.navigate(routeId: route);
      return null;
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(error.toString())));
      }
      rethrow;
    }
  }

  /// Loads drawer data before subscribing to its live updates.
  Future<void> _initializeDrawerData() async {
    await _loadDrawerConversations();
    if (!mounted) {
      return;
    }
    _watchDrawerConversations();
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
    _synchronizeSidebarDock();
    if (!_requestedInitialToolPkgNavigationRefresh) {
      _requestedInitialToolPkgNavigationRefresh = true;
      _refreshToolPkgNavigationModel();
    }
  }

  @override
  void dispose() {
    unawaited(DesktopWidgets.setActionHandler(null));
    unawaited(DesktopWidgets.setSnapshotRenderer(null));
    AppRouterGateway.clear();
    AppRouteDiscoveryGateway.clear();
    NotificationActivationService.instance.clearChatHandler();
    _toolPkgCatalogChangeSubscription?.cancel();
    _drawerHistoriesSubscription?.cancel();
    _drawerCurrentChatSubscription?.cancel();
    _drawerActiveStreamingChatIdsSubscription?.cancel();
    _drawerConversationState.dispose();
    _drawerOpenState.dispose();
    _routerState.dispose();
    _topBarController.dispose();
    _mainLayoutController.dispose();
    _sidebarDockController.dispose();
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
    final packageManager = _clients.application.packageManager();
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
      _synchronizeSidebarDock();
    });
  }

  /// Synchronizes dock placement with the current navigation catalog.
  void _synchronizeSidebarDock() {
    _sidebarDockController.synchronize(
      pluginEntries: _navigationModel.navigationEntries
          .where(
            (entry) => entry.surface == NavigationSurface.mainSidebarPlugins,
          )
          .toList(growable: false),
      routesById: _navigationModel.routesById,
    );
  }

  /// Loads the persisted sidebar dock layout through the Flutter preference layer.
  Future<void> _loadSidebarDockPreferences() async {
    try {
      await _sidebarDockController.loadPreferences();
    } catch (error, stackTrace) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stackTrace,
          library: 'sidebar dock persistence',
          context: ErrorDescription('while loading sidebar dock layout'),
        ),
      );
    }
  }

  /// Loads only canonical conversations and generic runtime selection state.
  Future<void> _loadDrawerConversations() async {
    final currentState = _drawerConversationState.value;
    _drawerConversationState.value = DrawerConversationState(
      histories: currentState.histories,
      activeStreamingChatIds: currentState.activeStreamingChatIds,
      currentChatId: currentState.currentChatId,
      loading: true,
    );
    try {
      final results = await Future.wait<Object?>(<Future<Object?>>[
        _clients.chatRuntimeHolderMain.chatHistoryListItemsFlow().first,
        _clients.chatRuntimeHolderMain.currentChatIdFlow().first,
        _clients.chatRuntimeHolderMain.activeStreamingChatIds(),
      ]);
      final histories = results[0] as List<core_proxy.ChatHistoryListItem>;
      final currentChatId = results[1] as String?;
      final activeStreamingChatIds = results[2] as List<String>;
      if (!mounted) {
        return;
      }
      _drawerConversationState.value = DrawerConversationState(
        histories: List<core_proxy.ChatHistoryListItem>.unmodifiable(histories),
        activeStreamingChatIds: Set<String>.unmodifiable(
          activeStreamingChatIds,
        ),
        currentChatId: currentChatId,
        loading: false,
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to load drawer conversations: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      final state = _drawerConversationState.value;
      _drawerConversationState.value = DrawerConversationState(
        histories: state.histories,
        activeStreamingChatIds: state.activeStreamingChatIds,
        currentChatId: state.currentChatId,
        errorMessage: error.toString(),
        loading: false,
      );
    }
  }

  void _watchDrawerConversations() {
    _drawerHistoriesSubscription?.cancel();
    _drawerHistoriesSubscription = _clients.chatRuntimeHolderMain
        .chatHistoryListItemsFlow()
        .listen(
          (histories) {
            if (!mounted) {
              return;
            }
            final state = _drawerConversationState.value;
            _drawerConversationState.value = DrawerConversationState(
              histories: List<core_proxy.ChatHistoryListItem>.unmodifiable(
                histories,
              ),
              activeStreamingChatIds: state.activeStreamingChatIds,
              currentChatId: state.currentChatId,
              loading: false,
            );
          },
          onError: (Object error, StackTrace stackTrace) {
            debugPrint(
              'Failed to watch drawer conversations: $error\n$stackTrace',
            );
            if (!mounted) {
              return;
            }
            final state = _drawerConversationState.value;
            _drawerConversationState.value = DrawerConversationState(
              histories: state.histories,
              activeStreamingChatIds: state.activeStreamingChatIds,
              currentChatId: state.currentChatId,
              errorMessage: error.toString(),
              loading: false,
            );
          },
        );

    _drawerCurrentChatSubscription?.cancel();
    _drawerCurrentChatSubscription = _clients.chatRuntimeHolderMain
        .currentChatIdFlow()
        .listen(
          (chatId) {
            if (!mounted) {
              return;
            }
            final state = _drawerConversationState.value;
            _drawerConversationState.value = DrawerConversationState(
              histories: state.histories,
              activeStreamingChatIds: state.activeStreamingChatIds,
              currentChatId: chatId,
              errorMessage: state.errorMessage,
              loading: state.loading,
            );
          },
          onError: (Object error, StackTrace stackTrace) {
            debugPrint(
              'Failed to watch drawer current chat id: $error\n$stackTrace',
            );
            if (!mounted) {
              return;
            }
            final state = _drawerConversationState.value;
            _drawerConversationState.value = DrawerConversationState(
              histories: state.histories,
              activeStreamingChatIds: state.activeStreamingChatIds,
              currentChatId: state.currentChatId,
              errorMessage: error.toString(),
              loading: state.loading,
            );
          },
        );

    _drawerActiveStreamingChatIdsSubscription?.cancel();
    _drawerActiveStreamingChatIdsSubscription = _clients.chatRuntimeHolderMain
        .activeStreamingChatIdsFlow()
        .listen(
          (activeChatIds) {
            if (!mounted) {
              return;
            }
            final state = _drawerConversationState.value;
            _drawerConversationState.value = DrawerConversationState(
              histories: state.histories,
              activeStreamingChatIds: Set<String>.unmodifiable(activeChatIds),
              currentChatId: state.currentChatId,
              errorMessage: state.errorMessage,
              loading: state.loading,
            );
          },
          onError: (Object error, StackTrace stackTrace) {
            debugPrint(
              'Failed to watch drawer active chat ids: $error\n$stackTrace',
            );
            if (!mounted) {
              return;
            }
            final state = _drawerConversationState.value;
            _drawerConversationState.value = DrawerConversationState(
              histories: state.histories,
              activeStreamingChatIds: state.activeStreamingChatIds,
              currentChatId: state.currentChatId,
              errorMessage: error.toString(),
              loading: state.loading,
            );
          },
        );
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
    _drawerOpenState.value = false;
    _isNavigatingBack = false;
    _resetToRoute(entry.routeId, entry.routeArgs, RouteEntrySource.drawer);
  }

  Future<void> _runToolPkgNavigationEntryAction({
    required NavigationEntrySpec entry,
    required NavigationEntryActionSpec action,
    required String ownerPackageName,
  }) async {
    try {
      await _clients.application
          .packageManager()
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
    _drawerOpenState.value = false;
    _isNavigatingBack = false;
    if (_routerState.currentEntry.routeId == entry.routeId) {
      return;
    }
    _resetToRoute(entry.routeId, <String, Object?>{
      'conversationActivatedAt': DateTime.now().microsecondsSinceEpoch,
    }, RouteEntrySource.drawer);
  }

  /// Opens the target chat selected by a system notification activation.
  Future<void> _activateNotificationChat(String chatId) async {
    ChatSelectionTransition.begin(chatId);
    _activateConversationRoute();
    await WidgetsBinding.instance.endOfFrame;
    if (!mounted) {
      return;
    }
    try {
      await _clients.chatRuntimeHolderMain.switchChat(chatId: chatId);
    } catch (_) {
      ChatSelectionTransition.complete(chatId);
      rethrow;
    }
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

  /// Terminates the Android process after the exit gesture is confirmed.
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
      const MethodChannel(
        'operit/runtime',
      ).invokeMethod<void>('terminateApplication');
    }
  }

  void _handleSystemBack(OperitScreen currentScreen) {
    if (_drawerOpenState.value) {
      _drawerOpenState.value = false;
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
      animation: Listenable.merge(<Listenable>[
        _routerState,
        _sidebarDockController,
      ]),
      builder: (context, _) {
        final currentRouteEntry = _routerState.currentEntry;
        final currentScreen = AppRouteCatalog.resolveScreen(
          _navigationModel,
          currentRouteEntry,
        );
        final pluginSidebarEntries = _sidebarDockController.primaryEntries;
        final appBarEntries = _navigationModel.navigationEntries
            .where((entry) => entry.surface == NavigationSurface.appBar)
            .toList(growable: false);
        final currentRouteTitle =
            _navigationModel.routesById[currentRouteEntry.routeId]!.title ??
            currentScreen.title ??
            '';
        final screenSize = MediaQuery.sizeOf(context);
        final useTabletLayout = useTabletLayoutForWidth(screenSize.width);
        final content = Stack(
          children: <Widget>[
            AppContent(
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
              appBarEntries: appBarEntries,
              onGoBack: _goBack,
              onNavigationButtonPressed: () {
                if (useTabletLayout) {
                  setState(() {
                    _isTabletSidebarExpanded = !_isTabletSidebarExpanded;
                  });
                } else {
                  _drawerOpenState.value = true;
                }
              },
              onAppBarEntrySelected: _navigateToNavigationEntry,
            ),
          ],
        );

        return SidebarDockScope(
          controller: _sidebarDockController,
          child: MainLayoutScope(
            controller: _mainLayoutController,
            child: TopBarScope(
              controller: _topBarController,
              child: ValueListenableBuilder<bool>(
                valueListenable: _drawerOpenState,
                child: Scaffold(
                  body: useTabletLayout
                      ? TabletLayout(
                          content: content,
                          navigationEntries: _navigationModel.navigationEntries,
                          pluginSidebarEntries: pluginSidebarEntries,
                          selectedRouteId: currentRouteEntry.routeId,
                          drawerConversationState: _drawerConversationState,
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
                          drawerConversationState: _drawerConversationState,
                          drawerWidth: screenSize.width * 0.75,
                          drawerOpenState: _drawerOpenState,
                          enableNavigationAnimation: true,
                          onOpenDrawer: () {
                            _drawerOpenState.value = true;
                          },
                          onCloseDrawer: () {
                            _drawerOpenState.value = false;
                          },
                          onNavigationEntrySelected: _navigateToNavigationEntry,
                          onConversationActivated: _activateConversationRoute,
                        ),
                ),
                builder: (context, drawerOpen, child) {
                  final phoneDrawerOpen = !useTabletLayout && drawerOpen;
                  return PopScope(
                    canPop:
                        defaultTargetPlatform != TargetPlatform.android &&
                        !phoneDrawerOpen &&
                        !_routerState.canPop,
                    onPopInvokedWithResult: (didPop, result) {
                      if (didPop) {
                        return;
                      }
                      _handleSystemBack(currentScreen);
                    },
                    child: child!,
                  );
                },
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
    NavigationSurface.appBar => 'app_bar',
  };
}
