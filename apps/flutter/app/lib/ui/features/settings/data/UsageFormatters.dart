// ignore_for_file: file_names

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core;
import '../../../../l10n/generated/app_localizations.dart';

/// Applies Magpie's 100ms streaming minimum when deciding whether a request yields valid `tok/s`.
bool isSpeedSampleEligible(core.UsageRequestRecord record) {
  return record.stream && record.outputDurationMs >= 100 && record.outputTokens > 0;
}

/// Computes individual generation speed in tokens/second when `isSpeedSampleEligible` holds.
double? recordTokensPerSecond(core.UsageRequestRecord record) {
  if (!isSpeedSampleEligible(record)) return null;
  return record.outputTokens / (record.outputDurationMs / 1000.0);
}

/// Computes weighted average tokens/second across all eligible streamed requests.
double? weightedAverageTokensPerSecond(Iterable<core.UsageRequestRecord> records) {
  var totalOutput = 0;
  var totalMs = 0;
  for (final record in records) {
    if (!isSpeedSampleEligible(record)) continue;
    totalOutput += record.outputTokens;
    totalMs += record.outputDurationMs;
  }
  if (totalMs < 100 || totalOutput <= 0) return null;
  return totalOutput / (totalMs / 1000.0);
}

/// Computes the median integer from non-zero latency samples (`waitDurationMs`).
int? medianLatencyMs(Iterable<int> samples) {
  final valid = samples.where((ms) => ms > 0).toList()..sort();
  if (valid.isEmpty) return null;
  final mid = valid.length ~/ 2;
  if (valid.length.isOdd) return valid[mid];
  return ((valid[mid - 1] + valid[mid]) / 2).round();
}

String formatCompactTokenCount(int value) {
  if (value >= 1000000000) {
    return '${(value / 1000000000).toStringAsFixed(2)}B';
  }
  if (value >= 1000000) {
    return '${(value / 1000000).toStringAsFixed(2)}M';
  }
  if (value >= 1000) {
    return '${(value / 1000).toStringAsFixed(1)}k';
  }
  return formatGroupedInt(value);
}

String formatGroupedInt(int value) => value.toString().replaceAllMapped(
  RegExp(r'(\d)(?=(\d{3})+(?!\d))'),
  (match) => '${match[1]},',
);

String formatLatencyDuration(int? ms) {
  if (ms == null || ms <= 0) return '—';
  if (ms >= 10000) return '${(ms / 1000).toStringAsFixed(1)}s';
  if (ms >= 1000) return '${(ms / 1000).toStringAsFixed(2)}s';
  return '${ms}ms';
}

String formatCurrencyWithCode(String currency, num amount, {int decimals = 2}) {
  final code = currency.trim().toUpperCase();
  final val = amount.toDouble();
  final formatted = val != 0 && val.abs() < 0.0001
      ? val.toStringAsFixed(4)
      : val.toStringAsFixed(decimals);
  return switch (code) {
    'CNY' || 'RMB' => '¥$formatted',
    'USD' => '\$$formatted',
    '' => formatted,
    _ => '$code $formatted',
  };
}

String formatCountdownFromWindow(core.QuotaWindow window) {
  int? secs = window.resetSecs;
  if (secs == null && window.resetsAtMs != null) {
    final diff = ((window.resetsAtMs! - DateTime.now().millisecondsSinceEpoch) / 1000)
        .floor();
    secs = diff < 0 ? 0 : diff;
  }
  if (secs == null) return '';
  if (secs <= 0) return '0m';
  final days = secs ~/ 86400;
  final hours = (secs % 86400) ~/ 3600;
  final minutes = (secs % 3600) ~/ 60;
  if (days > 0) return '${days}d ${hours}h';
  if (hours > 0) return '${hours}h ${minutes}m';
  return '${minutes.clamp(1, 59)}m';
}

String formatShortTimestamp(int ms) {
  if (ms <= 0) return '—';
  final dt = DateTime.fromMillisecondsSinceEpoch(ms);
  final hh = dt.hour.toString().padLeft(2, '0');
  final mm = dt.minute.toString().padLeft(2, '0');
  return '${dt.month}/${dt.day} $hh:$mm';
}

String usageSourceLabel(AppLocalizations l10n, core.UsageRequestSource source) {
  return switch (source) {
    core.UsageRequestSource.chatResponse =>
      l10n.settingsDataDetailedStatsSourceChat,
    core.UsageRequestSource.toolResultResponse =>
      l10n.settingsDataDetailedStatsSourceToolResult,
    core.UsageRequestSource.summaryGeneration =>
      l10n.settingsDataDetailedStatsSourceSummary,
    core.UsageRequestSource.titleGeneration =>
      l10n.settingsDataDetailedStatsSourceTitleGeneration,
    core.UsageRequestSource.memoryAnalysis => l10n.settingsModelFunctionMemory,
  };
}

String usageStatusLabel(AppLocalizations l10n, core.UsageRequestStatus status) {
  return switch (status) {
    core.UsageRequestStatus.completed => l10n.settingsUsageStatusCompleted,
    core.UsageRequestStatus.cancelled => l10n.settingsUsageStatusCancelled,
    core.UsageRequestStatus.failed => l10n.settingsUsageStatusFailed,
  };
}
