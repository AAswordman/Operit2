import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/ui/features/settings/data/UsagePricing.dart';
import 'package:operit2/ui/features/settings/data/UsageTimeline.dart';

void main() {
  const usd = UsageModelPrice(
    currency: 'USD',
    inputPerMillion: 2,
    cachedPerMillion: 0.2,
    outputPerMillion: 8,
  );
  final start = DateTime(2026, 11, 1, 10).millisecondsSinceEpoch;
  const minute = Duration.millisecondsPerMinute;
  UsageTimelineRecord record(
    int offset, {
    String model = 'model',
    int input = 100,
    int cached = 20,
    int output = 10,
    UsageModelPrice? price = usd,
  }) => UsageTimelineRecord(
    occurredAt: DateTime.fromMillisecondsSinceEpoch(start + offset),
    provider: 'A',
    model: model,
    inputTokens: input,
    outputTokens: output,
    cachedInputTokens: cached,
    pricing: price,
  );
  UsageTimelineData data(
    List<UsageTimelineRecord> records,
    UsageTimelineMetric metric, {
    String? currency,
  }) => UsageTimelineData(
    records: records,
    start: start,
    end: start + 3 * minute,
    metric: metric,
    currency: currency,
  );
  core.TokenStatsModel saved(String id, double rate) => core.TokenStatsModel(
    configId: id,
    provider: 'OpenAI',
    model: 'gpt-5',
    billingMode: 'TOKEN',
    currency: 'USD',
    inputPricePerMillion: rate,
    cachedInputPricePerMillion: null,
    cacheWritePricePerMillion: null,
    outputPricePerMillion: 8,
    pricePerRequest: null,
  );

  test(
    'estimates cached and uncached input separately without charging input twice',
    () {
      expect(
        usd.estimate(input: 165000, cached: 45000, output: 42000),
        closeTo(0.585, 1e-10),
      );
      const fallback = UsageModelPrice(
        currency: 'USD',
        inputPerMillion: 2,
        outputPerMillion: 8,
      );
      expect(
        fallback.estimate(input: 100, cached: 20, output: 10),
        closeTo(0.00028, 1e-10),
      );
    },
  );

  test(
    'cache savings use the regular input price and keep surcharges signed',
    () {
      expect(
        usd.estimateCacheSavings(input: 100, cached: 20, output: 10),
        closeTo(0.000036, 1e-12),
      );
      const expensiveCache = UsageModelPrice(
        currency: 'USD',
        inputPerMillion: 2,
        cachedPerMillion: 5,
        outputPerMillion: 8,
      );
      final result = data([
        record(0, price: expensiveCache),
      ], UsageTimelineMetric.cacheSavings);
      expect(result.amount, closeTo(-0.00006, 1e-12));
      expect(result.series.single.values.first, closeTo(-0.00006, 1e-12));
      const counted = UsageModelPrice(
        currency: 'USD',
        billingMode: 'COUNT',
        perRequest: 0.1,
      );
      expect(
        counted.estimateCacheSavings(input: 100, cached: 20, output: 10),
        0,
      );
      const unknownBaseline = UsageModelPrice(
        currency: 'USD',
        cachedPerMillion: 0.2,
        outputPerMillion: 8,
      );
      expect(
        unknownBaseline.estimateCacheSavings(
          input: 100,
          cached: 100,
          output: 10,
        ),
        isNull,
      );
    },
  );

  test('savings separate currencies and retain unknown-price gaps', () {
    const cny = UsageModelPrice(
      currency: 'CNY',
      inputPerMillion: 10,
      cachedPerMillion: 1,
      outputPerMillion: 20,
    );
    final result = data(
      [record(0), record(minute, price: null), record(2 * minute, price: cny)],
      UsageTimelineMetric.cacheSavings,
      currency: 'USD',
    );
    expect(result.amount, closeTo(0.000036, 1e-12));
    expect(result.unpricedRequests, 1);
    expect(result.series.single.values[1], isNull);
    expect(result.series.single.values[2], 0);
  });

  test(
    'average request input uses request counts and keeps idle buckets empty',
    () {
      final result = data([
        record(0, input: 100),
        record(1000, input: 300),
        record(minute, input: 0, cached: 0),
      ], UsageTimelineMetric.averageInput);
      expect(result.series.single.values, [200, 0, null]);
      expect(result.requestCount, 3);
      expect(
        result.inputByModel.values.single / result.requestCount,
        closeTo(400 / 3, 1e-9),
      );
    },
  );

  test('count billing charges once per request regardless of token count', () {
    const counted = UsageModelPrice(
      currency: 'CNY',
      billingMode: 'COUNT',
      perRequest: 0.125,
    );
    final result = data(
      [record(0, price: counted), record(1000, price: counted)],
      UsageTimelineMetric.spend,
      currency: 'CNY',
    );
    expect(result.amount, 0.25);
    expect(result.series.single.values, [0.25, 0, 0]);
  });

  test(
    'cache percentages use token weights in every bucket and across the window',
    () {
      final result = data([
        record(0, input: 1, cached: 1),
        record(1000, input: 99, cached: 0),
        record(minute, input: 100, cached: 99),
      ], UsageTimelineMetric.cacheRate);
      expect(result.series.single.values, [1.0, 99.0, null]);
      expect(
        result.cacheRateFor(result.series.map((series) => series.key)),
        50,
      );
      final zeros = data([
        record(0, input: 0, cached: 0),
      ], UsageTimelineMetric.cacheRate);
      expect(
        zeros.cacheRateFor(zeros.series.map((series) => series.key)),
        isNull,
      );
      expect(zeros.series.single.values, [null, null, null]);
    },
  );

  test(
    'zero hits remain 0%, idle minutes are gaps, and inconsistent cached counts are clamped',
    () {
      final result = data([
        record(0, cached: 0),
        record(2 * minute, cached: 1000),
      ], UsageTimelineMetric.cacheRate);
      expect(result.series.single.values, [0, null, 100]);
    },
  );

  test(
    'currencies are never added together, including when selection is omitted',
    () {
      const cny = UsageModelPrice(
        currency: 'CNY',
        billingMode: 'COUNT',
        perRequest: 1,
      );
      const dollars = UsageModelPrice(
        currency: 'USD',
        billingMode: 'COUNT',
        perRequest: 2,
      );
      final records = [
        record(0, model: 'yuan', price: cny),
        record(0, model: 'dollar', price: dollars),
      ];
      final yuan = data(records, UsageTimelineMetric.spend, currency: 'CNY');
      expect(yuan.amount, 1);
      expect(yuan.series.single.label, 'A · yuan');
      expect(
        data(records, UsageTimelineMetric.spend, currency: 'USD').amount,
        2,
      );
      expect(data(records, UsageTimelineMetric.spend).amount, 1);
    },
  );

  test(
    'unknown price is a gap, not a free request, while configured zero is valid',
    () {
      const free = UsageModelPrice(
        currency: 'USD',
        inputPerMillion: 0,
        outputPerMillion: 0,
      );
      final result = data(
        [
          record(0),
          record(minute, price: null),
          record(2 * minute, price: free),
        ],
        UsageTimelineMetric.spend,
        currency: 'USD',
      );
      expect(result.unpricedRequests, 1);
      expect(result.series.single.values[1], isNull);
      expect(result.series.single.values[2], 0);
      expect(result.amount, closeTo(0.000244, 1e-10));
      final unknown = data([record(0, price: null)], UsageTimelineMetric.spend);
      expect(unknown.series, isEmpty);
      expect(unknown.unpricedRequests, 1);
    },
  );

  test('invalid or incomplete prices are not reported as zero', () {
    const incomplete = UsageModelPrice(currency: 'USD', inputPerMillion: 2);
    expect(incomplete.estimate(input: 100, cached: 20, output: 10), isNull);
    const invalid = UsageModelPrice(
      currency: 'USD',
      inputPerMillion: -1,
      outputPerMillion: 2,
    );
    expect(invalid.estimate(input: 100, cached: 0, output: 10), isNull);
  });

  test(
    'imported prices match provider/model and conflicting configurations remain unknown',
    () {
      final index = UsagePricingIndex(saved: [saved('one', 2)]);
      expect(index.forModel('openai', 'gpt-5')!.inputPerMillion, 2);
      expect(index.forModel('another', 'gpt-5'), isNull);
      expect(
        UsagePricingIndex(
          saved: [saved('one', 2), saved('two', 3)],
        ).forModel('OpenAI', 'gpt-5'),
        isNull,
      );
    },
  );

  test(
    'current model prices match both profile names and runtime provider identifiers',
    () {
      const capabilities = core.ModelCapabilities(
        directImage: false,
        directAudio: false,
        directVideo: false,
        toolCall: true,
      );
      const pricing = core.ModelPricing(
        billingMode: 'TOKEN',
        inputPricePerMillion: 2,
        cachedInputPricePerMillion: 0.2,
        cacheWritePricePerMillion: null,
        outputPricePerMillion: 8,
        pricePerRequest: 0,
        currency: core.PricingCurrency.usd,
      );
      const model = core.ProviderModelSummary(
        providerId: 'profile-id',
        providerName: 'My OpenAI',
        providerTypeId: 'OPENAI',
        endpoint: '',
        modelId: 'gpt-5',
        capabilities: capabilities,
        pricing: pricing,
      );
      final index = UsagePricingIndex(configured: [model]);
      expect(index.forModel('OpenAI', 'gpt-5')!.cachedPerMillion, 0.2);
      expect(index.forModel('profile-id', 'gpt-5')!.inputPerMillion, 2);
      expect(index.forModel('My OpenAI', 'gpt-5')!.currency, 'USD');
    },
  );
}
