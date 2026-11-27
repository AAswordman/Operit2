// ignore_for_file: file_names

import 'dart:convert';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../model/ProviderLogo.dart';
import 'UsageAttributionSection.dart';
import 'UsageBentoComponents.dart';
import 'UsageFormatters.dart';
import 'UsagePricing.dart';
import 'UsageQuotaCenterSection.dart';
import 'UsageTimeline.dart';

const double _dashboardMaxWidth = 1440;
const double _dashboardCompactPadding = 16;
const double _dashboardWidePadding = 24;
const double _dashboardSectionSpacing = 12;

enum _DashboardSection { analytics, quotas }

enum _HeatmapMetric { tokens, requests, cost }

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

  @override
  State<UsageStatisticsDetailScreen> createState() =>
      _UsageStatisticsDetailScreenState();
}

class _UsageStatisticsSnapshot {
  const _UsageStatisticsSnapshot({
    required this.records,
    required this.pricing,
    required this.pricingLoadFailed,
    required this.quotaState,
    required this.quotaError,
    required this.deviceSpace,
  });

  final List<core_proxy.UsageRequestRecord> records;
  final UsagePricingIndex pricing;
  final bool pricingLoadFailed;
  final core_proxy.ProviderQuotaCenterState? quotaState;
  final String? quotaError;
  final core_proxy.RuntimeDeviceSpaceSnapshot? deviceSpace;
}

class _UsageStatisticsDetailScreenState
    extends State<UsageStatisticsDetailScreen> {
  late Future<_UsageStatisticsSnapshot> _future;
  _DashboardSection _activeSection = _DashboardSection.analytics;
  _UsageDateRange _selectedDateRange = _UsageDateRange.lastDay;
  DateTimeRange? _customDateRange;
  String? _selectedNodeId;
  bool _resetting = false;

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

    final quotaFuture = () async {
      try {
        final state = await widget.clients.servicesProviderQuotaService
            .getQuotaCenterState();
        return (state, null);
      } catch (error) {
        return (null, '$error');
      }
    }();

    final spaceFuture = widget.clients.serverRuntimeRemoteLinkService
        .deviceSpaceSnapshot();
    final recordsFuture = store.getAllRequestRecords();

    final records = await recordsFuture;
    final deviceSpace = await spaceFuture;
    final (pricing, pricingFailed) = await pricingFuture;
    final (quotaState, quotaError) = await quotaFuture;

    return _UsageStatisticsSnapshot(
      records: records,
      pricing: pricing,
      pricingLoadFailed: pricingFailed,
      quotaState: quotaState,
      quotaError: quotaError,
      deviceSpace: deviceSpace,
    );
  }

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  Future<void> _reloadAndAwait() async {
    final next = _load();
    setState(() {
      _future = next;
    });
    await next;
  }

  Future<void> _resetStatistics() async {
    final l10n = AppLocalizations.of(context)!;
    var selectedScope = core_proxy.UsageResetScope.localOnly;

    final scope = await showDialog<core_proxy.UsageResetScope>(
      context: context,
      builder: (dialogContext) => StatefulBuilder(
        builder: (context, setDialogState) => AlertDialog(
          title: Text(l10n.settingsUsageResetScopeTitle),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(l10n.settingsUsageResetScopeWarning),
              const SizedBox(height: 12),
              RadioGroup<core_proxy.UsageResetScope>(
                groupValue: selectedScope,
                onChanged: (value) {
                  if (value != null) {
                    setDialogState(() => selectedScope = value);
                  }
                },
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    RadioListTile<core_proxy.UsageResetScope>(
                      value: core_proxy.UsageResetScope.localOnly,
                      contentPadding: EdgeInsets.zero,
                      title: Text(l10n.settingsUsageResetLocalOnly),
                    ),
                    RadioListTile<core_proxy.UsageResetScope>(
                      value: core_proxy.UsageResetScope.allDevicesInSpace,
                      contentPadding: EdgeInsets.zero,
                      title: Text(l10n.settingsUsageResetAllSpace),
                    ),
                  ],
                ),
              ),
            ],
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(),
              child: Text(l10n.cancel),
            ),
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(selectedScope),
              child: Text(l10n.settingsDataResetTokenStats),
            ),
          ],
        ),
      ),
    );

    if (scope == null || !mounted) return;
    setState(() => _resetting = true);
    try {
      await widget.clients.servicesProviderQuotaService.resetUsageStatistics(
        scope: scope,
      );
      if (!mounted) return;
      _reload();
    } finally {
      if (mounted) {
        setState(() => _resetting = false);
      }
    }
  }

  Future<void> _selectDateRange() async {
    final now = DateTime.now();
    final initialRange =
        _customDateRange ??
        DateTimeRange(
          start: DateTime(now.year, now.month, now.day).subtract(
            const Duration(days: 6),
          ),
          end: DateTime(now.year, now.month, now.day),
        );
    final selection = await showDialog<_DateRangeSelection>(
      context: context,
      builder: (_) => _DateRangePickerDialog(
        selectedRange: _selectedDateRange,
        customRange: initialRange,
      ),
    );
    if (selection == null || !mounted) return;
    setState(() {
      _selectedDateRange = selection.range;
      if (selection.range == _UsageDateRange.custom &&
          selection.customRange != null) {
        _customDateRange = selection.customRange;
      }
    });
  }

  Future<void> _exportUsageData(
    List<core_proxy.UsageRequestRecord> records,
    UsagePricingIndex pricing, {
    required bool asCsv,
  }) async {
    final l10n = AppLocalizations.of(context)!;
    late final String content;
    if (asCsv) {
      final buffer = StringBuffer()
        ..writeln(
          'id,createdAtMs,provider,providerId,modelName,functionType,source,status,stream,inputTokens,cachedInputTokens,cacheWriteTokens,outputTokens,reasoningTokens,waitDurationMs,outputDurationMs,totalDurationMs,estimatedCost,currency,originNodeId,keyFingerprint',
        );
      for (final r in records) {
        final resolvedCost = pricing.costForRecord(r);
        buffer.writeln(
          [
            _csvCell(r.id),
            r.createdAtMs,
            _csvCell(r.provider),
            _csvCell(r.providerId),
            _csvCell(r.modelName),
            _csvCell(r.functionType.value),
            _csvCell(r.source.value),
            _csvCell(r.status.value),
            r.stream,
            r.inputTokens,
            r.cachedInputTokens,
            r.cacheWriteTokens,
            r.outputTokens,
            r.reasoningTokens,
            r.waitDurationMs,
            r.outputDurationMs,
            r.totalDurationMs,
            resolvedCost?.cost.toStringAsFixed(6) ?? '',
            _csvCell(resolvedCost?.currency ?? ''),
            _csvCell(r.originNodeId),
            _csvCell(r.keyFingerprint),
          ].join(','),
        );
      }
      content = buffer.toString();
    } else {
      final payload = records.map((r) {
        final resolvedCost = pricing.costForRecord(r);
        return <String, Object?>{
          'id': r.id,
          'createdAtMs': r.createdAtMs,
          'provider': r.provider,
          'providerId': r.providerId,
          'modelName': r.modelName,
          'functionType': r.functionType.value,
          'source': r.source.value,
          'status': r.status.value,
          'stream': r.stream,
          'inputTokens': r.inputTokens,
          'cachedInputTokens': r.cachedInputTokens,
          'cacheWriteTokens': r.cacheWriteTokens,
          'outputTokens': r.outputTokens,
          'reasoningTokens': r.reasoningTokens,
          'waitDurationMs': r.waitDurationMs,
          'outputDurationMs': r.outputDurationMs,
          'totalDurationMs': r.totalDurationMs,
          'estimatedCost': resolvedCost?.cost,
          'currency': resolvedCost?.currency,
          'originNodeId': r.originNodeId,
          'keyFingerprint': r.keyFingerprint,
        };
      }).toList();
      content = const JsonEncoder.withIndent('  ').convert(payload);
    }
    await Clipboard.setData(ClipboardData(text: content));
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(l10n.settingsUsageExportCopied)),
    );
  }

  String _dateRangeLabel(AppLocalizations l10n) => switch (_selectedDateRange) {
    _UsageDateRange.lastDay => l10n.settingsDataDetailedStatsRangeLastDay,
    _UsageDateRange.last7Days => l10n.settingsDataDetailedStatsRangeLast7Days,
    _UsageDateRange.last30Days => l10n.settingsDataDetailedStatsRangeLast30Days,
    _UsageDateRange.all => l10n.settingsDataDetailedStatsRangeAll,
    _UsageDateRange.custom =>
      _customDateRange == null
          ? l10n.settingsDataDetailedStatsRangeCustom
          : '${_shortDate(_customDateRange!.start)} – ${_shortDate(_customDateRange!.end)}',
  };

  DateTimeRange? _effectiveDateRange(DateTime now) {
    switch (_selectedDateRange) {
      case _UsageDateRange.lastDay:
        return DateTimeRange(
          start: now.subtract(const Duration(hours: 24)),
          end: now,
        );
      case _UsageDateRange.last7Days:
        final today = DateTime(now.year, now.month, now.day);
        return DateTimeRange(
          start: today.subtract(const Duration(days: 6)),
          end: DateTime(now.year, now.month, now.day, 23, 59, 59, 999),
        );
      case _UsageDateRange.last30Days:
        final today = DateTime(now.year, now.month, now.day);
        return DateTimeRange(
          start: today.subtract(const Duration(days: 29)),
          end: DateTime(now.year, now.month, now.day, 23, 59, 59, 999),
        );
      case _UsageDateRange.all:
        return null;
      case _UsageDateRange.custom:
        final custom = _customDateRange;
        if (custom == null) return null;
        return DateTimeRange(
          start: DateTime(
            custom.start.year,
            custom.start.month,
            custom.start.day,
          ),
          end: DateTime(
            custom.end.year,
            custom.end.month,
            custom.end.day,
            23,
            59,
            59,
            999,
          ),
        );
    }
  }

  String _deviceScopeLabel(
    AppLocalizations l10n,
    Map<String, core_proxy.RuntimeDeviceSpaceDevice> devicesById,
    String? localDeviceId,
  ) {
    final selected = _selectedNodeId;
    if (selected == null) return l10n.settingsUsageDeviceScopeAll;
    if (selected == localDeviceId) return l10n.settingsUsageDeviceScopeLocal;
    final device = devicesById[selected];
    if (device != null && device.deviceName.trim().isNotEmpty) {
      return device.deviceName.trim();
    }
    return selected.length > 12 ? '${selected.substring(0, 8)}...' : selected;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;

    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.settingsUsageReportTitle),
        actions: <Widget>[
          IconButton(
            tooltip: l10n.refresh,
            onPressed: _reload,
            icon: const Icon(Icons.refresh),
          ),
          IconButton(
            tooltip: l10n.settingsDataResetTokenStats,
            onPressed: _resetting ? null : _resetStatistics,
            icon: const Icon(Icons.delete_sweep_outlined),
          ),
        ],
      ),
      body: FutureBuilder<_UsageStatisticsSnapshot>(
        future: _future,
        builder: (context, snapshot) {
          if (snapshot.connectionState == ConnectionState.waiting &&
              !snapshot.hasData) {
            return const Center(child: M3LoadingIndicator());
          }
          if (snapshot.hasError) {
            return Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    Text(
                      '${snapshot.error}',
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: 12),
                    FilledButton.icon(
                      onPressed: _reload,
                      icon: const Icon(Icons.refresh),
                      label: Text(l10n.refresh),
                    ),
                  ],
                ),
              ),
            );
          }

          final data = snapshot.data!;
          final spaceDevices =
              data.deviceSpace?.topology.devices ??
              const <core_proxy.RuntimeDeviceSpaceDevice>[];
          final localDeviceId = data.deviceSpace?.topology.currentDeviceId;
          final devicesById = <String, core_proxy.RuntimeDeviceSpaceDevice>{
            for (final d in spaceDevices) d.deviceId: d,
          };

          final now = DateTime.now();
          final dateRange = _effectiveDateRange(now);
          final viewData = _UsageStatisticsViewData.compute(
            records: data.records,
            pricing: data.pricing,
            dateRange: dateRange,
            selectedNodeId: _selectedNodeId,
            localDeviceId: localDeviceId,
            now: now,
          );

          return LayoutBuilder(
            builder: (context, constraints) {
              final compact = constraints.maxWidth < 720;
              final hPadding = compact
                  ? _dashboardCompactPadding
                  : _dashboardWidePadding;
              final availableWidth =
                  math.min(_dashboardMaxWidth, constraints.maxWidth) -
                  hPadding * 2;

              return Align(
                alignment: Alignment.topCenter,
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: _dashboardMaxWidth),
                  child: ListView(
                    padding: EdgeInsets.fromLTRB(hPadding, 12, hPadding, 28),
                    children: <Widget>[
                      _UsageStatisticsHeaderBar(
                        activeSection: _activeSection,
                        onSectionChanged: (section) =>
                            setState(() => _activeSection = section),
                        dateLabel: _dateRangeLabel(l10n),
                        onDateRangePressed: _selectDateRange,
                        sourceLabel: viewData.sourceSummaryLabel(l10n),
                        deviceScopeLabel: _deviceScopeLabel(
                          l10n,
                          devicesById,
                          localDeviceId,
                        ),
                        spaceDevices: spaceDevices,
                        localDeviceId: localDeviceId,
                        onDeviceScopeSelected: (nodeId) =>
                            setState(() => _selectedNodeId = nodeId),
                        exportEnabled:
                            _activeSection == _DashboardSection.analytics &&
                            viewData.filteredRecords.isNotEmpty,
                        onExportSelected: (csv) => _exportUsageData(
                          viewData.filteredRecords,
                          data.pricing,
                          asCsv: csv,
                        ),
                      ),
                      const SizedBox(height: _dashboardSectionSpacing),
                      AnimatedSwitcher(
                        duration: const Duration(milliseconds: 220),
                        switchInCurve: Curves.easeOutCubic,
                        switchOutCurve: Curves.easeInCubic,
                        child: _activeSection == _DashboardSection.analytics
                            ? Column(
                                key: const ValueKey('usage-section-analytics'),
                                crossAxisAlignment: CrossAxisAlignment.stretch,
                                children: _buildAnalyticsWidgets(
                                  context,
                                  l10n,
                                  data,
                                  viewData,
                                  devicesById,
                                  dateRange,
                                  compact: compact,
                                  wide: constraints.maxWidth >= 900,
                                  availableWidth: availableWidth,
                                ),
                              )
                            : KeyedSubtree(
                                key: const ValueKey('usage-section-quotas'),
                                child: _buildQuotaCenterBody(
                                  context,
                                  l10n,
                                  data,
                                  spaceDevices,
                                  localDeviceId,
                                ),
                              ),
                      ),
                    ],
                  ),
                ),
              );
            },
          );
        },
      ),
    );
  }

  List<Widget> _buildAnalyticsWidgets(
    BuildContext context,
    AppLocalizations l10n,
    _UsageStatisticsSnapshot snapshot,
    _UsageStatisticsViewData viewData,
    Map<String, core_proxy.RuntimeDeviceSpaceDevice> devicesById,
    DateTimeRange? dateRange, {
    required bool compact,
    required bool wide,
    required double availableWidth,
  }) {
    if (snapshot.records.isEmpty) {
      return <Widget>[
        _UsageStatisticsEmptyState(
          icon: Icons.query_stats_outlined,
          title: l10n.settingsDataDetailedStatsEmpty,
        ),
      ];
    }

    final colors = Theme.of(context).colorScheme;
    final avgSpeed = viewData.avgSpeedTokPerSec;
    final costValue = viewData.costDisplay(l10n);
    final cachePct = viewData.inputTokens > 0
        ? (viewData.cachedInputTokens / viewData.inputTokens * 100.0)
            .clamp(0.0, 100.0)
        : 0.0;
    final directInput = math.max(
      0,
      viewData.inputTokens - viewData.cachedInputTokens,
    );
    final successPct = viewData.requestCount > 0
        ? (viewData.completedCount / viewData.requestCount * 100.0)
            .clamp(0.0, 100.0)
        : 100.0;

    final kpiCards = <Widget>[
      BentoStaggeredEntrance(
        delay: Duration.zero,
        child: _MetricCard(
          key: const ValueKey('usage-kpi-tokens'),
          icon: Icons.bolt_outlined,
          label: l10n.settingsDataDetailedStatsTotalTokens,
          value: _formatCompactInt(viewData.totalTokens),
          accent: colors.primary,
          badgeText: viewData.cachedInputTokens > 0
              ? '${cachePct.toStringAsFixed(0)}%'
              : null,
          badgeColor: const Color(0xFF10B981),
          segments: [
            BentoBarSegment(
              value: directInput.toDouble(),
              color: colors.primary,
            ),
            BentoBarSegment(
              value: viewData.cachedInputTokens.toDouble(),
              color: const Color(0xFF10B981),
            ),
            BentoBarSegment(
              value: viewData.outputTokens.toDouble(),
              color: const Color(0xFFF59E0B),
            ),
          ],
          subtitle:
              '${l10n.settingsDataInputTokens} ${_formatCompactInt(viewData.inputTokens)} · ${l10n.settingsDataOutputTokens} ${_formatCompactInt(viewData.outputTokens)} · ${l10n.settingsDataDetailedStatsCachedInput} ${_formatCompactInt(viewData.cachedInputTokens)}',
        ),
      ),
      BentoStaggeredEntrance(
        delay: const Duration(milliseconds: 45),
        child: _MetricCard(
          key: const ValueKey('usage-kpi-requests'),
          icon: Icons.receipt_long_outlined,
          label: l10n.settingsDataDetailedStatsTotalRequests,
          value: _formatFullInt(viewData.requestCount),
          accent: colors.secondary,
          badgeText: viewData.requestCount > 0
              ? '${successPct.toStringAsFixed(0)}%'
              : null,
          badgeColor: viewData.issueCount > 0
              ? const Color(0xFFF59E0B)
              : const Color(0xFF10B981),
          segments: [
            BentoBarSegment(
              value: viewData.completedCount.toDouble(),
              color: const Color(0xFF10B981),
            ),
            BentoBarSegment(
              value: viewData.issueCount.toDouble(),
              color: colors.error,
            ),
          ],
          subtitle:
              '${l10n.settingsUsageKpiSuccessRate(viewData.completedCount, viewData.issueCount)} · ${l10n.settingsDataDetailedStatsActiveDays}: ${viewData.activeDays} · ${l10n.settingsDataDetailedStatsChats}: ${viewData.uniqueChats}',
        ),
      ),
      BentoStaggeredEntrance(
        delay: const Duration(milliseconds: 90),
        child: _MetricCard(
          key: const ValueKey('usage-kpi-speed'),
          icon: Icons.speed_outlined,
          label: l10n.settingsUsageKpiThroughput,
          value: avgSpeed == null ? '—' : '${avgSpeed.toStringAsFixed(1)} tok/s',
          accent: const Color(0xFF38BDF8),
          mutedValue: avgSpeed == null,
          badgeText: viewData.medianTtftMs != null
              ? formatLatencyDuration(viewData.medianTtftMs)
              : null,
          badgeColor: const Color(0xFF38BDF8),
          progress: avgSpeed == null ? 0.0 : (avgSpeed / 120.0).clamp(0.08, 1.0),
          subtitle: avgSpeed != null
              ? (viewData.medianTtftMs != null
                  ? l10n.settingsUsageKpiTtftMedian(
                      formatLatencyDuration(viewData.medianTtftMs),
                    )
                  : l10n.settingsUsageKpiSpeedSamples(
                      viewData.speedSampleCount,
                    ))
              : l10n.settingsUsageKpiStreamHint,
        ),
      ),
      BentoStaggeredEntrance(
        delay: const Duration(milliseconds: 135),
        child: _MetricCard(
          key: const ValueKey('usage-kpi-spend'),
          icon: Icons.payments_outlined,
          label: l10n.settingsUsageColSpend,
          value: costValue,
          accent: colors.tertiary,
          mutedValue: viewData.costByCurrency.isEmpty,
          badgeText: viewData.primaryCurrency,
          badgeColor: colors.tertiary,
          progress: viewData.costByCurrency.isEmpty ? 0.0 : 1.0,
          subtitle: snapshot.pricingLoadFailed
              ? l10n.settingsDataTimelinePricesFailed
              : viewData.costByCurrency.isEmpty
              ? l10n.settingsUsageKpiNoPriceHint
              : (viewData.reasoningTokens > 0
                  ? '${l10n.settingsUsageKpiReasoning}: ${_formatCompactInt(viewData.reasoningTokens)}'
                  : viewData.cacheWriteTokens > 0
                  ? '${l10n.settingsUsageKpiCacheWrite}: ${_formatCompactInt(viewData.cacheWriteTokens)}'
                  : l10n.settingsDataDetailedStatsRequestCountSummary(
                      viewData.pricedRequestCount,
                    )),
        ),
      ),
    ];

    const spacing = 12.0;

    return <Widget>[
      if (wide)
        IntrinsicHeight(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              for (var i = 0; i < kpiCards.length; i++) ...[
                Expanded(child: kpiCards[i]),
                if (i + 1 < kpiCards.length) const SizedBox(width: spacing),
              ],
            ],
          ),
        )
      else
        Column(
          children: [
            IntrinsicHeight(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(child: kpiCards[0]),
                  const SizedBox(width: spacing),
                  Expanded(child: kpiCards[1]),
                ],
              ),
            ),
            const SizedBox(height: spacing),
            IntrinsicHeight(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(child: kpiCards[2]),
                  const SizedBox(width: spacing),
                  Expanded(child: kpiCards[3]),
                ],
              ),
            ),
          ],
        ),
      const SizedBox(height: _dashboardSectionSpacing),
      if (viewData.filteredRecords.isEmpty)
        _UsageStatisticsEmptyState(
          icon: Icons.filter_alt_off_outlined,
          title: l10n.settingsDataDetailedStatsNoDataInRange,
        )
      else ...<Widget>[
        BentoStaggeredEntrance(
          delay: const Duration(milliseconds: 165),
          child: _UsageHeatmapCard(
            title: l10n.settingsDataDetailedStatsHeatmapTitle,
            subtitle: l10n.settingsDataDetailedStatsHeatmapTapHint,
            emptyLabel: l10n.settingsDataDetailedStatsNoDataInRange,
            dayPoints: viewData.dayPoints,
            primaryCurrency: viewData.primaryCurrency,
            now: viewData.computedAt,
            availableWidth: availableWidth,
          ),
        ),
        const SizedBox(height: _dashboardSectionSpacing),
        BentoStaggeredEntrance(
          delay: const Duration(milliseconds: 200),
          child: _ModelCompositionBentoCard(viewData: viewData, wide: wide),
        ),
        const SizedBox(height: _dashboardSectionSpacing),
        BentoStaggeredEntrance(
          delay: const Duration(milliseconds: 230),
          child: UsageTimeline(
            records: viewData.timelineRecords,
            dateRange: dateRange,
            pricingLoadFailed: snapshot.pricingLoadFailed,
          ),
        ),
        const SizedBox(height: _dashboardSectionSpacing),
        BentoStaggeredEntrance(
          delay: const Duration(milliseconds: 260),
          child: UsageAttributionSection(
            records: viewData.filteredRecords,
            pricing: snapshot.pricing,
            devicesById: devicesById,
            selectedCurrency: viewData.primaryCurrency,
          ),
        ),
      ],
    ];
  }

  Widget _buildQuotaCenterBody(
    BuildContext context,
    AppLocalizations l10n,
    _UsageStatisticsSnapshot snapshot,
    List<core_proxy.RuntimeDeviceSpaceDevice> spaceDevices,
    String? localDeviceId,
  ) {
    if (snapshot.quotaError != null || snapshot.quotaState == null) {
      return Container(
        padding: const EdgeInsets.all(20),
        decoration: BoxDecoration(
          color: Theme.of(context).colorScheme.errorContainer.withValues(
            alpha: 0.3,
          ),
          borderRadius: BorderRadius.circular(16),
        ),
        child: Column(
          children: <Widget>[
            Text(snapshot.quotaError ?? l10n.settingsQuotaStatusError),
            const SizedBox(height: 12),
            FilledButton.icon(
              onPressed: _reload,
              icon: const Icon(Icons.refresh),
              label: Text(l10n.refresh),
            ),
          ],
        ),
      );
    }

    return UsageQuotaCenterSection(
      clients: widget.clients,
      quotaState: snapshot.quotaState!,
      records: snapshot.records,
      pricing: snapshot.pricing,
      spaceDevices: spaceDevices,
      localDeviceId: localDeviceId,
      onStateUpdated: _reloadAndAwait,
    );
  }
}

class _ModelUsageSummary {
  const _ModelUsageSummary({
    required this.provider,
    required this.modelName,
    required this.tokens,
    required this.requests,
    required this.avgSpeedTokPerSec,
  });

  final String provider;
  final String modelName;
  final int tokens;
  final int requests;
  final double? avgSpeedTokPerSec;
}

class _UsageStatisticsViewData {
  const _UsageStatisticsViewData({
    required this.filteredRecords,
    required this.timelineRecords,
    required this.dayPoints,
    required this.topModels,
    required this.computedAt,
    required this.requestCount,
    required this.completedCount,
    required this.issueCount,
    required this.speedSampleCount,
    required this.pricedRequestCount,
    required this.inputTokens,
    required this.outputTokens,
    required this.cachedInputTokens,
    required this.cacheWriteTokens,
    required this.reasoningTokens,
    required this.avgSpeedTokPerSec,
    required this.medianTtftMs,
    required this.costByCurrency,
    required this.primaryCurrency,
    required this.activeDays,
    required this.uniqueChats,
    required this.allRecordCount,
  });

  final List<core_proxy.UsageRequestRecord> filteredRecords;
  final List<UsageTimelineRecord> timelineRecords;
  final List<_DayUsagePoint> dayPoints;
  final List<_ModelUsageSummary> topModels;
  final DateTime computedAt;
  final int requestCount;
  final int completedCount;
  final int issueCount;
  final int speedSampleCount;
  final int pricedRequestCount;
  final int inputTokens;
  final int outputTokens;
  final int cachedInputTokens;
  final int cacheWriteTokens;
  final int reasoningTokens;
  final double? avgSpeedTokPerSec;
  final int? medianTtftMs;
  final Map<String, double> costByCurrency;
  final String? primaryCurrency;
  final int activeDays;
  final int uniqueChats;
  final int allRecordCount;

  int get totalTokens => inputTokens + outputTokens;

  String sourceSummaryLabel(AppLocalizations l10n) =>
      '${l10n.settingsDataDetailedStatsSourceLabel}: ${_formatFullInt(filteredRecords.length)} / ${_formatFullInt(allRecordCount)}';

  String costDisplay(AppLocalizations l10n) {
    if (costByCurrency.isEmpty) return '—';
    final primary = primaryCurrency;
    if (primary != null && costByCurrency.containsKey(primary)) {
      return formatCurrencyWithCode(
        primary,
        costByCurrency[primary]!,
        decimals: 2,
      );
    }
    return costByCurrency.entries
        .map((e) => formatCurrencyWithCode(e.key, e.value, decimals: 2))
        .join(' · ');
  }

  factory _UsageStatisticsViewData.compute({
    required List<core_proxy.UsageRequestRecord> records,
    required UsagePricingIndex pricing,
    required DateTimeRange? dateRange,
    required String? selectedNodeId,
    required String? localDeviceId,
    required DateTime now,
  }) {
    final startMs = dateRange?.start.millisecondsSinceEpoch;
    final endMs = dateRange?.end.millisecondsSinceEpoch;

    final filtered = <core_proxy.UsageRequestRecord>[];
    final timelineRecords = <UsageTimelineRecord>[];
    final days = <DateTime, _DayUsageAccumulator>{};
    final chats = <String>{};
    final costByCurrency = <String, double>{};
    final waitSamples = <int>[];
    final modelBuckets = <String, List<core_proxy.UsageRequestRecord>>{};

    var completedCount = 0;
    var issueCount = 0;
    var speedSampleCount = 0;
    var pricedRequestCount = 0;
    var inputTokens = 0;
    var outputTokens = 0;
    var cachedInputTokens = 0;
    var cacheWriteTokens = 0;
    var reasoningTokens = 0;

    for (final record in records) {
      if (selectedNodeId != null) {
        final nodeId = record.originNodeId.trim();
        final matchesSelected =
            nodeId == selectedNodeId ||
            (selectedNodeId == localDeviceId && nodeId.isEmpty);
        if (!matchesSelected) continue;
      }
      if (startMs != null && record.createdAtMs < startMs) continue;
      if (endMs != null && record.createdAtMs > endMs) continue;

      filtered.add(record);
      final modelBucketKey = '${record.provider}::${record.modelName}';
      modelBuckets.putIfAbsent(modelBucketKey, () => []).add(record);

      final occurredAt = DateTime.fromMillisecondsSinceEpoch(
        record.createdAtMs,
      );
      final dayKey = DateTime(
        occurredAt.year,
        occurredAt.month,
        occurredAt.day,
      );
      if (record.chatId != null && record.chatId!.trim().isNotEmpty) {
        chats.add(record.chatId!.trim());
      }
      if (record.status == core_proxy.UsageRequestStatus.completed) {
        completedCount++;
      } else {
        issueCount++;
      }
      if (recordTokensPerSecond(record) != null) {
        speedSampleCount++;
      }
      inputTokens += record.inputTokens;
      outputTokens += record.outputTokens;
      cachedInputTokens += record.cachedInputTokens;
      cacheWriteTokens += record.cacheWriteTokens;
      reasoningTokens += record.reasoningTokens;
      if (record.waitDurationMs > 0) {
        waitSamples.add(record.waitDurationMs);
      }

      final snapCurrency = record.currency.trim().toUpperCase();
      final resolvedCost = pricing.costForRecord(record);
      if (resolvedCost != null) {
        pricedRequestCount++;
        costByCurrency.update(
          resolvedCost.currency,
          (prev) => prev + resolvedCost.cost,
          ifAbsent: () => resolvedCost.cost,
        );
      }
      days
          .putIfAbsent(dayKey, _DayUsageAccumulator.new)
          .add(
            input: record.inputTokens,
            output: record.outputTokens,
            currency: resolvedCost?.currency,
            cost: resolvedCost?.cost,
          );

      timelineRecords.add(
        UsageTimelineRecord(
          occurredAt: occurredAt,
          provider: record.provider,
          model: record.modelName,
          inputTokens: record.inputTokens,
          outputTokens: record.outputTokens,
          cachedInputTokens: record.cachedInputTokens,
          cacheWriteTokens: record.cacheWriteTokens,
          reasoningTokens: record.reasoningTokens,
          waitDurationMs: record.waitDurationMs,
          outputDurationMs: record.outputDurationMs,
          stream: record.stream,
          estimatedCostOverride:
              snapCurrency.isNotEmpty ? record.estimatedCost : null,
          currencyOverride: snapCurrency.isNotEmpty ? snapCurrency : null,
          pricing: pricing.forModel(record.provider, record.modelName),
        ),
      );
    }

    final dayPoints = days.entries
        .map(
          (entry) => _DayUsagePoint(
            date: entry.key,
            inputTokens: entry.value.inputTokens,
            outputTokens: entry.value.outputTokens,
            requestCount: entry.value.requestCount,
            costByCurrency: Map<String, double>.unmodifiable(
              entry.value.costByCurrency,
            ),
          ),
        )
        .toList()
      ..sort((a, b) => a.date.compareTo(b.date));

    final topModels = <_ModelUsageSummary>[];
    for (final bucket in modelBuckets.values) {
      final first = bucket.first;
      final sumTokens = bucket.fold<int>(
        0,
        (sum, r) => sum + r.inputTokens + r.outputTokens,
      );
      topModels.add(
        _ModelUsageSummary(
          provider: first.provider,
          modelName: first.modelName,
          tokens: sumTokens,
          requests: bucket.length,
          avgSpeedTokPerSec: weightedAverageTokensPerSecond(bucket),
        ),
      );
    }
    topModels.sort((a, b) => b.tokens.compareTo(a.tokens));

    final sortedCurrencies = costByCurrency.keys.toList()..sort();

    return _UsageStatisticsViewData(
      filteredRecords: filtered,
      timelineRecords: timelineRecords,
      dayPoints: dayPoints,
      topModels: topModels.take(4).toList(),
      computedAt: now,
      requestCount: filtered.length,
      completedCount: completedCount,
      issueCount: issueCount,
      speedSampleCount: speedSampleCount,
      pricedRequestCount: pricedRequestCount,
      inputTokens: inputTokens,
      outputTokens: outputTokens,
      cachedInputTokens: cachedInputTokens,
      cacheWriteTokens: cacheWriteTokens,
      reasoningTokens: reasoningTokens,
      avgSpeedTokPerSec: weightedAverageTokensPerSecond(filtered),
      medianTtftMs: medianLatencyMs(waitSamples),
      costByCurrency: costByCurrency,
      primaryCurrency: sortedCurrencies.firstOrNull,
      activeDays: days.length,
      uniqueChats: chats.length,
      allRecordCount: records.length,
    );
  }
}

class _DayUsageAccumulator {
  int inputTokens = 0;
  int outputTokens = 0;
  int requestCount = 0;
  final Map<String, double> costByCurrency = <String, double>{};

  void add({
    required int input,
    required int output,
    String? currency,
    double? cost,
  }) {
    inputTokens += input;
    outputTokens += output;
    requestCount++;
    if (currency != null && cost != null && cost > 0) {
      costByCurrency.update(
        currency,
        (prev) => prev + cost,
        ifAbsent: () => cost,
      );
    }
  }
}

class _DayUsagePoint {
  const _DayUsagePoint({
    required this.date,
    required this.inputTokens,
    required this.outputTokens,
    required this.requestCount,
    required this.costByCurrency,
  });

  final DateTime date;
  final int inputTokens;
  final int outputTokens;
  final int requestCount;
  final Map<String, double> costByCurrency;

  int get totalTokens => inputTokens + outputTokens;

  ({double cost, String? currency}) resolveCost(String? preferredCurrency) {
    if (costByCurrency.isEmpty) return (cost: 0.0, currency: preferredCurrency);
    if (preferredCurrency != null &&
        costByCurrency.containsKey(preferredCurrency)) {
      return (cost: costByCurrency[preferredCurrency]!, currency: preferredCurrency);
    }
    final first = costByCurrency.entries.first;
    return (cost: first.value, currency: first.key);
  }
}

class _HeatmapDay {
  const _HeatmapDay({
    required this.date,
    required this.totalTokens,
    required this.requestCount,
    required this.cost,
    required this.currency,
    required this.isActive,
  });

  final DateTime date;
  final int totalTokens;
  final int requestCount;
  final double cost;
  final String? currency;
  final bool isActive;

  double metricValue(_HeatmapMetric metric) => switch (metric) {
    _HeatmapMetric.tokens => totalTokens.toDouble(),
    _HeatmapMetric.requests => requestCount.toDouble(),
    _HeatmapMetric.cost => cost,
  };
}

class _UsageStatisticsEmptyState extends StatelessWidget {
  const _UsageStatisticsEmptyState({
    required this.icon,
    required this.title,
  });

  final IconData icon;
  final String title;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final text = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 48, horizontal: 16),
      child: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(icon, size: 42, color: colors.onSurfaceVariant),
            const SizedBox(height: 12),
            Text(
              title,
              textAlign: TextAlign.center,
              style: text.titleMedium?.copyWith(color: colors.onSurfaceVariant),
            ),
          ],
        ),
      ),
    );
  }
}

class _UsageStatisticsHeaderBar extends StatelessWidget {
  const _UsageStatisticsHeaderBar({
    required this.activeSection,
    required this.onSectionChanged,
    required this.dateLabel,
    required this.onDateRangePressed,
    required this.sourceLabel,
    required this.deviceScopeLabel,
    required this.spaceDevices,
    required this.localDeviceId,
    required this.onDeviceScopeSelected,
    required this.exportEnabled,
    required this.onExportSelected,
  });

  final _DashboardSection activeSection;
  final ValueChanged<_DashboardSection> onSectionChanged;
  final String dateLabel;
  final VoidCallback onDateRangePressed;
  final String sourceLabel;
  final String deviceScopeLabel;
  final List<core_proxy.RuntimeDeviceSpaceDevice> spaceDevices;
  final String? localDeviceId;
  final ValueChanged<String?> onDeviceScopeSelected;
  final bool exportEnabled;
  final ValueChanged<bool> onExportSelected;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;

    return BentoSurfaceCard(
      padding: const EdgeInsets.all(14),
      radius: 18,
      accent: colorScheme.primary,
      hoverable: false,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          LayoutBuilder(
            builder: (context, constraints) {
              final segmented = BentoSegmentedControl<_DashboardSection>(
                expand: constraints.maxWidth < 600,
                selected: activeSection,
                onSelected: onSectionChanged,
                options: [
                  BentoSegmentedOption<_DashboardSection>(
                    value: _DashboardSection.analytics,
                    itemKey: const ValueKey('usage-tab-analytics'),
                    icon: Icons.analytics_outlined,
                    label: l10n.settingsUsageTabAnalytics,
                  ),
                  BentoSegmentedOption<_DashboardSection>(
                    value: _DashboardSection.quotas,
                    itemKey: const ValueKey('usage-tab-quotas'),
                    icon: Icons.account_balance_wallet_outlined,
                    label: l10n.settingsUsageTabQuotas,
                  ),
                ],
              );
              final exportButton = exportEnabled
                  ? PopupMenuButton<bool>(
                      key: const ValueKey('usage-export-menu'),
                      onSelected: onExportSelected,
                      itemBuilder: (context) => <PopupMenuEntry<bool>>[
                        PopupMenuItem<bool>(
                          value: false,
                          child: Text(l10n.settingsUsageExportJson),
                        ),
                        PopupMenuItem<bool>(
                          value: true,
                          child: Text(l10n.settingsUsageExportCsv),
                        ),
                      ],
                      child: const BentoInteractivePill(
                        icon: Icons.download_outlined,
                        trailingIcon: Icons.expand_more,
                        label: 'JSON / CSV',
                      ),
                    )
                  : null;
              if (constraints.maxWidth >= 600 && exportButton != null) {
                return Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Flexible(child: segmented),
                    const SizedBox(width: 12),
                    exportButton,
                  ],
                );
              }
              return Wrap(
                spacing: 10,
                runSpacing: 8,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  segmented,
                  ?exportButton,
                ],
              );
            },
          ),
          if (activeSection == _DashboardSection.analytics) ...<Widget>[
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              runSpacing: 6,
              children: <Widget>[
                BentoInteractivePill(
                  icon: Icons.date_range_outlined,
                  trailingIcon: Icons.expand_more,
                  label: dateLabel,
                  onTap: onDateRangePressed,
                ),
                if (spaceDevices.isNotEmpty)
                  PopupMenuButton<String>(
                    key: const ValueKey('usage-device-scope-menu'),
                    onSelected: (value) =>
                        onDeviceScopeSelected(value.isEmpty ? null : value),
                    itemBuilder: (context) => <PopupMenuEntry<String>>[
                      PopupMenuItem<String>(
                        value: '',
                        child: Text(l10n.settingsUsageDeviceScopeAll),
                      ),
                      for (final device in spaceDevices)
                        PopupMenuItem<String>(
                          value: device.deviceId,
                          child: Text(
                            device.deviceId == localDeviceId
                                ? '${l10n.settingsUsageDeviceScopeLocal} · ${device.deviceName}'
                                : device.deviceName,
                          ),
                        ),
                    ],
                    child: BentoInteractivePill(
                      icon: Icons.devices_other_outlined,
                      trailingIcon: Icons.expand_more,
                      label: deviceScopeLabel,
                    ),
                  ),
                BentoInteractivePill(
                  icon: Icons.storage_outlined,
                  label: sourceLabel,
                ),
              ],
            ),
          ],
        ],
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

  @override
  State<_DateRangePickerDialog> createState() => _DateRangePickerDialogState();
}

class _DateRangePickerDialogState extends State<_DateRangePickerDialog> {
  late _UsageDateRange _range;
  late DateTimeRange _customRange;

  @override
  void initState() {
    super.initState();
    _range = widget.selectedRange;
    _customRange = widget.customRange;
  }

  Future<void> _pickEndpoint(_DateRangeEndpoint endpoint) async {
    final initial = endpoint == _DateRangeEndpoint.start
        ? _customRange.start
        : _customRange.end;
    final picked = await showDatePicker(
      context: context,
      initialDate: initial,
      firstDate: DateTime(2020),
      lastDate: DateTime.now().add(const Duration(days: 1)),
    );
    if (picked == null || !mounted) return;
    setState(() {
      _range = _UsageDateRange.custom;
      if (endpoint == _DateRangeEndpoint.start) {
        final end = picked.isAfter(_customRange.end) ? picked : _customRange.end;
        _customRange = DateTimeRange(start: picked, end: end);
      } else {
        final start =
            picked.isBefore(_customRange.start) ? picked : _customRange.start;
        _customRange = DateTimeRange(start: start, end: picked);
      }
    });
  }

  String _rangeLabel(AppLocalizations l10n, _UsageDateRange range) =>
      switch (range) {
        _UsageDateRange.lastDay => l10n.settingsDataDetailedStatsRangeLastDay,
        _UsageDateRange.last7Days =>
          l10n.settingsDataDetailedStatsRangeLast7Days,
        _UsageDateRange.last30Days =>
          l10n.settingsDataDetailedStatsRangeLast30Days,
        _UsageDateRange.all => l10n.settingsDataDetailedStatsRangeAll,
        _UsageDateRange.custom => l10n.settingsDataDetailedStatsRangeCustom,
      };

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.settingsDataDetailedStatsCustomRangePickerTitle),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            for (final option in _UsageDateRange.values)
              ListTile(
                dense: true,
                selected: _range == option,
                title: Text(_rangeLabel(l10n, option)),
                trailing:
                    _range == option ? const Icon(Icons.check, size: 18) : null,
                onTap: () => setState(() => _range = option),
              ),
            if (_range == _UsageDateRange.custom) ...<Widget>[
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: <Widget>[
                  OutlinedButton(
                    onPressed: () => _pickEndpoint(_DateRangeEndpoint.start),
                    child: Text(_shortDate(_customRange.start)),
                  ),
                  OutlinedButton(
                    onPressed: () => _pickEndpoint(_DateRangeEndpoint.end),
                    child: Text(_shortDate(_customRange.end)),
                  ),
                ],
              ),
            ],
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(
            _DateRangeSelection(
              range: _range,
              customRange: _range == _UsageDateRange.custom ? _customRange : null,
            ),
          ),
          child: Text(l10n.save),
        ),
      ],
    );
  }
}

class _UsageStatisticsSectionCard extends StatelessWidget {
  const _UsageStatisticsSectionCard({
    super.key,
    required this.title,
    required this.subtitle,
    required this.child,
    this.trailing,
    this.accent,
  });

  final String title;
  final String subtitle;
  final Widget child;
  final Widget? trailing;
  final Color? accent;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return BentoSurfaceCard(
      padding: const EdgeInsets.all(16),
      radius: 18,
      accent: accent ?? colorScheme.primary,
      hoverable: false,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      title,
                      style: textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      subtitle,
                      style: textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              if (trailing != null) ...[
                const SizedBox(width: 8),
                trailing!,
              ],
            ],
          ),
          const SizedBox(height: 14),
          child,
        ],
      ),
    );
  }
}

class _MetricCard extends StatelessWidget {
  const _MetricCard({
    super.key,
    required this.icon,
    required this.label,
    required this.value,
    required this.accent,
    required this.subtitle,
    this.badgeText,
    this.badgeColor,
    this.segments,
    this.progress,
    this.mutedValue = false,
  });

  final IconData icon;
  final String label;
  final String value;
  final Color accent;
  final String subtitle;
  final String? badgeText;
  final Color? badgeColor;
  final List<BentoBarSegment>? segments;
  final double? progress;
  final bool mutedValue;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return BentoSurfaceCard(
      padding: const EdgeInsets.fromLTRB(14, 12, 14, 12),
      radius: 16,
      accent: accent,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            children: <Widget>[
              Container(
                width: 26,
                height: 26,
                decoration: BoxDecoration(
                  color: accent.withValues(alpha: 0.14),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Icon(icon, size: 15, color: accent),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.labelLarge?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              if (badgeText != null && badgeText!.trim().isNotEmpty) ...[
                const SizedBox(width: 6),
                BentoDeltaBadge(
                  label: badgeText!,
                  color: badgeColor ?? accent,
                ),
              ],
            ],
          ),
          const SizedBox(height: 10),
          SizedBox(
            width: double.infinity,
            child: FittedBox(
              fit: BoxFit.scaleDown,
              alignment: Alignment.centerLeft,
              child: Text(
                value,
                maxLines: 1,
                style: tabularTextStyle(
                  textTheme.headlineSmall?.copyWith(
                    fontWeight: FontWeight.w800,
                    letterSpacing: -0.5,
                    color: mutedValue
                        ? colorScheme.onSurfaceVariant.withValues(alpha: 0.65)
                        : colorScheme.onSurface,
                  ),
                ),
              ),
            ),
          ),
          const SizedBox(height: 10),
          if (segments != null)
            BentoStackedBar(segments: segments!, height: 5)
          else if (progress != null)
            BentoAnimatedProgressBar(
              value: progress!,
              color: accent,
              height: 5,
            ),
          const SizedBox(height: 8),
          Text(
            subtitle,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: tabularTextStyle(
              textTheme.labelSmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
                height: 1.3,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _ModelCompositionBentoCard extends StatelessWidget {
  const _ModelCompositionBentoCard({
    required this.viewData,
    required this.wide,
  });

  final _UsageStatisticsViewData viewData;
  final bool wide;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colors = Theme.of(context).colorScheme;
    final text = Theme.of(context).textTheme;
    final grandTokens = math.max(1, viewData.totalTokens);
    final palette = [
      colors.primary,
      const Color(0xFF10B981),
      const Color(0xFFF59E0B),
      const Color(0xFFA855F7),
    ];

    final directInput = math.max(
      0,
      viewData.inputTokens - viewData.cachedInputTokens,
    );

    final modelsColumn = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (viewData.topModels.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 16),
            child: Center(
              child: Text(
                l10n.settingsUsageNoModelsInRange,
                style: text.bodySmall?.copyWith(color: colors.onSurfaceVariant),
              ),
            ),
          )
        else
          for (var i = 0; i < viewData.topModels.length; i++) ...[
            _buildModelRow(
              context,
              viewData.topModels[i],
              grandTokens,
              palette[i % palette.length],
            ),
            if (i + 1 < viewData.topModels.length) const SizedBox(height: 10),
          ],
      ],
    );

    final compositionColumn = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          l10n.settingsUsageTokenMixLabel,
          style: text.labelMedium?.copyWith(
            fontWeight: FontWeight.w700,
            color: colors.onSurface,
          ),
        ),
        const SizedBox(height: 10),
        BentoStackedBar(
          height: 9,
          segments: [
            BentoBarSegment(
              value: directInput.toDouble(),
              color: colors.primary,
            ),
            BentoBarSegment(
              value: viewData.cachedInputTokens.toDouble(),
              color: const Color(0xFF10B981),
            ),
            BentoBarSegment(
              value: viewData.cacheWriteTokens.toDouble(),
              color: const Color(0xFF38BDF8),
            ),
            BentoBarSegment(
              value: viewData.outputTokens.toDouble(),
              color: const Color(0xFFF59E0B),
            ),
            BentoBarSegment(
              value: viewData.reasoningTokens.toDouble(),
              color: const Color(0xFFA855F7),
            ),
          ],
        ),
        const SizedBox(height: 12),
        Wrap(
          spacing: 14,
          runSpacing: 8,
          children: [
            _mixLegendItem(
              context,
              color: colors.primary,
              label: l10n.settingsUsageDirectInputLabel,
              value: _formatCompactInt(directInput),
            ),
            _mixLegendItem(
              context,
              color: const Color(0xFF10B981),
              label: l10n.settingsDataDetailedStatsCachedInput,
              value: _formatCompactInt(viewData.cachedInputTokens),
            ),
            _mixLegendItem(
              context,
              color: const Color(0xFFF59E0B),
              label: l10n.settingsDataOutputTokens,
              value: _formatCompactInt(viewData.outputTokens),
            ),
            if (viewData.cacheWriteTokens > 0)
              _mixLegendItem(
                context,
                color: const Color(0xFF38BDF8),
                label: l10n.settingsUsageKpiCacheWrite,
                value: _formatCompactInt(viewData.cacheWriteTokens),
              ),
            if (viewData.reasoningTokens > 0)
              _mixLegendItem(
                context,
                color: const Color(0xFFA855F7),
                label: l10n.settingsUsageKpiReasoning,
                value: _formatCompactInt(viewData.reasoningTokens),
              ),
          ],
        ),
      ],
    );

    return _UsageStatisticsSectionCard(
      key: const ValueKey('usage-model-composition-card'),
      title: l10n.settingsUsageTopModelsTitle,
      subtitle:
          '${_formatCompactInt(viewData.totalTokens)} tok · ${l10n.settingsDataDetailedStatsRequestCountSummary(viewData.requestCount)}',
      accent: const Color(0xFF10B981),
      child: wide
          ? IntrinsicHeight(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(flex: 6, child: modelsColumn),
                  VerticalDivider(
                    width: 28,
                    thickness: 1,
                    color: colors.outlineVariant.withValues(alpha: 0.25),
                  ),
                  Expanded(flex: 5, child: compositionColumn),
                ],
              ),
            )
          : Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                modelsColumn,
                const SizedBox(height: 14),
                Divider(
                  height: 1,
                  color: colors.outlineVariant.withValues(alpha: 0.25),
                ),
                const SizedBox(height: 12),
                compositionColumn,
              ],
            ),
    );
  }

  Widget _buildModelRow(
    BuildContext context,
    _ModelUsageSummary item,
    int grandTokens,
    Color accent,
  ) {
    final colors = Theme.of(context).colorScheme;
    final text = Theme.of(context).textTheme;
    final share = (item.tokens / grandTokens).clamp(0.0, 1.0);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            ProviderLogo(
              providerTypeId: item.provider,
              fallbackName: item.provider,
              size: 20,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                '${item.provider} · ${item.modelName}',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: text.labelMedium?.copyWith(fontWeight: FontWeight.w600),
              ),
            ),
            const SizedBox(width: 8),
            if (item.avgSpeedTokPerSec != null) ...[
              Text(
                '${item.avgSpeedTokPerSec!.toStringAsFixed(1)} t/s',
                style: tabularTextStyle(
                  text.labelSmall?.copyWith(color: colors.onSurfaceVariant),
                ),
              ),
              const SizedBox(width: 8),
            ],
            Text(
              '${_formatCompactInt(item.tokens)} (${(share * 100).toStringAsFixed(0)}%)',
              style: tabularTextStyle(
                text.labelSmall?.copyWith(
                  color: accent,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 5),
        BentoAnimatedProgressBar(
          value: share,
          color: accent,
          height: 5,
        ),
      ],
    );
  }

  Widget _mixLegendItem(
    BuildContext context, {
    required Color color,
    required String label,
    required String value,
  }) {
    final colors = Theme.of(context).colorScheme;
    final text = Theme.of(context).textTheme;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: 7,
          height: 7,
          decoration: BoxDecoration(shape: BoxShape.circle, color: color),
        ),
        const SizedBox(width: 5),
        Text(
          '$label $value',
          style: tabularTextStyle(
            text.labelSmall?.copyWith(color: colors.onSurfaceVariant),
          ),
        ),
      ],
    );
  }
}

class _UsageHeatmapCard extends StatefulWidget {
  const _UsageHeatmapCard({
    required this.title,
    required this.subtitle,
    required this.emptyLabel,
    required this.dayPoints,
    required this.primaryCurrency,
    required this.now,
    required this.availableWidth,
  });

  final String title;
  final String subtitle;
  final String emptyLabel;
  final List<_DayUsagePoint> dayPoints;
  final String? primaryCurrency;
  final DateTime now;
  final double availableWidth;

  @override
  State<_UsageHeatmapCard> createState() => _UsageHeatmapCardState();
}

class _UsageHeatmapCardState extends State<_UsageHeatmapCard> {
  static const int _weeksCount = 53;
  static const double _weekdayAxisWidth = 30.0;
  static const double _monthHeaderHeight = 18.0;
  static const List<double> _levelAlphas = <double>[0.28, 0.48, 0.72, 0.96];

  _HeatmapMetric _metric = _HeatmapMetric.tokens;
  _HeatmapDay? _selectedDay;
  _HeatmapDay? _hoveredDay;

  @override
  void didUpdateWidget(covariant _UsageHeatmapCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    final selected = _selectedDay;
    if (selected != null) {
      final hasMatch = widget.dayPoints.any((p) => p.date == selected.date);
      if (!hasMatch) _selectedDay = null;
    }
    final hovered = _hoveredDay;
    if (hovered != null) {
      final hasMatch = widget.dayPoints.any((p) => p.date == hovered.date);
      if (!hasMatch) _hoveredDay = null;
    }
  }

  Color _accentFor(ColorScheme colors) => switch (_metric) {
    _HeatmapMetric.tokens => colors.primary,
    _HeatmapMetric.requests => const Color(0xFF10B981),
    _HeatmapMetric.cost => const Color(0xFFF59E0B),
  };

  String _detailLabel(AppLocalizations l10n) {
    final day = _hoveredDay ?? _selectedDay;
    if (day == null) return widget.subtitle;
    final parts = <String>[
      _shortDate(day.date),
      '${_formatCompactInt(day.totalTokens)} tok',
      l10n.settingsDataDetailedStatsRequestCountSummary(day.requestCount),
    ];
    if (day.cost > 0 && day.currency != null && day.currency!.isNotEmpty) {
      parts.add(formatCurrencyWithCode(day.currency!, day.cost, decimals: 3));
    }
    return parts.join(' · ');
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final l10n = AppLocalizations.of(context)!;
    final locale = Localizations.localeOf(context);
    final accent = _accentFor(colorScheme);

    final days = _buildHeatmapDays(
      widget.dayPoints,
      primaryCurrency: widget.primaryCurrency,
      now: widget.now,
      targetWeeks: _weeksCount,
    );
    final weeks = math.max(1, (days.length / 7).ceil());

    final gridAvailableWidth = math.max(
      120.0,
      widget.availableWidth - 34 - _weekdayAxisWidth - 6,
    );
    final rawPitch = gridAvailableWidth / weeks;
    final fitsContainer = rawPitch >= 14.5;
    final cellSize = fitsContainer ? (rawPitch * 0.77).clamp(11.5, 16.0) : 12.0;
    final cellGap = fitsContainer
        ? (rawPitch - cellSize).clamp(3.0, 4.4)
        : 3.5;

    final gridWidth = weeks * (cellSize + cellGap) - cellGap;
    final gridHeight = 7 * (cellSize + cellGap) - cellGap;
    final canvasHeight = _monthHeaderHeight + gridHeight;

    var maxValue = 0.0;
    for (final day in days) {
      if (day.isActive) {
        maxValue = math.max(maxValue, day.metricValue(_metric));
      }
    }

    final axisStyle = tabularTextStyle(
      textTheme.labelSmall?.copyWith(
        fontSize: 10.5,
        color: colorScheme.onSurfaceVariant.withValues(alpha: 0.78),
        fontWeight: FontWeight.w500,
      ),
    )!;

    final metricSwitcher = BentoSegmentedControl<_HeatmapMetric>(
      compact: true,
      selected: _metric,
      onSelected: (next) => setState(() => _metric = next),
      options: [
        BentoSegmentedOption<_HeatmapMetric>(
          value: _HeatmapMetric.tokens,
          itemKey: const ValueKey('usage-heatmap-metric-tokens'),
          label: l10n.settingsUsageHeatmapMetricTokens,
        ),
        BentoSegmentedOption<_HeatmapMetric>(
          value: _HeatmapMetric.requests,
          itemKey: const ValueKey('usage-heatmap-metric-requests'),
          label: l10n.settingsUsageHeatmapMetricRequests,
        ),
        BentoSegmentedOption<_HeatmapMetric>(
          value: _HeatmapMetric.cost,
          itemKey: const ValueKey('usage-heatmap-metric-cost'),
          label: l10n.settingsUsageHeatmapMetricCost,
        ),
      ],
    );

    final gridInteractive = MouseRegion(
      cursor: SystemMouseCursors.click,
      onHover: (event) {
        final hovered = _heatmapDayAtPosition(
          days,
          event.localPosition,
          cellSize: cellSize,
          cellGap: cellGap,
          topOffset: _monthHeaderHeight,
        );
        if (hovered?.date != _hoveredDay?.date) {
          setState(() => _hoveredDay = hovered);
        }
      },
      onExit: (_) {
        if (_hoveredDay != null) {
          setState(() => _hoveredDay = null);
        }
      },
      child: GestureDetector(
        onTapUp: (details) {
          final tapped = _heatmapDayAtPosition(
            days,
            details.localPosition,
            cellSize: cellSize,
            cellGap: cellGap,
            topOffset: _monthHeaderHeight,
          );
          setState(() {
            if (tapped != null && _selectedDay?.date == tapped.date) {
              _selectedDay = null;
            } else {
              _selectedDay = tapped;
            }
          });
        },
        child: SizedBox(
          width: gridWidth,
          height: canvasHeight,
          child: RepaintBoundary(
            child: CustomPaint(
              painter: _UsageHeatmapPainter(
                days: days,
                weeks: weeks,
                cellSize: cellSize,
                cellGap: cellGap,
                monthHeaderHeight: _monthHeaderHeight,
                colorScheme: colorScheme,
                accentColor: accent,
                metric: _metric,
                maxValue: maxValue,
                levelAlphas: _levelAlphas,
                selectedDay: _selectedDay,
                hoveredDay: _hoveredDay,
                axisStyle: axisStyle,
                textDirection: Directionality.of(context),
                locale: locale,
              ),
            ),
          ),
        ),
      ),
    );

    final compactHeader = widget.availableWidth < 540;

    return _UsageStatisticsSectionCard(
      title: widget.title,
      subtitle: widget.dayPoints.isEmpty
          ? widget.emptyLabel
          : l10n.settingsUsageHeatmapSubtitle(weeks),
      trailing: compactHeader ? null : metricSwitcher,
      accent: accent,
      child: widget.dayPoints.isEmpty
          ? Padding(
              padding: const EdgeInsets.all(16),
              child: Center(child: Text(widget.emptyLabel)),
            )
          : Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                if (compactHeader) ...[
                  Align(
                    alignment: Alignment.centerLeft,
                    child: metricSwitcher,
                  ),
                  const SizedBox(height: 12),
                ],
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SizedBox(
                      width: _weekdayAxisWidth,
                      height: canvasHeight,
                      child: CustomPaint(
                        painter: _WeekdayAxisPainter(
                          cellSize: cellSize,
                          cellGap: cellGap,
                          topOffset: _monthHeaderHeight,
                          style: axisStyle,
                          textDirection: Directionality.of(context),
                          labels: <int, String>{
                            0: l10n.settingsUsageWeekdayMon,
                            2: l10n.settingsUsageWeekdayWed,
                            4: l10n.settingsUsageWeekdayFri,
                            6: l10n.settingsUsageWeekdaySun,
                          },
                        ),
                      ),
                    ),
                    const SizedBox(width: 6),
                    Expanded(
                      child: fitsContainer
                          ? Align(
                              alignment: Alignment.centerLeft,
                              child: gridInteractive,
                            )
                          : SingleChildScrollView(
                              scrollDirection: Axis.horizontal,
                              reverse: true,
                              child: gridInteractive,
                            ),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                _UsageHeatmapLegend(
                  detailLabel: _detailLabel(l10n),
                  lowLabel: l10n.settingsDataDetailedStatsHeatmapLow,
                  highLabel: l10n.settingsDataDetailedStatsHeatmapHigh,
                  accentColor: accent,
                  levelAlphas: _levelAlphas,
                ),
              ],
            ),
    );
  }
}

class _UsageHeatmapLegend extends StatelessWidget {
  const _UsageHeatmapLegend({
    required this.detailLabel,
    required this.lowLabel,
    required this.highLabel,
    required this.accentColor,
    required this.levelAlphas,
  });

  final String detailLabel;
  final String lowLabel;
  final String highLabel;
  final Color accentColor;
  final List<double> levelAlphas;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isDark = colorScheme.brightness == Brightness.dark;
    final labelStyle = tabularTextStyle(
      Theme.of(
        context,
      ).textTheme.labelSmall?.copyWith(color: colorScheme.onSurfaceVariant),
    );
    final emptySquareColor = colorScheme.surfaceContainerHighest.withValues(
      alpha: isDark ? 0.55 : 0.72,
    );

    return Row(
      children: <Widget>[
        Expanded(
          child: Text(
            detailLabel,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: labelStyle,
          ),
        ),
        const SizedBox(width: 12),
        Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Text(lowLabel, style: labelStyle),
            const SizedBox(width: 6),
            Container(
              width: 10,
              height: 10,
              margin: const EdgeInsets.symmetric(horizontal: 1.5),
              decoration: BoxDecoration(
                color: emptySquareColor,
                borderRadius: BorderRadius.circular(2.5),
              ),
            ),
            for (final alpha in levelAlphas)
              Container(
                width: 10,
                height: 10,
                margin: const EdgeInsets.symmetric(horizontal: 1.5),
                decoration: BoxDecoration(
                  color: accentColor.withValues(alpha: alpha),
                  borderRadius: BorderRadius.circular(2.5),
                ),
              ),
            const SizedBox(width: 6),
            Text(highLabel, style: labelStyle),
          ],
        ),
      ],
    );
  }
}

class _WeekdayAxisPainter extends CustomPainter {
  const _WeekdayAxisPainter({
    required this.cellSize,
    required this.cellGap,
    required this.topOffset,
    required this.style,
    required this.textDirection,
    required this.labels,
  });

  final double cellSize;
  final double cellGap;
  final double topOffset;
  final TextStyle style;
  final TextDirection textDirection;
  final Map<int, String> labels;

  @override
  void paint(Canvas canvas, Size size) {
    final step = cellSize + cellGap;
    for (final entry in labels.entries) {
      final painter = TextPainter(
        text: TextSpan(text: entry.value, style: style),
        textDirection: textDirection,
        textAlign: TextAlign.right,
      )..layout(maxWidth: size.width);
      final y =
          topOffset + entry.key * step + (cellSize - painter.height) / 2;
      painter.paint(canvas, Offset(size.width - painter.width, y));
    }
  }

  @override
  bool shouldRepaint(covariant _WeekdayAxisPainter oldDelegate) =>
      oldDelegate.cellSize != cellSize ||
      oldDelegate.cellGap != cellGap ||
      oldDelegate.topOffset != topOffset ||
      oldDelegate.style != style ||
      oldDelegate.labels != labels;
}

class _UsageHeatmapPainter extends CustomPainter {
  const _UsageHeatmapPainter({
    required this.days,
    required this.weeks,
    required this.cellSize,
    required this.cellGap,
    required this.monthHeaderHeight,
    required this.colorScheme,
    required this.accentColor,
    required this.metric,
    required this.maxValue,
    required this.levelAlphas,
    required this.selectedDay,
    required this.hoveredDay,
    required this.axisStyle,
    required this.textDirection,
    required this.locale,
  });

  final List<_HeatmapDay> days;
  final int weeks;
  final double cellSize;
  final double cellGap;
  final double monthHeaderHeight;
  final ColorScheme colorScheme;
  final Color accentColor;
  final _HeatmapMetric metric;
  final double maxValue;
  final List<double> levelAlphas;
  final _HeatmapDay? selectedDay;
  final _HeatmapDay? hoveredDay;
  final TextStyle axisStyle;
  final TextDirection textDirection;
  final Locale locale;

  @override
  void paint(Canvas canvas, Size size) {
    final step = cellSize + cellGap;
    final isDark = colorScheme.brightness == Brightness.dark;
    final emptyColor = colorScheme.surfaceContainerHighest.withValues(
      alpha: isDark ? 0.55 : 0.72,
    );

    // Paint month labels aligned to column start of each month.
    var lastLabelRight = -100.0;
    for (var col = 0; col < weeks; col++) {
      final startIdx = col * 7;
      if (startIdx >= days.length) break;
      final endIdx = math.min(days.length - 1, startIdx + 6);
      DateTime? labelDate;
      if (col == 0 && days[startIdx].date.day <= 10) {
        labelDate = days[startIdx].date;
      } else {
        for (var idx = startIdx; idx <= endIdx; idx++) {
          if (days[idx].date.day == 1) {
            labelDate = days[idx].date;
            break;
          }
        }
      }
      if (labelDate == null) continue;
      final x = col * step;
      if (x - lastLabelRight < 28) continue;
      final label = _monthShortLabel(locale, labelDate.month);
      final painter = TextPainter(
        text: TextSpan(text: label, style: axisStyle),
        textDirection: textDirection,
      )..layout();
      if (x + painter.width <= size.width + 6) {
        painter.paint(canvas, Offset(x, 0));
        lastLabelRight = x + painter.width;
      }
    }

    final fill = Paint()..style = PaintingStyle.fill;
    final selectedBorder = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.6
      ..color = colorScheme.onSurface;
    final hoverBorder = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.2
      ..color = colorScheme.onSurface.withValues(alpha: 0.65);

    for (var i = 0; i < days.length; i++) {
      final day = days[i];
      if (!day.isActive) continue;
      final col = i ~/ 7;
      final row = i % 7;
      final rect = Rect.fromLTWH(
        col * step,
        monthHeaderHeight + row * step,
        cellSize,
        cellSize,
      );
      final value = day.metricValue(metric);
      if (value <= 0 || maxValue <= 0) {
        fill.color = emptyColor;
      } else {
        final norm = math.pow((value / maxValue).clamp(0.0, 1.0), 0.58);
        final level = norm >= 0.75
            ? 3
            : norm >= 0.50
            ? 2
            : norm >= 0.25
            ? 1
            : 0;
        fill.color = accentColor.withValues(alpha: levelAlphas[level]);
      }
      final rrect = RRect.fromRectAndRadius(rect, const Radius.circular(3));
      canvas.drawRRect(rrect, fill);
      if (selectedDay != null && selectedDay!.date == day.date) {
        canvas.drawRRect(rrect, selectedBorder);
      } else if (hoveredDay != null && hoveredDay!.date == day.date) {
        canvas.drawRRect(rrect, hoverBorder);
      }
    }
  }

  @override
  bool shouldRepaint(covariant _UsageHeatmapPainter oldDelegate) {
    return oldDelegate.days != days ||
        oldDelegate.cellSize != cellSize ||
        oldDelegate.cellGap != cellGap ||
        oldDelegate.accentColor != accentColor ||
        oldDelegate.metric != metric ||
        oldDelegate.maxValue != maxValue ||
        oldDelegate.selectedDay != selectedDay ||
        oldDelegate.hoveredDay != hoveredDay ||
        oldDelegate.locale != locale;
  }
}

String _monthShortLabel(Locale locale, int month) {
  if (locale.languageCode == 'zh') return '$month月';
  if (locale.languageCode == 'fr') {
    const frMonths = <String>[
      'janv.',
      'févr.',
      'mars',
      'avr.',
      'mai',
      'juin',
      'juil.',
      'août',
      'sept.',
      'oct.',
      'nov.',
      'déc.',
    ];
    return frMonths[month - 1];
  }
  const enMonths = <String>[
    'Jan',
    'Feb',
    'Mar',
    'Apr',
    'May',
    'Jun',
    'Jul',
    'Aug',
    'Sep',
    'Oct',
    'Nov',
    'Dec',
  ];
  return enMonths[month - 1];
}

List<_HeatmapDay> _buildHeatmapDays(
  List<_DayUsagePoint> points, {
  required String? primaryCurrency,
  required DateTime now,
  int targetWeeks = 53,
}) {
  if (points.isEmpty) return const <_HeatmapDay>[];
  final pointsByDay = <DateTime, _DayUsagePoint>{
    for (final p in points) p.date: p,
  };
  final today = DateTime(now.year, now.month, now.day);
  final latest = points.last.date;
  final endDay = latest.isAfter(today) ? latest : today;
  final weeksCount = math.max(12, targetWeeks);
  final currentWeekMonday = endDay.subtract(
    Duration(days: endDay.weekday - DateTime.monday),
  );
  final gridStart = currentWeekMonday.subtract(
    Duration(days: (weeksCount - 1) * 7),
  );
  final cellCount = weeksCount * 7;

  return List<_HeatmapDay>.generate(cellCount, (index) {
    final day = gridStart.add(Duration(days: index));
    final active = !day.isAfter(endDay);
    final point = pointsByDay[day];
    final resolved = point?.resolveCost(primaryCurrency);
    return _HeatmapDay(
      date: day,
      totalTokens: point?.totalTokens ?? 0,
      requestCount: point?.requestCount ?? 0,
      cost: resolved?.cost ?? 0.0,
      currency: resolved?.currency,
      isActive: active,
    );
  });
}

_HeatmapDay? _heatmapDayAtPosition(
  List<_HeatmapDay> days,
  Offset localPosition, {
  required double cellSize,
  required double cellGap,
  double topOffset = 0.0,
}) {
  final dy = localPosition.dy - topOffset;
  if (dy < 0 || localPosition.dx < 0) return null;
  final step = cellSize + cellGap;
  final col = (localPosition.dx / step).floor();
  final row = (dy / step).floor();
  if (col < 0 || row < 0 || row >= 7) return null;
  if ((localPosition.dx - col * step) > cellSize ||
      (dy - row * step) > cellSize) {
    return null;
  }
  final index = col * 7 + row;
  if (index < 0 || index >= days.length) return null;
  final day = days[index];
  return day.isActive ? day : null;
}

String _shortDate(DateTime date) =>
    '${date.year}-${date.month.toString().padLeft(2, '0')}-${date.day.toString().padLeft(2, '0')}';

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

String _formatFullInt(int value) => formatGroupedInt(value);

String _csvCell(String raw) {
  final escaped = raw.replaceAll('"', '""');
  return '"$escaped"';
}
