// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitGlassSurface.dart';
import 'UsagePricing.dart';
import 'UsageTimelineCurve.dart';

/// One request, kept at its original timestamp until a chart window is chosen.
class UsageTimelineRecord {
  const UsageTimelineRecord({
    required this.occurredAt,
    required this.provider,
    required this.model,
    required this.inputTokens,
    required this.outputTokens,
    required this.cachedInputTokens,
    this.pricing,
  });

  final DateTime occurredAt;
  final String provider;
  final String model;
  final int inputTokens;
  final int outputTokens;
  final int cachedInputTokens;
  final UsageModelPrice? pricing;

  double? get estimatedCost => pricing?.estimate(
    input: inputTokens,
    cached: cachedInputTokens,
    output: outputTokens,
  );
  double? get estimatedCacheSavings => pricing?.estimateCacheSavings(
    input: inputTokens,
    cached: cachedInputTokens,
    output: outputTokens,
  );
  String? get currency => estimatedCost == null ? null : pricing?.currency;

  // Length-prefixed keys avoid merging equal model names from different providers.
  String get modelKey => '${provider.length}:$provider$model';
  String get label => '$provider · $model';
}

enum UsageTimelineMetric {
  tokens,
  input,
  output,
  cached,
  requests,
  spend,
  cacheRate,
  averageInput,
  cacheSavings;

  bool get isMoney => this == spend || this == cacheSavings;
}

class UsageTimelineSeries {
  const UsageTimelineSeries({
    required this.key,
    required this.label,
    required this.values,
  });

  final String key;
  final String label;
  final List<num?> values;
  num get total => values.fold<num>(0, (sum, value) => sum + (value ?? 0));
}

/// Time buckets chosen for the visible window and available drawing width.
class UsageTimelineData {
  UsageTimelineData({
    required List<UsageTimelineRecord> records,
    required this.start,
    required this.end,
    required this.metric,
    String? currency,
    int targetBucketCount = 120,
    Duration minimumBucketDuration = const Duration(minutes: 1),
  }) {
    final target = targetBucketCount.clamp(1, 1440);
    final minimum = math.max(1000, minimumBucketDuration.inMilliseconds);
    final ideal = math.max(minimum, (end - start) / target);
    bucketMs =
        const [
              5,
              10,
              15,
              30,
              60,
              120,
              180,
              300,
              600,
              900,
              1800,
              3600,
              7200,
              10800,
              21600,
              43200,
              86400,
            ]
            .map((seconds) => seconds * 1000)
            .firstWhere(
              (milliseconds) => milliseconds >= ideal,
              orElse: () =>
                  (ideal / const Duration(days: 1).inMilliseconds).ceil() *
                  const Duration(days: 1).inMilliseconds,
            );
    bucketStart = (start ~/ bucketMs) * bucketMs;
    final count = ((end - bucketStart) / bucketMs).ceil();
    final models = <String, UsageTimelineRecord>{};
    final inWindow = records.where((record) {
      final at = record.occurredAt.millisecondsSinceEpoch;
      return at >= start && at < end;
    }).toList();
    for (final record in inWindow) {
      models[record.modelKey] = record;
    }
    final availableCurrencies =
        inWindow
            .map((record) => record.currency)
            .whereType<String>()
            .toSet()
            .toList()
          ..sort();
    this.currency = currency ?? availableCurrencies.firstOrNull;
    final ordered = models.values.toList()
      ..sort((a, b) => a.label.compareTo(b.label));
    final values = <String, List<num?>>{
      for (final model in ordered)
        model.modelKey: List<num?>.filled(
          count,
          metric == UsageTimelineMetric.cacheRate ||
                  metric == UsageTimelineMetric.averageInput
              ? null
              : 0,
        ),
    };
    final inputs = <String, List<int>>{
      for (final model in ordered) model.modelKey: List.filled(count, 0),
    };
    final hits = <String, List<int>>{
      for (final model in ordered) model.modelKey: List.filled(count, 0),
    };
    final counts = <String, List<int>>{
      if (metric == UsageTimelineMetric.averageInput)
        for (final model in ordered) model.modelKey: List.filled(count, 0),
    };
    for (final record in inWindow) {
      final at = record.occurredAt.millisecondsSinceEpoch;
      final index = (at - bucketStart) ~/ bucketMs;
      final row = values[record.modelKey]!;
      inputByModel.update(
        record.modelKey,
        (value) => value + record.inputTokens,
        ifAbsent: () => record.inputTokens,
      );
      final cached = record.cachedInputTokens.clamp(0, record.inputTokens);
      cachedByModel.update(
        record.modelKey,
        (value) => value + cached,
        ifAbsent: () => cached,
      );
      if (metric.isMoney) {
        final cost = metric == UsageTimelineMetric.cacheSavings
            ? record.estimatedCacheSavings
            : record.estimatedCost;
        if (cost == null) {
          unpricedRequests++;
          row[index] = null;
        } else if (record.currency == this.currency) {
          if (row[index] != null) row[index] = row[index]! + cost;
          amount += cost;
          amountByModel.update(
            record.modelKey,
            (value) => value + cost,
            ifAbsent: () => cost,
          );
        }
      } else if (metric == UsageTimelineMetric.averageInput) {
        inputs[record.modelKey]![index] += record.inputTokens;
        counts[record.modelKey]![index]++;
      } else if (metric == UsageTimelineMetric.cacheRate) {
        inputs[record.modelKey]![index] += record.inputTokens;
        hits[record.modelKey]![index] += record.cachedInputTokens.clamp(
          0,
          record.inputTokens,
        );
      } else {
        row[index] =
            row[index]! +
            switch (metric) {
              UsageTimelineMetric.tokens =>
                record.inputTokens + record.outputTokens,
              UsageTimelineMetric.input => record.inputTokens,
              UsageTimelineMetric.output => record.outputTokens,
              UsageTimelineMetric.cached => record.cachedInputTokens,
              UsageTimelineMetric.requests => 1,
              _ => 0,
            };
      }
      requestCount++;
    }
    for (final model in ordered) {
      if (metric == UsageTimelineMetric.averageInput) {
        for (var index = 0; index < count; index++) {
          final requests = counts[model.modelKey]![index];
          if (requests > 0) {
            values[model.modelKey]![index] =
                inputs[model.modelKey]![index] / requests;
          }
        }
      }
      if (metric == UsageTimelineMetric.cacheRate) {
        for (var index = 0; index < count; index++) {
          final input = inputs[model.modelKey]![index];
          if (input > 0) {
            values[model.modelKey]![index] =
                hits[model.modelKey]![index] / input * 100;
          }
        }
      }
    }
    series = [
      for (final model in ordered)
        if (!metric.isMoney || amountByModel.containsKey(model.modelKey))
          UsageTimelineSeries(
            key: model.modelKey,
            label: model.label,
            values: values[model.modelKey]!,
          ),
    ];
  }

  final int start;
  final int end;
  final UsageTimelineMetric metric;
  late final String? currency;
  final inputByModel = <String, int>{};
  final cachedByModel = <String, int>{};
  final amountByModel = <String, double>{};
  int unpricedRequests = 0;
  double amount = 0;

  double? cacheRateFor(Iterable<String> keys) {
    var input = 0;
    var cached = 0;
    for (final key in keys) {
      input += inputByModel[key] ?? 0;
      cached += cachedByModel[key] ?? 0;
    }
    return input == 0 ? null : cached / input * 100;
  }

  int get bucketMinutes => bucketMs ~/ _minuteMs;
  late final int bucketStart;
  late final List<UsageTimelineSeries> series;
  int requestCount = 0;
  late final int bucketMs;

  int bucketAt(double fraction) {
    final at = start + ((end - start) * fraction.clamp(0, 1)).floor();
    return ((at - bucketStart) ~/ bucketMs).clamp(
      0,
      series.isEmpty ? 0 : series.first.values.length - 1,
    );
  }
}

const _minuteMs = Duration.millisecondsPerMinute;
const _presets = [60, 180, 360, 1440, 10080, 0];

class UsageTimeline extends StatefulWidget {
  const UsageTimeline({
    super.key,
    required this.records,
    this.dateRange,
    this.pricingLoadFailed = false,
  });

  final List<UsageTimelineRecord> records;
  final DateTimeRange? dateRange;
  final bool pricingLoadFailed;

  @override
  State<UsageTimeline> createState() => _UsageTimelineState();
}

class _UsageTimelineState extends State<UsageTimeline> {
  UsageTimelineMetric _metric = UsageTimelineMetric.tokens;
  final _hidden = <String>{};
  final _colorIndices = <String, int>{};
  int? _preset = 180;
  late int _historyStart;
  late int _start;
  late int _end;
  int? _selected;
  String _selectedPlot = 'usage-timeline-plot';
  int _gestureStart = 0;
  int _gestureEnd = 0;
  double _gestureX = 0;
  UsageTimelineData? _cachedData;
  UsageTimelineData? _cachedSpend;
  UsageTimelineData? _cachedRate;
  UsageTimelineData? _cachedAverageInput;
  UsageTimelineData? _cachedSavings;
  String? _currency;
  int _targetBucketCount = 120;
  List<UsageTimelineRecord>? _insightRecords;
  List<String>? _insightCurrencies;

  int get _upper {
    final now =
        (DateTime.now().millisecondsSinceEpoch ~/ _minuteMs + 1) * _minuteMs;
    final range = widget.dateRange;
    if (range == null) return now;
    final end = range.end.millisecondsSinceEpoch;
    return range.start.millisecondsSinceEpoch < now ? math.min(end, now) : end;
  }

  int get _lower =>
      widget.dateRange?.start.millisecondsSinceEpoch ??
      math.min(_historyStart, _upper - const Duration(days: 1).inMilliseconds);

  @override
  void initState() {
    super.initState();
    if (widget.dateRange != null) _preset = 0;
    _syncRecords();
    _resetWindow();
  }

  @override
  void didUpdateWidget(covariant UsageTimeline oldWidget) {
    super.didUpdateWidget(oldWidget);
    _syncRecords();
    _cachedData = null;
    _cachedSpend = null;
    _cachedRate = null;
    _cachedAverageInput = null;
    _cachedSavings = null;
    _insightRecords = null;
    _insightCurrencies = null;
    if (oldWidget.dateRange != widget.dateRange) {
      _preset = widget.dateRange == null ? 180 : 0;
      _resetWindow();
    } else if (_preset != null) {
      _resetWindow();
    } else {
      _setWindow(_start, _end);
    }
  }

  void _syncRecords() {
    _historyStart = _upper;
    final keys = <String>{};
    for (final record in widget.records) {
      keys.add(record.modelKey);
      _historyStart = math.min(
        _historyStart,
        (record.occurredAt.millisecondsSinceEpoch ~/ _minuteMs) * _minuteMs,
      );
    }
    for (final key in keys.toList()..sort()) {
      _colorIndices.putIfAbsent(key, () => _colorIndices.length);
    }
  }

  Color _colorFor(UsageTimelineSeries series, ColorScheme colors) =>
      _seriesColor(_colorIndices[series.key]!, colors);

  void _resetWindow() {
    _end = _upper;
    _start = _preset == 0
        ? _lower
        : math.max(_lower, _end - _preset! * _minuteMs);
    _selected = null;
    _cachedData = null;
    _cachedSpend = null;
    _cachedRate = null;
    _cachedAverageInput = null;
    _cachedSavings = null;
    _insightRecords = null;
    _insightCurrencies = null;
  }

  void _setWindow(int start, int end) {
    final lower = _lower;
    final upper = _upper;
    final span = (end - start)
        .clamp(math.min(5 * _minuteMs, upper - lower), upper - lower)
        .toInt();
    _start = start.clamp(lower, upper - span).toInt();
    _end = _start + span;
    _preset = null;
    _selected = null;
    _cachedData = null;
    _cachedSpend = null;
    _cachedRate = null;
    _cachedAverageInput = null;
    _cachedSavings = null;
    _insightRecords = null;
    _insightCurrencies = null;
  }

  void _zoom(double factor, [double? anchor]) {
    final focal = anchor ?? (_end == _upper ? 1.0 : 0.5);
    final span = _end - _start;
    final nextSpan = (span * factor)
        .round()
        .clamp(5 * _minuteMs, math.max(5 * _minuteMs, _upper - _lower))
        .toInt();
    final start = _start + ((span - nextSpan) * focal).round();
    setState(() => _setWindow(start, start + nextSpan));
  }

  void _pan(int direction) {
    final offset = ((_end - _start) * 0.75 * direction).round();
    setState(() => _setWindow(_start + offset, _end + offset));
  }

  UsageTimelineData get _data => _cachedData ??= UsageTimelineData(
    records: widget.records,
    start: _start,
    end: _end,
    metric: _metric,
    targetBucketCount: _targetBucketCount,
    minimumBucketDuration: const Duration(seconds: 5),
  );

  Widget _buildInsights(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colors = Theme.of(context).colorScheme;
    final inWindow = _insightRecords ??= widget.records.where((record) {
      final at = record.occurredAt.millisecondsSinceEpoch;
      return at >= _start && at < _end && !_hidden.contains(record.modelKey);
    }).toList();
    final currencies = _insightCurrencies ??=
        inWindow
            .map((record) => record.currency)
            .whereType<String>()
            .toSet()
            .toList()
          ..sort();
    final currency = currencies.contains(_currency)
        ? _currency
        : currencies.firstOrNull;
    if (_cachedSpend?.currency != currency) _cachedSpend = null;
    if (_cachedSavings?.currency != currency) _cachedSavings = null;
    final spend = _cachedSpend ??= UsageTimelineData(
      records: inWindow,
      start: _start,
      end: _end,
      metric: UsageTimelineMetric.spend,
      currency: currency,
      targetBucketCount: _targetBucketCount,
      minimumBucketDuration: const Duration(seconds: 5),
    );
    final rate = _cachedRate ??= UsageTimelineData(
      records: inWindow,
      start: _start,
      end: _end,
      metric: UsageTimelineMetric.cacheRate,
      targetBucketCount: _targetBucketCount,
      minimumBucketDuration: const Duration(seconds: 5),
    );
    final averageInput = _cachedAverageInput ??= UsageTimelineData(
      records: inWindow,
      start: _start,
      end: _end,
      metric: UsageTimelineMetric.averageInput,
      targetBucketCount: _targetBucketCount,
      minimumBucketDuration: const Duration(seconds: 5),
    );
    final savings = _cachedSavings ??= UsageTimelineData(
      records: inWindow,
      start: _start,
      end: _end,
      metric: UsageTimelineMetric.cacheSavings,
      currency: currency,
      targetBucketCount: _targetBucketCount,
      minimumBucketDuration: const Duration(seconds: 5),
    );
    final keys = rate.series.map((series) => series.key);
    final hitRate = rate.cacheRateFor(keys);
    Widget chart({
      required String title,
      required String summary,
      required String hint,
      required UsageTimelineData data,
      Widget? trailing,
    }) {
      return Container(
        key: ValueKey('usage-timeline-${data.metric.name}-card'),
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          border: Border.all(
            color: colors.outlineVariant.withValues(alpha: 0.4),
          ),
          borderRadius: BorderRadius.circular(12),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Wrap(
              spacing: 12,
              runSpacing: 4,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                Text(title, style: Theme.of(context).textTheme.titleSmall),
                Text(
                  summary,
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
                ?trailing,
              ],
            ),
            const SizedBox(height: 6),
            Text(
              hint,
              style: Theme.of(
                context,
              ).textTheme.bodySmall?.copyWith(color: colors.onSurfaceVariant),
            ),
            _buildPlot(
              context,
              data,
              data.series,
              height: 220,
              plotKey: 'usage-timeline-${data.metric.name}-plot',
            ),
            if (_selected != null &&
                _selectedPlot == 'usage-timeline-${data.metric.name}-plot') ...[
              Text(
                '${_dateTime(math.max(_start, data.bucketStart + _selected! * data.bucketMs), seconds: data.bucketMs < _minuteMs)} — ${_dateTime(math.min(_end, data.bucketStart + (_selected! + 1) * data.bucketMs), seconds: data.bucketMs < _minuteMs)}',
                key: ValueKey('usage-timeline-${data.metric.name}-detail'),
                style: Theme.of(context).textTheme.labelSmall,
              ),
              for (final series in data.series)
                Text(
                  '${series.label}  ${_measure(series.values[_selected!], data.metric, data.currency)}',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: _colorFor(series, colors),
                  ),
                ),
            ],
          ],
        ),
      );
    }

    final spending = chart(
      title: l10n.settingsDataTimelineSpend,
      summary: spend.amountByModel.isEmpty
          ? '—'
          : '${currency ?? ''} ${formatUsageMoney(spend.amount)}',
      hint: widget.pricingLoadFailed
          ? l10n.settingsDataTimelinePricesFailed
          : '${l10n.settingsDataTimelineSpendHint}${spend.unpricedRequests > 0 ? ' · ${l10n.settingsDataTimelineUnpriced(spend.unpricedRequests)}' : ''}',
      data: spend,
      trailing: currencies.length < 2
          ? null
          : DropdownButton<String>(
              key: const ValueKey('usage-timeline-currency'),
              value: currency,
              items: [
                for (final code in currencies)
                  DropdownMenuItem(value: code, child: Text(code)),
              ],
              onChanged: (value) => setState(() {
                _currency = value;
                _cachedSpend = null;
                _cachedSavings = null;
              }),
            ),
    );
    final caching = chart(
      title: l10n.settingsDataTimelineCacheRate,
      summary: hitRate == null ? '—' : '${hitRate.toStringAsFixed(1)}%',
      hint: l10n.settingsDataTimelineCacheRateHint,
      data: rate,
    );
    final requestSize = chart(
      title: l10n.settingsDataTimelineAverageInput,
      summary: averageInput.requestCount == 0
          ? '—'
          : l10n.settingsDataTimelineTokensPerRequest(
              _compact(
                averageInput.inputByModel.values.fold<int>(
                      0,
                      (sum, input) => sum + input,
                    ) /
                    averageInput.requestCount,
              ),
            ),
      hint: l10n.settingsDataTimelineAverageInputHint,
      data: averageInput,
    );
    final cacheSavings = chart(
      title: l10n.settingsDataTimelineCacheSavings,
      summary: savings.amountByModel.isEmpty
          ? '—'
          : '${currency ?? ''} ${formatUsageMoney(savings.amount)}',
      hint: widget.pricingLoadFailed
          ? l10n.settingsDataTimelinePricesFailed
          : '${l10n.settingsDataTimelineCacheSavingsHint}${savings.unpricedRequests > 0 ? ' · ${l10n.settingsDataTimelineUnknownSavings(savings.unpricedRequests)}' : ''}',
      data: savings,
    );
    return LayoutBuilder(
      builder: (context, constraints) {
        final wide = constraints.maxWidth >= 1000;
        final width = wide
            ? (constraints.maxWidth - 12) / 2
            : constraints.maxWidth;
        return Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            SizedBox(width: width, child: spending),
            SizedBox(width: width, child: caching),
            SizedBox(width: width, child: requestSize),
            SizedBox(width: width, child: cacheSavings),
          ],
        );
      },
    );
  }

  Widget _buildPlot(
    BuildContext context,
    UsageTimelineData data,
    List<UsageTimelineSeries> visible, {
    double height = 260,
    String plotKey = 'usage-timeline-plot',
  }) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final l10n = AppLocalizations.of(context)!;
        final colors = Theme.of(context).colorScheme;
        final text = Theme.of(context).textTheme;
        final width = constraints.maxWidth;
        final bucketLabel = _bucketLabel(l10n, data);
        final plot = _plotRect(Size(width, height));
        void select(Offset position) {
          if (!plot.contains(position)) return;
          final index = data.bucketAt((position.dx - plot.left) / plot.width);
          if (index != _selected || _selectedPlot != plotKey) {
            setState(() {
              _selected = index;
              _selectedPlot = plotKey;
            });
          }
        }

        return Semantics(
          label: '${_metricLabel(l10n, data.metric)}, $bucketLabel',
          child: Listener(
            onPointerSignal: (event) {
              if (event is PointerScrollEvent &&
                  plot.contains(event.localPosition)) {
                GestureBinding.instance.pointerSignalResolver.register(event, (
                  _,
                ) {
                  _zoom(
                    event.scrollDelta.dy > 0 ? 1.3 : 1 / 1.3,
                    ((event.localPosition.dx - plot.left) / plot.width).clamp(
                      0,
                      1,
                    ),
                  );
                });
              }
            },
            child: MouseRegion(
              cursor: SystemMouseCursors.precise,
              onHover: (event) => select(event.localPosition),
              onExit: (_) {
                if (_selectedPlot == plotKey) setState(() => _selected = null);
              },
              child: GestureDetector(
                key: ValueKey(plotKey),
                behavior: HitTestBehavior.opaque,
                onTapDown: (event) => select(event.localPosition),
                onLongPressMoveUpdate: (event) => select(event.localPosition),
                onScaleStart: (event) {
                  _gestureStart = _start;
                  _gestureEnd = _end;
                  _gestureX = event.localFocalPoint.dx;
                },
                onScaleUpdate: (event) {
                  final oldSpan = _gestureEnd - _gestureStart;
                  final span = (oldSpan / event.scale.clamp(0.01, 100)).round();
                  final anchor = ((_gestureX - plot.left) / plot.width).clamp(
                    0,
                    1,
                  );
                  final shift =
                      (event.localFocalPoint.dx - _gestureX) /
                      plot.width *
                      span;
                  final start =
                      _gestureStart +
                      ((oldSpan - span) * anchor - shift).round();
                  setState(() => _setWindow(start, start + span));
                },
                child: SizedBox(
                  height: height,
                  width: double.infinity,
                  child: CustomPaint(
                    painter: _TimelinePainter(
                      data: data,
                      visible: visible,
                      colors: colors,
                      selected: _selected,
                      colorFor: (series) => _colorFor(series, colors),
                      textDirection: Directionality.of(context),
                      labelStyle: text.labelSmall!.copyWith(
                        color: colors.onSurfaceVariant,
                        fontSize: 10,
                      ),
                    ),
                    child: data.requestCount == 0 || visible.isEmpty
                        ? Center(
                            child: Container(
                              padding: const EdgeInsets.all(12),
                              color: colors.surface.withValues(alpha: 0.9),
                              child: Text(
                                data.requestCount == 0
                                    ? l10n.settingsDataDetailedStatsNoDataInRange
                                    : visible.isEmpty &&
                                          data.metric ==
                                              UsageTimelineMetric.spend &&
                                          data.unpricedRequests > 0
                                    ? l10n.settingsDataTimelineNoPrices
                                    : l10n.settingsDataTimelineSelectModel,
                              ),
                            ),
                          )
                        : null,
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final contentWidth = constraints.maxWidth - 32;
      // All three plots share buckets; use the narrower companion plot width.
      final plotWidth = contentWidth >= 1000
          ? (contentWidth - 12) / 2 - 24 - 56
          : contentWidth - 24 - 56;
      final target = (plotWidth / 8).floor().clamp(24, 160);
      if (target != _targetBucketCount) {
        _targetBucketCount = target;
        _selected = null;
        _cachedData = null;
        _cachedSpend = null;
        _cachedRate = null;
        _cachedAverageInput = null;
        _cachedSavings = null;
      }
      return _buildContent(context);
    },
  );

  Widget _buildContent(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colors = Theme.of(context).colorScheme;
    final text = Theme.of(context).textTheme;
    final data = _data;
    final visible = data.series
        .where((series) => !_hidden.contains(series.key))
        .toList();
    final total = visible.fold<num>(0, (sum, series) => sum + series.total);
    final bucketLabel = _bucketLabel(l10n, data);
    return OperitGlassSurface(
      color: colors.surfaceContainerHighest.withValues(alpha: 0.32),
      borderRadius: BorderRadius.circular(14),
      border: Border.all(color: colors.outlineVariant.withValues(alpha: 0.16)),
      material: true,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              l10n.settingsDataTimelineTitle,
              style: text.titleMedium?.copyWith(fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 6),
            Text(
              l10n.settingsDataTimelineHint,
              style: text.bodySmall?.copyWith(color: colors.onSurfaceVariant),
            ),
            const SizedBox(height: 16),
            Wrap(
              spacing: 6,
              runSpacing: 6,
              children: [
                for (final minutes in _presets)
                  ChoiceChip(
                    label: Text(
                      minutes == 0
                          ? l10n.settingsDataDetailedStatsRangeAll
                          : minutes == 10080
                          ? l10n.settingsDataDetailedStatsRangeLast7Days
                          : l10n.settingsDataTimelineHours(minutes ~/ 60),
                    ),
                    selected: _preset == minutes,
                    onSelected: (_) => setState(() {
                      _preset = minutes;
                      _resetWindow();
                    }),
                  ),
              ],
            ),
            const SizedBox(height: 8),
            Wrap(
              crossAxisAlignment: WrapCrossAlignment.center,
              spacing: 12,
              runSpacing: 4,
              children: [
                DropdownButton<UsageTimelineMetric>(
                  key: const ValueKey('usage-timeline-metric'),
                  value: _metric,
                  underline: const SizedBox.shrink(),
                  items: [
                    for (final metric in UsageTimelineMetric.values.where(
                      (metric) =>
                          !metric.isMoney &&
                          metric != UsageTimelineMetric.averageInput &&
                          metric != UsageTimelineMetric.cacheRate,
                    ))
                      DropdownMenuItem(
                        value: metric,
                        child: Text(_metricLabel(l10n, metric)),
                      ),
                  ],
                  onChanged: (metric) {
                    if (metric == null) return;
                    setState(() {
                      _metric = metric;
                      _cachedData = null;
                      _cachedSpend = null;
                      _cachedRate = null;
                      _cachedAverageInput = null;
                      _cachedSavings = null;
                      _insightRecords = null;
                      _insightCurrencies = null;
                    });
                  },
                ),
                Text(
                  '${_number(total)} · $bucketLabel',
                  style: text.bodySmall?.copyWith(
                    color: colors.onSurfaceVariant,
                  ),
                ),
              ],
            ),
            Text(
              '${_dateTime(_start, seconds: data.bucketMs < _minuteMs)} — ${_dateTime(_end, seconds: data.bucketMs < _minuteMs)}',
              style: text.bodySmall,
            ),
            const SizedBox(height: 12),
            _buildPlot(context, data, visible),
            IconButtonTheme(
              data: const IconButtonThemeData(
                style: ButtonStyle(visualDensity: VisualDensity.compact),
              ),
              child: Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 2,
                children: [
                  IconButton(
                    tooltip: l10n.settingsDataTimelineEarlier,
                    onPressed: _start <= _lower ? null : () => _pan(-1),
                    icon: const Icon(Icons.chevron_left),
                  ),
                  IconButton(
                    tooltip: l10n.zoomIn,
                    onPressed: _end - _start <= 5 * _minuteMs
                        ? null
                        : () => _zoom(0.5),
                    icon: const Icon(Icons.zoom_in),
                  ),
                  IconButton(
                    tooltip: l10n.zoomOut,
                    onPressed: _end - _start >= _upper - _lower
                        ? null
                        : () => _zoom(2),
                    icon: const Icon(Icons.zoom_out),
                  ),
                  IconButton(
                    tooltip: l10n.settingsDataTimelineLater,
                    onPressed: _end >= _upper ? null : () => _pan(1),
                    icon: const Icon(Icons.chevron_right),
                  ),
                  TextButton.icon(
                    onPressed: () => setState(() {
                      _preset = 180;
                      _resetWindow();
                    }),
                    icon: const Icon(Icons.update, size: 18),
                    label: Text(l10n.settingsDataTimelineRecent),
                  ),
                ],
              ),
            ),
            Semantics(
              label: l10n.settingsDataTimelineRange,
              child: RangeSlider(
                values: RangeValues(
                  ((_start - _lower) / (_upper - _lower)).clamp(0, 1),
                  ((_end - _lower) / (_upper - _lower)).clamp(0, 1),
                ),
                labels: RangeLabels(
                  _dateTime(_start, seconds: data.bucketMs < _minuteMs),
                  _dateTime(_end, seconds: data.bucketMs < _minuteMs),
                ),
                onChanged: (values) {
                  final lower = _lower;
                  final span = _upper - lower;
                  setState(
                    () => _setWindow(
                      lower + (values.start * span).round(),
                      lower + (values.end * span).round(),
                    ),
                  );
                },
              ),
            ),
            if (_selected != null &&
                _selectedPlot == 'usage-timeline-plot') ...[
              Container(
                key: const ValueKey('usage-timeline-detail'),
                width: double.infinity,
                padding: const EdgeInsets.all(12),
                margin: const EdgeInsets.only(bottom: 12),
                decoration: BoxDecoration(
                  color: colors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      '${_dateTime(math.max(_start, data.bucketStart + _selected! * data.bucketMs), seconds: data.bucketMs < _minuteMs)} — ${_dateTime(math.min(_end, data.bucketStart + (_selected! + 1) * data.bucketMs), seconds: data.bucketMs < _minuteMs)}',
                      style: text.labelMedium,
                    ),
                    const SizedBox(height: 6),
                    for (final series in visible)
                      Text(
                        '${series.label}  ${_measure(series.values[_selected!], data.metric, data.currency)}',
                        style: text.bodySmall?.copyWith(
                          color: _colorFor(series, colors),
                        ),
                      ),
                  ],
                ),
              ),
            ],
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                for (var index = 0; index < data.series.length; index++)
                  FilterChip(
                    label: Text(
                      '${data.series[index].label} · ${_number(data.series[index].total)}',
                    ),
                    avatar: CircleAvatar(
                      backgroundColor: _colorFor(data.series[index], colors),
                      radius: 5,
                    ),
                    selected: !_hidden.contains(data.series[index].key),
                    onSelected: (selected) => setState(() {
                      _cachedSpend = null;
                      _cachedRate = null;
                      _cachedAverageInput = null;
                      _cachedSavings = null;
                      _insightRecords = null;
                      _insightCurrencies = null;
                      final key = data.series[index].key;
                      if (selected) {
                        _hidden.remove(key);
                      } else {
                        _hidden.add(key);
                      }
                    }),
                  ),
              ],
            ),
            const SizedBox(height: 20),
            _buildInsights(context),
          ],
        ),
      ),
    );
  }
}

Rect _plotRect(Size size) =>
    Rect.fromLTRB(44, 12, size.width - 12, size.height - 38);

class _TimelinePainter extends CustomPainter {
  const _TimelinePainter({
    required this.data,
    required this.visible,
    required this.colors,
    required this.selected,
    required this.colorFor,
    required this.textDirection,
    required this.labelStyle,
  });

  final UsageTimelineData data;
  final List<UsageTimelineSeries> visible;
  final ColorScheme colors;
  final int? selected;
  final Color Function(UsageTimelineSeries) colorFor;
  final TextDirection textDirection;
  final TextStyle labelStyle;

  @override
  void paint(Canvas canvas, Size size) {
    final plot = _plotRect(size);
    var peak = 0.0;
    var low = 0.0;
    for (final series in visible) {
      for (final value in series.values) {
        if (value != null) {
          peak = math.max(peak, value.toDouble());
          low = math.min(low, value.toDouble());
        }
      }
    }
    final rate = data.metric == UsageTimelineMetric.cacheRate;
    final money = data.metric.isMoney;
    if (peak == 0 && low == 0) peak = money ? 0.001 : 1;
    final targetStep = rate
        ? 25.0
        : math.max(money ? 0.000001 : 1.0, (peak - low) / 4);
    final magnitude = math
        .pow(10, (math.log(targetStep) / math.ln10).floor())
        .toDouble();
    final step = rate
        ? 25.0
        : const [
                1,
                2,
                5,
                10,
              ].firstWhere((value) => value * magnitude >= targetStep) *
              magnitude;
    final minY = rate ? 0.0 : (low / step).floor() * step;
    final maxY = rate ? 100.0 : (peak / step).ceil() * step;
    final gridSteps = ((maxY - minY) / step).round();
    double yAt(num value) =>
        plot.bottom - (value - minY) / (maxY - minY) * plot.height;
    final grid = Paint()
      ..color = colors.outlineVariant.withValues(alpha: 0.35)
      ..strokeWidth = 0.7;
    final style = labelStyle;
    for (var i = 0; i <= gridSteps; i++) {
      final y = plot.bottom - plot.height * i / gridSteps;
      canvas.drawLine(Offset(plot.left, y), Offset(plot.right, y), grid);
      _text(
        canvas,
        rate
            ? '${(i * 25)}%'
            : money
            ? _moneyAxis(minY + step * i)
            : _compact(minY + step * i),
        Offset(0, y - 6),
        38,
        style,
        TextAlign.right,
      );
    }
    final tickCount = size.width < 500 ? 3 : 5;
    for (var i = 0; i < tickCount; i++) {
      final fraction = i / (tickCount - 1);
      final at = data.start + ((data.end - data.start) * fraction).round();
      final date = DateTime.fromMillisecondsSinceEpoch(at);
      final label =
          data.end - data.start > const Duration(days: 2).inMilliseconds
          ? '${date.month}/${date.day}\n${_time(date)}'
          : data.end - data.start > const Duration(hours: 12).inMilliseconds
          ? '${date.month}/${date.day} ${_time(date)}'
          : _time(date, seconds: data.bucketMs < _minuteMs);
      final width = math.min(90.0, plot.width / tickCount);
      final x = (plot.left + plot.width * fraction - width / 2).clamp(
        plot.left,
        plot.right - width,
      );
      _text(
        canvas,
        label,
        Offset(x, plot.bottom + 10),
        width,
        style,
        i == 0
            ? TextAlign.left
            : i == tickCount - 1
            ? TextAlign.right
            : TextAlign.center,
      );
    }
    double xAt(int bucket) {
      final bucketStart = data.bucketStart + bucket * data.bucketMs;
      final left = math.max(data.start, bucketStart);
      final right = math.min(data.end, bucketStart + data.bucketMs);
      return plot.left +
          (((left + right) / 2 - data.start) / (data.end - data.start)) *
              plot.width;
    }

    canvas.save();
    canvas.clipRect(plot.inflate(1));
    for (final series in visible) {
      final points = [
        for (var index = 0; index < series.values.length; index++)
          series.values[index] == null
              ? null
              : Offset(xAt(index), yAt(series.values[index]!)),
      ];
      final color = colorFor(series);
      canvas.drawPath(
        buildUsageTimelineCurve(points),
        Paint()
          ..color = color
          ..style = PaintingStyle.stroke
          ..strokeWidth = 2
          ..strokeCap = StrokeCap.round
          ..strokeJoin = StrokeJoin.round,
      );
      final spacing = data.bucketMs / (data.end - data.start) * plot.width;
      for (var index = 0; index < points.length; index++) {
        final point = points[index];
        if (point == null) continue;
        final isolated =
            (index == 0 || points[index - 1] == null) &&
            (index == points.length - 1 || points[index + 1] == null);
        if (isolated ||
            (spacing >= 18 && (series.values[index] != 0 || rate))) {
          canvas.drawCircle(point, 1.8, Paint()..color = color);
        }
      }
    }
    if (selected != null) {
      final x = xAt(selected!).clamp(plot.left, plot.right);
      canvas.drawLine(
        Offset(x, plot.top),
        Offset(x, plot.bottom),
        Paint()
          ..color = colors.onSurface.withValues(alpha: 0.5)
          ..strokeWidth = 1,
      );
      for (final series in visible) {
        if (series.values[selected!] == null) continue;
        final point = Offset(x, yAt(series.values[selected!]!));
        canvas.drawCircle(point, 4, Paint()..color = colors.surface);
        canvas.drawCircle(point, 3, Paint()..color = colorFor(series));
      }
    }
    canvas.restore();
  }

  void _text(
    Canvas canvas,
    String text,
    Offset at,
    double width,
    TextStyle style,
    TextAlign align,
  ) {
    final painter = TextPainter(
      text: TextSpan(text: text, style: style),
      textDirection: textDirection,
      textAlign: align,
    )..layout(minWidth: width, maxWidth: width);
    painter.paint(canvas, at);
  }

  @override
  bool shouldRepaint(covariant _TimelinePainter oldDelegate) => true;
}

Color _seriesColor(int index, ColorScheme colors) {
  const hues = [210.0, 28.0, 155.0, 285.0, 350.0, 185.0, 70.0];
  final hue = (hues[index % hues.length] + (index ~/ hues.length) * 17) % 360;
  return HSLColor.fromAHSL(
    1,
    hue,
    0.68,
    colors.brightness == Brightness.dark ? 0.68 : 0.38,
  ).toColor();
}

String _metricLabel(AppLocalizations l10n, UsageTimelineMetric metric) =>
    switch (metric) {
      UsageTimelineMetric.tokens => l10n.settingsDataDetailedStatsTotalTokens,
      UsageTimelineMetric.input => l10n.settingsDataInputTokens,
      UsageTimelineMetric.output => l10n.settingsDataOutputTokens,
      UsageTimelineMetric.cached => l10n.settingsDataDetailedStatsCachedInput,
      UsageTimelineMetric.requests =>
        l10n.settingsDataDetailedStatsRequestsSeries,
      UsageTimelineMetric.spend => l10n.settingsDataTimelineSpend,
      UsageTimelineMetric.cacheRate => l10n.settingsDataTimelineCacheRate,
      UsageTimelineMetric.averageInput => l10n.settingsDataTimelineAverageInput,
      UsageTimelineMetric.cacheSavings => l10n.settingsDataTimelineCacheSavings,
    };

String _bucketLabel(AppLocalizations l10n, UsageTimelineData data) =>
    data.bucketMs < _minuteMs
    ? l10n.settingsDataTimelineSecondsBucket(data.bucketMs ~/ 1000)
    : l10n.settingsDataTimelineBucket(data.bucketMinutes);

String _time(DateTime date, {bool seconds = false}) =>
    '${date.hour.toString().padLeft(2, '0')}:${date.minute.toString().padLeft(2, '0')}${seconds ? ':${date.second.toString().padLeft(2, '0')}' : ''}';
String _dateTime(int ms, {bool seconds = false}) {
  final date = DateTime.fromMillisecondsSinceEpoch(ms);
  return '${date.year}/${date.month}/${date.day} ${_time(date, seconds: seconds)}';
}

String _number(num value) => value.toString().replaceAllMapped(
  RegExp(r'(\d)(?=(\d{3})+(?!\d))'),
  (match) => '${match[1]},',
);
String _compact(double value) => value >= 1000000
    ? '${(value / 1000000).toStringAsFixed(1)}M'
    : value >= 1000
    ? '${(value / 1000).toStringAsFixed(1)}k'
    : value.toStringAsFixed(
        value < 4 && value != value.roundToDouble() ? 1 : 0,
      );

String _measure(num? value, UsageTimelineMetric metric, String? currency) {
  if (value == null) return '—';
  if (metric == UsageTimelineMetric.cacheRate) {
    return '${value.toStringAsFixed(1)}%';
  }
  if (metric.isMoney) {
    return '${currency ?? ''} ${formatUsageMoney(value, detailed: true)}';
  }
  if (metric == UsageTimelineMetric.averageInput) {
    return value.toStringAsFixed(1);
  }
  return _number(value);
}

String _moneyAxis(double value) => value == 0
    ? '0'
    : value.abs() >= 1
    ? value.toStringAsFixed(2)
    : value.abs() >= 0.01
    ? value.toStringAsFixed(3)
    : value.abs() >= 0.001
    ? value.toStringAsFixed(4)
    : value.toStringAsExponential(1);
