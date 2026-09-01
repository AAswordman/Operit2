// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';

const double _dashboardMaxWidth = 1440;
const double _dashboardCompactPadding = 16;
const double _dashboardWidePadding = 24;
const double _dashboardSectionSpacing = 12;
const double _dashboardTrendSpacing = 16;
const int _dashboardRankingRowLimit = 6;

enum _UsageDateRange { all, last7Days, last30Days, custom }

enum _DateRangeEndpoint { start, end }

class _DateRangeSelection {
  const _DateRangeSelection({required this.range, this.customRange});

  final _UsageDateRange range;
  final DateTimeRange? customRange;
}

class UsageStatisticsDetailScreen extends StatefulWidget {
  const UsageStatisticsDetailScreen({
    super.key,
    GeneratedCoreProxyClients? clients,
  }) : clients =
           clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  static Future<void> open({
    required BuildContext context,
    required GeneratedCoreProxyClients clients,
  }) {
    return Navigator.of(context).push<void>(
      MaterialPageRoute<void>(
        builder: (context) => UsageStatisticsDetailScreen(clients: clients),
      ),
    );
  }

  @override
  State<UsageStatisticsDetailScreen> createState() =>
      _UsageStatisticsDetailScreenState();
}

class _UsageStatisticsDetailScreenState
    extends State<UsageStatisticsDetailScreen> {
  late Future<List<core_proxy.UsageRequestRecord>> _future;
  _UsageDateRange _selectedDateRange = _UsageDateRange.all;
  DateTimeRange? _customDateRange;

  @override
  void initState() {
    super.initState();
    _future = _load();
  }

  Future<List<core_proxy.UsageRequestRecord>> _load() {
    return widget.clients.repositoryUsageStatisticsStore.getAllRequestRecords();
  }

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  /// Opens a popup dialog and applies the selected date range.
  Future<void> _openDateRangeDialog(_UsageStatisticsViewData viewData) async {
    final dataStart = _dateOnly(viewData.firstRecordAt)!;
    final dataEnd = _dateOnly(viewData.lastRecordAt)!;
    final initialCustomRange = _customDateRange == null
        ? DateTimeRange(start: dataStart, end: dataEnd)
        : _customDateRange!;
    final selection = await showDialog<_DateRangeSelection>(
      context: context,
      builder: (context) => _DateRangePickerDialog(
        selectedRange: _selectedDateRange,
        customRange: initialCustomRange,
      ),
    );
    if (!mounted || selection == null) {
      return;
    }
    setState(() {
      _selectedDateRange = selection.range;
      _customDateRange = selection.customRange;
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.settingsDataDetailedStatsTitle),
        leading: IconButton(
          tooltip: l10n.close,
          onPressed: () => Navigator.of(context).pop(),
          icon: const Icon(Icons.close),
        ),
        actions: <Widget>[
          IconButton(
            tooltip: l10n.refresh,
            onPressed: _reload,
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      body: FutureBuilder<List<core_proxy.UsageRequestRecord>>(
        future: _future,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
          }
          final records = snapshot.data;
          if (records == null) {
            return const M3LoadingPane();
          }
          final allViewData = _UsageStatisticsViewData.fromRequestRecords(
            records,
            l10n,
          );
          if (allViewData.records.isEmpty) {
            return _UsageStatisticsEmptyState(
              title: l10n.settingsDataDetailedStatsEmpty,
              description: l10n.settingsDataDetailedStatsDescription,
            );
          }
          final viewData = allViewData.selectDateRange(
            _selectedDateRange,
            _customDateRange,
            l10n,
          );
          return LayoutBuilder(
            builder: (context, constraints) {
              final pagePadding = _dashboardPagePadding(constraints.maxWidth);
              final contentWidth = math.min(
                constraints.maxWidth - pagePadding * 2,
                _dashboardMaxWidth,
              );
              final metricWidth = _responsiveCardWidth(
                maxWidth: contentWidth,
                minWidth: 132,
                maxColumns: 6,
              );
              final trendWidth = _responsiveCardWidth(
                maxWidth: contentWidth,
                minWidth: 520,
                maxColumns: 2,
                spacing: _dashboardTrendSpacing,
              );
              final pieWidth = _responsiveCardWidth(
                maxWidth: contentWidth,
                minWidth: 320,
                maxColumns: 3,
              );
              return ListView(
                padding: EdgeInsets.fromLTRB(pagePadding, 12, pagePadding, 20),
                children: <Widget>[
                  Align(
                    alignment: Alignment.topCenter,
                    child: SizedBox(
                      width: contentWidth,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: <Widget>[
                          _UsageStatisticsHeaderBar(
                            title: l10n.settingsDataDetailedStatsTitle,
                            description:
                                l10n.settingsDataDetailedStatsDescription,
                            dateLabel: _usageDateRangeSummary(
                              l10n,
                              viewData,
                              _selectedDateRange,
                              _customDateRange,
                            ),
                            sourceLabel:
                                l10n.settingsDataDetailedStatsSourceLabel,
                            onDateRangePressed: () =>
                                _openDateRangeDialog(allViewData),
                          ),
                          const SizedBox(height: _dashboardSectionSpacing),
                          Wrap(
                            spacing: _dashboardSectionSpacing,
                            runSpacing: _dashboardSectionSpacing,
                            children: <Widget>[
                              SizedBox(
                                width: metricWidth,
                                child: _MetricCard(
                                  icon: Icons.bolt_outlined,
                                  label: l10n
                                      .settingsDataDetailedStatsTotalRequests,
                                  value: _formatFullInt(viewData.totalRequests),
                                  accent: Theme.of(context).colorScheme.primary,
                                ),
                              ),
                              SizedBox(
                                width: metricWidth,
                                child: _MetricCard(
                                  icon: Icons.south_west_outlined,
                                  label: l10n.settingsDataInputTokens,
                                  value: _formatFullInt(
                                    viewData.totalInputTokens,
                                  ),
                                  accent: Theme.of(
                                    context,
                                  ).colorScheme.secondary,
                                ),
                              ),
                              SizedBox(
                                width: metricWidth,
                                child: _MetricCard(
                                  icon: Icons.north_east_outlined,
                                  label: l10n.settingsDataOutputTokens,
                                  value: _formatFullInt(
                                    viewData.totalOutputTokens,
                                  ),
                                  accent: Theme.of(
                                    context,
                                  ).colorScheme.tertiary,
                                ),
                              ),
                              SizedBox(
                                width: metricWidth,
                                child: _MetricCard(
                                  icon: Icons.layers_outlined,
                                  label:
                                      l10n.settingsDataDetailedStatsCachedInput,
                                  value: _formatFullInt(
                                    viewData.totalCachedInputTokens,
                                  ),
                                  accent: Theme.of(context).colorScheme.primary,
                                ),
                              ),
                              SizedBox(
                                width: metricWidth,
                                child: _MetricCard(
                                  icon: Icons.calendar_view_week_outlined,
                                  label:
                                      l10n.settingsDataDetailedStatsActiveDays,
                                  value: _formatFullInt(viewData.activeDays),
                                  accent: Theme.of(
                                    context,
                                  ).colorScheme.secondary,
                                ),
                              ),
                              SizedBox(
                                width: metricWidth,
                                child: _MetricCard(
                                  icon: Icons.forum_outlined,
                                  label: l10n
                                      .settingsDataDetailedStatsFunctionModels,
                                  value: _formatFullInt(viewData.chatCount),
                                  accent: Theme.of(
                                    context,
                                  ).colorScheme.tertiary,
                                ),
                              ),
                              SizedBox(
                                width: metricWidth,
                                child: _MetricCard(
                                  icon: Icons.hub_outlined,
                                  label:
                                      l10n.settingsDataDetailedStatsProviders,
                                  value: _formatFullInt(viewData.providerCount),
                                  accent: Theme.of(context).colorScheme.primary,
                                ),
                              ),
                              SizedBox(
                                width: metricWidth,
                                child: _MetricCard(
                                  icon: Icons.view_in_ar_outlined,
                                  label: l10n.settingsDataDetailedStatsModels,
                                  value: _formatFullInt(viewData.modelCount),
                                  accent: Theme.of(
                                    context,
                                  ).colorScheme.secondary,
                                ),
                              ),
                              SizedBox(
                                width: metricWidth,
                                child: _MetricCard(
                                  icon: Icons.help_outline,
                                  label: l10n
                                      .settingsDataDetailedStatsUnknownTokenRecords,
                                  value: _formatFullInt(
                                    viewData.unknownTokenRecords,
                                  ),
                                  accent: Theme.of(context).colorScheme.error,
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: _dashboardSectionSpacing),
                          _UsageHeatmapCard(
                            title: l10n.settingsDataDetailedStatsHeatmapTitle,
                            subtitle:
                                l10n.settingsDataDetailedStatsHeatmapSubtitle,
                            emptyLabel:
                                l10n.settingsDataDetailedStatsNoDataInRange,
                            days: viewData.heatmapDays,
                          ),
                          const SizedBox(height: _dashboardSectionSpacing),
                          Wrap(
                            spacing: _dashboardTrendSpacing,
                            runSpacing: _dashboardSectionSpacing,
                            children: <Widget>[
                              SizedBox(
                                width: trendWidth,
                                child: _LineChartCard(
                                  title: l10n
                                      .settingsDataDetailedStatsDailyUsageTitle,
                                  subtitle: l10n
                                      .settingsDataDetailedStatsDailyUsageSubtitle,
                                  points: viewData.dailyPoints,
                                  xLabels: _buildDateAxisLabels(
                                    viewData.dailyPoints,
                                  ),
                                  series: <_ChartSeries>[
                                    _ChartSeries(
                                      label: l10n
                                          .settingsDataDetailedStatsRequestsSeries,
                                      color: Theme.of(
                                        context,
                                      ).colorScheme.primary,
                                      selector: (_DayUsagePoint point) =>
                                          point.requestCount,
                                    ),
                                  ],
                                ),
                              ),
                              SizedBox(
                                width: trendWidth,
                                child: _LineChartCard(
                                  title: l10n
                                      .settingsDataDetailedStatsInputOutputTitle,
                                  subtitle: l10n
                                      .settingsDataDetailedStatsInputOutputSubtitle,
                                  points: viewData.dailyPoints,
                                  xLabels: _buildDateAxisLabels(
                                    viewData.dailyPoints,
                                  ),
                                  series: <_ChartSeries>[
                                    _ChartSeries(
                                      label: l10n.settingsDataInputTokens,
                                      color: Theme.of(
                                        context,
                                      ).colorScheme.secondary,
                                      selector: (_DayUsagePoint point) =>
                                          point.inputTokens,
                                    ),
                                    _ChartSeries(
                                      label: l10n.settingsDataOutputTokens,
                                      color: Theme.of(
                                        context,
                                      ).colorScheme.tertiary,
                                      selector: (_DayUsagePoint point) =>
                                          point.outputTokens,
                                    ),
                                  ],
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: _dashboardSectionSpacing),
                          Wrap(
                            spacing: _dashboardSectionSpacing,
                            runSpacing: _dashboardSectionSpacing,
                            children: <Widget>[
                              SizedBox(
                                width: pieWidth,
                                child: _PieChartCard(
                                  title: l10n
                                      .settingsDataDetailedStatsProviderPieTitle,
                                  totalLabel:
                                      l10n.settingsDataDetailedStatsTotalTokens,
                                  slices: viewData.providerSlices,
                                ),
                              ),
                              SizedBox(
                                width: pieWidth,
                                child: _PieChartCard(
                                  title: l10n
                                      .settingsDataDetailedStatsModelPieTitle,
                                  totalLabel:
                                      l10n.settingsDataDetailedStatsTotalTokens,
                                  slices: viewData.modelSlices,
                                ),
                              ),
                              SizedBox(
                                width: pieWidth,
                                child: _PieChartCard(
                                  title: l10n
                                      .settingsDataDetailedStatsFunctionModelPieTitle,
                                  totalLabel:
                                      l10n.settingsDataDetailedStatsTotalTokens,
                                  slices: viewData.chatSlices,
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: _dashboardSectionSpacing),
                          Wrap(
                            spacing: _dashboardSectionSpacing,
                            runSpacing: _dashboardSectionSpacing,
                            children: <Widget>[
                              SizedBox(
                                width: contentWidth,
                                child: _TopListCard(
                                  title: l10n
                                      .settingsDataDetailedStatsTopFunctionModelsTitle,
                                  subtitle: l10n
                                      .settingsDataDetailedStatsTopFunctionModelsSubtitle,
                                  rows: viewData.topChatRows,
                                  emptyLabel:
                                      l10n.settingsDataDetailedStatsNoRankRows,
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ),
                ],
              );
            },
          );
        },
      ),
    );
  }
}

class _UsageStatisticsViewData {
  const _UsageStatisticsViewData({
    required this.records,
    required this.dailyPoints,
    required this.heatmapDays,
    required this.totalRequests,
    required this.totalInputTokens,
    required this.totalOutputTokens,
    required this.totalCachedInputTokens,
    required this.activeDays,
    required this.chatCount,
    required this.providerCount,
    required this.modelCount,
    required this.unknownTokenRecords,
    required this.firstRecordAt,
    required this.lastRecordAt,
    required this.providerSlices,
    required this.modelSlices,
    required this.chatSlices,
    required this.topChatRows,
  });

  final List<_UsageRecord> records;
  final List<_DayUsagePoint> dailyPoints;
  final List<_HeatmapDay> heatmapDays;
  final int totalRequests;
  final int totalInputTokens;
  final int totalOutputTokens;
  final int totalCachedInputTokens;
  final int activeDays;
  final int chatCount;
  final int providerCount;
  final int modelCount;
  final int unknownTokenRecords;
  final DateTime? firstRecordAt;
  final DateTime? lastRecordAt;
  final List<_PieSliceData> providerSlices;
  final List<_PieSliceData> modelSlices;
  final List<_PieSliceData> chatSlices;
  final List<_TopListRowData> topChatRows;

  /// Builds view data for the selected date range.
  _UsageStatisticsViewData selectDateRange(
    _UsageDateRange range,
    DateTimeRange? customRange,
    AppLocalizations l10n,
  ) {
    if (range == _UsageDateRange.all) {
      return this;
    }
    assert(range != _UsageDateRange.custom || customRange != null);
    final endDay = _dateOnly(lastRecordAt);
    final selectedRecords = switch (range) {
      _UsageDateRange.all => records,
      _UsageDateRange.last7Days =>
        records
            .where((record) => _recordIsInsideTrailingDays(record, endDay, 7))
            .toList(growable: false),
      _UsageDateRange.last30Days =>
        records
            .where((record) => _recordIsInsideTrailingDays(record, endDay, 30))
            .toList(growable: false),
      _UsageDateRange.custom =>
        records
            .where(
              (record) => _recordIsInsideDateSpan(
                record,
                customRange!.start,
                customRange.end,
              ),
            )
            .toList(growable: false),
    };
    return _UsageStatisticsViewData.fromUsageRecords(selectedRecords, l10n);
  }

  /// Normalizes detailed request records for dashboard aggregation.
  static _UsageStatisticsViewData fromRequestRecords(
    List<core_proxy.UsageRequestRecord> requestRecords,
    AppLocalizations l10n,
  ) {
    final records = requestRecords
        .map((record) {
          final providerLabel = _providerLabel(record.provider, l10n);
          final modelLabel = _modelLabel(record.modelName, l10n);
          final functionTitle = _functionTypeLabel(record.functionType, l10n);
          return _UsageRecord(
            chatId: _functionModelKey(
              record.functionType,
              providerLabel,
              modelLabel,
            ),
            chatTitle: _functionModelTitle(
              functionTitle,
              providerLabel,
              modelLabel,
            ),
            occurredAt: DateTime.fromMillisecondsSinceEpoch(
              record.createdAtMs,
            ).toLocal(),
            providerLabel: providerLabel,
            modelLabel: modelLabel,
            requestCount: 1,
            hasUnknownTokens: false,
            inputTokens: record.inputTokens,
            outputTokens: record.outputTokens,
            cachedInputTokens: record.cachedInputTokens,
          );
        })
        .toList(growable: false);
    return _UsageStatisticsViewData.fromUsageRecords(records, l10n);
  }

  /// Builds dashboard aggregates from normalized usage records.
  static _UsageStatisticsViewData fromUsageRecords(
    List<_UsageRecord> sourceRecords,
    AppLocalizations l10n,
  ) {
    final records = List<_UsageRecord>.of(sourceRecords);
    if (records.isEmpty) {
      return _UsageStatisticsViewData(
        records: const <_UsageRecord>[],
        dailyPoints: const <_DayUsagePoint>[],
        heatmapDays: const <_HeatmapDay>[],
        totalRequests: 0,
        totalInputTokens: 0,
        totalOutputTokens: 0,
        totalCachedInputTokens: 0,
        activeDays: 0,
        chatCount: 0,
        providerCount: 0,
        modelCount: 0,
        unknownTokenRecords: 0,
        firstRecordAt: null,
        lastRecordAt: null,
        providerSlices: const <_PieSliceData>[],
        modelSlices: const <_PieSliceData>[],
        chatSlices: const <_PieSliceData>[],
        topChatRows: const <_TopListRowData>[],
      );
    }

    records.sort((left, right) {
      final leftAt = left.occurredAt;
      final rightAt = right.occurredAt;
      if (leftAt == null && rightAt == null) return 0;
      if (leftAt == null) return 1;
      if (rightAt == null) return -1;
      return leftAt.compareTo(rightAt);
    });
    final dailyAccumulators = <DateTime, _DayUsageAccumulator>{};
    final providerAccumulators = <String, _AggregateAccumulator>{};
    final modelAccumulators = <String, _AggregateAccumulator>{};
    final chatAccumulators = <String, _AggregateAccumulator>{};

    var totalInputTokens = 0;
    var totalOutputTokens = 0;
    var totalCachedInputTokens = 0;

    for (final record in records) {
      totalInputTokens += record.inputTokens;
      totalOutputTokens += record.outputTokens;
      totalCachedInputTokens += record.cachedInputTokens;

      final occurredAt = record.occurredAt;
      if (occurredAt != null) {
        final dayKey = DateTime(
          occurredAt.year,
          occurredAt.month,
          occurredAt.day,
        );
        dailyAccumulators
            .putIfAbsent(dayKey, _DayUsageAccumulator.new)
            .add(record);
      }
      providerAccumulators
          .putIfAbsent(record.providerLabel, _AggregateAccumulator.new)
          .add(record);
      modelAccumulators
          .putIfAbsent(record.modelLabel, _AggregateAccumulator.new)
          .add(record);
      chatAccumulators
          .putIfAbsent(record.chatId, _AggregateAccumulator.new)
          .add(record);
    }

    final dailyPoints = dailyAccumulators.entries.toList()
      ..sort((left, right) => left.key.compareTo(right.key));

    final providerPalette = _buildPalette(constraintsSeed: 1);
    final modelPalette = _buildPalette(constraintsSeed: 2);
    final chatPalette = _buildPalette(constraintsSeed: 3);

    final providerSlices = _buildPieSlices(
      rows: providerAccumulators.entries
          .map(
            (entry) => _AggregateRow(
              label: entry.key,
              totalTokens: entry.value.totalTokens,
            ),
          )
          .toList(growable: false),
      otherLabel: l10n.settingsDataDetailedStatsOther,
      palette: providerPalette,
    );
    final modelSlices = _buildPieSlices(
      rows: modelAccumulators.entries
          .map(
            (entry) => _AggregateRow(
              label: entry.key,
              totalTokens: entry.value.totalTokens,
            ),
          )
          .toList(growable: false),
      otherLabel: l10n.settingsDataDetailedStatsOther,
      palette: modelPalette,
    );
    final chatSlices = _buildPieSlices(
      rows: chatAccumulators.entries
          .map(
            (entry) => _AggregateRow(
              label: entry.value.chatTitle,
              totalTokens: entry.value.totalTokens,
            ),
          )
          .toList(growable: false),
      otherLabel: l10n.settingsDataDetailedStatsOther,
      palette: chatPalette,
    );

    final topChatRows = chatAccumulators.values.toList()
      ..sort(
        (left, right) => right.totalTokens.compareTo(left.totalTokens) != 0
            ? right.totalTokens.compareTo(left.totalTokens)
            : right.chatTitle.compareTo(left.chatTitle),
      );

    final mappedDailyPoints = dailyPoints
        .map(
          (entry) => _DayUsagePoint(
            day: entry.key,
            requestCount: entry.value.requestCount,
            inputTokens: entry.value.inputTokens,
            outputTokens: entry.value.outputTokens,
          ),
        )
        .toList(growable: false);

    return _UsageStatisticsViewData(
      records: records,
      dailyPoints: mappedDailyPoints,
      heatmapDays: _buildHeatmapDays(mappedDailyPoints),
      totalRequests: records.fold<int>(
        0,
        (sum, record) => sum + record.requestCount,
      ),
      totalInputTokens: totalInputTokens,
      totalOutputTokens: totalOutputTokens,
      totalCachedInputTokens: totalCachedInputTokens,
      activeDays: dailyAccumulators.length,
      chatCount: chatAccumulators.length,
      providerCount: providerAccumulators.length,
      modelCount: modelAccumulators.length,
      unknownTokenRecords: records
          .where((record) => record.hasUnknownTokens)
          .length,
      firstRecordAt: _firstDatedRecord(records),
      lastRecordAt: _lastDatedRecord(records),
      providerSlices: providerSlices,
      modelSlices: modelSlices,
      chatSlices: chatSlices,
      topChatRows: topChatRows
          .take(_dashboardRankingRowLimit)
          .map(
            (aggregate) => _TopListRowData(
              title: aggregate.chatTitle,
              subtitle: l10n.settingsDataDetailedStatsRequestCountSummary(
                aggregate.requestCount,
              ),
              trailing: _formatCompactInt(aggregate.totalTokens),
            ),
          )
          .toList(growable: false),
    );
  }
}

class _UsageRecord {
  const _UsageRecord({
    required this.chatId,
    required this.chatTitle,
    required this.occurredAt,
    required this.providerLabel,
    required this.modelLabel,
    required this.requestCount,
    required this.hasUnknownTokens,
    required this.inputTokens,
    required this.outputTokens,
    required this.cachedInputTokens,
  });

  final String chatId;
  final String chatTitle;
  final DateTime? occurredAt;
  final String providerLabel;
  final String modelLabel;
  final int requestCount;
  final bool hasUnknownTokens;
  final int inputTokens;
  final int outputTokens;
  final int cachedInputTokens;

  int get totalTokens => inputTokens + outputTokens;
}

class _DayUsageAccumulator {
  int requestCount = 0;
  int inputTokens = 0;
  int outputTokens = 0;

  void add(_UsageRecord record) {
    requestCount += record.requestCount;
    inputTokens += record.inputTokens;
    outputTokens += record.outputTokens;
  }
}

class _AggregateAccumulator {
  int totalTokens = 0;
  int requestCount = 0;
  String chatTitle = '';

  void add(_UsageRecord record) {
    totalTokens += record.totalTokens;
    requestCount += record.requestCount;
    chatTitle = record.chatTitle;
  }
}

class _AggregateRow {
  const _AggregateRow({required this.label, required this.totalTokens});

  final String label;
  final int totalTokens;
}

class _DayUsagePoint {
  const _DayUsagePoint({
    required this.day,
    required this.requestCount,
    required this.inputTokens,
    required this.outputTokens,
  });

  final DateTime day;
  final int requestCount;
  final int inputTokens;
  final int outputTokens;
}

class _HeatmapDay {
  const _HeatmapDay({
    required this.day,
    required this.requestCount,
    required this.totalTokens,
  });

  final DateTime day;
  final int requestCount;
  final int totalTokens;
}

class _ChartSeries {
  const _ChartSeries({
    required this.label,
    required this.color,
    required this.selector,
  });

  final String label;
  final Color color;
  final int Function(_DayUsagePoint point) selector;
}

class _PieSliceData {
  const _PieSliceData({
    required this.label,
    required this.value,
    required this.color,
  });

  final String label;
  final int value;
  final Color color;
}

class _TopListRowData {
  const _TopListRowData({
    required this.title,
    required this.subtitle,
    required this.trailing,
  });

  final String title;
  final String subtitle;
  final String trailing;
}

class _UsageStatisticsEmptyState extends StatelessWidget {
  const _UsageStatisticsEmptyState({
    required this.title,
    required this.description,
  });

  final String title;
  final String description;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: OperitGlassSurface(
            color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
            borderRadius: BorderRadius.circular(20),
            border: Border.all(
              color: colorScheme.outlineVariant.withValues(alpha: 0.18),
            ),
            material: true,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 28),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Icon(
                    Icons.query_stats_outlined,
                    size: 42,
                    color: colorScheme.primary,
                  ),
                  const SizedBox(height: 14),
                  Text(
                    title,
                    style: Theme.of(context).textTheme.titleLarge?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                    textAlign: TextAlign.center,
                  ),
                  const SizedBox(height: 8),
                  Text(
                    description,
                    style: TextStyle(color: colorScheme.onSurfaceVariant),
                    textAlign: TextAlign.center,
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

class _UsageStatisticsHeaderBar extends StatelessWidget {
  const _UsageStatisticsHeaderBar({
    required this.title,
    required this.description,
    required this.dateLabel,
    required this.sourceLabel,
    required this.onDateRangePressed,
  });

  final String title;
  final String description;
  final String dateLabel;
  final String sourceLabel;
  final VoidCallback onDateRangePressed;

  /// Builds the dashboard header with a popup-backed date pill.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return OperitGlassSurface(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.28),
      borderRadius: BorderRadius.circular(14),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      material: true,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 10, 12, 10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: textTheme.labelLarge?.copyWith(
                fontWeight: FontWeight.w700,
              ),
            ),
            const SizedBox(height: 4),
            Text(
              description,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 8),
            Wrap(
              spacing: 8,
              runSpacing: 6,
              children: <Widget>[
                _UsageStatisticsMetaPill(
                  icon: Icons.date_range_outlined,
                  label: dateLabel,
                  onTap: onDateRangePressed,
                ),
                _UsageStatisticsMetaPill(
                  icon: Icons.storage_outlined,
                  label: sourceLabel,
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _DateRangePickerDialog extends StatefulWidget {
  const _DateRangePickerDialog({
    required this.selectedRange,
    required this.customRange,
  });

  final _UsageDateRange selectedRange;
  final DateTimeRange customRange;

  /// Creates state for the popup date range selector.
  @override
  State<_DateRangePickerDialog> createState() => _DateRangePickerDialogState();
}

class _DateRangePickerDialogState extends State<_DateRangePickerDialog> {
  late _UsageDateRange _range;
  late DateTimeRange _customRange;

  /// Initializes the dialog with the active dashboard range.
  @override
  void initState() {
    super.initState();
    _range = widget.selectedRange;
    _customRange = widget.customRange;
  }

  /// Selects one of the popup range modes.
  void _selectRange(_UsageDateRange range) {
    setState(() {
      _range = range;
    });
  }

  /// Opens a date picker for one custom range endpoint.
  Future<void> _pickEndpoint(_DateRangeEndpoint endpoint) async {
    final pickedDate = await showDatePicker(
      context: context,
      initialDate: endpoint == _DateRangeEndpoint.start
          ? _customRange.start
          : _customRange.end,
      firstDate: DateTime(1970),
      lastDate: DateTime(2100, 12, 31),
    );
    if (pickedDate == null) {
      return;
    }
    final pickedDay = _dateOnly(pickedDate)!;
    setState(() {
      var startDay = _customRange.start;
      var endDay = _customRange.end;
      switch (endpoint) {
        case _DateRangeEndpoint.start:
          startDay = pickedDay;
          if (endDay.isBefore(startDay)) {
            endDay = startDay;
          }
        case _DateRangeEndpoint.end:
          endDay = pickedDay;
          if (startDay.isAfter(endDay)) {
            startDay = endDay;
          }
      }
      _range = _UsageDateRange.custom;
      _customRange = DateTimeRange(start: startDay, end: endDay);
    });
  }

  /// Closes the dialog with the selected date range.
  void _submit() {
    Navigator.of(context).pop(
      _DateRangeSelection(
        range: _range,
        customRange: _dateOnlyRange(_customRange),
      ),
    );
  }

  /// Builds the popup date range selector.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final materialL10n = MaterialLocalizations.of(context);
    final colorScheme = Theme.of(context).colorScheme;
    return AlertDialog(
      title: Text(l10n.settingsDataDetailedStatsCustomRangePickerTitle),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 420),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            for (final range in _UsageDateRange.values) ...<Widget>[
              ListTile(
                dense: true,
                selected: _range == range,
                selectedTileColor: colorScheme.primaryContainer.withValues(
                  alpha: 0.42,
                ),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(12),
                ),
                title: Text(_usageDateRangeLabel(l10n, range)),
                trailing: _range == range
                    ? Icon(Icons.check_circle, color: colorScheme.primary)
                    : null,
                onTap: () => _selectRange(range),
              ),
              if (range != _UsageDateRange.custom) const SizedBox(height: 4),
            ],
            AnimatedSwitcher(
              duration: const Duration(milliseconds: 160),
              child: _range == _UsageDateRange.custom
                  ? Padding(
                      key: const ValueKey<String>('custom-range-fields'),
                      padding: const EdgeInsets.only(top: 12),
                      child: Column(
                        children: <Widget>[
                          SizedBox(
                            width: double.infinity,
                            child: OutlinedButton.icon(
                              icon: const Icon(Icons.today_outlined),
                              label: Text(
                                '${materialL10n.dateRangeStartLabel}: ${_formatDate(_customRange.start)}',
                              ),
                              onPressed: () =>
                                  _pickEndpoint(_DateRangeEndpoint.start),
                            ),
                          ),
                          const SizedBox(height: 8),
                          SizedBox(
                            width: double.infinity,
                            child: OutlinedButton.icon(
                              icon: const Icon(Icons.event_available_outlined),
                              label: Text(
                                '${materialL10n.dateRangeEndLabel}: ${_formatDate(_customRange.end)}',
                              ),
                              onPressed: () =>
                                  _pickEndpoint(_DateRangeEndpoint.end),
                            ),
                          ),
                        ],
                      ),
                    )
                  : const SizedBox.shrink(),
            ),
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _submit, child: Text(l10n.save)),
      ],
    );
  }
}

class _UsageStatisticsMetaPill extends StatelessWidget {
  const _UsageStatisticsMetaPill({
    required this.icon,
    required this.label,
    this.onTap,
  });

  final IconData icon;
  final String label;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      type: MaterialType.transparency,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(999),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 5),
          decoration: BoxDecoration(
            color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.42),
            borderRadius: BorderRadius.circular(999),
            border: Border.all(
              color: colorScheme.outlineVariant.withValues(alpha: 0.18),
            ),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(icon, size: 15, color: colorScheme.primary),
              const SizedBox(width: 6),
              Flexible(
                child: Text(
                  label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.labelSmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
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

class _UsageStatisticsSectionCard extends StatelessWidget {
  const _UsageStatisticsSectionCard({
    required this.title,
    required this.subtitle,
    required this.child,
  });

  final String title;
  final String subtitle;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return OperitGlassSurface(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.32),
      borderRadius: BorderRadius.circular(14),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      material: true,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 10, 12, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: textTheme.labelLarge?.copyWith(
                fontWeight: FontWeight.w700,
              ),
            ),
            const SizedBox(height: 4),
            Text(
              subtitle,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 10),
            child,
          ],
        ),
      ),
    );
  }
}

class _MetricCard extends StatelessWidget {
  const _MetricCard({
    required this.icon,
    required this.label,
    required this.value,
    required this.accent,
  });

  final IconData icon;
  final String label;
  final String value;
  final Color accent;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.32),
      borderRadius: BorderRadius.circular(14),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      material: true,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(10, 8, 10, 8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Icon(icon, size: 18, color: accent),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(color: colorScheme.onSurfaceVariant),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 6),
            SizedBox(
              width: double.infinity,
              child: LayoutBuilder(
                builder: (context, constraints) {
                  return FittedBox(
                    fit: BoxFit.scaleDown,
                    alignment: Alignment.centerLeft,
                    child: ConstrainedBox(
                      constraints: BoxConstraints(
                        maxWidth: constraints.maxWidth,
                      ),
                      child: Text(
                        value,
                        maxLines: 1,
                        style: Theme.of(context).textTheme.titleLarge?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
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

class _UsageHeatmapCard extends StatefulWidget {
  const _UsageHeatmapCard({
    required this.title,
    required this.subtitle,
    required this.emptyLabel,
    required this.days,
  });

  final String title;
  final String subtitle;
  final String emptyLabel;
  final List<_HeatmapDay> days;

  /// Creates state for the interactive heatmap.
  @override
  State<_UsageHeatmapCard> createState() => _UsageHeatmapCardState();
}

class _UsageHeatmapCardState extends State<_UsageHeatmapCard> {
  _HeatmapDay? _selectedDay;

  /// Synchronizes the selected cell with the active range data.
  @override
  void didUpdateWidget(covariant _UsageHeatmapCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    final selectedDay = _selectedDay;
    if (selectedDay == null) {
      return;
    }
    _HeatmapDay? updatedDay;
    for (final day in widget.days) {
      if (_sameCalendarDay(day.day, selectedDay.day)) {
        updatedDay = day;
        break;
      }
    }
    _selectedDay = updatedDay;
  }

  /// Selects the heatmap cell at a tapped canvas position.
  void _handleTapDown(TapDownDetails details, double cellSize, double cellGap) {
    final tappedDay = _heatmapDayAtPosition(
      widget.days,
      details.localPosition,
      cellSize,
      cellGap,
    );
    if (tappedDay == null) {
      return;
    }
    setState(() {
      final selectedDay = _selectedDay;
      _selectedDay =
          selectedDay != null &&
              _sameCalendarDay(selectedDay.day, tappedDay.day)
          ? null
          : tappedDay;
    });
  }

  /// Builds the detail text shown below the heatmap grid.
  String _detailLabel(AppLocalizations l10n) {
    final selectedDay = _selectedDay;
    if (selectedDay == null) {
      return l10n.settingsDataDetailedStatsHeatmapTapHint;
    }
    return l10n.settingsDataDetailedStatsHeatmapDayDetail(
      _formatDate(selectedDay.day),
      _formatCompactInt(selectedDay.totalTokens),
      _formatFullInt(selectedDay.requestCount),
    );
  }

  /// Builds the interactive usage heatmap card.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context)!;
    return _UsageStatisticsSectionCard(
      title: widget.title,
      subtitle: widget.subtitle,
      child: widget.days.isEmpty
          ? _TopListEmptyRow(label: widget.emptyLabel)
          : LayoutBuilder(
              builder: (context, constraints) {
                const cellSize = 12.0;
                const cellGap = 4.0;
                final weekCount = _heatmapWeekCount(widget.days);
                final heatmapWidth =
                    weekCount * cellSize + math.max(0, weekCount - 1) * cellGap;
                final heatmapHeight =
                    DateTime.daysPerWeek * cellSize +
                    (DateTime.daysPerWeek - 1) * cellGap;
                final paintWidth = math.max(heatmapWidth, constraints.maxWidth);
                final maxTokens = widget.days.fold<int>(
                  0,
                  (currentMax, day) => math.max(currentMax, day.totalTokens),
                );
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    SingleChildScrollView(
                      scrollDirection: Axis.horizontal,
                      child: SizedBox(
                        width: paintWidth,
                        height: heatmapHeight,
                        child: GestureDetector(
                          behavior: HitTestBehavior.opaque,
                          onTapDown: (details) =>
                              _handleTapDown(details, cellSize, cellGap),
                          child: MouseRegion(
                            cursor: SystemMouseCursors.click,
                            child: CustomPaint(
                              painter: _UsageHeatmapPainter(
                                days: widget.days,
                                cellSize: cellSize,
                                cellGap: cellGap,
                                colorScheme: colorScheme,
                                maxTokens: maxTokens,
                                selectedDay: _selectedDay,
                              ),
                              child: const SizedBox.expand(),
                            ),
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(height: 10),
                    _UsageHeatmapLegend(
                      detailLabel: _detailLabel(l10n),
                      lowLabel: l10n.settingsDataDetailedStatsHeatmapLow,
                      highLabel: l10n.settingsDataDetailedStatsHeatmapHigh,
                    ),
                  ],
                );
              },
            ),
    );
  }
}

class _UsageHeatmapLegend extends StatelessWidget {
  const _UsageHeatmapLegend({
    required this.detailLabel,
    required this.lowLabel,
    required this.highLabel,
  });

  final String detailLabel;
  final String lowLabel;
  final String highLabel;

  /// Builds the heatmap footer with details and intensity legend.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final labelStyle = Theme.of(
      context,
    ).textTheme.labelSmall?.copyWith(color: colorScheme.onSurfaceVariant);
    return Wrap(
      spacing: 10,
      runSpacing: 8,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: <Widget>[
        ConstrainedBox(
          constraints: const BoxConstraints(minWidth: 160),
          child: Text(
            detailLabel,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: labelStyle,
          ),
        ),
        Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Text(lowLabel, style: labelStyle),
            const SizedBox(width: 8),
            for (var index = 0; index < 5; index += 1) ...<Widget>[
              Container(
                width: 12,
                height: 12,
                decoration: BoxDecoration(
                  color: _heatmapCellColor(index / 4, colorScheme),
                  borderRadius: BorderRadius.circular(3),
                ),
              ),
              if (index < 4) const SizedBox(width: 4),
            ],
            const SizedBox(width: 8),
            Text(highLabel, style: labelStyle),
          ],
        ),
      ],
    );
  }
}

class _LineChartCard extends StatelessWidget {
  const _LineChartCard({
    required this.title,
    required this.subtitle,
    required this.points,
    required this.series,
    required this.xLabels,
  });

  final String title;
  final String subtitle;
  final List<_DayUsagePoint> points;
  final List<_ChartSeries> series;
  final List<String> xLabels;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return _UsageStatisticsSectionCard(
      title: title,
      subtitle: subtitle,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final chartHeight = (constraints.maxWidth * 0.54)
              .clamp(188.0, 260.0)
              .toDouble();
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: series
                    .map(
                      (item) => _LegendChip(
                        color: item.color,
                        label: item.label,
                        value: _formatCompactInt(
                          points.fold<int>(
                            0,
                            (sum, point) => sum + item.selector(point),
                          ),
                        ),
                      ),
                    )
                    .toList(growable: false),
              ),
              const SizedBox(height: 12),
              SizedBox(
                height: chartHeight,
                child: CustomPaint(
                  painter: _LineChartPainter(
                    points: points,
                    series: series,
                    colorScheme: colorScheme,
                  ),
                  child: const SizedBox.expand(),
                ),
              ),
              const SizedBox(height: 8),
              Row(
                children: xLabels
                    .map(
                      (label) => Expanded(
                        child: Text(
                          label,
                          textAlign: label == xLabels.first
                              ? TextAlign.start
                              : label == xLabels.last
                              ? TextAlign.end
                              : TextAlign.center,
                          style: TextStyle(color: colorScheme.onSurfaceVariant),
                        ),
                      ),
                    )
                    .toList(growable: false),
              ),
            ],
          );
        },
      ),
    );
  }
}

class _PieChartCard extends StatelessWidget {
  const _PieChartCard({
    required this.title,
    required this.totalLabel,
    required this.slices,
  });

  final String title;
  final String totalLabel;
  final List<_PieSliceData> slices;

  @override
  Widget build(BuildContext context) {
    final totalValue = slices.fold<int>(0, (sum, slice) => sum + slice.value);
    return _UsageStatisticsSectionCard(
      title: title,
      subtitle: totalLabel,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final horizontal = constraints.maxWidth >= 430;
          final chartSize = horizontal
              ? math.min<double>(176.0, constraints.maxWidth * 0.38)
              : (constraints.maxWidth * 0.62).clamp(156.0, 190.0).toDouble();
          final chart = SizedBox(
            width: chartSize,
            height: chartSize,
            child: Stack(
              alignment: Alignment.center,
              children: <Widget>[
                CustomPaint(
                  painter: _PieChartPainter(slices: slices),
                  child: const SizedBox.expand(),
                ),
                Column(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    FittedBox(
                      fit: BoxFit.scaleDown,
                      child: Text(
                        _formatCompactInt(totalValue),
                        maxLines: 1,
                        style: Theme.of(context).textTheme.titleLarge?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      totalLabel,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ],
            ),
          );
          final legend = Column(
            children: slices
                .map(
                  (slice) => Padding(
                    padding: const EdgeInsets.symmetric(vertical: 4),
                    child: _PieLegendRow(
                      color: slice.color,
                      label: slice.label,
                      percent: totalValue == 0
                          ? '0%'
                          : _formatPercent(slice.value / totalValue),
                      value: _formatCompactInt(slice.value),
                    ),
                  ),
                )
                .toList(growable: false),
          );
          if (horizontal) {
            return Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                chart,
                const SizedBox(width: 16),
                Expanded(child: legend),
              ],
            );
          }
          return Column(
            children: <Widget>[chart, const SizedBox(height: 12), legend],
          );
        },
      ),
    );
  }
}

class _TopListCard extends StatelessWidget {
  const _TopListCard({
    required this.title,
    required this.subtitle,
    required this.rows,
    required this.emptyLabel,
  });

  final String title;
  final String subtitle;
  final List<_TopListRowData> rows;
  final String emptyLabel;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return _UsageStatisticsSectionCard(
      title: title,
      subtitle: subtitle,
      child: rows.isEmpty
          ? _TopListEmptyRow(label: emptyLabel)
          : Column(
              children: rows
                  .asMap()
                  .entries
                  .map(
                    (entry) => Column(
                      children: <Widget>[
                        if (entry.key > 0)
                          Divider(
                            height: 1,
                            color: colorScheme.outlineVariant.withValues(
                              alpha: 0.18,
                            ),
                          ),
                        _TopListRow(rank: entry.key + 1, row: entry.value),
                      ],
                    ),
                  )
                  .toList(growable: false),
            ),
    );
  }
}

class _TopListRow extends StatelessWidget {
  const _TopListRow({required this.rank, required this.row});

  final int rank;
  final _TopListRowData row;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return ConstrainedBox(
      constraints: const BoxConstraints(minHeight: 46),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 7),
        child: Row(
          children: <Widget>[
            SizedBox(
              width: 30,
              child: Text(
                '#$rank',
                textAlign: TextAlign.left,
                style: textTheme.labelSmall?.copyWith(
                  color: colorScheme.primary,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Text(
                    row.title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: textTheme.bodySmall?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    row.subtitle,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: textTheme.labelSmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            SizedBox(
              width: 72,
              child: Text(
                row.trailing,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                textAlign: TextAlign.right,
                style: textTheme.bodySmall?.copyWith(
                  color: colorScheme.primary,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _TopListEmptyRow extends StatelessWidget {
  const _TopListEmptyRow({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return ConstrainedBox(
      constraints: const BoxConstraints(minHeight: 46),
      child: Align(
        alignment: Alignment.centerLeft,
        child: Text(
          label,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: Theme.of(
            context,
          ).textTheme.bodySmall?.copyWith(color: colorScheme.onSurfaceVariant),
        ),
      ),
    );
  }
}

class _LegendChip extends StatelessWidget {
  const _LegendChip({
    required this.color,
    required this.label,
    required this.value,
  });

  final Color color;
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.48),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.2),
        ),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Container(
            width: 10,
            height: 10,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          ),
          const SizedBox(width: 8),
          Text('$label · $value'),
        ],
      ),
    );
  }
}

class _PieLegendRow extends StatelessWidget {
  const _PieLegendRow({
    required this.color,
    required this.label,
    required this.percent,
    required this.value,
  });

  final Color color;
  final String label;
  final String percent;
  final String value;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Row(
      children: <Widget>[
        Container(
          width: 11,
          height: 11,
          decoration: BoxDecoration(color: color, shape: BoxShape.circle),
        ),
        const SizedBox(width: 10),
        Expanded(
          child: Text(label, maxLines: 1, overflow: TextOverflow.ellipsis),
        ),
        const SizedBox(width: 10),
        Text(percent, style: TextStyle(color: colorScheme.onSurfaceVariant)),
        const SizedBox(width: 10),
        Text(value, style: const TextStyle(fontWeight: FontWeight.w700)),
      ],
    );
  }
}

class _LineChartPainter extends CustomPainter {
  const _LineChartPainter({
    required this.points,
    required this.series,
    required this.colorScheme,
  });

  final List<_DayUsagePoint> points;
  final List<_ChartSeries> series;
  final ColorScheme colorScheme;

  @override
  void paint(Canvas canvas, Size size) {
    const leftPadding = 46.0;
    const topPadding = 8.0;
    const rightPadding = 12.0;
    const bottomPadding = 20.0;
    final chartRect = Rect.fromLTWH(
      leftPadding,
      topPadding,
      size.width - leftPadding - rightPadding,
      size.height - topPadding - bottomPadding,
    );
    if (chartRect.width <= 0 || chartRect.height <= 0 || points.isEmpty) {
      return;
    }

    final values = series
        .expand((item) => points.map(item.selector))
        .toList(growable: false);
    final maxValue = math.max(
      1,
      values.fold<int>(0, (maxValue, value) => math.max(maxValue, value)),
    );

    final gridPaint = Paint()
      ..color = colorScheme.outlineVariant.withValues(alpha: 0.22)
      ..strokeWidth = 1;

    for (var index = 0; index < 5; index++) {
      final ratio = index / 4;
      final y = chartRect.bottom - chartRect.height * ratio;
      canvas.drawLine(
        Offset(chartRect.left, y),
        Offset(chartRect.right, y),
        gridPaint,
      );
      final value = (maxValue * ratio).round();
      _paintText(
        canvas,
        text: _formatCompactInt(value),
        offset: Offset(0, y - 8),
        style: TextStyle(color: colorScheme.onSurfaceVariant, fontSize: 11),
        maxWidth: leftPadding - 8,
        textAlign: TextAlign.right,
      );
    }

    canvas.drawLine(
      Offset(chartRect.left, chartRect.bottom),
      Offset(chartRect.right, chartRect.bottom),
      Paint()
        ..color = colorScheme.outlineVariant.withValues(alpha: 0.28)
        ..strokeWidth = 1.2,
    );

    for (final entry in series) {
      final path = Path();
      final dotPaint = Paint()
        ..color = entry.color
        ..style = PaintingStyle.fill;
      final linePaint = Paint()
        ..color = entry.color
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2.5
        ..strokeCap = StrokeCap.round
        ..strokeJoin = StrokeJoin.round;

      for (var index = 0; index < points.length; index++) {
        final point = points[index];
        final x = points.length == 1
            ? chartRect.center.dx
            : chartRect.left + chartRect.width * index / (points.length - 1);
        final y =
            chartRect.bottom -
            chartRect.height * entry.selector(point) / maxValue;
        final offset = Offset(x, y);
        if (index == 0) {
          path.moveTo(offset.dx, offset.dy);
        } else {
          path.lineTo(offset.dx, offset.dy);
        }
        canvas.drawCircle(offset, 3.4, dotPaint);
      }
      canvas.drawPath(path, linePaint);
    }
  }

  @override
  bool shouldRepaint(covariant _LineChartPainter oldDelegate) {
    return oldDelegate.points != points ||
        oldDelegate.series != series ||
        oldDelegate.colorScheme != colorScheme;
  }
}

class _UsageHeatmapPainter extends CustomPainter {
  const _UsageHeatmapPainter({
    required this.days,
    required this.cellSize,
    required this.cellGap,
    required this.colorScheme,
    required this.maxTokens,
    required this.selectedDay,
  });

  final List<_HeatmapDay> days;
  final double cellSize;
  final double cellGap;
  final ColorScheme colorScheme;
  final int maxTokens;
  final _HeatmapDay? selectedDay;

  /// Paints the heatmap cells and the selected-cell indicator.
  @override
  void paint(Canvas canvas, Size size) {
    if (days.isEmpty) {
      return;
    }
    final leadingCells = days.first.day.weekday - DateTime.monday;
    final divisor = math.max(1, maxTokens);
    final selectedStrokePaint = Paint()
      ..color = colorScheme.onSurface.withValues(alpha: 0.72)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.8;
    for (var index = 0; index < days.length; index += 1) {
      final day = days[index];
      final cellIndex = leadingCells + index;
      final column = cellIndex ~/ DateTime.daysPerWeek;
      final row = cellIndex % DateTime.daysPerWeek;
      final ratio = day.totalTokens / divisor;
      final rect = Rect.fromLTWH(
        column * (cellSize + cellGap),
        row * (cellSize + cellGap),
        cellSize,
        cellSize,
      );
      final paint = Paint()..color = _heatmapCellColor(ratio, colorScheme);
      canvas.drawRRect(
        RRect.fromRectAndRadius(rect, const Radius.circular(3)),
        paint,
      );
      final selectedDay = this.selectedDay;
      if (selectedDay != null && _sameCalendarDay(selectedDay.day, day.day)) {
        canvas.drawRRect(
          RRect.fromRectAndRadius(rect.deflate(1), const Radius.circular(3)),
          selectedStrokePaint,
        );
      }
    }
  }

  /// Returns whether a heatmap repaint is required.
  @override
  bool shouldRepaint(covariant _UsageHeatmapPainter oldDelegate) {
    return oldDelegate.days != days ||
        oldDelegate.cellSize != cellSize ||
        oldDelegate.cellGap != cellGap ||
        oldDelegate.colorScheme != colorScheme ||
        oldDelegate.maxTokens != maxTokens ||
        oldDelegate.selectedDay != selectedDay;
  }
}

class _PieChartPainter extends CustomPainter {
  const _PieChartPainter({required this.slices});

  final List<_PieSliceData> slices;

  @override
  void paint(Canvas canvas, Size size) {
    if (slices.isEmpty) {
      return;
    }
    final rect = Offset.zero & size;
    final total = slices.fold<int>(0, (sum, slice) => sum + slice.value);
    if (total <= 0) {
      return;
    }
    final strokeWidth = math.max<double>(20.0, size.shortestSide * 0.18);
    final arcRect = rect.deflate(strokeWidth / 2 + 6);
    var startAngle = -math.pi / 2;
    for (final slice in slices) {
      final sweepAngle = (slice.value / total) * math.pi * 2;
      final paint = Paint()
        ..color = slice.color
        ..style = PaintingStyle.stroke
        ..strokeWidth = strokeWidth
        ..strokeCap = StrokeCap.butt;
      canvas.drawArc(arcRect, startAngle, sweepAngle, false, paint);
      startAngle += sweepAngle;
    }
  }

  @override
  bool shouldRepaint(covariant _PieChartPainter oldDelegate) {
    return oldDelegate.slices != slices;
  }
}

/// Returns the localized label for a model function slot.
String _functionTypeLabel(
  core_proxy.FunctionType functionType,
  AppLocalizations l10n,
) {
  return switch (functionType) {
    core_proxy.FunctionType.chat => l10n.settingsModelFunctionChat,
    core_proxy.FunctionType.summary => l10n.settingsModelFunctionSummary,
    core_proxy.FunctionType.titleGeneration =>
      l10n.settingsModelFunctionTitleGeneration,
    core_proxy.FunctionType.memory => l10n.settingsModelFunctionMemory,
    core_proxy.FunctionType.uiController =>
      l10n.settingsModelFunctionUiController,
    core_proxy.FunctionType.translation =>
      l10n.settingsModelFunctionTranslation,
    core_proxy.FunctionType.grep => l10n.settingsModelFunctionGrep,
    core_proxy.FunctionType.roleResponsePlanner =>
      l10n.settingsModelFunctionRoleResponsePlanner,
    core_proxy.FunctionType.imageRecognition =>
      l10n.settingsModelFunctionImageRecognition,
    core_proxy.FunctionType.audioRecognition =>
      l10n.settingsModelFunctionAudioRecognition,
    core_proxy.FunctionType.videoRecognition =>
      l10n.settingsModelFunctionVideoRecognition,
  };
}

/// Builds the stable aggregation key for a function-model pair.
String _functionModelKey(
  core_proxy.FunctionType functionType,
  String providerLabel,
  String modelLabel,
) {
  return '${functionType.value}::$providerLabel::$modelLabel';
}

/// Builds the visible label for a function-model pair.
String _functionModelTitle(
  String functionTitle,
  String providerLabel,
  String modelLabel,
) {
  return '$functionTitle · $providerLabel · $modelLabel';
}

String _providerLabel(String provider, AppLocalizations l10n) {
  final value = provider.trim();
  return value.isEmpty
      ? l10n.settingsDataDetailedStatsUnlabeledProvider
      : value;
}

String _modelLabel(String model, AppLocalizations l10n) {
  final value = model.trim();
  return value.isEmpty ? l10n.settingsDataDetailedStatsUnlabeledModel : value;
}

List<_PieSliceData> _buildPieSlices({
  required List<_AggregateRow> rows,
  required String otherLabel,
  required List<Color> palette,
}) {
  final sortedRows = List<_AggregateRow>.of(rows)
    ..sort((left, right) => right.totalTokens.compareTo(left.totalTokens));
  final visibleRows = <_AggregateRow>[];
  var otherValue = 0;
  for (var index = 0; index < sortedRows.length; index++) {
    if (index < 5) {
      visibleRows.add(sortedRows[index]);
      continue;
    }
    otherValue += sortedRows[index].totalTokens;
  }
  if (otherValue > 0) {
    visibleRows.add(_AggregateRow(label: otherLabel, totalTokens: otherValue));
  }
  return visibleRows
      .asMap()
      .entries
      .map(
        (entry) => _PieSliceData(
          label: entry.value.label,
          value: entry.value.totalTokens,
          color: palette[entry.key % palette.length],
        ),
      )
      .toList(growable: false);
}

List<Color> _buildPalette({required int constraintsSeed}) {
  const basePalette = <Color>[
    Color(0xFF8BC34A),
    Color(0xFF4DB6AC),
    Color(0xFFFFB74D),
    Color(0xFF64B5F6),
    Color(0xFFE57373),
    Color(0xFF9575CD),
    Color(0xFF4DD0E1),
    Color(0xFFA1887F),
  ];
  final offset = constraintsSeed % basePalette.length;
  return List<Color>.generate(
    basePalette.length,
    (index) => basePalette[(index + offset) % basePalette.length],
    growable: false,
  );
}

/// Builds continuous day cells for the usage heatmap.
List<_HeatmapDay> _buildHeatmapDays(List<_DayUsagePoint> points) {
  if (points.isEmpty) {
    return const <_HeatmapDay>[];
  }
  final pointsByDay = <DateTime, _DayUsagePoint>{
    for (final point in points) point.day: point,
  };
  final firstDay = points.first.day;
  final lastDay = points.last.day;
  final days = <_HeatmapDay>[];
  for (
    var day = firstDay;
    !day.isAfter(lastDay);
    day = day.add(const Duration(days: 1))
  ) {
    final point = pointsByDay[day];
    days.add(
      _HeatmapDay(
        day: day,
        requestCount: point?.requestCount ?? 0,
        totalTokens: point == null ? 0 : point.inputTokens + point.outputTokens,
      ),
    );
  }
  return days;
}

/// Returns the heatmap day at a local canvas position.
_HeatmapDay? _heatmapDayAtPosition(
  List<_HeatmapDay> days,
  Offset position,
  double cellSize,
  double cellGap,
) {
  if (days.isEmpty || position.dx < 0 || position.dy < 0) {
    return null;
  }
  final step = cellSize + cellGap;
  final column = position.dx ~/ step;
  final row = position.dy ~/ step;
  final xInsideCell = position.dx - column * step;
  final yInsideCell = position.dy - row * step;
  if (xInsideCell >= cellSize || yInsideCell >= cellSize) {
    return null;
  }
  final leadingCells = days.first.day.weekday - DateTime.monday;
  final dayIndex = column * DateTime.daysPerWeek + row - leadingCells;
  if (dayIndex < 0 || dayIndex >= days.length) {
    return null;
  }
  return days[dayIndex];
}

/// Returns whether two timestamps land on the same calendar day.
bool _sameCalendarDay(DateTime left, DateTime right) {
  return left.year == right.year &&
      left.month == right.month &&
      left.day == right.day;
}

/// Returns the number of week columns needed by the heatmap.
int _heatmapWeekCount(List<_HeatmapDay> days) {
  if (days.isEmpty) {
    return 0;
  }
  final leadingCells = days.first.day.weekday - DateTime.monday;
  final cellCount = leadingCells + days.length;
  return (cellCount / DateTime.daysPerWeek).ceil();
}

/// Returns the heatmap color for a normalized token intensity.
Color _heatmapCellColor(double ratio, ColorScheme colorScheme) {
  if (ratio <= 0) {
    return colorScheme.surfaceContainerHighest.withValues(alpha: 0.42);
  }
  final adjustedRatio = 0.24 + ratio.clamp(0.0, 1.0) * 0.76;
  return Color.lerp(
    colorScheme.surfaceContainerHighest,
    colorScheme.primary,
    adjustedRatio,
  )!;
}

/// Returns dashboard horizontal padding for the active viewport width.
double _dashboardPagePadding(double width) {
  return width >= 720 ? _dashboardWidePadding : _dashboardCompactPadding;
}

/// Returns the localized label for a date range chip.
String _usageDateRangeLabel(AppLocalizations l10n, _UsageDateRange range) {
  return switch (range) {
    _UsageDateRange.all => l10n.settingsDataDetailedStatsRangeAll,
    _UsageDateRange.last7Days => l10n.settingsDataDetailedStatsRangeLast7Days,
    _UsageDateRange.last30Days => l10n.settingsDataDetailedStatsRangeLast30Days,
    _UsageDateRange.custom => l10n.settingsDataDetailedStatsRangeCustom,
  };
}

/// Builds the compact date summary shown in the dashboard toolbar.
String _usageDateRangeSummary(
  AppLocalizations l10n,
  _UsageStatisticsViewData viewData,
  _UsageDateRange range,
  DateTimeRange? customRange,
) {
  if (range == _UsageDateRange.custom) {
    final selectedRange = customRange!;
    return l10n.settingsDataDetailedStatsDateRange(
      _formatDate(selectedRange.start),
      _formatDate(selectedRange.end),
    );
  }
  final firstRecordAt = viewData.firstRecordAt;
  final lastRecordAt = viewData.lastRecordAt;
  if (firstRecordAt == null || lastRecordAt == null) {
    if (range == _UsageDateRange.all) {
      return l10n.settingsDataDetailedStatsHistoricalTotal;
    }
    return '${_usageDateRangeLabel(l10n, range)} · ${l10n.settingsDataDetailedStatsNoDataInRange}';
  }
  final dateSpan = l10n.settingsDataDetailedStatsDateRange(
    _formatDate(firstRecordAt),
    _formatDate(lastRecordAt),
  );
  if (range == _UsageDateRange.all) {
    return dateSpan;
  }
  return '${_usageDateRangeLabel(l10n, range)} · $dateSpan';
}

/// Returns the date component of a timestamp.
DateTime? _dateOnly(DateTime? value) {
  if (value == null) {
    return null;
  }
  return DateTime(value.year, value.month, value.day);
}

/// Returns a date-only range for inclusive day matching.
DateTimeRange _dateOnlyRange(DateTimeRange range) {
  return DateTimeRange(
    start: _dateOnly(range.start)!,
    end: _dateOnly(range.end)!,
  );
}

/// Returns whether a record occurred within the trailing day window.
bool _recordIsInsideTrailingDays(
  _UsageRecord record,
  DateTime? endDay,
  int dayCount,
) {
  if (endDay == null) {
    return false;
  }
  final startDay = endDay.subtract(Duration(days: dayCount - 1));
  return _recordIsInsideDateSpan(record, startDay, endDay);
}

/// Returns whether a record occurred inside an inclusive date span.
bool _recordIsInsideDateSpan(
  _UsageRecord record,
  DateTime startDay,
  DateTime endDay,
) {
  final occurredDay = _dateOnly(record.occurredAt);
  if (occurredDay == null) {
    return false;
  }
  return !occurredDay.isBefore(startDay) && !occurredDay.isAfter(endDay);
}

/// Builds the three chart labels used for the visible date axis.
List<String> _buildDateAxisLabels(List<_DayUsagePoint> points) {
  if (points.isEmpty) {
    return const <String>['', '', ''];
  }
  final middle = points[points.length ~/ 2].day;
  return <String>[
    _formatShortDate(points.first.day),
    _formatShortDate(middle),
    _formatShortDate(points.last.day),
  ];
}

/// Computes the width for cards in a wrapping responsive row.
double _responsiveCardWidth({
  required double maxWidth,
  required double minWidth,
  required int maxColumns,
  double spacing = 12,
}) {
  for (var columns = maxColumns; columns > 1; columns--) {
    final candidate = (maxWidth - spacing * (columns - 1)) / columns;
    if (candidate >= minWidth) {
      return candidate;
    }
  }
  return maxWidth;
}

void _paintText(
  Canvas canvas, {
  required String text,
  required Offset offset,
  required TextStyle style,
  required double maxWidth,
  TextAlign textAlign = TextAlign.left,
}) {
  final painter = TextPainter(
    text: TextSpan(text: text, style: style),
    textDirection: TextDirection.ltr,
    textAlign: textAlign,
    maxLines: 1,
    ellipsis: '…',
  )..layout(maxWidth: maxWidth);
  painter.paint(canvas, offset);
}

String _formatCompactInt(int value) {
  final absValue = value.abs();
  if (absValue >= 1000000000) {
    return '${(value / 1000000000).toStringAsFixed(absValue >= 10000000000 ? 0 : 1)}B';
  }
  if (absValue >= 1000000) {
    return '${(value / 1000000).toStringAsFixed(absValue >= 10000000 ? 0 : 1)}M';
  }
  if (absValue >= 1000) {
    return '${(value / 1000).toStringAsFixed(absValue >= 10000 ? 0 : 1)}K';
  }
  return value.toString();
}

String _formatFullInt(int value) {
  final digits = value.abs().toString();
  final buffer = StringBuffer();
  for (var index = 0; index < digits.length; index++) {
    final reverseIndex = digits.length - index;
    buffer.write(digits[index]);
    if (reverseIndex > 1 && reverseIndex % 3 == 1) {
      buffer.write(',');
    }
  }
  return value < 0 ? '-$buffer' : buffer.toString();
}

String _formatPercent(double ratio) {
  final percentage = ratio * 100;
  return percentage >= 10
      ? '${percentage.toStringAsFixed(0)}%'
      : '${percentage.toStringAsFixed(1)}%';
}

String _formatDate(DateTime value) {
  return '${value.year}-${_twoDigits(value.month)}-${_twoDigits(value.day)}';
}

String _formatShortDate(DateTime value) {
  return '${_twoDigits(value.month)}/${_twoDigits(value.day)}';
}

String _twoDigits(int value) => value.toString().padLeft(2, '0');

/// Returns the earliest recorded timestamp.
DateTime? _firstDatedRecord(List<_UsageRecord> records) {
  for (final record in records) {
    final occurredAt = record.occurredAt;
    if (occurredAt != null) return occurredAt;
  }
  return null;
}

/// Returns the latest recorded timestamp.
DateTime? _lastDatedRecord(List<_UsageRecord> records) {
  for (var index = records.length - 1; index >= 0; index -= 1) {
    final occurredAt = records[index].occurredAt;
    if (occurredAt != null) return occurredAt;
  }
  return null;
}
