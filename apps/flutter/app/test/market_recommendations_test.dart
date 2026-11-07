import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/features/packages/market/ArtifactMarketSupport.dart';
import 'package:operit2/ui/features/packages/market/MarketRecommendations.dart';

final _now = DateTime.utc(2026, 11, 5);

/// Creates market candidates with controlled ranking inputs.
MarketEntrySummary _entry(
  String id, {
  int days = 1,
  int downloads = 0,
  int likes = 0,
  String type = 'package',
  String? createdAt,
  String? updatedAt,
  String? apiVersion,
}) => MarketEntrySummary.fromJson({
  'id': id,
  'type': type,
  'createdAt':
      createdAt ?? _now.subtract(Duration(days: days)).toIso8601String(),
  'updatedAt':
      updatedAt ?? _now.subtract(Duration(days: days)).toIso8601String(),
  'downloads': downloads,
  if (apiVersion != null) 'latestVersion': {'apiVersion': apiVersion},
  'reactions': [
    {'reaction': '+1', 'total': likes},
  ],
});

void main() {
  for (final apiVersion in [
    '2.0.0',
    '2.0.0+build.1',
    '2.0.1',
    '2.10.0',
    '10.0.0',
  ]) {
    test('ToolPkg API $apiVersion receives a recommendation boost', () {
      final legacy = _entry('a-legacy', apiVersion: '1.99.99');
      final modern = _entry('z-modern', days: 2, apiVersion: apiVersion);
      expect(marketRecommendations([legacy, modern], now: _now), [
        modern,
        legacy,
      ]);
      expect(marketRecommendations([modern, legacy], now: _now), [
        modern,
        legacy,
      ]);
    });
  }

  for (final apiVersion in [null, '', 'invalid', '1.99.99', '2.0.0-beta.1']) {
    test('ToolPkg API $apiVersion does not receive a recommendation boost', () {
      final baseline = _entry('a-baseline');
      final candidate = _entry('z-candidate', apiVersion: apiVersion);
      expect(marketRecommendations([candidate, baseline], now: _now), [
        baseline,
        candidate,
      ]);
    });
  }

  test(
    'the boost uses the latest API version rather than package version or history',
    () {
      final modern = _entry('z-modern', apiVersion: '2.0.0');
      final legacy = MarketEntrySummary.fromJson({
        ..._entry('a-legacy').toJson(),
        'latestVersion': {'version': '10.0.0', 'apiVersion': '1.0.0'},
        'versions': [
          {'apiVersion': '3.0.0'},
        ],
      });
      expect(marketRecommendations([legacy, modern], now: _now), [
        modern,
        legacy,
      ]);
    },
  );

  test('API version boost preserves recency decay', () {
    final fresh = _entry('fresh', days: 0);
    final stale = _entry('stale', days: 30, apiVersion: '2.0.0');
    expect(marketRecommendations([stale, fresh], now: _now), [fresh, stale]);
  });

  test('new releases outrank historical leaders regardless of feedback', () {
    final recent = _entry('recent');
    final old = _entry('old', days: 365, downloads: 1000000, likes: 1000);
    expect(marketRecommendations([old, recent], now: _now), [recent, old]);
  });

  test('recent updates can bring established entries back into banners', () {
    final updated = _entry(
      'updated',
      days: 365,
      updatedAt: _now.toIso8601String(),
    );
    final fresh = _entry('new');
    expect(marketRecommendations([fresh, updated], now: _now), [
      updated,
      fresh,
    ]);
  });

  test('feedback weighs batch approvals without exact timestamp ties', () {
    final recent = _entry('recent', downloads: 100, likes: 10);
    final liked = _entry(
      'liked',
      downloads: 100,
      likes: 20,
      updatedAt: _now
          .subtract(const Duration(days: 1, minutes: 2))
          .toIso8601String(),
    );
    expect(marketRecommendations([recent, liked], now: _now), [liked, recent]);
    final entries = [for (var i = 0; i < 15; i++) _entry('entry-$i', days: i)];
    expect(
      marketRecommendations(entries, now: _now),
      entries.take(12).toList(),
    );
  });

  test('feedback can outweigh a small age gap but not a stale update', () {
    final fresh = _entry('fresh', days: 0);
    final popularUpdate = _entry(
      'popular-update',
      days: 2,
      downloads: 1000000,
      likes: 1000,
    );
    final stale = _entry(
      'stale',
      days: 7,
      downloads: 1000000000,
      likes: 1000000,
    );
    expect(marketRecommendations([fresh, stale, popularUpdate], now: _now), [
      popularUpdate,
      fresh,
      stale,
    ]);
  });

  test('deduplication uses type and id and ranking ties remain stable', () {
    final a = _entry('a');
    final b = _entry('b');
    final script = _entry('a', type: 'script');
    expect(marketRecommendations([b, a, a, script], now: _now), [a, b, script]);
    expect(marketRecommendations([script, a, b], now: _now), [a, b, script]);
  });

  test('invalid dates and negative counters do not break recommendation', () {
    final invalid = _entry(
      'invalid',
      createdAt: 'bad',
      updatedAt: '',
      downloads: -1,
      likes: -1,
    );
    final valid = _entry('valid');
    expect(marketRecommendations([invalid, valid], now: _now), [
      valid,
      invalid,
    ]);
    expect(marketRecommendations([], now: _now), isEmpty);
  });

  test('known categories have Chinese names and descriptions', () {
    for (final id in [
      'search_research',
      'dev_code',
      'automation_workflow',
      'docs_knowledge',
      'media_content',
      'chat_communication',
      'integration_api',
      'system_data',
      'business_productivity',
      'life_entertainment',
      'other',
    ]) {
      final category = MarketCategoryInfo(
        id: id,
        name: 'English',
        description: 'Description',
      );
      final text = marketCategoryText(category, languageCode: 'zh');
      expect(text.name, isNot('English'));
      expect(text.description, isNot('Description'));
      expect(marketCategoryText(category, languageCode: 'en'), (
        name: 'English',
        description: 'Description',
      ));
    }
  });

  test('unknown categories preserve server text in Chinese', () {
    const category = MarketCategoryInfo(
      id: 'future',
      name: 'Future',
      description: null,
    );
    expect(marketCategoryText(category, languageCode: 'zh'), (
      name: 'Future',
      description: '',
    ));
  });
}
