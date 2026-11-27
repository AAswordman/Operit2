// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core;
import '../../../../l10n/generated/app_localizations.dart';
import '../model/ProviderLogo.dart';
import 'UsageBentoComponents.dart';
import 'UsageFormatters.dart';
import 'UsagePricing.dart';

enum UsageAttributionTab { models, sources, devices, logs }

enum UsageLogFilter { recent, peakTokens, issues, reasoning }

class UsageAttributionSection extends StatefulWidget {
  const UsageAttributionSection({
    super.key,
    required this.records,
    required this.pricing,
    required this.devicesById,
    required this.selectedCurrency,
  });

  final List<core.UsageRequestRecord> records;
  final UsagePricingIndex pricing;
  final Map<String, core.RuntimeDeviceSpaceDevice> devicesById;
  final String? selectedCurrency;

  @override
  State<UsageAttributionSection> createState() =>
      _UsageAttributionSectionState();
}

class _UsageAttributionSectionState extends State<UsageAttributionSection> {
  UsageAttributionTab _tab = UsageAttributionTab.models;
  UsageLogFilter _logFilter = UsageLogFilter.recent;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colors = Theme.of(context).colorScheme;
    final text = Theme.of(context).textTheme;

    return BentoSurfaceCard(
      padding: const EdgeInsets.all(16),
      radius: 18,
      accent: colors.secondary,
      hoverable: false,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          BentoSegmentedControl<UsageAttributionTab>(
            selected: _tab,
            onSelected: (tab) => setState(() => _tab = tab),
            options: [
              for (final tab in UsageAttributionTab.values)
                BentoSegmentedOption<UsageAttributionTab>(
                  value: tab,
                  itemKey: ValueKey('usage-attr-tab-${tab.name}'),
                  icon: switch (tab) {
                    UsageAttributionTab.models => Icons.hub_outlined,
                    UsageAttributionTab.sources => Icons.auto_awesome_outlined,
                    UsageAttributionTab.devices => Icons.devices_outlined,
                    UsageAttributionTab.logs => Icons.receipt_long_outlined,
                  },
                  label: _tabLabel(l10n, tab),
                ),
            ],
          ),
          const SizedBox(height: 14),
          AnimatedSwitcher(
            duration: const Duration(milliseconds: 200),
            switchInCurve: Curves.easeOutCubic,
            switchOutCurve: Curves.easeInCubic,
            child: KeyedSubtree(
              key: ValueKey('usage-attr-view-${_tab.name}'),
              child: switch (_tab) {
                UsageAttributionTab.models => _buildModelsView(
                  context,
                  l10n,
                  text,
                ),
                UsageAttributionTab.sources => _buildSourcesView(
                  context,
                  l10n,
                  text,
                ),
                UsageAttributionTab.devices => _buildDevicesView(
                  context,
                  l10n,
                  text,
                ),
                UsageAttributionTab.logs => _buildLogsView(context, l10n, text),
              },
            ),
          ),
        ],
      ),
    );
  }

  String _tabLabel(AppLocalizations l10n, UsageAttributionTab tab) => switch (tab) {
    UsageAttributionTab.models => l10n.settingsUsageAttrModels,
    UsageAttributionTab.sources => l10n.settingsUsageAttrSources,
    UsageAttributionTab.devices => l10n.settingsUsageAttrDevices,
    UsageAttributionTab.logs => l10n.settingsUsageAttrLogs,
  };

  Widget _buildModelsView(
    BuildContext context,
    AppLocalizations l10n,
    TextTheme text,
  ) {
    final groups = _groupRecords(
      widget.records,
      keyOf: (r) => '${r.provider}:${r.modelName}',
      titleOf: (r) => '${r.provider.isEmpty ? l10n.settingsDataDetailedStatsUnlabeledProvider : r.provider} · ${r.modelName.isEmpty ? l10n.settingsDataDetailedStatsUnlabeledModel : r.modelName}',
      providerOf: (r) => r.provider,
    );
    if (groups.isEmpty) {
      return _emptyPlaceholder(context, l10n.settingsDataDetailedStatsNoDataInRange);
    }
    final grandTokens = groups.fold<int>(0, (sum, g) => sum + g.totalTokens);
    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = constraints.maxWidth < 660;
        return Column(
          children: [
            for (final group in groups)
              _buildAttributionRowCard(
                context: context,
                l10n: l10n,
                text: text,
                compact: compact,
                leading: ProviderLogo(
                  providerTypeId: group.providerTypeId,
                  fallbackName: group.title,
                  size: 30,
                ),
                title: group.title,
                subtitle:
                    '${l10n.settingsDataDetailedStatsRequestCountSummary(group.records.length)} · In ${formatCompactTokenCount(group.inputTokens)} · Out ${formatCompactTokenCount(group.outputTokens)}${group.cachedTokens > 0 ? ' · Cache ${formatCompactTokenCount(group.cachedTokens)}' : ''}${group.reasoningTokens > 0 ? ' · ${l10n.settingsUsageKpiReasoning} ${formatCompactTokenCount(group.reasoningTokens)}' : ''}',
                group: group,
                grandTokens: grandTokens,
              ),
          ],
        );
      },
    );
  }

  Widget _buildSourcesView(
    BuildContext context,
    AppLocalizations l10n,
    TextTheme text,
  ) {
    final functionGroups = _groupRecords(
      widget.records,
      keyOf: (r) => r.functionType.value,
      titleOf: (r) => r.functionType.value,
      providerOf: (r) => r.provider,
    );
    final sourceGroups = _groupRecords(
      widget.records,
      keyOf: (r) => r.source.value,
      titleOf: (r) => usageSourceLabel(l10n, r.source),
      providerOf: (r) => r.provider,
    );
    if (functionGroups.isEmpty) {
      return _emptyPlaceholder(context, l10n.settingsDataDetailedStatsNoDataInRange);
    }
    final grandTokens = functionGroups.fold<int>(0, (s, g) => s + g.totalTokens);
    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = constraints.maxWidth < 660;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              l10n.settingsDataDetailedStatsFunctionModels,
              style: text.labelLarge?.copyWith(fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 8),
            for (final group in functionGroups)
              _buildAttributionRowCard(
                context: context,
                l10n: l10n,
                text: text,
                compact: compact,
                leading: _badgeIcon(context, Icons.auto_awesome_outlined),
                title: group.title,
                subtitle: l10n.settingsDataDetailedStatsRequestCountSummary(group.records.length),
                group: group,
                grandTokens: grandTokens,
              ),
            const SizedBox(height: 14),
            Text(
              l10n.settingsDataDetailedStatsSourceLabel,
              style: text.labelLarge?.copyWith(fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 8),
            for (final group in sourceGroups)
              _buildAttributionRowCard(
                context: context,
                l10n: l10n,
                text: text,
                compact: compact,
                leading: _badgeIcon(context, Icons.bolt_outlined),
                title: group.title,
                subtitle: l10n.settingsDataDetailedStatsRequestCountSummary(group.records.length),
                group: group,
                grandTokens: grandTokens,
              ),
          ],
        );
      },
    );
  }

  Widget _buildDevicesView(
    BuildContext context,
    AppLocalizations l10n,
    TextTheme text,
  ) {
    final groups = _groupRecords(
      widget.records,
      keyOf: (r) => r.originNodeId.trim(),
      titleOf: (r) {
        final id = r.originNodeId.trim();
        if (id.isEmpty) return l10n.settingsUsageDeviceUnknown;
        final device = widget.devicesById[id];
        if (device != null && device.deviceName.trim().isNotEmpty) {
          return device.deviceName.trim();
        }
        return id.length > 12 ? '${id.substring(0, 8)}...' : id;
      },
      providerOf: (r) => '',
    );
    if (groups.isEmpty) {
      return _emptyPlaceholder(context, l10n.settingsDataDetailedStatsNoDataInRange);
    }
    final grandTokens = groups.fold<int>(0, (s, g) => s + g.totalTokens);
    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = constraints.maxWidth < 660;
        return Column(
          children: [
            for (final group in groups)
              Builder(
                builder: (context) {
                  final device = widget.devicesById[group.key];
                  final statusLabel = device == null
                      ? l10n.settingsDataDetailedStatsRequestCountSummary(group.records.length)
                      : '${device.platform} · ${l10n.settingsDataDetailedStatsRequestCountSummary(group.records.length)}';
                  return _buildAttributionRowCard(
                    context: context,
                    l10n: l10n,
                    text: text,
                    compact: compact,
                    leading: _badgeIcon(
                      context,
                      device?.online == true ? Icons.devices_other : Icons.computer_outlined,
                    ),
                    title: group.title,
                    subtitle: statusLabel,
                    group: group,
                    grandTokens: grandTokens,
                  );
                },
              ),
          ],
        );
      },
    );
  }

  Widget _buildLogsView(
    BuildContext context,
    AppLocalizations l10n,
    TextTheme text,
  ) {
    final colors = Theme.of(context).colorScheme;
    final filtered = widget.records.where((record) {
      return switch (_logFilter) {
        UsageLogFilter.issues =>
          record.status != core.UsageRequestStatus.completed,
        UsageLogFilter.reasoning => record.reasoningTokens > 0,
        _ => true,
      };
    }).toList();
    switch (_logFilter) {
      case UsageLogFilter.recent:
      case UsageLogFilter.issues:
      case UsageLogFilter.reasoning:
        filtered.sort((a, b) => b.createdAtMs.compareTo(a.createdAtMs));
      case UsageLogFilter.peakTokens:
        filtered.sort(
          (a, b) => (b.inputTokens + b.outputTokens).compareTo(
            a.inputTokens + a.outputTokens,
          ),
        );
    }
    final shown = filtered.take(60).toList();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        BentoSegmentedControl<UsageLogFilter>(
          compact: true,
          selected: _logFilter,
          onSelected: (filter) => setState(() => _logFilter = filter),
          options: [
            for (final filter in UsageLogFilter.values)
              BentoSegmentedOption<UsageLogFilter>(
                value: filter,
                label: _logFilterLabel(l10n, filter),
              ),
          ],
        ),
        const SizedBox(height: 12),
        if (shown.isEmpty)
          _emptyPlaceholder(context, l10n.settingsDataDetailedStatsNoDataInRange)
        else
          for (final record in shown)
            Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: BentoSurfaceCard(
                padding: const EdgeInsets.all(12),
                radius: 13,
                onTap: () => _showRequestDetail(context, l10n, record),
                child: _buildLogRowContent(context, l10n, text, colors, record),
              ),
            ),
      ],
    );
  }

  Widget _buildLogRowContent(
    BuildContext context,
    AppLocalizations l10n,
    TextTheme text,
    ColorScheme colors,
    core.UsageRequestRecord record,
  ) {
    final speed = recordTokensPerSecond(record);
    final cost = widget.pricing.costForRecord(record);
    final statusColor = switch (record.status) {
      core.UsageRequestStatus.completed => colors.primary,
      core.UsageRequestStatus.cancelled => colors.tertiary,
      core.UsageRequestStatus.failed => colors.error,
    };
    final totalTok = record.inputTokens + record.outputTokens;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                '${record.provider} · ${record.modelName}',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: text.bodyMedium?.copyWith(fontWeight: FontWeight.w600),
              ),
            ),
            const SizedBox(width: 8),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
              decoration: BoxDecoration(
                color: statusColor.withValues(alpha: 0.12),
                borderRadius: BorderRadius.circular(999),
              ),
              child: Text(
                usageStatusLabel(l10n, record.status),
                style: text.labelSmall?.copyWith(
                  color: statusColor,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 6),
        Wrap(
          spacing: 10,
          runSpacing: 4,
          children: [
            Text(
              formatShortTimestamp(record.createdAtMs),
              style: text.labelSmall?.copyWith(color: colors.onSurfaceVariant),
            ),
            Text(
              '${formatCompactTokenCount(totalTok)} tok (${formatCompactTokenCount(record.inputTokens)}/${formatCompactTokenCount(record.outputTokens)})',
              style: text.labelSmall,
            ),
            if (speed != null)
              Text(
                '${speed.toStringAsFixed(1)} tok/s',
                style: text.labelSmall?.copyWith(
                  color: colors.primary,
                  fontWeight: FontWeight.w600,
                ),
              ),
            if (record.waitDurationMs > 0)
              Text(
                '${l10n.settingsUsageColTtft} ${formatLatencyDuration(record.waitDurationMs)}',
                style: text.labelSmall?.copyWith(color: colors.onSurfaceVariant),
              ),
            if (cost != null)
              Text(
                formatCurrencyWithCode(cost.currency, cost.cost, decimals: 4),
                style: text.labelSmall?.copyWith(fontWeight: FontWeight.w600),
              ),
            if (record.keyFingerprint.isNotEmpty)
              Text(
                record.keyFingerprint,
                style: text.labelSmall?.copyWith(color: colors.onSurfaceVariant),
              ),
          ],
        ),
      ],
    );
  }

  String _logFilterLabel(AppLocalizations l10n, UsageLogFilter filter) =>
      switch (filter) {
        UsageLogFilter.recent => l10n.settingsUsageLogFilterRecent,
        UsageLogFilter.peakTokens => l10n.settingsUsageLogFilterTopTokens,
        UsageLogFilter.issues => l10n.settingsUsageLogFilterIssues,
        UsageLogFilter.reasoning => l10n.settingsUsageLogFilterReasoning,
      };

  void _showRequestDetail(
    BuildContext context,
    AppLocalizations l10n,
    core.UsageRequestRecord record,
  ) {
    final speed = recordTokensPerSecond(record);
    final cost = widget.pricing.costForRecord(record);
    final deviceName = record.originNodeId.isEmpty
        ? l10n.settingsUsageDeviceUnknown
        : (widget.devicesById[record.originNodeId]?.deviceName ??
              record.originNodeId);
    showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      showDragHandle: true,
      builder: (context) => SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 4, 20, 20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                l10n.settingsUsageRequestDetailTitle,
                style: Theme.of(context)
                    .textTheme
                    .titleMedium
                    ?.copyWith(fontWeight: FontWeight.w700),
              ),
              const SizedBox(height: 12),
              _kvRow(context, l10n.model, '${record.provider}:${record.modelName}'),
              _kvRow(context, l10n.settingsDataDetailedStatsSourceLabel,
                  '${record.functionType.value} · ${usageSourceLabel(l10n, record.source)}'),
              _kvRow(context, l10n.settingsUsageColStatus,
                  '${usageStatusLabel(l10n, record.status)} · ${record.stream ? l10n.settingsUsageModeStream : l10n.settingsUsageModeNonStream}'),
              _kvRow(context, l10n.settingsUsageColTime, formatShortTimestamp(record.createdAtMs)),
              _kvRow(context, l10n.settingsDataInputTokens, formatGroupedInt(record.inputTokens)),
              _kvRow(context, l10n.settingsDataDetailedStatsCachedInput, formatGroupedInt(record.cachedInputTokens)),
              if (record.cacheWriteTokens > 0)
                _kvRow(context, l10n.settingsUsageKpiCacheWrite, formatGroupedInt(record.cacheWriteTokens)),
              _kvRow(context, l10n.settingsDataOutputTokens, formatGroupedInt(record.outputTokens)),
              if (record.reasoningTokens > 0)
                _kvRow(context, l10n.settingsUsageKpiReasoning, formatGroupedInt(record.reasoningTokens)),
              _kvRow(context, l10n.settingsUsageColTtft,
                  l10n.settingsUsageLatencyBreakdown(
                    formatLatencyDuration(record.waitDurationMs),
                    formatLatencyDuration(record.outputDurationMs),
                  )),
              if (speed != null)
                _kvRow(context, l10n.settingsUsageColSpeed, '${speed.toStringAsFixed(1)} tok/s'),
              if (cost != null)
                _kvRow(context, l10n.settingsUsageColSpend,
                    formatCurrencyWithCode(cost.currency, cost.cost, decimals: 6)),
              _kvRow(context, l10n.settingsUsageAttrDevices, deviceName),
              if (record.keyFingerprint.isNotEmpty)
                _kvRow(context, l10n.settingsModelApiKey, record.keyFingerprint),
            ],
          ),
        ),
      ),
    );
  }

  Widget _kvRow(BuildContext context, String label, String value) {
    final colors = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 120,
            child: Text(
              label,
              style: Theme.of(context)
                  .textTheme
                  .bodySmall
                  ?.copyWith(color: colors.onSurfaceVariant),
            ),
          ),
          Expanded(
            child: Text(
              value,
              style: Theme.of(context)
                  .textTheme
                  .bodySmall
                  ?.copyWith(fontWeight: FontWeight.w600),
            ),
          ),
        ],
      ),
    );
  }

  Widget _badgeIcon(BuildContext context, IconData icon) {
    final colors = Theme.of(context).colorScheme;
    return Container(
      width: 30,
      height: 30,
      decoration: BoxDecoration(
        color: colors.primaryContainer.withValues(alpha: 0.55),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Icon(icon, size: 17, color: colors.onPrimaryContainer),
    );
  }

  Widget _emptyPlaceholder(BuildContext context, String label) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 24),
      child: Center(
        child: Text(
          label,
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        ),
      ),
    );
  }

  Widget _buildAttributionRowCard({
    required BuildContext context,
    required AppLocalizations l10n,
    required TextTheme text,
    required bool compact,
    required Widget leading,
    required String title,
    required String subtitle,
    required _RecordGroup group,
    required int grandTokens,
  }) {
    final colors = Theme.of(context).colorScheme;
    final share = grandTokens > 0 ? group.totalTokens / grandTokens : 0.0;
    final costSummary = _formatGroupCosts(group.costByCurrency);

    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: BentoSurfaceCard(
        padding: const EdgeInsets.all(12),
        radius: 14,
        accent: colors.primary,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                leading,
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: text.bodyMedium?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      Text(
                        subtitle,
                        maxLines: compact ? 2 : 1,
                        overflow: TextOverflow.ellipsis,
                        style: tabularTextStyle(
                          text.labelSmall?.copyWith(
                            color: colors.onSurfaceVariant,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                Column(
                  crossAxisAlignment: CrossAxisAlignment.end,
                  children: [
                    Text(
                      '${formatCompactTokenCount(group.totalTokens)} tok',
                      style: tabularTextStyle(
                        text.bodyMedium?.copyWith(fontWeight: FontWeight.w700),
                      ),
                    ),
                    Text(
                      '${(share * 100).toStringAsFixed(1)}%',
                      style: tabularTextStyle(
                        text.labelSmall?.copyWith(
                          color: colors.primary,
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                    ),
                  ],
                ),
              ],
            ),
            const SizedBox(height: 8),
            BentoAnimatedProgressBar(
              value: share.clamp(0.0, 1.0),
              color: colors.primary,
              height: 6,
            ),
            if (group.avgSpeedTokPerSec != null ||
                group.medianTtftMs != null ||
                costSummary.isNotEmpty) ...[
              const SizedBox(height: 8),
              Wrap(
                spacing: 12,
                runSpacing: 4,
                children: [
                  if (group.avgSpeedTokPerSec != null)
                    Text(
                      '${l10n.settingsUsageColSpeed}: ${group.avgSpeedTokPerSec!.toStringAsFixed(1)} tok/s',
                      style: tabularTextStyle(
                        text.labelSmall?.copyWith(
                          color: colors.onSurfaceVariant,
                        ),
                      ),
                    ),
                  if (group.medianTtftMs != null)
                    Text(
                      '${l10n.settingsUsageColTtft}: ${formatLatencyDuration(group.medianTtftMs)}',
                      style: tabularTextStyle(
                        text.labelSmall?.copyWith(
                          color: colors.onSurfaceVariant,
                        ),
                      ),
                    ),
                  if (costSummary.isNotEmpty)
                    Text(
                      '${l10n.settingsUsageColSpend}: $costSummary',
                      style: tabularTextStyle(
                        text.labelSmall?.copyWith(
                          color: colors.primary,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }

  String _formatGroupCosts(Map<String, double> byCurrency) {
    if (byCurrency.isEmpty) return '';
    final preferred = widget.selectedCurrency;
    if (preferred != null && byCurrency.containsKey(preferred)) {
      return formatCurrencyWithCode(preferred, byCurrency[preferred]!, decimals: 4);
    }
    return byCurrency.entries
        .map((e) => formatCurrencyWithCode(e.key, e.value, decimals: 4))
        .join(' + ');
  }

  List<_RecordGroup> _groupRecords(
    List<core.UsageRequestRecord> records, {
    required String Function(core.UsageRequestRecord) keyOf,
    required String Function(core.UsageRequestRecord) titleOf,
    required String Function(core.UsageRequestRecord) providerOf,
  }) {
    final buckets = <String, List<core.UsageRequestRecord>>{};
    final titles = <String, String>{};
    final providers = <String, String>{};
    for (final record in records) {
      final key = keyOf(record);
      buckets.putIfAbsent(key, () => []).add(record);
      titles.putIfAbsent(key, () => titleOf(record));
      providers.putIfAbsent(key, () => providerOf(record));
    }
    final groups = <_RecordGroup>[];
    for (final entry in buckets.entries) {
      var input = 0;
      var output = 0;
      var cached = 0;
      var reasoning = 0;
      final costByCurrency = <String, double>{};
      for (final r in entry.value) {
        input += r.inputTokens;
        output += r.outputTokens;
        cached += r.cachedInputTokens;
        reasoning += r.reasoningTokens;
        final cost = widget.pricing.costForRecord(r);
        if (cost != null) {
          costByCurrency.update(
            cost.currency,
            (prev) => prev + cost.cost,
            ifAbsent: () => cost.cost,
          );
        }
      }
      groups.add(
        _RecordGroup(
          key: entry.key,
          title: titles[entry.key]!,
          providerTypeId: providers[entry.key]!,
          records: entry.value,
          inputTokens: input,
          outputTokens: output,
          cachedTokens: cached,
          reasoningTokens: reasoning,
          costByCurrency: costByCurrency,
          avgSpeedTokPerSec: weightedAverageTokensPerSecond(entry.value),
          medianTtftMs: medianLatencyMs(entry.value.map((r) => r.waitDurationMs)),
        ),
      );
    }
    groups.sort((a, b) => b.totalTokens.compareTo(a.totalTokens));
    return groups;
  }
}

class _RecordGroup {
  const _RecordGroup({
    required this.key,
    required this.title,
    required this.providerTypeId,
    required this.records,
    required this.inputTokens,
    required this.outputTokens,
    required this.cachedTokens,
    required this.reasoningTokens,
    required this.costByCurrency,
    required this.avgSpeedTokPerSec,
    required this.medianTtftMs,
  });

  final String key;
  final String title;
  final String providerTypeId;
  final List<core.UsageRequestRecord> records;
  final int inputTokens;
  final int outputTokens;
  final int cachedTokens;
  final int reasoningTokens;
  final Map<String, double> costByCurrency;
  final double? avgSpeedTokPerSec;
  final int? medianTtftMs;

  int get totalTokens => inputTokens + outputTokens;
}
