// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

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
          final viewData = _UsageStatisticsViewData.fromRequestRecords(
            records,
            l10n,
          );
          if (viewData.records.isEmpty) {
            return _UsageStatisticsEmptyState(
              title: l10n.settingsDataDetailedStatsEmpty,
              description: l10n.settingsDataDetailedStatsDescription,
            );
          }
          return LayoutBuilder(
            builder: (context, constraints) {
              final metricWidth = _responsiveCardWidth(
                maxWidth: constraints.maxWidth,
                minWidth: 168,
                maxColumns: 3,
              );
              final pieWidth = _responsiveCardWidth(
                maxWidth: constraints.maxWidth,
                minWidth: 320,
                maxColumns: 3,
              );
              final rankingWidth = _responsiveCardWidth(
                maxWidth: constraints.maxWidth,
                minWidth: 360,
                maxColumns: 2,
              );
              return ListView(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
                children: <Widget>[
                  _UsageStatisticsSectionCard(
                    title: l10n.settingsDataDetailedStatsTitle,
                    subtitle: l10n.settingsDataDetailedStatsDescription,
                    child: Wrap(
                      spacing: 8,
                      runSpacing: 8,
                      children: <Widget>[
                        _InfoPill(
                          icon: Icons.date_range_outlined,
                          label: l10n.settingsDataDetailedStatsDateRange(
                            _formatDate(viewData.firstRecordAt),
                            _formatDate(viewData.lastRecordAt),
                          ),
                        ),
                        _InfoPill(
                          icon: Icons.storage_outlined,
                          label: l10n.settingsDataDetailedStatsSourceLabel,
                        ),
                      ],
                    ),
                  ),
                  Wrap(
                    spacing: 12,
                    runSpacing: 12,
                    children: <Widget>[
                      SizedBox(
                        width: metricWidth,
                        child: _MetricCard(
                          icon: Icons.bolt_outlined,
                          label: l10n.settingsDataDetailedStatsTotalRequests,
                          value: _formatFullInt(viewData.totalRequests),
                          accent: Theme.of(context).colorScheme.primary,
                        ),
                      ),
                      SizedBox(
                        width: metricWidth,
                        child: _MetricCard(
                          icon: Icons.south_west_outlined,
                          label: l10n.settingsDataInputTokens,
                          value: _formatFullInt(viewData.totalInputTokens),
                          accent: Theme.of(context).colorScheme.secondary,
                        ),
                      ),
                      SizedBox(
                        width: metricWidth,
                        child: _MetricCard(
                          icon: Icons.north_east_outlined,
                          label: l10n.settingsDataOutputTokens,
                          value: _formatFullInt(viewData.totalOutputTokens),
                          accent: Theme.of(context).colorScheme.tertiary,
                        ),
                      ),
                      SizedBox(
                        width: metricWidth,
                        child: _MetricCard(
                          icon: Icons.layers_outlined,
                          label: l10n.settingsDataDetailedStatsCachedInput,
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
                          label: l10n.settingsDataDetailedStatsActiveDays,
                          value: _formatFullInt(viewData.activeDays),
                          accent: Theme.of(context).colorScheme.secondary,
                        ),
                      ),
                      SizedBox(
                        width: metricWidth,
                        child: _MetricCard(
                          icon: Icons.forum_outlined,
                          label: l10n.settingsDataDetailedStatsChats,
                          value: _formatFullInt(viewData.chatCount),
                          accent: Theme.of(context).colorScheme.tertiary,
                        ),
                      ),
                      SizedBox(
                        width: metricWidth,
                        child: _MetricCard(
                          icon: Icons.hub_outlined,
                          label: l10n.settingsDataDetailedStatsProviders,
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
                          accent: Theme.of(context).colorScheme.secondary,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  _LineChartCard(
                    title: l10n.settingsDataDetailedStatsDailyUsageTitle,
                    subtitle: l10n.settingsDataDetailedStatsDailyUsageSubtitle,
                    points: viewData.dailyPoints,
                    xLabels: _buildDateAxisLabels(viewData.dailyPoints),
                    series: <_ChartSeries>[
                      _ChartSeries(
                        label: l10n.settingsDataDetailedStatsRequestsSeries,
                        color: Theme.of(context).colorScheme.primary,
                        selector: (_DayUsagePoint point) => point.requestCount,
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  _LineChartCard(
                    title: l10n.settingsDataDetailedStatsInputOutputTitle,
                    subtitle: l10n.settingsDataDetailedStatsInputOutputSubtitle,
                    points: viewData.dailyPoints,
                    xLabels: _buildDateAxisLabels(viewData.dailyPoints),
                    series: <_ChartSeries>[
                      _ChartSeries(
                        label: l10n.settingsDataInputTokens,
                        color: Theme.of(context).colorScheme.secondary,
                        selector: (_DayUsagePoint point) => point.inputTokens,
                      ),
                      _ChartSeries(
                        label: l10n.settingsDataOutputTokens,
                        color: Theme.of(context).colorScheme.tertiary,
                        selector: (_DayUsagePoint point) => point.outputTokens,
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  Wrap(
                    spacing: 12,
                    runSpacing: 12,
                    children: <Widget>[
                      SizedBox(
                        width: pieWidth,
                        child: _PieChartCard(
                          title: l10n.settingsDataDetailedStatsProviderPieTitle,
                          totalLabel: l10n.settingsDataDetailedStatsTotalTokens,
                          slices: viewData.providerSlices,
                        ),
                      ),
                      SizedBox(
                        width: pieWidth,
                        child: _PieChartCard(
                          title: l10n.settingsDataDetailedStatsModelPieTitle,
                          totalLabel: l10n.settingsDataDetailedStatsTotalTokens,
                          slices: viewData.modelSlices,
                        ),
                      ),
                      SizedBox(
                        width: pieWidth,
                        child: _PieChartCard(
                          title: l10n.settingsDataDetailedStatsChatPieTitle,
                          totalLabel: l10n.settingsDataDetailedStatsTotalTokens,
                          slices: viewData.chatSlices,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  Wrap(
                    spacing: 12,
                    runSpacing: 12,
                    children: <Widget>[
                      SizedBox(
                        width: rankingWidth,
                        child: _TopListCard(
                          title: l10n.settingsDataDetailedStatsTopRequestsTitle,
                          subtitle:
                              l10n.settingsDataDetailedStatsTopRequestsSubtitle,
                          rows: viewData.topRequestRows,
                        ),
                      ),
                      SizedBox(
                        width: rankingWidth,
                        child: _TopListCard(
                          title: l10n.settingsDataDetailedStatsTopChatsTitle,
                          subtitle:
                              l10n.settingsDataDetailedStatsTopChatsSubtitle,
                          rows: viewData.topChatRows,
                        ),
                      ),
                    ],
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
    required this.totalRequests,
    required this.totalInputTokens,
    required this.totalOutputTokens,
    required this.totalCachedInputTokens,
    required this.activeDays,
    required this.chatCount,
    required this.providerCount,
    required this.modelCount,
    required this.firstRecordAt,
    required this.lastRecordAt,
    required this.providerSlices,
    required this.modelSlices,
    required this.chatSlices,
    required this.topRequestRows,
    required this.topChatRows,
  });

  final List<_UsageRecord> records;
  final List<_DayUsagePoint> dailyPoints;
  final int totalRequests;
  final int totalInputTokens;
  final int totalOutputTokens;
  final int totalCachedInputTokens;
  final int activeDays;
  final int chatCount;
  final int providerCount;
  final int modelCount;
  final DateTime firstRecordAt;
  final DateTime lastRecordAt;
  final List<_PieSliceData> providerSlices;
  final List<_PieSliceData> modelSlices;
  final List<_PieSliceData> chatSlices;
  final List<_TopListRowData> topRequestRows;
  final List<_TopListRowData> topChatRows;

  static _UsageStatisticsViewData fromRequestRecords(
    List<core_proxy.UsageRequestRecord> requestRecords,
    AppLocalizations l10n,
  ) {
    final records = requestRecords.map((record) {
      final chatId = record.chatId?.trim() ?? '';
      final groupKey = chatId.isEmpty
          ? 'source:${record.source.value}'
          : 'chat:$chatId';
      final groupTitle = chatId.isEmpty
          ? _usageSourceLabel(record.source, l10n)
          : chatId;
      return _UsageRecord(
        chatId: groupKey,
        chatTitle: groupTitle,
        occurredAt: DateTime.fromMillisecondsSinceEpoch(
          record.createdAtMs,
        ).toLocal(),
        providerLabel: _providerLabel(record.provider, l10n),
        modelLabel: _modelLabel(record.modelName, l10n),
        inputTokens: record.inputTokens,
        outputTokens: record.outputTokens,
        cachedInputTokens: record.cachedInputTokens,
      );
    }).toList(growable: false);
    if (records.isEmpty) {
      final now = DateTime.now();
      return _UsageStatisticsViewData(
        records: const <_UsageRecord>[],
        dailyPoints: const <_DayUsagePoint>[],
        totalRequests: 0,
        totalInputTokens: 0,
        totalOutputTokens: 0,
        totalCachedInputTokens: 0,
        activeDays: 0,
        chatCount: 0,
        providerCount: 0,
        modelCount: 0,
        firstRecordAt: now,
        lastRecordAt: now,
        providerSlices: const <_PieSliceData>[],
        modelSlices: const <_PieSliceData>[],
        chatSlices: const <_PieSliceData>[],
        topRequestRows: const <_TopListRowData>[],
        topChatRows: const <_TopListRowData>[],
      );
    }

    records.sort((left, right) => left.occurredAt.compareTo(right.occurredAt));
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

      final dayKey = DateTime(
        record.occurredAt.year,
        record.occurredAt.month,
        record.occurredAt.day,
      );
      dailyAccumulators
          .putIfAbsent(dayKey, _DayUsageAccumulator.new)
          .add(record);
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

    final topRequestRows = List<_UsageRecord>.of(records)
      ..sort(
        (left, right) => right.totalTokens.compareTo(left.totalTokens) != 0
            ? right.totalTokens.compareTo(left.totalTokens)
            : right.occurredAt.compareTo(left.occurredAt),
      );

    final topChatRows = chatAccumulators.values.toList()
      ..sort(
        (left, right) => right.totalTokens.compareTo(left.totalTokens) != 0
            ? right.totalTokens.compareTo(left.totalTokens)
            : right.chatTitle.compareTo(left.chatTitle),
      );

    return _UsageStatisticsViewData(
      records: records,
      dailyPoints: dailyPoints
          .map(
            (entry) => _DayUsagePoint(
              day: entry.key,
              requestCount: entry.value.requestCount,
              inputTokens: entry.value.inputTokens,
              outputTokens: entry.value.outputTokens,
            ),
          )
          .toList(growable: false),
      totalRequests: records.length,
      totalInputTokens: totalInputTokens,
      totalOutputTokens: totalOutputTokens,
      totalCachedInputTokens: totalCachedInputTokens,
      activeDays: dailyAccumulators.length,
      chatCount: chatAccumulators.length,
      providerCount: providerAccumulators.length,
      modelCount: modelAccumulators.length,
      firstRecordAt: records.first.occurredAt,
      lastRecordAt: records.last.occurredAt,
      providerSlices: providerSlices,
      modelSlices: modelSlices,
      chatSlices: chatSlices,
      topRequestRows: topRequestRows
          .take(8)
          .map(
            (record) => _TopListRowData(
              title: '${record.providerLabel} · ${record.modelLabel}',
              subtitle: l10n.settingsDataDetailedStatsInputOutputSummary(
                _formatCompactInt(record.inputTokens),
                _formatCompactInt(record.outputTokens),
                record.chatTitle,
                _formatDateTime(record.occurredAt),
              ),
              trailing: _formatCompactInt(record.totalTokens),
            ),
          )
          .toList(growable: false),
      topChatRows: topChatRows
          .take(8)
          .map(
            (aggregate) => _TopListRowData(
              title: aggregate.chatTitle,
              subtitle: l10n.settingsDataDetailedStatsRequestModelSummary(
                aggregate.requestCount,
                aggregate.distinctModels.length,
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
    required this.inputTokens,
    required this.outputTokens,
    required this.cachedInputTokens,
  });

  final String chatId;
  final String chatTitle;
  final DateTime occurredAt;
  final String providerLabel;
  final String modelLabel;
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
    requestCount += 1;
    inputTokens += record.inputTokens;
    outputTokens += record.outputTokens;
  }
}

class _AggregateAccumulator {
  int totalTokens = 0;
  int requestCount = 0;
  String chatTitle = '';
  final Set<String> distinctModels = <String>{};

  void add(_UsageRecord record) {
    totalTokens += record.totalTokens;
    requestCount += 1;
    chatTitle = record.chatTitle;
    distinctModels.add(record.modelLabel);
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
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 14, 16, 14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                title,
                style: SettingsControlStyles.sectionTitleTextStyle(context),
              ),
              const SizedBox(height: 6),
              Text(
                subtitle,
                style: TextStyle(color: colorScheme.onSurfaceVariant),
              ),
              const SizedBox(height: 12),
              child,
            ],
          ),
        ),
      ),
    );
  }
}

class _InfoPill extends StatelessWidget {
  const _InfoPill({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.24),
        ),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(icon, size: 18, color: colorScheme.primary),
          const SizedBox(width: 8),
          Flexible(child: Text(label)),
        ],
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
      borderRadius: BorderRadius.circular(16),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      material: true,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 14, 16, 14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(icon, size: 20, color: accent),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    label,
                    style: TextStyle(color: colorScheme.onSurfaceVariant),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 10),
            Text(
              value,
              style: Theme.of(
                context,
              ).textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w700),
            ),
          ],
        ),
      ),
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
      child: Column(
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
            height: 240,
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
              ? math.min<double>(180.0, constraints.maxWidth * 0.42)
              : 190.0;
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
                    Text(
                      _formatCompactInt(totalValue),
                      style: Theme.of(context).textTheme.titleLarge?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      totalLabel,
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
  });

  final String title;
  final String subtitle;
  final List<_TopListRowData> rows;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return _UsageStatisticsSectionCard(
      title: title,
      subtitle: subtitle,
      child: Column(
        children: rows
            .asMap()
            .entries
            .map(
              (entry) => Column(
                children: <Widget>[
                  if (entry.key > 0)
                    Divider(
                      height: 18,
                      color: colorScheme.outlineVariant.withValues(alpha: 0.24),
                    ),
                  _TopListRow(row: entry.value),
                ],
              ),
            )
            .toList(growable: false),
      ),
    );
  }
}

class _TopListRow extends StatelessWidget {
  const _TopListRow({required this.row});

  final _TopListRowData row;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                row.title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(fontWeight: FontWeight.w700),
              ),
              const SizedBox(height: 4),
              Text(
                row.subtitle,
                style: TextStyle(color: colorScheme.onSurfaceVariant),
              ),
            ],
          ),
        ),
        const SizedBox(width: 12),
        Text(
          row.trailing,
          style: TextStyle(
            color: colorScheme.primary,
            fontWeight: FontWeight.w700,
          ),
        ),
      ],
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

String _usageSourceLabel(
  core_proxy.UsageRequestSource source,
  AppLocalizations l10n,
) {
  return switch (source) {
    core_proxy.UsageRequestSource.chatResponse =>
      l10n.settingsDataDetailedStatsSourceChat,
    core_proxy.UsageRequestSource.toolResultResponse =>
      l10n.settingsDataDetailedStatsSourceToolResult,
    core_proxy.UsageRequestSource.summaryGeneration =>
      l10n.settingsDataDetailedStatsSourceSummary,
    core_proxy.UsageRequestSource.memoryAnalysis =>
      l10n.settingsDataDetailedStatsSourceMemory,
  };
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

String _formatDateTime(DateTime value) {
  return '${_formatShortDate(value)} ${_twoDigits(value.hour)}:${_twoDigits(value.minute)}';
}

String _twoDigits(int value) => value.toString().padLeft(2, '0');
