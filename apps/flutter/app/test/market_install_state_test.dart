import 'dart:async';
import 'dart:io';
import 'dart:convert';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/packages/market/MarketInstallStateStore.dart';
import 'package:operit2/ui/features/packages/model/ExtensionCatalogController.dart';
import 'package:operit2/ui/features/packages/screens/MarketEntryDetailScreen.dart';
import 'package:operit2/ui/features/packages/screens/MarketPane.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/main/TopBarController.dart';
import 'package:operit2/ui/main/MainLayoutController.dart';
import 'package:operit2/ui/main/components/AppContent.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/main/screens/OperitScreens.dart';
import 'package:operit2/ui/features/packages/screens/StoreBrowseScreen.dart';
import 'package:operit2/ui/features/packages/components/StoreDetailKit.dart';
import 'package:operit2/ui/features/packages/market/MarketStatsSupport.dart';

/// Verifies market installation state, market search and the entry detail.
void main() {
  final entry = _entry();

  testWidgets('ranking returns with loaded pages, sort and scroll intact', (
    tester,
  ) async {
    final bridge = _MarketBridge()
      ..listPageLoader = (page) async => {
        'page': page,
        'pageSize': 25,
        'total': 50,
        'items': [
          for (var i = (page - 1) * 25; i < page * 25; i++)
            {..._entry().toJson(), 'id': 'rank-$i', 'title': 'Plugin $i'},
        ],
      };
    final clients = GeneratedCoreProxyClients(bridge);
    final router = AppRouterState(RouteEntry(routeId: 'ranking'));
    final topBar = TopBarController();
    final layout = MainLayoutController();
    addTearDown(router.dispose);
    addTearDown(topBar.dispose);
    addTearDown(layout.dispose);
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: MainLayoutScope(
          controller: layout,
          child: TopBarScope(
            controller: topBar,
            child: Scaffold(
              body: ListenableBuilder(
                listenable: router,
                builder: (context, _) {
                  final inList = router.currentEntry.routeId == 'ranking';
                  return AppContent(
                    routerState: router,
                    currentScreen: inList
                        ? _RankingRoute(clients)
                        : const _DetailProbeRoute(),
                    currentRouteEntry: router.currentEntry,
                    currentRouteTitle: '排行榜',
                    useTabletLayout: false,
                    isTabletSidebarExpanded: false,
                    canGoBack: router.canPop,
                    enableNavigationAnimation: true,
                    isNavigatingBack: inList,
                    topBarController: topBar,
                    appBarEntries: const [],
                    onGoBack: router.pop,
                    onNavigationButtonPressed: () {},
                    onAppBarEntrySelected: (_) {},
                  );
                },
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final listState = tester.state(find.byType(StoreMarketListing));
    final scroll = tester
        .widget<CustomScrollView>(find.byType(CustomScrollView))
        .controller!;
    await tester.drag(find.byType(CustomScrollView), const Offset(0, -1900));
    await tester.pumpAndSettle();
    final offset = scroll.offset;
    final calls = List.of(bridge.listPages);
    expect(offset, greaterThan(0));
    expect(calls, contains(2));
    router.navigate(routeId: 'detail');
    await tester.pumpAndSettle();
    router.pop();
    await tester.pumpAndSettle();
    expect(tester.state(find.byType(StoreMarketListing)), same(listState));
    expect(scroll.offset, offset);
    expect(bridge.listPages, calls);
    expect(find.text('下载最多'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  for (final width in [390.0, 1000.0]) {
    testWidgets(
      'comments collapse, paginate newest first and return to latest at $width',
      (tester) async {
        tester.view.physicalSize = Size(width, 980);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.reset);
        const output = String.fromEnvironment('MARKET_SCREENSHOT_DIR');
        if (output.isNotEmpty) {
          await tester.runAsync(() async {
            for (final font in {
              'MarketChinese': '/System/Library/Fonts/Hiragino Sans GB.ttc',
              'MaterialIcons':
                  '/Users/ylvira/fvm/default/bin/cache/artifacts/material_fonts/MaterialIcons-Regular.otf',
            }.entries) {
              await (FontLoader(font.key)..addFont(
                    Future.value(
                      ByteData.sublistView(
                        await File(font.value).readAsBytes(),
                      ),
                    ),
                  ))
                  .load();
            }
          });
        }
        final bridge = _MarketBridge()..commentTotal = 23;
        final boundary = GlobalKey();
        await tester.pumpWidget(
          RepaintBoundary(
            key: boundary,
            child: _testApp(
              Builder(
                builder: (context) => Theme(
                  data: Theme.of(context).copyWith(
                    textTheme: Theme.of(context).textTheme.apply(
                      fontFamily: output.isEmpty ? null : 'MarketChinese',
                    ),
                  ),
                  child: Scaffold(
                    body: MarketEntryDetailScreen(
                      entryId: 'entry',
                      clients: GeneratedCoreProxyClients(bridge),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        await tester.scrollUntilVisible(
          find.text('查看全部 23 条评论'),
          300,
          scrollable: find
              .descendant(
                of: find.byType(CustomScrollView),
                matching: find.byType(Scrollable),
              )
              .first,
        );
        await tester.pumpAndSettle();
        expect(find.textContaining('最新优先'), findsNothing);
        expect(find.textContaining('每页 10 条'), findsNothing);
        expect(find.byType(StoreCommentTile), findsNWidgets(3));
        expect(find.text('momo'), findsOneWidget);
        expect(
          tester
              .widgetList<StoreCommentTile>(find.byType(StoreCommentTile))
              .any((tile) => tile.comment.id == 'comment-0'),
          isFalse,
        );
        if (output.isNotEmpty) {
          await _capture(
            tester,
            boundary,
            output,
            'comments-collapsed-${width.toInt()}',
          );
        }
        if (output.isNotEmpty) {
          final quote = find.byKey(const ValueKey('comment-parent-comment-22'));
          await tester.ensureVisible(quote);
          await tester.tap(quote);
          await tester.pumpAndSettle();
          await tester.ensureVisible(
            find.byKey(const ValueKey('comment-thread-comment-22')),
          );
          await tester.pumpAndSettle();
          await _capture(
            tester,
            boundary,
            output,
            'comments-conversation-${width.toInt()}',
          );
          await tester.ensureVisible(quote);
          await tester.tap(quote);
          await tester.pumpAndSettle();
          await tester.ensureVisible(find.text('查看全部 23 条评论'));
        }
        await tester.tap(find.text('查看全部 23 条评论'));
        await tester.pumpAndSettle();
        expect(find.byType(StoreCommentTile), findsNWidgets(10));
        expect(find.text('1 / 3'), findsOneWidget);
        final conversation = find.byKey(
          const ValueKey('comment-parent-comment-22'),
        );
        await tester.ensureVisible(conversation);
        await tester.tap(conversation);
        await tester.pumpAndSettle();
        expect(find.byKey(const ValueKey('context-comment-0')), findsOneWidget);
        expect(
          find.byWidgetPredicate(
            (widget) =>
                widget is StoreCommentTile &&
                widget.key == const ValueKey('comment-21'),
          ),
          findsNothing,
        );
        await tester.ensureVisible(find.text('收起评论'));
        await tester.pumpAndSettle();
        await tester.tap(find.text('收起评论'));
        await tester.pumpAndSettle();
        expect(find.byType(StoreCommentTile), findsNWidgets(3));
        expect(find.byKey(const ValueKey('context-comment-0')), findsNothing);
        expect(
          find.byWidgetPredicate(
            (widget) =>
                widget is StoreCommentTile &&
                widget.key == const ValueKey('comment-21'),
          ),
          findsOneWidget,
        );
        await tester.ensureVisible(find.text('查看全部 23 条评论'));
        await tester.tap(find.text('查看全部 23 条评论'));
        await tester.pumpAndSettle();
        await tester.scrollUntilVisible(
          find.text('下一页'),
          350,
          scrollable: find
              .descendant(
                of: find.byType(CustomScrollView),
                matching: find.byType(Scrollable),
              )
              .first,
        );
        await tester.pumpAndSettle();
        if (output.isNotEmpty) {
          tester.view.physicalSize = Size(width, 2200);
          await tester.pumpAndSettle();
          await tester.ensureVisible(find.text('评论与反馈'));
          await tester.pumpAndSettle();
          await _capture(
            tester,
            boundary,
            output,
            'comments-expanded-${width.toInt()}',
          );
          tester.view.physicalSize = Size(width, 980);
          await tester.pumpAndSettle();
          await tester.ensureVisible(find.text('下一页'));
          await tester.pumpAndSettle();
        }
        await tester.pumpAndSettle();
        await tester.ensureVisible(find.text('下一页'));
        await tester.pumpAndSettle();
        await tester.tap(find.text('下一页'));
        await tester.pumpAndSettle();
        expect(find.text('2 / 3'), findsOneWidget);
        expect(find.text('momo'), findsNothing);
        await tester.scrollUntilVisible(
          find.text('下一页'),
          350,
          scrollable: find
              .descendant(
                of: find.byType(CustomScrollView),
                matching: find.byType(Scrollable),
              )
              .first,
        );
        await tester.pumpAndSettle();
        await tester.ensureVisible(find.text('下一页'));
        await tester.pumpAndSettle();
        await tester.tap(find.text('下一页'));
        await tester.pumpAndSettle();
        expect(find.text('3 / 3'), findsOneWidget);
        expect(find.byType(StoreCommentTile), findsNWidgets(3));
        await tester.ensureVisible(find.text('收起评论'));
        await tester.tap(find.text('收起评论'));
        await tester.pumpAndSettle();
        expect(find.byType(StoreCommentTile), findsNWidgets(3));
        expect(find.text('momo'), findsOneWidget);
        expect(find.text('查看全部 23 条评论'), findsOneWidget);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets(
    'reply references open the exchange and posting preserves its parent',
    (tester) async {
      tester.view.physicalSize = const Size(390, 980);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final bridge = _MarketBridge()..commentTotal = 23;
      await tester.pumpWidget(_detail(bridge));
      await tester.pumpAndSettle();
      expect(find.textContaining('最新优先'), findsNothing);
      expect(find.textContaining('每页 10 条'), findsNothing);
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('comment-22')),
          matching: find.text('小陆'),
        ),
        findsOneWidget,
      );
      expect(find.text('stone'), findsOneWidget);
      final quote = find.byKey(const ValueKey('comment-parent-comment-22'));
      await tester.ensureVisible(quote);
      await tester.tap(quote);
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      final thread = find.byKey(const ValueKey('comment-thread-comment-22'));
      final tiles = tester.widgetList<StoreCommentTile>(
        find.descendant(of: thread, matching: find.byType(StoreCommentTile)),
      );
      expect(tiles.map((tile) => tile.comment.id), ['comment-0', 'comment-21']);
      expect(
        find.byWidgetPredicate(
          (widget) =>
              widget is StoreCommentTile &&
              widget.key == const ValueKey('comment-21'),
        ),
        findsNothing,
      );
      expect(find.byKey(const ValueKey('context-comment-21')), findsOneWidget);
      final reply = find.descendant(
        of: thread,
        matching: find.byKey(const ValueKey('comment-reply-comment-21')),
      );
      await tester.ensureVisible(reply);
      await tester.tap(reply);
      await tester.pumpAndSettle();
      expect(find.text('写下你的回复'), findsOneWidget);
      await tester.enterText(find.byType(TextField), '收到，感谢更新！');
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.text('发布'));
      await tester.tap(find.text('发布'));
      await tester.pumpAndSettle();
      expect(bridge.submittedComments.single['parent_id'], 'comment-21');
      expect(bridge.submittedComments.single['entry_id'], 'entry');
      expect(bridge.submittedComments.single['body'], '收到，感谢更新！');
      expect(find.byType(TextField), findsNothing);
      expect(find.text('收到，感谢更新！'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('cancelling a reply posts a top-level comment without a parent', (
    tester,
  ) async {
    final bridge = _MarketBridge()..commentTotal = 3;
    await tester.pumpWidget(_detail(bridge));
    await tester.pumpAndSettle();
    final reply = find.byKey(const ValueKey('comment-reply-comment-2'));
    await tester.ensureVisible(reply);
    await tester.tap(reply);
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), '独立评论');
    await tester.tap(find.byTooltip('取消回复'));
    await tester.pumpAndSettle();
    expect(find.text('分享你的使用体验'), findsOneWidget);
    await tester.ensureVisible(find.text('发布'));
    await tester.tap(find.text('发布'));
    await tester.pumpAndSettle();
    expect(bridge.submittedComments.single['parent_id'], isNull);
    expect(bridge.submittedComments.single['body'], '独立评论');
    expect(tester.takeException(), isNull);
  });

  test('uninstalled entries have no misleading installed or update badge', () {
    expect(
      resolveMarketLocalInstallState(entry),
      MarketLocalInstallState.notInstalled,
    );
    expect(MarketLocalInstallState.notInstalled.badgeLabel, isNull);
  });

  test('Kotlin entry/version markers distinguish installed and updatable', () {
    expect(
      resolveMarketLocalInstallState(entry, installedVersionId: 'v2'),
      MarketLocalInstallState.installed,
    );
    expect(
      resolveMarketLocalInstallState(entry, installedVersionId: 'v1'),
      MarketLocalInstallState.updateAvailable,
    );
    expect(MarketLocalInstallState.installed.actionLabel(entry), '已安装');
    expect(MarketLocalInstallState.updateAvailable.badgeLabel, '可更新');
  });

  test('missing latest version does not advertise a nonexistent update', () {
    expect(
      resolveMarketLocalInstallState(
        _entry(latest: null),
        installedVersionId: 'v1',
      ),
      MarketLocalInstallState.installed,
    );
  });

  test(
    'previous artifact imports are recognized using local package metadata',
    () {
      final artifact = _entry(type: 'package');
      expect(
        resolveMarketLocalInstallState(
          artifact,
          localPackage: _package('v2.0.0'),
        ),
        MarketLocalInstallState.installed,
      );
      expect(
        resolveMarketLocalInstallState(
          artifact,
          localPackage: _package('1.0.0'),
        ),
        MarketLocalInstallState.updateAvailable,
      );
      expect(
        resolveMarketLocalInstallState(artifact, localPackage: _package(null)),
        MarketLocalInstallState.installed,
      );
    },
  );

  test('markers take precedence over inferred artifact display versions', () {
    expect(
      resolveMarketLocalInstallState(
        _entry(type: 'package'),
        installedVersionId: 'v2',
        localPackage: _package('1.0.0'),
      ),
      MarketLocalInstallState.installed,
    );
  });

  test(
    'installed state is restored from Core, not from a screen-local flag',
    () async {
      final bridge = _MarketBridge()..versions['entry'] = 'v1';
      final store = MarketInstallStateStore.of(
        GeneratedCoreProxyClients(bridge),
      );
      await store.refresh();
      expect(store.stateFor(entry), MarketLocalInstallState.updateAvailable);
      bridge.versions['entry'] = 'v2';
      await store.refresh();
      expect(store.stateFor(entry), MarketLocalInstallState.installed);
      bridge.versions.clear();
      await store.refresh();
      expect(store.stateFor(entry), MarketLocalInstallState.notInstalled);
    },
  );

  test(
    'install state and duplicate protection are shared across screens',
    () async {
      final bridge = _MarketBridge()..pendingInstall = Completer<String>();
      final clients = GeneratedCoreProxyClients(bridge);
      final first = MarketInstallStateStore.of(clients);
      final second = MarketInstallStateStore.of(
        GeneratedCoreProxyClients(bridge),
      );
      expect(identical(first, second), isTrue);
      final install = first.install(entry);
      expect(second.isInstalling(entry.id), isTrue);
      await second.install(entry);
      expect(bridge.installCalls, 1);
      bridge.pendingInstall!.complete('v2');
      await install;
      expect(second.isInstalling(entry.id), isFalse);
      expect(second.stateFor(entry), MarketLocalInstallState.installed);
    },
  );

  test('failed updates preserve the previous installed marker', () async {
    final bridge = _MarketBridge()..versions['entry'] = 'v1';
    final store = MarketInstallStateStore.of(GeneratedCoreProxyClients(bridge));
    await store.refresh();
    bridge.installError = 'SHA-256 mismatch';
    await expectLater(store.install(entry), throwsStateError);
    expect(store.isInstalling(entry.id), isFalse);
    expect(store.stateFor(entry), MarketLocalInstallState.updateAvailable);
  });

  test(
    'selecting an old version records that version, not the latest',
    () async {
      final bridge = _MarketBridge();
      final store = MarketInstallStateStore.of(
        GeneratedCoreProxyClients(bridge),
      );
      await store.install(entry, versionId: 'v1');
      expect(bridge.requestedVersion, 'v1');
      expect(store.stateFor(entry), MarketLocalInstallState.updateAvailable);
    },
  );

  test('stale status refresh cannot erase a successful installation', () async {
    final bridge = _MarketBridge()
      ..pendingVersions = Completer<Map<String, String>>();
    final store = MarketInstallStateStore.of(GeneratedCoreProxyClients(bridge));
    final refresh = store.refresh();
    await store.install(entry);
    bridge.pendingVersions!.complete(<String, String>{});
    await refresh;
    expect(store.stateFor(entry), MarketLocalInstallState.installed);
  });

  testWidgets('market search matches entries across every listing page', (
    tester,
  ) async {
    final bridge = _MarketBridge()
      ..listPageLoader = (page) async => switch (page) {
        1 => _marketPage(1, 2, <MarketEntrySummary>[_entry()]),
        _ => _marketPage(2, 2, <MarketEntrySummary>[
          MarketEntrySummary.fromJson(<String, Object?>{
            ..._entry().toJson(),
            'id': 'later',
            'title': 'Later plugin',
          }),
        ]),
      };
    await tester.pumpWidget(_market(bridge, query: 'plugin'));
    await tester.pumpAndSettle();
    expect(find.text('Plugin'), findsOneWidget);
    expect(find.text('Later plugin'), findsOneWidget);

    await tester.pumpWidget(_market(bridge, query: 'later'));
    await tester.pumpAndSettle();
    expect(find.text('Plugin'), findsNothing);
    expect(find.text('Later plugin'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('market rows install inline without any popup', (tester) async {
    final bridge = _MarketBridge();
    await tester.pumpWidget(_market(bridge, query: 'plugin'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('获取'));
    await tester.pumpAndSettle();
    expect(bridge.installCalls, 1);
    expect(find.text('已安装'), findsOneWidget);
    expect(find.byType(SnackBar), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('successful installation updates detail inline with no popup', (
    tester,
  ) async {
    final bridge = _MarketBridge();
    await tester.pumpWidget(_detail(bridge));
    await tester.pumpAndSettle();
    await tester.tap(find.text('安装'));
    await tester.pumpAndSettle();
    expect(bridge.installCalls, 1);
    // The action shows its done state and the header badge agrees.
    expect(find.text('已安装'), findsNWidgets(2));
    expect(find.byIcon(Icons.check_rounded), findsOneWidget);
    expect(find.byType(SnackBar), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'failed installation still shows an error, never an installed badge',
    (tester) async {
      final bridge = _MarketBridge()..installError = 'Download failed';
      await tester.pumpWidget(_detail(bridge));
      await tester.pumpAndSettle();
      await tester.tap(find.text('安装'));
      await tester.pumpAndSettle();
      expect(find.byType(SnackBar), findsOneWidget);
      expect(find.textContaining('Download failed'), findsOneWidget);
      expect(find.text('已安装'), findsNothing);
      expect(find.text('安装'), findsOneWidget);
    },
  );

  testWidgets(
    'updatable details switch to installed after update without a popup',
    (tester) async {
      final bridge = _MarketBridge()..versions['entry'] = 'v1';
      await tester.pumpWidget(_detail(bridge));
      await tester.pumpAndSettle();
      expect(find.text('可更新'), findsOneWidget);
      await tester.tap(find.text('更新'));
      await tester.pumpAndSettle();
      expect(find.text('可更新'), findsNothing);
      expect(find.text('已安装'), findsNWidgets(2));
      expect(find.byType(SnackBar), findsNothing);
    },
  );
}

MarketEntrySummary _entry({String type = 'skill', String? latest = 'v2'}) =>
    MarketEntrySummary.fromJson({
      'id': 'entry',
      'type': type,
      'title': 'Plugin',
      'featured': false,
      'description': 'A plugin',
      'latestVersion': latest == null
          ? null
          : {'id': latest, 'version': '2.0.0', 'runtimePackageId': 'plugin'},
      'artifact': type == 'package'
          ? {'projectId': 'project', 'runtimePackageId': 'plugin'}
          : null,
    });

PublishablePackageSource _package(String? version) => PublishablePackageSource(
  packageName: 'plugin',
  displayName: 'Plugin',
  description: '',
  author: const [],
  sourcePath: '/plugins/plugin.js',
  sourceFileName: 'plugin.js',
  fileExtension: 'js',
  isToolPkg: false,
  inferredVersion: version,
  apiVersion: null,
);

Widget _detail(_MarketBridge bridge) => _testApp(
  Scaffold(
    body: MarketEntryDetailScreen(
      entryId: 'entry',
      clients: GeneratedCoreProxyClients(bridge),
    ),
  ),
);

Widget _market(_MarketBridge bridge, {required String query}) {
  final clients = GeneratedCoreProxyClients(bridge);
  return _testApp(
    Scaffold(
      body: MarketPane(
        controller: ExtensionCatalogController.of(clients),
        query: query,
      ),
    ),
  );
}

Widget _testApp(Widget child) {
  final controller = TopBarController();
  addTearDown(controller.dispose);
  return OperitTheme(
    initialThemePreferenceSnapshot:
        UserPreferencesManager.defaultThemePreferenceSnapshot,
    initialThemeIsReady: false,
    unconfiguredChildEnabled: true,
    hostInteractionHostsEnabled: false,
    child: TopBarScope(
      controller: controller,
      child: ScaffoldMessenger(child: child),
    ),
  );
}

Map<String, Object?> _marketPage(
  int page,
  int total,
  List<MarketEntrySummary> entries,
) => {
  'page': page,
  'pageSize': 1,
  'total': total,
  'items': entries.map((entry) => entry.toJson()).toList(),
};

class _MarketBridge extends OperitRuntimeBridge {
  final Map<String, String> versions = <String, String>{};
  int installCalls = 0;
  String? requestedVersion;
  String? installError;
  Completer<String>? pendingInstall;
  Completer<Map<String, String>>? pendingVersions;
  final List<int> listPages = [];
  int commentTotal = 0;
  final List<Map<String, Object?>> submittedComments = [];
  final Map<int, Map<String, Object?>> _postedComments = {};
  Future<Map<String, Object?>> Function(int page)? listPageLoader;

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    final Object? result;
    switch (request.methodName) {
      case 'getInstalledMarketVersions':
        result = pendingVersions == null
            ? Map<String, String>.of(versions)
            : await pendingVersions!.future;
      case 'getPublishablePackageSources':
        result = <Object?>[];
      case 'installMarketEntry':
        installCalls++;
        requestedVersion = (request.args as Map)['versionId'] as String?;
        if (installError != null) throw StateError(installError!);
        final version = pendingInstall == null
            ? requestedVersion ?? 'v2'
            : await pendingInstall!.future;
        versions[(request.args as Map)['entryId'] as String] = version;
        result = version;
      case 'get_list_page':
        final page = (request.args as Map)['page'] as int;
        listPages.add(page);
        result = listPageLoader != null
            ? await listPageLoader!(page)
            : {
                'page': 1,
                'pageSize': 50,
                'total': 1,
                'items': [_entry().toJson()],
              };
      case 'get_manifest':
        result = <String, Object?>{
          'ok': true,
          'marketVersion': 2,
          'generatedAt': null,
          'types': <Object?>[],
          'formatVersions': <Object?>[],
          'categories': <Object?>[],
          'states': <Object?>[],
        };
      case 'get_entry_by_id':
        result = _entry().toJson();
      case 'create_entry_comment':
      case 'reply_entry_comment':
        final args = Map<String, Object?>.from(request.args as Map);
        submittedComments.add(args);
        _postedComments[commentTotal] = {
          'body': args['body'],
          'parentId': args['parent_id'],
          'author': {'id': 'gh_1', 'login': 'tester', 'avatar': ''},
        };
        result = 'comment-${commentTotal++}';
      case 'get_comments_page':
        result = {
          'entryId': 'entry',
          'page': 1,
          'pageSize': 50,
          'total': commentTotal,
          'items': <Object?>[
            for (var i = 0; i < commentTotal; i++)
              {
                'id': 'comment-$i',
                'parentId': i == 22
                    ? 'comment-21'
                    : i == 21
                    ? 'comment-0'
                    : null,
                'body': i == 22
                    ? '收到，谢谢！更新后我再试试。'
                    : i == 21
                    ? '已经在做了，下个版本会支持批量导入。'
                    : i == 0
                    ? '能支持批量导入吗？现在需要一条条添加。'
                    : _commentBodies[i % _commentBodies.length],
                'createdAt':
                    '2026-11-${(i ~/ 3 + 6).toString().padLeft(2, '0')}T08:00:00Z',
                'author': {
                  'id': 'user-$i',
                  'login': i == 22
                      ? 'momo'
                      : i == 0
                      ? 'stone'
                      : ['林间', '小陆', 'Evan', '北岛', '阿白'][i % 5],
                  'avatar': '',
                },
                ...?_postedComments[i],
              },
          ],
        };
      case 'get_current_github_user':
        result = {'id': 1, 'login': 'tester'};
      case 'isLoggedIn':
        result = false;
      default:
        throw UnsupportedError('Unexpected Core call: ${request.methodName}');
    }
    return encodeCoreLink(<Object?>[0, result]);
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      const Stream<CoreEvent>.empty();
}

class _RankingRoute extends MarketBrowseScreenRoute {
  const _RankingRoute(this.clients)
    : super(heading: '下载排行榜', sort: MarketSortOption.downloads);
  final GeneratedCoreProxyClients clients;
  @override
  Widget build(BuildContext context) =>
      StoreBrowseScreen(heading: heading, initialSort: sort, clients: clients);
}

class _DetailProbeRoute extends OperitScreen {
  const _DetailProbeRoute() : super(routeTypeName: 'DetailProbe');
  @override
  Widget build(BuildContext context) => const Center(child: Text('插件详情'));
}

const _commentBodies = [
  '配置比想象中简单，文档里的例子可以直接用。',
  '连续用了几天很稳定，工作流里已经离不开这个插件了。',
  '希望后续支持批量导入，现在的体验已经很好了。',
  '更新后速度快了不少，感谢作者持续维护！',
  '手机上也能顺畅使用，界面很清爽。',
  '搭配自己的知识库效果不错，推荐试试。',
];

Future<void> _capture(
  WidgetTester tester,
  GlobalKey key,
  String directory,
  String name,
) async {
  final rect = tester.getRect(
    find.byWidgetPredicate(
      (widget) => widget is StoreDetailSection && widget.title == '评论与反馈',
    ),
  );
  await tester.runAsync(() async {
    final image =
        await (key.currentContext!.findRenderObject()! as RenderRepaintBoundary)
            .toImage(pixelRatio: 2);
    final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
    await Directory(directory).create(recursive: true);
    await File(
      '$directory/$name.png',
    ).writeAsBytes(bytes!.buffer.asUint8List());
    await File('$directory/$name.json').writeAsString(
      jsonEncode({
        'left': rect.left * 2,
        'top': rect.top * 2,
        'right': rect.right * 2,
        'bottom': rect.bottom * 2,
      }),
    );
    image.dispose();
  });
}
