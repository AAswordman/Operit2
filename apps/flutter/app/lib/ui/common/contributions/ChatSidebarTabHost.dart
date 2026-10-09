// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';

import '../../../core/application/PluginHotReload.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../data/preferences/UserPreferencesManager.dart';
import '../../features/packages/screens/ToolPkgUiLauncherScreen.dart';
import '../components/M3LoadingIndicator.dart';
import '../../main/navigation/ToolPkgCatalogChangeBus.dart';

/// Produces an unambiguous package-and-entry namespace without decoding plugin input.
String chatSidebarTabIdentity(String packageName, String entryId) =>
    jsonEncode(<String>[packageName, entryId]);

/// Encodes only the locked generic metadata accepted by embedded sidebar tabs.
Map<String, Object?> chatSidebarSummary(core_proxy.ChatHistoryListItem chat) =>
    <String, Object?>{
      'id': chat.id,
      'title': chat.title,
      'updatedAt': chat.updatedAt,
      'displayOrder': chat.displayOrder,
      'workspaceId': chat.workspaceId,
      'workspaceName': chat.workspaceName,
      'locked': chat.locked,
      'pinned': chat.pinned,
      'group': chat.group,
    };

/// Returns a chat activation only for the exact generic discriminator and shape.
String? chatSidebarActivation(Object? raw) {
  if (raw is! Map || raw['type'] != 'toolpkg.chat.activate') {
    return null;
  }
  final chatId = raw['chatId'];
  if (raw.length != 2 || chatId is! String || chatId.trim().isEmpty) {
    throw const FormatException(
      'Chat activation requires exactly type and a nonblank chatId.',
    );
  }
  return chatId;
}

/// Retains a real registration together with its exact owning Compose route.
class _SidebarTab {
  /// Creates catalog state without interpreting any registration parameters.
  const _SidebarTab({
    required this.entry,
    required this.route,
    required this.plugin,
  });

  final core_proxy.ToolPkgNavigationEntry entry;
  final core_proxy.ToolPkgUiRoute route;
  final core_proxy.ToolPkgContainerRuntime plugin;

  /// Namespaces tab selection independently of the plugin's chosen route ID.
  String get id =>
      chatSidebarTabIdentity(entry.containerPackageName, entry.entryId);
}

/// Embeds registered sidebar tabs next to the single native workspace tab.
class ChatSidebarTabHost extends StatefulWidget {
  /// Receives generic live chat state and the host's existing activation chain.
  const ChatSidebarTabHost({
    super.key,
    required this.clients,
    required this.chats,
    required this.currentChatId,
    required this.activeStreamingChatIds,
    required this.workspaceBuilder,
    required this.onActivateChat,
    this.headerBuilder,
    this.workspaceWithTabsBuilder,
    this.pluginFooter,
  });

  final GeneratedCoreProxyClients clients;
  final List<core_proxy.ChatHistoryListItem> chats;
  final String? currentChatId;
  final Set<String> activeStreamingChatIds;
  final WidgetBuilder workspaceBuilder;
  final Widget Function(BuildContext context, Widget tabs)? headerBuilder;

  /// Lets the built-in workspace retain its original single scrolling surface.
  final Widget Function(BuildContext context, Widget tabs)?
  workspaceWithTabsBuilder;

  /// Optional host navigation outside embedded plugin-owned content.
  final Widget? pluginFooter;
  final Future<void> Function(String chatId) onActivateChat;

  /// Creates the registry and preference observers for this live sidebar surface.
  @override
  State<ChatSidebarTabHost> createState() => _ChatSidebarTabHostState();
}

class _ChatSidebarTabHostState extends State<ChatSidebarTabHost> {
  List<_SidebarTab> _tabs = const [];
  Object? _catalogError;
  bool _catalogLoading = true;
  StreamSubscription<String>? _preferenceSubscription;
  StreamSubscription<void>? _catalogSubscription;
  String? _selectedId;
  String? _language;
  Object? _preferenceError;
  Object? _actionError;
  int _catalogGeneration = 0;
  bool _activating = false;
  final Set<String> _visitedTabs = {};

  /// Uses the shared preference backend rather than browser-local selection state.
  UserPreferencesManager get _preferences =>
      UserPreferencesManager(clients: widget.clients);

  /// Observes explicit tab selection and invalidates cached routes on catalog changes.
  @override
  void initState() {
    super.initState();
    _watchSelection();
    _catalogSubscription = ToolPkgCatalogChangeBus.listen(_reloadCatalog);
    PluginHotReload.revision.addListener(_reloadCatalog);
  }

  /// Discovers localized registrations after the host locale becomes available.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final language = Localizations.localeOf(context).languageCode;
    if (_language != language) {
      _language = language;
      _reloadCatalog();
    }
  }

  /// Rebinds registry and preference reads when the receiving runtime changes.
  @override
  void didUpdateWidget(covariant ChatSidebarTabHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.clients.bridge != widget.clients.bridge) {
      unawaited(_preferenceSubscription?.cancel());
      _selectedId = null;
      _watchSelection();
      _reloadCatalog();
    }
  }

  /// Invalidates pending callbacks before releasing surface observers.
  @override
  void dispose() {
    _catalogGeneration += 1;
    unawaited(_preferenceSubscription?.cancel());
    unawaited(_catalogSubscription?.cancel());
    PluginHotReload.revision.removeListener(_reloadCatalog);
    super.dispose();
  }

  /// Applies committed opaque tab IDs without translating old grouping preferences.
  void _watchSelection() {
    _preferenceSubscription = _preferences.chatSidebarTabFlow().listen(
      (tabId) {
        if (!mounted) return;
        setState(() {
          _selectedId = tabId;
          _preferenceError = null;
          _actionError = null;
        });
      },
      onError: (Object error, StackTrace stackTrace) {
        if (mounted) setState(() => _preferenceError = error);
      },
    );
  }

  /// Resolves every enabled registration against an exact package-owned route.
  Future<List<_SidebarTab>> _readCatalog() async {
    final manager = widget.clients.application.packageManager();
    final results = await Future.wait<Object>(<Future<Object>>[
      manager.getToolPkgNavigationEntries(useEnglish: _language == 'en'),
      manager.getToolPkgUiRoutes(
        runtime: 'compose_dsl',
        useEnglish: _language == 'en',
      ),
    ]);
    final entries = results[0] as List<core_proxy.ToolPkgNavigationEntry>;
    final routes = results[1] as List<core_proxy.ToolPkgUiRoute>;
    final ids = <String>{};
    final tabs = <_SidebarTab>[];
    for (final entry in entries) {
      if (entry.surface != 'chat_sidebar_tabs') {
        continue;
      }
      final id = chatSidebarTabIdentity(
        entry.containerPackageName,
        entry.entryId,
      );
      if (entry.containerPackageName.trim().isEmpty ||
          entry.entryId.trim().isEmpty ||
          !ids.add(id)) {
        throw StateError('Invalid or duplicate sidebar tab registration: $id');
      }
      final matches = routes.where(
        (route) =>
            route.containerPackageName == entry.containerPackageName &&
            route.routeId == entry.routeId,
      );
      if (matches.length != 1 ||
          matches.single.runtime != 'compose_dsl' ||
          matches.single.routeId.trim().isEmpty ||
          matches.single.screen.trim().isEmpty ||
          entry.action != null) {
        throw StateError(
          'Sidebar tab requires one exact registered Compose route: $id',
        );
      }
      final plugin = await manager.getToolPkgContainerRuntime(
        containerPackageName: entry.containerPackageName,
      );
      if (plugin == null) {
        throw StateError(
          'Sidebar tab owner is disabled: ${entry.containerPackageName}',
        );
      }
      tabs.add(
        _SidebarTab(entry: entry, route: matches.single, plugin: plugin),
      );
    }
    tabs.sort((left, right) => left.entry.order.compareTo(right.entry.order));
    return List<_SidebarTab>.unmodifiable(tabs);
  }

  /// Stops rendering cached plugin routes until fresh catalog validation completes.
  void _reloadCatalog() {
    if (!mounted || _language == null) return;
    final generation = ++_catalogGeneration;
    setState(() {
      _tabs = const [];
      _catalogError = null;
      _catalogLoading = true;
      _actionError = null;
    });
    unawaited(_loadCatalog(generation));
  }

  /// Handles every request immediately and ignores stale asynchronous results.
  Future<void> _loadCatalog(int generation) async {
    try {
      final tabs = await _readCatalog();
      if (!mounted || generation != _catalogGeneration) return;
      setState(() {
        _tabs = tabs;
        _catalogLoading = false;
      });
    } catch (error) {
      if (!mounted || generation != _catalogGeneration) return;
      setState(() {
        _catalogError = error;
        _catalogLoading = false;
      });
    }
  }

  /// Persists an explicit user choice and revalidates its registration before rendering.
  Future<void> _selectTab(String id) async {
    try {
      await _preferences.saveChatSidebarTab(id);
      if (!mounted || _selectedId == id) return;
      setState(() => _selectedId = id);
    } catch (error) {
      if (mounted) setState(() => _preferenceError = error);
    }
  }

  /// Checks fresh route and chat existence before invoking the real host switch chain.
  Future<void> _handleResult(
    Object? raw,
    _SidebarTab tab,
    int generation,
  ) async {
    var acquiredActivation = false;
    try {
      final chatId = chatSidebarActivation(raw);
      if (chatId == null) return;
      if (!mounted ||
          generation != _catalogGeneration ||
          _selectedId != tab.id) {
        throw StateError('The activating sidebar tab is no longer current.');
      }
      if (_activating) {
        throw StateError('A sidebar chat activation is already running.');
      }
      _activating = true;
      acquiredActivation = true;
      final List<_SidebarTab> catalog;
      try {
        catalog = await _readCatalog();
      } catch (_) {
        _reloadCatalog();
        rethrow;
      }
      final current = catalog.where((candidate) => candidate.id == tab.id);
      if (current.length != 1 ||
          current.single.route.screen != tab.route.screen ||
          current.single.route.routeId != tab.route.routeId) {
        _reloadCatalog();
        throw StateError(
          'The activating sidebar route is no longer registered.',
        );
      }
      final chats = await widget.clients.chatRuntimeHolderMain
          .chatHistoryListItemsFlow()
          .first;
      if (!chats.any((chat) => chat.id == chatId)) {
        throw StateError('The requested conversation does not exist: $chatId');
      }
      if (!mounted ||
          generation != _catalogGeneration ||
          _selectedId != tab.id) {
        throw StateError(
          'The sidebar selection changed while activation was being validated.',
        );
      }
      await widget.onActivateChat(chatId);
    } catch (error) {
      if (mounted) setState(() => _actionError = error);
    } finally {
      if (acquiredActivation) {
        _activating = false;
      }
    }
  }

  /// Displays an explicit invalid state without selecting a different tab.
  Widget _error(Object error) => Padding(
    padding: const EdgeInsets.all(12),
    child: Text(
      error.toString(),
      key: const ValueKey('chat-sidebar-tab-error'),
    ),
  );

  /// Builds real embedded tab content rather than routing through a toolbox shortcut.
  Widget _content(List<_SidebarTab> tabs, String selectedId) {
    if (selectedId == UserPreferencesManager.CHAT_SIDEBAR_WORKSPACE_TAB) {
      return widget.workspaceBuilder(context);
    }
    final matches = tabs.where((tab) => tab.id == selectedId);
    if (matches.length != 1) {
      return _error(
        StateError(
          'Selected plugin sidebar tab is not enabled or registered: $selectedId',
        ),
      );
    }
    final tab = matches.single;
    final generation = _catalogGeneration;
    return _embedded(tab, generation);
  }

  Widget _embedded(_SidebarTab tab, int generation) {
    return ToolPkgUiLauncherScreen(
      key: ValueKey('sidebar-route:${tab.id}:$generation'),
      clients: widget.clients,
      plugin: tab.plugin,
      initialRouteId: tab.route.routeId,
      embeddedScreenPath: tab.route.screen,
      showLauncherChrome: false,
      initialModuleSpec: tab.route.moduleSpec,
      initialState: <String, Object?>{
        'input': tab.entry.params,
        'chatSidebar': <String, Object?>{
          'chats': widget.chats.map(chatSidebarSummary).toList(growable: false),
          'currentChatId': widget.currentChatId,
          'activeStreamingChatIds': widget.activeStreamingChatIds.toList(
            growable: false,
          ),
        },
      },
      onActionResult: (raw) => unawaited(_handleResult(raw, tab, generation)),
    );
  }

  /// Matches the legacy compact title-bar capsule instead of adding a row of filter chips.
  Widget _tabSwitch() {
    final scheme = Theme.of(context).colorScheme;
    final entries = <({String id, String label})>[
      for (final tab in _tabs.where(
        (tab) => tab.entry.surface == 'chat_sidebar_tabs',
      ))
        (id: tab.id, label: tab.entry.title),
      (id: UserPreferencesManager.CHAT_SIDEBAR_WORKSPACE_TAB, label: '工作区'),
    ];
    final selected = entries.indexWhere((entry) => entry.id == _selectedId);
    return Container(
      key: const ValueKey('chat-sidebar-segmented-switch'),
      width: entries.length * 49.0,
      height: 23,
      padding: const EdgeInsets.all(2),
      decoration: BoxDecoration(
        color: scheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(12),
      ),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final width = constraints.maxWidth / entries.length;
          return Stack(
            children: <Widget>[
              if (selected >= 0)
                AnimatedPositioned(
                  duration: const Duration(milliseconds: 180),
                  curve: Curves.easeOutCubic,
                  left: selected * width,
                  top: 0,
                  bottom: 0,
                  width: width,
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      color: scheme.secondaryContainer,
                      borderRadius: BorderRadius.circular(10),
                    ),
                  ),
                ),
              Material(
                color: Colors.transparent,
                child: Row(
                  children: <Widget>[
                    for (final entry in entries)
                      Expanded(
                        child: Semantics(
                          selected: entry.id == _selectedId,
                          button: true,
                          child: InkWell(
                            key:
                                entry.id ==
                                    UserPreferencesManager
                                        .CHAT_SIDEBAR_WORKSPACE_TAB
                                ? const ValueKey('chat-sidebar-workspace-tab')
                                : ValueKey('chat-sidebar-tab:${entry.id}'),
                            borderRadius: BorderRadius.circular(10),
                            onTap: entry.id == _selectedId
                                ? null
                                : () => unawaited(_selectTab(entry.id)),
                            child: Center(
                              child: Text(
                                entry.label,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: TextStyle(
                                  fontSize: 10.5,
                                  letterSpacing: -0.2,
                                  fontWeight: entry.id == _selectedId
                                      ? FontWeight.w600
                                      : FontWeight.w400,
                                  color: entry.id == _selectedId
                                      ? scheme.onSecondaryContainer
                                      : scheme.onSurfaceVariant,
                                ),
                              ),
                            ),
                          ),
                        ),
                      ),
                  ],
                ),
              ),
            ],
          );
        },
      ),
    );
  }

  /// Renders the native workspace and every independent registered plugin tab.
  Widget _surface(BuildContext context, String selectedId) {
    if (_preferenceError != null) return _error(_preferenceError!);
    if (_selectedId == null) {
      return const Center(child: M3LoadingIndicator());
    }
    final workspaceSelected =
        selectedId == UserPreferencesManager.CHAT_SIDEBAR_WORKSPACE_TAB;
    final tabs = _tabSwitch();
    final workspaceWithTabs = widget.workspaceWithTabsBuilder;
    if (workspaceSelected && workspaceWithTabs != null) {
      // Native workspace history is independent of plugin catalog availability.
      return Material(
        type: MaterialType.transparency,
        child: workspaceWithTabs(context, tabs),
      );
    }
    return Material(
      type: MaterialType.transparency,
      child: Column(
        children: <Widget>[
          if (widget.headerBuilder != null)
            widget.headerBuilder!(context, tabs)
          else
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Align(alignment: Alignment.centerRight, child: tabs),
            ),
          if (_actionError != null) _error(_actionError!),
          Expanded(
            child: workspaceSelected
                ? widget.workspaceBuilder(context)
                : _catalogError != null
                ? _error(_catalogError!)
                : _catalogLoading
                ? const Center(child: M3LoadingIndicator())
                : _content(_tabs, selectedId),
          ),
          if (!workspaceSelected && widget.pluginFooter != null)
            widget.pluginFooter!,
        ],
      ),
    );
  }

  /// Keeps visited surfaces mounted, so switching tabs does not destroy workers,
  /// pending catalog reads, scroll positions, or local group expansion state.
  @override
  Widget build(BuildContext context) {
    if (_preferenceError != null) return _error(_preferenceError!);
    final selected = _selectedId;
    if (selected == null) return const Center(child: M3LoadingIndicator());
    _visitedTabs.add(selected);
    final available = {
      UserPreferencesManager.CHAT_SIDEBAR_WORKSPACE_TAB,
      ..._tabs
          .where((tab) => tab.entry.surface == 'chat_sidebar_tabs')
          .map((tab) => tab.id),
      selected,
    };
    _visitedTabs.removeWhere((id) => !available.contains(id));
    return Stack(
      fit: StackFit.expand,
      children: [
        for (final id in _visitedTabs)
          Offstage(
            key: ValueKey('sidebar-surface:$id'),
            offstage: id != selected,
            child: TickerMode(
              enabled: id == selected,
              child: _surface(context, id),
            ),
          ),
      ],
    );
  }
}
