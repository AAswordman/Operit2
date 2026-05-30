// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../components/EmptyState.dart';

enum MarketHomeTab { artifact, skill, mcp, mine }

enum MarketSortOption { downloads, updated }

class UnifiedMarketScreen extends StatefulWidget {
  const UnifiedMarketScreen({super.key, GeneratedCoreProxyClients? clients})
    : clients =
          clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  @override
  State<UnifiedMarketScreen> createState() => _UnifiedMarketScreenState();
}

class _UnifiedMarketScreenState extends State<UnifiedMarketScreen> {
  MarketHomeTab _selectedTab = MarketHomeTab.artifact;
  MarketSortOption _sortOption = MarketSortOption.downloads;
  String _searchInput = '';
  String _searchQuery = '';
  Timer? _searchDebounce;

  @override
  void dispose() {
    _searchDebounce?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: <Widget>[
        DefaultTabController(
          key: ValueKey<MarketHomeTab>(_selectedTab),
          length: MarketHomeTab.values.length,
          initialIndex: _selectedTab.index,
          child: TabBar(
            onTap: (index) {
              setState(() {
                _selectedTab = MarketHomeTab.values[index];
                _searchInput = '';
                _searchQuery = '';
                _searchDebounce?.cancel();
              });
            },
            tabs: const <Widget>[
              Tab(text: 'Artifact'),
              Tab(text: 'Skill'),
              Tab(text: 'MCP'),
              Tab(text: 'Mine'),
            ],
          ),
        ),
        _MarketControls(
          query: _searchInput,
          sortOption: _sortOption,
          searchEnabled: _selectedTab != MarketHomeTab.mine,
          onQueryChanged: _onSearchChanged,
          onSortChanged: (sortOption) {
            setState(() {
              _sortOption = sortOption;
            });
          },
        ),
        Expanded(
          child: switch (_selectedTab) {
            MarketHomeTab.artifact => _ArtifactMarketPane(
              clients: widget.clients,
              sortOption: _sortOption,
              searchQuery: _searchQuery,
            ),
            MarketHomeTab.skill => _IssueMarketPane(
              clients: widget.clients,
              type: 'skill',
              sortOption: _sortOption,
              searchQuery: _searchQuery,
            ),
            MarketHomeTab.mcp => _IssueMarketPane(
              clients: widget.clients,
              type: 'mcp',
              sortOption: _sortOption,
              searchQuery: _searchQuery,
            ),
            MarketHomeTab.mine => const _MarketMinePane(),
          },
        ),
      ],
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
    });
  }
}

class _ArtifactMarketPane extends StatefulWidget {
  const _ArtifactMarketPane({
    required this.clients,
    required this.sortOption,
    required this.searchQuery,
  });

  final GeneratedCoreProxyClients clients;
  final MarketSortOption sortOption;
  final String searchQuery;

  @override
  State<_ArtifactMarketPane> createState() => _ArtifactMarketPaneState();
}

class _ArtifactMarketPaneState extends State<_ArtifactMarketPane> {
  bool _loading = true;
  bool _loadingMore = false;
  String? _errorMessage;
  int _page = 1;
  int _totalPages = 1;
  List<core_proxy.ArtifactProjectRankEntryResponse> _items =
      <core_proxy.ArtifactProjectRankEntryResponse>[];

  GeneratedApiMarketStatsApiServiceCoreProxy get _market =>
      widget.clients.apiMarketStatsApiService;

  @override
  void initState() {
    super.initState();
    _loadFirstPage();
  }

  @override
  void didUpdateWidget(covariant _ArtifactMarketPane oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.sortOption != widget.sortOption) {
      _loadFirstPage();
    }
  }

  Future<void> _loadFirstPage() async {
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    try {
      final page = await _market.getArtifactRankPage(
        type: 'artifact',
        metric: _metric,
        page: 1,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _items = page.items;
        _page = page.page;
        _totalPages = page.totalPages;
        _loading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load artifact market: $error\n$stackTrace');
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
    if (_loadingMore || _page >= _totalPages) {
      return;
    }
    setState(() {
      _loadingMore = true;
    });
    try {
      final page = await _market.getArtifactRankPage(
        type: 'artifact',
        metric: _metric,
        page: _page + 1,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _items = <core_proxy.ArtifactProjectRankEntryResponse>[
          ..._items,
          ...page.items,
        ];
        _page = page.page;
        _totalPages = page.totalPages;
        _loadingMore = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load more artifact market: $error\n$stackTrace');
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

  String get _metric => switch (widget.sortOption) {
    MarketSortOption.downloads => 'downloads',
    MarketSortOption.updated => 'updated',
  };

  @override
  Widget build(BuildContext context) {
    final error = _errorMessage;
    if (_loading && _items.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }
    if (error != null && _items.isEmpty) {
      return EmptyState(
        icon: Icons.error_outline,
        title: '加载失败',
        message: error,
        action: TextButton.icon(
          onPressed: _loadFirstPage,
          icon: const Icon(Icons.refresh),
          label: const Text('刷新'),
        ),
      );
    }
    final query = widget.searchQuery.toLowerCase();
    final displayed = _items
        .where(
          (item) =>
              query.isEmpty ||
              item.projectDisplayName.toLowerCase().contains(query) ||
              item.projectDescription.toLowerCase().contains(query) ||
              item.rootPublisherLogin.toLowerCase().contains(query),
        )
        .toList(growable: false);
    return _MarketList(
      isLoading: _loading,
      isLoadingMore: _loadingMore,
      hasMore: _page < _totalPages && widget.searchQuery.trim().isEmpty,
      isEmpty: displayed.isEmpty,
      emptyTitle: widget.searchQuery.trim().isEmpty ? '暂无 Artifact' : '没有匹配结果',
      onRefresh: _loadFirstPage,
      onLoadMore: _loadMore,
      children: displayed
          .map(
            (item) => _MarketCard(
              title: item.projectDisplayName,
              description: item.projectDescription,
              author: item.rootPublisherLogin,
              downloads: item.downloads,
              likes: item.likes,
              updatedAt: item.latestPublishedAt,
              onTap: () => _showDetails(
                item.projectDisplayName,
                item.projectDescription,
                <String>[
                  'ID: ${item.projectId}',
                  '作者: ${item.rootPublisherLogin}',
                  '下载: ${item.downloads}',
                  '喜欢: ${item.likes}',
                  '贡献者: ${item.contributorCount}',
                  '最新节点: ${item.latestNodeId}',
                ],
              ),
            ),
          )
          .toList(growable: false),
    );
  }

  void _showDetails(String title, String description, List<String> rows) {
    showDialog<void>(
      context: context,
      builder: (context) => _MarketDetailsDialog(
        title: title,
        description: description,
        rows: rows,
      ),
    );
  }
}

class _IssueMarketPane extends StatefulWidget {
  const _IssueMarketPane({
    required this.clients,
    required this.type,
    required this.sortOption,
    required this.searchQuery,
  });

  final GeneratedCoreProxyClients clients;
  final String type;
  final MarketSortOption sortOption;
  final String searchQuery;

  @override
  State<_IssueMarketPane> createState() => _IssueMarketPaneState();
}

class _IssueMarketPaneState extends State<_IssueMarketPane> {
  bool _loading = true;
  bool _loadingMore = false;
  String? _errorMessage;
  int _page = 1;
  int _totalPages = 1;
  List<core_proxy.MarketRankIssueEntryResponse> _items =
      <core_proxy.MarketRankIssueEntryResponse>[];

  GeneratedApiMarketStatsApiServiceCoreProxy get _market =>
      widget.clients.apiMarketStatsApiService;

  @override
  void initState() {
    super.initState();
    _loadFirstPage();
  }

  @override
  void didUpdateWidget(covariant _IssueMarketPane oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.sortOption != widget.sortOption ||
        oldWidget.type != widget.type) {
      _loadFirstPage();
    }
  }

  Future<void> _loadFirstPage() async {
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    try {
      final page = await _market.getRankPage(
        type: widget.type,
        metric: _metric,
        page: 1,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _items = page.items;
        _page = page.page;
        _totalPages = page.totalPages;
        _loading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load ${widget.type} market: $error\n$stackTrace');
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
    if (_loadingMore || _page >= _totalPages) {
      return;
    }
    setState(() {
      _loadingMore = true;
    });
    try {
      final page = await _market.getRankPage(
        type: widget.type,
        metric: _metric,
        page: _page + 1,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _items = <core_proxy.MarketRankIssueEntryResponse>[
          ..._items,
          ...page.items,
        ];
        _page = page.page;
        _totalPages = page.totalPages;
        _loadingMore = false;
      });
    } catch (error, stackTrace) {
      debugPrint(
        'Failed to load more ${widget.type} market: $error\n$stackTrace',
      );
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

  String get _metric => switch (widget.sortOption) {
    MarketSortOption.downloads => 'downloads',
    MarketSortOption.updated => 'updated',
  };

  @override
  Widget build(BuildContext context) {
    final error = _errorMessage;
    if (_loading && _items.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }
    if (error != null && _items.isEmpty) {
      return EmptyState(
        icon: Icons.error_outline,
        title: '加载失败',
        message: error,
        action: TextButton.icon(
          onPressed: _loadFirstPage,
          icon: const Icon(Icons.refresh),
          label: const Text('刷新'),
        ),
      );
    }
    final query = widget.searchQuery.toLowerCase();
    final displayed = _items
        .where(
          (item) =>
              query.isEmpty ||
              item.displayTitle.toLowerCase().contains(query) ||
              item.summaryDescription.toLowerCase().contains(query) ||
              item.authorLogin.toLowerCase().contains(query),
        )
        .toList(growable: false);
    return _MarketList(
      isLoading: _loading,
      isLoadingMore: _loadingMore,
      hasMore: _page < _totalPages && widget.searchQuery.trim().isEmpty,
      isEmpty: displayed.isEmpty,
      emptyTitle: widget.searchQuery.trim().isEmpty ? '暂无项目' : '没有匹配结果',
      onRefresh: _loadFirstPage,
      onLoadMore: _loadMore,
      children: displayed
          .map(
            (item) => _MarketCard(
              title: item.displayTitle,
              description: item.summaryDescription,
              author: item.authorLogin,
              downloads: item.downloads,
              likes: item.issue.reactions?.thumbsUp ?? 0,
              updatedAt: item.updatedAt,
              onTap: () => _showDetails(
                item.displayTitle,
                item.summaryDescription,
                <String>[
                  'Issue: #${item.issue.number}',
                  '作者: ${item.authorLogin}',
                  '下载: ${item.downloads}',
                  '更新时间: ${item.updatedAt}',
                  item.issue.htmlUrl,
                ],
              ),
            ),
          )
          .toList(growable: false),
    );
  }

  void _showDetails(String title, String description, List<String> rows) {
    showDialog<void>(
      context: context,
      builder: (context) => _MarketDetailsDialog(
        title: title,
        description: description,
        rows: rows,
      ),
    );
  }
}

class _MarketList extends StatelessWidget {
  const _MarketList({
    required this.isLoading,
    required this.isLoadingMore,
    required this.hasMore,
    required this.isEmpty,
    required this.emptyTitle,
    required this.onRefresh,
    required this.onLoadMore,
    required this.children,
  });

  final bool isLoading;
  final bool isLoadingMore;
  final bool hasMore;
  final bool isEmpty;
  final String emptyTitle;
  final AsyncCallback onRefresh;
  final VoidCallback onLoadMore;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return NotificationListener<ScrollNotification>(
      onNotification: (notification) {
        if (notification.metrics.extentAfter < 360 &&
            hasMore &&
            !isLoadingMore) {
          onLoadMore();
        }
        return false;
      },
      child: Stack(
        children: <Widget>[
          RefreshIndicator(
            onRefresh: onRefresh,
            child: ListView(
              physics: const AlwaysScrollableScrollPhysics(),
              padding: const EdgeInsets.fromLTRB(12, 4, 12, 120),
              children: <Widget>[
                if (isEmpty)
                  EmptyState(
                    icon: Icons.store_outlined,
                    title: emptyTitle,
                    message: '刷新或调整关键词后重试。',
                    scrollable: false,
                  )
                else
                  ...children,
                if (isLoadingMore)
                  const Padding(
                    padding: EdgeInsets.symmetric(vertical: 18),
                    child: Center(child: CircularProgressIndicator()),
                  ),
              ],
            ),
          ),
          if (isLoading && !isEmpty)
            const Center(child: CircularProgressIndicator()),
        ],
      ),
    );
  }
}

class _MarketControls extends StatelessWidget {
  const _MarketControls({
    required this.query,
    required this.sortOption,
    required this.searchEnabled,
    required this.onQueryChanged,
    required this.onSortChanged,
  });

  final String query;
  final MarketSortOption sortOption;
  final bool searchEnabled;
  final ValueChanged<String> onQueryChanged;
  final ValueChanged<MarketSortOption> onSortChanged;

  @override
  Widget build(BuildContext context) {
    if (!searchEnabled) {
      return const SizedBox(height: 8);
    }
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
      child: Row(
        children: <Widget>[
          Expanded(
            child: SearchBar(
              leading: const Icon(Icons.search),
              hintText: '搜索市场',
              elevation: const WidgetStatePropertyAll<double>(0),
              controller: TextEditingController(text: query)
                ..selection = TextSelection.collapsed(offset: query.length),
              onChanged: onQueryChanged,
            ),
          ),
          const SizedBox(width: 8),
          SegmentedButton<MarketSortOption>(
            segments: const <ButtonSegment<MarketSortOption>>[
              ButtonSegment(
                value: MarketSortOption.downloads,
                icon: Icon(Icons.download_outlined),
              ),
              ButtonSegment(
                value: MarketSortOption.updated,
                icon: Icon(Icons.update),
              ),
            ],
            selected: <MarketSortOption>{sortOption},
            showSelectedIcon: false,
            onSelectionChanged: (value) => onSortChanged(value.single),
          ),
        ],
      ),
    );
  }
}

class _MarketCard extends StatelessWidget {
  const _MarketCard({
    required this.title,
    required this.description,
    required this.author,
    required this.downloads,
    required this.likes,
    required this.updatedAt,
    required this.onTap,
  });

  final String title;
  final String description;
  final String author;
  final int downloads;
  final int likes;
  final String? updatedAt;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Card(
      elevation: 1,
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            children: <Widget>[
              CircleAvatar(
                backgroundColor: colorScheme.primaryContainer,
                foregroundColor: colorScheme.onPrimaryContainer,
                child: Text(title.trim().isEmpty ? '?' : title.trim()[0]),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    if (description.trim().isNotEmpty)
                      Text(
                        description,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    const SizedBox(height: 8),
                    Wrap(
                      spacing: 6,
                      runSpacing: 6,
                      children: <Widget>[
                        _SmallChip(text: author),
                        _SmallChip(text: '$downloads 下载'),
                        if (likes > 0) _SmallChip(text: '$likes 喜欢'),
                        if (updatedAt != null) _SmallChip(text: updatedAt!),
                      ],
                    ),
                  ],
                ),
              ),
              const Icon(Icons.chevron_right),
            ],
          ),
        ),
      ),
    );
  }
}

class _MarketDetailsDialog extends StatelessWidget {
  const _MarketDetailsDialog({
    required this.title,
    required this.description,
    required this.rows,
  });

  final String title;
  final String description;
  final List<String> rows;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      icon: const Icon(Icons.store_outlined),
      title: Text(title),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 620, maxHeight: 520),
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              if (description.trim().isNotEmpty) Text(description),
              const SizedBox(height: 12),
              for (final row in rows)
                Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: SelectableText(row),
                ),
            ],
          ),
        ),
      ),
      actions: <Widget>[
        FilledButton.tonal(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('关闭'),
        ),
      ],
    );
  }
}

class _SmallChip extends StatelessWidget {
  const _SmallChip({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    if (text.trim().isEmpty) {
      return const SizedBox.shrink();
    }
    final colorScheme = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        child: Text(
          text,
          style: Theme.of(
            context,
          ).textTheme.labelSmall?.copyWith(color: colorScheme.onSurfaceVariant),
        ),
      ),
    );
  }
}

class _MarketMinePane extends StatelessWidget {
  const _MarketMinePane();

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 120),
      children: const <Widget>[
        _MineActionCard(icon: Icons.settings_outlined, title: '管理 Artifact'),
        SizedBox(height: 8),
        _MineActionCard(icon: Icons.settings_outlined, title: '管理 Skill'),
        SizedBox(height: 8),
        _MineActionCard(icon: Icons.settings_outlined, title: '管理 MCP'),
        SizedBox(height: 16),
        _MineActionCard(icon: Icons.add, title: '发布 Artifact'),
        SizedBox(height: 8),
        _MineActionCard(icon: Icons.add, title: '发布 Skill'),
        SizedBox(height: 8),
        _MineActionCard(icon: Icons.add, title: '发布 MCP'),
      ],
    );
  }
}

class _MineActionCard extends StatelessWidget {
  const _MineActionCard({required this.icon, required this.title});

  final IconData icon;
  final String title;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: ListTile(
        leading: Icon(icon),
        title: Text(title),
        trailing: const Icon(Icons.chevron_right),
      ),
    );
  }
}
