// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../settings/components/SettingsControlStyles.dart';
import '../components/ExtensionAvatar.dart';
import '../components/ExtensionMotion.dart';
import '../components/ExtensionNavigation.dart';
import '../components/StoreCards.dart';
import '../components/StoreOfficialExtensionRow.dart';
import '../components/StoreDetailKit.dart';
import '../components/StoreVisuals.dart';
import '../market/MarketInstallStateStore.dart';
import '../market/ArtifactMarketSupport.dart';
import '../market/MarketRecommendations.dart';
import '../market/MarketStatsSupport.dart';
import '../model/ExtensionCatalogController.dart';
import '../model/GitHubAccountState.dart';
import 'StoreBrowseScreen.dart';

/// Market section: featured entries, official offline extensions, community
/// shelves, categories, search and the way into the creator center.
class MarketPane extends StatefulWidget {
  const MarketPane({super.key, required this.controller, required this.query});

  final ExtensionCatalogController controller;
  final String query;

  @override
  State<MarketPane> createState() => _MarketPaneState();
}

class _MarketPaneState extends State<MarketPane> {
  static const double _rowHeight = 80;
  static const double _cardHeight = 224;

  late final MarketInstallStateStore _installState;
  late final GitHubAccountState _account = GitHubAccountState.of(_clients);
  StoreKind? _kind;
  MarketSortOption _kindSort = MarketSortOption.updated;
  int _homeGeneration = 0;
  bool _homeLoading = true;
  String? _homeError;
  List<core_proxy.MarketEntrySummary> _recent =
      const <core_proxy.MarketEntrySummary>[];
  List<core_proxy.MarketEntrySummary> _popular =
      const <core_proxy.MarketEntrySummary>[];
  List<core_proxy.MarketEntrySummary> _loved =
      const <core_proxy.MarketEntrySummary>[];
  List<core_proxy.MarketEntrySummary> _recommendations =
      const <core_proxy.MarketEntrySummary>[];
  List<core_proxy.MarketCategoryInfo> _categories =
      const <core_proxy.MarketCategoryInfo>[];
  List<core_proxy.MarketEntrySummary>? _allEntries;
  bool _searchLoading = false;
  String? _searchError;

  /// Official candidates in first-seen order, so installed ones keep their slot.
  final List<core_proxy.ExtensionCatalogCandidate> _official =
      <core_proxy.ExtensionCatalogCandidate>[];

  ExtensionCatalogController get _controller => widget.controller;
  GeneratedCoreProxyClients get _clients => widget.controller.clients;
  GeneratedProvidersMarketStatsApiServiceCoreProxy get _market =>
      _clients.providersMarketStatsApiService;

  @override
  void initState() {
    super.initState();
    _installState = MarketInstallStateStore.of(_clients);
    _installState.addListener(_onChanged);
    _controller.addListener(_onChanged);
    _account.addListener(_onChanged);
    unawaited(_installState.refresh());
    unawaited(_account.ensureLoaded());
    unawaited(_loadHome());
    if (widget.query.trim().isNotEmpty) {
      unawaited(_loadAllEntries());
    }
  }

  @override
  void didUpdateWidget(covariant MarketPane oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.query.trim().isNotEmpty &&
        _allEntries == null &&
        !_searchLoading) {
      unawaited(_loadAllEntries());
    }
  }

  @override
  void dispose() {
    _installState.removeListener(_onChanged);
    _controller.removeListener(_onChanged);
    _account.removeListener(_onChanged);
    super.dispose();
  }

  void _onChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  Future<void> _loadHome() async {
    final generation = ++_homeGeneration;
    setState(() {
      _homeLoading = true;
      _homeError = null;
    });
    try {
      final results = await Future.wait<Object>(<Future<Object>>[
        _market.getListPage(sort: 'updated', page: 1),
        _market.getListPage(sort: 'downloads', page: 1),
        _market.getListPage(sort: 'likes', page: 1),
        _market.getManifest(),
      ]);
      if (!mounted || generation != _homeGeneration) return;
      final recent = results[0] as core_proxy.MarketListPage;
      final candidates = recent.items;
      setState(() {
        _recent = recent.items;
        _popular = (results[1] as core_proxy.MarketListPage).items;
        _loved = (results[2] as core_proxy.MarketListPage).items;
        _categories = (results[3] as core_proxy.MarketManifest).categories;
        _recommendations = marketRecommendations(candidates);
        _homeLoading = false;
      });
      final extra = await Future.wait([
        for (var page = 2; page <= math.min(3, marketPageCount(recent)); page++)
          _market
              .getListPage(sort: 'updated', page: page)
              .then(
                (result) => result.items,
                onError: (Object error, StackTrace stackTrace) {
                  debugPrint(
                    'Failed to load market recommendations: $error\n$stackTrace',
                  );
                  return <core_proxy.MarketEntrySummary>[];
                },
              ),
      ]);
      if (!mounted || generation != _homeGeneration || extra.isEmpty) return;
      setState(() {
        _recommendations = marketRecommendations([
          ...candidates,
          ...extra.expand((entries) => entries),
        ]);
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load market home: $error\n$stackTrace');
      if (!mounted || generation != _homeGeneration) return;
      setState(() {
        _homeError = error.toString();
        _homeLoading = false;
      });
    }
  }

  /// Loads every listing page once so typed queries can match the whole market.
  Future<void> _loadAllEntries() async {
    setState(() {
      _searchLoading = true;
      _searchError = null;
    });
    try {
      final entries = <core_proxy.MarketEntrySummary>[];
      var page = 1;
      var pageCount = 1;
      do {
        final result = await _market.getListPage(sort: 'updated', page: page);
        entries.addAll(result.items);
        pageCount = marketPageCount(result);
        page += 1;
      } while (page <= pageCount);
      if (!mounted) return;
      setState(() {
        _allEntries = entries;
        _searchLoading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load market for search: $error\n$stackTrace');
      if (!mounted) return;
      setState(() {
        _searchError = error.toString();
        _searchLoading = false;
      });
    }
  }

  void _syncOfficial() {
    final snapshot = _controller.snapshot;
    if (snapshot == null) {
      return;
    }
    for (final candidate in snapshot.bundledAvailable) {
      final known = _official.any(
        (item) => item.kind == candidate.kind && item.id == candidate.id,
      );
      if (!known) {
        _official.add(candidate);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    _syncOfficial();
    final motion = ExtensionMotion.of(context);
    final query = widget.query.trim();
    final kind = _kind;
    final Widget body;
    final String bodyKey;
    if (query.isNotEmpty) {
      body = _buildSearch(context, query.toLowerCase());
      bodyKey = 'search';
    } else if (kind != null) {
      body = StoreMarketListing(
        clients: _clients,
        sort: _kindSort,
        type: kind.marketType,
      );
      bodyKey = 'kind:${kind.id}';
    } else {
      body = _buildHome(context);
      bodyKey = 'home';
    }
    return Column(
      children: <Widget>[
        _buildFilterBar(context, searching: query.isNotEmpty),
        Expanded(
          // Only the incoming view is shown; it rises in without a cross-fade,
          // so glass cards never fade.
          child: AnimatedSwitcher(
            duration: motion.emphasized,
            switchInCurve: ExtensionMotion.enter,
            layoutBuilder: (current, previous) =>
                current ?? const SizedBox.shrink(),
            transitionBuilder: (child, animation) => SlideTransition(
              position: Tween<Offset>(
                begin: const Offset(0, 0.02),
                end: Offset.zero,
              ).animate(animation),
              child: child,
            ),
            child: KeyedSubtree(key: ValueKey<String>(bodyKey), child: body),
          ),
        ),
      ],
    );
  }

  /// Keeps action labels only when the filter row has room for the tabs.
  Widget _buildFilterBar(BuildContext context, {required bool searching}) {
    final kind = _kind;
    return LayoutBuilder(
      builder: (context, constraints) {
        final padding = storeSliverPadding(
          constraints.maxWidth,
          gutter: constraints.maxWidth < 600 ? 16 : 24,
        );
        final showSort = kind != null && !searching;
        final compactActions =
            showSort &&
            constraints.maxWidth - padding.horizontal <
                600 * storeTextScale(context);
        return Padding(
          padding: EdgeInsets.fromLTRB(padding.left, 8, padding.right, 8),
          child: Row(
            children: <Widget>[
              Expanded(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    children: <Widget>[
                      StoreFilterChip(
                        label: searching ? '全部' : '推荐',
                        icon: searching
                            ? Icons.apps_rounded
                            : Icons.local_fire_department_outlined,
                        selected: kind == null,
                        onSelected: () => setState(() => _kind = null),
                      ),
                      for (final option in StoreKind.values) ...<Widget>[
                        const SizedBox(width: 8),
                        StoreFilterChip(
                          label: option.label,
                          icon: option.icon,
                          selected: identical(kind, option),
                          onSelected: () => setState(() => _kind = option),
                        ),
                      ],
                    ],
                  ),
                ),
              ),
              if (showSort)
                MarketSortMenu(
                  sort: _kindSort,
                  compact: compactActions,
                  onChanged: (sort) => setState(() => _kindSort = sort),
                ),
              const SizedBox(width: 4),
              _CreatorEntry(account: _account, compact: compactActions),
            ],
          ),
        );
      },
    );
  }

  // --------------------------------------------------------------------- home

  Widget _buildHome(BuildContext context) {
    return RefreshIndicator(
      onRefresh: () async {
        await Future.wait<void>(<Future<void>>[
          _installState.refresh(),
          _controller.refresh(),
          _loadHome(),
        ]);
      },
      child: LayoutBuilder(
        builder: (context, constraints) {
          final width = constraints.maxWidth;
          final padding = storeSliverPadding(
            width,
            gutter: width < 600 ? 16 : 24,
          );
          final contentWidth = width - padding.horizontal;
          return CustomScrollView(
            physics: const AlwaysScrollableScrollPhysics(),
            slivers: <Widget>[
              if (_recommendations.isNotEmpty)
                SliverToBoxAdapter(
                  child: _FeaturedCarousel(
                    entries: _recommendations,
                    itemBuilder: (context, entry) => StoreFeaturedBanner(
                      entry: entry,
                      installState: _installState,
                    ),
                  ),
                ),
              if (_official.isNotEmpty)
                _section(
                  padding: padding,
                  title: '官方扩展',
                  subtitle: '随 Operit2 一起提供，不联网也能安装',
                  onSeeAll: openOfficialExtensions,
                  child: _rowShelf<core_proxy.ExtensionCatalogCandidate>(
                    items: _official,
                    contentWidth: contentWidth,
                    rowBuilder: (candidate, index) => _candidateRow(candidate),
                  ),
                ),
              if (_homeLoading)
                SliverPadding(
                  padding: padding.copyWith(top: 24),
                  sliver: const SliverToBoxAdapter(child: _HomeSkeleton()),
                )
              else if (_homeError != null)
                SliverPadding(
                  padding: padding.copyWith(top: 24),
                  sliver: SliverToBoxAdapter(
                    child: StoreNoticeCard(
                      icon: Icons.cloud_off_rounded,
                      tone: StoreNoticeTone.error,
                      title: '市场暂时无法连接',
                      message: _homeError,
                      action: TextButton(
                        onPressed: _loadHome,
                        child: const Text('重试'),
                      ),
                    ),
                  ),
                )
              else ...<Widget>[
                if (_recent.isNotEmpty)
                  _section(
                    padding: padding,
                    title: '最近更新',
                    subtitle: '社区刚刚发布或更新的扩展',
                    onSeeAll: () => openMarketBrowse(heading: '最近更新'),
                    child: _cardShelf(_recent),
                  ),
                if (_popular.isNotEmpty)
                  _section(
                    padding: padding,
                    title: '下载排行',
                    onSeeAll: () => openMarketBrowse(
                      heading: '下载排行',
                      sort: MarketSortOption.downloads,
                    ),
                    child: _rowShelf<core_proxy.MarketEntrySummary>(
                      items: _popular,
                      contentWidth: contentWidth,
                      rowBuilder: (entry, index) => StoreMarketRow(
                        entry: entry,
                        installState: _installState,
                        rank: index + 1,
                      ),
                    ),
                  ),
                if (_loved.isNotEmpty)
                  _section(
                    padding: padding,
                    title: '大家都喜欢',
                    subtitle: '获得点赞最多的扩展',
                    onSeeAll: () => openMarketBrowse(
                      heading: '大家都喜欢',
                      sort: MarketSortOption.likes,
                    ),
                    child: _cardShelf(_loved),
                  ),
                if (_categories.isNotEmpty)
                  _section(
                    padding: padding,
                    title: '按分类浏览',
                    child: _CategoryGrid(
                      categories: _categories,
                      onTap: (category) => openMarketBrowse(
                        heading: marketCategoryText(
                          category,
                          languageCode: Localizations.localeOf(
                            context,
                          ).languageCode,
                        ).name,
                        categoryId: category.id,
                      ),
                    ),
                  ),
              ],
              const SliverToBoxAdapter(child: SizedBox(height: 48)),
            ],
          );
        },
      ),
    );
  }

  Widget _section({
    required EdgeInsets padding,
    required String title,
    String? subtitle,
    VoidCallback? onSeeAll,
    required Widget child,
  }) {
    return SliverPadding(
      padding: EdgeInsets.fromLTRB(padding.left, 24, padding.right, 0),
      sliver: SliverToBoxAdapter(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            StoreSectionHeader(
              title: title,
              subtitle: subtitle,
              trailing: onSeeAll == null
                  ? null
                  : StoreSeeAllButton(onPressed: onSeeAll),
            ),
            child,
          ],
        ),
      ),
    );
  }

  Widget _cardShelf(List<core_proxy.MarketEntrySummary> entries) {
    return StoreShelf(
      itemCount: entries.length,
      itemWidth: 228,
      height: _cardHeight * storeTextScale(context) + 16,
      itemBuilder: (context, index) =>
          StoreMarketCard(entry: entries[index], installState: _installState),
    );
  }

  /// Shelf of three-row columns that page sideways.
  Widget _rowShelf<T>({
    required List<T> items,
    required double contentWidth,
    required Widget Function(T item, int index) rowBuilder,
  }) {
    const rows = 3;
    final columns = (items.length + rows - 1) ~/ rows;
    final columnWidth = contentWidth < 720 ? contentWidth * 0.88 : 380.0;
    final rowHeight = _rowHeight * storeTextScale(context);
    return StoreShelf(
      itemCount: columns,
      itemWidth: columnWidth,
      height: rows * rowHeight + 16,
      itemBuilder: (context, column) => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          for (var row = 0; row < rows; row += 1)
            if (column * rows + row < items.length)
              SizedBox(
                height: rowHeight,
                child: rowBuilder(
                  items[column * rows + row],
                  column * rows + row,
                ),
              ),
        ],
      ),
    );
  }

  Widget _candidateRow(core_proxy.ExtensionCatalogCandidate candidate) {
    return StoreOfficialExtensionRow(
      candidate: candidate,
      controller: _controller,
    );
  }

  // ------------------------------------------------------------------- search

  Widget _buildSearch(BuildContext context, String query) {
    final kind = _kind;
    bool kindMatches(String value) =>
        kind == null || identical(StoreKind.of(value), kind);
    final official = _official
        .where(
          (candidate) =>
              kindMatches(candidate.kind) &&
              <String>[
                candidate.displayName,
                candidate.id,
                candidate.description,
              ].any((value) => value.toLowerCase().contains(query)),
        )
        .toList(growable: false);
    final market = (_allEntries ?? const <core_proxy.MarketEntrySummary>[])
        .where(
          (entry) =>
              kindMatches(entry.type) &&
              <String>[
                entry.title,
                entry.id,
                entry.description,
                marketEntryPublisher(entry),
              ].any((value) => value.toLowerCase().contains(query)),
        )
        .toList(growable: false);
    return LayoutBuilder(
      builder: (context, constraints) {
        final padding = storeSliverPadding(
          constraints.maxWidth,
          gutter: constraints.maxWidth < 600 ? 12 : 24,
        );
        final grid = SliverGridDelegateWithMaxCrossAxisExtent(
          maxCrossAxisExtent: 520,
          mainAxisExtent: 88 * storeTextScale(context),
          crossAxisSpacing: 16,
          mainAxisSpacing: 4,
        );
        return CustomScrollView(
          slivers: <Widget>[
            if (official.isNotEmpty) ...<Widget>[
              _searchHeader(padding, '官方扩展', official.length),
              SliverPadding(
                padding: EdgeInsets.fromLTRB(padding.left, 0, padding.right, 0),
                sliver: SliverGrid(
                  gridDelegate: grid,
                  delegate: SliverChildBuilderDelegate(
                    (context, index) => _candidateRow(official[index]),
                    childCount: official.length,
                  ),
                ),
              ),
            ],
            _searchHeader(
              padding,
              '社区市场',
              _searchLoading ? null : market.length,
            ),
            if (_searchLoading)
              SliverPadding(
                padding: EdgeInsets.fromLTRB(padding.left, 0, padding.right, 0),
                sliver: const SliverToBoxAdapter(
                  child: StoreListSkeleton(rows: 4),
                ),
              )
            else if (_searchError != null)
              SliverPadding(
                padding: EdgeInsets.fromLTRB(padding.left, 0, padding.right, 0),
                sliver: SliverToBoxAdapter(
                  child: StoreNoticeCard(
                    icon: Icons.cloud_off_rounded,
                    tone: StoreNoticeTone.error,
                    title: '市场暂时无法连接',
                    message: _searchError,
                    action: TextButton(
                      onPressed: _loadAllEntries,
                      child: const Text('重试'),
                    ),
                  ),
                ),
              )
            else if (market.isEmpty)
              SliverPadding(
                padding: EdgeInsets.fromLTRB(padding.left, 8, padding.right, 0),
                sliver: SliverToBoxAdapter(
                  child: Text(
                    '市场里没有找到「${widget.query.trim()}」',
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
              )
            else
              SliverPadding(
                padding: EdgeInsets.fromLTRB(padding.left, 0, padding.right, 0),
                sliver: SliverGrid(
                  gridDelegate: grid,
                  delegate: SliverChildBuilderDelegate(
                    (context, index) => StoreMarketRow(
                      entry: market[index],
                      installState: _installState,
                    ),
                    childCount: market.length,
                  ),
                ),
              ),
            const SliverToBoxAdapter(child: SizedBox(height: 48)),
          ],
        );
      },
    );
  }

  Widget _searchHeader(EdgeInsets padding, String title, int? count) {
    return SliverPadding(
      padding: EdgeInsets.fromLTRB(padding.left + 10, 16, padding.right, 6),
      sliver: SliverToBoxAdapter(
        child: Text(
          count == null ? title : '$title · $count',
          style: SettingsControlStyles.sectionTitleTextStyle(context),
        ),
      ),
    );
  }
}

/// Auto-advancing banner carousel; neighbours shrink beside the centred one.
class _FeaturedCarousel extends StatefulWidget {
  const _FeaturedCarousel({required this.entries, required this.itemBuilder});

  final List<core_proxy.MarketEntrySummary> entries;
  final Widget Function(
    BuildContext context,
    core_proxy.MarketEntrySummary entry,
  )
  itemBuilder;

  @override
  State<_FeaturedCarousel> createState() => _FeaturedCarouselState();
}

class _FeaturedCarouselState extends State<_FeaturedCarousel> {
  PageController? _controller;
  double _fraction = 0;
  int _page = 0;
  bool _hovered = false;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _timer = Timer.periodic(const Duration(seconds: 6), (_) => _advance());
  }

  @override
  void dispose() {
    _timer?.cancel();
    _controller?.dispose();
    super.dispose();
  }

  void _advance() {
    final controller = _controller;
    if (!mounted ||
        _hovered ||
        controller == null ||
        !controller.hasClients ||
        widget.entries.length < 2 ||
        !TickerMode.getValuesNotifier(context).value.enabled ||
        !ExtensionMotion.of(context).ambient) {
      return;
    }
    controller.animateToPage(
      (_page + 1) % widget.entries.length,
      duration: const Duration(milliseconds: 650),
      curve: Curves.easeInOutCubic,
    );
  }

  PageController _controllerFor(double fraction) {
    final current = _controller;
    if (current != null && fraction == _fraction) {
      return current;
    }
    if (current != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) => current.dispose());
    }
    _fraction = fraction;
    return _controller = PageController(
      viewportFraction: fraction,
      initialPage: _page,
    );
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final motion = ExtensionMotion.of(context);
    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth;
        final bannerWidth = math.min(width * 0.92, 820.0);
        final fraction = (bannerWidth / width).clamp(0.3, 1.0);
        final controller = _controllerFor(fraction);
        final height = (width < 600 ? 216.0 : 232.0) * storeTextScale(context);
        return MouseRegion(
          onEnter: (_) => _hovered = true,
          onExit: (_) => _hovered = false,
          child: Column(
            children: <Widget>[
              SizedBox(
                height: height,
                child: ScrollConfiguration(
                  behavior: ScrollConfiguration.of(context).copyWith(
                    scrollbars: false,
                    dragDevices: <PointerDeviceKind>{
                      PointerDeviceKind.touch,
                      PointerDeviceKind.mouse,
                      PointerDeviceKind.trackpad,
                      PointerDeviceKind.stylus,
                    },
                  ),
                  child: PageView.builder(
                    key: ValueKey<double>(fraction),
                    controller: controller,
                    itemCount: widget.entries.length,
                    onPageChanged: (page) => setState(() => _page = page),
                    itemBuilder: (context, index) => AnimatedBuilder(
                      animation: controller,
                      child: Padding(
                        padding: const EdgeInsets.fromLTRB(6, 8, 6, 14),
                        child: widget.itemBuilder(
                          context,
                          widget.entries[index],
                        ),
                      ),
                      builder: (context, child) {
                        final page =
                            controller.hasClients &&
                                controller.position.haveDimensions
                            ? controller.page!
                            : _page.toDouble();
                        final distance = (page - index).abs().clamp(0.0, 1.0);
                        return Transform.scale(
                          scale: 1 - distance * 0.06,
                          child: Opacity(
                            opacity: 1 - distance * 0.35,
                            child: child,
                          ),
                        );
                      },
                    ),
                  ),
                ),
              ),
              if (widget.entries.length > 1)
                Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: <Widget>[
                    for (
                      var index = 0;
                      index < widget.entries.length;
                      index += 1
                    )
                      AnimatedContainer(
                        duration: motion.standard,
                        curve: ExtensionMotion.enter,
                        margin: const EdgeInsets.symmetric(horizontal: 3),
                        width: index == _page ? 20 : 6,
                        height: 6,
                        decoration: BoxDecoration(
                          color: index == _page
                              ? colorScheme.primary
                              : colorScheme.onSurfaceVariant.withValues(
                                  alpha: 0.3,
                                ),
                          borderRadius: BorderRadius.circular(3),
                        ),
                      ),
                  ],
                ),
            ],
          ),
        );
      },
    );
  }
}

/// Shares each row's available width, including an incomplete final row.
class _CategoryGrid extends StatelessWidget {
  const _CategoryGrid({required this.categories, required this.onTap});

  final List<core_proxy.MarketCategoryInfo> categories;
  final ValueChanged<core_proxy.MarketCategoryInfo> onTap;

  @override
  Widget build(BuildContext context) {
    if (categories.isEmpty) return const SizedBox.shrink();
    final textScale = storeTextScale(context);
    return LayoutBuilder(
      builder: (context, constraints) {
        const spacing = 10.0;
        final columns = math.min(
          categories.length,
          math.max(1, (constraints.maxWidth / (176 * textScale)).round()),
        );
        return Column(
          children: <Widget>[
            for (var start = 0; start < categories.length; start += columns)
              Padding(
                padding: EdgeInsets.only(top: start == 0 ? 0 : spacing),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    for (
                      var index = start;
                      index < math.min(start + columns, categories.length);
                      index++
                    ) ...<Widget>[
                      if (index > start) const SizedBox(width: spacing),
                      Expanded(
                        child: _CategoryTile(
                          category: categories[index],
                          onTap: () => onTap(categories[index]),
                        ),
                      ),
                    ],
                  ],
                ),
              ),
          ],
        );
      },
    );
  }
}

/// Category in its own tonal colors, its first character as quiet artwork.
class _CategoryTile extends StatelessWidget {
  const _CategoryTile({required this.category, required this.onTap});

  final core_proxy.MarketCategoryInfo category;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final dark = theme.brightness == Brightness.dark;
    final palette = ExtensionAvatarPalette.of(
      context,
      'category:${category.id}',
    );
    final ink = palette.onContainer;
    final text = marketCategoryText(
      category,
      languageCode: Localizations.localeOf(context).languageCode,
    );
    final description = text.description;
    return ConstrainedBox(
      constraints: BoxConstraints(minHeight: 80 * storeTextScale(context)),
      child: DecoratedBox(
        decoration: BoxDecoration(
          borderRadius: storeCardRadius,
          border: Border.all(color: ink.withValues(alpha: dark ? 0.12 : 0.08)),
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: <Color>[
              Color.lerp(palette.container, Colors.white, dark ? 0.05 : 0.3)!,
              palette.container,
            ],
          ),
        ),
        child: ClipRRect(
          borderRadius: storeCardRadius,
          child: Material(
            type: MaterialType.transparency,
            child: InkWell(
              onTap: onTap,
              child: Stack(
                children: <Widget>[
                  Positioned(
                    right: -6,
                    bottom: -24,
                    child: Text(
                      extensionMonogram(text.name),
                      maxLines: 1,
                      softWrap: false,
                      textScaler: TextScaler.noScaling,
                      style: TextStyle(
                        fontSize: 72,
                        height: 1,
                        fontWeight: FontWeight.w800,
                        color: ink.withValues(alpha: 0.08),
                      ),
                    ),
                  ),
                  Padding(
                    padding: const EdgeInsets.all(12),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          text.name,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: theme.textTheme.titleSmall?.copyWith(
                            color: ink,
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                        if (description.isNotEmpty) ...<Widget>[
                          const SizedBox(height: 4),
                          Text(
                            description,
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                            style: theme.textTheme.labelSmall?.copyWith(
                              color: ink.withValues(alpha: 0.75),
                            ),
                          ),
                        ],
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _HomeSkeleton extends StatelessWidget {
  const _HomeSkeleton();

  @override
  Widget build(BuildContext context) {
    return StoreShimmer(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          for (var section = 0; section < 2; section += 1) ...<Widget>[
            const StoreSkeletonBox(width: 140, height: 18),
            const SizedBox(height: 14),
            Row(
              children: <Widget>[
                for (var card = 0; card < 3; card += 1) ...<Widget>[
                  if (card > 0) const SizedBox(width: 12),
                  const Expanded(
                    child: StoreSkeletonBox(height: 188, radius: 12),
                  ),
                ],
              ],
            ),
            const SizedBox(height: 28),
          ],
        ],
      ),
    );
  }
}

/// Way into the creator center; wears the signed-in GitHub profile picture.
class _CreatorEntry extends StatelessWidget {
  const _CreatorEntry({required this.account, required this.compact});

  final GitHubAccountState account;
  final bool compact;

  /// Keeps the account avatar and accessible label in either presentation.
  @override
  Widget build(BuildContext context) {
    final user = account.user;
    final icon = user != null
        ? StoreAvatar(name: user.login, url: user.avatarUrl, radius: 10)
        : const Icon(Icons.badge_outlined, size: 18);
    if (compact) {
      return IconButton(
        tooltip: '创作者中心',
        onPressed: openMarketCreator,
        color: Theme.of(context).colorScheme.primary,
        icon: icon,
      );
    }
    return TextButton(
      onPressed: openMarketCreator,
      style: SettingsControlStyles.sectionTextButton(),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[icon, const SizedBox(width: 6), const Text('创作者中心')],
      ),
    );
  }
}
