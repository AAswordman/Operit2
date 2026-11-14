// ignore_for_file: file_names

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart';
import 'MarketStatsSupport.dart';

export 'MarketStatsSupport.dart';

enum MarketTimeRange { week, month, all }

String marketTimeRangeLabel(MarketTimeRange range) => switch (range) {
  MarketTimeRange.week => '最近一周',
  MarketTimeRange.month => '最近一个月',
  MarketTimeRange.all => '全部时间',
};

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

/// Immutable query shared by the filter panel, search and paginated lists.
class MarketFilters {
  const MarketFilters({
    this.sort = MarketSortOption.updated,
    this.timeRange = MarketTimeRange.all,
    this.categoryId,
  });

  factory MarketFilters.forSort(MarketSortOption sort, {String? categoryId}) =>
      MarketFilters(
        sort: sort,
        timeRange: sort == MarketSortOption.updated
            ? MarketTimeRange.all
            : MarketTimeRange.month,
        categoryId: categoryId,
      );

  final MarketSortOption sort;
  final MarketTimeRange timeRange;
  final String? categoryId;

  /// Selecting a different ranking starts with its useful default period.
  MarketFilters withSort(MarketSortOption value) => value == sort
      ? this
      : MarketFilters.forSort(value, categoryId: categoryId);

  bool matches(MarketEntrySummary entry, DateTime now) {
    if (categoryId != null && entry.categoryId != categoryId) return false;
    if (timeRange == MarketTimeRange.all) return true;
    // Rankings use the first publication, so updates cannot re-enter old
    // cumulative leaders into a recent-content ranking. Never fall back to an
    // update or latest-version timestamp for entries missing publication data.
    final date = sort == MarketSortOption.updated
        ? DateTime.tryParse(entry.updatedAt)
        : DateTime.tryParse(entry.publishedAt ?? '') ??
              DateTime.tryParse(entry.createdAt);
    if (date == null || date.isAfter(now)) return false;
    final cutoff = now.subtract(
      Duration(days: timeRange == MarketTimeRange.week ? 7 : 30),
    );
    return !date.isBefore(cutoff);
  }

  @override
  bool operator ==(Object other) =>
      other is MarketFilters &&
      sort == other.sort &&
      timeRange == other.timeRange &&
      categoryId == other.categoryId;

  @override
  int get hashCode => Object.hash(sort, timeRange, categoryId);
}

int marketPageCount(MarketListPage page) => page.pageSize <= 0
    ? page.page
    : ((page.total + page.pageSize - 1) ~/ page.pageSize).clamp(1, 1 << 31);

/// Keeps the server's ranking order and checks every page, including pages
/// containing no matches. Static feeds currently have no period-count endpoint.
Future<List<MarketEntrySummary>> loadMarketPeriodEntries(
  GeneratedProvidersMarketStatsApiServiceCoreProxy market, {
  required MarketFilters filters,
  String? type,
  DateTime? now,
  bool Function()? isCancelled,
}) async {
  final snapshot = now ?? DateTime.now();
  final entries = <String, MarketEntrySummary>{};
  var pageCount = 1;
  for (var page = 1; page <= pageCount; page++) {
    if (isCancelled?.call() ?? false) return const [];
    final result = await loadMarketPage(
      market,
      filters: filters,
      type: type,
      page: page,
    );
    pageCount = marketPageCount(result);
    for (final entry in result.items) {
      if (filters.matches(entry, snapshot)) entries[entry.id] = entry;
    }
  }
  return entries.values.toList(growable: false);
}

Future<MarketListPage> loadMarketPage(
  GeneratedProvidersMarketStatsApiServiceCoreProxy market, {
  required MarketFilters filters,
  String? type,
  required int page,
}) {
  final categoryId = filters.categoryId;
  final sort = marketSortId(filters.sort);
  if (type != null && categoryId != null) {
    return market.getTypeCategoryPage(
      type: type,
      categoryId: categoryId,
      sort: sort,
      page: page,
    );
  }
  if (type != null) {
    return market.getTypePage(type: type, sort: sort, page: page);
  }
  if (categoryId != null) {
    return market.getCategoryPage(
      categoryId: categoryId,
      sort: sort,
      page: page,
    );
  }
  return market.getListPage(sort: sort, page: page);
}
