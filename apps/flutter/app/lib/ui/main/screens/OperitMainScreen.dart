// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/notifications/NotificationActivationService.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../features/chat/viewmodel/ChatSwitchRenderCoordinator.dart';
import '../components/AppContent.dart';
import '../components/DrawerConversationState.dart';
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
  late final ValueNotifier<DrawerConversationState> _drawerConversationState;
  StreamSubscription<List<core_proxy.ChatHistoryListItem>>?
  _drawerHistoriesSubscription;
  StreamSubscription<String?>? _drawerCurrentChatSubscription;
  StreamSubscription<List<core_proxy.CharacterGroupCard>>?
  _drawerCharacterGroupsSubscription;
  StreamSubscription<List<String>>? _drawerCharacterCardIdsSubscription;
  final Map<String, StreamSubscription<core_proxy.CharacterCard>>
  _drawerCharacterCardSubscriptions =
      <String, StreamSubscription<core_proxy.CharacterCard>>{};
  final Map<String, String> _drawerCharacterCardNamesById = <String, String>{};
  late final ValueNotifier<bool> _drawerOpenState;
  bool _isTabletSidebarExpanded = false;
  bool _isNavigatingBack = false;
  bool _requestedInitialToolPkgNavigationRefresh = false;
  int _backPressedTime = 0;

  /// Initializes navigation services and drawer data subscriptions.
  @override
  void initState() {
    super.initState();
    _topBarController = TopBarController();
    _mainLayoutController = MainLayoutController();
    _routerState = AppRouterState(AppRouteCatalog.initialEntry());
    _drawerConversationState = ValueNotifier<DrawerConversationState>(
      const DrawerConversationState(),
    );
    _drawerOpenState = ValueNotifier<bool>(false);
    AppRouterGateway.install(handler: _navigateToRoute, reset: _resetToRoute);
    NotificationActivationService.instance.installChatHandler(
      _activateNotificationChat,
    );
    unawaited(_initializeDrawerData());
  }

  /// Loads drawer data before subscribing to its live updates.
  Future<void> _initializeDrawerData() async {
    await _loadDrawerConversations();
    if (!mounted) {
      return;
    }
    _watchDrawerConversations();
    _watchDrawerCharacterGroups();
    _watchDrawerCharacterCards();
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
    NotificationActivationService.instance.clearChatHandler();
    _drawerHistoriesSubscription?.cancel();
    _drawerCurrentChatSubscription?.cancel();
    _drawerCharacterGroupsSubscription?.cancel();
    _drawerCharacterCardIdsSubscription?.cancel();
    for (final subscription in _drawerCharacterCardSubscriptions.values) {
      subscription.cancel();
    }
    _drawerConversationState.dispose();
    _drawerOpenState.dispose();
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
    });
  }

  /// Loads drawer conversations and their character avatar metadata.
  Future<void> _loadDrawerConversations() async {
    final currentState = _drawerConversationState.value;
    _drawerConversationState.value = DrawerConversationState(
      histories: currentState.histories,
      characterGroupNamesById: currentState.characterGroupNamesById,
      characterCardAvatarUrisByName: currentState.characterCardAvatarUrisByName,
      currentChatId: currentState.currentChatId,
      loading: true,
    );
    try {
      final characterGroupCoreProxy =
          _clients.preferencesCharacterGroupCardManager;
      final characterCardCoreProxy = _clients.preferencesCharacterCardManager;
      await Future.wait<void>(<Future<void>>[
        characterGroupCoreProxy.initializeIfNeeded(),
        characterCardCoreProxy.initializeIfNeeded(),
      ]);
      final results = await Future.wait<Object?>(<Future<Object?>>[
        _clients.chatRuntimeHolderMain.chatHistoryListItemsFlowSnapshot(),
        _clients.chatRuntimeHolderMain.currentChatIdFlowSnapshot(),
        characterGroupCoreProxy.allCharacterGroupCardsFlowSnapshot(),
        characterCardCoreProxy.getAllCharacterCards(),
      ]);
      final histories = results[0] as List<core_proxy.ChatHistoryListItem>;
      final currentChatId = results[1] as String?;
      final characterGroups = results[2] as List<core_proxy.CharacterGroupCard>;
      final characterCards = results[3] as List<core_proxy.CharacterCard>;
      if (!mounted) {
        return;
      }
      _replaceDrawerCharacterCardAvatars(characterCards);
      _drawerConversationState.value = DrawerConversationState(
        histories: List<core_proxy.ChatHistoryListItem>.unmodifiable(histories),
        characterGroupNamesById: _characterGroupNameMap(characterGroups),
        characterCardAvatarUrisByName:
            _drawerConversationState.value.characterCardAvatarUrisByName,
        currentChatId: currentChatId,
        loading: false,
      );
      await _syncDrawerCharacterCardSubscriptions(
        characterCards.map((card) => card.id).toList(growable: false),
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to load drawer conversations: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      final state = _drawerConversationState.value;
      _drawerConversationState.value = DrawerConversationState(
        histories: state.histories,
        characterGroupNamesById: state.characterGroupNamesById,
        characterCardAvatarUrisByName: state.characterCardAvatarUrisByName,
        currentChatId: state.currentChatId,
        errorMessage: error.toString(),
        loading: false,
      );
    }
  }

  void _watchDrawerConversations() {
    _drawerHistoriesSubscription?.cancel();
    _drawerHistoriesSubscription = _clients.chatRuntimeHolderMain
        .chatHistoryListItemsFlowChanges()
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
              characterGroupNamesById: state.characterGroupNamesById,
              characterCardAvatarUrisByName:
                  state.characterCardAvatarUrisByName,
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
              characterGroupNamesById: state.characterGroupNamesById,
              characterCardAvatarUrisByName:
                  state.characterCardAvatarUrisByName,
              currentChatId: state.currentChatId,
              errorMessage: error.toString(),
              loading: false,
            );
          },
        );

    _drawerCurrentChatSubscription?.cancel();
    _drawerCurrentChatSubscription = _clients.chatRuntimeHolderMain
        .currentChatIdFlowChanges()
        .listen(
          (chatId) {
            if (!mounted) {
              return;
            }
            final state = _drawerConversationState.value;
            _drawerConversationState.value = DrawerConversationState(
              histories: state.histories,
              characterGroupNamesById: state.characterGroupNamesById,
              characterCardAvatarUrisByName:
                  state.characterCardAvatarUrisByName,
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
              characterGroupNamesById: state.characterGroupNamesById,
              characterCardAvatarUrisByName:
                  state.characterCardAvatarUrisByName,
              currentChatId: state.currentChatId,
              errorMessage: error.toString(),
              loading: state.loading,
            );
          },
        );
  }

  void _watchDrawerCharacterGroups() {
    _drawerCharacterGroupsSubscription?.cancel();
    _drawerCharacterGroupsSubscription = _clients
        .preferencesCharacterGroupCardManager
        .allCharacterGroupCardsFlowChanges()
        .listen(
          (groups) {
            if (!mounted) {
              return;
            }
            final state = _drawerConversationState.value;
            _drawerConversationState.value = DrawerConversationState(
              histories: state.histories,
              characterGroupNamesById: _characterGroupNameMap(groups),
              characterCardAvatarUrisByName:
                  state.characterCardAvatarUrisByName,
              currentChatId: state.currentChatId,
              errorMessage: state.errorMessage,
              loading: state.loading,
            );
          },
          onError: (Object error, StackTrace stackTrace) {
            debugPrint(
              'Failed to watch drawer character groups: $error\n$stackTrace',
            );
            if (!mounted) {
              return;
            }
            final state = _drawerConversationState.value;
            _drawerConversationState.value = DrawerConversationState(
              histories: state.histories,
              characterGroupNamesById: state.characterGroupNamesById,
              characterCardAvatarUrisByName:
                  state.characterCardAvatarUrisByName,
              currentChatId: state.currentChatId,
              errorMessage: error.toString(),
              loading: state.loading,
            );
          },
        );
  }

  /// Watches the character-card collection used to supply drawer avatars.
  void _watchDrawerCharacterCards() {
    final characterCardCoreProxy = _clients.preferencesCharacterCardManager;
    _drawerCharacterCardIdsSubscription?.cancel();
    _drawerCharacterCardIdsSubscription = characterCardCoreProxy
        .characterCardListFlowChanges()
        .listen(
          (cardIds) {
            unawaited(_syncDrawerCharacterCardSubscriptions(cardIds));
          },
          onError: (Object error, StackTrace stackTrace) {
            _reportDrawerCharacterCardError(error, stackTrace);
          },
        );
  }

  /// Synchronizes per-card avatar subscriptions with the current card list.
  Future<void> _syncDrawerCharacterCardSubscriptions(
    List<String> cardIds,
  ) async {
    try {
      final desiredCardIds = cardIds
          .map((id) => id.trim())
          .where((id) => id.isNotEmpty)
          .toSet();
      final removedCardIds = _drawerCharacterCardSubscriptions.keys
          .where((id) => !desiredCardIds.contains(id))
          .toList(growable: false);
      for (final cardId in removedCardIds) {
        await _drawerCharacterCardSubscriptions.remove(cardId)!.cancel();
        final previousName = _drawerCharacterCardNamesById.remove(cardId);
        if (previousName != null) {
          _removeDrawerCharacterCardAvatar(previousName);
        }
      }

      final characterCardCoreProxy = _clients.preferencesCharacterCardManager;
      for (final cardId in desiredCardIds) {
        if (_drawerCharacterCardSubscriptions.containsKey(cardId)) {
          continue;
        }
        _drawerCharacterCardSubscriptions[cardId] = characterCardCoreProxy
            .getCharacterCardFlowChanges(id: cardId)
            .listen(
              _updateDrawerCharacterCardAvatar,
              onError: (Object error, StackTrace stackTrace) {
                _reportDrawerCharacterCardError(error, stackTrace);
              },
            );
      }

      final cards = await Future.wait<core_proxy.CharacterCard>(
        desiredCardIds.map(
          (id) => characterCardCoreProxy.getCharacterCardFlowSnapshot(id: id),
        ),
      );
      if (!mounted) {
        return;
      }
      _replaceDrawerCharacterCardAvatars(cards);
    } catch (error, stackTrace) {
      _reportDrawerCharacterCardError(error, stackTrace);
    }
  }

  /// Replaces all drawer avatar metadata with the supplied character cards.
  void _replaceDrawerCharacterCardAvatars(
    List<core_proxy.CharacterCard> cards,
  ) {
    final avatarUrisByName = <String, String>{};
    _drawerCharacterCardNamesById.clear();
    for (final card in cards) {
      final id = card.id.trim();
      final name = card.name.trim();
      if (id.isEmpty || name.isEmpty) {
        continue;
      }
      _drawerCharacterCardNamesById[id] = name;
      final avatarUri = card.avatarUri?.trim();
      if (avatarUri != null && avatarUri.isNotEmpty) {
        avatarUrisByName[name] = avatarUri;
      }
    }
    final state = _drawerConversationState.value;
    _drawerConversationState.value = DrawerConversationState(
      histories: state.histories,
      characterGroupNamesById: state.characterGroupNamesById,
      characterCardAvatarUrisByName: avatarUrisByName,
      currentChatId: state.currentChatId,
      errorMessage: state.errorMessage,
      loading: state.loading,
    );
  }

  /// Applies a single character-card avatar update to drawer state.
  void _updateDrawerCharacterCardAvatar(core_proxy.CharacterCard card) {
    if (!mounted) {
      return;
    }
    final cardId = card.id.trim();
    final cardName = card.name.trim();
    if (cardId.isEmpty || cardName.isEmpty) {
      return;
    }
    final avatarUrisByName = Map<String, String>.of(
      _drawerConversationState.value.characterCardAvatarUrisByName,
    );
    final previousName = _drawerCharacterCardNamesById[cardId];
    if (previousName != null) {
      avatarUrisByName.remove(previousName);
    }
    _drawerCharacterCardNamesById[cardId] = cardName;
    final avatarUri = card.avatarUri?.trim();
    if (avatarUri != null && avatarUri.isNotEmpty) {
      avatarUrisByName[cardName] = avatarUri;
    }
    final state = _drawerConversationState.value;
    _drawerConversationState.value = DrawerConversationState(
      histories: state.histories,
      characterGroupNamesById: state.characterGroupNamesById,
      characterCardAvatarUrisByName: avatarUrisByName,
      currentChatId: state.currentChatId,
      errorMessage: state.errorMessage,
      loading: state.loading,
    );
  }

  /// Removes a deleted character-card avatar from drawer state.
  void _removeDrawerCharacterCardAvatar(String cardName) {
    if (!mounted) {
      return;
    }
    final avatarUrisByName = Map<String, String>.of(
      _drawerConversationState.value.characterCardAvatarUrisByName,
    )..remove(cardName);
    final state = _drawerConversationState.value;
    _drawerConversationState.value = DrawerConversationState(
      histories: state.histories,
      characterGroupNamesById: state.characterGroupNamesById,
      characterCardAvatarUrisByName: avatarUrisByName,
      currentChatId: state.currentChatId,
      errorMessage: state.errorMessage,
      loading: state.loading,
    );
  }

  /// Publishes a character avatar loading error to the drawer state.
  void _reportDrawerCharacterCardError(Object error, StackTrace stackTrace) {
    debugPrint('Failed to watch drawer character cards: $error\n$stackTrace');
    if (!mounted) {
      return;
    }
    final state = _drawerConversationState.value;
    _drawerConversationState.value = DrawerConversationState(
      histories: state.histories,
      characterGroupNamesById: state.characterGroupNamesById,
      characterCardAvatarUrisByName: state.characterCardAvatarUrisByName,
      currentChatId: state.currentChatId,
      errorMessage: error.toString(),
      loading: false,
    );
  }

  Map<String, String> _characterGroupNameMap(
    List<core_proxy.CharacterGroupCard> groups,
  ) {
    return <String, String>{
      for (final group in groups)
        if (group.id.trim().isNotEmpty && group.name.trim().isNotEmpty)
          group.id.trim(): group.name.trim(),
    };
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
    ChatSwitchRenderCoordinator.prepareForChat(chatId);
    _activateConversationRoute();
    await WidgetsBinding.instance.endOfFrame;
    if (!mounted) {
      return;
    }
    await _clients.chatRuntimeHolderMain.switchChat(chatId: chatId);
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
        final appBarEntries = _navigationModel.navigationEntries
            .where((entry) => entry.surface == NavigationSurface.appBar)
            .toList(growable: false);
        final currentRouteTitle =
            _navigationModel.routesById[currentRouteEntry.routeId]!.title ??
            currentScreen.title ??
            '';
        final screenSize = MediaQuery.sizeOf(context);
        final useTabletLayout = useTabletLayoutForWidth(screenSize.width);
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
        );

        return MainLayoutScope(
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
