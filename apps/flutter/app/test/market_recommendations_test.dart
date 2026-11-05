import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/features/packages/market/ArtifactMarketSupport.dart';
import 'package:operit2/ui/features/packages/market/MarketRecommendations.dart';

final _now = DateTime.utc(2026, 11, 5);

MarketEntrySummary _entry(
  String id, {
  int days = 1,
  int downloads = 0,
  int likes = 0,
  String type = 'package',
  String? createdAt,
  String? updatedAt,
}) => MarketEntrySummary.fromJson({
  'id': id,
  'type': type,
  'createdAt':
      createdAt ?? _now.subtract(Duration(days: days)).toIso8601String(),
  'updatedAt': updatedAt ?? _now.toIso8601String(),
  'downloads': downloads,
  'reactions': [
    {'reaction': '+1', 'total': likes},
  ],
});

void main() {
  test('recent well-received entries can outrank old cumulative leaders', () {
    final recent = _entry('recent', downloads: 100, likes: 10);
    final old = _entry('old', days: 365, downloads: 1000000, likes: 1000);
    expect(marketRecommendations([old, recent], now: _now), [recent, old]);
    final liked = _entry('liked', downloads: 100, likes: 20);
    expect(marketRecommendations([recent, liked], now: _now).first, liked);
  });

  test('every fourth slot can expose a new entry with no feedback yet', () {
    final entries = [
      for (var i = 0; i < 12; i++)
        _entry('established-$i', days: 30, downloads: 100000, likes: 1000),
      for (var i = 0; i < 3; i++) _entry('new-$i'),
    ];
    final result = marketRecommendations(entries, now: _now);
    expect(result.length, 12);
    expect(
      [result[3].id, result[7].id, result[11].id],
      ['new-0', 'new-1', 'new-2'],
    );
  });

  test('editing an old entry does not reset its publication age', () {
    final edited = _entry('edited', days: 365);
    final fresh = _entry('new');
    expect(marketRecommendations([edited, fresh], now: _now), [fresh, edited]);
    final unknownAge = _entry('unknown', createdAt: '');
    expect(marketRecommendations([unknownAge, fresh], now: _now), [
      fresh,
      unknownAge,
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
