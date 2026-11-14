// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart';
import '../market/ArtifactMarketSupport.dart';
import '../market/MarketFilters.dart';

/// One entry point for sorting, publication period and category selection.
class MarketSortMenu extends StatelessWidget {
  const MarketSortMenu({
    super.key,
    required this.filters,
    required this.clients,
    required this.onChanged,
    this.categories,
    this.compact = false,
  });

  final MarketFilters filters;
  final GeneratedCoreProxyClients clients;
  final ValueChanged<MarketFilters> onChanged;
  final List<MarketCategoryInfo>? categories;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    Future<void> open() async {
      final selected = await showModalBottomSheet<MarketFilters>(
        context: context,
        isScrollControlled: true,
        useSafeArea: true,
        showDragHandle: true,
        constraints: const BoxConstraints(maxWidth: 640),
        builder: (context) => MarketFilterPanel(
          initialFilters: filters,
          clients: clients,
          categories: categories,
        ),
      );
      if (context.mounted && selected != null) onChanged(selected);
    }

    final label =
        '筛选：${marketSortLabel(filters.sort)}，'
        '${marketTimeRangeLabel(filters.timeRange)}';
    return compact
        ? IconButton(
            tooltip: label,
            onPressed: open,
            icon: const Icon(Icons.sort_rounded, size: 20),
          )
        : Tooltip(
            message: label,
            child: InkWell(
              onTap: open,
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 8,
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(Icons.sort_rounded, size: 20),
                    const SizedBox(width: 6),
                    Text(marketSortLabel(filters.sort)),
                  ],
                ),
              ),
            ),
          );
  }
}

class MarketFilterPanel extends StatefulWidget {
  const MarketFilterPanel({
    super.key,
    required this.initialFilters,
    required this.clients,
    this.categories,
  });

  final MarketFilters initialFilters;
  final GeneratedCoreProxyClients clients;
  final List<MarketCategoryInfo>? categories;

  @override
  State<MarketFilterPanel> createState() => _MarketFilterPanelState();
}

class _MarketFilterPanelState extends State<MarketFilterPanel> {
  late MarketFilters _draft = widget.initialFilters;
  List<MarketCategoryInfo>? _categories;
  bool _categoryError = false;

  @override
  void initState() {
    super.initState();
    _categories = widget.categories;
    if (_categories == null) _loadCategories();
  }

  Future<void> _loadCategories() async {
    setState(() => _categoryError = false);
    try {
      final manifest = await widget.clients.providersMarketStatsApiService
          .getManifest();
      if (mounted) setState(() => _categories = manifest.categories);
    } catch (_) {
      if (mounted) setState(() => _categoryError = true);
    }
  }

  void _set({
    MarketTimeRange? timeRange,
    String? categoryId,
    bool category = false,
  }) {
    setState(
      () => _draft = MarketFilters(
        sort: _draft.sort,
        timeRange: timeRange ?? _draft.timeRange,
        categoryId: category ? categoryId : _draft.categoryId,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final categories = _categories;
    Widget section(String title, List<Widget> choices) => Padding(
      padding: const EdgeInsets.only(bottom: 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            title,
            style: theme.textTheme.titleSmall?.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 12),
          Wrap(spacing: 8, runSpacing: 8, children: choices),
        ],
      ),
    );
    Widget choice(String label, bool selected, VoidCallback onTap) =>
        ChoiceChip(
          label: Text(label),
          selected: selected,
          onSelected: (_) => onTap(),
          showCheckmark: true,
          visualDensity: VisualDensity.standard,
        );
    return SafeArea(
      top: false,
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxHeight: MediaQuery.sizeOf(context).height * .85,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 0, 12, 16),
              child: Row(
                children: [
                  Icon(Icons.tune_rounded, color: colors.primary, size: 22),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Text(
                      '筛选市场',
                      style: theme.textTheme.titleLarge?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  IconButton(
                    tooltip: '关闭',
                    onPressed: () => Navigator.pop(context),
                    icon: const Icon(Icons.close_rounded),
                  ),
                ],
              ),
            ),
            Flexible(
              child: SingleChildScrollView(
                padding: const EdgeInsets.symmetric(horizontal: 24),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    section('排序方式', [
                      for (final sort in MarketSortOption.values)
                        choice(
                          marketSortLabel(sort),
                          _draft.sort == sort,
                          () => setState(() => _draft = _draft.withSort(sort)),
                        ),
                    ]),
                    section('时间范围', [
                      for (final range in MarketTimeRange.values)
                        choice(
                          marketTimeRangeLabel(range),
                          _draft.timeRange == range,
                          () => _set(timeRange: range),
                        ),
                    ]),
                    Padding(
                      padding: const EdgeInsets.only(bottom: 24),
                      child: Container(
                        padding: const EdgeInsets.all(12),
                        decoration: BoxDecoration(
                          color: colors.surfaceContainerHighest.withValues(
                            alpha: .55,
                          ),
                          borderRadius: BorderRadius.circular(12),
                        ),
                        child: Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Icon(
                              Icons.info_outline_rounded,
                              size: 16,
                              color: colors.onSurfaceVariant,
                            ),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(
                                _draft.sort == MarketSortOption.updated
                                    ? '按最近更新时间筛选内容。'
                                    : '热门榜默认最近 30 天首次发布的内容，按累计${_draft.sort == MarketSortOption.likes ? '点赞' : '下载'}排序。更新不会重置发布时间。',
                                style: theme.textTheme.bodySmall?.copyWith(
                                  color: colors.onSurfaceVariant,
                                  height: 1.5,
                                ),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                    section('内容分类', [
                      choice(
                        '全部分类',
                        _draft.categoryId == null,
                        () => _set(category: true),
                      ),
                      if (categories != null)
                        for (final item in categories)
                          choice(
                            marketCategoryText(
                              item,
                              languageCode: Localizations.localeOf(
                                context,
                              ).languageCode,
                            ).name,
                            _draft.categoryId == item.id,
                            () => _set(category: true, categoryId: item.id),
                          ),
                      if (_draft.categoryId != null &&
                          !(categories?.any(
                                (item) => item.id == _draft.categoryId,
                              ) ??
                              false))
                        choice(_draft.categoryId!, true, () {}),
                      if (_categoryError)
                        TextButton.icon(
                          onPressed: _loadCategories,
                          icon: const Icon(Icons.refresh_rounded, size: 18),
                          label: const Text('分类加载失败，重试'),
                        )
                      else if (categories == null)
                        const Padding(
                          padding: EdgeInsets.all(10),
                          child: SizedBox(
                            width: 18,
                            height: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          ),
                        ),
                    ]),
                  ],
                ),
              ),
            ),
            const Divider(height: 1),
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 16, 24, 20),
              child: Row(
                children: [
                  TextButton(
                    onPressed: () => setState(
                      () => _draft = MarketFilters.forSort(
                        widget.initialFilters.sort,
                      ),
                    ),
                    child: const Text('重置'),
                  ),
                  const SizedBox(width: 20),
                  Expanded(
                    child: FilledButton(
                      onPressed: () => Navigator.pop(context, _draft),
                      style: FilledButton.styleFrom(
                        minimumSize: const Size(0, 48),
                      ),
                      child: const Text('应用筛选'),
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
