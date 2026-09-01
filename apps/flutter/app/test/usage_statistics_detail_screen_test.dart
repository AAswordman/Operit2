import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/features/settings/data/UsageStatisticsDetailScreen.dart';
import 'package:operit2/ui/theme/OperitGlassSurface.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies the token statistics dashboard adapts to phone and desktop widths.
void main() {
  testWidgets('reflows metric cards without layout overflow', (tester) async {
    final bridge = _UsageStatisticsBridge();

    final phoneCardSize = await _pumpDashboard(
      tester,
      bridge: bridge,
      size: const Size(390, 844),
    );
    expect(phoneCardSize.width, lessThan(180));
    expect(phoneCardSize.height, lessThanOrEqualTo(86));
    expect(find.text('Usage heatmap'), findsOneWidget);
    expect(tester.takeException(), isNull);

    final desktopCardSize = await _pumpDashboard(
      tester,
      bridge: bridge,
      size: const Size(1440, 1000),
    );
    expect(desktopCardSize.width, greaterThan(200));
    expect(desktopCardSize.width, lessThan(240));
    expect(desktopCardSize.height, lessThanOrEqualTo(86));
    expect(find.text('Usage heatmap'), findsOneWidget);
    expect(find.text('Tap a cell to view details'), findsOneWidget);
    expect(find.text('All'), findsNothing);
    await _openDateRangeDialog(tester);
    expect(find.text('All'), findsOneWidget);
    expect(find.text('7 days'), findsOneWidget);
    expect(find.text('30 days'), findsOneWidget);
    expect(find.text('Custom'), findsOneWidget);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    await _scrollToText(tester, 'Function model ranking');
    expect(find.text('Chat · LegacyAI · legacy-model'), findsOneWidget);
    await tester.drag(find.byType(ListView), const Offset(0, 1200));
    await tester.pumpAndSettle();
    await _openDateRangeDialog(tester);
    await tester.tap(find.text('7 days'));
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();
    final desktopDailyTrend = _topLeftOfText(tester, 'Daily usage trend');
    final desktopTokenTrend = _topLeftOfText(
      tester,
      'Input / output consumption trend',
    );
    expect(desktopTokenTrend.dx, greaterThan(desktopDailyTrend.dx));
    expect(desktopTokenTrend.dy, closeTo(desktopDailyTrend.dy, 0.1));
    await _scrollToText(tester, 'Function model ranking');
    expect(find.text('Chat · OpenAI · gpt-5'), findsOneWidget);
    expect(find.text('Chat · LegacyAI · legacy-model'), findsNothing);
    expect(tester.takeException(), isNull);
  });
}

/// Scrolls the dashboard until a label is visible.
Future<void> _scrollToText(WidgetTester tester, String text) {
  return tester.scrollUntilVisible(find.text(text), 260);
}

/// Opens the popup date range selector from the dashboard header.
Future<void> _openDateRangeDialog(WidgetTester tester) async {
  await tester.tap(find.byIcon(Icons.date_range_outlined).first);
  await tester.pumpAndSettle();
}

/// Returns the top-left position of one unique dashboard label.
Offset _topLeftOfText(WidgetTester tester, String text) {
  final finder = find.text(text);
  expect(finder, findsOneWidget);
  return tester.getTopLeft(finder);
}

/// Pumps the dashboard at one viewport and returns the request metric size.
Future<Size> _pumpDashboard(
  WidgetTester tester, {
  required _UsageStatisticsBridge bridge,
  required Size size,
}) async {
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
  await tester.pumpWidget(
    OperitTheme(
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
  static final List<Map<String, Object?>> _records = <Map<String, Object?>>[
    <String, Object?>{
      'id': '1',
      'createdAtMs': DateTime(2026, 8, 30).millisecondsSinceEpoch,
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
      'createdAtMs': DateTime(2026, 8, 31).millisecondsSinceEpoch,
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
      'createdAtMs': DateTime(2026, 7, 1).millisecondsSinceEpoch,
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
    if (request.methodName != 'getAllRequestRecords') {
      throw StateError('Unexpected Core call: ${request.methodName}');
    }
    return encodeCoreLink(<Object?>[0, _records]);
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
