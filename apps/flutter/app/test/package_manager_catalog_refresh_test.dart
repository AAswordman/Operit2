import 'dart:async';
import 'dart:math' as math;
import 'dart:typed_data';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/host/AppToastController.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/ui/features/packages/components/PackageListItem.dart';
import 'package:operit2/ui/features/packages/screens/PackageManagerScreen.dart';
import 'package:operit2/ui/features/packages/dialogs/PackageEnvironmentVariablesDialog.dart';
import 'package:operit2/ui/main/navigation/ToolPkgCatalogChangeBus.dart';
import 'package:operit2/ui/common/components/RetainedPage.dart';
import 'package:operit2/ui/common/contributions/ChatSidebarTabHost.dart';

/// Verifies installation notifications update an already mounted package list.
void main() {
  setUp(AppToastController.instance.reset);
  tearDown(AppToastController.instance.reset);
  for (final scope in ['device', 'space']) {
    for (final isToolPkg in [true, false]) {
      testWidgets(
        'bundled source update stays in $scope (ToolPkg: $isToolPkg)',
        (tester) async {
          await tester.binding.setSurfaceSize(const Size(400, 900));
          addTearDown(() => tester.binding.setSurfaceSize(null));
          final name = isToolPkg ? 'demo_toolpkg' : 'demo_script';
          final bridge = _CatalogBridge()
            ..installed = isToolPkg
            ..enabled = true
            ..scopes[name] = scope;
          if (!isToolPkg) {
            bridge.executablePackages[name] = _namedPackage(
              name,
              'Demo Script',
            );
          }
          bridge.extraCandidates.add(
            core_proxy.BundledExternalPackageCandidate.fromJson(
              _extraCandidate(name).toJson()..['isToolPkg'] = isToolPkg,
            ),
          );
          await tester.pumpWidget(_manager(bridge));
          await tester.pumpAndSettle();
          if (!isToolPkg) {
            await tester.tap(
              find.descendant(
                of: find.byType(TabBar),
                matching: find.text('包'),
              ),
            );
            await tester.pumpAndSettle();
          }
          final update = find.byKey(ValueKey<String>('bundled-update:$name'));
          expect(update, findsOneWidget);
          expect(
            find.byKey(
              ValueKey<String>(
                'bundled-${isToolPkg ? 'plugin' : 'package'}:$name',
              ),
            ),
            findsNothing,
          );
          expect(find.text(scope == 'space' ? '设备空间' : '仅本设备'), findsOneWidget);
          await tester.ensureVisible(update);
          await tester.tap(update);
          await tester.pumpAndSettle();
          expect(bridge.imports, [name]);
          expect(bridge.scopeChanges, isEmpty);
          expect(bridge.scopes[name], scope);
          expect(bridge.enabled, isTrue);
          expect(update, findsNothing);
          expect(find.text('选择安装范围'), findsNothing);
          expect(tester.takeException(), isNull);
        },
      );
    }
  }

  for (final width in <double>[400, 1400]) {
    testWidgets('plugin drag keeps its laid-out card width at $width', (
      tester,
    ) async {
      await tester.binding.setSurfaceSize(Size(width, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final bridge = _CatalogBridge()..installed = true;
      for (var index = 1; index < 3; index++) {
        final name = 'demo_$index';
        bridge.installedContainers.add(
          _pluginRuntime().toJson()..['packageName'] = name,
        );
        bridge.scopes[name] = 'device';
      }
      await tester.pumpWidget(_manager(bridge));
      await tester.pumpAndSettle();
      final card = find.byKey(const ValueKey<String>('plugin:demo_toolpkg'));
      final cardWidth = tester.getSize(card).width;
      final drag = tester.widget<LongPressDraggable<String>>(
        find.ancestor(
          of: card,
          matching: find.byType(LongPressDraggable<String>),
        ),
      );
      final gesture = await tester.startGesture(tester.getCenter(card));
      await tester.pump(const Duration(milliseconds: 200));
      await gesture.moveBy(const Offset(20, 20));
      await tester.pump();
      expect(
        tester.getSize(find.byWidget(drag.feedback)).width,
        closeTo(cardWidth, 0.01),
      );
      await gesture.up();
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets(
    'customized built-in displays its status and uninstalls through plugin details',
    (tester) async {
      final bridge = _CatalogBridge()
        ..installed = true
        ..enabled = true
        ..customizedPackageNames.add('demo_toolpkg');
      await tester.pumpWidget(_manager(bridge));
      await tester.pumpAndSettle();
      expect(find.text('自定义'), findsOneWidget);
      await tester.tap(find.text('Demo ToolPkg').first);
      await tester.pumpAndSettle();
      expect(find.text('卸载自定义'), findsOneWidget);
      await tester.tap(find.text('卸载自定义'));
      await tester.pumpAndSettle();
      expect(find.textContaining('数据和配置会保留'), findsOneWidget);
      await tester.tap(find.widgetWithText(FilledButton, '卸载自定义'));
      await tester.pumpAndSettle();
      expect(bridge.deletedPackages, ['demo_toolpkg']);
      expect(bridge.enabled, isTrue);
      expect(find.text('自定义'), findsNothing);
      expect(find.text('Demo ToolPkg'), findsOneWidget);
      await tester.tap(find.text('Demo ToolPkg'));
      await tester.pumpAndSettle();
      expect(find.text('卸载自定义'), findsNothing);
      expect(find.text('删除'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  for (final scope in <String>['device', 'space']) {
    testWidgets('bundled plugin uses its actual movable scope: $scope', (
      tester,
    ) async {
      final bridge = _CatalogBridge()
        ..installed = true
        ..enabled = true
        ..restoredBuiltin = true
        ..scopes['demo_toolpkg'] = scope;
      await tester.pumpWidget(_manager(bridge));
      await tester.pumpAndSettle();
      expect(find.text(scope == 'space' ? '设备空间' : '仅本设备'), findsOneWidget);
      await tester.tap(find.byTooltip('展开'));
      await tester.pumpAndSettle();
      final target = scope == 'space' ? '仅本设备' : '设备空间';
      await tester.tap(find.byTooltip('移至$target'));
      await tester.pumpAndSettle();
      expect(find.text('移至$target？'), findsOneWidget);
      await tester.tap(find.widgetWithText(FilledButton, '移动'));
      await tester.pumpAndSettle();
      expect(bridge.scopeChanges.single['id'], 'demo_toolpkg');
      expect(
        bridge.scopeChanges.single['scope'],
        scope == 'space' ? 'device' : 'space',
      );
      await tester.tap(find.text('Demo ToolPkg'));
      await tester.pumpAndSettle();
      expect(find.text('删除'), findsOneWidget);
      await tester.tap(find.text('删除'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, '删除'));
      await tester.pumpAndSettle();
      expect(bridge.deletedPackages, ['demo_toolpkg']);
      expect(find.text('Demo ToolPkg'), findsNothing);
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('customized script retains its built-in row after uninstall', (
    tester,
  ) async {
    final bridge = _CatalogBridge()
      ..executablePackages['demo_script'] = _namedPackage(
        'demo_script',
        'Demo Script',
      )
      ..scopes['demo_script'] = 'device'
      ..customizedPackageNames.add('demo_script');
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();
    await tester.tap(
      find.descendant(of: find.byType(TabBar), matching: find.text('包')),
    );
    await tester.pumpAndSettle();
    expect(find.text('自定义'), findsOneWidget);
    await tester.tap(find.text('Demo Script'));
    await tester.pumpAndSettle();
    expect(find.text('卸载自定义'), findsOneWidget);
    await tester.tap(find.text('卸载自定义'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, '卸载自定义'));
    await tester.pumpAndSettle();
    expect(bridge.deletedPackages, ['demo_script']);
    expect(find.text('自定义'), findsNothing);
    expect(find.text('Demo Script'), findsOneWidget);
    await tester.tap(find.text('Demo Script'));
    await tester.pumpAndSettle();
    expect(find.text('卸载自定义'), findsNothing);
    expect(find.text('删除'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  for (final selection in <String>[
    chatSidebarTabIdentity('demo_toolpkg', 'sidebar-demo'),
    chatSidebarTabIdentity('another_toolpkg', 'sidebar-demo'),
    UserPreferencesManager.CHAT_SIDEBAR_WORKSPACE_TAB,
  ]) {
    testWidgets(
      'disabling a plugin updates only its selected tab: $selection',
      (tester) async {
        final bridge = _CatalogBridge()
          ..installed = true
          ..enabled = true;
        bridge.preferences['chat_sidebar_tab_v1'] = selection;
        final selectionsAtNotification = <String?>[];
        final reloadPolicies = <bool>[];
        final subscription = ToolPkgCatalogChangeBus.listenChanges((change) {
          selectionsAtNotification.add(
            bridge.preferences['chat_sidebar_tab_v1'],
          );
          reloadPolicies.add(change.reloadPluginCode);
        });
        addTearDown(subscription.cancel);
        await tester.pumpWidget(_manager(bridge));
        await tester.pumpAndSettle();
        expect(tester.widget<Switch>(find.byType(Switch)).value, isTrue);
        expect(bridge.catalogReads, 1);
        await tester.tap(find.byType(Switch));
        await tester.pumpAndSettle();
        final expected =
            selection == chatSidebarTabIdentity('demo_toolpkg', 'sidebar-demo')
            ? UserPreferencesManager.CHAT_SIDEBAR_WORKSPACE_TAB
            : selection;
        expect(bridge.enabled, isFalse);
        expect(bridge.catalogReads, 2);
        expect(bridge.scans, 0);
        expect(bridge.preferences['chat_sidebar_tab_v1'], expected);
        expect(selectionsAtNotification, [expected]);
        expect(reloadPolicies, [false]);
        expect(
          bridge.mutations,
          expected != selection ? ['disable', 'selection'] : ['disable'],
        );
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets('enabling a plugin requests a registration-only refresh', (
    tester,
  ) async {
    final bridge = _CatalogBridge()..installed = true;
    final changes = <ToolPkgCatalogChange>[];
    final subscription = ToolPkgCatalogChangeBus.listenChanges(changes.add);
    addTearDown(subscription.cancel);
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();
    expect(tester.widget<Switch>(find.byType(Switch)).value, isFalse);
    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    expect(bridge.enabled, isTrue);
    expect(bridge.mutations, ['enable']);
    expect(changes.map((change) => change.reloadPluginCode), [false]);
    expect(tester.takeException(), isNull);
  });

  testWidgets('failed plugin disable keeps the current sidebar selection', (
    tester,
  ) async {
    final bridge = _CatalogBridge()
      ..installed = true
      ..enabled = true
      ..disableError = StateError('Disable rejected');
    final selection = chatSidebarTabIdentity('demo_toolpkg', 'sidebar-demo');
    bridge.preferences['chat_sidebar_tab_v1'] = selection;
    var notifications = 0;
    final subscription = ToolPkgCatalogChangeBus.listen(() => notifications++);
    addTearDown(subscription.cancel);
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();
    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    expect(bridge.enabled, isTrue);
    expect(bridge.preferences['chat_sidebar_tab_v1'], selection);
    expect(bridge.mutations, isEmpty);
    expect(notifications, 0);
    expect(find.textContaining('Disable rejected'), findsOneWidget);
    AppToastController.instance.reset();
    expect(tester.takeException(), isNull);
  });

  for (final name in <String>[
    'com.operit.plan_mode_bundle',
    'com.operit.goal_mode_bundle',
    'com.operit.character_cards',
  ]) {
    for (final runtimeFallback in <bool>[false, true]) {
      testWidgets(
        'fixed-scope extra loads directly: $name, fallback=$runtimeFallback',
        (tester) async {
          final bridge = _CatalogBridge();
          if (runtimeFallback) {
            bridge.extraRuntimes.add(
              _pluginRuntime().toJson()
                ..['packageName'] = name
                ..['manifestExtensions'] = <String, Object?>{
                  'installation_scope': 'space',
                },
            );
          } else {
            bridge.extraCandidates.add(
              _extraCandidate(name, requiredScope: 'space'),
            );
          }
          await tester.pumpWidget(_manager(bridge));
          await tester.pumpAndSettle();
          await tester.tap(find.byTooltip('展开'));
          await tester.pumpAndSettle();
          await tester.ensureVisible(find.text('加载'));
          await tester.tap(find.text('加载'));
          await tester.pumpAndSettle();
          expect(find.text('选择存放位置'), findsNothing);
          expect(bridge.imports, [name]);
          expect(bridge.scopeChanges, isEmpty);
          expect(tester.takeException(), isNull);
        },
      );
    }
  }

  for (final scope in <String>['device', 'space']) {
    testWidgets('unrestricted extra keeps scope selection: $scope', (
      tester,
    ) async {
      final bridge = _CatalogBridge()
        ..extraCandidates.add(_extraCandidate('flexible.plugin'));
      await tester.pumpWidget(_manager(bridge));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('展开'));
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.text('加载'));
      await tester.tap(find.text('加载'));
      await tester.pumpAndSettle();
      expect(find.text('选择存放位置'), findsOneWidget);
      expect(bridge.imports, isEmpty);
      await tester.tap(find.text(scope == 'space' ? '设备空间' : '仅本设备'));
      await tester.pumpAndSettle();
      expect(bridge.imports, ['flexible.plugin']);
      expect(bridge.scopeChanges.single['scope'], scope);
      expect(tester.takeException(), isNull);
    });
  }

  for (final packageName in <String?>['broken_toolpkg', null]) {
    testWidgets(
      'failed package can be deleted from its details: $packageName',
      (tester) async {
        final issue = core_proxy.ToolPkgLoadIssue(
          sourcePath: '/packages/broken.toolpkg',
          packageName: packageName,
          displayName: 'Broken package',
          code: 'parse_failed',
          message: 'Invalid manifest',
          packageKind: 'toolpkg',
        );
        final bridge = _CatalogBridge()..loadIssues.add(issue);
        await tester.pumpWidget(_manager(bridge));
        await tester.pumpAndSettle();
        await tester.tap(find.text('Broken package'));
        await tester.pumpAndSettle();
        expect(find.text('Invalid manifest'), findsWidgets);
        await tester.tap(find.widgetWithText(FilledButton, '删除'));
        await tester.pumpAndSettle();
        expect(find.text('删除加载失败的包'), findsOneWidget);
        await tester.tap(find.widgetWithText(FilledButton, '删除'));
        await tester.pumpAndSettle();
        expect(bridge.deletedIssues.single.toJson(), issue.toJson());
        expect(find.text('Broken package'), findsNothing);
        expect(find.text('已删除，可重新添加'), findsOneWidget);
        AppToastController.instance.reset();
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets('broken import bookkeeping can be deleted from its details', (
    tester,
  ) async {
    final bridge = _CatalogBridge()
      ..loadIssues.add(
        const core_proxy.ToolPkgLoadIssue(
          sourcePath: '/packages',
          packageName: null,
          displayName: '插件导入记录',
          code: 'scan_failed',
          message: 'missing field scope',
          packageKind: 'package_manager',
        ),
      );
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();
    await tester.tap(find.text('包'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('插件导入记录'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, '删除'));
    await tester.pumpAndSettle();
    expect(find.text('删除导入记录'), findsOneWidget);
    await tester.tap(find.widgetWithText(FilledButton, '删除'));
    await tester.pumpAndSettle();
    expect(bridge.deletedIssues.single.packageKind, 'package_manager');
    expect(find.text('插件导入记录'), findsNothing);
    expect(find.text('导入记录已删除'), findsOneWidget);
    AppToastController.instance.reset();
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'plugin packages and failed plugins stay out of the package tab',
    (tester) async {
      final bridge = _CatalogBridge()
        ..scopes.addAll(<String, String>{
          'owner_plugin': 'device',
          'standalone_tools': 'device',
        })
        ..executablePackages.addAll(<String, core_proxy.ToolPackage>{
          'plugin_inner_tools': _namedPackage(
            'plugin_inner_tools',
            'Inner tools',
          ),
          'standalone_tools': _namedPackage(
            'standalone_tools',
            'Standalone tools',
          ),
        })
        ..installedContainers.add(
          _pluginRuntime().toJson()
            ..['packageName'] = 'owner_plugin'
            ..['displayName'] = <String, Object?>{
              'values': <String, String>{'default': 'Owner plugin'},
            }
            ..['sourcePath'] = 'owner_plugin.toolpkg'
            ..['subpackages'] = <Object?>[
              const core_proxy.ToolPkgSubpackageRuntime(
                packageName: 'plugin_inner_tools',
                containerPackageName: 'owner_plugin',
                subpackageId: 'tools',
                entryPath: 'dist/tools.js',
                displayName: core_proxy.LocalizedText(
                  values: <String, String>{'default': 'Inner tools'},
                ),
                description: core_proxy.LocalizedText(
                  values: <String, String>{'default': 'Owned by the plugin'},
                ),
                enabledByDefault: true,
                toolCount: 1,
              ).toJson(),
            ],
        )
        ..loadIssues.addAll(<core_proxy.ToolPkgLoadIssue>[
          const core_proxy.ToolPkgLoadIssue(
            sourcePath: '/packages/broken.toolpkg',
            packageName: 'broken_plugin',
            displayName: 'Broken plugin',
            code: 'toolpkg_load',
            message: 'Invalid plugin manifest',
            packageKind: 'package',
          ),
          const core_proxy.ToolPkgLoadIssue(
            sourcePath: '/packages/broken.js',
            packageName: 'broken_script',
            displayName: 'Broken script',
            code: 'package_parse',
            message: 'Invalid script package',
            packageKind: 'javascript',
          ),
        ]);
      await tester.pumpWidget(_manager(bridge));
      await tester.pumpAndSettle();
      expect(find.text('Owner plugin'), findsOneWidget);
      expect(
        find.byKey(
          const ValueKey<String>(
            'plugin-load-issue:/packages/broken.toolpkg:toolpkg_load:0',
          ),
        ),
        findsOneWidget,
      );
      expect(find.text('Inner tools'), findsNothing);
      expect(find.text('Standalone tools'), findsNothing);
      await tester.tap(find.text('包'));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey<String>('package:standalone_tools')),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey<String>('package:plugin_inner_tools')),
        findsNothing,
      );
      expect(
        find.byKey(
          const ValueKey<String>(
            'package-load-issue:/packages/broken.js:package_parse:0',
          ),
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(
          const ValueKey<String>(
            'package-load-issue:/packages/broken.toolpkg:toolpkg_load:0',
          ),
        ),
        findsNothing,
      );
      expect(find.text('Inner tools'), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('cancelling failed package deletion keeps its source', (
    tester,
  ) async {
    final bridge = _CatalogBridge()
      ..loadIssues.add(
        const core_proxy.ToolPkgLoadIssue(
          sourcePath: '/packages/broken.toolpkg',
          packageName: 'broken',
          displayName: 'Broken package',
          code: 'parse_failed',
          message: 'Invalid manifest',
          packageKind: 'toolpkg',
        ),
      );
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Broken package'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, '删除'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('取消'));
    await tester.pumpAndSettle();
    expect(bridge.deletedIssues, isEmpty);
    expect(find.text('Broken package'), findsOneWidget);
  });

  testWidgets(
    'failed deletion reports the error and keeps the failed package',
    (tester) async {
      final bridge = _CatalogBridge()
        ..deleteIssueError = StateError('Storage rejected deletion')
        ..loadIssues.add(
          const core_proxy.ToolPkgLoadIssue(
            sourcePath: '/packages/broken.toolpkg',
            packageName: 'broken',
            displayName: 'Broken package',
            code: 'parse_failed',
            message: 'Invalid manifest',
            packageKind: 'toolpkg',
          ),
        );
      await tester.pumpWidget(_manager(bridge));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Broken package'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, '删除'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, '删除'));
      await tester.pumpAndSettle();
      expect(find.textContaining('Storage rejected deletion'), findsOneWidget);
      expect(find.text('Broken package'), findsOneWidget);
      expect(bridge.deletedIssues, isEmpty);
      AppToastController.instance.reset();
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('catalog change refreshes list without reopening or rescanning', (
    tester,
  ) async {
    final bridge = _CatalogBridge();
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: PackageManagerScreen(clients: GeneratedCoreProxyClients(bridge)),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Demo ToolPkg'), findsNothing);
    final state = tester.state(find.byType(PackageManagerScreen));
    bridge.installed = true;
    ToolPkgCatalogChangeBus.notifyCatalogChanged();
    await tester.pumpAndSettle();
    expect(find.text('Demo ToolPkg'), findsOneWidget);
    expect(tester.state(find.byType(PackageManagerScreen)), same(state));
    expect(bridge.scans, 0);
    bridge.installed = false;
    ToolPkgCatalogChangeBus.notifyCatalogChanged();
    await tester.pumpAndSettle();
    expect(find.text('Demo ToolPkg'), findsNothing);
    await tester.pumpWidget(const SizedBox());
    ToolPkgCatalogChangeBus.notifyCatalogChanged();
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });

  testWidgets('revision and local notifications share one catalog read', (
    tester,
  ) async {
    final bridge = _CatalogBridge();
    addTearDown(bridge.revisions.close);
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();
    expect(bridge.catalogReads, 1);
    bridge.installed = true;
    for (var revision = 1; revision <= 4; revision += 1) {
      bridge.revisions.add(revision);
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
    }
    await tester.pumpAndSettle();
    expect(find.text('Demo ToolPkg'), findsOneWidget);
    expect(bridge.catalogReads, 2);
    expect(bridge.scans, 0);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('catalog changes during a read discard it and read latest once', (
    tester,
  ) async {
    final gate = Completer<void>();
    final started = Completer<void>();
    final bridge = _CatalogBridge()..installed = true;
    addTearDown(bridge.revisions.close);
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();
    bridge
      ..catalogReadGate = gate
      ..catalogReadStarted = started;
    final refresh = tester
        .widget<RefreshIndicator>(find.byType(RefreshIndicator).first)
        .onRefresh();
    await tester.pump(const Duration(milliseconds: 1));
    expect(started.isCompleted, isTrue);
    expect(bridge.catalogReads, 2);
    bridge.installed = false;
    for (var revision = 1; revision <= 4; revision += 1) {
      bridge.revisions.add(revision);
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
    }
    await tester.pump();
    expect(bridge.catalogReads, 2);
    bridge.catalogReadGate = null;
    gate.complete();
    await tester.pumpAndSettle();
    await refresh;
    expect(find.text('Demo ToolPkg'), findsNothing);
    expect(bridge.catalogReads, 3);
    expect(bridge.maxActiveCatalogReads, 1);
    expect(bridge.scans, 0);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'disposing the page detaches notifications during a pending read',
    (tester) async {
      final bridge = _CatalogBridge()..installed = true;
      addTearDown(bridge.revisions.close);
      await tester.pumpWidget(_manager(bridge));
      await tester.pumpAndSettle();
      final gate = Completer<void>();
      bridge.catalogReadGate = gate;
      final refresh = tester
          .widget<RefreshIndicator>(find.byType(RefreshIndicator).first)
          .onRefresh();
      await tester.pump(const Duration(milliseconds: 1));
      expect(bridge.catalogReads, 2);
      await tester.pumpWidget(const SizedBox());
      expect(bridge.watchers, 0);
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
      bridge.revisions.add(1);
      gate.complete();
      await tester.pumpAndSettle();
      await refresh;
      expect(bridge.catalogReads, 2);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('manual refresh reads the catalog without rebuilding packages', (
    tester,
  ) async {
    final bridge = _CatalogBridge()..installed = true;
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();
    bridge.installed = false;
    final refresh = tester
        .widget<RefreshIndicator>(find.byType(RefreshIndicator).first)
        .onRefresh();
    await tester.pumpAndSettle();
    await refresh;
    expect(find.text('Demo ToolPkg'), findsNothing);
    expect(bridge.catalogReads, 2);
    expect(bridge.scans, 0);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('cached manager stops catalog work and refreshes on activation', (
    tester,
  ) async {
    final bridge = _CatalogBridge();
    final active = ValueNotifier<bool>(true);
    addTearDown(active.dispose);
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: ValueListenableBuilder<bool>(
          valueListenable: active,
          child: PackageManagerScreen(
            clients: GeneratedCoreProxyClients(bridge),
          ),
          builder: (context, visible, child) =>
              RetainedPage(active: visible, child: child!),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final state = tester.state(find.byType(PackageManagerScreen));
    expect(bridge.watchers, 1);
    expect(bridge.catalogReads, 1);
    expect(bridge.scans, 0);
    active.value = false;
    await tester.pumpAndSettle();
    expect(bridge.watchers, 0);
    final reads = bridge.catalogReads;
    bridge.installed = true;
    for (var i = 0; i < 4; i += 1) {
      ToolPkgCatalogChangeBus.notifyCatalogChanged();
      await tester.pump(const Duration(milliseconds: 400));
    }
    expect(bridge.catalogReads, reads);
    expect(
      tester.state(find.byType(PackageManagerScreen, skipOffstage: false)),
      same(state),
    );
    active.value = true;
    await tester.pumpAndSettle();
    expect(bridge.watchers, 1);
    expect(bridge.catalogReads, reads + 1);
    expect(bridge.scans, 0);
    expect(find.text('Demo ToolPkg'), findsOneWidget);
    expect(tester.state(find.byType(PackageManagerScreen)), same(state));
    await tester.pumpWidget(const SizedBox.shrink());
    expect(bridge.watchers, 0);
    expect(tester.takeException(), isNull);
  });

  testWidgets('plugin and package tabs keep visited cards without rebuilding', (
    tester,
  ) async {
    final bridge = _CatalogBridge()..installed = true;
    await tester.pumpWidget(_manager(bridge));
    await tester.pumpAndSettle();
    final expand = find.byIcon(Icons.keyboard_arrow_down);
    await tester.ensureVisible(expand);
    await tester.tap(expand);
    await tester.pumpAndSettle();
    expect(find.text('demo_toolpkg'), findsOneWidget);

    await tester.tap(
      find.descendant(of: find.byType(TabBar), matching: find.text('包')),
    );
    await tester.pumpAndSettle();
    expect(find.text('快速创作你的插件'), findsOneWidget);
    expect(find.text('demo_toolpkg'), findsNothing);

    await tester.tap(
      find.descendant(of: find.byType(TabBar), matching: find.text('插件')),
    );
    await tester.pumpAndSettle();
    expect(find.text('demo_toolpkg'), findsOneWidget);
    expect(find.text('快速创作你的插件'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('package cards do not blur when transparent surfaces are on', (
    tester,
  ) async {
    final snapshot = ThemePreferenceSnapshot.fromJson(<String, Object?>{
      ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
      'transparentSurfaceEnabled': true,
    });
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot: snapshot,
        initialThemeIsReady: true,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: const Material(
          child: PackageListItem(
            icon: Icons.extension_outlined,
            title: 'Demo package',
            subtitle: 'A package card',
            metadata: <String>['demo'],
            enabled: true,
            onEnabledChanged: _ignorePackageEnabled,
          ),
        ),
      ),
    );
    expect(
      find.descendant(
        of: find.byType(PackageListItem),
        matching: find.byType(BackdropFilter),
      ),
      findsNothing,
    );
    expect(find.text('Demo package'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('package manager action opens the runtime environment editor', (
    tester,
  ) async {
    final bridge = _CatalogBridge();
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: PackageManagerScreen(clients: GeneratedCoreProxyClients(bridge)),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('更多操作'));
    await tester.pumpAndSettle();
    final action = find.ancestor(
      of: find.byIcon(Icons.settings_outlined),
      matching: find.byType(FloatingActionButton),
    );
    expect(action, findsOneWidget);
    await tester.tap(action);
    await tester.pumpAndSettle();
    expect(find.byType(PackageEnvironmentVariablesDialog), findsOneWidget);
    expect(
      find.descendant(
        of: find.byType(PackageEnvironmentVariablesDialog),
        matching: find.byType(TextField),
      ),
      findsNothing,
    );
    expect(bridge.environmentCatalogReads, 1);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });
}

/// Ignores switch changes in the package-card rendering probe.
void _ignorePackageEnabled(bool enabled) {}

/// Mounts the production package controls with the supplied runtime bridge.
Widget _manager(_CatalogBridge bridge) => OperitTheme(
  initialThemePreferenceSnapshot:
      UserPreferencesManager.defaultThemePreferenceSnapshot,
  initialThemeIsReady: false,
  unconfiguredChildEnabled: true,
  hostInteractionHostsEnabled: false,
  child: PackageManagerScreen(clients: GeneratedCoreProxyClients(bridge)),
);

class _CatalogBridge extends OperitRuntimeBridge {
  /// Rejects push operations outside this catalog fixture.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  /// Rejects snapshot watches outside this catalog fixture.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Exposes the catalog revision stream with observable listener ownership.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    if (request.propertyName != 'extensionCatalogRevisionFlow') {
      throw StateError('Unexpected catalog watch: ${request.propertyName}');
    }
    return Stream<CoreEvent>.multi((controller) {
      watchers += 1;
      controller.add(_revisionEvent(request, 'Snapshot', 0));
      final subscription = revisions.stream.listen((revision) {
        controller.add(_revisionEvent(request, 'Changed', revision));
      });
      controller.onCancel = () {
        watchers -= 1;
        unawaited(subscription.cancel());
      };
    });
  }

  /// Creates a complete revision event using the same typed payload as Core.
  CoreEvent _revisionEvent(
    CoreWatchRequest request,
    String kind,
    int revision,
  ) => CoreEvent.raw(
    requestId: request.requestId,
    target: request.target,
    propertyName: request.propertyName,
    kind: kind,
    valueBytes: encodeCoreLink(revision),
    decodeValue: decodeCoreLink<Object?>,
  );

  final revisions = StreamController<int>.broadcast();
  Completer<void>? catalogReadGate;
  Completer<void>? catalogReadStarted;
  int activeCatalogReads = 0;
  int maxActiveCatalogReads = 0;
  bool installed = false;
  bool enabled = false;
  Object? disableError;
  Object? deleteIssueError;
  final loadIssues = <core_proxy.ToolPkgLoadIssue>[];
  final deletedIssues = <core_proxy.ToolPkgLoadIssue>[];
  final extraCandidates = <core_proxy.BundledExternalPackageCandidate>[];
  final extraRuntimes = <Map<String, Object?>>[];
  final installedContainers = <Map<String, Object?>>[];
  final executablePackages = <String, core_proxy.ToolPackage>{};
  final scopes = <String, String>{};
  final customizedPackageNames = <String>[];
  final deletedPackages = <String>[];
  bool restoredBuiltin = false;
  final imports = <String>[];
  final scopeChanges = <Map<String, Object?>>[];
  final preferences = <String, String>{};
  final mutations = <String>[];
  int scans = 0;
  int watchers = 0;
  int catalogReads = 0;
  int environmentCatalogReads = 0;

  /// Encodes catalog responses with the native bridge envelope.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async =>
      encodeCoreLink(<Object?>[0, await call(request)]);

  /// Returns the catalog state available at the time of each request.
  @override
  Future<Object?> call(CoreCallRequest request) async {
    switch (request.methodName) {
      case 'loadAvailablePackages':
        scans++;
        return null;
      case 'getAvailablePackages':
        environmentCatalogReads++;
        return <String, Object?>{};
      case 'getCustomizedPackageNames':
        return List<String>.of(customizedPackageNames);
      case 'getToolPkgContainerDetails':
        return null;
      case 'deletePackage':
        final name = (request.args as Map)['packageName'] as String;
        deletedPackages.add(name);
        final customized = customizedPackageNames.remove(name);
        if (customized) {
          if (executablePackages[name]
              case final core_proxy.ToolPackage package) {
            executablePackages[name] = core_proxy.ToolPackage.fromJson(
              package.toJson()..['is_built_in'] = true,
            );
          }
          restoredBuiltin = true;
        } else if (name == 'demo_toolpkg') {
          installed = false;
          enabled = false;
        } else {
          executablePackages.remove(name);
        }
        return true;
      case 'getExtensionScopes':
        return <String, Object?>{
          if (installed) 'demo_toolpkg': 'device',
          ...scopes,
        };
      case 'getExecutableAvailablePackages':
        return executablePackages.map(
          (name, package) => MapEntry(name, package.toJson()),
        );
      case 'getToolPkgContainerRuntimes':
        catalogReads += 1;
        activeCatalogReads += 1;
        maxActiveCatalogReads = math.max(
          maxActiveCatalogReads,
          activeCatalogReads,
        );
        final result = <Object?>[
          if (installed)
            _pluginRuntime().toJson()
              ..['sourceType'] = restoredBuiltin
                  ? core_proxy.ToolPkgSourceType.asset.toJson()
                  : core_proxy.ToolPkgSourceType.externalValue.toJson(),
          ...installedContainers,
        ];
        final started = catalogReadStarted;
        catalogReadStarted = null;
        started?.complete();
        await catalogReadGate?.future;
        activeCatalogReads -= 1;
        return result;
      case 'getEnabledPackageNames':
        return installed && enabled ? ['demo_toolpkg'] : [];
      case 'enableToolPkgContainer':
        expect((request.args as Map)['containerPackageName'], 'demo_toolpkg');
        enabled = true;
        mutations.add('enable');
        return 'Enabled';
      case 'disableToolPkgContainer':
        expect((request.args as Map)['containerPackageName'], 'demo_toolpkg');
        if (disableError case final Object error) throw error;
        enabled = false;
        mutations.add('disable');
        return 'Disabled';
      case 'getPreferences':
        expect(
          (request.args as Map)['fileName'],
          'user_preferences.preferences.json',
        );
        return Map<String, String>.of(preferences);
      case 'setPreferences':
        expect(
          (request.args as Map)['fileName'],
          'user_preferences.preferences.json',
        );
        preferences.addAll(
          ((request.args as Map)['values'] as Map).cast<String, String>(),
        );
        mutations.add('selection');
        return null;
      case 'deleteToolPkgLoadIssue':
        if (deleteIssueError case final Object error) throw error;
        final issue = core_proxy.ToolPkgLoadIssue.fromJson(
          ((request.args as Map)['issue'] as Map).cast<String, Object?>(),
        );
        deletedIssues.add(issue);
        loadIssues.clear();
        return null;
      case 'getToolPkgLoadIssues':
        return loadIssues.map((issue) => issue.toJson()).toList();
      case 'getBundledExternalPackageCandidates':
        return extraCandidates.map((candidate) => candidate.toJson()).toList();
      case 'getBundledExternalToolPkgContainerRuntimes':
        return extraRuntimes;
      case 'importBundledExternalPackage':
        imports.add((request.args as Map)['packageName'] as String);
        extraCandidates.clear();
        return 'Successfully imported package';
      case 'importBundledExternalToolPkgContainer':
        imports.add((request.args as Map)['containerPackageName'] as String);
        extraCandidates.clear();
        extraRuntimes.clear();
        return 'Successfully imported package';
      case 'setExtensionScope':
        scopeChanges.add((request.args as Map).cast<String, Object?>());
        return null;
      case 'getToolPkgContainerOrder':
        return [
          if (installed) 'demo_toolpkg',
          for (final container in installedContainers)
            container['packageName'] as String,
        ];
    }
    throw StateError('Unexpected catalog call: ${request.methodName}');
  }
}

/// Creates a standalone script package shown on the package tab.
core_proxy.ToolPackage _namedPackage(String name, String displayName) {
  return core_proxy.ToolPackage(
    name: name,
    description: const core_proxy.LocalizedText(values: <String, String>{}),
    displayName: core_proxy.LocalizedText(
      values: <String, String>{'default': displayName},
    ),
    tools: const <core_proxy.PackageTool>[],
    states: const <core_proxy.ToolPackageState>[],
    env: const <core_proxy.EnvVar>[],
    isBuiltIn: false,
    enabledByDefault: true,
    category: 'Other',
    author: const <String>[],
  );
}

/// Creates an installed plugin fixture for catalog refresh assertions.
core_proxy.ToolPkgContainerRuntime _pluginRuntime() {
  return const core_proxy.ToolPkgContainerRuntime(
    chatLifecycleHooks: [],
    packageName: 'demo_toolpkg',
    displayName: core_proxy.LocalizedText(
      values: <String, String>{'default': 'Demo ToolPkg'},
    ),
    description: core_proxy.LocalizedText(
      values: <String, String>{'default': 'DSL test package'},
    ),
    version: '1.0.0',
    apiVersion: '2.0.0',
    publicApi: null,
    publicApis: <core_proxy.ToolPkgRegisteredFunctionHook>[],
    requires: <core_proxy.ToolPkgManifestRequirement>[],
    dependencyIssues: <core_proxy.ToolPkgDependencyIssue>[],
    manifestExtensions: <String, Object?>{},
    author: <String>['Operit'],
    mainEntry: 'dist/main.js',
    themes: <core_proxy.ToolPkgManifestTheme>[],
    license: null,
    sourceType: core_proxy.ToolPkgSourceType.externalValue,
    sourcePath: 'test',
    subpackages: <core_proxy.ToolPkgSubpackageRuntime>[],
    resources: <core_proxy.ToolPkgResourceRuntime>[],
    wasmModules: <core_proxy.ToolPkgWasmModuleRuntime>[],
    workflowTemplates: <core_proxy.ToolPkgWorkflowTemplateRuntime>[],
    workspaceTemplates: <core_proxy.ToolPkgWorkspaceTemplateRuntime>[],
    uiModules: <core_proxy.ToolPkgUiModuleRuntime>[
      core_proxy.ToolPkgUiModuleRuntime(
        id: 'main',
        runtime: 'compose_dsl',
        screenExport: null,
        screen: 'ui/main.js',
        title: core_proxy.LocalizedText(
          values: <String, String>{'default': 'Main route'},
        ),
        keepAlive: true,
      ),
    ],
    uiRoutes: <core_proxy.ToolPkgUiRouteRuntime>[
      core_proxy.ToolPkgUiRouteRuntime(
        id: 'main',
        routeId: 'main',
        runtime: 'compose_dsl',
        screenExport: null,
        screen: 'ui/main.js',
        title: core_proxy.LocalizedText(
          values: <String, String>{'default': 'Main route'},
        ),
        keepAlive: true,
      ),
    ],
    chatComposerSlots: <core_proxy.ToolPkgChatComposerSlotRuntime>[],
    navigationEntries: <core_proxy.ToolPkgNavigationEntryRuntime>[
      core_proxy.ToolPkgNavigationEntryRuntime(
        id: 'sidebar-demo',
        routeId: 'main',
        params: null,
        surface: 'chat_sidebar_tabs',
        title: core_proxy.LocalizedText(
          values: <String, String>{'default': 'Demo sidebar'},
        ),
        action: null,
        icon: null,
        order: 0,
      ),
    ],
    desktopWidgets: <core_proxy.ToolPkgDesktopWidgetRuntime>[],
    appLifecycleHooks: <core_proxy.ToolPkgAppLifecycleHookRuntime>[],
    messageProcessingPlugins: <core_proxy.ToolPkgFunctionHookRuntime>[],
    xmlRenderPlugins: <core_proxy.ToolPkgTagFunctionHookRuntime>[],
    inputMenuTogglePlugins: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatInputHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatViewHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatMessageHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatResponseHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatMessageMenuItems: <core_proxy.ToolPkgChatMessageMenuItemRuntime>[],
    chatRuntimeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    hostEventHooks: <core_proxy.ToolPkgHostEventHookRuntime>[],
    toolLifecycleHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptInputHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptHistoryHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptEstimateHistoryHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    systemPromptComposeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    toolPromptComposeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptFinalizeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptEstimateFinalizeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    summaryGenerateHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    coreCommands: <core_proxy.ToolPkgCoreCommandRuntime>[],
    aiProviders: <core_proxy.ToolPkgAiProviderRuntime>[],
    manifestExtensionHandlers:
        <core_proxy.ToolPkgRegisteredManifestExtension>[],
    logoResource: null,
    marketOrigin: null,
  );
}

/// Creates extra-package metadata with an optional manifest installation restriction.
core_proxy.BundledExternalPackageCandidate _extraCandidate(
  String name, {
  String? requiredScope,
}) => core_proxy.BundledExternalPackageCandidate(
  packageName: name,
  displayName: const core_proxy.LocalizedText(
    values: {'default': 'Extra plugin'},
  ),
  description: const core_proxy.LocalizedText(
    values: {'default': 'Extra plugin fixture'},
  ),
  author: const ['Operit'],
  packageKind: 'toolpkg',
  sourcePath: '$name.toolpkg',
  sourceFileName: '$name.toolpkg',
  isToolPkg: true,
  version: '1.0.0',
  category: 'ToolPkg',
  toolCount: 0,
  subpackageCount: 0,
  requiredInstallationScope: requiredScope,
);
