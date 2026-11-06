import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/packages/components/StoreVisuals.dart';
import 'package:operit2/ui/features/packages/components/StoreCards.dart';
import 'package:operit2/ui/features/packages/screens/StoreBrowseScreen.dart';
import 'package:operit2/ui/features/packages/screens/MarketPane.dart';
import 'package:operit2/ui/features/packages/market/MarketStatsSupport.dart';
import 'package:operit2/ui/features/packages/screens/PackageManagerScreen.dart';
import 'package:operit2/ui/main/TopBarController.dart';
import 'package:operit2/ui/main/navigation/ToolPkgCatalogChangeBus.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  testWidgets('library groups every kind and regroups by status', (
    tester,
  ) async {
    final bridge = _CatalogBridge();
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Demo toolpkg'), findsOneWidget);
    expect(find.text('Demo skill'), findsOneWidget);
    expect(find.text('Demo mcp'), findsOneWidget);
    expect(_sectionHeader('插件'), findsOneWidget);
    expect(_sectionHeader('技能'), findsOneWidget);

    await tester.tap(find.text('按状态'));
    await tester.pumpAndSettle();
    expect(_sectionHeader('已启用'), findsOneWidget);
    expect(_sectionHeader('已停用'), findsOneWidget);
    expect(_sectionHeader('插件'), findsNothing);
    expect(bridge.snapshotReads, greaterThanOrEqualTo(1));
    expect(tester.takeException(), isNull);
  });

  testWidgets('a library switch enables only its own extension', (
    tester,
  ) async {
    final changes = <bool>[];
    final bridge = _CatalogBridge()..onEnabledChanged = changes.add;
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();

    final switches = find.byType(Switch);
    expect(switches, findsNWidgets(3));
    expect(tester.widget<Switch>(switches.first).value, isFalse);

    await tester.tap(switches.first);
    await tester.pumpAndSettle();

    expect(changes, <bool>[true]);
    expect(
      tester
          .widgetList<Switch>(find.byType(Switch))
          .every((item) => item.value),
      isTrue,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('official extensions with a declared scope install directly', (
    tester,
  ) async {
    final bridge = _CatalogBridge();
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();
    await tester.tap(find.text('市场'));
    await tester.pumpAndSettle();

    expect(find.text('Official Extra'), findsOneWidget);
    final install = find.text('获取').first;
    await tester.ensureVisible(install);
    await tester.tap(install);
    await tester.pumpAndSettle();

    expect(bridge.imports, <String>['official_extra']);
    expect(tester.takeException(), isNull);
  });

  testWidgets('load diagnostics only offer deletion for installed sources', (
    tester,
  ) async {
    final bridge = _CatalogBridge();
    bridge.diagnostics.addAll(const <core_proxy.ExtensionCatalogDiagnostic>[
      core_proxy.ExtensionCatalogDiagnostic(
        sourcePath: 'assets/plugins/buildin/failed.toolpkg',
        packageId: null,
        displayName: 'Bundled load failure',
        code: 'toolpkg_load_failed',
        message: 'Syntax error',
        packageKind: 'toolpkg',
        canDelete: false,
      ),
      core_proxy.ExtensionCatalogDiagnostic(
        sourcePath: '/runtime/extensions/device/packages/broken.toolpkg',
        packageId: null,
        displayName: 'Broken installed archive',
        code: 'toolpkg_load_failed',
        message: 'Invalid archive',
        packageKind: 'toolpkg',
        canDelete: true,
      ),
    ]);
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();
    await tester.tap(find.text('2 个文件加载失败'));
    await tester.pumpAndSettle();

    expect(find.byTooltip('此加载错误没有可删除的安装来源'), findsOneWidget);
    expect(find.byTooltip('删除失败来源'), findsOneWidget);
    await tester.tap(find.byTooltip('删除失败来源'));
    await tester.pumpAndSettle();

    expect(bridge.deletedDiagnostics, <String>[
      '/runtime/extensions/device/packages/broken.toolpkg',
    ]);
    expect(find.text('Broken installed archive'), findsNothing);
    expect(find.text('Bundled load failure'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('location grouping separates shared and device-only extensions', (
    tester,
  ) async {
    final bridge = _CatalogBridge();
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();
    expect(find.text('设备'), findsNothing);
    await tester.tap(find.text('按位置'));
    await tester.pumpAndSettle();

    expect(_sectionHeader('设备空间'), findsOneWidget);
    expect(_sectionHeader('仅本机'), findsOneWidget);
    expect(find.text('Demo skill'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  for (final failExtraPage in [false, true]) {
    testWidgets(
      'recommendations read at most three recent pages (failure: $failExtraPage)',
      (tester) async {
        final bridge = _CatalogBridge()..marketTotal = 20;
        core_proxy.MarketEntrySummary entry(String id, int downloads) =>
            core_proxy.MarketEntrySummary.fromJson({
              'id': id,
              'type': 'package',
              'title': id,
              'updatedAt': '2026-11-05T12:00:00Z',
              'downloads': downloads,
            });
        bridge.marketPages[('updated', 1)] = [entry('first', 1)];
        bridge.marketPages[('updated', 2)] = [entry('second', 10)];
        bridge.marketPages[('updated', 3)] = [entry('third', 1000)];
        bridge.marketPages[('downloads', 1)] = [
          entry('old-downloads', 1000000),
        ];
        bridge.marketPages[('likes', 1)] = [entry('old-likes', 1000000)];
        if (failExtraPage) bridge.failedMarketPage = 2;
        await tester.pumpWidget(_manager(bridge));
        await tester.pumpAndSettle();
        await tester.tap(find.text('市场'));
        await tester.pumpAndSettle();
        expect(
          bridge.marketRequests.where((request) => request.$1 == 'updated'),
          [('updated', 1), ('updated', 2), ('updated', 3)],
        );
        expect(
          tester
              .widget<StoreFeaturedBanner>(
                find.byType(StoreFeaturedBanner).first,
              )
              .entry
              .id,
          'third',
        );
        expect(
          tester
              .widgetList<StoreFeaturedBanner>(find.byType(StoreFeaturedBanner))
              .map((banner) => banner.entry.id),
          isNot(contains(startsWith('old-'))),
        );
        expect(find.text('推荐 · 插件'), findsWidgets);
        expect(find.text('市场暂时无法连接'), findsNothing);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
  }

  testWidgets(
    'recent listings group rows by date; other sorts have no date headings',
    (tester) async {
      final bridge = _CatalogBridge();
      final entries = [
        for (final (id, date) in [
          ('a', '2026-11-05'),
          ('b', '2026-11-05'),
          ('c', '2026-11-04'),
        ])
          core_proxy.MarketEntrySummary.fromJson({
            'id': id,
            'type': 'package',
            'title': id,
            'updatedAt': '${date}T12:00:00Z',
          }),
      ];
      bridge.marketPages[('updated', 1)] = entries;
      bridge.marketPages[('downloads', 1)] = entries;
      final sort = ValueNotifier(MarketSortOption.updated);
      addTearDown(sort.dispose);
      await tester.pumpWidget(
        _manager(
          bridge,
          child: ValueListenableBuilder(
            valueListenable: sort,
            builder: (context, value, child) => StoreMarketListing(
              clients: GeneratedCoreProxyClients(bridge),
              sort: value,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('2026-11-05'), findsOneWidget);
      expect(find.text('2026-11-04'), findsOneWidget);
      expect(find.text('a'), findsOneWidget);
      expect(find.text('b'), findsOneWidget);
      sort.value = MarketSortOption.downloads;
      await tester.pumpAndSettle();
      expect(find.text('2026-11-05'), findsNothing);
      expect(find.text('2026-11-04'), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'Chinese market categories show localized titles and descriptions',
    (tester) async {
      tester.platformDispatcher.localesTestValue = const [Locale('zh')];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      final bridge = _CatalogBridge();
      bridge.categories.add(
        const core_proxy.MarketCategoryInfo(
          id: 'search_research',
          name: 'Search & Research',
          description: 'Search engines, research tools, knowledge retrieval',
        ),
      );
      await tester.pumpWidget(_manager(bridge));
      await tester.pumpAndSettle();
      await tester.tap(find.text('市场'));
      await tester.pumpAndSettle();
      await tester.scrollUntilVisible(
        find.text('搜索与研究'),
        250,
        scrollable: find.descendant(
          of: find.byType(MarketPane),
          matching: find.byWidgetPredicate(
            (widget) =>
                widget is Scrollable &&
                widget.axisDirection == AxisDirection.down,
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('Search & Research'), findsNothing);
      expect(find.text('搜索引擎、研究工具与知识检索'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );
}

Finder _sectionHeader(String title) => find.byWidgetPredicate(
  (widget) => widget is StoreSectionHeader && widget.title == title,
);

/// Renders the shell top bar the screen fills, so tabs are reachable.
Widget _manager(_CatalogBridge bridge, {Widget? child}) {
  final topBar = TopBarController();
  addTearDown(topBar.dispose);
  return OperitTheme(
    initialThemePreferenceSnapshot:
        UserPreferencesManager.defaultThemePreferenceSnapshot,
    initialThemeIsReady: false,
    unconfiguredChildEnabled: true,
    hostInteractionHostsEnabled: false,
    child: TopBarScope(
      controller: topBar,
      child: AnimatedBuilder(
        animation: topBar,
        child:
            child ??
            PackageManagerScreen(clients: GeneratedCoreProxyClients(bridge)),
        builder: (context, child) => Scaffold(
          appBar: AppBar(
            title: topBar.titleContent?.content(context),
            actions: topBar.actions?.call(context),
          ),
          body: child,
        ),
      ),
    ),
  );
}

core_proxy.ExtensionCatalogEntry _entry({
  required String id,
  required String kind,
  required String source,
  bool enabled = false,
  String scope = 'device',
}) {
  return core_proxy.ExtensionCatalogEntry(
    id: id,
    kind: kind,
    source: source,
    displayName: 'Demo $id',
    description: 'Demo description',
    version: '1.0.0',
    authors: const <String>['Operit'],
    category: kind,
    scope: scope,
    activationState: enabled ? 'enabled' : 'disabled',
    runtimeState: 'ready',
    updateState: 'noSource',
    isCustomized: false,
    marketEntryId: null,
    installedMarketVersionId: null,
    logoResourceKey: null,
    logoMimeType: null,
    toolCount: 0,
    subpackageCount: 0,
    uiModuleCount: 0,
    hasUi: false,
    chatSidebarEntryIds: const <String>[],
    dependencyIssueCount: 0,
  );
}

class _CatalogBridge extends OperitRuntimeBridge {
  int snapshotReads = 0;
  final marketPages = <(String, int), List<core_proxy.MarketEntrySummary>>{};
  final marketRequests = <(String, int)>[];
  int marketTotal = 0;
  int? failedMarketPage;
  final categories = <core_proxy.MarketCategoryInfo>[];
  bool enabled = false;
  final imports = <String>[];
  final scopeMoves = <String>[];
  final diagnostics = <core_proxy.ExtensionCatalogDiagnostic>[];
  final deletedDiagnostics = <String>[];
  void Function(bool enabled)? onEnabledChanged;

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    return switch (request.propertyName) {
      'extensionCatalogRevisionFlow' => Stream<CoreEvent>.multi((controller) {
        controller.add(_event(request, 1));
        final subscription = ToolPkgCatalogChangeBus.listenChanges((_) {
          controller.add(_event(request, 1));
        });
        controller.onCancel = () {
          unawaited(subscription.cancel());
        };
      }),
      // Watches stay open like Core's, so nothing resubscribes mid-test.
      'deviceSpaceSnapshotFlow' => Stream<CoreEvent>.multi((controller) {
        controller.add(_event(request, _deviceSpace().toJson()));
      }),
      'spaceSyncProgressFlow' => Stream<CoreEvent>.multi((controller) {
        controller.add(_event(request, <Object?>[]));
      }),
      _ => throw StateError('Unexpected watch: ${request.propertyName}'),
    };
  }

  CoreEvent _event(CoreWatchRequest request, Object? value) => CoreEvent.raw(
    requestId: request.requestId,
    target: request.target,
    propertyName: request.propertyName,
    kind: 'Changed',
    valueBytes: encodeCoreLink(value),
    decodeValue: decodeCoreLink<Object?>,
  );

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async =>
      encodeCoreLink(<Object?>[0, await call(request)]);

  @override
  Future<Object?> call(CoreCallRequest request) async {
    switch (request.methodName) {
      case 'getExtensionCatalogSnapshot':
        snapshotReads += 1;
        return _snapshot().toJson();
      case 'deleteExtensionCatalogDiagnostic':
        final diagnostic = (request.args as Map)['diagnostic'] as Map;
        final path = diagnostic['sourcePath'] as String;
        deletedDiagnostics.add(path);
        diagnostics.removeWhere((item) => item.sourcePath == path);
        return true;
      case 'getInstalledMarketVersions':
        return <String, String>{};
      case 'getPublishablePackageSources':
        return <Object?>[];
      case 'isLoggedIn':
        return false;
      case 'get_list_page':
        final args = request.args as Map;
        final sort = args['sort'] as String;
        final page = args['page'] as int;
        marketRequests.add((sort, page));
        if (page == failedMarketPage) {
          throw StateError('Extra page unavailable');
        }
        return core_proxy.MarketListPage(
          generatedAt: null,
          sort: sort,
          page: page,
          pageSize: 2,
          total: marketTotal,
          items: marketPages[(sort, page)] ?? <core_proxy.MarketEntrySummary>[],
        ).toJson();
      case 'get_manifest':
        return core_proxy.MarketManifest(
          ok: true,
          marketVersion: 1,
          generatedAt: null,
          types: <core_proxy.MarketTypeInfo>[],
          formatVersions: <core_proxy.MarketFormatInfo>[],
          categories: categories,
          states: <core_proxy.MarketStateInfo>[],
        ).toJson();
      case 'enableToolPkgContainer':
        enabled = true;
        onEnabledChanged?.call(true);
        return 'Enabled';
      case 'importBundledExternalToolPkgContainer':
        imports.add((request.args as Map)['containerPackageName'] as String);
        return 'Imported';
      case 'setExtensionScope':
        scopeMoves.add((request.args as Map)['id'] as String);
        return null;
    }
    throw StateError('Unexpected catalog call: ${request.methodName}');
  }

  core_proxy.RuntimeDeviceSpaceSnapshot _deviceSpace() {
    return const core_proxy.RuntimeDeviceSpaceSnapshot(
      space: core_proxy.CoreSpace(
        spaceId: 'space',
        spaceName: 'Home',
        spaceRevision: 1,
        members: <String>['this'],
      ),
      topology: core_proxy.RuntimeDeviceSpaceTopology(
        currentDeviceId: 'this',
        devices: <core_proxy.RuntimeDeviceSpaceDevice>[
          core_proxy.RuntimeDeviceSpaceDevice(
            deviceId: 'this',
            userName: 'tester',
            deviceName: 'Desk',
            platform: 'windows',
            model: 'PC',
            coreVersion: null,
            online: true,
            relayHops: null,
            relayPath: <String>[],
            currentIdentity: null,
          ),
        ],
        connections: <core_proxy.RuntimeDeviceSpaceConnection>[],
      ),
    );
  }

  core_proxy.ExtensionCatalogSnapshot _snapshot() {
    final toolpkg = _entry(
      id: 'toolpkg',
      kind: 'toolpkg',
      source: 'builtin',
      enabled: enabled,
    );
    return core_proxy.ExtensionCatalogSnapshot(
      revision: 1,
      installed: <core_proxy.ExtensionCatalogEntry>[
        toolpkg,
        _entry(
          id: 'skill',
          kind: 'skill',
          source: 'market',
          enabled: true,
          scope: 'space',
        ),
        _entry(id: 'mcp', kind: 'mcp', source: 'manual', enabled: true),
      ],
      bundledAvailable: const <core_proxy.ExtensionCatalogCandidate>[
        core_proxy.ExtensionCatalogCandidate(
          id: 'official_extra',
          kind: 'toolpkg',
          displayName: 'Official Extra',
          description: 'Offline extension',
          version: '1.1.0',
          authors: <String>['Operit'],
          category: 'ToolPkg',
          requiredScope: 'device',
          toolCount: 1,
          subpackageCount: 0,
        ),
      ],
      diagnostics: List<core_proxy.ExtensionCatalogDiagnostic>.of(diagnostics),
    );
  }
}
