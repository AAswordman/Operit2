// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../components/ExtensionTopBar.dart';
import '../components/StoreCards.dart';
import '../components/StoreDetailKit.dart';
import '../components/StoreVisuals.dart';
import '../market/MarketInstallStateStore.dart';
import '../market/ArtifactMarketSupport.dart';
import '../market/MarketStatsSupport.dart';

String marketSortLabel(MarketSortOption sort) => switch (sort) {
  MarketSortOption.updated => '最近更新',
  MarketSortOption.downloads => '下载最多',
  MarketSortOption.likes => '最受欢迎',
};

String marketSortId(MarketSortOption sort) => switch (sort) {
  MarketSortOption.updated => 'updated',
  MarketSortOption.downloads => 'downloads',
  MarketSortOption.likes => 'likes',
};

/// Number of pages a listing spans.
int marketPageCount(core_proxy.MarketListPage page) {
  if (page.pageSize <= 0) {
    return page.page;
  }
  return math.max(1, (page.total + page.pageSize - 1) ~/ page.pageSize);
}

/// Compact menu that switches the listing order.
class MarketSortMenu extends StatelessWidget {
  const MarketSortMenu({
    super.key,
    required this.sort,
    required this.onChanged,
    this.compact = false,
  });

  final MarketSortOption sort;
  final ValueChanged<MarketSortOption> onChanged;
  final bool compact;

  /// Shows the current order in the tooltip when only the icon is visible.
  @override
  Widget build(BuildContext context) {
    return PopupMenuButton<MarketSortOption>(
      tooltip: '排序：${marketSortLabel(sort)}',
      initialValue: sort,
      onSelected: onChanged,
      icon: compact ? const Icon(Icons.sort_rounded, size: 20) : null,
      itemBuilder: (context) => <PopupMenuEntry<MarketSortOption>>[
        for (final option in MarketSortOption.values)
          PopupMenuItem<MarketSortOption>(
            value: option,
            child: Text(marketSortLabel(option)),
          ),
      ],
      child: compact
          ? null
          : Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  const Icon(Icons.sort_rounded, size: 20),
                  const SizedBox(width: 6),
                  Text(marketSortLabel(sort)),
                ],
              ),
            ),
    );
  }
}

/// Full marketplace listing for one sort, type or category, paged on scroll.
class StoreBrowseScreen extends StatefulWidget {
  const StoreBrowseScreen({
    super.key,
    required this.heading,
    this.type,
    this.categoryId,
    this.initialSort = MarketSortOption.updated,
    GeneratedCoreProxyClients? clients,
  }) : clients =
           clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final String heading;
  final String? type;
  final String? categoryId;
  final MarketSortOption initialSort;
  final GeneratedCoreProxyClients clients;

  @override
  State<StoreBrowseScreen> createState() => _StoreBrowseScreenState();
}

class _StoreBrowseScreenState extends State<StoreBrowseScreen> {
  late MarketSortOption _sort = widget.initialSort;

  @override
  Widget build(BuildContext context) {
    return ExtensionTopBar(
      title: (context) => Text(
        widget.heading,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: Theme.of(
          context,
        ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w600),
      ),
      actions: (context) => <Widget>[
        MarketSortMenu(
          sort: _sort,
          onChanged: (sort) => setState(() => _sort = sort),
        ),
      ],
      child: Material(
        type: MaterialType.transparency,
        child: StoreMarketListing(
          clients: widget.clients,
          type: widget.type,
          categoryId: widget.categoryId,
          sort: _sort,
        ),
      ),
    );
  }
}

/// Paged grid of marketplace rows; reloads when its query changes.
class StoreMarketListing extends StatefulWidget {
  const StoreMarketListing({
    super.key,
    required this.clients,
    required this.sort,
    this.type,
    this.categoryId,
  });

  final GeneratedCoreProxyClients clients;
  final MarketSortOption sort;
  final String? type;
  final String? categoryId;

  @override
  State<StoreMarketListing> createState() => _StoreMarketListingState();
}

class _StoreMarketListingState extends State<StoreMarketListing> {
  late final MarketInstallStateStore _installState;
  List<core_proxy.MarketEntrySummary> _items =
      <core_proxy.MarketEntrySummary>[];
  int _page = 0;
  int _pageCount = 1;
  bool _loading = true;
  bool _loadingMore = false;
  String? _error;
  int _generation = 0;
  final Set<String> _revealed = <String>{};

  GeneratedProvidersMarketStatsApiServiceCoreProxy get _market =>
      widget.clients.providersMarketStatsApiService;

  @override
  void initState() {
    super.initState();
    _installState = MarketInstallStateStore.of(widget.clients);
    _installState.addListener(_onInstallStateChanged);
    unawaited(_installState.refresh());
    unawaited(_reload());
  }

  @override
  void didUpdateWidget(covariant StoreMarketListing oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.sort != widget.sort ||
        oldWidget.type != widget.type ||
        oldWidget.categoryId != widget.categoryId) {
      unawaited(_reload());
    }
  }

  @override
  void dispose() {
    _installState.removeListener(_onInstallStateChanged);
    super.dispose();
  }

  void _onInstallStateChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  Future<core_proxy.MarketListPage> _loadPage(int page) {
    final type = widget.type;
    final categoryId = widget.categoryId;
    final sort = marketSortId(widget.sort);
    if (type != null && categoryId != null) {
      return _market.getTypeCategoryPage(
        type: type,
        categoryId: categoryId,
        sort: sort,
        page: page,
      );
    }
    if (type != null) {
      return _market.getTypePage(type: type, sort: sort, page: page);
    }
    if (categoryId != null) {
      return _market.getCategoryPage(
        categoryId: categoryId,
        sort: sort,
        page: page,
      );
    }
    return _market.getListPage(sort: sort, page: page);
  }

  Future<void> _reload() async {
    final generation = ++_generation;
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final page = await _loadPage(1);
      if (!mounted || generation != _generation) return;
      setState(() {
        _items = page.items;
        _page = page.page;
        _pageCount = marketPageCount(page);
        _loading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load market listing: $error\n$stackTrace');
      if (!mounted || generation != _generation) return;
      setState(() {
        _error = error.toString();
        _loading = false;
      });
    }
  }

  Future<void> _loadMore() async {
    if (_loading || _loadingMore || _page >= _pageCount) return;
    final generation = _generation;
    setState(() => _loadingMore = true);
    try {
      final page = await _loadPage(_page + 1);
      if (!mounted || generation != _generation) return;
      setState(() {
        _items = <core_proxy.MarketEntrySummary>[..._items, ...page.items];
        _page = page.page;
        _pageCount = marketPageCount(page);
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load more market entries: $error\n$stackTrace');
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('$error'), behavior: SnackBarBehavior.floating),
      );
    } finally {
      if (mounted) {
        setState(() => _loadingMore = false);
      }
    }
  }

  Iterable<Widget> _entrySlivers(EdgeInsets padding) sync* {
    final dated = widget.sort == MarketSortOption.updated;
    String date(core_proxy.MarketEntrySummary entry) =>
        entry.updatedAt.trim().isEmpty
        ? '更早'
        : formatMarketDate(entry.updatedAt);
    final groups = <List<core_proxy.MarketEntrySummary>>[];
    for (final entry in _items) {
      if (groups.isEmpty || (dated && date(groups.last.last) != date(entry))) {
        groups.add(<core_proxy.MarketEntrySummary>[]);
      }
      groups.last.add(entry);
    }
    for (final group in groups) {
      if (dated) {
        yield SliverPadding(
          padding: EdgeInsets.fromLTRB(padding.left + 10, 16, padding.right, 8),
          sliver: SliverToBoxAdapter(
            child: Text(
              date(group.first),
              style: Theme.of(context).textTheme.labelLarge?.copyWith(
                fontWeight: FontWeight.w600,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ),
        );
      }
      yield SliverPadding(
        padding: dated ? padding.copyWith(top: 0, bottom: 16) : padding,
        sliver: SliverGrid(
          gridDelegate: SliverGridDelegateWithMaxCrossAxisExtent(
            maxCrossAxisExtent: 520,
            mainAxisExtent: 88 * storeTextScale(context),
            crossAxisSpacing: 16,
            mainAxisSpacing: 4,
          ),
          delegate: SliverChildBuilderDelegate((context, index) {
            final entry = group[index];
            return StoreEntrance(
              index: index % 12,
              animate: _revealed.add(entry.id),
              child: StoreMarketRow(entry: entry, installState: _installState),
            );
          }, childCount: group.length),
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return NotificationListener<ScrollNotification>(
      onNotification: (notification) {
        if (notification.metrics.axis == Axis.vertical &&
            notification.metrics.extentAfter < 480) {
          unawaited(_loadMore());
        }
        return false;
      },
      child: LayoutBuilder(
        builder: (context, constraints) {
          final padding = storeSliverPadding(
            constraints.maxWidth,
            gutter: constraints.maxWidth < 600 ? 12 : 24,
            top: 8,
            bottom: 40,
          );
          return CustomScrollView(
            slivers: <Widget>[
              if (_loading)
                SliverPadding(
                  padding: padding,
                  sliver: const SliverToBoxAdapter(child: StoreListSkeleton()),
                )
              else if (_error != null)
                SliverPadding(
                  padding: padding,
                  sliver: SliverToBoxAdapter(
                    child: StoreNoticeCard(
                      icon: Icons.cloud_off_rounded,
                      tone: StoreNoticeTone.error,
                      title: '市场暂时无法连接',
                      message: _error,
                      action: TextButton(
                        onPressed: _reload,
                        child: const Text('重试'),
                      ),
                    ),
                  ),
                )
              else if (_items.isEmpty)
                SliverFillRemaining(
                  hasScrollBody: false,
                  child: Center(
                    child: Text(
                      '这里还没有内容',
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                  ),
                )
              else
                ..._entrySlivers(padding),
              if (_loadingMore)
                const SliverToBoxAdapter(
                  child: Padding(
                    padding: EdgeInsets.only(bottom: 32),
                    child: Center(child: CircularProgressIndicator()),
                  ),
                ),
            ],
          );
        },
      ),
    );
  }
}

/// Shimmering placeholder rows for marketplace lists.
class StoreListSkeleton extends StatelessWidget {
  const StoreListSkeleton({super.key, this.rows = 6});

  final int rows;

  @override
  Widget build(BuildContext context) {
    return StoreShimmer(
      child: Column(
        children: <Widget>[
          for (var index = 0; index < rows; index += 1)
            const Padding(
              padding: EdgeInsets.symmetric(vertical: 12, horizontal: 10),
              child: Row(
                children: <Widget>[
                  StoreSkeletonBox(width: 44, height: 44, radius: 13),
                  SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        StoreSkeletonBox(width: 160, height: 14),
                        SizedBox(height: 8),
                        StoreSkeletonBox(width: 220, height: 12),
                      ],
                    ),
                  ),
                  SizedBox(width: 14),
                  StoreSkeletonBox(width: 72, height: 32, radius: 16),
                ],
              ),
            ),
        ],
      ),
    );
  }
}
