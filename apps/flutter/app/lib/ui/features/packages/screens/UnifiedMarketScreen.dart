// ignore_for_file: file_names

import 'dart:convert';
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/LazyIndexedStack.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../main/TopBarController.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/EmptyState.dart';
import '../market/ArtifactMarketSupport.dart';
import '../market/MarketBrowseControls.dart';
import '../market/MarketBrowseList.dart';
import '../market/MarketStatsSupport.dart';
import 'ArtifactDetailScreen.dart';
import 'ArtifactProjectNodeTreeDialog.dart';
import 'MarketIssueDetailScreen.dart';
import 'ArtifactPublishScreen.dart';

enum MarketHomeTab { artifact, skill, mcp, mine }

const List<String> _artifactMarketTypes = <String>['script', 'package'];

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
  bool _searchExpanded = false;
  Timer? _searchDebounce;
  TopBarController? _topBarController;

  bool get _searchEnabled => _selectedTab != MarketHomeTab.mine;

  bool get _isSearchActive =>
      _searchEnabled && (_searchExpanded || _searchInput.trim().isNotEmpty);

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
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: Colors.transparent,
      child: Column(
        children: <Widget>[
          OperitGlassSurface(
            color: colorScheme.surface,
            layer: OperitGlassSurfaceLayer.panel,
            transparentAlpha: 0.035,
            clip: false,
            material: true,
            child: DefaultTabController(
              key: ValueKey<MarketHomeTab>(_selectedTab),
              length: MarketHomeTab.values.length,
              initialIndex: _selectedTab.index,
              child: TabBar(
                onTap: (index) {
                  setState(() {
                    _selectedTab = MarketHomeTab.values[index];
                    _searchInput = '';
                    _searchQuery = '';
                    _searchExpanded = false;
                    _searchDebounce?.cancel();
                  });
                  _syncTopBar();
                },
                tabs: const <Widget>[
                  Tab(text: 'Artifact'),
                  Tab(text: 'Skill'),
                  Tab(text: 'MCP'),
                  Tab(text: 'Mine'),
                ],
              ),
            ),
          ),
          MarketBrowseControls(
            sortOption: _sortOption,
            enabled: _searchEnabled,
            onSortChanged: (sortOption) {
              setState(() {
                _sortOption = sortOption;
              });
            },
          ),
          Expanded(
            child: LazyIndexedStack(
              index: _selectedTab.index,
              itemCount: MarketHomeTab.values.length,
              itemBuilder: (context, index) {
                return switch (MarketHomeTab.values[index]) {
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
                  MarketHomeTab.mine => _MarketMinePane(
                    clients: widget.clients,
                  ),
                };
              },
            ),
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
  final Map<String, int> _pages = <String, int>{};
  final Map<String, int> _totalPagesByType = <String, int>{};
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
      final pages = await Future.wait(
        _artifactMarketTypes.map(
          (type) =>
              _market.getArtifactRankPage(type: type, metric: _metric, page: 1),
        ),
      );
      if (!mounted) {
        return;
      }
      final nextPages = <String, int>{};
      final nextTotalPages = <String, int>{};
      final nextItems = <core_proxy.ArtifactProjectRankEntryResponse>[];
      for (var index = 0; index < pages.length; index += 1) {
        final type = _artifactMarketTypes[index];
        final page = pages[index];
        nextPages[type] = page.page;
        nextTotalPages[type] = page.totalPages < 1 ? 1 : page.totalPages;
        nextItems.addAll(page.items);
      }
      setState(() {
        _items = _sortArtifactItems(nextItems);
        _pages
          ..clear()
          ..addAll(nextPages);
        _totalPagesByType
          ..clear()
          ..addAll(nextTotalPages);
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
    if (_loadingMore || !_hasMore) {
      return;
    }
    setState(() {
      _loadingMore = true;
    });
    try {
      final requests = _artifactMarketTypes
          .where((type) {
            final currentPage = _pages[type] ?? 0;
            final totalPages = _totalPagesByType[type] ?? 1;
            return currentPage < totalPages;
          })
          .toList(growable: false);
      final pages = await Future.wait(
        requests.map(
          (type) => _market.getArtifactRankPage(
            type: type,
            metric: _metric,
            page: (_pages[type] ?? 0) + 1,
          ),
        ),
      );
      if (!mounted) {
        return;
      }
      final nextItems = <core_proxy.ArtifactProjectRankEntryResponse>[
        ..._items,
      ];
      for (var index = 0; index < pages.length; index += 1) {
        final type = requests[index];
        final page = pages[index];
        _pages[type] = page.page;
        _totalPagesByType[type] = page.totalPages < 1 ? 1 : page.totalPages;
        nextItems.addAll(page.items);
      }
      setState(() {
        _items = _sortArtifactItems(nextItems);
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

  bool get _hasMore => _artifactMarketTypes.any((type) {
    final currentPage = _pages[type] ?? 0;
    final totalPages = _totalPagesByType[type] ?? 1;
    return currentPage < totalPages;
  });

  List<core_proxy.ArtifactProjectRankEntryResponse> _sortArtifactItems(
    List<core_proxy.ArtifactProjectRankEntryResponse> items,
  ) {
    final sorted = items.toList(growable: false);
    sorted.sort((left, right) {
      if (widget.sortOption == MarketSortOption.updated) {
        return (right.latestPublishedAt ?? '').compareTo(
          left.latestPublishedAt ?? '',
        );
      }
      return right.downloads.compareTo(left.downloads);
    });
    return sorted;
  }

  @override
  Widget build(BuildContext context) {
    final error = _errorMessage;
    if (_loading && _items.isEmpty) {
      return const M3LoadingPane();
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
    return MarketBrowseList(
      isLoading: _loading,
      isLoadingMore: _loadingMore,
      hasMore: _hasMore && widget.searchQuery.trim().isEmpty,
      isEmpty: displayed.isEmpty,
      emptyTitle: widget.searchQuery.trim().isEmpty ? '暂无 Artifact' : '没有匹配结果',
      onRefresh: _loadFirstPage,
      onLoadMore: _loadMore,
      items: displayed,
      groupByUpdatedDate: widget.sortOption == MarketSortOption.updated,
      updatedAt: (item) => item.latestPublishedAt ?? '',
      itemBuilder: (item) => MarketGridCard(
        title: item.projectDisplayName,
        description: item.projectDescription,
        author: item.rootPublisherLogin,
        downloads: item.downloads,
        likes: item.likes,
        hearts: 0,
        actionLabel: '下载',
        actionIcon: Icons.download_outlined,
        actionBusy: _busyProjectIds.contains(item.projectId),
        onAction: () => _downloadArtifact(item),
        onTap: () => _openArtifactProject(item.projectId),
      ),
    );
  }

  Future<void> _downloadArtifact(
    core_proxy.ArtifactProjectRankEntryResponse item,
  ) async {
    setState(() {
      _busyProjectIds.add(item.projectId);
    });
    try {
      final project = await _loadArtifactProject(item.projectId);
      final node = _selectArtifactInstallNode(
        project,
        requestedNodeId: item.defaultNode?.nodeId,
      );
      final result = await _installArtifactNode(project, node);
      if (!mounted) {
        return;
      }
      if (result == null) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(result), behavior: SnackBarBehavior.floating),
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to install artifact: $error\n$stackTrace');
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

  Future<void> _openArtifactProject(String projectId) async {
    setState(() {
      _busyProjectIds.add(projectId);
    });
    try {
      final project = await _loadArtifactProject(projectId);
      if (!mounted) {
        return;
      }
      if (project.nodes.length == 1) {
        _showArtifactNodeDetails(project, project.nodes.single);
      } else {
        _showArtifactNodeTree(project);
      }
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

  Future<core_proxy.ArtifactProjectDetailResponse> _loadArtifactProject(
    String projectId,
  ) {
    return _market.getArtifactProject(projectId: projectId);
  }

  void _showArtifactNodeTree(core_proxy.ArtifactProjectDetailResponse project) {
    showDialog<void>(
      context: context,
      builder: (context) => ArtifactProjectNodeTreeDialog(
        project: project,
        onSelectNode: (node) {
          Navigator.of(context).pop();
          _showArtifactNodeDetails(project, node);
        },
      ),
    );
  }

  void _showArtifactNodeDetails(
    core_proxy.ArtifactProjectDetailResponse project,
    core_proxy.ArtifactProjectNodeResponse node,
  ) {
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) => ArtifactNodeDetailsScreen(
          clients: widget.clients,
          project: project,
          node: node,
        ),
      ),
    );
  }

  core_proxy.ArtifactProjectNodeResponse _selectArtifactInstallNode(
    core_proxy.ArtifactProjectDetailResponse project, {
    String? requestedNodeId,
  }) {
    final nodeIds = <String>[
      if (requestedNodeId != null && requestedNodeId.trim().isNotEmpty)
        requestedNodeId.trim(),
      if (project.defaultNodeId.trim().isNotEmpty) project.defaultNodeId.trim(),
      if (project.latestOpenNodeId.trim().isNotEmpty)
        project.latestOpenNodeId.trim(),
      if (project.latestNodeId.trim().isNotEmpty) project.latestNodeId.trim(),
    ];
    for (final nodeId in nodeIds) {
      for (final node in project.nodes) {
        if (node.nodeId == nodeId) {
          return node;
        }
      }
    }
    throw StateError('Default artifact node not found');
  }

  Future<String?> _installArtifactNode(
    core_proxy.ArtifactProjectDetailResponse project,
    core_proxy.ArtifactProjectNodeResponse node,
  ) async {
    final confirmed = await confirmArtifactNodeCompatibility(
      context: context,
      project: project,
      node: node,
    );
    if (!confirmed) {
      return null;
    }
    return runCoreMarketInstall(
      clients: widget.clients,
      type: node.type,
      projectId: project.projectId,
      nodeId: node.nodeId,
    );
  }
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
  List<_ManagedArtifactIssue> _issues = <_ManagedArtifactIssue>[];

  @override
  void initState() {
    super.initState();
    _loadIssues();
  }

  Future<void> _loadIssues() async {
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    try {
      final auth = widget.clients.preferencesGitHubAuthPreferences;
      final user = await auth.getCurrentUserInfo();
      if (user == null) {
        throw StateError('Unable to read GitHub user info');
      }
      final items = await _loadManagedArtifactIssues(
        clients: widget.clients,
        creatorLogin: user.login,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _issues = items;
        _loading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load managed artifacts: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
        _loading = false;
      });
    }
  }

  Future<void> _setIssueState(_ManagedArtifactIssue item, String state) async {
    try {
      final updated = await _updateArtifactIssueState(
        clients: widget.clients,
        item: item,
        state: state,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _issues = _issues
            .map(
              (existing) => existing.issue.id == item.issue.id
                  ? existing.copyWith(issue: updated)
                  : existing,
            )
            .toList(growable: false);
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(state == 'closed' ? '移除请求已提交' : '重新上架请求已提交'),
          behavior: SnackBarBehavior.floating,
        ),
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to update artifact state: $error\n$stackTrace');
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

  Future<void> _editIssue(_ManagedArtifactIssue item) async {
    final updated = await showDialog<core_proxy.GitHubIssue>(
      context: context,
      builder: (context) =>
          _ArtifactIssueEditDialog(clients: widget.clients, item: item),
    );
    if (updated == null || !mounted) {
      return;
    }
    setState(() {
      _issues = _issues
          .map(
            (existing) => existing.issue.id == item.issue.id
                ? _ManagedArtifactIssue.fromIssue(item.type, updated)
                : existing,
          )
          .toList(growable: false);
    });
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('作品信息已更新'),
        behavior: SnackBarBehavior.floating,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final error = _errorMessage;
    return Scaffold(
      backgroundColor: Colors.transparent,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        title: const Text('管理 Artifact'),
        actions: <Widget>[
          IconButton(
            onPressed: _loading ? null : _loadIssues,
            icon: const Icon(Icons.refresh),
            tooltip: '刷新',
          ),
        ],
      ),
      body: Builder(
        builder: (context) {
          if (_loading && _issues.isEmpty) {
            return const M3LoadingPane();
          }
          if (error != null && _issues.isEmpty) {
            return EmptyState(
              icon: Icons.error_outline,
              title: '加载失败',
              message: error,
              action: TextButton.icon(
                onPressed: _loadIssues,
                icon: const Icon(Icons.refresh),
                label: const Text('刷新'),
              ),
            );
          }
          if (_issues.isEmpty) {
            return EmptyState(
              icon: Icons.store_outlined,
              title: '还没有发布 Artifact',
              message: '从发布入口提交第一个 Artifact 后会显示在这里。',
              scrollable: false,
            );
          }
          return RefreshIndicator(
            onRefresh: _loadIssues,
            child: ListView.separated(
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 120),
              itemCount: _issues.length,
              separatorBuilder: (context, index) => const SizedBox(height: 10),
              itemBuilder: (context, index) {
                final item = _issues[index];
                return _ManagedArtifactCard(
                  item: item,
                  onEdit: () => _editIssue(item),
                  onClose: () => _setIssueState(item, 'closed'),
                  onOpen: () => _setIssueState(item, 'open'),
                );
              },
            ),
          );
        },
      ),
    );
  }
}

class _ManagedArtifactCard extends StatelessWidget {
  const _ManagedArtifactCard({
    required this.item,
    required this.onEdit,
    required this.onClose,
    required this.onOpen,
  });

  final _ManagedArtifactIssue item;
  final VoidCallback onEdit;
  final VoidCallback onClose;
  final VoidCallback onOpen;

  @override
  Widget build(BuildContext context) {
    final issue = item.issue;
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final title = item.displayName;
    return OperitGlassSurface(
      color: colorScheme.surfaceContainerLow,
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(16),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      const SizedBox(height: 6),
                      Text(
                        item.description,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                _SmallChip(text: issue.state == 'open' ? '已上架' : '已下架'),
              ],
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 6,
              runSpacing: 6,
              children: <Widget>[
                _SmallChip(text: artifactTypeLabel(item.type)),
                _SmallChip(text: '#${issue.number}'),
                _SmallChip(text: item.version),
                _SmallChip(text: formatMarketDate(issue.updatedAt)),
                if (item.runtimePackageId.isNotEmpty)
                  _SmallChip(text: item.runtimePackageId),
              ],
            ),
            const SizedBox(height: 12),
            Row(
              children: <Widget>[
                TextButton.icon(
                  onPressed: onEdit,
                  icon: const Icon(Icons.edit_outlined, size: 18),
                  label: const Text('编辑'),
                ),
                const Spacer(),
                if (issue.state == 'open')
                  TextButton.icon(
                    onPressed: onClose,
                    icon: const Icon(Icons.delete_outline, size: 18),
                    label: const Text('移除'),
                  )
                else
                  FilledButton.tonalIcon(
                    onPressed: onOpen,
                    icon: const Icon(Icons.refresh, size: 18),
                    label: const Text('重新上架'),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _ArtifactIssueEditDialog extends StatefulWidget {
  const _ArtifactIssueEditDialog({required this.clients, required this.item});

  final GeneratedCoreProxyClients clients;
  final _ManagedArtifactIssue item;

  @override
  State<_ArtifactIssueEditDialog> createState() =>
      _ArtifactIssueEditDialogState();
}

class _ArtifactIssueEditDialogState extends State<_ArtifactIssueEditDialog> {
  late final TextEditingController _displayNameController =
      TextEditingController(text: widget.item.displayName);
  late final TextEditingController _descriptionController =
      TextEditingController(text: widget.item.description);
  late final TextEditingController _minVersionController =
      TextEditingController(text: widget.item.minSupportedAppVersion);
  late final TextEditingController _maxVersionController =
      TextEditingController(text: widget.item.maxSupportedAppVersion);
  bool _saving = false;

  @override
  void dispose() {
    _displayNameController.dispose();
    _descriptionController.dispose();
    _minVersionController.dispose();
    _maxVersionController.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (_saving) {
      return;
    }
    setState(() {
      _saving = true;
    });
    try {
      final updated = await _updateArtifactIssueContent(
        clients: widget.clients,
        item: widget.item,
        displayName: _displayNameController.text,
        description: _descriptionController.text,
        minSupportedAppVersion: _minVersionController.text,
        maxSupportedAppVersion: _maxVersionController.text,
      );
      if (!mounted) {
        return;
      }
      Navigator.of(context).pop(updated);
    } catch (error, stackTrace) {
      debugPrint('Failed to edit artifact issue: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _saving = false;
      });
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
    return AlertDialog(
      icon: const Icon(Icons.edit_outlined),
      title: const Text('编辑作品信息'),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              TextField(
                controller: _displayNameController,
                enabled: !_saving,
                decoration: const InputDecoration(
                  labelText: '显示名称',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _descriptionController,
                enabled: !_saving,
                minLines: 3,
                maxLines: 6,
                decoration: const InputDecoration(
                  labelText: '简介',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _minVersionController,
                enabled: !_saving,
                decoration: const InputDecoration(
                  labelText: '最低支持版本',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _maxVersionController,
                enabled: !_saving,
                decoration: const InputDecoration(
                  labelText: '最高支持版本',
                  border: OutlineInputBorder(),
                ),
              ),
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _saving ? null : () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton.icon(
          onPressed: _saving ? null : _save,
          icon: _saving
              ? M3LoadingIndicator(
                  size: 18,
                  color: Theme.of(context).colorScheme.onPrimary,
                )
              : const Icon(Icons.save_outlined),
          label: const Text('保存'),
        ),
      ],
    );
  }
}

class _ManagedArtifactIssue {
  const _ManagedArtifactIssue({
    required this.type,
    required this.issue,
    required this.metadata,
  });

  factory _ManagedArtifactIssue.fromIssue(
    String type,
    core_proxy.GitHubIssue issue,
  ) {
    return _ManagedArtifactIssue(
      type: type,
      issue: issue,
      metadata: _artifactIssueMetadata(issue),
    );
  }

  final String type;
  final core_proxy.GitHubIssue issue;
  final Map<String, Object?> metadata;

  String get displayName =>
      _artifactMetadataString(metadata, 'displayName').isEmpty
      ? issue.title
      : _artifactMetadataString(metadata, 'displayName');

  String get description =>
      _artifactMetadataString(metadata, 'description').isEmpty
      ? (issue.body ?? '')
      : _artifactMetadataString(metadata, 'description');

  String get version => _artifactMetadataString(metadata, 'version').isEmpty
      ? '-'
      : _artifactMetadataString(metadata, 'version');

  String get runtimePackageId =>
      _artifactMetadataString(metadata, 'runtimePackageId');

  String get minSupportedAppVersion =>
      _artifactMetadataString(metadata, 'minSupportedAppVersion');

  String get maxSupportedAppVersion =>
      _artifactMetadataString(metadata, 'maxSupportedAppVersion');

  _ManagedArtifactIssue copyWith({core_proxy.GitHubIssue? issue}) {
    return _ManagedArtifactIssue.fromIssue(type, issue ?? this.issue);
  }
}

Future<List<_ManagedArtifactIssue>> _loadManagedArtifactIssues({
  required GeneratedCoreProxyClients clients,
  required String creatorLogin,
}) async {
  final items = <_ManagedArtifactIssue>[];
  for (final definition in _artifactIssueRepositories()) {
    final issues = await _githubSearchIssues(
      clients: clients,
      query:
          'repo:${definition.owner}/${definition.repo} is:issue author:$creatorLogin label:"${definition.label}"',
    );
    items.addAll(
      issues.map(
        (issue) => _ManagedArtifactIssue.fromIssue(definition.type, issue),
      ),
    );
  }
  items.sort(
    (left, right) => right.issue.updatedAt.compareTo(left.issue.updatedAt),
  );
  return items;
}

Future<core_proxy.GitHubIssue> _updateArtifactIssueState({
  required GeneratedCoreProxyClients clients,
  required _ManagedArtifactIssue item,
  required String state,
}) async {
  final repo = artifactIssueRepository(item.type);
  final value = await _githubJsonRequest(
    clients: clients,
    method: 'PATCH',
    uri: Uri.https(
      'api.github.com',
      '/repos/${repo.owner}/${repo.repo}/issues/${item.issue.number}',
    ),
    body: <String, Object?>{'state': state},
  );
  return core_proxy.GitHubIssue.fromJson(value as Map<String, Object?>);
}

Future<core_proxy.GitHubIssue> _updateArtifactIssueContent({
  required GeneratedCoreProxyClients clients,
  required _ManagedArtifactIssue item,
  required String displayName,
  required String description,
  required String minSupportedAppVersion,
  required String maxSupportedAppVersion,
}) async {
  final trimmedDisplayName = displayName.trim();
  final trimmedDescription = description.trim();
  if (trimmedDisplayName.isEmpty) {
    throw StateError('插件名称不能为空');
  }
  if (trimmedDescription.isEmpty) {
    throw StateError('简介不能为空');
  }
  await _ensureArtifactDisplayNameAvailable(
    clients: clients,
    displayName: trimmedDisplayName,
    currentIssueId: item.issue.id,
  );
  final repo = artifactIssueRepository(item.type);
  final body = _buildUpdatedArtifactIssueBody(
    item: item,
    displayName: trimmedDisplayName,
    description: trimmedDescription,
    minSupportedAppVersion: minSupportedAppVersion.trim(),
    maxSupportedAppVersion: maxSupportedAppVersion.trim(),
  );
  final value = await _githubJsonRequest(
    clients: clients,
    method: 'PATCH',
    uri: Uri.https(
      'api.github.com',
      '/repos/${repo.owner}/${repo.repo}/issues/${item.issue.number}',
    ),
    body: <String, Object?>{'title': trimmedDisplayName, 'body': body},
  );
  return core_proxy.GitHubIssue.fromJson(value as Map<String, Object?>);
}

Future<void> _ensureArtifactDisplayNameAvailable({
  required GeneratedCoreProxyClients clients,
  required String displayName,
  required int currentIssueId,
}) async {
  final normalizedTitle = displayName
      .trim()
      .replaceAll(RegExp(r'\s+'), ' ')
      .toLowerCase();
  for (final definition in _artifactIssueRepositories()) {
    final issues = await _githubSearchIssues(
      clients: clients,
      query:
          'repo:${definition.owner}/${definition.repo} is:issue is:open in:title "$displayName"',
    );
    final conflict = issues.any(
      (issue) =>
          issue.id != currentIssueId &&
          issue.title.trim().replaceAll(RegExp(r'\s+'), ' ').toLowerCase() ==
              normalizedTitle,
    );
    if (conflict) {
      throw StateError('市场里已经有同名已发布插件「$displayName」，请换一个名称。');
    }
  }
}

Future<List<core_proxy.GitHubIssue>> _githubSearchIssues({
  required GeneratedCoreProxyClients clients,
  required String query,
}) async {
  final issues = <core_proxy.GitHubIssue>[];
  var page = 1;
  while (true) {
    final value = await _githubJsonRequest(
      clients: clients,
      method: 'GET',
      uri: Uri.https('api.github.com', '/search/issues', <String, String>{
        'q': query,
        'sort': 'updated',
        'order': 'desc',
        'page': page.toString(),
        'per_page': '100',
      }),
    );
    final items = (value as Map<String, Object?>)['items'] as List<Object?>;
    issues.addAll(
      items.map(
        (item) => core_proxy.GitHubIssue.fromJson(item as Map<String, Object?>),
      ),
    );
    if (items.length < 100) {
      break;
    }
    page += 1;
  }
  return issues;
}

Future<Object?> _githubJsonRequest({
  required GeneratedCoreProxyClients clients,
  required String method,
  required Uri uri,
  Object? body,
}) async {
  final token = await clients.preferencesGitHubAuthPreferences
      .getCurrentAccessToken();
  if (token == null || token.trim().isEmpty) {
    throw StateError('GitHub login required');
  }
  final headers = <String, String>{
    'Accept':
        'application/vnd.github+json, application/vnd.github.squirrel-girl-preview+json',
    'Authorization': 'Bearer ${token.trim()}',
    'X-GitHub-Api-Version': '2022-11-28',
  };
  final response = switch (method) {
    'GET' => await http.get(uri, headers: headers),
    'PATCH' => await http.patch(
      uri,
      headers: <String, String>{...headers, 'Content-Type': 'application/json'},
      body: jsonEncode(body),
    ),
    final value => throw StateError('Unsupported GitHub method: $value'),
  };
  if (response.statusCode < 200 || response.statusCode >= 300) {
    throw StateError(
      'HTTP ${response.statusCode}: ${_summarizeHttpBody(response.body)}',
    );
  }
  if (response.body.trim().isEmpty) {
    return null;
  }
  return jsonDecode(response.body);
}

String _buildUpdatedArtifactIssueBody({
  required _ManagedArtifactIssue item,
  required String displayName,
  required String description,
  required String minSupportedAppVersion,
  required String maxSupportedAppVersion,
}) {
  final metadata = Map<String, Object?>.from(item.metadata);
  final nodeId = _requiredArtifactMetadata(metadata, 'nodeId');
  final rootNodeId = _requiredArtifactMetadata(metadata, 'rootNodeId');
  final isRootNode = rootNodeId == nodeId;
  metadata['displayName'] = displayName;
  metadata['description'] = description;
  metadata['minSupportedAppVersion'] = minSupportedAppVersion.isEmpty
      ? null
      : minSupportedAppVersion;
  metadata['maxSupportedAppVersion'] = maxSupportedAppVersion.isEmpty
      ? null
      : maxSupportedAppVersion;
  if (isRootNode) {
    metadata['projectDisplayName'] = displayName;
    metadata['projectDescription'] = description;
  }
  final parentNodeIds = _artifactMetadataStringList(metadata, 'parentNodeIds');
  final timestamp = DateTime.now()
      .toIso8601String()
      .replaceFirst('T', ' ')
      .substring(0, 19);
  final supportedVersions = _formatSupportedAppVersions(
    metadata['minSupportedAppVersion']?.toString(),
    metadata['maxSupportedAppVersion']?.toString(),
  );
  final parentNodeText = parentNodeIds.isEmpty ? '-' : parentNodeIds.join(', ');
  return '''
<!-- operit-market-json: ${jsonEncode(metadata)} -->
<!-- operit-parser-version: forge-v3 -->

## ${artifactTypeLabel(item.type)}

$description

## Project Cluster

- Project ID: `${_requiredArtifactMetadata(metadata, 'projectId')}`
- Runtime package ID: `${_requiredArtifactMetadata(metadata, 'runtimePackageId')}`
- Node ID: `$nodeId`
- Root node ID: `$rootNodeId`
- Parent node IDs: `$parentNodeText`
- Project display name: `${_requiredArtifactMetadata(metadata, 'projectDisplayName')}`
- Project description: ${_requiredArtifactMetadata(metadata, 'projectDescription')}

## Artifact

- Publisher: `${_requiredArtifactMetadata(metadata, 'publisherLogin')}`
- Forge repo: `${_requiredArtifactMetadata(metadata, 'forgeRepo')}`
- Release tag: `${_requiredArtifactMetadata(metadata, 'releaseTag')}`
- Asset: `${_requiredArtifactMetadata(metadata, 'assetName')}`
- SHA-256: `${_requiredArtifactMetadata(metadata, 'sha256')}`
- Download: ${_requiredArtifactMetadata(metadata, 'downloadUrl')}

## Metadata

| Field | Value |
| --- | --- |
| Type | ${_requiredArtifactMetadata(metadata, 'type')} |
| Project ID | ${_requiredArtifactMetadata(metadata, 'projectId')} |
| Runtime package ID | ${_requiredArtifactMetadata(metadata, 'runtimePackageId')} |
| Node ID | $nodeId |
| Root node ID | $rootNodeId |
| Parent node IDs | $parentNodeText |
| Version | ${_requiredArtifactMetadata(metadata, 'version')} |
| Supported app versions | $supportedVersions |
| Source file | ${_requiredArtifactMetadata(metadata, 'sourceFileName')} |
| Updated at | $timestamp |
''';
}

Map<String, Object?> _artifactIssueMetadata(core_proxy.GitHubIssue issue) {
  final body = issue.body ?? '';
  const prefix = '<!-- operit-market-json: ';
  final start = body.indexOf(prefix);
  if (start < 0) {
    throw StateError('Invalid artifact metadata');
  }
  final jsonStart = start + prefix.length;
  final end = body.indexOf(' -->', jsonStart);
  if (end <= jsonStart) {
    throw StateError('Invalid artifact metadata');
  }
  final decoded = jsonDecode(body.substring(jsonStart, end));
  if (decoded is! Map) {
    throw StateError('Invalid artifact metadata');
  }
  return decoded.map((key, value) => MapEntry(key.toString(), value));
}

String _artifactMetadataString(Map<String, Object?> metadata, String key) {
  return metadata[key]?.toString().trim() ?? '';
}

String _requiredArtifactMetadata(Map<String, Object?> metadata, String key) {
  final value = _artifactMetadataString(metadata, key);
  if (value.isEmpty) {
    throw StateError('Invalid artifact metadata: $key');
  }
  return value;
}

List<String> _artifactMetadataStringList(
  Map<String, Object?> metadata,
  String key,
) {
  final value = metadata[key];
  if (value is List<Object?>) {
    return value
        .map((item) => item.toString().trim())
        .where((item) => item.isNotEmpty)
        .toList(growable: false);
  }
  return const <String>[];
}

String _formatSupportedAppVersions(String? minVersion, String? maxVersion) {
  final minValue = minVersion?.trim() ?? '';
  final maxValue = maxVersion?.trim() ?? '';
  if (minValue.isNotEmpty && maxValue.isNotEmpty) {
    return '$minValue - $maxValue';
  }
  if (minValue.isNotEmpty) {
    return '>= $minValue';
  }
  if (maxValue.isNotEmpty) {
    return '<= $maxValue';
  }
  return '未声明';
}

String _summarizeHttpBody(String body) {
  final trimmed = body.trim();
  if (trimmed.isEmpty) {
    return '';
  }
  if (trimmed.contains('<html') || trimmed.contains('<!DOCTYPE html')) {
    return '[html body omitted]';
  }
  return trimmed.split('\n').first.trim();
}

List<ArtifactIssueRepository> _artifactIssueRepositories() {
  return const <ArtifactIssueRepository>[
    ArtifactIssueRepository(
      type: 'script',
      owner: 'AAswordman',
      repo: 'OperitScriptMarket',
      label: 'script-artifact',
    ),
    ArtifactIssueRepository(
      type: 'package',
      owner: 'AAswordman',
      repo: 'OperitPackageMarket',
      label: 'package-artifact',
    ),
  ];
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
      return const M3LoadingPane();
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
    return MarketBrowseList(
      isLoading: _loading,
      isLoadingMore: _loadingMore,
      hasMore: _page < _totalPages && widget.searchQuery.trim().isEmpty,
      isEmpty: displayed.isEmpty,
      emptyTitle: widget.searchQuery.trim().isEmpty ? '暂无项目' : '没有匹配结果',
      onRefresh: _loadFirstPage,
      onLoadMore: _loadMore,
      items: displayed,
      groupByUpdatedDate: widget.sortOption == MarketSortOption.updated,
      updatedAt: (item) => item.updatedAt ?? '',
      itemBuilder: (item) => MarketGridCard(
        title: item.displayTitle,
        description: item.summaryDescription,
        author: item.authorLogin,
        downloads: item.downloads,
        likes: item.issue.reactions?.thumbsUp ?? 0,
        hearts: item.issue.reactions?.heart ?? 0,
        actionLabel: widget.type == 'skill' ? '安装' : '安装',
        actionIcon: Icons.download_outlined,
        actionBusy: _busyIssueIds.contains(item.id),
        onAction: () => _installIssueItem(item),
        onTap: () => _showDetails(item),
      ),
    );
  }

  void _showDetails(core_proxy.MarketRankIssueEntryResponse item) {
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) => MarketIssueDetailScreen(
          clients: widget.clients,
          type: widget.type,
          item: item,
        ),
      ),
    );
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
            onLogin: () => _showGitHubTokenDialog(context),
            onLogout: _logout,
          ),
        const SizedBox(height: 16),
        _MineSectionTitle(text: '管理'),
        _MineActionCard(
          icon: Icons.settings_outlined,
          title: '管理 Artifact',
          subtitle: '查看已发布的 Artifact 项目。',
          onTap: () => _openArtifactManage(context),
        ),
        const SizedBox(height: 8),
        _MineActionCard(
          icon: Icons.settings_outlined,
          title: '管理 Skill',
          subtitle: '查看已发布的技能。',
          onTap: () => _handleMineAction(context, '管理 Skill'),
        ),
        const SizedBox(height: 8),
        _MineActionCard(
          icon: Icons.settings_outlined,
          title: '管理 MCP',
          subtitle: '查看已发布的 MCP 服务。',
          onTap: () => _handleMineAction(context, '管理 MCP'),
        ),
        const SizedBox(height: 16),
        _MineSectionTitle(text: '发布'),
        _MineActionCard(
          icon: Icons.add,
          title: '发布 Artifact',
          subtitle: '发布工具包、工作流或运行时资源。',
          onTap: () => _openArtifactPublish(context),
        ),
        const SizedBox(height: 8),
        _MineActionCard(
          icon: Icons.add,
          title: '发布 Skill',
          subtitle: '分享一个技能仓库。',
          onTap: () => _handleMineAction(context, '发布 Skill'),
        ),
        const SizedBox(height: 8),
        _MineActionCard(
          icon: Icons.add,
          title: '发布 MCP',
          subtitle: '分享一个 MCP 服务配置。',
          onTap: () => _handleMineAction(context, '发布 MCP'),
        ),
      ],
    );
  }

  void _handleMineAction(BuildContext context, String label) {
    if (!_loggedIn) {
      _showGitHubTokenDialog(context);
      return;
    }
    _showMineMessage(context, label);
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

  void _showMineMessage(BuildContext context, String label) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text('$label 页面尚未迁移到 Flutter'),
        behavior: SnackBarBehavior.floating,
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
              final apiUser = await widget.clients.apiMarketStatsApiService
                  .getCurrentUser();
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
              ScaffoldMessenger.of(parentContext).showSnackBar(
                const SnackBar(
                  content: Text('GitHub 登录完成'),
                  behavior: SnackBarBehavior.floating,
                ),
              );
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
