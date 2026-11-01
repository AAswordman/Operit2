import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/settings/data/UsageTimeline.dart';

void main() {
  final start = DateTime(2026, 11, 1, 10).millisecondsSinceEpoch;
  const minute = Duration.millisecondsPerMinute;
  UsageTimelineRecord record(
    int offset, {
    String provider = 'A',
    String model = 'model',
    int input = 10,
    int output = 2,
    int cached = 4,
  }) => UsageTimelineRecord(
    occurredAt: DateTime.fromMillisecondsSinceEpoch(start + offset),
    provider: provider,
    model: model,
    inputTokens: input,
    outputTokens: output,
    cachedInputTokens: cached,
  );
  UsageTimelineData data(
    List<UsageTimelineRecord> records, {
    UsageTimelineMetric metric = UsageTimelineMetric.tokens,
    int? end,
  }) => UsageTimelineData(
    records: records,
    start: start,
    end: end ?? start + 3 * minute,
    metric: metric,
  );

  test(
    'sums same-minute requests, fills idle minutes, and excludes end boundary',
    () {
      final result = data([
        record(1000),
        record(59000),
        record(2 * minute),
        record(-1),
        record(3 * minute),
      ]);
      expect(result.bucketMinutes, 1);
      expect(result.series.single.values, [24, 0, 12]);
      expect(result.requestCount, 3);
    },
  );

  test('keeps providers separate and preserves models without tokens', () {
    final result = data([
      record(0),
      record(0, provider: 'B'),
      record(minute, model: 'unknown', input: 0, output: 0),
    ]);
    expect(result.series.map((s) => s.label), [
      'A · model',
      'A · unknown',
      'B · model',
    ]);
    expect(result.series.map((s) => s.total), [12, 0, 12]);
    expect(result.requestCount, 3);
    expect(
      data([
        record(0, input: 0, output: 0),
      ], metric: UsageTimelineMetric.requests).series.single.total,
      1,
    );
  });

  test('input includes cached input once; metrics remain independent', () {
    final records = [record(0), record(1000)];
    expect(data(records).series.single.total, 24);
    expect(
      data(records, metric: UsageTimelineMetric.input).series.single.total,
      20,
    );
    expect(
      data(records, metric: UsageTimelineMetric.output).series.single.total,
      4,
    );
    expect(
      data(records, metric: UsageTimelineMetric.cached).series.single.total,
      8,
    );
    expect(
      data(records, metric: UsageTimelineMetric.requests).series.single.total,
      2,
    );
  });

  test(
    'day, week and multi-year windows adapt resolution and stay bounded',
    () {
      expect(
        data([
          record(0),
        ], end: start + const Duration(days: 1).inMilliseconds).bucketMinutes,
        15,
      );
      final week = data([
        record(0),
        record(2 * minute),
      ], end: start + const Duration(days: 7).inMilliseconds);
      expect(week.bucketMinutes, 120);
      expect(week.series.single.total, 24);
      final years = data([
        record(0),
      ], end: start + const Duration(days: 3650).inMilliseconds);
      expect(years.series.single.values.length, lessThanOrEqualTo(121));
      expect(years.series.single.total, 12);
    },
  );

  test('resolution follows the point budget without changing totals', () {
    UsageTimelineData window(int budget) => UsageTimelineData(
      records: [for (var i = 0; i < 360; i++) record(i * minute)],
      start: start + 1000,
      end: start + 360 * minute,
      metric: UsageTimelineMetric.tokens,
      targetBucketCount: budget,
    );
    final narrow = window(30);
    final wide = window(100);
    expect(narrow.bucketMs, greaterThan(wide.bucketMs));
    expect(narrow.series.single.values.length, lessThanOrEqualTo(31));
    expect(wide.series.single.values.length, lessThanOrEqualTo(101));
    expect(narrow.series.single.total, 359 * 12);
    expect(wide.series.single.total, narrow.series.single.total);
    expect(narrow.requestCount, wide.requestCount);
  });

  test(
    'close zoom uses real second-level timestamps and keeps idle buckets',
    () {
      final result = UsageTimelineData(
        records: [
          record(1000),
          record(9000),
          record(20000),
          record(5 * minute),
        ],
        start: start,
        end: start + 5 * minute,
        metric: UsageTimelineMetric.tokens,
        targetBucketCount: 30,
        minimumBucketDuration: const Duration(seconds: 5),
      );
      expect(result.bucketMs, 10000);
      expect(result.series.single.values.length, 30);
      expect(result.series.single.values.take(4), [24, 0, 12, 0]);
      expect(result.series.single.total, 36);
      expect(result.bucketAt(20 / 300), 2);
      expect(result.bucketAt(1), 29);
    },
  );

  test('partial-minute window clips records and selection at either edge', () {
    final result = UsageTimelineData(
      records: [
        record(1000),
        record(40000),
        record(minute + 20000),
        record(2 * minute),
      ],
      start: start + 30000,
      end: start + minute + 30000,
      metric: UsageTimelineMetric.tokens,
    );
    expect(result.series.single.values, [12, 12]);
    expect(result.bucketAt(-1), 0);
    expect(result.bucketAt(1), 1);
    expect(result.requestCount, 2);
  });

  test('idle recent window stays empty even when historical records exist', () {
    final result = data([record(-10 * minute)]);
    expect(result.requestCount, 0);
    expect(result.series, isEmpty);
  });
}
