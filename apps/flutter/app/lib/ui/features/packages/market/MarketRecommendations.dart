// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:pub_semver/pub_semver.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../components/StoreVisuals.dart';

final _preferredToolPkgApiVersion = Version(2, 0, 0);

/// Checks whether the declared API version meets the recommendation threshold.
bool _hasPreferredToolPkgApi(String? apiVersion) {
  if (apiVersion == null) return false;
  try {
    return Version.parse(apiVersion) >= _preferredToolPkgApiVersion;
  } on FormatException {
    return false;
  }
}

/// Weights recent updates with bounded feedback and ToolPkg API version boosts.
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
    final apiWeight = _hasPreferredToolPkgApi(entry.latestVersion?.apiVersion)
        ? 1.5
        : 1.0;
    ranked.add((
      entry: entry,
      // Seven-day half-life, up to 50% feedback boost, and 50% API version boost.
      score:
          math.pow(0.5, days / 7).toDouble() *
          (1 + 0.5 * feedback / (feedback + 10)) *
          apiWeight,
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
