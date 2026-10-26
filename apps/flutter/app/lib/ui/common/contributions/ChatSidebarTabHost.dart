// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart' show listEquals;
import 'package:flutter/material.dart';

import '../../../core/application/PluginHotReload.dart';
import '../../../core/logging/ClientLogger.dart';
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
      'parentChatId': chat.parentChatId,
    };

/// Returns a chat activation only for the exact generic discriminator and shape.
String? chatSidebarActivation(Object? raw) =>
    _chatSidebarActionChatId(raw, 'toolpkg.chat.activate');

/// Returns a detached-window request only for the exact generic action shape.
String? chatSidebarWindowRequest(Object? raw) =>
    _chatSidebarActionChatId(raw, 'toolpkg.chat.openInWindow');

String? _chatSidebarActionChatId(Object? raw, String type) {
  if (raw is! Map || raw['type'] != type) {
    return null;
  }
  final chatId = raw['chatId'];
  if (raw.length != 2 || chatId is! String || chatId.trim().isEmpty) {
    throw const FormatException(
      'Sidebar chat action requires exactly type and a nonblank chatId.',
    );
  }
  return chatId;
}

/// Compares opaque decoded values without treating JSON object key order as state.
bool _sidebarValueEquals(Object? left, Object? right) {
  if (identical(left, right)) return true;
  if (left is Map && right is Map) {
    return left.length == right.length &&
        left.entries.every(
          (entry) =>
              right.containsKey(entry.key) &&
              _sidebarValueEquals(entry.value, right[entry.key]),
        );
  }
  if (left is List && right is List) {
    return left.length == right.length &&
        Iterable<int>.generate(
          left.length,
        ).every((index) => _sidebarValueEquals(left[index], right[index]));
  }
  return left == right;
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

  /// Compares registration values rather than newly decoded proxy DTO identities.
  bool hasSameRegistration(_SidebarTab other) =>
      _sidebarValueEquals(entry.toJson(), other.entry.toJson()) &&
      _sidebarValueEquals(route.toJson(), other.route.toJson()) &&
      _sidebarValueEquals(plugin.toJson(), other.plugin.toJson());
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
    this.onOpenChatInWindow,
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

  /// Advertises detached windows only when the host can actually open them.
  final Future<void> Function(core_proxy.ChatHistoryListItem chat)?
  onOpenChatInWindow;

  /// Creates the registry and preference observers for this live sidebar surface.
  @override
  State<ChatSidebarTabHost> createState() => _ChatSidebarTabHostState();
}

class _ChatSidebarTabHostState extends State<ChatSidebarTabHost> {
  List<_SidebarTab> _tabs = const [];
  Object? _catalogError;
  bool _catalogLoading = true;
  StreamSubscription<String>? _preferenceSubscription;
  StreamSubscription<ToolPkgCatalogChange>? _catalogSubscription;
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

  /// Observes selection and reconciles registrations without resetting other tabs.
  @override
  void initState() {
    super.initState();
    _watchSelection();
    _catalogSubscription = ToolPkgCatalogChangeBus.listenChanges((change) {
      if (change.reloadPluginCode) {
        _reloadCatalog();
      } else {
        _refreshCatalog();
      }
    });
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

  /// Revalidates catalog notifications while keeping unchanged tab sessions mounted.
  void _refreshCatalog() {
    if (!mounted || _language == null) return;
    unawaited(_loadCatalog(++_catalogGeneration));
  }

  /// Retires all sessions for runtime/locale changes, hot reload, or invalid routes.
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
      final catalog = await _readCatalog();
      if (!mounted || generation != _catalogGeneration) return;
      final previousById = {for (final tab in _tabs) tab.id: tab};
      final tabs = [
        for (final tab in catalog)
          if (previousById[tab.id] case final previous?
              when previous.hasSameRegistration(tab))
            previous
          else
            tab,
      ];
      if (!_catalogLoading &&
          _catalogError == null &&
          listEquals(tabs, _tabs)) {
        return;
      }
      setState(() {
        _tabs = tabs;
        _catalogError = null;
        _catalogLoading = false;
      });
    } catch (error) {
      if (!mounted || generation != _catalogGeneration) return;
      setState(() {
        // A failed validation must never leave unverified plugin routes active.
        _tabs = const [];
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
  Future<void> _handleResult(Object? raw, _SidebarTab tab) async {
    var acquiredActivation = false;
    try {
      final windowChatId = chatSidebarWindowRequest(raw);
      final chatId = windowChatId ?? chatSidebarActivation(raw);
      if (chatId == null) return;
      if (windowChatId != null && widget.onOpenChatInWindow == null) {
        throw UnsupportedError('Detached chat windows are not available.');
      }
      if (!mounted || !_tabs.contains(tab) || _selectedId != tab.id) {
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
      if (current.length != 1 || !current.single.hasSameRegistration(tab)) {
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
      if (!mounted || !_tabs.contains(tab) || _selectedId != tab.id) {
        throw StateError(
          'The sidebar selection changed while activation was being validated.',
        );
      }
      if (windowChatId != null) {
        final openInWindow = widget.onOpenChatInWindow;
        if (openInWindow == null) {
          throw UnsupportedError('Detached chat windows are not available.');
        }
        await openInWindow(chats.firstWhere((chat) => chat.id == chatId));
      } else {
        await widget.onActivateChat(chatId);
      }
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
      return _pluginScroll(
        selectedId,
        SliverToBoxAdapter(
          child: _error(
            StateError(
              'Selected plugin sidebar tab is not enabled or registered: $selectedId',
            ),
          ),
        ),
      );
    }
    final tab = matches.single;
    return _embedded(tab);
  }

  /// Keeps plugin execution intact while the sidebar composes its sliver content.
  Widget _embedded(_SidebarTab tab) {
    final prepared = ChatSidebarPreparationScope.maybeOf(
      context,
    )?.read(tab.id, widget.chats);
    return ToolPkgUiLauncherScreen(
      // Reconciled registrations retain identity across unrelated catalog changes.
      key: ObjectKey(tab),
      clients: widget.clients,
      plugin: tab.plugin,
      initialRouteId: tab.route.routeId,
      embeddedScreenPath: tab.route.screen,
      showLauncherChrome: false,
      showLoadingIndicator: false,
      contentBuilder: (context, content) =>
          _pluginScroll(tab.id, content.sliver),
      initialModuleSpec: tab.route.moduleSpec,
      initialState: <String, Object?>{
        'chatSidebarPrepared': ?prepared,
        'input': tab.entry.params,
        'chatSidebarCapabilities': <String, Object?>{
          'openInWindow': widget.onOpenChatInWindow != null,
        },
        'chatSidebar': <String, Object?>{
          'chats': widget.chats.map(chatSidebarSummary).toList(growable: false),
          'currentChatId': widget.currentChatId,
          'activeStreamingChatIds': widget.activeStreamingChatIds.toList(
            growable: false,
          ),
        },
      },
      onActionResult: (raw) => unawaited(_handleResult(raw, tab)),
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
    if (workspaceSelected) {
      return Material(
        type: MaterialType.transparency,
        child: Column(
          children: [
            _header(tabs),
            if (_actionError != null) _error(_actionError!),
            Expanded(child: _content(_tabs, selectedId)),
          ],
        ),
      );
    }
    return Material(
      type: MaterialType.transparency,
      child: _catalogError != null
          ? _pluginScroll(
              selectedId,
              SliverToBoxAdapter(child: _error(_catalogError!)),
            )
          : _catalogLoading
          ? _pluginScroll(
              selectedId,
              const SliverToBoxAdapter(child: SizedBox.shrink()),
            )
          : _content(_tabs, selectedId),
    );
  }

  /// Builds the same title and tab controls for each owning viewport.
  Widget _header(Widget tabs) => widget.headerBuilder != null
      ? widget.headerBuilder!(context, tabs)
      : Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          child: Align(alignment: Alignment.centerRight, child: tabs),
        );

  /// Composes host chrome and plugin rows in one viewport with item-level laziness.
  Widget _pluginScroll(String selectedId, Widget content) => CustomScrollView(
    key: ValueKey('sidebar-scroll:$selectedId'),
    slivers: [
      SliverToBoxAdapter(child: _header(_tabSwitch())),
      if (_actionError != null)
        SliverToBoxAdapter(child: _error(_actionError!)),
      content,
      if (widget.pluginFooter != null)
        SliverToBoxAdapter(child: widget.pluginFooter!),
    ],
  );

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

/// Prepares opaque plugin data without opening a Compose session or a UI route.
class ChatSidebarPreparation extends ChangeNotifier {
  ChatSidebarPreparation({required this.clients}) {
    _catalogSubscription = ToolPkgCatalogChangeBus.listenChanges((change) {
      if (change.reloadPluginCode) {
        _invalidate();
      } else {
        final chats = _chats;
        if (chats != null) unawaited(_prepare(chats, refresh: true));
      }
    });
    PluginHotReload.revision.addListener(_invalidate);
  }

  static const api = 'chat.sidebar.prepare';
  final GeneratedCoreProxyClients clients;
  StreamSubscription<void>? _catalogSubscription;
  List<core_proxy.ChatHistoryListItem>? _chats;
  String? _key;
  int _generation = 0;
  bool _disposed = false;
  Map<String, Object?> _data = const {};

  /// Only full generic history changes require new plugin projections.
  Future<void> prepare(List<core_proxy.ChatHistoryListItem> chats) =>
      _prepare(chats);

  /// Catalog-only refreshes retain same-history data until changed results arrive.
  Future<void> _prepare(
    List<core_proxy.ChatHistoryListItem> chats, {
    bool refresh = false,
  }) async {
    if (_disposed) return;
    final summaries = chats.map(chatSidebarSummary).toList(growable: false);
    final key = jsonEncode(summaries);
    if (key == _key && !refresh) return;
    final preserveData = refresh && key == _key;
    _chats = List.unmodifiable(chats);
    _key = key;
    final generation = ++_generation;
    if (!preserveData) {
      _data = const {};
      notifyListeners();
    }
    final refreshedData = <String, Object?>{};
    final watch = Stopwatch()..start();
    try {
      final manager = clients.application.packageManager();
      final owners = await manager.getToolPkgPublicApiOwners(apiName: api);
      if (!_current(generation)) return;
      final seen = <String>{};
      for (final owner in owners) {
        if (owner.apiName != api ||
            owner.containerPackageName.trim().isEmpty ||
            !seen.add(owner.containerPackageName)) {
          throw const FormatException('Invalid sidebar preparation API owner.');
        }
      }
      // A failed optional provider must not suppress another owner's data.
      await Future.wait(
        owners.map((owner) async {
          try {
            final raw = await manager.invokeToolPkgPublicApi(
              packageName: owner.containerPackageName,
              methodName: api,
              payload: <String, Object?>{'chats': summaries},
            );
            if (!_current(generation)) return;
            if (raw is! Map || raw.length != 1 || raw['entries'] is! List) {
              throw const FormatException(
                'Sidebar preparation must return entries.',
              );
            }
            final prepared = <String, Object?>{};
            for (final entry in raw['entries'] as List) {
              if (entry is! Map ||
                  entry.length != 2 ||
                  !entry.containsKey('data') ||
                  entry['entryId'] is! String ||
                  (entry['entryId'] as String).trim().isEmpty) {
                throw const FormatException('Invalid prepared sidebar entry.');
              }
              final id = chatSidebarTabIdentity(
                owner.containerPackageName,
                entry['entryId'] as String,
              );
              if (prepared.containsKey(id)) {
                throw const FormatException(
                  'Duplicate prepared sidebar entry.',
                );
              }
              prepared[id] = entry['data'];
            }
            if (preserveData) {
              refreshedData.addAll(prepared);
            } else {
              _data = {..._data, ...prepared};
              notifyListeners();
            }
            ClientLogger.v(
              'event=sidebar_data_prepared package=${owner.containerPackageName} '
              'entries=${prepared.length} elapsedMs=${watch.elapsedMilliseconds}',
              tag: 'ChatSidebarPreparation',
              verboseLevel: VerboseLevel.level4,
            );
          } catch (error) {
            if (_current(generation)) _logFailure(error);
          }
        }),
      );
    } catch (error) {
      if (_current(generation)) _logFailure(error);
    }
    if (preserveData &&
        _current(generation) &&
        !_sidebarValueEquals(_data, refreshedData)) {
      _data = refreshedData;
      notifyListeners();
    }
  }

  /// Returns only same-history data; the host never interprets business fields.
  Object? read(String id, List<core_proxy.ChatHistoryListItem> chats) =>
      _key == jsonEncode(chats.map(chatSidebarSummary).toList(growable: false))
      ? _data[id]
      : null;

  bool _current(int generation) => !_disposed && generation == _generation;

  void _logFailure(Object error) => ClientLogger.w(
    'Optional sidebar preparation failed; route will load normally: $error',
    tag: 'ChatSidebarPreparation',
  );

  void _invalidate() {
    if (_disposed) return;
    _key = null;
    final chats = _chats;
    if (chats != null) unawaited(prepare(chats));
  }

  @override
  void dispose() {
    _disposed = true;
    ++_generation;
    unawaited(_catalogSubscription?.cancel());
    PluginHotReload.revision.removeListener(_invalidate);
    super.dispose();
  }
}

/// Keeps prepared data owned by the main window, not the transient drawer tree.
class ChatSidebarPreparationScope
    extends InheritedNotifier<ChatSidebarPreparation> {
  const ChatSidebarPreparationScope({
    super.key,
    required ChatSidebarPreparation preparation,
    required super.child,
  }) : super(notifier: preparation);

  static ChatSidebarPreparation? maybeOf(BuildContext context) => context
      .dependOnInheritedWidgetOfExactType<ChatSidebarPreparationScope>()
      ?.notifier;
}
