import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/settings/data/UsageStatisticsDetailScreen.dart';
import 'package:operit2/ui/theme/OperitGlassSurface.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  testWidgets('keeps heatmap and one model timeline on phone and desktop', (
    tester,
  ) async {
    for (final size in [const Size(390, 844), const Size(1440, 1000)]) {
      await tester.pumpWidget(const SizedBox.shrink());
      final cardSize = await _pumpDashboard(
        tester,
        bridge: _UsageStatisticsBridge(),
        size: size,
      );
      expect(cardSize.height, lessThanOrEqualTo(86));
      expect(find.text('Usage heatmap'), findsOneWidget);
      expect(find.text('Tap a cell to view details'), findsOneWidget);
      await _scrollToText(tester, 'Model usage over time');
      expect(find.text('3h'), findsOneWidget);
      expect(find.textContaining('min buckets'), findsOneWidget);
      expect(find.textContaining('OpenAI · gpt-5'), findsOneWidget);
      expect(find.text('Daily usage trend'), findsNothing);
      expect(find.text('Function model ranking'), findsNothing);
      expect(tester.takeException(), isNull);
    }
  });

  testWidgets(
    'changes minute scale, inspects values, filters models and changes metric',
    (tester) async {
      await _pumpDashboard(
        tester,
        bridge: _UsageStatisticsBridge(),
        size: const Size(1440, 1100),
      );
      await _openDateRangeDialog(tester);
      await tester.tap(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.text('All'),
        ),
      );
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
      await _scrollToText(tester, 'Model usage over time');
      await tester.tap(find.text('24h'));
      await tester.pumpAndSettle();
      final day = _plotData(tester);
      expect(day.bucketMinutes, greaterThan(1));
      await tester.tap(find.text('7 days'));
      await tester.pumpAndSettle();
      expect(_plotData(tester).bucketMs, greaterThan(day.bucketMs));
      await tester.tap(find.text('3h'));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('Zoom in'));
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<ChoiceChip>(
              find.ancestor(
                of: find.text('3h'),
                matching: find.byType(ChoiceChip),
              ),
            )
            .selected,
        isFalse,
      );
      await tester.tap(find.byKey(const ValueKey('usage-timeline-plot')));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('usage-timeline-detail')),
        findsOneWidget,
      );
      final model = find.ancestor(
        of: find.descendant(
          of: find.byType(FilterChip),
          matching: find.textContaining('OpenAI · gpt-5'),
        ),
        matching: find.byType(FilterChip),
      );
      await tester.ensureVisible(model);
      await tester.pumpAndSettle();
      await tester.tap(model);
      await tester.pumpAndSettle();
      expect(tester.widget<FilterChip>(model).selected, isFalse);
      final metric = find.byKey(const ValueKey('usage-timeline-metric'));
      await tester.ensureVisible(metric);
      await tester.pumpAndSettle();
      await tester.tap(metric);
      await tester.pumpAndSettle();
      await tester.tap(find.text('Requests').last);
      await tester.pumpAndSettle();
      expect(find.text('Requests'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'adapts plot density to width and reveals seconds on close zoom',
    (tester) async {
      await _pumpDashboard(
        tester,
        bridge: _UsageStatisticsBridge(),
        size: const Size(1440, 1100),
      );
      await _scrollToText(tester, 'Model usage over time');
      final wide = _plotData(tester);
      tester.view.physicalSize = const Size(390, 844);
      await tester.pumpAndSettle();
      final narrow = _plotData(tester);
      expect(narrow.bucketMs, greaterThan(wide.bucketMs));
      expect(narrow.series.first.total, wide.series.first.total);
      expect(narrow.start, wide.start);
      expect(narrow.end, wide.end);
      await tester.ensureVisible(find.text('3h'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('3h'));
      await tester.pumpAndSettle();
      for (var i = 0; i < 6; i++) {
        await tester.ensureVisible(find.byTooltip('Zoom in'));
        await tester.pumpAndSettle();
        await tester.tap(find.byTooltip('Zoom in'));
        await tester.pumpAndSettle();
      }
      final close = _plotData(tester);
      expect(
        close.end - close.start,
        const Duration(minutes: 5).inMilliseconds,
      );
      expect(close.bucketMs, lessThan(Duration.millisecondsPerMinute));
      expect(find.textContaining('sec buckets'), findsOneWidget);
      for (final key in [
        'usage-timeline-spend-plot',
        'usage-timeline-cacheRate-plot',
        'usage-timeline-averageInput-plot',
        'usage-timeline-cacheSavings-plot',
      ]) {
        final companion = _plotData(tester, key);
        expect(companion.bucketMs, close.bucketMs);
        expect(companion.start, close.start);
        expect(companion.end, close.end);
      }
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'defaults the page header and all statistics to a rolling 24 hours',
    (tester) async {
      final now = DateTime.now();
      Map<String, Object?> extra(int hours, String id) => {
        ..._UsageStatisticsBridge._records.first,
        'id': id,
        'createdAtMs': now
            .subtract(Duration(hours: hours))
            .millisecondsSinceEpoch,
        'inputTokens': 100,
        'outputTokens': 0,
        'cachedInputTokens': 0,
      };
      await _pumpDashboard(
        tester,
        bridge: _UsageStatisticsBridge(
          extraRecords: [extra(23, 'inside'), extra(25, 'outside')],
        ),
        size: const Size(1440, 1200),
      );
      expect(find.textContaining('Last 24 hours'), findsOneWidget);
      final window = _plotData(tester);
      expect(
        window.end - window.start,
        const Duration(hours: 24).inMilliseconds,
      );
      expect(window.requestCount, 3);
      expect(
        window.inputByModel.values.fold<int>(
          0,
          (int sum, dynamic input) => sum + (input as int),
        ),
        265100,
      );
      expect(window.end, closeTo(now.millisecondsSinceEpoch, 5000));
      await tester.tap(find.byTooltip('Refresh'));
      await tester.pumpAndSettle();
      expect(find.textContaining('Last 24 hours'), findsOneWidget);
      expect(_plotData(tester).requestCount, 3);
      await _openDateRangeDialog(tester);
      final selected = tester.widget<ListTile>(
        find.ancestor(
          of: find.text('Last 24 hours'),
          matching: find.byType(ListTile),
        ),
      );
      expect(selected.selected, isTrue);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'adds request-size and cache-savings insights with signed savings',
    (tester) async {
      await _pumpDashboard(
        tester,
        bridge: _UsageStatisticsBridge(),
        size: const Size(1440, 1200),
      );
      final average = find.byKey(
        const ValueKey('usage-timeline-averageInput-card'),
      );
      final savings = find.byKey(
        const ValueKey('usage-timeline-cacheSavings-card'),
      );
      expect(
        find.descendant(
          of: average,
          matching: find.text('132.5k tokens / request'),
        ),
        findsOneWidget,
      );
      expect(
        find.descendant(of: savings, matching: find.text('USD 0.1350')),
        findsOneWidget,
      );
      await tester.pumpWidget(const SizedBox.shrink());
      await _pumpDashboard(
        tester,
        bridge: _UsageStatisticsBridge(cachePriceMultiplier: 100),
        size: const Size(390, 844),
      );
      await tester.ensureVisible(savings);
      await tester.pumpAndSettle();
      expect(
        _plotData(tester, 'usage-timeline-cacheSavings-plot').amount,
        lessThan(0),
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('pans and pinches the timeline on a phone', (tester) async {
    await _pumpDashboard(
      tester,
      bridge: _UsageStatisticsBridge(),
      size: const Size(390, 844),
    );
    await _scrollToText(tester, 'Model usage over time');
    await tester.tap(find.text('3h'));
    await tester.pumpAndSettle();
    final plot = find.byKey(const ValueKey('usage-timeline-plot'));
    await tester.ensureVisible(plot);
    await tester.pumpAndSettle();
    RangeValues range() =>
        tester.widget<RangeSlider>(find.byType(RangeSlider)).values;
    final before = range();
    await tester.timedDrag(
      plot,
      const Offset(100, 0),
      const Duration(milliseconds: 300),
    );
    await tester.pumpAndSettle();
    expect(range().start, lessThan(before.start));
    final span = range().end - range().start;
    final center = tester.getCenter(plot);
    final left = await tester.createGesture(pointer: 1);
    final right = await tester.createGesture(pointer: 2);
    await left.down(center - const Offset(35, 0));
    await right.down(center + const Offset(35, 0));
    await tester.pump();
    await left.moveTo(center - const Offset(75, 0));
    await right.moveTo(center + const Offset(75, 0));
    await tester.pump();
    await left.up();
    await right.up();
    await tester.pumpAndSettle();
    expect(range().end - range().start, lessThan(span));
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'shows spending and weighted cache-rate curves sharing the minute window',
    (tester) async {
      await _pumpDashboard(
        tester,
        bridge: _UsageStatisticsBridge(),
        size: const Size(1440, 1200),
      );
      await _scrollToText(tester, 'Model usage over time');
      final spend = find.byKey(const ValueKey('usage-timeline-spend-card'));
      final rate = find.byKey(const ValueKey('usage-timeline-cacheRate-card'));
      expect(
        find.descendant(of: spend, matching: find.text('USD 1.2510')),
        findsOneWidget,
      );
      expect(
        find.descendant(of: rate, matching: find.text('24.5%')),
        findsOneWidget,
      );
      dynamic data(String key) {
        final dynamic painter = tester
            .widget<CustomPaint>(
              find.descendant(
                of: find.byKey(ValueKey(key)),
                matching: find.byType(CustomPaint),
              ),
            )
            .painter;
        return painter.data;
      }

      final mainBefore = data('usage-timeline-plot');
      await tester.tap(find.byTooltip('Zoom in'));
      await tester.pumpAndSettle();
      final mainAfter = data('usage-timeline-plot');
      expect(
        mainAfter.end - mainAfter.start,
        lessThan(mainBefore.end - mainBefore.start),
      );
      for (final key in [
        'usage-timeline-spend-plot',
        'usage-timeline-cacheRate-plot',
        'usage-timeline-averageInput-plot',
        'usage-timeline-cacheSavings-plot',
      ]) {
        expect(data(key).start, mainAfter.start);
        expect(data(key).end, mainAfter.end);
        expect(data(key).bucketMs, mainAfter.bucketMs);
      }
      await tester.ensureVisible(
        find.byKey(const ValueKey('usage-timeline-cacheRate-plot')),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const ValueKey('usage-timeline-cacheRate-plot')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('usage-timeline-cacheRate-detail')),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'keeps currencies separate and leaves usage available if prices cannot load',
    (tester) async {
      await _pumpDashboard(
        tester,
        bridge: _UsageStatisticsBridge(anthropicCurrency: 'CNY'),
        size: const Size(1440, 1200),
      );
      final spend = find.byKey(const ValueKey('usage-timeline-spend-card'));
      final currency = find.byKey(const ValueKey('usage-timeline-currency'));
      await tester.ensureVisible(currency);
      await tester.pumpAndSettle();
      expect(
        find.descendant(of: spend, matching: find.text('CNY 0.6660')),
        findsOneWidget,
      );
      await tester.tap(currency);
      await tester.pumpAndSettle();
      await tester.tap(find.text('USD').last);
      await tester.pumpAndSettle();
      expect(
        find.descendant(of: spend, matching: find.text('USD 0.5850')),
        findsOneWidget,
      );
      await tester.pumpWidget(const SizedBox.shrink());
      await _pumpDashboard(
        tester,
        bridge: _UsageStatisticsBridge(failPricing: true),
        size: const Size(1440, 1200),
      );
      expect(find.text('Model usage over time'), findsOneWidget);
      expect(
        find.text('Could not load model prices. Refresh to retry.'),
        findsNWidgets(2),
      );
      expect(
        find.byKey(const ValueKey('usage-timeline-cacheRate-plot')),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'date filter removes older models and refresh keeps the chosen date range',
    (tester) async {
      await _pumpDashboard(
        tester,
        bridge: _UsageStatisticsBridge(),
        size: const Size(1440, 1200),
      );
      await _openDateRangeDialog(tester);
      await tester.tap(find.text('7 days').last);
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
      await _scrollToText(tester, 'Model usage over time');
      expect(find.textContaining('OpenAI · gpt-5'), findsOneWidget);
      expect(find.textContaining('LegacyAI · legacy-model'), findsNothing);
      await tester.tap(find.byTooltip('Refresh'));
      await tester.pumpAndSettle();
      expect(find.textContaining('LegacyAI · legacy-model'), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );
}

Future<void> _scrollToText(WidgetTester tester, String text) async {
  await tester.scrollUntilVisible(
    find.text(text),
    260,
    scrollable: find
        .descendant(
          of: find.byType(ListView).first,
          matching: find.byType(Scrollable),
        )
        .first,
  );
  await tester.pumpAndSettle();
}

Future<void> _openDateRangeDialog(WidgetTester tester) async {
  await tester.tap(find.byIcon(Icons.date_range_outlined).first);
  await tester.pumpAndSettle();
}

dynamic _plotData(WidgetTester tester, [String key = 'usage-timeline-plot']) {
  final dynamic painter = tester
      .widget<CustomPaint>(
        find.descendant(
          of: find.byKey(ValueKey(key)),
          matching: find.byType(CustomPaint),
        ),
      )
      .painter;
  return painter.data;
}

/// Pumps the dashboard at one viewport and returns the request metric size.
Future<Size> _pumpDashboard(
  WidgetTester tester, {
  required _UsageStatisticsBridge bridge,
  required Size size,
}) async {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = size;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: UsageStatisticsDetailScreen(
        clients: GeneratedCoreProxyClients(bridge),
      ),
    ),
  );
  await tester.pumpAndSettle();

  final metricSurface = find
      .ancestor(
        of: find.byIcon(Icons.bolt_outlined),
        matching: find.byType(OperitGlassSurface),
      )
      .first;
  expect(metricSurface, findsOneWidget);
  return tester.getSize(metricSurface);
}

class _UsageStatisticsBridge extends OperitRuntimeBridge {
  _UsageStatisticsBridge({
    this.anthropicCurrency = 'USD',
    this.failPricing = false,
    this.extraRecords = const [],
    this.cachePriceMultiplier = 1,
  });
  final String anthropicCurrency;
  final bool failPricing;
  final List<Map<String, Object?>> extraRecords;
  final double cachePriceMultiplier;
  static final List<Map<String, Object?>> _records = <Map<String, Object?>>[
    <String, Object?>{
      'id': '1',
      'createdAtMs': DateTime.now()
          .subtract(const Duration(minutes: 20))
          .millisecondsSinceEpoch,
      'providerModel': 'OpenAI/gpt-5',
      'provider': 'OpenAI',
      'modelName': 'gpt-5',
      'functionType': 'CHAT',
      'source': 'CHAT_RESPONSE',
      'chatId': 'work',
      'inputTokens': 165000,
      'outputTokens': 42000,
      'cachedInputTokens': 45000,
    },
    <String, Object?>{
      'id': '2',
      'createdAtMs': DateTime.now()
          .subtract(const Duration(minutes: 40))
          .millisecondsSinceEpoch,
      'providerModel': 'Anthropic/claude-sonnet',
      'provider': 'Anthropic',
      'modelName': 'claude-sonnet',
      'functionType': 'SUMMARY',
      'source': 'SUMMARY_GENERATION',
      'chatId': 'personal',
      'inputTokens': 100000,
      'outputTokens': 28000,
      'cachedInputTokens': 20000,
    },
    <String, Object?>{
      'id': '3',
      'createdAtMs': DateTime.now()
          .subtract(const Duration(days: 60))
          .millisecondsSinceEpoch,
      'providerModel': 'LegacyAI/legacy-model',
      'provider': 'LegacyAI',
      'modelName': 'legacy-model',
      'functionType': 'CHAT',
      'source': 'CHAT_RESPONSE',
      'chatId': 'archive',
      'inputTokens': 1000000,
      'outputTokens': 250000,
      'cachedInputTokens': 200000,
    },
  ];

  /// Returns deterministic token records through the generated bridge codec.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    switch (request.methodName) {
      case 'getAllRequestRecords':
        return encodeCoreLink(<Object?>[
          0,
          [..._records, ...extraRecords],
        ]);
      case 'getAllModelSummaries':
        return encodeCoreLink(<Object?>[0, <Object?>[]]);
      case 'getAllTokenStatsModels':
        if (failPricing) throw StateError('Unavailable pricing');
        return encodeCoreLink(<Object?>[
          0,
          [
            for (final provider in ['OpenAI', 'Anthropic'])
              <String, Object?>{
                'configId': provider,
                'provider': provider,
                'model': provider == 'OpenAI' ? 'gpt-5' : 'claude-sonnet',
                'billingMode': 'TOKEN',
                'currency': provider == 'Anthropic' ? anthropicCurrency : 'USD',
                'inputPricePerMillion': provider == 'OpenAI' ? 2.0 : 3.0,
                'cachedInputPricePerMillion':
                    (provider == 'OpenAI' ? 0.2 : 0.3) * cachePriceMultiplier,
                'cacheWritePricePerMillion': null,
                'outputPricePerMillion': provider == 'OpenAI' ? 8.0 : 15.0,
                'pricePerRequest': null,
              },
          ],
        ]);
      default:
        throw StateError('Unexpected Core call: ${request.methodName}');
    }
  }

  /// Rejects client-owned streams because this page only issues one Core call.
  @override
  Future<CorePushSink> push(CorePushRequest request) {
    throw StateError('Unexpected Core push: ${request.methodName}');
  }

  /// Rejects snapshots because this page does not subscribe to Core state.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) {
    throw StateError('Unexpected Core snapshot: ${request.propertyName}');
  }

  /// Rejects streams because this page does not subscribe to Core state.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    throw StateError('Unexpected Core watch: ${request.propertyName}');
  }
}
