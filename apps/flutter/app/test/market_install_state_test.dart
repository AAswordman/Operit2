import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/packages/market/MarketBrowseList.dart';
import 'package:operit2/ui/features/packages/market/MarketInstallStateStore.dart';
import 'package:operit2/ui/features/packages/screens/MarketEntryDetailScreen.dart';
import 'package:operit2/ui/features/packages/screens/UnifiedMarketScreen.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/main/TopBarController.dart';

/// Verifies market installation state and bottom navigation layout.
void main() {
  final entry = _entry();

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

  testWidgets(
    'installed market cards expose their status without reinstalling',
    (tester) async {
      var actions = 0;
      var details = 0;
      await tester.pumpWidget(
        _testApp(
          Scaffold(
            body: MarketGridCard(
              title: 'Plugin',
              description: 'A plugin',
              author: 'Author',
              downloads: 0,
              likes: 0,
              hearts: 0,
              actionLabel: '已安装',
              actionIcon: Icons.check,
              actionBusy: false,
              actionEnabled: false,
              statusLabel: '已安装',
              onAction: () => actions++,
              onTap: () => details++,
            ),
          ),
        ),
      );
      expect(find.text('已安装'), findsOneWidget);
      await tester.tap(find.byIcon(Icons.check));
      expect(actions, 0);
      details = 0;
      await tester.tap(find.text('Plugin'));
      expect(details, 1);
      expect(tester.takeException(), isNull);
    },
  );

  for (final (topInset, bottomInset) in <(double, double)>[
    (0, 0),
    (24, 0),
    (24, 48),
    (48, 24),
  ]) {
    testWidgets('market bottom navigation excludes the top system inset '
        '(top: $topInset, bottom: $bottomInset)', (tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(400, 800);
      tester.view.padding = FakeViewPadding(top: topInset, bottom: bottomInset);
      tester.view.viewPadding = FakeViewPadding(
        top: topInset,
        bottom: bottomInset,
      );
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetPadding);
      addTearDown(tester.view.resetViewPadding);

      await tester.pumpWidget(
        _testApp(
          Scaffold(
            body: SafeArea(
              top: false,
              child: UnifiedMarketScreen(
                clients: GeneratedCoreProxyClients(_MarketBridge()),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      final navigation = find.byType(NavigationBar);
      final bounds = tester.getRect(navigation);
      expect(bounds.height, 56);
      expect(bounds.bottom, 800 - bottomInset);
      expect(
        tester.getCenter(find.byIcon(Icons.storefront)).dy,
        bounds.center.dy,
      );
      expect(
        MediaQuery.paddingOf(
          tester.element(find.byType(UnifiedMarketScreen)),
        ).top,
        topInset,
      );
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('list installation succeeds inline without any bottom popup', (
    tester,
  ) async {
    final bridge = _MarketBridge();
    await tester.pumpWidget(
      _testApp(
        Scaffold(
          body: UnifiedMarketScreen(clients: GeneratedCoreProxyClients(bridge)),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('安装'));
    await tester.pumpAndSettle();
    expect(find.text('已安装'), findsOneWidget);
    expect(find.byIcon(Icons.check), findsOneWidget);
    expect(find.byType(SnackBar), findsNothing);
    expect(bridge.installCalls, 1);
  });

  testWidgets(
    'list shows available updates and clears the badge after updating',
    (tester) async {
      final bridge = _MarketBridge()..versions['entry'] = 'v1';
      await tester.pumpWidget(
        _testApp(
          Scaffold(
            body: UnifiedMarketScreen(
              clients: GeneratedCoreProxyClients(bridge),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('可更新'), findsOneWidget);
      await tester.tap(find.byTooltip('更新'));
      await tester.pumpAndSettle();
      expect(find.text('可更新'), findsNothing);
      expect(find.text('已安装'), findsOneWidget);
      expect(find.byType(SnackBar), findsNothing);
    },
  );

  testWidgets('installing in detail immediately updates the underlying list', (
    tester,
  ) async {
    final bridge = _MarketBridge();
    await tester.pumpWidget(
      _testApp(
        Scaffold(
          body: UnifiedMarketScreen(clients: GeneratedCoreProxyClients(bridge)),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Plugin'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, '安装'));
    await tester.pumpAndSettle();
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(find.text('已安装'), findsOneWidget);
    expect(find.byType(SnackBar), findsNothing);
    expect(bridge.installCalls, 1);
  });

  testWidgets(
    'successful installation updates detail inline with no bottom popup',
    (tester) async {
      final bridge = _MarketBridge();
      await tester.pumpWidget(_detail(bridge));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, '安装'));
      await tester.pumpAndSettle();
      expect(bridge.installCalls, 1);
      expect(find.text('已安装'), findsNWidgets(2));
      expect(find.byType(SnackBar), findsNothing);
      expect(find.byIcon(Icons.check), findsOneWidget);
      final button = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, '已安装'),
      );
      expect(button.onPressed, isNull);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'failed installation still shows an error, never an installed badge',
    (tester) async {
      final bridge = _MarketBridge()..installError = 'Download failed';
      await tester.pumpWidget(_detail(bridge));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, '安装'));
      await tester.pumpAndSettle();
      expect(find.byType(SnackBar), findsOneWidget);
      expect(find.textContaining('Download failed'), findsOneWidget);
      expect(find.text('已安装'), findsNothing);
      expect(find.widgetWithText(FilledButton, '安装'), findsOneWidget);
    },
  );

  testWidgets(
    'updatable details switch to installed after update without a popup',
    (tester) async {
      final bridge = _MarketBridge()..versions['entry'] = 'v1';
      await tester.pumpWidget(_detail(bridge));
      await tester.pumpAndSettle();
      expect(find.text('可更新'), findsOneWidget);
      await tester.tap(find.widgetWithText(FilledButton, '更新'));
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
      'featured': true,
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
  MarketEntryDetailScreen(
    clients: GeneratedCoreProxyClients(bridge),
    entry: _entry(),
  ),
);

Widget _testApp(Widget child) {
  final controller = TopBarController();
  addTearDown(controller.dispose);
  return OperitTheme(
    initialThemePreferenceSnapshot:
        UserPreferencesManager.defaultThemePreferenceSnapshot,
    initialThemeIsReady: false,
    unconfiguredChildEnabled: true,
    hostInteractionHostsEnabled: false,
    child: TopBarScope(controller: controller, child: child),
  );
}

class _MarketBridge extends OperitRuntimeBridge {
  final Map<String, String> versions = <String, String>{};
  int installCalls = 0;
  String? requestedVersion;
  String? installError;
  Completer<String>? pendingInstall;
  Completer<Map<String, String>>? pendingVersions;

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
        result = {
          'page': 1,
          'pageSize': 50,
          'total': 1,
          'items': [_entry().toJson()],
        };
      case 'get_comments_page':
        result = {
          'entryId': 'entry',
          'page': 1,
          'pageSize': 50,
          'total': 0,
          'items': <Object?>[],
        };
      case 'get_current_github_user':
        result = {'id': 1, 'login': 'tester'};
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
