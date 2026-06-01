// ignore_for_file: file_names

import 'dart:convert';
import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../components/EmptyState.dart';
import 'ArtifactPublishScreen.dart';

enum MarketHomeTab { artifact, skill, mcp, mine }

enum MarketSortOption { downloads, updated }

const List<String> _artifactMarketTypes = <String>['script', 'package'];
const String _currentAppVersion = '1.0.0+1';

class UnifiedMarketScreen extends StatefulWidget {
  const UnifiedMarketScreen({
    super.key,
    this.initialTab = MarketHomeTab.artifact,
    this.showBackButton = false,
    GeneratedCoreProxyClients? clients,
  }) : clients =
           clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final MarketHomeTab initialTab;
  final bool showBackButton;
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
          if (widget.showBackButton)
            _MarketRouteHeader(
              title: _marketTabTitle(_selectedTab),
              onBack: () => Navigator.of(context).maybePop(),
            ),
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
      hasMore: _hasMore && widget.searchQuery.trim().isEmpty,
      isEmpty: displayed.isEmpty,
      emptyTitle: widget.searchQuery.trim().isEmpty ? '暂无 Artifact' : '没有匹配结果',
      onRefresh: _loadFirstPage,
      onLoadMore: _loadMore,
      items: displayed,
      groupByUpdatedDate: widget.sortOption == MarketSortOption.updated,
      updatedAt: (item) => item.latestPublishedAt ?? '',
      itemBuilder: (item) => _MarketGridCard(
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
      builder: (context) => _ArtifactProjectNodeTreeDialog(
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
    showDialog<void>(
      context: context,
      builder: (context) => _ArtifactNodeDetailsDialog(
        clients: widget.clients,
        project: project,
        node: node,
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
    final confirmed = await _confirmArtifactNodeCompatibility(
      context: context,
      project: project,
      node: node,
    );
    if (!confirmed) {
      return null;
    }
    return _runCoreMarketInstall(
      clients: widget.clients,
      type: node.type,
      projectId: project.projectId,
      nodeId: node.nodeId,
    );
  }
}

class _ArtifactProjectNodeTreeDialog extends StatelessWidget {
  const _ArtifactProjectNodeTreeDialog({
    required this.project,
    required this.onSelectNode,
  });

  final core_proxy.ArtifactProjectDetailResponse project;
  final ValueChanged<core_proxy.ArtifactProjectNodeResponse> onSelectNode;

  @override
  Widget build(BuildContext context) {
    final rows = _artifactTreeRows(project);
    final viewport = MediaQuery.sizeOf(context);
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Dialog(
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxWidth: 760,
          maxHeight: viewport.height * 0.88,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 18, 8, 10),
              child: Row(
                children: <Widget>[
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          project.projectDisplayName.trim().isEmpty
                              ? project.projectId
                              : project.projectDisplayName,
                          style: textTheme.titleLarge?.copyWith(
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                        const SizedBox(height: 6),
                        Wrap(
                          spacing: 6,
                          runSpacing: 6,
                          children: <Widget>[
                            _SmallChip(text: _artifactTypeLabel(project.type)),
                            _SmallChip(text: '${project.nodes.length} 节点'),
                            _SmallChip(text: '${project.downloads} 下载'),
                            if (project.likes > 0)
                              _SmallChip(text: '${project.likes} 喜欢'),
                          ],
                        ),
                      ],
                    ),
                  ),
                  IconButton(
                    onPressed: () => Navigator.of(context).pop(),
                    icon: const Icon(Icons.close),
                    tooltip: '关闭',
                  ),
                ],
              ),
            ),
            if (project.projectDescription.trim().isNotEmpty)
              Padding(
                padding: const EdgeInsets.fromLTRB(20, 0, 20, 12),
                child: Text(
                  project.projectDescription,
                  maxLines: 4,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodyMedium?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
            const Divider(height: 1),
            Flexible(
              child: ListView.separated(
                shrinkWrap: true,
                padding: const EdgeInsets.fromLTRB(8, 8, 8, 12),
                itemCount: rows.length,
                separatorBuilder: (context, index) => const SizedBox(height: 4),
                itemBuilder: (context, index) {
                  final row = rows[index];
                  final node = row.node;
                  final isDefault = node.nodeId == project.defaultNodeId;
                  final isLatestOpen = node.nodeId == project.latestOpenNodeId;
                  return Padding(
                    padding: EdgeInsets.only(left: 18.0 * row.depth),
                    child: ListTile(
                      dense: true,
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(8),
                      ),
                      tileColor: isDefault
                          ? colorScheme.primaryContainer.withValues(alpha: 0.36)
                          : colorScheme.surfaceContainerLow,
                      leading: Icon(
                        isDefault
                            ? Icons.account_tree
                            : Icons.radio_button_unchecked,
                        color: isDefault
                            ? colorScheme.primary
                            : colorScheme.onSurfaceVariant,
                      ),
                      title: Text(
                        _artifactNodeTitle(node),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: textTheme.titleSmall?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      subtitle: Wrap(
                        spacing: 6,
                        runSpacing: 4,
                        children: <Widget>[
                          _SmallChip(text: node.version),
                          _SmallChip(text: node.publisherLogin),
                          _SmallChip(text: node.state),
                          if (isDefault) const _SmallChip(text: '默认'),
                          if (isLatestOpen) const _SmallChip(text: '最新开放'),
                          _SmallChip(text: node.nodeId),
                        ],
                      ),
                      trailing: const Icon(Icons.chevron_right),
                      onTap: () => onSelectNode(node),
                    ),
                  );
                },
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ArtifactTreeRow {
  const _ArtifactTreeRow({required this.node, required this.depth});

  final core_proxy.ArtifactProjectNodeResponse node;
  final int depth;
}

List<_ArtifactTreeRow> _artifactTreeRows(
  core_proxy.ArtifactProjectDetailResponse project,
) {
  final nodesById = <String, core_proxy.ArtifactProjectNodeResponse>{
    for (final node in project.nodes) node.nodeId: node,
  };
  final childrenByParent =
      <String, List<core_proxy.ArtifactProjectNodeResponse>>{
        for (final node in project.nodes)
          node.nodeId: <core_proxy.ArtifactProjectNodeResponse>[],
      };
  for (final edge in project.edges) {
    final child = nodesById[edge.childNodeId];
    if (child != null && childrenByParent.containsKey(edge.parentNodeId)) {
      childrenByParent[edge.parentNodeId]!.add(child);
    }
  }
  for (final node in project.nodes) {
    for (final parentId in node.parentNodeIds) {
      final children = childrenByParent[parentId];
      if (children != null &&
          !children.any((child) => child.nodeId == node.nodeId)) {
        children.add(node);
      }
    }
  }
  for (final children in childrenByParent.values) {
    children.sort((left, right) {
      final dateOrder = (left.publishedAt ?? '').compareTo(
        right.publishedAt ?? '',
      );
      return dateOrder == 0 ? left.nodeId.compareTo(right.nodeId) : dateOrder;
    });
  }

  final rows = <_ArtifactTreeRow>[];
  final visited = <String>{};

  void appendNode(core_proxy.ArtifactProjectNodeResponse node, int depth) {
    if (!visited.add(node.nodeId)) {
      return;
    }
    rows.add(_ArtifactTreeRow(node: node, depth: depth));
    for (final child
        in childrenByParent[node.nodeId] ??
            const <core_proxy.ArtifactProjectNodeResponse>[]) {
      appendNode(child, depth + 1);
    }
  }

  final root = nodesById[project.rootNodeId];
  if (root != null) {
    appendNode(root, 0);
  }
  for (final node in project.nodes) {
    if (node.parentNodeIds.isEmpty) {
      appendNode(node, 0);
    }
  }
  for (final node in project.nodes) {
    appendNode(node, 0);
  }
  return rows;
}

String _artifactNodeTitle(core_proxy.ArtifactProjectNodeResponse node) {
  return node.displayName.trim().isEmpty ? node.nodeId : node.displayName;
}

String _firstNonBlank(Iterable<String> values) {
  for (final value in values) {
    final trimmed = value.trim();
    if (trimmed.isNotEmpty) {
      return trimmed;
    }
  }
  return '';
}

String _artifactTypeLabel(String type) {
  return switch (type.trim()) {
    'package' => 'Package',
    'script' => 'Script',
    final value when value.isNotEmpty => value,
    _ => 'Artifact',
  };
}

class _ArtifactNodeDetailsDialog extends StatefulWidget {
  const _ArtifactNodeDetailsDialog({
    required this.clients,
    required this.project,
    required this.node,
  });

  final GeneratedCoreProxyClients clients;
  final core_proxy.ArtifactProjectDetailResponse project;
  final core_proxy.ArtifactProjectNodeResponse node;

  @override
  State<_ArtifactNodeDetailsDialog> createState() =>
      _ArtifactNodeDetailsDialogState();
}

class _ArtifactNodeDetailsDialogState
    extends State<_ArtifactNodeDetailsDialog> {
  final TextEditingController _commentController = TextEditingController();
  bool _communityLoading = true;
  bool _postingComment = false;
  bool _reacting = false;
  bool _installing = false;
  String? _communityError;
  List<core_proxy.GitHubComment> _comments = <core_proxy.GitHubComment>[];
  List<core_proxy.GitHubReaction> _reactions = <core_proxy.GitHubReaction>[];
  core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser? _currentUser;

  GeneratedApiMarketStatsApiServiceCoreProxy get _market =>
      widget.clients.apiMarketStatsApiService;

  @override
  void initState() {
    super.initState();
    _loadCommunity();
  }

  @override
  void dispose() {
    _commentController.dispose();
    super.dispose();
  }

  Future<void> _loadCommunity() async {
    setState(() {
      _communityLoading = true;
      _communityError = null;
    });
    try {
      final repo = _artifactIssueRepository(widget.node.type);
      final auth = widget.clients.preferencesGitHubAuthPreferences;
      final loggedIn = await auth.isLoggedIn();
      final user = loggedIn ? await auth.getCurrentUserInfo() : null;
      final comments = await _market.getIssueComments(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: widget.node.issue.number,
        page: 1,
        perPage: 50,
      );
      final reactions = await _market.getIssueReactions(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: widget.node.issue.number,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _currentUser = user;
        _comments = comments;
        _reactions = reactions;
        _communityLoading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load artifact community: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _communityError = error.toString();
        _communityLoading = false;
      });
    }
  }

  Future<void> _postComment() async {
    final body = _commentController.text.trim();
    if (body.isEmpty || _currentUser == null || _postingComment) {
      return;
    }
    setState(() {
      _postingComment = true;
    });
    try {
      final repo = _artifactIssueRepository(widget.node.type);
      await _market.createIssueComment(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: widget.node.issue.number,
        body: body,
      );
      final comments = await _market.getIssueComments(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: widget.node.issue.number,
        page: 1,
        perPage: 50,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _comments = comments;
        _commentController.clear();
        _postingComment = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to post artifact comment: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _postingComment = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  Future<void> _react(String content) async {
    if (_currentUser == null || _reacting || _hasReaction(content)) {
      return;
    }
    setState(() {
      _reacting = true;
    });
    try {
      final repo = _artifactIssueRepository(widget.node.type);
      final reaction = await _market.createIssueReaction(
        owner: repo.owner,
        repo: repo.repo,
        issueNumber: widget.node.issue.number,
        content: content,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _reactions = <core_proxy.GitHubReaction>[..._reactions, reaction];
        _reacting = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to react artifact issue: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _reacting = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  Future<void> _installNode() async {
    if (_installing) {
      return;
    }
    final confirmed = await _confirmArtifactNodeCompatibility(
      context: context,
      project: widget.project,
      node: widget.node,
    );
    if (!confirmed) {
      return;
    }
    setState(() {
      _installing = true;
    });
    try {
      final result = await _runCoreMarketInstall(
        clients: widget.clients,
        type: widget.node.type,
        projectId: widget.project.projectId,
        nodeId: widget.node.nodeId,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _installing = false;
      });
      if (result.trim().isNotEmpty) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(result), behavior: SnackBarBehavior.floating),
        );
      }
    } catch (error, stackTrace) {
      debugPrint('Failed to install artifact node: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _installing = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  void _continuePublishNode() {
    final node = widget.node;
    final navigator = Navigator.of(context);
    navigator.pop();
    navigator.push(
      MaterialPageRoute<void>(
        builder: (context) => ArtifactPublishScreen(
          clients: widget.clients,
          publishContext: ArtifactPublishClusterContext(
            projectId: node.projectId,
            rootNodeId: node.rootNodeId,
            runtimePackageId: node.runtimePackageId,
            parentNodeIds: <String>[node.nodeId],
            lockedDisplayName: _artifactNodeTitle(node),
            projectDisplayName: _firstNonBlank(<String>[
              widget.project.projectDisplayName,
              node.projectDisplayName,
              _artifactNodeTitle(node),
            ]),
            projectDescription: _firstNonBlank(<String>[
              widget.project.projectDescription,
              node.projectDescription,
              node.description,
            ]),
          ),
        ),
      ),
    );
  }

  bool _hasReaction(String content) {
    final login = _currentUser?.login;
    if (login == null || login.isEmpty) {
      return false;
    }
    return _reactions.any(
      (reaction) => reaction.content == content && reaction.user.login == login,
    );
  }

  int _reactionCount(String content) {
    if (_reactions.isNotEmpty) {
      return _reactions.where((reaction) => reaction.content == content).length;
    }
    final issueReactions = widget.node.issue.reactions;
    return switch (content) {
      '+1' => issueReactions?.thumbsUp ?? 0,
      'heart' => issueReactions?.heart ?? 0,
      'rocket' => issueReactions?.rocket ?? 0,
      _ => 0,
    };
  }

  @override
  Widget build(BuildContext context) {
    final node = widget.node;
    final viewport = MediaQuery.sizeOf(context);
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final currentUser = _currentUser;
    return Dialog(
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxWidth: 780,
          maxHeight: viewport.height * 0.9,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 18, 8, 8),
              child: Row(
                children: <Widget>[
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          _artifactNodeTitle(node),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: textTheme.titleLarge?.copyWith(
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                        const SizedBox(height: 6),
                        Wrap(
                          spacing: 6,
                          runSpacing: 6,
                          children: <Widget>[
                            _SmallChip(text: _artifactTypeLabel(node.type)),
                            _SmallChip(text: node.version),
                            _SmallChip(text: node.publisherLogin),
                            _SmallChip(text: node.state),
                          ],
                        ),
                      ],
                    ),
                  ),
                  IconButton(
                    onPressed: () => Navigator.of(context).pop(),
                    icon: const Icon(Icons.close),
                    tooltip: '关闭',
                  ),
                ],
              ),
            ),
            const Divider(height: 1),
            Flexible(
              child: SingleChildScrollView(
                padding: const EdgeInsets.fromLTRB(20, 16, 20, 20),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    if (node.description.trim().isNotEmpty) ...<Widget>[
                      Text(
                        node.description,
                        style: textTheme.bodyMedium?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                      const SizedBox(height: 16),
                    ],
                    Wrap(
                      spacing: 8,
                      runSpacing: 8,
                      children: <Widget>[
                        _ArtifactMetricChip(
                          icon: Icons.download_outlined,
                          text: '${widget.project.downloads} 下载',
                        ),
                        _ArtifactMetricChip(
                          icon: Icons.thumb_up_alt_outlined,
                          text: '${widget.project.likes} 喜欢',
                        ),
                        _ArtifactMetricChip(
                          icon: Icons.calendar_today_outlined,
                          text: _formatMarketDate(node.issue.createdAt),
                        ),
                      ],
                    ),
                    const SizedBox(height: 18),
                    if (!_isArtifactNodeCompatible(node)) ...<Widget>[
                      _ArtifactCompatibilityBanner(
                        project: widget.project,
                        node: node,
                      ),
                      const SizedBox(height: 18),
                    ],
                    _ArtifactSectionTitle(text: '元数据'),
                    _ArtifactInfoTable(rows: _metadataRows()),
                    const SizedBox(height: 18),
                    _ArtifactSectionTitle(text: '社区反馈'),
                    if (_communityLoading)
                      const Padding(
                        padding: EdgeInsets.symmetric(vertical: 18),
                        child: Center(child: CircularProgressIndicator()),
                      )
                    else if (_communityError != null)
                      _ArtifactErrorPanel(
                        message: _communityError!,
                        onRetry: _loadCommunity,
                      )
                    else ...<Widget>[
                      Wrap(
                        spacing: 8,
                        runSpacing: 8,
                        children: <Widget>[
                          _reactionButton('+1', '喜欢', Icons.thumb_up_outlined),
                          _reactionButton('heart', '收藏', Icons.favorite_border),
                          _reactionButton(
                            'rocket',
                            '推荐',
                            Icons.rocket_launch_outlined,
                          ),
                        ],
                      ),
                      const SizedBox(height: 14),
                      if (_comments.isEmpty)
                        Text(
                          '暂无评论',
                          style: textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        )
                      else
                        for (final comment in _comments)
                          _ArtifactCommentTile(comment: comment),
                      const SizedBox(height: 12),
                      TextField(
                        controller: _commentController,
                        enabled: currentUser != null && !_postingComment,
                        minLines: 2,
                        maxLines: 5,
                        decoration: InputDecoration(
                          labelText: currentUser == null
                              ? '登录 GitHub 后评论'
                              : '发表评论',
                          border: const OutlineInputBorder(),
                          suffixIcon: IconButton(
                            onPressed: currentUser == null || _postingComment
                                ? null
                                : _postComment,
                            icon: _postingComment
                                ? const SizedBox.square(
                                    dimension: 18,
                                    child: CircularProgressIndicator(
                                      strokeWidth: 2,
                                    ),
                                  )
                                : const Icon(Icons.send),
                            tooltip: '发送',
                          ),
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ),
            const Divider(height: 1),
            Padding(
              padding: const EdgeInsets.fromLTRB(12, 10, 12, 12),
              child: Row(
                children: <Widget>[
                  TextButton(
                    onPressed: () => Navigator.of(context).pop(),
                    child: const Text('关闭'),
                  ),
                  const Spacer(),
                  OutlinedButton.icon(
                    onPressed: node.runtimePackageId.trim().isEmpty
                        ? null
                        : _continuePublishNode,
                    icon: const Icon(Icons.update_outlined),
                    label: const Text('发布新版本'),
                  ),
                  const SizedBox(width: 8),
                  FilledButton.icon(
                    onPressed: _installing ? null : _installNode,
                    icon: _installing
                        ? const SizedBox.square(
                            dimension: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.download_outlined),
                    label: Text(_installing ? '下载中' : '下载'),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _reactionButton(String content, String label, IconData icon) {
    final selected = _hasReaction(content);
    return FilterChip(
      selected: selected,
      avatar: Icon(icon, size: 18),
      label: Text('$label ${_reactionCount(content)}'),
      onSelected: selected || _currentUser == null || _reacting
          ? null
          : (_) => _react(content),
    );
  }

  List<_ArtifactInfoRow> _metadataRows() {
    final node = widget.node;
    return <_ArtifactInfoRow>[
      _ArtifactInfoRow(label: '类型', value: _artifactTypeLabel(node.type)),
      _ArtifactInfoRow(label: '版本', value: node.version),
      _ArtifactInfoRow(label: '项目簇', value: node.projectId),
      _ArtifactInfoRow(label: '节点 ID', value: node.nodeId),
      _ArtifactInfoRow(label: '运行时包', value: node.runtimePackageId),
      _ArtifactInfoRow(label: '资源文件', value: node.assetName),
      _ArtifactInfoRow(label: 'Release', value: node.releaseTag),
      _ArtifactInfoRow(label: 'SHA-256', value: node.sha256),
      _ArtifactInfoRow(label: '源文件', value: node.sourceFileName),
      _ArtifactInfoRow(label: '支持版本', value: _supportedVersionLabel(node)),
      const _ArtifactInfoRow(label: '当前软件版本', value: _currentAppVersion),
      _ArtifactInfoRow(
        label: '发布',
        value: _formatMarketDate(node.issue.createdAt),
      ),
      _ArtifactInfoRow(
        label: '更新',
        value: _formatMarketDate(node.issue.updatedAt),
      ),
      _ArtifactInfoRow(label: 'Issue', value: '#${node.issue.number}'),
    ].where((row) => row.value.trim().isNotEmpty).toList(growable: false);
  }
}

class _ArtifactIssueRepository {
  const _ArtifactIssueRepository({
    required this.type,
    required this.owner,
    required this.repo,
    required this.label,
  });

  final String type;
  final String owner;
  final String repo;
  final String label;
}

_ArtifactIssueRepository _artifactIssueRepository(String type) {
  return switch (type.trim()) {
    'package' => const _ArtifactIssueRepository(
      type: 'package',
      owner: 'AAswordman',
      repo: 'OperitPackageMarket',
      label: 'package-artifact',
    ),
    'script' => const _ArtifactIssueRepository(
      type: 'script',
      owner: 'AAswordman',
      repo: 'OperitScriptMarket',
      label: 'script-artifact',
    ),
    final value => throw StateError('Unsupported artifact type: $value'),
  };
}

class _ArtifactMetricChip extends StatelessWidget {
  const _ArtifactMetricChip({required this.icon, required this.text});

  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.secondaryContainer.withValues(alpha: 0.44),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(icon, size: 16, color: colorScheme.onSecondaryContainer),
            const SizedBox(width: 6),
            Text(
              text,
              style: Theme.of(context).textTheme.labelMedium?.copyWith(
                color: colorScheme.onSecondaryContainer,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ArtifactSectionTitle extends StatelessWidget {
  const _ArtifactSectionTitle({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Text(
        text,
        style: Theme.of(
          context,
        ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700),
      ),
    );
  }
}

class _ArtifactInfoRow {
  const _ArtifactInfoRow({required this.label, required this.value});

  final String label;
  final String value;
}

class _ArtifactInfoTable extends StatelessWidget {
  const _ArtifactInfoTable({required this.rows});

  final List<_ArtifactInfoRow> rows;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: colorScheme.outlineVariant),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        children: <Widget>[
          for (var index = 0; index < rows.length; index += 1)
            DecoratedBox(
              decoration: BoxDecoration(
                border: index == rows.length - 1
                    ? null
                    : Border(
                        bottom: BorderSide(color: colorScheme.outlineVariant),
                      ),
              ),
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 9,
                ),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    SizedBox(
                      width: 88,
                      child: Text(
                        rows[index].label,
                        style: textTheme.labelMedium?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                    Expanded(
                      child: SelectableText(
                        rows[index].value,
                        style: textTheme.bodySmall,
                      ),
                    ),
                  ],
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _ArtifactCommentTile extends StatelessWidget {
  const _ArtifactCommentTile({required this.comment});

  final core_proxy.GitHubComment comment;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: colorScheme.surfaceContainerLow,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                children: <Widget>[
                  CircleAvatar(
                    radius: 13,
                    backgroundImage: comment.user.avatarUrl.trim().isEmpty
                        ? null
                        : NetworkImage(comment.user.avatarUrl),
                    child: comment.user.avatarUrl.trim().isEmpty
                        ? Text(
                            comment.user.login.trim().isEmpty
                                ? '?'
                                : comment.user.login.trim()[0],
                          )
                        : null,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      comment.user.login,
                      style: textTheme.labelLarge?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
                  Text(
                    _formatMarketDate(comment.createdAt),
                    style: textTheme.labelSmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              SelectableText(comment.body, style: textTheme.bodySmall),
            ],
          ),
        ),
      ),
    );
  }
}

class _ArtifactCompatibilityBanner extends StatelessWidget {
  const _ArtifactCompatibilityBanner({
    required this.project,
    required this.node,
  });

  final core_proxy.ArtifactProjectDetailResponse project;
  final core_proxy.ArtifactProjectNodeResponse node;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.errorContainer,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Icon(
              Icons.warning_amber_outlined,
              color: colorScheme.onErrorContainer,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    '当前软件版本可能不兼容',
                    style: textTheme.titleSmall?.copyWith(
                      color: colorScheme.onErrorContainer,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    _unsupportedArtifactVersionMessage(project, node),
                    style: textTheme.bodySmall?.copyWith(
                      color: colorScheme.onErrorContainer,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ArtifactErrorPanel extends StatelessWidget {
  const _ArtifactErrorPanel({required this.message, required this.onRetry});

  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        children: <Widget>[
          Expanded(child: Text(message)),
          TextButton.icon(
            onPressed: onRetry,
            icon: const Icon(Icons.refresh),
            label: const Text('重试'),
          ),
        ],
      ),
    );
  }
}

Future<bool> _confirmArtifactNodeCompatibility({
  required BuildContext context,
  required core_proxy.ArtifactProjectDetailResponse project,
  required core_proxy.ArtifactProjectNodeResponse node,
}) async {
  if (_isArtifactNodeCompatible(node)) {
    return true;
  }
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('当前软件版本可能不兼容'),
      content: Text(_unsupportedArtifactVersionMessage(project, node)),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('取消'),
        ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: const Text('仍然继续下载'),
        ),
      ],
    ),
  );
  return confirmed == true;
}

Future<String> _runCoreMarketInstall({
  required GeneratedCoreProxyClients clients,
  required String type,
  required String projectId,
  required String nodeId,
}) async {
  final normalizedType = type.trim();
  if (normalizedType.isEmpty) {
    throw StateError('Artifact type is empty');
  }
  final value = await clients.bridge.call(
    CoreCallRequest(
      requestId: 'flutter-market-${DateTime.now().microsecondsSinceEpoch}',
      targetPath: CoreObjectPath.parse('application'),
      methodName: 'runCoreCommand',
      args: <String, Object?>{
        'args': <String>[
          'market',
          'install',
          normalizedType,
          projectId,
          nodeId,
        ],
      },
    ),
  );
  if (value is! Map<Object?, Object?>) {
    throw StateError('Invalid core command output');
  }
  final stderr = value['stderr']?.toString().trim() ?? '';
  if (stderr.isNotEmpty) {
    throw StateError(stderr);
  }
  final stdout = value['stdout']?.toString().trim() ?? '';
  return stdout.isEmpty ? '安装完成' : stdout;
}

String _unsupportedArtifactVersionMessage(
  core_proxy.ArtifactProjectDetailResponse project,
  core_proxy.ArtifactProjectNodeResponse node,
) {
  final name = _firstNonBlank(<String>[
    node.displayName,
    node.projectDisplayName,
    project.projectDisplayName,
    node.nodeId,
  ]);
  return '「$name」声明支持的软件版本为 ${_supportedVersionLabel(node)}，当前软件版本是 $_currentAppVersion。继续下载仍可能失败或不可用。';
}

String _supportedVersionLabel(core_proxy.ArtifactProjectNodeResponse node) {
  try {
    final minVersion = _normalizeAppVersionOrNull(node.minSupportedAppVersion);
    final maxVersion = _normalizeAppVersionOrNull(node.maxSupportedAppVersion);
    if (minVersion != null && maxVersion != null) {
      return '$minVersion - $maxVersion';
    }
    if (minVersion != null) {
      return '>= $minVersion';
    }
    if (maxVersion != null) {
      return '<= $maxVersion';
    }
    return 'Any';
  } catch (error, stackTrace) {
    debugPrint(
      'Failed to format supported versions for node=${node.nodeId}: $error\n$stackTrace',
    );
    return 'Invalid';
  }
}

bool _isArtifactNodeCompatible(core_proxy.ArtifactProjectNodeResponse node) {
  try {
    return _isAppVersionSupported(
      appVersion: _currentAppVersion,
      minSupportedAppVersion: node.minSupportedAppVersion,
      maxSupportedAppVersion: node.maxSupportedAppVersion,
    );
  } catch (error, stackTrace) {
    debugPrint(
      'Failed to evaluate compatibility for node=${node.nodeId}: $error\n$stackTrace',
    );
    return false;
  }
}

bool _isAppVersionSupported({
  required String appVersion,
  required String? minSupportedAppVersion,
  required String? maxSupportedAppVersion,
}) {
  final normalizedCurrent = _normalizeAppVersionOrNull(appVersion);
  if (normalizedCurrent == null) {
    return true;
  }
  final normalizedMin = _normalizeAppVersionOrNull(minSupportedAppVersion);
  final normalizedMax = _normalizeAppVersionOrNull(maxSupportedAppVersion);
  if (normalizedMin != null &&
      _compareAppVersions(normalizedCurrent, normalizedMin) < 0) {
    return false;
  }
  if (normalizedMax != null &&
      _compareAppVersions(normalizedCurrent, normalizedMax) > 0) {
    return false;
  }
  return true;
}

String? _normalizeAppVersionOrNull(String? value) {
  final trimmed = value?.trim() ?? '';
  if (trimmed.isEmpty) {
    return null;
  }
  final match = RegExp(
    r'^(\d+)\.(\d+)\.(\d+)(?:\+(\d+))?$',
  ).firstMatch(trimmed);
  if (match == null) {
    return null;
  }
  final build = match.group(4);
  return build == null
      ? '${match.group(1)}.${match.group(2)}.${match.group(3)}'
      : '${match.group(1)}.${match.group(2)}.${match.group(3)}+$build';
}

int _compareAppVersions(String left, String right) {
  final leftParts = _appVersionParts(left);
  final rightParts = _appVersionParts(right);
  for (var index = 0; index < leftParts.length; index += 1) {
    final order = leftParts[index].compareTo(rightParts[index]);
    if (order != 0) {
      return order;
    }
  }
  return 0;
}

List<int> _appVersionParts(String value) {
  final match = RegExp(r'^(\d+)\.(\d+)\.(\d+)(?:\+(\d+))?$').firstMatch(value);
  if (match == null) {
    throw StateError('版本格式应为 1.2.3 或 1.2.3+4');
  }
  return <int>[
    int.parse(match.group(1)!),
    int.parse(match.group(2)!),
    int.parse(match.group(3)!),
    int.parse(match.group(4) ?? '0'),
  ];
}

String _formatMarketDate(String value) {
  final trimmed = value.trim();
  if (trimmed.isEmpty) {
    return '-';
  }
  return trimmed.length >= 10 ? trimmed.substring(0, 10) : trimmed;
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
      appBar: AppBar(
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
            return const Center(child: CircularProgressIndicator());
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
    return Card(
      elevation: 0,
      color: colorScheme.surfaceContainerLow,
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
                _SmallChip(text: _artifactTypeLabel(item.type)),
                _SmallChip(text: '#${issue.number}'),
                _SmallChip(text: item.version),
                _SmallChip(text: _formatMarketDate(issue.updatedAt)),
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
              ? const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
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
  final repo = _artifactIssueRepository(item.type);
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
  final repo = _artifactIssueRepository(item.type);
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

## ${_artifactTypeLabel(item.type)}

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

List<_ArtifactIssueRepository> _artifactIssueRepositories() {
  return const <_ArtifactIssueRepository>[
    _ArtifactIssueRepository(
      type: 'script',
      owner: 'AAswordman',
      repo: 'OperitScriptMarket',
      label: 'script-artifact',
    ),
    _ArtifactIssueRepository(
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
      items: displayed,
      groupByUpdatedDate: widget.sortOption == MarketSortOption.updated,
      updatedAt: (item) => item.updatedAt ?? '',
      itemBuilder: (item) => _MarketGridCard(
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

List<_MarketGridGroup<T>> _marketGridGroups<T>({
  required List<T> items,
  required bool groupByUpdatedDate,
  required String Function(T item) updatedAt,
}) {
  if (!groupByUpdatedDate) {
    return <_MarketGridGroup<T>>[
      _MarketGridGroup<T>(label: null, items: items),
    ];
  }
  final groups = <_MarketGridGroup<T>>[];
  String? currentLabel;
  var currentItems = <T>[];
  for (final item in items) {
    final label = _marketUpdatedDateLabel(updatedAt(item));
    if (label != currentLabel) {
      if (currentLabel != null) {
        groups.add(
          _MarketGridGroup<T>(label: currentLabel, items: currentItems),
        );
      }
      currentLabel = label;
      currentItems = <T>[];
    }
    currentItems.add(item);
  }
  if (currentLabel != null) {
    groups.add(_MarketGridGroup<T>(label: currentLabel, items: currentItems));
  }
  return groups;
}

class _MarketGridGroup<T> {
  const _MarketGridGroup({required this.label, required this.items});

  final String? label;
  final List<T> items;
}

String _marketUpdatedDateLabel(String value) {
  final trimmed = value.trim();
  if (trimmed.isEmpty) {
    return '更早';
  }
  return trimmed.length >= 10 ? trimmed.substring(0, 10) : trimmed;
}

class _MarketList<T> extends StatelessWidget {
  const _MarketList({
    required this.isLoading,
    required this.isLoadingMore,
    required this.hasMore,
    required this.isEmpty,
    required this.emptyTitle,
    required this.onRefresh,
    required this.onLoadMore,
    required this.items,
    required this.groupByUpdatedDate,
    required this.updatedAt,
    required this.itemBuilder,
  });

  final bool isLoading;
  final bool isLoadingMore;
  final bool hasMore;
  final bool isEmpty;
  final String emptyTitle;
  final AsyncCallback onRefresh;
  final VoidCallback onLoadMore;
  final List<T> items;
  final bool groupByUpdatedDate;
  final String Function(T item) updatedAt;
  final Widget Function(T item) itemBuilder;

  @override
  Widget build(BuildContext context) {
    final groups = _marketGridGroups<T>(
      items: items,
      groupByUpdatedDate: groupByUpdatedDate,
      updatedAt: updatedAt,
    );
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
            child: CustomScrollView(
              physics: const AlwaysScrollableScrollPhysics(),
              slivers: <Widget>[
                if (isEmpty)
                  SliverPadding(
                    padding: const EdgeInsets.fromLTRB(12, 4, 12, 120),
                    sliver: SliverFillRemaining(
                      hasScrollBody: false,
                      child: EmptyState(
                        icon: Icons.store_outlined,
                        title: emptyTitle,
                        message: '刷新或调整关键词后重试。',
                        scrollable: false,
                      ),
                    ),
                  )
                else
                  for (var index = 0; index < groups.length; index += 1) ...[
                    if (groups[index].label != null)
                      SliverPadding(
                        padding: const EdgeInsets.symmetric(horizontal: 12),
                        sliver: SliverToBoxAdapter(
                          child: _MarketDateHeader(text: groups[index].label!),
                        ),
                      ),
                    SliverPadding(
                      padding: EdgeInsets.fromLTRB(
                        12,
                        groups[index].label == null && index == 0 ? 4 : 0,
                        12,
                        0,
                      ),
                      sliver: _MarketGridSliver<T>(
                        items: groups[index].items,
                        itemBuilder: itemBuilder,
                      ),
                    ),
                  ],
                if (isLoadingMore)
                  const SliverToBoxAdapter(
                    child: Padding(
                      padding: EdgeInsets.symmetric(vertical: 18),
                      child: Center(child: CircularProgressIndicator()),
                    ),
                  ),
                const SliverToBoxAdapter(child: SizedBox(height: 120)),
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

class _MarketGridSliver<T> extends StatelessWidget {
  const _MarketGridSliver({required this.items, required this.itemBuilder});

  final List<T> items;
  final Widget Function(T item) itemBuilder;

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
          itemCount: items.length,
          gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
            crossAxisCount: columnCount,
            crossAxisSpacing: 12,
            mainAxisSpacing: 12,
            mainAxisExtent: 180,
          ),
          itemBuilder: (context, index) => itemBuilder(items[index]),
        );
      },
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

class _MarketRouteHeader extends StatelessWidget {
  const _MarketRouteHeader({required this.title, required this.onBack});

  final String title;
  final VoidCallback onBack;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return SizedBox(
      height: 52,
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border(bottom: BorderSide(color: colorScheme.outlineVariant)),
        ),
        child: Row(
          children: <Widget>[
            const SizedBox(width: 4),
            IconButton(
              onPressed: onBack,
              icon: const Icon(Icons.arrow_back),
              tooltip: '返回',
            ),
            const SizedBox(width: 4),
            Text(
              title,
              style: Theme.of(
                context,
              ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
            ),
          ],
        ),
      ),
    );
  }
}

String _marketTabTitle(MarketHomeTab tab) {
  return switch (tab) {
    MarketHomeTab.artifact => 'Artifact 市场',
    MarketHomeTab.skill => '技能市场',
    MarketHomeTab.mcp => 'MCP 市场',
    MarketHomeTab.mine => '我的市场',
  };
}

class _MarketGridCard extends StatelessWidget {
  const _MarketGridCard({
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
    final textTheme = Theme.of(context).textTheme;
    return Card(
      elevation: 0,
      color: colorScheme.surfaceContainerLow,
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                children: <Widget>[
                  CircleAvatar(
                    radius: 17,
                    backgroundColor: colorScheme.primaryContainer,
                    foregroundColor: colorScheme.onPrimaryContainer,
                    child: Text(
                      title.trim().isEmpty ? '?' : title.trim()[0],
                      style: textTheme.labelLarge?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Text(
                      title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
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
              const SizedBox(height: 10),
              Text(
                description,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 12),
              Wrap(
                spacing: 6,
                runSpacing: 6,
                children: <Widget>[
                  _SmallChip(text: author),
                  _SmallChip(text: '$downloads 下载'),
                  if (likes > 0) _SmallChip(text: '$likes 喜欢'),
                  if (updatedAt != null)
                    _SmallChip(text: _marketUpdatedDateLabel(updatedAt!)),
                  _SmallChip(text: statusLabel),
                ],
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
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
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
