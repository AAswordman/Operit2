// ignore_for_file: file_names

import 'dart:convert';
import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../components/EmptyState.dart';

enum MarketHomeTab { artifact, skill, mcp, mine }

enum MarketSortOption { downloads, updated }

class UnifiedMarketScreen extends StatefulWidget {
  const UnifiedMarketScreen({
    super.key,
    this.initialTab = MarketHomeTab.artifact,
    GeneratedCoreProxyClients? clients,
  }) : clients =
           clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final MarketHomeTab initialTab;
  final GeneratedCoreProxyClients clients;

  @override
  State<UnifiedMarketScreen> createState() => _UnifiedMarketScreenState();
}

class _UnifiedMarketScreenState extends State<UnifiedMarketScreen> {
  late MarketHomeTab _selectedTab = widget.initialTab;
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
    return Material(
      color: Theme.of(context).colorScheme.surface,
      child: Column(
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
              MarketHomeTab.mine => _MarketMinePane(clients: widget.clients),
            },
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
  final Set<String> _busyProjectIds = <String>{};
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
      children:
          _buildMarketChildren<core_proxy.ArtifactProjectRankEntryResponse>(
            items: displayed,
            groupByUpdatedDate: widget.sortOption == MarketSortOption.updated,
            updatedAt: (item) => item.latestPublishedAt ?? '',
            itemBuilder: (item) => _MarketCard(
              title: item.projectDisplayName,
              description: item.projectDescription,
              author: item.rootPublisherLogin,
              downloads: item.downloads,
              likes: item.likes,
              updatedAt: item.latestPublishedAt,
              statusLabel: item.defaultNode == null ? '需要详情' : '可下载',
              actionLabel: '下载',
              actionIcon: Icons.download_outlined,
              actionBusy: _busyProjectIds.contains(item.projectId),
              onAction: () => _downloadArtifact(item),
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
          ),
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

  Future<void> _downloadArtifact(
    core_proxy.ArtifactProjectRankEntryResponse item,
  ) async {
    final node = item.defaultNode;
    if (node == null || node.downloadUrl.trim().isEmpty) {
      await _showArtifactProjectDetails(item.projectId);
      return;
    }
    setState(() {
      _busyProjectIds.add(item.projectId);
    });
    try {
      await _market.trackDownload(
        type: item.type.isEmpty ? 'artifact' : item.type,
        id: item.projectId,
        targetUrl: node.downloadUrl,
      );
      final uri = Uri.parse(node.downloadUrl);
      await launchUrl(uri, mode: LaunchMode.externalApplication);
    } catch (error, stackTrace) {
      debugPrint('Failed to open artifact download: $error\n$stackTrace');
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
          _busyProjectIds.remove(item.projectId);
        });
      }
    }
  }

  Future<void> _showArtifactProjectDetails(String projectId) async {
    setState(() {
      _busyProjectIds.add(projectId);
    });
    try {
      final project = await _market.getArtifactProject(projectId: projectId);
      if (!mounted) {
        return;
      }
      _showDetails(project.projectDisplayName, project.projectDescription, <
        String
      >[
        'ID: ${project.projectId}',
        '作者: ${project.rootPublisherLogin}',
        '下载: ${project.downloads}',
        '喜欢: ${project.likes}',
        '节点: ${project.nodes.length}',
        for (final node in project.nodes)
          '${node.displayName.isEmpty ? node.nodeId : node.displayName}  ${node.version}  ${node.downloadUrl}',
      ]);
    } catch (error, stackTrace) {
      debugPrint('Failed to load artifact project: $error\n$stackTrace');
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
          _busyProjectIds.remove(projectId);
        });
      }
    }
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
  final Set<String> _busyIssueIds = <String>{};
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
      children: _buildMarketChildren<core_proxy.MarketRankIssueEntryResponse>(
        items: displayed,
        groupByUpdatedDate: widget.sortOption == MarketSortOption.updated,
        updatedAt: (item) => item.updatedAt ?? '',
        itemBuilder: (item) => _MarketCard(
          title: item.displayTitle,
          description: item.summaryDescription,
          author: item.authorLogin,
          downloads: item.downloads,
          likes: item.issue.reactions?.thumbsUp ?? 0,
          updatedAt: item.updatedAt,
          statusLabel: _issueStatusLabel(item),
          actionLabel: widget.type == 'skill' ? '安装' : '安装',
          actionIcon: Icons.download_outlined,
          actionBusy: _busyIssueIds.contains(item.id),
          onAction: () => _installIssueItem(item),
          onTap: () =>
              _showDetails(item.displayTitle, item.summaryDescription, <String>[
                'Issue: #${item.issue.number}',
                '作者: ${item.authorLogin}',
                '下载: ${item.downloads}',
                '更新时间: ${item.updatedAt}',
                item.issue.htmlUrl,
              ]),
        ),
      ),
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

  String _issueStatusLabel(core_proxy.MarketRankIssueEntryResponse item) {
    if (item.issue.state != 'open') {
      return '已关闭';
    }
    final metadata = _marketIssueMetadata(item.issue, widget.type);
    if (widget.type == 'skill') {
      return (metadata['repositoryUrl'] ?? '').trim().isEmpty ? '缺少仓库' : '可安装';
    }
    return (metadata['repositoryUrl'] ?? '').trim().isEmpty ||
            (metadata['installConfig'] ?? '').trim().isEmpty
        ? '缺少配置'
        : '可安装';
  }

  Future<void> _installIssueItem(
    core_proxy.MarketRankIssueEntryResponse item,
  ) async {
    setState(() {
      _busyIssueIds.add(item.id);
    });
    try {
      final metadata = _marketIssueMetadata(item.issue, widget.type);
      if (widget.type == 'skill') {
        await _installSkill(item, metadata);
      } else {
        await _installMcp(item, metadata);
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to install ${widget.type}: $error\n$stackTrace');
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
          _busyIssueIds.remove(item.id);
        });
      }
    }
  }

  Future<void> _installSkill(
    core_proxy.MarketRankIssueEntryResponse item,
    Map<String, String> metadata,
  ) async {
    final repoUrl = metadata['repositoryUrl']?.trim() ?? '';
    if (repoUrl.isEmpty) {
      throw StateError('技能缺少 repositoryUrl');
    }
    final result = await widget.clients.skillRepository
        .importSkillFromGitHubRepo(repoUrl: repoUrl);
    await _market.trackDownload(type: 'skill', id: item.id, targetUrl: repoUrl);
    if (!mounted) {
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(result), behavior: SnackBarBehavior.floating),
    );
  }

  Future<void> _installMcp(
    core_proxy.MarketRankIssueEntryResponse item,
    Map<String, String> metadata,
  ) async {
    final repoUrl = metadata['repositoryUrl']?.trim() ?? '';
    final installConfig = metadata['installConfig']?.trim() ?? '';
    if (repoUrl.isEmpty) {
      throw StateError('MCP 缺少 repositoryUrl');
    }
    if (installConfig.isEmpty) {
      throw StateError('MCP 缺少 installConfig');
    }
    final result = await widget.clients.mcpRepository
        .installMcpServerWithObjectForFlutter(
          pluginId: _safePackageId(item.displayTitle),
          repoUrl: repoUrl,
          name: item.displayTitle,
          description: item.summaryDescription,
          mcpConfig: installConfig,
        );
    await _market.trackDownload(type: 'mcp', id: item.id, targetUrl: repoUrl);
    if (!mounted) {
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(result), behavior: SnackBarBehavior.floating),
    );
  }
}

List<Widget> _buildMarketChildren<T>({
  required List<T> items,
  required bool groupByUpdatedDate,
  required String Function(T item) updatedAt,
  required Widget Function(T item) itemBuilder,
}) {
  if (!groupByUpdatedDate) {
    return items.map(itemBuilder).toList(growable: false);
  }
  final children = <Widget>[];
  String? currentLabel;
  for (final item in items) {
    final label = _marketUpdatedDateLabel(updatedAt(item));
    if (label != currentLabel) {
      currentLabel = label;
      children.add(_MarketDateHeader(text: label));
    }
    children.add(itemBuilder(item));
  }
  return children;
}

String _marketUpdatedDateLabel(String value) {
  final trimmed = value.trim();
  if (trimmed.isEmpty) {
    return '更早';
  }
  return trimmed.length >= 10 ? trimmed.substring(0, 10) : trimmed;
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
    required this.statusLabel,
    required this.actionLabel,
    required this.actionIcon,
    required this.actionBusy,
    required this.onAction,
    required this.onTap,
  });

  final String title;
  final String description;
  final String author;
  final int downloads;
  final int likes;
  final String? updatedAt;
  final String statusLabel;
  final String actionLabel;
  final IconData actionIcon;
  final bool actionBusy;
  final VoidCallback onAction;
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
            crossAxisAlignment: CrossAxisAlignment.center,
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
                        _SmallChip(text: statusLabel),
                      ],
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Tooltip(
                message: actionLabel,
                child: IconButton.filledTonal(
                  onPressed: actionBusy ? null : onAction,
                  icon: actionBusy
                      ? const SizedBox(
                          width: 18,
                          height: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : Icon(actionIcon, size: 18),
                  style: IconButton.styleFrom(
                    fixedSize: const Size.square(34),
                    minimumSize: const Size.square(34),
                    padding: EdgeInsets.zero,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _MarketDateHeader extends StatelessWidget {
  const _MarketDateHeader({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 10, 4, 2),
      child: Text(
        text,
        style: Theme.of(context).textTheme.labelLarge?.copyWith(
          fontWeight: FontWeight.w600,
          color: Theme.of(context).colorScheme.onSurfaceVariant,
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

Map<String, String> _marketIssueMetadata(
  core_proxy.GitHubIssue issue,
  String type,
) {
  final body = issue.body ?? '';
  final prefix = type == 'skill'
      ? '<!-- operit-skill-json: '
      : '<!-- operit-mcp-json: ';
  final start = body.indexOf(prefix);
  if (start < 0) {
    return <String, String>{};
  }
  final jsonStart = start + prefix.length;
  final end = body.indexOf(' -->', jsonStart);
  if (end <= jsonStart) {
    return <String, String>{};
  }
  final decoded = jsonDecode(body.substring(jsonStart, end));
  if (decoded is! Map) {
    return <String, String>{};
  }
  final metadata = decoded.map(
    (key, value) => MapEntry(key.toString(), value?.toString() ?? ''),
  );
  if ((metadata['repositoryUrl'] ?? '').isEmpty &&
      (metadata['repoUrl'] ?? '').isNotEmpty) {
    metadata['repositoryUrl'] = metadata['repoUrl']!;
  }
  if ((metadata['installConfig'] ?? '').isEmpty &&
      (metadata['installCommand'] ?? '').isNotEmpty) {
    metadata['installConfig'] = metadata['installCommand']!;
  }
  return metadata;
}

String _safePackageId(String raw) {
  final normalized = raw
      .trim()
      .replaceAll(RegExp(r'[^a-zA-Z0-9_]'), '_')
      .replaceAll(RegExp(r'_+'), '_')
      .replaceAll(RegExp(r'^_|_$'), '');
  return normalized.isEmpty ? 'market_item' : normalized;
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
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('已退出 GitHub'),
          behavior: SnackBarBehavior.floating,
        ),
      );
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
            onLogin: () => _showMineMessage(context, 'GitHub 登录'),
            onLogout: _logout,
          ),
        const SizedBox(height: 16),
        _MineSectionTitle(text: '管理'),
        _MineActionCard(
          icon: Icons.settings_outlined,
          title: '管理 Artifact',
          subtitle: '查看已发布的 Artifact 项目。',
          onTap: () => _showMineMessage(context, '管理 Artifact'),
        ),
        const SizedBox(height: 8),
        _MineActionCard(
          icon: Icons.settings_outlined,
          title: '管理 Skill',
          subtitle: '查看已发布的技能。',
          onTap: () => _showMineMessage(context, '管理 Skill'),
        ),
        const SizedBox(height: 8),
        _MineActionCard(
          icon: Icons.settings_outlined,
          title: '管理 MCP',
          subtitle: '查看已发布的 MCP 服务。',
          onTap: () => _showMineMessage(context, '管理 MCP'),
        ),
        const SizedBox(height: 16),
        _MineSectionTitle(text: '发布'),
        _MineActionCard(
          icon: Icons.add,
          title: '发布 Artifact',
          subtitle: '发布工具包、工作流或运行时资源。',
          onTap: () => _showMineMessage(context, '发布 Artifact'),
        ),
        const SizedBox(height: 8),
        _MineActionCard(
          icon: Icons.add,
          title: '发布 Skill',
          subtitle: '分享一个技能仓库。',
          onTap: () => _showMineMessage(context, '发布 Skill'),
        ),
        const SizedBox(height: 8),
        _MineActionCard(
          icon: Icons.add,
          title: '发布 MCP',
          subtitle: '分享一个 MCP 服务配置。',
          onTap: () => _showMineMessage(context, '发布 MCP'),
        ),
      ],
    );
  }

  void _showMineMessage(BuildContext context, String label) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text('$label 页面待接入'),
        behavior: SnackBarBehavior.floating,
      ),
    );
  }
}

class _MineAccountLoadingCard extends StatelessWidget {
  const _MineAccountLoadingCard();

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Card(
      elevation: 0,
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.62),
      child: const ListTile(
        leading: SizedBox.square(
          dimension: 24,
          child: CircularProgressIndicator(strokeWidth: 2),
        ),
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
    return Card(
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
    return Card(
      elevation: 0,
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.62),
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
