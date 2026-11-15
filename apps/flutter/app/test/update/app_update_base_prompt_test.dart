import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/update/AppUpdateController.dart';
import 'package:operit2/core/update/AppUpdateModels.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/common/OperitLogoMark.dart';
import 'package:operit2/ui/features/settings/about/AboutOperitScreen.dart';
import 'package:operit2/ui/features/update/AppUpdateBadge.dart';
import 'package:operit2/ui/features/update/AppUpdateDialog.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:package_info_plus/package_info_plus.dart';

class _Backend extends AppUpdateBackend {
  bool fullAvailable = true;
  bool rejectBase = false;
  bool hasUpdate = true;
  bool failCheck = false;
  int plans = 0, downloads = 0, imports = 0;
  AppUpdateMode transfer = AppUpdateMode.needsBase;
  Completer<void>? verification;

  /// Describes the same Core decision throughout a single update attempt.
  AppUpdatePlan _plan(AppUpdateMode mode) => AppUpdatePlan(
    mode: mode,
    fullAvailable: fullAvailable,
    incrementalAvailable: true,
    baseVersion: '2.0.0-preview.35',
    fullBytes: 120000000,
    incrementalBytes: 12000000,
  );

  /// Models a newer release without initiating a package transfer.
  @override
  Future<AppUpdateRelease?> check() async {
    if (failCheck) throw StateError('HTTP 403: Forbidden');
    return hasUpdate
        ? AppUpdateRelease(
            currentVersion: '2.0.0-preview.35',
            version: '2.0.0-preview.36',
            releasePageUrl: Uri.parse('https://example.com/release'),
          )
        : null;
  }

  /// Records the initial decision boundary.
  @override
  Future<AppUpdatePlan> plan(AppUpdateRelease release) async {
    plans++;
    return _plan(transfer);
  }

  /// Allows tests to observe that no patch starts before verification completes.
  @override
  Future<AppUpdatePlan> supplyBase(
    AppUpdateRelease release,
    String path,
  ) async {
    imports++;
    if (verification != null) await verification!.future;
    if (rejectBase) throw const AppUpdateBaseMismatch();
    return _plan(AppUpdateMode.incremental);
  }

  /// Preserves the known full availability when the user declines an original.
  @override
  Future<AppUpdatePlan> chooseFull(AppUpdateRelease release) async =>
      _plan(fullAvailable ? AppUpdateMode.full : AppUpdateMode.unavailable);

  /// Records which transfer was selected by the controller.
  @override
  Future<String> download(
    AppUpdateRelease release,
    AppUpdateProgress progress, {
    void Function()? onSelectingSource,
    void Function()? onReconstructing,
  }) async {
    downloads++;
    onReconstructing?.call();
    return 'verified-receipt';
  }

  /// Returns the shared installation state after a verified transfer.
  @override
  Future<AppUpdateInstallResult> install(String receipt) async =>
      const AppUpdateInstallResult(AppUpdateInstallState.restartRequired);

  /// Implements the update boundary without native work in tests.
  @override
  Future<void> cancelDownload() async {}

  /// Implements the update boundary without restarting tests.
  @override
  Future<void> restartToInstall() async {}
}

/// Renders real update components with their application theme dependencies.
Widget _app(Widget child) => OperitTheme(
  initialThemePreferenceSnapshot:
      UserPreferencesManager.defaultThemePreferenceSnapshot,
  initialThemeIsReady: false,
  unconfiguredChildEnabled: true,
  hostInteractionHostsEnabled: false,
  child: Scaffold(body: child),
);

/// Exercises user choices while treating Core's plan as the authority.
void main() {
  setUpAll(ClientLogger.initialize);
  test(
    'selected original must pass verification before any patch download',
    () async {
      final backend = _Backend()..verification = Completer<void>();
      final removed = <String>[];
      final controller = AppUpdateController(
        backend: backend,
        pickBase: () async => '/original',
        deleteSelection: (path) async {
          removed.add(path);
        },
      );
      await controller.check();
      await controller.start();
      expect(controller.phase, AppUpdatePhase.awaitingBase);
      expect(backend.downloads, 0);
      final selected = controller.selectBase();
      await Future<void>.delayed(Duration.zero);
      expect(controller.phase, AppUpdatePhase.validatingBase);
      expect(backend.downloads, 0);
      backend.verification!.complete();
      await selected;
      expect(controller.plan!.mode, AppUpdateMode.incremental);
      expect(backend.downloads, 1);
      expect(backend.plans, 1);
      expect(removed, ['/original']);
      expect(controller.phase, AppUpdatePhase.ready);
      controller.dispose();
    },
  );

  test(
    'picker cancellation and rejected original keep the prompt usable',
    () async {
      final backend = _Backend()..rejectBase = true;
      bool cancel = true;
      final controller = AppUpdateController(
        backend: backend,
        pickBase: () async => cancel ? null : '/wrong',
        deleteSelection: (_) async {},
      );
      await controller.check();
      await controller.start();
      await controller.selectBase();
      expect(controller.phase, AppUpdatePhase.awaitingBase);
      expect(controller.error, isNull);
      cancel = false;
      await controller.selectBase();
      expect(controller.error, contains('所选安装包不符合要求'));
      expect(controller.phase, AppUpdatePhase.awaitingBase);
      expect(backend.downloads, 0);
      backend.rejectBase = false;
      await controller.selectBase();
      expect(backend.downloads, 1);
      controller.dispose();
    },
  );

  test('no-original choice uses full only when declared available', () async {
    for (final full in [true, false]) {
      final backend = _Backend()..fullAvailable = full;
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      await controller.start();
      await controller.withoutBase();
      expect(backend.plans, 1);
      expect(backend.downloads, full ? 1 : 0);
      expect(
        controller.phase,
        full ? AppUpdatePhase.ready : AppUpdatePhase.unavailable,
      );
      if (!full) {
        controller.acknowledgeUnavailable();
        await controller.start();
        expect(controller.phase, AppUpdatePhase.awaitingBase);
        expect(backend.downloads, 0);
      }
      controller.dispose();
    }
  });

  test(
    'full plan downloads immediately without selecting an original',
    () async {
      final backend = _Backend()..transfer = AppUpdateMode.full;
      final controller = AppUpdateController(
        backend: backend,
        pickBase: () async => throw StateError('Picker must not open'),
      );
      await controller.check();
      await controller.start();
      expect(backend.downloads, 1);
      expect(backend.imports, 0);
      controller.dispose();
    },
  );

  testWidgets(
    'prompt has the required choices and unavailable state has only acknowledgement',
    (tester) async {
      final backend = _Backend()..fullAvailable = false;
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      await tester.pumpWidget(_app(AppUpdateDialog(controller: controller)));
      await tester.pumpAndSettle();
      expect(find.text('你还有上一次安装（版本 2.0.0-preview.35）的安装包吗？'), findsOneWidget);
      expect(find.text('选择'), findsOneWidget);
      expect(find.text('我没安装包了'), findsOneWidget);
      expect(backend.downloads, 0);
      await tester.tap(find.text('我没安装包了'));
      await tester.pumpAndSettle();
      expect(find.text('暂时无法完成更新'), findsOneWidget);
      expect(find.text('知道了'), findsOneWidget);
      expect(find.byType(TextButton), findsNothing);
      expect(find.byType(FilledButton), findsOneWidget);
      await tester.pumpWidget(const SizedBox());
      controller.dispose();
    },
  );

  testWidgets(
    'about exposes a failed check and retries without starting a download',
    (tester) async {
      PackageInfo.setMockInitialValues(
        appName: 'Operit2',
        packageName: 'app.operit',
        version: '2.0.0',
        buildNumber: '43',
        buildSignature: '',
      );
      final backend = _Backend()..failCheck = true;
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      await tester.pumpWidget(
        _app(
          AppUpdateScope(
            controller: controller,
            child: const AboutOperitScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('检查更新失败，点击重试'), findsOneWidget);
      expect(controller.error, contains('403'));
      backend.failCheck = false;
      await tester.tap(find.text('检查更新'));
      await tester.pumpAndSettle();
      expect(find.text('检查更新失败，点击重试'), findsNothing);
      expect(find.text('发现新版本 2.0.0-preview.36'), findsWidgets);
      expect(find.byType(AppUpdateDialog), findsOneWidget);
      expect(backend.downloads, 0);
      Navigator.of(tester.element(find.byType(AppUpdateDialog))).pop();
      await tester.pumpAndSettle();
      await tester.pumpWidget(const SizedBox());
      controller.dispose();
    },
  );

  testWidgets(
    'about keeps the logo centered without an indicator and uses a settings update row',
    (tester) async {
      PackageInfo.setMockInitialValues(
        appName: 'Operit2',
        packageName: 'app.operit',
        version: '2.0.0',
        buildNumber: '42',
        buildSignature: '',
      );
      final backend = _Backend()..hasUpdate = false;
      final controller = AppUpdateController(backend: backend);
      await tester.pumpWidget(
        _app(
          AppUpdateScope(
            controller: controller,
            child: const AboutOperitScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('从安装包更新'), findsNothing);
      expect(find.byKey(const ValueKey('app-update-logo')), findsNothing);
      expect(find.text('检查更新'), findsOneWidget);
      expect(find.byType(FilledButton), findsNothing);
      final logoBefore = tester.getRect(find.byType(OperitLogoMark));
      backend.hasUpdate = true;
      await controller.check(force: true);
      await tester.pumpAndSettle();
      expect(tester.getRect(find.byType(OperitLogoMark)), logoBefore);
      expect(find.byKey(const ValueKey('app-update-badge')), findsNothing);
      expect(find.text('发现新版本 2.0.0-preview.36'), findsOneWidget);
      await tester.tap(find.text('检查更新'));
      await tester.pumpAndSettle();
      expect(find.byType(AppUpdateDialog), findsOneWidget);
      expect(controller.phase, AppUpdatePhase.awaitingBase);
      Navigator.of(tester.element(find.byType(AppUpdateDialog))).pop();
      await tester.pumpAndSettle();
      await tester.pumpWidget(const SizedBox());
      controller.dispose();
    },
  );
}
