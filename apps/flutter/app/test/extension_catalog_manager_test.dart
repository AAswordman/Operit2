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
import 'package:operit2/ui/features/packages/model/ExtensionCatalogController.dart';
import 'package:operit2/ui/features/packages/model/ToolPkgDependencyGraph.dart';
import 'package:operit2/ui/features/packages/components/ExtensionAvatar.dart';
import 'package:operit2/ui/features/packages/components/ToolPkgDependencyTree.dart';
import 'package:operit2/ui/features/packages/screens/StoreBrowseScreen.dart';
import 'package:operit2/ui/features/packages/screens/MarketPane.dart';
import 'package:operit2/ui/features/packages/screens/OfficialExtensionsScreen.dart';
import 'package:operit2/ui/features/packages/market/MarketStatsSupport.dart';
import 'package:operit2/ui/features/packages/screens/PackageManagerScreen.dart';
import 'package:operit2/ui/features/packages/screens/ExtensionDetailScreen.dart';
import 'package:operit2/ui/main/MainLayoutController.dart';
import 'package:operit2/ui/main/TopBarController.dart';
import 'package:operit2/ui/main/components/AppContent.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/main/navigation/ToolPkgCatalogChangeBus.dart';
import 'package:operit2/ui/main/screens/OperitScreens.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  group('local package import scope', () {
    for (final installedScope in <String>['device', 'space']) {
      for (final requireToolPkg in <bool>[false, true]) {
        test('same ID ToolPkg update keeps $installedScope scope '
            '(requireToolPkg: $requireToolPkg)', () async {
          const id = 'local.plugin';
          const path = '/operitapp/data/downloads/updated.toolpkg';
          final bridge = _CatalogBridge()
            ..fileInstalledEntry = _entry(
              id: id,
              kind: 'toolpkg',
              source: 'manual',
              scope: installedScope,
              enabled: true,
            )
            ..fileImportedEntry = _entry(
              id: id,
              kind: 'toolpkg',
              source: 'manual',
              scope: installedScope,
              enabled: true,
              version: '2.0.0',
            )
            ..fileImportResult = _fileImportResult(
              id: id,
              format: 'toolpkg',
              scope: installedScope,
            )
            ..rejectScopeChanges = true;
          final controller = ExtensionCatalogController.of(
            GeneratedCoreProxyClients(bridge),
          );
          addTearDown(controller.dispose);

          await controller.importPackageFile(
            path,
            scope: defaultExtensionInstallScope,
            requireToolPkg: requireToolPkg,
          );

          expect(bridge.packageFileImports, <String>[path]);
          expect(bridge.scopeMoveRequests, isEmpty);
          expect(bridge.scopeMoves, isEmpty);
          final entry = controller.findEntry('toolpkg', id)!;
          expect(entry.scope, installedScope);
          expect(entry.version, '2.0.0');
          expect(entry.activationState, 'enabled');
          expect(bridge.snapshotReads, 1);
          expect(controller.isBusy('import:$path'), isFalse);
        });
      }
    }

    for (final coreScope in <String>['device', 'space']) {
      test('new ToolPkg keeps Core-selected $coreScope scope', () async {
        const id = 'new.plugin';
        const path = '/operitapp/data/downloads/new.toolpkg';
        final bridge = _CatalogBridge()
          ..fileImportedEntry = _entry(
            id: id,
            kind: 'toolpkg',
            source: 'manual',
            scope: coreScope,
          )
          ..fileImportResult = _fileImportResult(
            id: id,
            format: 'toolpkg',
            scope: coreScope,
          )
          ..rejectScopeChanges = true;
        final controller = ExtensionCatalogController.of(
          GeneratedCoreProxyClients(bridge),
        );
        addTearDown(controller.dispose);

        await controller.importPackageFile(
          path,
          scope: defaultExtensionInstallScope,
          requireToolPkg: true,
        );

        expect(bridge.packageFileImports, <String>[path]);
        expect(bridge.scopeMoveRequests, isEmpty);
        expect(controller.findEntry('toolpkg', id)!.scope, coreScope);
        expect(
          controller.findEntry('toolpkg', id)!.activationState,
          'disabled',
        );
      });
    }

    test(
      'failed Core import refreshes the old owner without moving it',
      () async {
        const id = 'local.plugin';
        const path = '/operitapp/data/downloads/broken.toolpkg';
        final bridge = _CatalogBridge()
          ..fileInstalledEntry = _entry(
            id: id,
            kind: 'toolpkg',
            source: 'manual',
            scope: 'space',
          )
          ..fileImportFailure = StateError('Core import failed');
        final controller = ExtensionCatalogController.of(
          GeneratedCoreProxyClients(bridge),
        );
        addTearDown(controller.dispose);
        final changes = <ToolPkgCatalogChange>[];
        final subscription = ToolPkgCatalogChangeBus.listenChanges(changes.add);
        addTearDown(subscription.cancel);

        await expectLater(
          controller.importPackageFile(
            path,
            scope: defaultExtensionInstallScope,
            requireToolPkg: true,
          ),
          throwsA(
            predicate<Object>(
              (error) => error.toString().contains('Core import failed'),
            ),
          ),
        );

        expect(bridge.packageFileImports, <String>[path]);
        expect(bridge.scopeMoveRequests, isEmpty);
        expect(bridge.snapshotReads, 1);
        expect(controller.findEntry('toolpkg', id)!.scope, 'space');
        expect(controller.findEntry('toolpkg', id)!.version, '1.0.0');
        expect(controller.isBusy('import:$path'), isFalse);
        expect(changes, isEmpty);
      },
    );

    for (final (format, extension) in <(String, String)>[
      ('javascript', 'js'),
      ('typescript', 'ts'),
      ('hjson', 'hjson'),
    ]) {
      test('$format import still applies the requested scope', () async {
        const id = 'local.script';
        final path = '/operitapp/data/downloads/source.$extension';
        final bridge = _CatalogBridge()
          ..fileImportedEntry = _entry(id: id, kind: 'script', source: 'manual')
          ..fileImportResult = _fileImportResult(
            id: id,
            format: format,
            scope: 'device',
          );
        final controller = ExtensionCatalogController.of(
          GeneratedCoreProxyClients(bridge),
        );
        addTearDown(controller.dispose);

        await controller.importPackageFile(
          path,
          scope: 'space',
          requireToolPkg: false,
        );

        expect(bridge.packageFileImports, <String>[path]);
        expect(bridge.scopeMoves, <String>[id]);
        expect(
          bridge.scopeMoveRequests,
          <({String kind, String id, String scope})>[
            (kind: 'package', id: id, scope: 'space'),
          ],
        );
        expect(controller.findEntry('script', id)!.scope, 'space');
      });
    }
  });

  for (final kind in <String>['skill', 'toolpkg']) {
    testWidgets('removing a $kind returns to the searchable package list', (
      tester,
    ) async {
      if (kind == 'toolpkg') {
        tester.view.physicalSize = const Size(390, 844);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
      }
      final bridge = _CatalogBridge()..externalToolpkg = kind == 'toolpkg';
      final clients = GeneratedCoreProxyClients(bridge);
      final topBar = TopBarController();
      final layout = MainLayoutController();
      final router = AppRouterState(RouteEntry(routeId: 'packages'));
      addTearDown(topBar.dispose);
      addTearDown(layout.dispose);
      addTearDown(router.dispose);
      addTearDown(AppRouterGateway.clear);
      AppRouterGateway.install(
        handler: (routeId, args, source) {
          router.navigate(routeId: routeId, args: args, source: source);
          topBar.clear();
        },
        reset: (routeId, args, source) {
          router.resetTo(
            RouteEntry(routeId: routeId, args: args, source: source),
          );
          topBar.clear();
        },
      );
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
              child: ListenableBuilder(
                listenable: router,
                builder: (context, child) {
                  final entry = router.currentEntry;
                  final inList = entry.routeId == 'packages';

                  /// Uses the shell back stack for Navigator.maybePop requests.
                  void goBack() {
                    router.pop();
                    topBar.clear();
                  }

                  return PopScope(
                    canPop: false,
                    onPopInvokedWithResult: (didPop, result) {
                      if (!didPop) goBack();
                    },
                    child: Scaffold(
                      body: AppContent(
                        routerState: router,
                        currentScreen: _CatalogScreen(
                          inList
                              ? PackageManagerScreen(clients: clients)
                              : ExtensionDetailScreen(
                                  kind: kind,
                                  id: kind,
                                  clients: clients,
                                ),
                          inList: inList,
                        ),
                        currentRouteEntry: entry,
                        currentRouteTitle: inList ? '包管理' : '扩展详情',
                        useTabletLayout: false,
                        isTabletSidebarExpanded: false,
                        canGoBack: router.canPop,
                        enableNavigationAnimation: true,
                        isNavigatingBack: inList,
                        topBarController: topBar,
                        appBarEntries: const <NavigationEntrySpec>[],
                        onGoBack: goBack,
                        onNavigationButtonPressed: () {},
                        onAppBarEntrySelected: (_) {},
                      ),
                    ),
                  );
                },
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byTooltip('搜索（Ctrl+F）'), findsOneWidget);
      await tester.tap(find.text('Demo $kind'));
      await tester.pumpAndSettle();
      await tester.scrollUntilVisible(
        find.text('移除扩展'),
        350,
        scrollable: find
            .descendant(
              of: find.byType(ExtensionDetailScreen),
              matching: find.byWidgetPredicate(
                (widget) =>
                    widget is Scrollable &&
                    widget.axisDirection == AxisDirection.down,
              ),
            )
            .first,
      );
      await tester.ensureVisible(find.text('移除'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('移除'));
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.widgetWithText(FilledButton, '移除'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, '移除'));
      await tester.pumpAndSettle();
      expect(
        kind == 'skill' ? bridge.skillDeleted : bridge.toolpkgDeleted,
        isTrue,
      );
      expect(router.currentEntry.routeId, 'packages');
      expect(find.byTooltip('搜索（Ctrl+F）'), findsOneWidget);
      expect(find.byTooltip('更多'), findsOneWidget);
      await tester.tap(find.byTooltip('搜索（Ctrl+F）'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), 'demo');
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('关闭搜索'));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('更多'));
      await tester.pumpAndSettle();
      expect(find.text('重新加载目录'), findsOneWidget);
      await tester.tap(find.text('重新加载目录'));
      await tester.pumpAndSettle();
      expect(find.byTooltip('搜索（Ctrl+F）'), findsOneWidget);
      expect(find.byTooltip('更多'), findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  }

  for (final hasStandaloneRoute in [false, true]) {
    testWidgets('detail opens only standalone routes ($hasStandaloneRoute)', (
      tester,
    ) async {
      const title = core_proxy.LocalizedText(values: {'en': 'Test UI'});
      final entries = [
        (id: 'conversation', surface: 'workspace'),
        if (hasStandaloneRoute) (id: 'settings', surface: 'toolbox'),
      ];
      final bridge = _CatalogBridge()
        ..enabled = true
        ..runtime = _toolpkgRuntime(
          uiRoutes: [
            for (final entry in entries)
              core_proxy.ToolPkgUiRouteRuntime(
                id: entry.id,
                routeId: 'toolpkg:toolpkg:ui:${entry.id}',
                runtime: 'compose_dsl',
                screen: '${entry.id}.ui.js',
                screenExport: null,
                title: title,
                keepAlive: false,
              ),
          ],
          navigationEntries: [
            for (final entry in entries)
              core_proxy.ToolPkgNavigationEntryRuntime(
                id: entry.id,
                routeId: 'toolpkg:toolpkg:ui:${entry.id}',
                surface: entry.surface,
                params: null,
                title: title,
                icon: null,
                action: null,
                order: 0,
              ),
          ],
        );
      await tester.pumpWidget(
        _manager(
          bridge,
          child: ExtensionDetailScreen(
            kind: 'toolpkg',
            id: 'toolpkg',
            clients: GeneratedCoreProxyClients(bridge),
          ),
        ),
      );
      await tester.pumpAndSettle();

      final open = find.text('打开');
      expect(open, hasStandaloneRoute ? findsOneWidget : findsNothing);
      if (hasStandaloneRoute) {
        await tester.tap(open);
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 400));
        expect(bridge.requestedUiModules, ['settings']);
      }
    });
  }

  for (final hasRequired in <bool>[true, false]) {
    testWidgets('detail shows optional dependencies (required: $hasRequired)', (
      tester,
    ) async {
      final bridge = _CatalogBridge()
        ..runtime = _toolpkgRuntime(
          requires: hasRequired
              ? const [
                  core_proxy.ToolPkgManifestRequirement(
                    id: 'com.operit.subagents',
                    description: '',
                    minVersion: '0.1.4',
                    maxVersion: null,
                  ),
                ]
              : const [],
          optionalRequires: const [
            core_proxy.ToolPkgManifestRequirement(
              id: 'com.operit.qqbot_bundle',
              description: 'QQ 渠道',
              minVersion: '1.1.6',
              maxVersion: '2.0.0',
            ),
            core_proxy.ToolPkgManifestRequirement(
              id: 'com.operit.wechat_bot',
              description: '',
              minVersion: '1.0.7',
              maxVersion: null,
            ),
          ],
        );
      await tester.pumpWidget(
        _manager(
          bridge,
          child: ExtensionDetailScreen(
            kind: 'toolpkg',
            id: 'toolpkg',
            clients: GeneratedCoreProxyClients(bridge),
          ),
        ),
      );
      await tester.pumpAndSettle();

      final tree = find.byType(ToolPkgDependencyTree);
      await tester.ensureVisible(tree);
      expect(find.text('插件依赖'), findsOneWidget);
      expect(find.text('可选 0/2'), findsOneWidget);
      expect(find.text('com.operit.qqbot_bundle'), findsOneWidget);
      expect(find.text('≥ 1.1.6 · ≤ 2.0.0'), findsOneWidget);
      expect(find.text('com.operit.wechat_bot'), findsOneWidget);
      expect(find.text('≥ 1.0.7'), findsOneWidget);
      expect(find.text('必需 0/1'), hasRequired ? findsOneWidget : findsNothing);
      if (hasRequired) {
        expect(find.text('com.operit.subagents'), findsOneWidget);
        expect(find.text('≥ 0.1.4'), findsOneWidget);
      }
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets(
    'installed dependencies show identity, status and open detail routes',
    (tester) async {
      tester.view.physicalSize = const Size(400, 1800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final bridge = _CatalogBridge()
        ..runtime = _toolpkgRuntime(
          requires: [
            _requirement('助手', min: '1.0.0'),
            _requirement('旧版本', min: '2.0.0'),
            _requirement('停用'),
            _requirement('缺失'),
          ],
        )
        ..dependencyEntries.addAll([
          _entry(id: '助手', kind: 'toolpkg', source: 'manual', enabled: true),
          _entry(id: '旧版本', kind: 'toolpkg', source: 'manual', enabled: true),
          _entry(id: '停用', kind: 'toolpkg', source: 'manual'),
        ]);
      String? route;
      Map<String, Object?>? routeArgs;
      AppRouterGateway.install(
        handler: (routeId, args, source) {
          route = routeId;
          routeArgs = args;
        },
        reset: (routeId, args, source) {},
      );
      addTearDown(AppRouterGateway.clear);
      await tester.pumpWidget(
        _manager(
          bridge,
          child: ExtensionDetailScreen(
            kind: 'toolpkg',
            id: 'toolpkg',
            clients: GeneratedCoreProxyClients(bridge),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.byType(ToolPkgDependencyTree));
      await tester.pumpAndSettle();
      expect(find.text('1/4'), findsOneWidget);
      expect(find.text('必需 1/4'), findsOneWidget);
      for (final id in ['助手', '旧版本', '停用']) {
        expect(find.text('Demo $id'), findsOneWidget);
        expect(
          find.byTooltip(
            'Demo $id · ${id == '助手'
                ? '已满足'
                : id == '旧版本'
                ? '版本不满足'
                : '未启用'} · 查看版本要求',
          ),
          findsOneWidget,
        );
        await tester.ensureVisible(find.text('Demo $id'));
        await tester.pumpAndSettle();
        await tester.tap(find.text('Demo $id'));
        await tester.pumpAndSettle();
        expect(route, 'native.extension_detail');
        expect(routeArgs, containsPair('kind', 'toolpkg'));
        expect(routeArgs, containsPair('id', id));
      }
      expect(find.byType(CatalogEntryAvatar), findsNWidgets(4));
      await tester.ensureVisible(find.text('缺失'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('缺失'));
      await tester.pumpAndSettle();
      expect(find.text('未安装'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'detail expands and deduplicates required and optional dependency chains',
    (tester) async {
      tester.view.physicalSize = const Size(400, 1800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final bridge = _CatalogBridge()
        ..runtime = _toolpkgRuntime(
          requires: [_requirement('助手')],
          optionalRequires: [_requirement('QQ')],
        )
        ..dependencyRuntimes.addAll([
          _toolpkgRuntime(
            id: '助手',
            requires: [_requirement('共享'), _requirement('缺失')],
          ),
          _toolpkgRuntime(
            id: 'QQ',
            requires: [_requirement('共享'), _requirement('渠道服务')],
          ),
          _toolpkgRuntime(id: '共享'),
        ])
        ..dependencyEntries.addAll([
          for (final id in ['助手', 'QQ', '共享'])
            _entry(id: id, kind: 'toolpkg', source: 'manual', enabled: true),
        ]);
      await tester.pumpWidget(
        _manager(
          bridge,
          child: ExtensionDetailScreen(
            kind: 'toolpkg',
            id: 'toolpkg',
            clients: GeneratedCoreProxyClients(bridge),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.byType(ToolPkgDependencyTree));
      await tester.pumpAndSettle();
      expect(find.text('必需 1/3'), findsOneWidget);
      expect(find.text('可选 0/2'), findsOneWidget);
      // Shared nodes appear as a link under their second parent, while the
      // summary still counts each plugin once.
      expect(find.text('Demo 共享'), findsNWidgets(2));
      expect(find.byTooltip('共享依赖，已在树中展开'), findsOneWidget);
      expect(
        tester.getTopLeft(find.text('Demo 共享').first).dx,
        greaterThan(tester.getTopLeft(find.text('Demo 助手')).dx),
      );
      await tester.ensureVisible(
        find.byTooltip('Demo 共享 · 已满足 · 查看版本要求').first,
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('Demo 共享 · 已满足 · 查看版本要求').first);
      await tester.pumpAndSettle();
      expect(find.text('Demo 助手需要'), findsOneWidget);
      expect(find.text('Demo QQ需要'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'dependency detail navigation restores each page scroll and releases popped pages',
    (tester) async {
      tester.view.physicalSize = const Size(390, 720);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final bridge = _CatalogBridge()
        ..runtime = _toolpkgRuntime(
          requires: [
            _requirement('child'),
            for (var i = 0; i < 18; i++) _requirement('root.missing.$i'),
          ],
        )
        ..dependencyRuntimes.addAll([
          _toolpkgRuntime(
            id: 'child',
            requires: [
              _requirement('leaf'),
              for (var i = 0; i < 12; i++) _requirement('child.missing.$i'),
            ],
          ),
          _toolpkgRuntime(id: 'leaf'),
        ])
        ..dependencyEntries.addAll([
          for (final id in ['child', 'leaf'])
            _entry(id: id, kind: 'toolpkg', source: 'manual', enabled: true),
        ]);
      final clients = GeneratedCoreProxyClients(bridge);
      final topBar = TopBarController();
      final layout = MainLayoutController();
      final router = AppRouterState(
        RouteEntry(
          routeId: 'native.extension_detail',
          args: {'kind': 'toolpkg', 'id': 'toolpkg'},
        ),
      );
      addTearDown(topBar.dispose);
      addTearDown(layout.dispose);
      addTearDown(router.dispose);
      addTearDown(AppRouterGateway.clear);
      var navigatingBack = false;
      void back() {
        navigatingBack = true;
        router.pop();
        topBar.clear();
      }

      AppRouterGateway.install(
        handler: (routeId, args, source) {
          navigatingBack = false;
          router.navigate(routeId: routeId, args: args, source: source);
          topBar.clear();
        },
        reset: (routeId, args, source) {
          router.resetTo(
            RouteEntry(routeId: routeId, args: args, source: source),
          );
        },
      );
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
              child: ListenableBuilder(
                listenable: router,
                builder: (context, _) {
                  final entry = router.currentEntry;
                  return AppContent(
                    routerState: router,
                    currentScreen: _DependencyDetailRoute(
                      id: entry.args['id']! as String,
                      clients: clients,
                    ),
                    currentRouteEntry: entry,
                    currentRouteTitle: '扩展详情',
                    useTabletLayout: false,
                    isTabletSidebarExpanded: false,
                    canGoBack: router.canPop,
                    enableNavigationAnimation: true,
                    isNavigatingBack: navigatingBack,
                    topBarController: topBar,
                    appBarEntries: const [],
                    onGoBack: back,
                    onNavigationButtonPressed: () {},
                    onAppBarEntrySelected: (_) {},
                  );
                },
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      Finder page(String id) => find.byWidgetPredicate(
        (widget) => widget is ExtensionDetailScreen && widget.id == id,
        skipOffstage: false,
      );
      ScrollPosition position(String id) => tester
          .state<ScrollableState>(
            find
                .descendant(
                  of: page(id),
                  matching: find.byType(Scrollable),
                  skipOffstage: false,
                )
                .first,
          )
          .position;
      final originalRoot = tester.state(page('toolpkg'));
      await tester.ensureVisible(find.text('Demo child').first);
      await tester.pumpAndSettle();
      final rootOffset = position('toolpkg').pixels;
      expect(rootOffset, greaterThan(0));
      await tester.tap(find.text('Demo child').first);
      await tester.pumpAndSettle();
      expect(tester.state(page('toolpkg')), same(originalRoot));
      final originalChild = tester.state(page('child'));
      await tester.ensureVisible(find.text('Demo leaf'));
      await tester.pumpAndSettle();
      final childOffset = position('child').pixels;
      expect(childOffset, greaterThan(0));
      await tester.tap(find.text('Demo leaf'));
      await tester.pumpAndSettle();
      back();
      await tester.pumpAndSettle();
      expect(page('leaf'), findsNothing);
      expect(tester.state(page('child')), same(originalChild));
      expect(position('child').pixels, closeTo(childOffset, 0.1));
      back();
      await tester.pumpAndSettle();
      expect(page('child'), findsNothing);
      expect(tester.state(page('toolpkg')), same(originalRoot));
      expect(position('toolpkg').pixels, closeTo(rootOffset, 0.1));
      expect(tester.takeException(), isNull);
    },
  );

  for (final width in [280.0, 320.0, 390.0]) {
    testWidgets(
      'dependency tree fits a $width pixel screen with deep shared branches',
      (tester) async {
        tester.view.physicalSize = Size(width, 720);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final root = _toolpkgRuntime(
          requires: [_requirement('0')],
          optionalRequires: [_requirement('1')],
        );
        final runtimes = [
          root,
          for (var i = 0; i < 7; i++)
            _toolpkgRuntime(
              id: '$i',
              requires: i == 6
                  ? [_requirement('0')]
                  : [_requirement('${i + 1}')],
            ),
        ];
        final graph = ToolPkgDependencyGraph(
          root: root,
          runtimes: runtimes,
          installed: [
            for (var i = 0; i < 7; i++)
              _entry(
                id: '$i',
                kind: 'toolpkg',
                source: 'manual',
                enabled: true,
              ),
          ],
        );
        final tree = ToolPkgDependencyTree(
          clients: GeneratedCoreProxyClients(_CatalogBridge()),
          rootId: root.packageName,
          nodes: graph.nodes.toList(),
        );
        await tester.pumpWidget(
          _manager(
            _CatalogBridge(),
            child: SingleChildScrollView(
              child: Padding(padding: const EdgeInsets.all(12), child: tree),
            ),
          ),
        );
        await tester.pumpAndSettle();
        expect(find.byType(CatalogEntryAvatar), findsNWidgets(9));
        expect(
          find.descendant(
            of: find.byType(ToolPkgDependencyTree),
            matching: find.byType(Divider),
          ),
          findsNothing,
        );
        expect(find.byTooltip('共享依赖，已在树中展开'), findsNWidgets(2));
        expect(tester.takeException(), isNull);
        final first = find.text('Demo 0').first;
        final second = find.text('Demo 1').first;
        expect(
          (tester.getTopLeft(second).dy - tester.getTopLeft(first).dy),
          lessThanOrEqualTo(52),
        );
      },
    );
  }

  test(
    'dependency graph promotes shared optional subtrees and stops cycles',
    () {
      final root = _toolpkgRuntime(
        requires: [_requirement('a'), _requirement('b')],
        optionalRequires: [_requirement('shared')],
      );
      final graph = ToolPkgDependencyGraph(
        root: root,
        runtimes: [
          root,
          _toolpkgRuntime(id: 'a', requires: [_requirement('b')]),
          _toolpkgRuntime(id: 'b', requires: [_requirement('shared')]),
          _toolpkgRuntime(
            id: 'shared',
            requires: [_requirement('leaf'), _requirement('toolpkg')],
          ),
          _toolpkgRuntime(id: 'leaf'),
        ],
        installed: [
          for (final id in ['toolpkg', 'a', 'b', 'shared', 'leaf'])
            _entry(id: id, kind: 'toolpkg', source: 'manual', enabled: true),
        ],
      );
      expect(graph.nodes.map((node) => node.id).toSet(), {
        'a',
        'b',
        'shared',
        'leaf',
      });
      expect(graph.nodes.every((node) => node.required), isTrue);
      expect(
        graph.nodes.firstWhere((node) => node.id == 'shared').sources.length,
        2,
      );
      expect(
        graph.nodes.firstWhere((node) => node.id == 'a').state,
        ToolPkgDependencyState.cycle,
      );
      expect(
        graph.nodes.firstWhere((node) => node.id == 'leaf').state,
        ToolPkgDependencyState.satisfied,
      );
    },
  );

  for (final (version, min, max, expected)
      in <(String, String?, String?, ToolPkgDependencyState)>[
        ('v1.0.0', '1.0.0', '1.0.0', ToolPkgDependencyState.satisfied),
        ('1.0.0+local', '1.0.0', '1.0.0', ToolPkgDependencyState.satisfied),
        ('1.0.0-preview.1', '1.0.0', null, ToolPkgDependencyState.incompatible),
        ('1.0.0', null, '0.9.0', ToolPkgDependencyState.incompatible),
        ('invalid', '1.0.0', null, ToolPkgDependencyState.incompatible),
      ]) {
    test('dependency bounds $min <= $version <= $max', () {
      final root = _toolpkgRuntime(
        requires: [_requirement('a', min: min, max: max)],
      );
      final graph = ToolPkgDependencyGraph(
        root: root,
        runtimes: [
          root,
          _toolpkgRuntime(id: 'a'),
        ],
        installed: [
          _entry(
            id: 'a',
            kind: 'toolpkg',
            source: 'manual',
            enabled: true,
            version: version,
          ),
        ],
      );
      expect(graph.nodes.single.state, expected);
    });
  }

  testWidgets('detail hides empty dependency sections', (tester) async {
    final bridge = _CatalogBridge();
    await tester.pumpWidget(
      _manager(
        bridge,
        child: ExtensionDetailScreen(
          kind: 'toolpkg',
          id: 'toolpkg',
          clients: GeneratedCoreProxyClients(bridge),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(ToolPkgDependencyTree), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('MCP startup error stays visible until a successful retry', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(900, 1800));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    const detail =
        'MCP start failed: mcp\n'
        'Remote MCP HTTP request failed with status 401: Bad credentials';
    final bridge = _CatalogBridge()..mcpStartupFailure = StateError(detail);
    await tester.pumpWidget(
      _manager(
        bridge,
        child: ExtensionDetailScreen(
          kind: 'mcp',
          id: 'mcp',
          clients: GeneratedCoreProxyClients(bridge),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('启动'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('启动'));
    await tester.pumpAndSettle();
    // Let the transient message disappear before inspecting the detail page.
    await tester.pump(const Duration(seconds: 6));
    await tester.pumpAndSettle();
    final scrollable = find.descendant(
      of: find.byType(ExtensionDetailScreen),
      matching: find.byWidgetPredicate(
        (widget) =>
            widget is Scrollable && widget.axisDirection == AxisDirection.down,
      ),
    ).first;
    tester.state<ScrollableState>(scrollable).position.jumpTo(0);
    await tester.pumpAndSettle();
    expect(find.text('MCP 服务启动失败'), findsOneWidget);
    expect(find.textContaining('Bad credentials'), findsOneWidget);
    expect(bridge.mcpStartAttempts, 1);

    bridge.mcpStartupFailure = null;
    await tester.scrollUntilVisible(find.text('启动'), 500, scrollable: scrollable);
    await tester.pumpAndSettle();
    await tester.tap(find.text('启动'));
    await tester.pumpAndSettle();
    tester.state<ScrollableState>(scrollable).position.jumpTo(0);
    await tester.pumpAndSettle();
    expect(find.text('MCP 服务启动失败'), findsNothing);
    expect(find.textContaining('Bad credentials'), findsNothing);
    expect(bridge.mcpStartAttempts, 2);
    expect(tester.takeException(), isNull);
  });

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

  testWidgets('official see all opens an offline list with install actions', (
    tester,
  ) async {
    final bridge = _CatalogBridge();
    String? openedRoute;
    AppRouterGateway.install(
      handler: (routeId, args, source) => openedRoute = routeId,
      reset: (routeId, args, source) => openedRoute = routeId,
    );
    addTearDown(AppRouterGateway.clear);
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();
    await tester.tap(find.text('市场'));
    await tester.pumpAndSettle();

    final seeAll = find.descendant(
      of: _sectionHeader('官方扩展'),
      matching: find.byType(StoreSeeAllButton),
    );
    expect(seeAll, findsOneWidget);
    await tester.ensureVisible(seeAll);
    await tester.tap(seeAll);
    expect(openedRoute, 'native.official_extensions');

    bridge.marketRequests.clear();
    await tester.pumpWidget(
      _manager(
        bridge,
        child: OfficialExtensionsScreen(
          clients: GeneratedCoreProxyClients(bridge),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Official Extra'), findsOneWidget);
    expect(bridge.marketRequests, isEmpty);
    await tester.tap(find.text('获取'));
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
        sourcePath: '/runtime/extensions/device/records/skill-demo.json',
        packageId: 'demo',
        displayName: 'Skipped local duplicate',
        code: 'duplicate_scope',
        message: '重复安装：已使用 space 版本，local 版本已跳过，本地数据保留。',
        packageKind: 'skill',
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
    await tester.tap(find.text('3 条加载诊断'));
    await tester.pumpAndSettle();

    expect(find.byTooltip('此加载错误没有可删除的安装来源'), findsOneWidget);
    expect(find.byTooltip('已使用设备空间版本，本机副本和数据保留'), findsOneWidget);
    expect(find.byIcon(Icons.warning_amber_rounded), findsOneWidget);
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

  testWidgets('failed external imports can be cleared from diagnostics', (
    tester,
  ) async {
    const path = '/storage/emulated/0/Download/Apktool-3.0.3-Operit.toolpkg';
    final bridge = _CatalogBridge();
    bridge.diagnostics.add(
      const core_proxy.ExtensionCatalogDiagnostic(
        sourcePath: path,
        packageId: null,
        displayName: path,
        code: 'import_failed',
        message: 'Cannot access file at path: $path',
        packageKind: 'package',
        canDelete: true,
      ),
    );
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();
    await tester.tap(find.text('1 条加载诊断'));
    await tester.pumpAndSettle();

    expect(find.byTooltip('删除失败记录'), findsOneWidget);
    await tester.tap(find.byTooltip('删除失败记录'));
    await tester.pumpAndSettle();

    expect(bridge.deletedDiagnostics, <String>[path]);
    expect(find.text(path), findsNothing);
    expect(find.text('已经全部处理完了。'), findsOneWidget);
    expect(find.text('1 条加载诊断'), findsNothing);
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
  bool hasUi = false,
  String scope = 'device',
  String version = '1.0.0',
}) {
  return core_proxy.ExtensionCatalogEntry(
    id: id,
    kind: kind,
    source: source,
    displayName: 'Demo $id',
    description: 'Demo description',
    version: version,
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
    hasUi: hasUi,
    chatSidebarEntryIds: const <String>[],
    dependencyIssueCount: 0,
  );
}

Map<String, Object?> _fileImportResult({
  required String id,
  required String format,
  required String scope,
}) {
  final extension = switch (format) {
    'javascript' => 'js',
    'typescript' => 'ts',
    _ => format,
  };
  return <String, Object?>{
    'packageName': id,
    'packageFormat': format,
    'storedPath': '/operitapp/data/extensions/$scope/packages/$id.$extension',
    'sourceNotice': null,
  };
}

class _CatalogBridge extends OperitRuntimeBridge {
  core_proxy.ToolPkgContainerRuntime runtime = _toolpkgRuntime();
  final requestedUiModules = <String>[];
  final dependencyRuntimes = <core_proxy.ToolPkgContainerRuntime>[];
  final dependencyEntries = <core_proxy.ExtensionCatalogEntry>[];
  bool skillDeleted = false;
  bool externalToolpkg = false;
  bool toolpkgDeleted = false;
  int snapshotReads = 0;
  final marketPages = <(String, int), List<core_proxy.MarketEntrySummary>>{};
  final marketRequests = <(String, int)>[];
  int marketTotal = 0;
  int? failedMarketPage;
  final categories = <core_proxy.MarketCategoryInfo>[];
  bool enabled = false;
  Object? mcpStartupFailure;
  int mcpStartAttempts = 0;
  final imports = <String>[];
  final scopeMoves = <String>[];
  final scopeMoveRequests = <({String kind, String id, String scope})>[];
  final packageFileImports = <String>[];
  Map<String, Object?>? fileImportResult;
  core_proxy.ExtensionCatalogEntry? fileInstalledEntry;
  core_proxy.ExtensionCatalogEntry? fileImportedEntry;
  Object? fileImportFailure;
  bool rejectScopeChanges = false;
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
      case 'getAllMCPServers':
        return <String, Object?>{
          'mcp': <String, Object?>{'url': 'https://api.githubcopilot.com/mcp/'},
        };
      case 'getAllPluginMetadata':
      case 'getAllServerStatus':
        return <String, Object?>{};
      case 'runCoreCommand':
        expect((request.args as Map)['args'], <String>['mcp', 'start', 'mcp']);
        mcpStartAttempts += 1;
        final failure = mcpStartupFailure;
        if (failure != null) throw failure;
        return <String, Object?>{'stdout': 'MCP server started: mcp', 'stderr': ''};
      case 'readSkillContent':
        return '# Demo skill';
      case 'deleteSkill':
        skillDeleted = true;
        return true;
      case 'getToolPkgContainerRuntimes':
        return <Object?>[
          runtime.toJson(),
          for (final item in dependencyRuntimes) item.toJson(),
        ];
      case 'getToolPkgContainerDetails':
        return null;
      case 'getToolPkgComposeDslScript':
        requestedUiModules.add((request.args as Map)['uiModuleId'] as String);
        return 'module.exports = {};';
      case 'getToolPkgComposeDslScreenPath':
        return '${(request.args as Map)['uiModuleId']}.ui.js';
      case 'acquireToolPkgExecutionEngine':
        return Completer<Object?>().future;
      case 'releaseToolPkgExecutionEngine':
        return null;
      case 'deletePackage':
        toolpkgDeleted = true;
        return true;
      case 'getExtensionCatalogSnapshot':
        snapshotReads += 1;
        return _snapshot().toJson();
      case 'addPackageFileFromExternalStorageResult':
        packageFileImports.add((request.args as Map)['filePath'] as String);
        final failure = fileImportFailure;
        if (failure != null) throw failure;
        final result = fileImportResult;
        if (result == null) {
          throw StateError('Package import result is missing');
        }
        fileInstalledEntry = fileImportedEntry ?? fileInstalledEntry;
        return result;
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
        final scopeArgs = request.args as Map;
        final id = scopeArgs['id'] as String;
        final scope = scopeArgs['scope'] as String;
        scopeMoves.add(id);
        scopeMoveRequests.add((
          kind: scopeArgs['kind'] as String,
          id: id,
          scope: scope,
        ));
        final current = fileInstalledEntry;
        if (current != null && current.id == id) {
          if (rejectScopeChanges && current.scope != scope) {
            throw StateError('Plugin database scope moves are not supported');
          }
          fileInstalledEntry = _entry(
            id: current.id,
            kind: current.kind,
            source: current.source,
            scope: scope,
            enabled: current.activationState == 'enabled',
            version: current.version,
          );
        }
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
            protocolVersion: 1,
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
      source: externalToolpkg ? 'manual' : 'builtin',
      enabled: enabled,
      hasUi: runtime.navigationEntries.isNotEmpty,
    );
    return core_proxy.ExtensionCatalogSnapshot(
      revision: 1,
      installed: <core_proxy.ExtensionCatalogEntry>[
        if (!toolpkgDeleted) toolpkg,
        ...dependencyEntries,
        ?fileInstalledEntry,
        if (!skillDeleted)
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

core_proxy.ToolPkgManifestRequirement _requirement(
  String id, {
  String? min,
  String? max,
}) => core_proxy.ToolPkgManifestRequirement(
  id: id,
  description: '',
  minVersion: min,
  maxVersion: max,
);

/// Supplies complete metadata for the external plugin used by the delete flow.
core_proxy.ToolPkgContainerRuntime _toolpkgRuntime({
  String id = 'toolpkg',
  List<core_proxy.ToolPkgManifestRequirement> requires = const [],
  List<core_proxy.ToolPkgManifestRequirement> optionalRequires = const [],
  List<core_proxy.ToolPkgUiRouteRuntime> uiRoutes = const [],
  List<core_proxy.ToolPkgNavigationEntryRuntime> navigationEntries = const [],
}) => core_proxy.ToolPkgContainerRuntime(
  packageName: id,
  displayName: core_proxy.LocalizedText(values: {'en': 'Demo $id'}),
  description: core_proxy.LocalizedText(values: {'en': 'Demo description'}),
  version: '1.0.0',
  apiVersion: '2.0.0',
  publicApi: null,
  publicApis: [],
  requires: requires,
  optionalRequires: optionalRequires,
  dependencyIssues: [],
  manifestExtensions: {},
  author: ['Operit'],
  mainEntry: 'main.js',
  themes: [],
  license: null,
  sourceType: core_proxy.ToolPkgSourceType.externalValue,
  sourcePath: '/runtime/extensions/device/packages/demo.toolpkg',
  subpackages: [],
  resources: [],
  wasmModules: [],
  workflowTemplates: [],
  workspaceTemplates: [],
  uiModules: [],
  uiRoutes: uiRoutes,
  chatComposerSlots: [],
  navigationEntries: navigationEntries,
  desktopWidgets: [],
  appLifecycleHooks: [],
  messageProcessingPlugins: [],
  messageTransformHooks: [],
  messageStreamTransforms: [],
  xmlRenderPlugins: [],
  inputMenuTogglePlugins: [],
  chatInputHooks: [],
  chatViewHooks: [],
  chatMessageHooks: [],
  chatLifecycleHooks: [],
  chatMessageMenuItems: [],
  chatRuntimeHooks: [],
  chatResponseHooks: [],
  hostEventHooks: [],
  toolLifecycleHooks: [],
  promptInputHooks: [],
  promptHistoryHooks: [],
  promptEstimateHistoryHooks: [],
  systemPromptComposeHooks: [],
  toolPromptComposeHooks: [],
  promptFinalizeHooks: [],
  promptEstimateFinalizeHooks: [],
  summaryGenerateHooks: [],
  coreCommands: [],
  aiProviders: [],
  manifestExtensionHandlers: [],
  logoResource: null,
  marketOrigin: null,
);

/// Runs the real package pages inside the retained shell page host.
class _CatalogScreen extends OperitScreen {
  const _CatalogScreen(this.child, {required this.inList})
    : super(routeTypeName: 'Catalog', keepAlive: inList);

  final Widget child;
  final bool inList;

  /// Keeps one retained page for the package list and one for the detail.
  @override
  String stableScreenKey() => inList ? 'packages' : 'detail';

  /// Supplies the page with the test runtime connection.
  @override
  Widget build(BuildContext context) => child;
}

/// Uses the production route retention policy with an in-memory Core connection.
class _DependencyDetailRoute extends ExtensionDetailScreenRoute {
  const _DependencyDetailRoute({required super.id, required this.clients})
    : super(kind: 'toolpkg');
  final GeneratedCoreProxyClients clients;
  @override
  Widget build(BuildContext context) =>
      ExtensionDetailScreen(kind: kind, id: id, clients: clients);
}
