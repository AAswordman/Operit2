// ignore_for_file: file_names

import 'dart:math' as math;

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../components/StoreVisuals.dart';

/// Weights recent updates most strongly, with a bounded boost from feedback.
List<core_proxy.MarketEntrySummary> marketRecommendations(
  Iterable<core_proxy.MarketEntrySummary> entries, {
  DateTime? now,
  int limit = 12,
}) {
  final clock = now ?? DateTime.now();
  final seen = <(String, String)>{};
  final ranked = <({core_proxy.MarketEntrySummary entry, double score})>[];
  for (final entry in entries) {
    if (entry.id.isEmpty || !seen.add((entry.type, entry.id))) continue;
    final updated =
        DateTime.tryParse(entry.updatedAt) ??
        DateTime.tryParse(entry.publishedAt ?? '') ??
        DateTime.tryParse(entry.createdAt);
    final days = updated == null
        ? 365.0
        : math.max(
            0.0,
            clock.difference(updated).inMilliseconds /
                Duration.millisecondsPerDay,
          );
    final feedback =
        math.log(1 + math.max(0, marketEntryDownloads(entry))) +
        2 * math.log(1 + math.max(0, marketEntryLikes(entry)));
    ranked.add((
      entry: entry,
      // Seven-day half-life; lifetime feedback can boost a score by at most 50%.
      score:
          math.pow(0.5, days / 7).toDouble() *
          (1 + 0.5 * feedback / (feedback + 10)),
    ));
  }
  ranked.sort((a, b) {
    final score = b.score.compareTo(a.score);
    if (score != 0) return score;
    final type = a.entry.type.compareTo(b.entry.type);
    return type != 0 ? type : a.entry.id.compareTo(b.entry.id);
  });
  return ranked.take(math.max(0, limit)).map((item) => item.entry).toList();
}
