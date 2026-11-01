// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import 'UsageTimeline.dart';
import 'UsagePricing.dart';

const double _dashboardMaxWidth = 1440;
const double _dashboardCompactPadding = 16;
const double _dashboardWidePadding = 24;
const double _dashboardSectionSpacing = 12;

enum _UsageDateRange { lastDay, last7Days, last30Days, all, custom }

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

class _UsageStatisticsSnapshot {
  const _UsageStatisticsSnapshot(
    this.records,
    this.pricing,
    this.pricingLoadFailed,
  );
  final List<core_proxy.UsageRequestRecord> records;
  final UsagePricingIndex pricing;
  final bool pricingLoadFailed;
}

class _UsageStatisticsDetailScreenState
    extends State<UsageStatisticsDetailScreen> {
  late Future<_UsageStatisticsSnapshot> _future;
  _UsageDateRange _selectedDateRange = _UsageDateRange.lastDay;
  DateTimeRange? _customDateRange;

  @override
  void initState() {
    super.initState();
    _future = _load();
  }

  Future<_UsageStatisticsSnapshot> _load() async {
    final store = widget.clients.repositoryUsageStatisticsStore;
    final pricingFuture = () async {
      try {
        final result = await Future.wait<Object>([
          store.getAllTokenStatsModels(),
          widget.clients.preferencesModelConfigManager.getAllModelSummaries(),
        ]);
        return (
          UsagePricingIndex(
            saved: result[0] as List<core_proxy.TokenStatsModel>,
            configured: result[1] as List<core_proxy.ProviderModelSummary>,
          ),
          false,
        );
      } catch (_) {
        return (UsagePricingIndex(), true);
      }
    }();
    final records = await store.getAllRequestRecords();
    final (pricing, failed) = await pricingFuture;
    return _UsageStatisticsSnapshot(records, pricing, failed);
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

  DateTimeRange? _timelineDateRange() {
    if (_selectedDateRange == _UsageDateRange.all) return null;
    final now = DateTime.now();
    if (_selectedDateRange == _UsageDateRange.lastDay) {
      return DateTimeRange(
        start: now.subtract(const Duration(hours: 24)),
        end: now,
      );
    }
    final today = _dateOnly(now)!;
    final start = switch (_selectedDateRange) {
      _UsageDateRange.last7Days => today.subtract(const Duration(days: 6)),
      _UsageDateRange.last30Days => today.subtract(const Duration(days: 29)),
      _UsageDateRange.custom => _customDateRange!.start,
      _UsageDateRange.all || _UsageDateRange.lastDay => today,
    };
    final lastDay = _selectedDateRange == _UsageDateRange.custom
        ? _customDateRange!.end
        : today;
    return DateTimeRange(
      start: start,
      end: DateTime(lastDay.year, lastDay.month, lastDay.day + 1),
    );
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
      body: FutureBuilder<_UsageStatisticsSnapshot>(
        future: _future,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
          }
          final loaded = snapshot.data;
          if (loaded == null) {
            return const M3LoadingPane();
          }
          final allViewData = _UsageStatisticsViewData.fromRequestRecords(
            loaded.records,
            l10n,
            loaded.pricing,
          );
          if (allViewData.records.isEmpty) {
            return _UsageStatisticsEmptyState(
              title: l10n.settingsDataDetailedStatsEmpty,
              description: l10n.settingsDataDetailedStatsDescription,
            );
          }
          final selectedWindow = _timelineDateRange();
          final viewData = allViewData.selectTimeRange(selectedWindow);
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
                                  icon: Icons.track_changes_outlined,
                                  label: l10n.settingsDataTimelineCacheRate,
                                  value: viewData.totalInputTokens == 0
                                      ? '—'
                                      : '${(viewData.totalCachedInputTokens / viewData.totalInputTokens * 100).toStringAsFixed(1)}%',
                                  accent: Theme.of(
                                    context,
                                  ).colorScheme.tertiary,
                                ),
                              ),
                              SizedBox(
                                width: metricWidth,
                                child: _MetricCard(
                                  icon: Icons.payments_outlined,
                                  label: viewData.hasUnpricedRecords
                                      ? l10n.settingsDataTimelineKnownSpend
                                      : l10n.settingsDataTimelineSpend,
                                  value: viewData.spendSummary,
                                  accent: Theme.of(
                                    context,
                                  ).colorScheme.secondary,
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
                          UsageTimeline(
                            records: viewData.records,
                            dateRange: selectedWindow,
                            pricingLoadFailed: loaded.pricingLoadFailed,
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
    required this.heatmapDays,
    required this.totalInputTokens,
    required this.totalOutputTokens,
    required this.totalCachedInputTokens,
  });

  final List<UsageTimelineRecord> records;
  final List<_HeatmapDay> heatmapDays;
  final int totalInputTokens;
  final int totalOutputTokens;
  final int totalCachedInputTokens;

  int get totalRequests => records.length;
  DateTime? get firstRecordAt => records.firstOrNull?.occurredAt;
  DateTime? get lastRecordAt => records.lastOrNull?.occurredAt;

  bool get hasUnpricedRecords =>
      records.any((record) => record.estimatedCost == null);

  String get spendSummary {
    final totals = <String, double>{};
    for (final record in records) {
      final cost = record.estimatedCost;
      if (cost != null) {
        totals.update(
          record.currency!,
          (value) => value + cost,
          ifAbsent: () => cost,
        );
      }
    }
    if (totals.isEmpty) return '—';
    return totals.entries
        .map((entry) => '${entry.key} ${formatUsageMoney(entry.value)}')
        .join(' / ');
  }

  _UsageStatisticsViewData selectTimeRange(DateTimeRange? range) {
    if (range == null) return this;
    return fromUsageRecords(
      records
          .where(
            (record) =>
                !record.occurredAt.isBefore(range.start) &&
                record.occurredAt.isBefore(range.end),
          )
          .toList(),
    );
  }

  static _UsageStatisticsViewData fromRequestRecords(
    List<core_proxy.UsageRequestRecord> requestRecords,
    AppLocalizations l10n,
    UsagePricingIndex pricing,
  ) {
    return fromUsageRecords(
      requestRecords
          .map(
            (record) => UsageTimelineRecord(
              occurredAt: DateTime.fromMillisecondsSinceEpoch(
                record.createdAtMs,
              ).toLocal(),
              provider: _providerLabel(record.provider, l10n),
              model: _modelLabel(record.modelName, l10n),
              inputTokens: record.inputTokens,
              outputTokens: record.outputTokens,
              cachedInputTokens: record.cachedInputTokens,
              pricing: pricing.forModel(record.provider, record.modelName),
            ),
          )
          .toList(),
    );
  }

  static _UsageStatisticsViewData fromUsageRecords(
    List<UsageTimelineRecord> source,
  ) {
    final records = List<UsageTimelineRecord>.of(source)
      ..sort((a, b) => a.occurredAt.compareTo(b.occurredAt));
    final days = <DateTime, _DayUsageAccumulator>{};
    var input = 0;
    var output = 0;
    var cached = 0;
    for (final record in records) {
      input += record.inputTokens;
      output += record.outputTokens;
      cached += record.cachedInputTokens;
      days
          .putIfAbsent(_dateOnly(record.occurredAt)!, _DayUsageAccumulator.new)
          .add(record);
    }
    return _UsageStatisticsViewData(
      records: records,
      heatmapDays: _buildHeatmapDays(
        days.entries
            .map(
              (entry) => _DayUsagePoint(
                day: entry.key,
                requestCount: entry.value.requestCount,
                inputTokens: entry.value.inputTokens,
                outputTokens: entry.value.outputTokens,
              ),
            )
            .toList(),
      ),
      totalInputTokens: input,
      totalOutputTokens: output,
      totalCachedInputTokens: cached,
    );
  }
}

class _DayUsageAccumulator {
  int requestCount = 0;
  int inputTokens = 0;
  int outputTokens = 0;

  void add(UsageTimelineRecord record) {
    requestCount += 1;
    inputTokens += record.inputTokens;
    outputTokens += record.outputTokens;
  }
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
          ? Padding(
              padding: const EdgeInsets.all(16),
              child: Text(widget.emptyLabel),
            )
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
    _UsageDateRange.lastDay => l10n.settingsDataDetailedStatsRangeLastDay,
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

String _formatDate(DateTime value) {
  return '${value.year}-${_twoDigits(value.month)}-${_twoDigits(value.day)}';
}

String _twoDigits(int value) => value.toString().padLeft(2, '0');
