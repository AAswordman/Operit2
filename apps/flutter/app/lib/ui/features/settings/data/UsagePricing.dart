// ignore_for_file: file_names

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core;

/// Prices are current configured estimates, not historical provider invoices.
class UsageRecordCost {
  const UsageRecordCost({
    required this.currency,
    required this.cost,
    required this.fromSnapshot,
  });

  final String currency;
  final double cost;
  final bool fromSnapshot;
}

class UsageModelPrice {
  const UsageModelPrice({
    required this.currency,
    this.billingMode = 'TOKEN',
    this.inputPerMillion,
    this.cachedPerMillion,
    this.cacheWritePerMillion,
    this.outputPerMillion,
    this.perRequest,
  });

  final String currency;
  final String billingMode;
  final double? inputPerMillion;
  final double? cachedPerMillion;
  final double? cacheWritePerMillion;
  final double? outputPerMillion;
  final double? perRequest;

  double? estimate({
    required int input,
    required int cached,
    int cacheWrite = 0,
    required int output,
  }) {
    bool valid(double? value) => value != null && value.isFinite && value >= 0;
    if (currency.trim().isEmpty) return null;
    if (billingMode == 'COUNT') return valid(perRequest) ? perRequest : null;
    if (billingMode != 'TOKEN') return null;
    final hits = cached.clamp(0, input);
    final misses = input - hits;
    final cachePrice = cachedPerMillion ?? inputPerMillion;
    final writePrice = cacheWritePerMillion ?? 0;
    if ((misses > 0 && !valid(inputPerMillion)) ||
        (hits > 0 && !valid(cachePrice)) ||
        (output > 0 && !valid(outputPerMillion))) {
      return null;
    }
    // A zero-token request still requires a configured price to count as priced.
    if (!valid(inputPerMillion) && !valid(outputPerMillion)) return null;
    return (misses * (inputPerMillion ?? 0) +
            hits * (cachePrice ?? 0) +
            cacheWrite.clamp(0, 1 << 60) * writePrice +
            output * (outputPerMillion ?? 0)) /
        1000000;
  }

  double? estimateCacheSavings({
    required int input,
    required int cached,
    required int output,
  }) {
    if (estimate(input: input, cached: cached, output: output) == null) {
      return null;
    }
    final hits = cached.clamp(0, input);
    if (billingMode == 'COUNT' || hits == 0) return 0;
    if (inputPerMillion == null ||
        !inputPerMillion!.isFinite ||
        inputPerMillion! < 0) {
      return null;
    }
    return hits *
        (inputPerMillion! - (cachedPerMillion ?? inputPerMillion!)) /
        1000000;
  }

  bool samePrice(UsageModelPrice other) =>
      currency == other.currency &&
      billingMode == other.billingMode &&
      inputPerMillion == other.inputPerMillion &&
      cachedPerMillion == other.cachedPerMillion &&
      outputPerMillion == other.outputPerMillion &&
      perRequest == other.perRequest;
}

/// A request stores provider/model, so ambiguous configuration matches stay unknown.
class UsagePricingIndex {
  UsagePricingIndex({
    List<core.TokenStatsModel> saved = const [],
    List<core.ProviderModelSummary> configured = const [],
  }) {
    for (final model in configured) {
      final price = model.pricing;
      if (price == null) continue;
      final value = UsageModelPrice(
        currency: price.currency.value,
        billingMode: (price.billingMode?.toString() ?? 'TOKEN').toUpperCase(),
        inputPerMillion: price.inputPricePerMillion,
        cachedPerMillion: price.cachedInputPricePerMillion,
        cacheWritePerMillion: price.cacheWritePricePerMillion,
        outputPerMillion: price.outputPricePerMillion,
        perRequest: price.pricePerRequest,
      );
      for (final provider in {
        model.providerId,
        model.providerName,
        model.providerTypeId,
      }) {
        _add(provider, model.modelId, value);
      }
    }
    for (final model in saved) {
      final value = UsageModelPrice(
        currency: model.currency?.trim().toUpperCase() ?? '',
        billingMode: (model.billingMode ?? 'TOKEN').toUpperCase(),
        inputPerMillion: model.inputPricePerMillion,
        cachedPerMillion: model.cachedInputPricePerMillion,
        outputPerMillion: model.outputPricePerMillion,
        perRequest: model.pricePerRequest,
      );
      _add(model.provider, model.model, value);
    }
  }

  final _prices = <String, UsageModelPrice?>{};
  static String _key(String provider, String model) =>
      '${provider.trim().toLowerCase().length}:${provider.trim().toLowerCase()}${model.trim()}';

  void _add(String provider, String model, UsageModelPrice price) {
    final key = _key(provider, model);
    if (!_prices.containsKey(key)) {
      _prices[key] = price;
    } else if (_prices[key]?.samePrice(price) != true) {
      _prices[key] = null;
    }
  }

  UsageModelPrice? forModel(String provider, String model) =>
      _prices[_key(provider, model)];

  /// Returns the snapshotted request cost when stored, otherwise estimates from pricing rules.
  UsageRecordCost? costForRecord(core.UsageRequestRecord record) {
    final currency = record.currency.trim().toUpperCase();
    if (currency.isNotEmpty) {
      return UsageRecordCost(
        currency: currency,
        cost: record.estimatedCost,
        fromSnapshot: true,
      );
    }
    final price = forModel(record.provider, record.modelName);
    final calculated = price?.estimate(
      input: record.inputTokens,
      cached: record.cachedInputTokens,
      cacheWrite: record.cacheWriteTokens,
      output: record.outputTokens,
    );
    if (calculated == null || price == null) {
      return null;
    }
    return UsageRecordCost(
      currency: price.currency.trim().toUpperCase(),
      cost: calculated,
      fromSnapshot: false,
    );
  }
}

String formatUsageMoney(num amount, {bool detailed = false}) {
  if (amount != 0 && amount.abs() < 0.000001) {
    return amount.toStringAsExponential(2);
  }
  return amount.toStringAsFixed(
    detailed || (amount != 0 && amount.abs() < 0.0001) ? 6 : 4,
  );
}
