import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/packages/components/ExtensionAvatar.dart';
import 'package:operit2/ui/features/packages/components/StoreCards.dart';
import 'package:operit2/ui/features/packages/market/MarketInstallStateStore.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

class _UnusedBridge extends OperitRuntimeBridge {
  @override
  dynamic noSuchMethod(Invocation invocation) => throw UnimplementedError();
}

void main() {
  for (final brightness in Brightness.values) {
    testWidgets(
      'API corners stay on banner and avatars in ${brightness.name}',
      (tester) async {
        final installState = MarketInstallStateStore.of(
          GeneratedCoreProxyClients(_UnusedBridge()),
        );
        final entry = MarketEntrySummary.fromJson({
          'id': 'modern',
          'type': 'package',
          'title': 'Music Studio',
          'description': 'Music tools',
          'latestVersion': {'apiVersion': '2.0.0'},
        });
        await tester.pumpWidget(_testApp(entry, installState, brightness));
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);

        final banner = find.byType(StoreFeaturedBanner);
        final bannerMark = _cornerIn(banner);
        expect(bannerMark, findsOneWidget);
        expect(tester.getSize(bannerMark), const Size.square(32));
        expect(tester.getTopLeft(bannerMark), tester.getTopLeft(banner));

        for (final host in [
          find.byType(StoreMarketCard),
          find.byType(StoreMarketRow),
        ]) {
          final mark = _cornerIn(host);
          final avatar = find.descendant(
            of: host,
            matching: find.byType(ExtensionAvatar),
          );
          expect(mark, findsOneWidget);
          expect(tester.getSize(mark), const Size.square(20));
          expect(tester.getTopLeft(mark), tester.getTopLeft(avatar));
          expect(
            tester.getRect(avatar).contains(tester.getBottomRight(mark)),
            isTrue,
          );
        }
      },
    );
  }

  for (final input in [
    (type: 'package', api: '1.0.0'),
    (type: 'package', api: null),
    (type: 'skill', api: '2.0.0'),
  ]) {
    testWidgets('${input.type} API ${input.api} has no v2 corners', (
      tester,
    ) async {
      final entry = MarketEntrySummary.fromJson({
        'id': 'legacy',
        'type': input.type,
        'title': 'Legacy',
        'latestVersion': {'apiVersion': input.api},
      });
      await tester.pumpWidget(
        _testApp(
          entry,
          MarketInstallStateStore.of(
            GeneratedCoreProxyClients(_UnusedBridge()),
          ),
          Brightness.light,
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(_corners, findsNothing);
    });
  }
}

final _corners = find.byWidgetPredicate(
  (widget) =>
      widget is Semantics && widget.properties.label == 'ToolPkg API 2.0.0',
);
Finder _cornerIn(Finder host) => find.descendant(of: host, matching: _corners);

Widget _testApp(
  MarketEntrySummary entry,
  MarketInstallStateStore installState,
  Brightness brightness,
) => OperitTheme(
  initialThemePreferenceSnapshot:
      UserPreferencesManager.defaultThemePreferenceSnapshot,
  initialThemeIsReady: false,
  unconfiguredChildEnabled: true,
  hostInteractionHostsEnabled: false,
  child: MaterialApp(
    theme: ThemeData(
      colorScheme: ColorScheme.fromSeed(
        seedColor: Colors.blue,
        brightness: brightness,
      ),
    ),
    home: Scaffold(
      body: Column(
        children: [
          SizedBox(
            height: 200,
            child: StoreFeaturedBanner(
              entry: entry,
              installState: installState,
            ),
          ),
          SizedBox(
            width: 260,
            height: 224,
            child: StoreMarketCard(entry: entry, installState: installState),
          ),
          StoreMarketRow(entry: entry, installState: installState),
        ],
      ),
    ),
  ),
);
