// ignore_for_file: file_names

import 'dart:math' as math;

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../components/StoreVisuals.dart';

/// Ranks recent candidates and reserves every fourth slot for a new extension.
List<core_proxy.MarketEntrySummary> marketRecommendations(
  Iterable<core_proxy.MarketEntrySummary> entries, {
  DateTime? now,
  int limit = 12,
}) {
  final clock = now ?? DateTime.now();
  final seen = <(String, String)>{};
  final ranked =
      <({core_proxy.MarketEntrySummary entry, double score, bool fresh})>[];
  for (final entry in entries) {
    if (entry.id.isEmpty || !seen.add((entry.type, entry.id))) continue;
    // Updating an old listing must not reset its new-extension exposure.
    final published =
        DateTime.tryParse(entry.createdAt) ??
        DateTime.tryParse(entry.publishedAt ?? '');
    final days = published == null
        ? 365.0
        : math.max(
            0.0,
            clock.difference(published).inMilliseconds /
                Duration.millisecondsPerDay,
          );
    final popularity =
        math.log(1 + math.max(0, marketEntryDownloads(entry))) +
        2 * math.log(1 + math.max(0, marketEntryLikes(entry)));
    ranked.add((
      entry: entry,
      score:
          (1 + popularity + (entry.featured ? 2 : 0)) /
          math.sqrt(1 + days / 14),
      fresh: published != null && days < 14,
    ));
  }
  ranked.sort((a, b) {
    final score = b.score.compareTo(a.score);
    if (score != 0) return score;
    final type = a.entry.type.compareTo(b.entry.type);
    return type != 0 ? type : a.entry.id.compareTo(b.entry.id);
  });
  final result = <core_proxy.MarketEntrySummary>[];
  while (ranked.isNotEmpty && result.length < limit) {
    final fresh = result.length % 4 == 3
        ? ranked.indexWhere((candidate) => candidate.fresh)
        : 0;
    result.add(ranked.removeAt(math.max(0, fresh)).entry);
  }
  return result;
}
