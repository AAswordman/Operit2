// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/AnimatedLazyIndexedStack.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../main/navigation/AppNavigationModels.dart';
import '../../../main/screens/OperitScreens.dart';
import '../../../main/screens/ScreenRouteRegistry.dart';
import '../../../main/TopBarController.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/EmptyState.dart';
import '../market/ArtifactMarketSupport.dart';
import '../market/MarketBrowseControls.dart';
import '../market/MarketBrowseList.dart';
import '../market/MarketStatsSupport.dart';
import 'ArtifactPublishScreen.dart';
import 'MarketEntryDetailScreen.dart';
import 'RepoMarketPublishScreen.dart';

enum MarketHomeTab { all, categories, mine }

class UnifiedMarketScreen extends StatefulWidget {
  const UnifiedMarketScreen({
    super.key,
    this.initialTab = MarketHomeTab.all,
    this.categoryId,
    this.categoryName,
    GeneratedCoreProxyClients? clients,
  }) : clients =
           clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final MarketHomeTab initialTab;
  final String? categoryId;
  final String? categoryName;
  final GeneratedCoreProxyClients clients;

  @override
  State<UnifiedMarketScreen> createState() => _UnifiedMarketScreenState();
}

class _UnifiedMarketScreenState extends State<UnifiedMarketScreen>
    with SingleTickerProviderStateMixin {
  late MarketHomeTab _selectedTab = widget.initialTab;
  late final TabController _tabController;
  MarketSortOption _sortOption = MarketSortOption.updated;
  bool _featuredOnly = true;
  String? _typeFilter;
  String _searchInput = '';
  String _searchQuery = '';
  bool _searchExpanded = false;
  Timer? _searchDebounce;
  TopBarController? _topBarController;

  bool get _hasCategoryScope => widget.categoryId?.trim().isNotEmpty == true;

  bool get _searchEnabled => _selectedTab == MarketHomeTab.all;

  bool get _isSearchActive =>
      _searchEnabled && (_searchExpanded || _searchInput.trim().isNotEmpty);

  @override
  void initState() {
    super.initState();
    _tabController = TabController(
      length: MarketHomeTab.values.length,
      initialIndex: _selectedTab.index,
      vsync: this,
    );
  }

  @override
  void didUpdateWidget(covariant UnifiedMarketScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.initialTab != widget.initialTab &&
        _selectedTab != widget.initialTab) {
      _selectedTab = widget.initialTab;
      _tabController.animateTo(_selectedTab.index);
      _searchInput = '';
      _searchQuery = '';
      _searchExpanded = false;
      _searchDebounce?.cancel();
      _syncTopBar();
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _topBarController = TopBarScope.of(context);
    _syncTopBar();
  }

  @override
  void dispose() {
    _searchDebounce?.cancel();
    _topBarController?.clearActions(owner: this);
    _topBarController?.clearTitleContent(owner: this);
    _tabController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Colors.transparent,
      child: Column(
        children: <Widget>[
          if (_hasCategoryScope)
            _MarketCategoryScopeHeader(
              title: widget.categoryName?.trim().isNotEmpty == true
                  ? widget.categoryName!.trim()
                  : widget.categoryId!.trim(),
            ),
          if (_selectedTab == MarketHomeTab.all)
            _MarketTypeFilterBar(
              selectedType: _typeFilter,
              onChanged: (type) {
                setState(() {
                  _typeFilter = type;
                });
              },
            ),
          Expanded(
            child: AnimatedLazyIndexedStack(
              index: _selectedTab.index,
              itemCount: MarketHomeTab.values.length,
              itemBuilder: (context, index) {
                return switch (MarketHomeTab.values[index]) {
                  MarketHomeTab.all => _MarketListPane(
                    clients: widget.clients,
                    type: _typeFilter,
                    categoryId: widget.categoryId,
                    sortOption: _sortOption,
                    featuredOnly: _featuredOnly,
                    onSortChanged: (sortOption) {
                      setState(() {
                        _sortOption = sortOption;
                      });
                    },
                    onFeaturedOnlyChanged: (value) {
                      setState(() {
                        _featuredOnly = value;
                      });
                    },
                    searchQuery: _searchQuery,
                  ),
                  MarketHomeTab.categories => _MarketCategoriesPane(
                    clients: widget.clients,
                  ),
                  MarketHomeTab.mine => _MarketMinePane(
                    clients: widget.clients,
                  ),
                };
              },
            ),
          ),
          if (!_hasCategoryScope)
            NavigationBar(
              height: 56,
              selectedIndex: _selectedTab.index,
              labelBehavior: NavigationDestinationLabelBehavior.alwaysHide,
              onDestinationSelected: (index) {
                setState(() {
                  _selectedTab = MarketHomeTab.values[index];
                  _searchInput = '';
                  _searchQuery = '';
                  _searchExpanded = false;
                  _searchDebounce?.cancel();
                });
                _syncTopBar();
              },
              destinations: const <NavigationDestination>[
                NavigationDestination(
                  icon: Icon(Icons.storefront_outlined),
                  selectedIcon: Icon(Icons.storefront),
                  label: '全部',
                ),
                NavigationDestination(
                  icon: Icon(Icons.category_outlined),
                  selectedIcon: Icon(Icons.category),
                  label: '分类',
                ),
                NavigationDestination(
                  icon: Icon(Icons.person_outline),
                  selectedIcon: Icon(Icons.person),
                  label: '我的',
                ),
              ],
            ),
        ],
      ),
    );
  }

  void _onSearchChanged(String value) {
    _searchDebounce?.cancel();
    setState(() {
      _searchInput = value;
    });
    _searchDebounce = Timer(const Duration(milliseconds: 320), () {
      if (!mounted) {
        return;
      }
      setState(() {
        _searchQuery = _searchInput.trim();
      });
      _syncTopBar();
    });
    _syncTopBar();
  }

  void _closeSearch() {
    _searchDebounce?.cancel();
    setState(() {
      _searchExpanded = false;
      _searchInput = '';
      _searchQuery = '';
    });
    _syncTopBar();
  }

  void _syncTopBar() {
    final controller = _topBarController;
    if (controller == null) {
      return;
    }
    if (!_searchEnabled) {
      controller.setActions((context) => const <Widget>[], owner: this);
      controller.clearTitleContent(owner: this);
      return;
    }
    controller.setActions((context) {
      if (_isSearchActive) {
        return const <Widget>[];
      }
      return <Widget>[
        IconButton(
          onPressed: () {
            setState(() {
              _searchExpanded = true;
            });
            _syncTopBar();
          },
          icon: const Icon(Icons.search),
          tooltip: '搜索',
        ),
      ];
    }, owner: this);
    if (_isSearchActive) {
      controller.setTitleContent(
        TopBarTitleContent(
          (context) => MarketTopBarSearchField(
            query: _searchInput,
            onQueryChanged: _onSearchChanged,
            onClose: _closeSearch,
          ),
        ),
        owner: this,
      );
    } else {
      controller.clearTitleContent(owner: this);
    }
  }
}

class _MarketListPane extends StatefulWidget {
  const _MarketListPane({
    required this.clients,
    required this.type,
    required this.categoryId,
    required this.sortOption,
    required this.featuredOnly,
    required this.onSortChanged,
    required this.onFeaturedOnlyChanged,
    required this.searchQuery,
  });

  final GeneratedCoreProxyClients clients;
  final String? type;
  final String? categoryId;
  final MarketSortOption sortOption;
  final bool featuredOnly;
  final ValueChanged<MarketSortOption> onSortChanged;
  final ValueChanged<bool> onFeaturedOnlyChanged;
  final String searchQuery;

  @override
  State<_MarketListPane> createState() => _MarketListPaneState();
}

class _MarketListPaneState extends State<_MarketListPane> {
  bool _loading = true;
  bool _loadingMore = false;
  String? _errorMessage;
  int _page = 1;
  int _totalPages = 1;
  final Set<String> _busyEntryIds = <String>{};
  List<core_proxy.MarketEntrySummary> _items =
      <core_proxy.MarketEntrySummary>[];
  List<core_proxy.MarketEntrySummary> _searchItems =
      <core_proxy.MarketEntrySummary>[];
  List<core_proxy.MarketEntrySummary>? _searchCorpus;
  bool _searchLoading = false;
  int _searchGeneration = 0;

  GeneratedProvidersMarketStatsApiServiceCoreProxy get _market =>
      widget.clients.providersMarketStatsApiService;

  @override
  void initState() {
    super.initState();
    _loadFirstPage();
  }

  @override
  void didUpdateWidget(covariant _MarketListPane oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.searchQuery != widget.searchQuery) {
      _loadSearchResults();
    }
    if (oldWidget.sortOption != widget.sortOption ||
        oldWidget.type != widget.type ||
        oldWidget.categoryId != widget.categoryId) {
      _searchCorpus = null;
      _loadFirstPage(clearExisting: true);
    }
  }

  Future<void> _loadFirstPage({bool clearExisting = false}) async {
    setState(() {
      _loading = true;
      _errorMessage = null;
      if (clearExisting) {
        _items = <core_proxy.MarketEntrySummary>[];
        _searchItems = <core_proxy.MarketEntrySummary>[];
        _searchCorpus = null;
        _page = 1;
        _totalPages = 1;
      }
    });
    try {
      final page = await _loadPage(1);
      if (!mounted) {
        return;
      }
      setState(() {
        _items = page.items;
        _searchItems = <core_proxy.MarketEntrySummary>[];
        _searchCorpus = null;
        _page = page.page;
        _totalPages = _pageCount(page.total, page.pageSize);
        _loading = false;
      });
      await _loadSearchResults();
    } catch (error, stackTrace) {
      debugPrint('Failed to load market: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
        _loading = false;
      });
    }
  }

  Future<void> _loadMore() async {
    if (_loadingMore || !_hasMore) {
      return;
    }
    setState(() {
      _loadingMore = true;
    });
    try {
      final page = await _loadPage(_page + 1);
      if (!mounted) {
        return;
      }
      setState(() {
        _items = <core_proxy.MarketEntrySummary>[..._items, ...page.items];
        _page = page.page;
        _totalPages = _pageCount(page.total, page.pageSize);
        _loadingMore = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load more market: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _loadingMore = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  Future<void> _loadSearchResults() async {
    final query = widget.searchQuery.trim();
    final generation = ++_searchGeneration;
    if (query.isEmpty) {
      if (!mounted) {
        return;
      }
      setState(() {
        _searchItems = <core_proxy.MarketEntrySummary>[];
        _searchLoading = false;
      });
      return;
    }
    setState(() {
      _searchLoading = true;
    });
    try {
      final corpus = _searchCorpus ?? await _loadAllPagesForLocalSearch();
      if (!mounted || generation != _searchGeneration) {
        return;
      }
      _searchCorpus = corpus;
      setState(() {
        _searchItems = corpus;
        _searchLoading = false;
      });
    } catch (error, stackTrace) {
      debugPrint(
        'Failed to load local market search corpus: $error\n$stackTrace',
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _searchLoading = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  Future<List<core_proxy.MarketEntrySummary>>
  _loadAllPagesForLocalSearch() async {
    final firstPage = await _loadPage(1);
    final totalPages = _pageCount(firstPage.total, firstPage.pageSize);
    final entries = <core_proxy.MarketEntrySummary>[...firstPage.items];
    for (var page = 2; page <= totalPages; page += 1) {
      final nextPage = await _loadPage(page);
      entries.addAll(nextPage.items);
    }
    return entries;
  }

  String get _metric => marketSortMetric(widget.sortOption);

  bool get _hasMore => _page < _totalPages;

  Future<core_proxy.MarketListPage> _loadPage(int page) {
    final type = widget.type?.trim();
    final categoryId = widget.categoryId?.trim();
    if (type != null &&
        type.isNotEmpty &&
        categoryId != null &&
        categoryId.isNotEmpty) {
      return _market.getTypeCategoryPage(
        type: type,
        categoryId: categoryId,
        sort: _metric,
        page: page,
      );
    }
    if (type != null && type.isNotEmpty) {
      return _market.getTypePage(type: type, sort: _metric, page: page);
    }
    if (categoryId != null && categoryId.isNotEmpty) {
      return _market.getCategoryPage(
        categoryId: categoryId,
        sort: _metric,
        page: page,
      );
    }
    return _market.getListPage(sort: _metric, page: page);
  }

  @override
  Widget build(BuildContext context) {
    final error = _errorMessage;
    Widget content;
    if (_loading && _items.isEmpty) {
      content = const M3LoadingPane();
    } else if (error != null && _items.isEmpty) {
      content = EmptyState(
        icon: Icons.error_outline,
        title: '加载失败',
        message: error,
        action: TextButton.icon(
          onPressed: _loadFirstPage,
          icon: const Icon(Icons.refresh),
          label: const Text('刷新'),
        ),
      );
    } else {
      final rawQuery = widget.searchQuery.trim();
      final query = rawQuery.toLowerCase();
      final sourceItems = rawQuery.isEmpty ? _items : _searchItems;
      final displayed = sourceItems
          .where((item) => !widget.featuredOnly || item.featured)
          .where(
            (item) =>
                query.isEmpty ||
                item.title.toLowerCase().contains(query) ||
                item.description.toLowerCase().contains(query) ||
                item.detail.toLowerCase().contains(query) ||
                item.id.toLowerCase().contains(query) ||
                item.type.toLowerCase().contains(query) ||
                (item.categoryId ?? '').toLowerCase().contains(query) ||
                (item.publisher?.login ?? item.author?.login ?? '')
                    .toLowerCase()
                    .contains(query),
          )
          .toList(growable: false);
      content = MarketBrowseList(
        isLoading: _loading || _searchLoading,
        isLoadingMore: _loadingMore,
        hasMore: _hasMore && rawQuery.isEmpty,
        isEmpty: displayed.isEmpty,
        emptyTitle: rawQuery.isEmpty ? '暂无项目' : '没有匹配结果',
        onRefresh: _loadFirstPage,
        onLoadMore: _loadMore,
        items: displayed,
        groupByUpdatedDate: widget.sortOption == MarketSortOption.updated,
        updatedAt: (item) => item.publishedAt ?? item.updatedAt,
        itemBuilder: (item) => MarketGridCard(
          title: item.title,
          description: item.description,
          author: item.publisher?.login ?? item.author?.login ?? '',
          downloads: _entryDownloads(item),
          likes: _reactionTotal(item, '+1'),
          hearts: _reactionTotal(item, 'heart'),
          actionLabel: _actionLabel(item),
          actionIcon: Icons.download_outlined,
          actionBusy: _busyEntryIds.contains(item.id),
          onAction: () => _installEntry(item),
          onTap: () => _openDetails(item),
        ),
      );
    }
    return Column(
      children: <Widget>[
        MarketBrowseControls(
          sortOption: widget.sortOption,
          enabled: true,
          featuredOnly: widget.featuredOnly,
          onSortChanged: widget.onSortChanged,
          onFeaturedOnlyChanged: widget.onFeaturedOnlyChanged,
        ),
        Expanded(child: content),
      ],
    );
  }

  void _openDetails(core_proxy.MarketEntrySummary item) {
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) =>
            MarketEntryDetailScreen(clients: widget.clients, entry: item),
      ),
    );
  }

  Future<void> _installEntry(core_proxy.MarketEntrySummary item) async {
    setState(() {
      _busyEntryIds.add(item.id);
    });
    try {
      if (item.type == 'skill') {
        await _installSkill(item);
      } else if (item.type == 'mcp') {
        await _installMcp(item);
      } else {
        final result = await runCoreMarketInstall(
          clients: widget.clients,
          type: item.type,
          entryId: item.id,
        );
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(result),
              behavior: SnackBarBehavior.floating,
            ),
          );
        }
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to install market entry: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    } finally {
      if (mounted) {
        setState(() {
          _busyEntryIds.remove(item.id);
        });
      }
    }
  }

  Future<void> _installSkill(core_proxy.MarketEntrySummary item) async {
    final repoUrl = item.source?.url.trim() ?? '';
    if (repoUrl.isEmpty) {
      throw StateError('技能缺少仓库地址');
    }
    final result = await widget.clients.application
        .skillRepository()
        .importSkillFromGitHubRepo(repoUrl: repoUrl);
    if (!mounted) {
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(result), behavior: SnackBarBehavior.floating),
    );
  }

  Future<void> _installMcp(core_proxy.MarketEntrySummary item) async {
    final repoUrl = item.source?.url.trim() ?? '';
    if (repoUrl.isEmpty) {
      throw StateError('MCP 缺少仓库地址');
    }
    final result = await widget.clients.application
        .mcpRepository()
        .installMcpServerWithObjectForFlutter(
          pluginId: _safePackageId(item.title),
          repoUrl: repoUrl,
          name: item.title,
          description: item.description,
          mcpConfig:
              item.repoVersion?.installConfig ??
              item.latestVersion?.installConfig ??
              '',
        );
    if (!mounted) {
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(result), behavior: SnackBarBehavior.floating),
    );
  }
}

String _actionLabel(core_proxy.MarketEntrySummary item) {
  return (item.type == 'script' || item.type == 'package') ? '下载' : '安装';
}

int _reactionTotal(core_proxy.MarketEntrySummary item, String reaction) {
  return item.reactions
      .where((count) => count.reaction == reaction)
      .fold<int>(0, (sum, count) => sum + count.total);
}

class _ArtifactManageScreen extends StatefulWidget {
  const _ArtifactManageScreen({required this.clients});

  final GeneratedCoreProxyClients clients;

  @override
  State<_ArtifactManageScreen> createState() => _ArtifactManageScreenState();
}

class _ArtifactManageScreenState extends State<_ArtifactManageScreen> {
  bool _loading = true;
  String? _errorMessage;
  String? _openingEntryId;
  List<core_proxy.MarketPublisherEntrySummary> _entries =
      <core_proxy.MarketPublisherEntrySummary>[];
  List<core_proxy.MarketNotification> _notifications =
      <core_proxy.MarketNotification>[];

  GeneratedProvidersMarketStatsApiServiceCoreProxy get _market =>
      widget.clients.providersMarketStatsApiService;

  @override
  void initState() {
    super.initState();
    _loadMine();
  }

  Future<void> _loadMine() async {
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    try {
      final mine = await _market.getMyEntries();
      final notifications = await _market.getNotifications(
        limit: 50,
        offset: 0,
        since: null,
      );
      if (!mounted) return;
      setState(() {
        _entries = mine.entries;
        _notifications = notifications.items;
        _loading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load market account data: $error\n$stackTrace');
      if (!mounted) return;
      setState(() {
        _errorMessage = error.toString();
        _loading = false;
      });
    }
  }

  Future<void> _openManagedEntry(
    core_proxy.MarketPublisherEntrySummary entry,
  ) async {
    if (entry.stateCode != 'approved') {
      _showPrivateEntrySummary(entry);
      return;
    }
    setState(() {
      _openingEntryId = entry.id;
    });
    try {
      final detail = await _market.getEntryById(entryId: entry.id);
      if (!mounted) return;
      await Navigator.of(context).push(
        MaterialPageRoute<void>(
          builder: (context) =>
              MarketEntryDetailScreen(clients: widget.clients, entry: detail),
        ),
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to open managed market entry: $error\n$stackTrace');
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    } finally {
      if (mounted) {
        setState(() {
          _openingEntryId = null;
        });
      }
    }
  }

  Future<void> _publishManagedVersion(
    core_proxy.MarketPublisherEntrySummary entry,
  ) async {
    if (entry.stateCode != 'approved') {
      _showPrivateEntrySummary(entry);
      return;
    }
    setState(() {
      _openingEntryId = entry.id;
    });
    try {
      final detail = await _market.getEntryById(entryId: entry.id);
      if (!mounted) return;
      final canEditEntry = entry.relation == 'owner';
      await Navigator.of(context).push(
        MaterialPageRoute<void>(
          builder: (context) {
            if (detail.type == 'skill' || detail.type == 'mcp') {
              return RepoMarketPublishScreen(
                clients: widget.clients,
                type: detail.type,
                publishContext: RepoMarketPublishContext(
                  entry: detail,
                  canEditEntry: canEditEntry,
                ),
              );
            }
            final artifact = detail.artifact;
            return ArtifactPublishScreen(
              clients: widget.clients,
              publishContext: ArtifactPublishClusterContext(
                entryId: detail.id,
                projectId: artifact?.projectId ?? '',
                runtimePackageId:
                    artifact?.runtimePackageId ??
                    detail.latestVersion?.runtimePackageId ??
                    '',
                lockedDisplayName: detail.title,
                canEditEntry: canEditEntry,
              ),
            );
          },
        ),
      );
      if (mounted) {
        await _loadMine();
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to open managed version publish: $error\n$stackTrace');
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    } finally {
      if (mounted) {
        setState(() {
          _openingEntryId = null;
        });
      }
    }
  }

  void _showPrivateEntrySummary(core_proxy.MarketPublisherEntrySummary entry) {
    showDialog<void>(
      context: context,
      builder: (context) {
        final reasons = entry.reasonCodes
            .map(_marketReasonLabel)
            .where((reason) => reason.trim().isNotEmpty)
            .join('\n');
        return AlertDialog(
          title: Text(entry.title),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text('状态：${_marketStateLabel(entry.stateCode)}'),
              const SizedBox(height: 8),
              Text('关系：${_marketRelationLabel(entry.relation)}'),
              if ((entry.categoryId ?? '').trim().isNotEmpty) ...<Widget>[
                const SizedBox(height: 8),
                Text('分类：${entry.categoryId}'),
              ],
              if (reasons.isNotEmpty) ...<Widget>[
                const SizedBox(height: 12),
                Text(
                  '审核原因',
                  style: Theme.of(
                    context,
                  ).textTheme.labelLarge?.copyWith(fontWeight: FontWeight.w700),
                ),
                const SizedBox(height: 6),
                Text(reasons),
              ],
            ],
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('知道了'),
            ),
          ],
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    final error = _errorMessage;
    return Scaffold(
      backgroundColor: Colors.transparent,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        title: const Text('我的市场'),
        actions: <Widget>[
          IconButton(
            onPressed: _loading ? null : _loadMine,
            icon: const Icon(Icons.refresh),
            tooltip: '刷新',
          ),
        ],
      ),
      body: Builder(
        builder: (context) {
          if (_loading && _entries.isEmpty && _notifications.isEmpty) {
            return const M3LoadingPane();
          }
          if (error != null && _entries.isEmpty && _notifications.isEmpty) {
            return EmptyState(
              icon: Icons.error_outline,
              title: '加载失败',
              message: error,
              action: TextButton.icon(
                onPressed: _loadMine,
                icon: const Icon(Icons.refresh),
                label: const Text('刷新'),
              ),
            );
          }
          return RefreshIndicator(
            onRefresh: _loadMine,
            child: ListView(
              padding: const EdgeInsets.fromLTRB(18, 14, 18, 32),
              children: <Widget>[
                Text(
                  '我的发布',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.w800,
                  ),
                ),
                const SizedBox(height: 10),
                if (_entries.isEmpty)
                  const Text('暂无发布记录')
                else
                  for (final entry in _entries) ...<Widget>[
                    _MarketManageEntryTile(
                      entry: entry,
                      opening: _openingEntryId == entry.id,
                      onOpen: () => _openManagedEntry(entry),
                      onPublishVersion: () => _publishManagedVersion(entry),
                    ),
                    const SizedBox(height: 10),
                  ],
                const SizedBox(height: 24),
                Text(
                  '通知',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.w800,
                  ),
                ),
                const SizedBox(height: 10),
                if (_notifications.isEmpty)
                  const Text('暂无通知')
                else
                  for (final notice in _notifications) ...<Widget>[
                    _MarketNotificationTile(notice: notice),
                    const SizedBox(height: 10),
                  ],
              ],
            ),
          );
        },
      ),
    );
  }
}

class _MarketManageEntryTile extends StatelessWidget {
  const _MarketManageEntryTile({
    required this.entry,
    required this.opening,
    required this.onOpen,
    required this.onPublishVersion,
  });

  final core_proxy.MarketPublisherEntrySummary entry;
  final bool opening;
  final VoidCallback onOpen;
  final VoidCallback onPublishVersion;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final reasons = entry.reasonCodes
        .map(_marketReasonLabel)
        .where((reason) => reason.trim().isNotEmpty)
        .toList(growable: false);
    final canPublishVersion = entry.stateCode == 'approved';
    return OperitGlassSurface(
      color: colorScheme.surface,
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(14),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      child: ListTile(
        onTap: opening ? null : onOpen,
        leading: Icon(_marketTypeIcon(entry.type)),
        title: Text(entry.title),
        subtitle: Padding(
          padding: const EdgeInsets.only(top: 4),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                '${_marketTypeLabel(entry.type)} · ${_marketStateLabel(entry.stateCode)} · ${_marketRelationLabel(entry.relation)}',
              ),
              if (reasons.isNotEmpty) ...<Widget>[
                const SizedBox(height: 4),
                Text(
                  reasons.join('、'),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(color: colorScheme.error),
                ),
              ],
              const SizedBox(height: 4),
              Text('更新于 ${formatMarketDate(entry.updatedAt)}'),
            ],
          ),
        ),
        trailing: opening
            ? const SizedBox.square(
                dimension: 20,
                child: CircularProgressIndicator(strokeWidth: 2),
              )
            : TextButton(
                onPressed: canPublishVersion ? onPublishVersion : onOpen,
                child: Text(canPublishVersion ? '发布新版本' : '查看状态'),
              ),
      ),
    );
  }
}

class _MarketNotificationTile extends StatelessWidget {
  const _MarketNotificationTile({required this.notice});

  final core_proxy.MarketNotification notice;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: colorScheme.surface,
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(14),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      child: ListTile(
        leading: Icon(_marketNotificationIcon(notice.kind)),
        title: Text(_marketNotificationTitle(notice)),
        subtitle: Text(_marketNotificationBody(notice)),
        trailing: Text(formatMarketDate(notice.createdAt)),
      ),
    );
  }
}

String _marketTypeLabel(String type) => switch (type) {
  'script' => '脚本',
  'package' => '包',
  'skill' => 'Skill',
  'mcp' => 'MCP',
  _ => type,
};

IconData _marketTypeIcon(String type) => switch (type) {
  'script' => Icons.code_outlined,
  'package' => Icons.inventory_2_outlined,
  'skill' => Icons.psychology_outlined,
  'mcp' => Icons.hub_outlined,
  _ => Icons.extension_outlined,
};

String _marketStateLabel(String stateCode) => switch (stateCode) {
  'pending' => '待审核',
  'approved' => '已发布',
  'changes_requested' => '需修改',
  'rejected' => '已拒绝',
  'withdrawn' => '已撤回',
  _ => stateCode,
};

String _marketRelationLabel(String relation) => switch (relation) {
  'owner' => '归属者',
  'contributor' => '贡献者',
  _ => relation,
};

String _marketReasonLabel(String reasonCode) => switch (reasonCode) {
  'metadata-incomplete' => '元信息不完整',
  'repository-unreachable' => '仓库无法访问',
  'invalid-artifact' => '资源文件无效',
  'invalid-version' => '版本信息无效',
  'malware-risk' => '存在安全风险',
  'policy-violation' => '不符合市场规范',
  'duplicate-entry' => '重复条目',
  _ => reasonCode,
};

IconData _marketNotificationIcon(String kind) => switch (kind) {
  'comment_new' => Icons.comment_outlined,
  'comment_reply' => Icons.reply_outlined,
  'review_approved' => Icons.verified_outlined,
  'review_rejected' => Icons.block_outlined,
  'review_changes' => Icons.rate_review_outlined,
  'entry_curated' => Icons.star_outline,
  _ => Icons.notifications_outlined,
};

String _marketNotificationTitle(core_proxy.MarketNotification notice) {
  final entrySuffix = notice.entryId == null ? '' : ' · ${notice.entryId}';
  return switch (notice.kind) {
    'comment_new' => '收到新评论$entrySuffix',
    'comment_reply' => '评论有新回复$entrySuffix',
    'review_approved' => '审核已通过$entrySuffix',
    'review_rejected' => '审核已拒绝$entrySuffix',
    'review_changes' => '审核要求修改$entrySuffix',
    'entry_curated' => '精选状态已更新$entrySuffix',
    _ => notice.title.isEmpty ? notice.kind : notice.title,
  };
}

String _marketNotificationBody(core_proxy.MarketNotification notice) {
  final body = notice.body.trim();
  return switch (notice.kind) {
    'comment_new' => body.isEmpty ? '有人在你的条目下发表了评论。' : body,
    'comment_reply' => body.isEmpty ? '有人回复了你的评论。' : body,
    'review_approved' => '你的提交已通过审核。',
    'review_rejected' => '你的提交未通过审核。',
    'review_changes' => '审核员要求你修改后重新提交。',
    'entry_curated' => '条目的精选状态发生变化。',
    _ => body.isEmpty ? '你有一条新的市场通知。' : body,
  };
}

String marketSortMetric(MarketSortOption option) => switch (option) {
  MarketSortOption.updated => 'updated',
  MarketSortOption.likes => 'likes',
  MarketSortOption.downloads => 'downloads',
};

int _entryDownloads(core_proxy.MarketEntrySummary entry) {
  return entry.downloadCount > entry.downloads
      ? entry.downloadCount
      : entry.downloads;
}

int _pageCount(int total, int pageSize) {
  final size = pageSize <= 0 ? 50 : pageSize;
  return ((total + size - 1) ~/ size).clamp(1, 1 << 30);
}

String _safePackageId(String raw) {
  final normalized = raw
      .trim()
      .replaceAll(RegExp(r'[^a-zA-Z0-9_]'), '_')
      .replaceAll(RegExp(r'_+'), '_')
      .replaceAll(RegExp(r'^_|_$'), '');
  return normalized.isEmpty ? 'market_item' : normalized;
}

class _MarketCategoryScopeHeader extends StatelessWidget {
  const _MarketCategoryScopeHeader({required this.title});

  final String title;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: colorScheme.surface.withValues(alpha: 0.72),
      layer: OperitGlassSurfaceLayer.panel,
      transparentAlpha: 0.035,
      clip: false,
      material: true,
      child: ListTile(
        dense: true,
        leading: const Icon(Icons.category_outlined),
        title: Text('分类：$title'),
        subtitle: const Text('可按类型、排序和精选继续筛选'),
      ),
    );
  }
}

class _MarketTypeFilterBar extends StatelessWidget {
  const _MarketTypeFilterBar({
    required this.selectedType,
    required this.onChanged,
  });

  final String? selectedType;
  final ValueChanged<String?> onChanged;

  @override
  Widget build(BuildContext context) {
    final selectedIndex = _marketTypeTabIndex(selectedType);
    final colorScheme = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.surface,
        border: Border(bottom: BorderSide(color: colorScheme.outlineVariant)),
      ),
      child: DefaultTabController(
        key: ValueKey<String>(selectedType ?? 'all'),
        length: _marketTypeTabs.length,
        initialIndex: selectedIndex,
        child: TabBar(
          isScrollable: true,
          tabAlignment: TabAlignment.start,
          padding: const EdgeInsets.symmetric(horizontal: 12),
          dividerColor: Colors.transparent,
          indicatorColor: colorScheme.primary,
          indicatorWeight: 2,
          indicatorSize: TabBarIndicatorSize.tab,
          labelColor: colorScheme.primary,
          unselectedLabelColor: colorScheme.onSurfaceVariant,
          labelStyle: Theme.of(
            context,
          ).textTheme.bodySmall?.copyWith(fontWeight: FontWeight.w600),
          unselectedLabelStyle: Theme.of(context).textTheme.bodySmall,
          onTap: (index) => onChanged(_marketTypeTabs[index].type),
          tabs: <Widget>[
            for (final tab in _marketTypeTabs)
              SizedBox(
                height: 48,
                child: Tab(
                  child: Text(
                    tab.label,
                    maxLines: 1,
                    softWrap: false,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

const List<_MarketTypeTabSpec> _marketTypeTabs = <_MarketTypeTabSpec>[
  _MarketTypeTabSpec(type: null, label: '全部'),
  _MarketTypeTabSpec(type: 'script', label: '脚本'),
  _MarketTypeTabSpec(type: 'package', label: '包'),
  _MarketTypeTabSpec(type: 'skill', label: '技能'),
  _MarketTypeTabSpec(type: 'mcp', label: 'MCP'),
];

int _marketTypeTabIndex(String? type) {
  for (var index = 0; index < _marketTypeTabs.length; index += 1) {
    if (_marketTypeTabs[index].type == type) {
      return index;
    }
  }
  throw StateError('Unknown market type tab: $type');
}

class _MarketTypeTabSpec {
  const _MarketTypeTabSpec({required this.type, required this.label});

  final String? type;
  final String label;
}

class _MarketCategoriesPane extends StatefulWidget {
  const _MarketCategoriesPane({required this.clients});

  final GeneratedCoreProxyClients clients;

  @override
  State<_MarketCategoriesPane> createState() => _MarketCategoriesPaneState();
}

class _MarketCategoriesPaneState extends State<_MarketCategoriesPane> {
  bool _loading = true;
  String? _errorMessage;
  List<core_proxy.MarketCategoryInfo> _categories =
      <core_proxy.MarketCategoryInfo>[];

  GeneratedProvidersMarketStatsApiServiceCoreProxy get _market =>
      widget.clients.providersMarketStatsApiService;

  @override
  void initState() {
    super.initState();
    _loadCategories();
  }

  Future<void> _loadCategories() async {
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    try {
      final manifest = await _market.getManifest();
      if (!mounted) {
        return;
      }
      setState(() {
        _categories = manifest.categories;
        _loading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load market categories: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final error = _errorMessage;
    if (_loading && _categories.isEmpty) {
      return const M3LoadingPane();
    }
    if (error != null && _categories.isEmpty) {
      return EmptyState(
        icon: Icons.error_outline,
        title: '加载失败',
        message: error,
        action: TextButton.icon(
          onPressed: _loadCategories,
          icon: const Icon(Icons.refresh),
          label: const Text('刷新'),
        ),
      );
    }
    return RefreshIndicator(
      onRefresh: _loadCategories,
      child: CustomScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        slivers: <Widget>[
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(12, 8, 12, 0),
            sliver: _MarketCategoryGridSliver(
              categories: _categories,
              onOpenCategory: _openCategory,
            ),
          ),
          const SliverToBoxAdapter(child: SizedBox(height: 120)),
        ],
      ),
    );
  }

  void _openCategory(core_proxy.MarketCategoryInfo category) {
    final entry = ScreenRouteRegistry.toEntry(
      screen: MarketScreenRoute(
        initialTab: MarketHomeTab.all,
        categoryId: category.id,
        categoryName: category.name,
      ),
    );
    AppRouterGateway.navigate(
      routeId: entry.routeId,
      args: entry.args,
      source: entry.source,
    );
  }
}

class _MarketCategoryGridSliver extends StatelessWidget {
  const _MarketCategoryGridSliver({
    required this.categories,
    required this.onOpenCategory,
  });

  final List<core_proxy.MarketCategoryInfo> categories;
  final ValueChanged<core_proxy.MarketCategoryInfo> onOpenCategory;

  @override
  Widget build(BuildContext context) {
    return SliverLayoutBuilder(
      builder: (context, constraints) {
        final columnCount = constraints.crossAxisExtent >= 1280
            ? 3
            : constraints.crossAxisExtent >= 760
            ? 2
            : 1;
        return SliverGrid.builder(
          itemCount: categories.length,
          gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
            crossAxisCount: columnCount,
            crossAxisSpacing: 10,
            mainAxisSpacing: 8,
            mainAxisExtent: 128,
          ),
          itemBuilder: (context, index) {
            final category = categories[index];
            return _MarketCategoryGridCard(
              category: category,
              onTap: () => onOpenCategory(category),
            );
          },
        );
      },
    );
  }
}

class _MarketCategoryGridCard extends StatelessWidget {
  const _MarketCategoryGridCard({required this.category, required this.onTap});

  final core_proxy.MarketCategoryInfo category;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final borderRadius = BorderRadius.circular(16);
    final description = category.description?.trim();
    return OperitGlassSurface(
      color: colorScheme.surface,
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: borderRadius,
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      material: true,
      child: InkWell(
        borderRadius: borderRadius,
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: <Widget>[
              Container(
                width: 42,
                height: 42,
                decoration: BoxDecoration(
                  color: colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(14),
                ),
                child: Icon(
                  Icons.category_outlined,
                  color: colorScheme.onPrimaryContainer,
                  size: 22,
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      category.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 4),
                    if (description != null && description.isNotEmpty)
                      Text(
                        description,
                        maxLines: 2,
                        softWrap: true,
                        style: textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    const SizedBox(height: 4),
                    Text(
                      category.id,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.labelSmall?.copyWith(
                        color: colorScheme.outline,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Icon(Icons.chevron_right, size: 20, color: colorScheme.outline),
            ],
          ),
        ),
      ),
    );
  }
}

class _MarketMinePane extends StatefulWidget {
  const _MarketMinePane({required this.clients});

  final GeneratedCoreProxyClients clients;

  @override
  State<_MarketMinePane> createState() => _MarketMinePaneState();
}

class _MarketMinePaneState extends State<_MarketMinePane> {
  bool _loading = true;
  bool _loggedIn = false;
  core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser? _user;

  GeneratedPreferencesGitHubAuthPreferencesCoreProxy get _githubAuth =>
      widget.clients.preferencesGitHubAuthPreferences;

  @override
  void initState() {
    super.initState();
    _loadAuthState();
  }

  Future<void> _loadAuthState() async {
    setState(() {
      _loading = true;
    });
    try {
      final loggedIn = await _githubAuth.isLoggedIn();
      final user = await _githubAuth.getCurrentUserInfo();
      if (!mounted) {
        return;
      }
      setState(() {
        _loggedIn = loggedIn;
        _user = user;
        _loading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load GitHub auth state: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _loading = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  Future<void> _logout() async {
    try {
      await _githubAuth.logout();
      await _loadAuthState();
    } catch (error, stackTrace) {
      debugPrint('Failed to logout GitHub: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 120),
      children: <Widget>[
        if (_loading)
          const _MineAccountLoadingCard()
        else
          _MineAccountCard(
            loggedIn: _loggedIn,
            user: _user,
            onLogin: () => _showGitHubTokenDialog(context),
            onLogout: _logout,
          ),
        const SizedBox(height: 16),
        _MineSectionTitle(text: '管理'),
        _MineActionCard(
          icon: Icons.settings_outlined,
          title: '我的市场',
          subtitle: '查看发布记录、通知和审核状态。',
          onTap: () => _openArtifactManage(context),
        ),
        const SizedBox(height: 16),
        _MineSectionTitle(text: '发布'),
        _MineActionCard(
          icon: Icons.add,
          title: '发布 Artifact',
          subtitle: '发布脚本、包或运行时资源。',
          onTap: () => _openArtifactPublish(context),
        ),
        const SizedBox(height: 10),
        _MineActionCard(
          icon: Icons.psychology_outlined,
          title: '发布 Skill',
          subtitle: '发布 GitHub 仓库形式的技能。',
          onTap: () => _openRepoPublish(context, 'skill'),
        ),
        const SizedBox(height: 10),
        _MineActionCard(
          icon: Icons.hub_outlined,
          title: '发布 MCP',
          subtitle: '发布 GitHub 仓库形式的 MCP 服务。',
          onTap: () => _openRepoPublish(context, 'mcp'),
        ),
      ],
    );
  }

  void _openArtifactManage(BuildContext context) {
    if (!_loggedIn) {
      _showGitHubTokenDialog(context);
      return;
    }
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) => _ArtifactManageScreen(clients: widget.clients),
      ),
    );
  }

  void _openArtifactPublish(BuildContext context) {
    if (!_loggedIn) {
      _showGitHubTokenDialog(context);
      return;
    }
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) => ArtifactPublishScreen(clients: widget.clients),
      ),
    );
  }

  void _openRepoPublish(BuildContext context, String type) {
    if (!_loggedIn) {
      _showGitHubTokenDialog(context);
      return;
    }
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) =>
            RepoMarketPublishScreen(clients: widget.clients, type: type),
      ),
    );
  }

  void _showGitHubTokenDialog(BuildContext context) {
    final parentContext = context;
    final tokenController = TextEditingController();
    var saving = false;
    showDialog<void>(
      context: context,
      builder: (dialogContext) => StatefulBuilder(
        builder: (context, setDialogState) {
          Future<void> saveToken() async {
            final token = tokenController.text.trim();
            if (token.isEmpty || saving) {
              return;
            }
            setDialogState(() {
              saving = true;
            });
            try {
              await _githubAuth.updateAccessToken(
                accessToken: token,
                tokenType: 'bearer',
                grantedScope: null,
              );
              final apiUser = await widget
                  .clients
                  .providersMarketStatsApiService
                  .getCurrentGithubUser();
              await _githubAuth.saveAuthInfo(
                accessToken: token,
                tokenType: 'bearer',
                userInfo: <String, Object?>{
                  'id': apiUser.id.toString(),
                  'login': apiUser.login,
                  'name': apiUser.name,
                  'email': apiUser.email,
                  'avatar_url': apiUser.avatarUrl,
                  'bio': apiUser.bio,
                  'public_repos': apiUser.publicRepos,
                  'followers': apiUser.followers,
                  'following': apiUser.following,
                },
                grantedScope: null,
              );
              await _loadAuthState();
              if (!mounted ||
                  !dialogContext.mounted ||
                  !parentContext.mounted) {
                return;
              }
              Navigator.of(dialogContext).pop();
            } catch (error, stackTrace) {
              debugPrint('Failed to save GitHub token: $error\n$stackTrace');
              await _githubAuth.logout();
              if (!mounted || !parentContext.mounted) {
                return;
              }
              setDialogState(() {
                saving = false;
              });
              ScaffoldMessenger.of(parentContext).showSnackBar(
                SnackBar(
                  content: Text(error.toString()),
                  behavior: SnackBarBehavior.floating,
                ),
              );
            }
          }

          return AlertDialog(
            icon: const Icon(Icons.login),
            title: const Text('GitHub 登录'),
            content: TextField(
              controller: tokenController,
              enabled: !saving,
              obscureText: true,
              decoration: const InputDecoration(
                labelText: 'GitHub Token',
                border: OutlineInputBorder(),
              ),
              onSubmitted: (_) => saveToken(),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: saving ? null : () => Navigator.of(context).pop(),
                child: const Text('取消'),
              ),
              FilledButton.icon(
                onPressed: saving ? null : saveToken,
                icon: saving
                    ? M3LoadingIndicator(
                        size: 18,
                        color: Theme.of(context).colorScheme.onPrimary,
                      )
                    : const Icon(Icons.login),
                label: const Text('登录'),
              ),
            ],
          );
        },
      ),
    ).whenComplete(tokenController.dispose);
  }
}

class _MineAccountLoadingCard extends StatelessWidget {
  const _MineAccountLoadingCard();

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.42),
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(16),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      material: true,
      child: const ListTile(
        leading: M3LoadingIndicator(size: 24),
        title: Text('GitHub 账号'),
        subtitle: Text('正在读取登录状态'),
      ),
    );
  }
}

class _MineActionCard extends StatelessWidget {
  const _MineActionCard({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.38),
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(16),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      material: true,
      child: ListTile(
        onTap: onTap,
        leading: Icon(icon),
        title: Text(title),
        subtitle: Text(subtitle),
        trailing: const Icon(Icons.chevron_right),
      ),
    );
  }
}

class _MineSectionTitle extends StatelessWidget {
  const _MineSectionTitle({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 0, 4, 8),
      child: Text(
        text,
        style: Theme.of(
          context,
        ).textTheme.labelLarge?.copyWith(fontWeight: FontWeight.w700),
      ),
    );
  }
}

class _MineAccountCard extends StatelessWidget {
  const _MineAccountCard({
    required this.loggedIn,
    required this.user,
    required this.onLogin,
    required this.onLogout,
  });

  final bool loggedIn;
  final core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser? user;
  final VoidCallback onLogin;
  final VoidCallback onLogout;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final currentUser = user;
    return OperitGlassSurface(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.42),
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(16),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      material: true,
      child: ListTile(
        onTap: loggedIn ? null : onLogin,
        leading: _MineAccountAvatar(user: currentUser),
        title: Text(
          loggedIn && currentUser != null
              ? _githubDisplayName(currentUser)
              : 'GitHub 账号',
        ),
        subtitle: Text(
          loggedIn && currentUser != null
              ? '@${currentUser.login}'
              : '发布和管理市场内容需要登录。',
        ),
        trailing: loggedIn
            ? IconButton.outlined(
                onPressed: onLogout,
                icon: const Icon(Icons.logout, size: 18),
                tooltip: '退出',
              )
            : FilledButton.tonalIcon(
                onPressed: onLogin,
                icon: const Icon(Icons.login, size: 18),
                label: const Text('登录'),
              ),
      ),
    );
  }
}

class _MineAccountAvatar extends StatelessWidget {
  const _MineAccountAvatar({required this.user});

  final core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser? user;

  @override
  Widget build(BuildContext context) {
    final currentUser = user;
    if (currentUser != null && currentUser.avatarUrl.trim().isNotEmpty) {
      return CircleAvatar(
        backgroundImage: NetworkImage(currentUser.avatarUrl),
        radius: 22,
      );
    }
    return const Icon(Icons.account_circle_outlined, size: 44);
  }
}

String _githubDisplayName(
  core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser user,
) {
  final name = user.name?.trim();
  if (name != null && name.isNotEmpty) {
    return name;
  }
  return user.login;
}
