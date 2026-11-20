import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/update/AppUpdateController.dart';
import 'package:operit2/core/update/AppUpdateModels.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/features/update/AppUpdateBadge.dart';
import 'package:operit2/ui/features/update/AppUpdateDialog.dart';
import 'package:operit2/ui/common/OperitLogoMark.dart';
import 'package:operit2/ui/main/components/CollapsedDrawerContent.dart';
import 'package:operit2/ui/main/components/DrawerContent.dart';
import 'package:operit2/ui/main/components/NavigationDrawerAppearance.dart';
import 'package:operit2/ui/main/screens/ScreenRouteRegistry.dart';
import '../support/native_sidebar_bridge.dart';

final release = AppUpdateRelease(
  currentVersion: '1.0.0',
  version: '1.1.0',
  releasePageUrl: Uri.parse(
    'https://github.com/AAswordman/Operit2/releases/tag/v1.1.0',
  ),
  releaseNotes: '更新体验优化\n改进原生应用安装反馈',
);

class Backend extends AppUpdateBackend {
  AppUpdateRelease? availableRelease = release;
  Completer<AppUpdateRelease?>? checking;
  bool selectingSource = false;
  void Function()? selectSource;
  int downloads = 0, installs = 0;
  bool failInstall = false;
  bool failCancel = false;
  AppUpdateInstallState state = AppUpdateInstallState.installerOpened;
  Completer<String>? downloadPending;
  Stream<AppUpdateNetwork> network = const Stream.empty();
  int restarts = 0;
  AppUpdateMode mode = AppUpdateMode.full;
  bool fullAvailable = true;
  int? fullBytes = 120000000, incrementalBytes = 5451829;
  int plans = 0;
  Completer<void>? planning;
  @override
  Stream<AppUpdateNetwork> get backgroundNetwork => network;
  AppUpdateProgress? onProgress;
  void Function()? onReconstructing;
  void Function(AppUpdatePreparationProgress)? onPreparationProgress;
  @override
  Future<AppUpdateRelease?> check() async =>
      checking == null ? availableRelease : checking!.future;

  /// Describes sizes for both routes without confusing the patch and final installer.
  AppUpdatePlan _plan(AppUpdateMode mode) => AppUpdatePlan(
    mode: mode,
    fullAvailable: fullAvailable,
    incrementalAvailable: mode != AppUpdateMode.full,
    fullBytes: fullBytes,
    incrementalBytes: incrementalBytes,
  );

  /// Models route planning independently of the current network policy.
  @override
  Future<AppUpdatePlan> plan(AppUpdateRelease release) async {
    plans++;
    await planning?.future;
    return _plan(mode);
  }

  /// Does not accept originals for a full-only plan.
  @override
  Future<AppUpdatePlan> supplyBase(
    AppUpdateRelease release,
    String path,
  ) async => throw StateError('No baseline requested');

  /// Confirms the full route in this fixture.
  @override
  Future<AppUpdatePlan> chooseFull(AppUpdateRelease release) async =>
      _plan(AppUpdateMode.full);
  @override
  Future<String> download(
    AppUpdateRelease r,
    AppUpdateProgress progress, {
    void Function()? onSelectingSource,
    void Function()? onReconstructing,
    void Function(AppUpdatePreparationProgress)? onPreparationProgress,
  }) async {
    downloads++;
    onProgress = progress;
    this.onReconstructing = onReconstructing;
    this.onPreparationProgress = onPreparationProgress;
    selectSource = onSelectingSource;
    if (selectingSource) {
      onSelectingSource?.call();
    } else {
      progress(48 * 1024 * 1024, 120 * 1024 * 1024);
    }
    return downloadPending == null
        ? 'fixture-update-receipt'
        : downloadPending!.future;
  }

  @override
  Future<void> cancelDownload() async {
    if (failCancel) throw StateError('cancel transport unavailable');
    if (downloadPending != null && !downloadPending!.isCompleted) {
      downloadPending!.completeError(const AppUpdateCancelled());
    }
  }

  @override
  Future<AppUpdateInstallResult> install(String path) async {
    installs++;
    if (failInstall) throw StateError('installation rejected');
    return AppUpdateInstallResult(state, details: '系统安装器已打开');
  }

  @override
  Future<void> restartToInstall() async {
    restarts++;
  }
}

void main() {
  setUpAll(ClientLogger.initialize);
  testWidgets(
    'local steps show measured progress and elapsed time across native waits',
    (tester) async {
      final backend = Backend()..downloadPending = Completer<String>();
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      final pending = controller.start();
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot:
              UserPreferencesManager.defaultThemePreferenceSnapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: AppUpdateDialog(controller: controller),
        ),
      );
      await tester.pump();
      backend.onReconstructing!();
      backend.onPreparationProgress!(
        const AppUpdatePreparationProgress(
          AppUpdatePreparationPhase.applyingPatch,
          40,
          100,
        ),
      );
      await tester.pump();
      expect(find.text('正在合并补丁…'), findsOneWidget);
      expect(find.text('当前步骤 40%'), findsOneWidget);
      expect(
        tester
            .widget<LinearProgressIndicator>(
              find.byType(LinearProgressIndicator),
            )
            .value,
        0.4,
      );
      backend.onPreparationProgress!(
        const AppUpdatePreparationProgress(
          AppUpdatePreparationPhase.restoringPackage,
          0,
          0,
        ),
      );
      await tester.pump(const Duration(seconds: 2));
      expect(find.text('正在生成新安装包…'), findsOneWidget);
      expect(find.text('当前步骤 40%'), findsNothing);
      expect(find.textContaining('已用时 '), findsOneWidget);
      expect(
        tester
            .widget<LinearProgressIndicator>(
              find.byType(LinearProgressIndicator),
            )
            .value,
        isNull,
      );
      expect(find.text('取消更新'), findsOneWidget);
      backend.onPreparationProgress!(
        const AppUpdatePreparationProgress(
          AppUpdatePreparationPhase.restoringPackage,
          75,
          100,
        ),
      );
      await tester.pump();
      expect(find.text('当前步骤 75%'), findsOneWidget);
      backend.downloadPending!.complete('fixture-update-receipt');
      await pending;
      await tester.pump();
      expect(controller.phase, AppUpdatePhase.ready);
      expect(find.textContaining('已用时 '), findsNothing);
      expect(backend.installs, 0);
      await tester.pumpWidget(const SizedBox.shrink());
      controller.dispose();
    },
  );
  testWidgets(
    'late native progress after disposal cannot restart repaint ticks',
    (tester) async {
      final backend = Backend()..downloadPending = Completer<String>();
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      final pending = controller.start();
      await tester.pump();
      backend.onReconstructing!();
      controller.dispose();
      await pending;
      backend.onPreparationProgress!(
        const AppUpdatePreparationProgress(
          AppUpdatePreparationPhase.restoringPackage,
          0,
          100,
        ),
      );
      backend.onReconstructing!();
      await tester.pump(const Duration(seconds: 2));
      expect(backend.installs, 0);
    },
  );
  test(
    'unmetered discovery prepares silently and installs only after confirmation',
    () async {
      final network = StreamController<AppUpdateNetwork>.broadcast();
      final backend = Backend()..network = network.stream;
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      expect(backend.downloads, 0);
      network.add(AppUpdateNetwork.unmetered);
      await Future<void>.delayed(Duration.zero);
      expect(controller.phase, AppUpdatePhase.ready);
      expect(backend.downloads, 1);
      expect(backend.installs, 0);
      await controller.start();
      expect(backend.downloads, 1);
      expect(backend.installs, 0);
      await controller.confirmInstall();
      expect(backend.installs, 1);
      controller.dispose();
      await network.close();
    },
  );
  test(
    'small patches and full packages prepare on restricted connections',
    () async {
      for (final mode in [AppUpdateMode.incremental, AppUpdateMode.full]) {
        final backend = Backend()
          ..mode = mode
          ..fullBytes = mode == AppUpdateMode.full ? 29999999 : 123536583
          ..network = Stream.value(AppUpdateNetwork.restricted);
        final controller = AppUpdateController(backend: backend);
        await controller.check();
        await Future<void>.delayed(Duration.zero);
        expect(controller.phase, AppUpdatePhase.ready);
        expect(backend.downloads, 1);
        expect(backend.installs, 0);
        controller.dispose();
      }
    },
  );
  test('30 MB and unknown sizes wait for unmetered networking', () async {
    for (final mode in [AppUpdateMode.incremental, AppUpdateMode.full]) {
      for (final size in [30000000, 30000001, null]) {
        final network = StreamController<AppUpdateNetwork>.broadcast();
        final backend = Backend()
          ..mode = mode
          ..fullBytes = size
          ..incrementalBytes = size
          ..network = network.stream;
        final controller = AppUpdateController(backend: backend);
        await controller.check();
        network.add(AppUpdateNetwork.restricted);
        await Future<void>.delayed(Duration.zero);
        expect(controller.phase, AppUpdatePhase.available);
        expect(backend.downloads, 0);
        network.add(AppUpdateNetwork.restricted);
        await Future<void>.delayed(Duration.zero);
        expect(backend.plans, 1);
        network.add(AppUpdateNetwork.unmetered);
        await Future<void>.delayed(Duration.zero);
        expect(controller.phase, AppUpdatePhase.ready);
        expect(backend.downloads, 1);
        controller.dispose();
        await network.close();
      }
    }
  });
  test(
    'missing originals apply the threshold to the selected full package',
    () async {
      for (final size in [29999999, 120000000]) {
        final backend = Backend()
          ..mode = AppUpdateMode.needsBase
          ..fullBytes = size
          ..network = Stream.value(AppUpdateNetwork.restricted);
        final controller = AppUpdateController(backend: backend);
        await controller.check();
        await Future<void>.delayed(Duration.zero);
        expect(controller.plan!.mode, AppUpdateMode.full);
        expect(backend.downloads, size < 30000000 ? 1 : 0);
        expect(backend.installs, 0);
        controller.dispose();
      }
    },
  );
  test(
    'a small update still requires an original and a connected network',
    () async {
      for (final network in [
        AppUpdateNetwork.offline,
        AppUpdateNetwork.restricted,
      ]) {
        final backend = Backend()
          ..mode = AppUpdateMode.needsBase
          ..fullAvailable = false
          ..network = Stream.value(network);
        final controller = AppUpdateController(backend: backend);
        await controller.check();
        await Future<void>.delayed(Duration.zero);
        expect(controller.phase, AppUpdatePhase.available);
        expect(backend.downloads, 0);
        expect(backend.plans, network == AppUpdateNetwork.offline ? 0 : 1);
        controller.dispose();
      }
    },
  );
  test(
    'small prefetch survives a switch to cellular but cancels offline',
    () async {
      final network = StreamController<AppUpdateNetwork>.broadcast();
      final backend = Backend()
        ..mode = AppUpdateMode.incremental
        ..network = network.stream
        ..downloadPending = Completer<String>();
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      network.add(AppUpdateNetwork.unmetered);
      await Future<void>.delayed(Duration.zero);
      network.add(AppUpdateNetwork.restricted);
      await Future<void>.delayed(Duration.zero);
      expect(controller.phase, AppUpdatePhase.downloading);
      expect(backend.downloadPending!.isCompleted, isFalse);
      expect(backend.downloads, 1);
      network.add(AppUpdateNetwork.offline);
      await Future<void>.delayed(Duration.zero);
      expect(controller.phase, AppUpdatePhase.available);
      expect(backend.downloadPending!.isCompleted, isTrue);
      controller.dispose();
      await network.close();
    },
  );
  test(
    'large prefetch stops on cellular and resumes on unmetered networking',
    () async {
      final network = StreamController<AppUpdateNetwork>.broadcast();
      final backend = Backend()
        ..network = network.stream
        ..downloadPending = Completer<String>();
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      network.add(AppUpdateNetwork.unmetered);
      await Future<void>.delayed(Duration.zero);
      network.add(AppUpdateNetwork.restricted);
      await Future<void>.delayed(Duration.zero);
      expect(controller.phase, AppUpdatePhase.available);
      expect(backend.downloads, 1);
      backend.downloadPending = null;
      network.add(AppUpdateNetwork.unmetered);
      await Future<void>.delayed(Duration.zero);
      expect(controller.phase, AppUpdatePhase.ready);
      expect(backend.downloads, 2);
      controller.dispose();
      await network.close();
    },
  );
  test('planning finishes on cellular and applies the selected size', () async {
    for (final size in [29999999, 30000000]) {
      final network = StreamController<AppUpdateNetwork>.broadcast();
      final backend = Backend()
        ..fullBytes = size
        ..network = network.stream
        ..planning = Completer<void>();
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      network.add(AppUpdateNetwork.unmetered);
      await Future<void>.delayed(Duration.zero);
      network.add(AppUpdateNetwork.restricted);
      await Future<void>.delayed(Duration.zero);
      backend.planning!.complete();
      await Future<void>.delayed(Duration.zero);
      expect(backend.downloads, size < 30000000 ? 1 : 0);
      expect(backend.plans, 1);
      controller.dispose();
      await network.close();
    }
  });
  test(
    'planning interrupted offline retries on a new cellular connection',
    () async {
      final network = StreamController<AppUpdateNetwork>.broadcast();
      final planning = Completer<void>();
      final backend = Backend()
        ..mode = AppUpdateMode.incremental
        ..network = network.stream
        ..planning = planning;
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      network.add(AppUpdateNetwork.unmetered);
      await Future<void>.delayed(Duration.zero);
      network.add(AppUpdateNetwork.offline);
      network.add(AppUpdateNetwork.restricted);
      await Future<void>.delayed(Duration.zero);
      backend.planning = null;
      planning.completeError(const AppUpdateCancelled());
      await Future<void>.delayed(Duration.zero);
      expect(controller.phase, AppUpdatePhase.ready);
      expect(backend.plans, 2);
      expect(backend.downloads, 1);
      controller.dispose();
      await network.close();
    },
  );
  test('manual cancellation suppresses small automatic retries', () async {
    final network = StreamController<AppUpdateNetwork>.broadcast();
    final backend = Backend()
      ..mode = AppUpdateMode.incremental
      ..network = network.stream
      ..downloadPending = Completer<String>();
    final controller = AppUpdateController(backend: backend);
    await controller.check();
    network.add(AppUpdateNetwork.restricted);
    await Future<void>.delayed(Duration.zero);
    await controller.cancelDownload();
    await Future<void>.delayed(Duration.zero);
    network.add(AppUpdateNetwork.offline);
    network.add(AppUpdateNetwork.unmetered);
    await Future<void>.delayed(Duration.zero);
    expect(controller.phase, AppUpdatePhase.available);
    expect(backend.downloads, 1);
    expect(backend.installs, 0);
    controller.dispose();
    await network.close();
  });
  testWidgets('a prepared update waits for the restart confirmation button', (
    tester,
  ) async {
    final backend = Backend()..state = AppUpdateInstallState.restartRequired;
    final controller = AppUpdateController(backend: backend);
    await controller.check();
    await controller.start();
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: AppUpdateDialog(controller: controller),
      ),
    );
    await tester.pump();
    expect(find.text('重启并更新'), findsOneWidget);
    expect(backend.installs, 0);
    expect(backend.restarts, 0);
    await tester.tap(find.text('重启并更新'));
    await tester.pump();
    expect(backend.installs, 1);
    expect(backend.restarts, 1);
    await tester.pumpWidget(const SizedBox.shrink());
    controller.dispose();
  });
  testWidgets(
    'opening updates during automatic download reuses the active task',
    (tester) async {
      final backend = Backend()
        ..mode = AppUpdateMode.incremental
        ..network = Stream.value(AppUpdateNetwork.restricted)
        ..downloadPending = Completer<String>();
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      await tester.pump();
      expect(controller.phase, AppUpdatePhase.downloading);
      expect(backend.downloads, 1);
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot:
              UserPreferencesManager.defaultThemePreferenceSnapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: AppUpdateDialog(controller: controller),
        ),
      );
      await tester.pump();
      await controller.start();
      await controller.confirmInstall();
      expect(backend.plans, 1);
      expect(backend.downloads, 1);
      expect(backend.installs, 0);
      expect(find.text('正在下载增量补丁…'), findsOneWidget);
      expect(find.text('重启并更新'), findsNothing);
      backend.onProgress!(75, 100);
      await tester.pump();
      expect(find.text('75%'), findsOneWidget);
      backend.downloadPending!.complete('fixture-update-receipt');
      await tester.pump();
      expect(controller.phase, AppUpdatePhase.ready);
      await tester.pump();
      expect(find.text('重启并更新'), findsOneWidget);
      expect(backend.installs, 0);
      await tester.tap(find.text('重启并更新'));
      await tester.pump();
      expect(backend.downloads, 1);
      expect(backend.installs, 1);
      await tester.pumpWidget(const SizedBox.shrink());
      controller.dispose();
    },
  );
  test(
    'cached bases use patches, missing bases use a published full package silently',
    () async {
      for (final mode in [AppUpdateMode.incremental, AppUpdateMode.needsBase]) {
        final backend = Backend()
          ..mode = mode
          ..network = Stream.value(AppUpdateNetwork.unmetered);
        final controller = AppUpdateController(backend: backend);
        await controller.check();
        await Future<void>.delayed(Duration.zero);
        expect(
          controller.plan!.mode,
          mode == AppUpdateMode.incremental ? mode : AppUpdateMode.full,
        );
        expect(controller.phase, AppUpdatePhase.ready);
        expect(backend.downloads, 1);
        expect(backend.installs, 0);
        controller.dispose();
      }
    },
  );
  test(
    'patch-only discovery waits for the first user-selected base without a prompt',
    () async {
      final backend = Backend()
        ..mode = AppUpdateMode.needsBase
        ..fullAvailable = false
        ..network = Stream.value(AppUpdateNetwork.unmetered);
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      await Future<void>.delayed(Duration.zero);
      expect(controller.phase, AppUpdatePhase.available);
      expect(backend.downloads, 0);
      await controller.start();
      expect(controller.phase, AppUpdatePhase.awaitingBase);
      controller.dispose();
    },
  );
  test(
    'leaving unmetered networking cancels prefetch and reconnecting starts a new attempt',
    () async {
      final network = StreamController<AppUpdateNetwork>.broadcast();
      final backend = Backend()
        ..network = network.stream
        ..downloadPending = Completer<String>();
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      network.add(AppUpdateNetwork.unmetered);
      await Future<void>.delayed(Duration.zero);
      expect(backend.downloads, 1);
      network.add(AppUpdateNetwork.offline);
      await Future<void>.delayed(Duration.zero);
      expect(controller.phase, AppUpdatePhase.available);
      expect(backend.installs, 0);
      backend.downloadPending = null;
      network.add(AppUpdateNetwork.unmetered);
      await Future<void>.delayed(Duration.zero);
      expect(backend.downloads, 2);
      expect(controller.phase, AppUpdatePhase.ready);
      controller.dispose();
      await network.close();
    },
  );
  testWidgets(
    'automatic source selection stays brief, resets switched progress, and can be cancelled',
    (tester) async {
      final backend = Backend()
        ..selectingSource = true
        ..downloadPending = Completer<String>();
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot:
              UserPreferencesManager.defaultThemePreferenceSnapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: AppUpdateDialog(controller: controller),
        ),
      );
      await tester.pump();
      expect(find.text('正在自动选择下载源…'), findsOneWidget);
      expect(controller.busy, isTrue);
      expect(find.text('取消更新'), findsOneWidget);
      backend.onProgress!(48, 120);
      await tester.pump();
      expect(controller.phase, AppUpdatePhase.downloading);
      backend.selectSource!();
      await tester.pump();
      expect(find.text('正在自动选择下载源…'), findsOneWidget);
      expect(controller.receivedBytes, 0);
      expect(controller.totalBytes, isNull);
      await tester.tap(find.text('取消更新'));
      await tester.pump();
      expect(controller.phase, AppUpdatePhase.available);
      expect(backend.installs, 0);
      await tester.pumpWidget(const SizedBox.shrink());
      controller.dispose();
    },
  );
  test(
    'installation retry reuses package and respects native permission/restart states',
    () async {
      final backend = Backend()..failInstall = true;
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      await controller.start();
      expect(controller.phase, AppUpdatePhase.ready);
      expect(backend.installs, 0);
      await controller.confirmInstall();
      expect(controller.phase, AppUpdatePhase.failed);
      expect(controller.packageReceipt, isNotNull);
      await controller.check();
      expect(controller.phase, AppUpdatePhase.failed);
      backend
        ..failInstall = false
        ..state = AppUpdateInstallState.permissionRequired;
      await controller.confirmInstall();
      expect(controller.phase, AppUpdatePhase.awaitingPermission);
      expect(backend.downloads, 1);
      backend.state = AppUpdateInstallState.restartRequired;
      await controller.confirmInstall();
      expect(controller.restartRequired, isTrue);
      expect(backend.downloads, 1);
      expect(backend.installs, 3);
      controller.dispose();
    },
  );
  test(
    'download cancellation and concurrent starts never launch installation',
    () async {
      final backend = Backend()..downloadPending = Completer<String>();
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      final pending = controller.start();
      await controller.start();
      expect(backend.downloads, 1);
      controller.cancelDownload();
      await pending;
      expect(controller.phase, AppUpdatePhase.available);
      expect(controller.packageReceipt, isNull);
      expect(backend.installs, 0);
      controller.dispose();
    },
  );
  test(
    'cancellation failure is visible and never auto-installs on completion',
    () async {
      final backend = Backend()
        ..downloadPending = Completer<String>()
        ..failCancel = true;
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      final pending = controller.start();
      await Future<void>.delayed(Duration.zero);
      await controller.cancelDownload();
      expect(controller.error, contains('取消下载失败'));
      expect(controller.phase, AppUpdatePhase.downloading);
      backend.downloadPending!.complete('fixture-update-receipt');
      await pending;
      expect(controller.phase, AppUpdatePhase.available);
      expect(backend.installs, 0);
      await controller.start();
      expect(backend.downloads, 1);
      expect(backend.installs, 0);
      await controller.confirmInstall();
      expect(backend.installs, 1);
      controller.dispose();
    },
  );
  Widget updateApp(
    AppUpdateController controller, {
    bool collapsed = false,
    double textScale = 1,
  }) => MaterialApp(
    builder: (context, child) => MediaQuery(
      data: MediaQuery.of(
        context,
      ).copyWith(textScaler: TextScaler.linear(textScale)),
      child: child!,
    ),
    home: AppUpdateScope(
      controller: controller,
      child: Scaffold(
        body: Wrap(
          children: [
            AppUpdateBadge(
              showIndicator: !collapsed,
              child: const Text('Operit'),
            ),
            const AppUpdateBadge(child: Text('Second sidebar')),
          ],
        ),
      ),
    ),
  );

  testWidgets(
    'discovery prompts once across sidebars and layout changes, and a later release prompts again',
    (tester) async {
      final backend = Backend()..checking = Completer<AppUpdateRelease?>();
      final controller = AppUpdateController(backend: backend);
      await tester.pumpWidget(updateApp(controller));
      await tester.pump();
      expect(find.byType(AppUpdateDialog), findsNothing);
      backend.checking!.complete(release);
      await tester.pumpAndSettle();
      expect(find.byType(AppUpdateDialog), findsOneWidget);
      expect(find.text('发现新版本'), findsOneWidget);
      expect(find.text(release.releaseNotes), findsOneWidget);
      expect(find.text('立即更新'), findsOneWidget);
      expect(backend.plans, 0);
      expect(backend.downloads, 0);
      expect(backend.installs, 0);
      await tester.tap(find.text('取消'));
      await tester.pumpAndSettle();
      backend.checking = null;
      await controller.check(force: true);
      await tester.pumpWidget(updateApp(controller, collapsed: true));
      await tester.pumpAndSettle();
      expect(find.byType(AppUpdateDialog), findsNothing);
      await tester.pumpWidget(const SizedBox());
      await tester.pumpWidget(updateApp(controller));
      await tester.pumpAndSettle();
      expect(find.byType(AppUpdateDialog), findsNothing);
      backend.availableRelease = AppUpdateRelease(
        currentVersion: release.currentVersion,
        version: '1.2.0',
        releasePageUrl: release.releasePageUrl,
      );
      await controller.check(force: true);
      await tester.pumpAndSettle();
      expect(find.byType(AppUpdateDialog), findsOneWidget);
      expect(find.text('1.2.0'), findsOneWidget);
      expect(find.text('本次更新内容'), findsNothing);
      await tester.tap(find.text('取消'));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('app-update-badge')).first);
      await tester.pumpAndSettle();
      expect(find.byType(AppUpdateDialog), findsOneWidget);
      expect(backend.downloads, 1);
      await tester.tap(find.text('关闭'));
      await tester.pumpAndSettle();
      await tester.pumpWidget(const SizedBox());
      controller.dispose();
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('accepting the reminder starts the existing update flow', (
    tester,
  ) async {
    final backend = Backend()..downloadPending = Completer<String>();
    final controller = AppUpdateController(backend: backend);
    await tester.pumpWidget(updateApp(controller));
    await tester.pumpAndSettle();
    expect(backend.downloads, 0);
    await tester.tap(find.text('立即更新'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    expect(backend.downloads, 1);
    expect(backend.installs, 0);
    expect(find.text('40%'), findsOneWidget);
    expect(find.byType(AppUpdateDialog), findsOneWidget);
    await tester.tap(find.text('取消更新'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('关闭'));
    await tester.pumpAndSettle();
    await tester.pumpWidget(const SizedBox());
    controller.dispose();
  });

  testWidgets(
    'a prepared update stays uninstalled until the reminder is accepted',
    (tester) async {
      final backend = Backend()
        ..network = Stream.value(AppUpdateNetwork.unmetered);
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      await tester.pump();
      expect(controller.phase, AppUpdatePhase.ready);
      await tester.pumpWidget(updateApp(controller, collapsed: true));
      await tester.pumpAndSettle();
      expect(find.text('重启并更新'), findsOneWidget);
      expect(backend.installs, 0);
      await tester.tap(find.text('重启并更新'));
      await tester.pumpAndSettle();
      expect(backend.downloads, 1);
      expect(backend.installs, 1);
      await tester.tap(find.text('关闭'));
      await tester.pumpAndSettle();
      await tester.pumpWidget(const SizedBox());
      controller.dispose();
    },
  );

  for (final textScale in [1.0, 1.6]) {
    testWidgets(
      'local progress fits a narrow screen with text scale $textScale',
      (tester) async {
        tester.view.physicalSize = const Size(360, 640);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final backend = Backend()..downloadPending = Completer<String>();
        final controller = AppUpdateController(backend: backend);
        await controller.check();
        controller.markUpdatePrompted();
        final pending = controller.start();
        await tester.pumpWidget(updateApp(controller, textScale: textScale));
        await tester.pump();
        await tester.tap(find.byKey(const ValueKey('app-update-badge')).first);
        await tester.pump();
        backend.onPreparationProgress!(
          const AppUpdatePreparationProgress(
            AppUpdatePreparationPhase.restoringPackage,
            75,
            100,
          ),
        );
        await tester.pump(const Duration(seconds: 2));
        expect(find.text('正在生成新安装包…'), findsOneWidget);
        expect(find.text('当前步骤 75%'), findsOneWidget);
        expect(find.textContaining('已用时 '), findsOneWidget);
        expect(find.text('后台更新').hitTestable(), findsOneWidget);
        expect(find.text('取消更新').hitTestable(), findsOneWidget);
        expect(tester.takeException(), isNull);
        await tester.tap(find.text('取消更新'));
        await pending;
        await tester.pumpAndSettle();
        await tester.tap(find.text('关闭'));
        await tester.pumpAndSettle();
        await tester.pumpWidget(const SizedBox());
        controller.dispose();
      },
    );
    testWidgets('reminder fits a narrow screen with text scale $textScale', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(360, 640);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final backend = Backend()
        ..availableRelease = AppUpdateRelease(
          currentVersion: '2.0.0-preview.35',
          version: '2.0.0-preview.36',
          releasePageUrl: release.releasePageUrl,
          releaseNotes: List.filled(50, '改进使用体验，修复已知问题。').join('\n'),
        );
      final controller = AppUpdateController(backend: backend);
      await tester.pumpWidget(updateApp(controller, textScale: textScale));
      await tester.pumpAndSettle();
      expect(find.text('立即更新').hitTestable(), findsOneWidget);
      expect(find.text('取消').hitTestable(), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.tap(find.text('取消'));
      await tester.pumpAndSettle();
      await tester.pumpWidget(const SizedBox());
      controller.dispose();
    });
  }

  testWidgets(
    'an update check completing after the sidebar is disposed does not prompt',
    (tester) async {
      final backend = Backend()..checking = Completer<AppUpdateRelease?>();
      final controller = AppUpdateController(backend: backend);
      await tester.pumpWidget(updateApp(controller));
      await tester.pump();
      await tester.pumpWidget(const SizedBox());
      backend.checking!.complete(release);
      await tester.pumpAndSettle();
      expect(find.byType(AppUpdateDialog), findsNothing);
      expect(controller.hasUnpromptedUpdate, isTrue);
      controller.dispose();
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'badge opens dialog before download, shows progress and resumes background task',
    (tester) async {
      final backend = Backend()..downloadPending = Completer<String>();
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      tester.view.physicalSize = const Size(1120, 760);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      await tester.runAsync(() async {
        final icons = File(
          '/Users/ylvira/fvm/default/bin/cache/artifacts/material_fonts/MaterialIcons-Regular.otf',
        );
        if (icons.existsSync()) {
          final loader = FontLoader('MaterialIcons')
            ..addFont(
              Future.value(ByteData.sublistView(await icons.readAsBytes())),
            );
          await loader.load();
        }
      });
      const output = String.fromEnvironment('UPDATE_SCREENSHOT_DIR');
      if (output.isNotEmpty) {
        final font = File('/System/Library/Fonts/Hiragino Sans GB.ttc');
        if (font.existsSync()) {
          await tester.runAsync(() async {
            final loader = FontLoader('UpdateChinese')
              ..addFont(
                Future.value(ByteData.sublistView(await font.readAsBytes())),
              );
            await loader.load();
          });
        }
      }
      final boundaryKey = GlobalKey();
      await tester.pumpWidget(
        RepaintBoundary(
          key: boundaryKey,
          child: AppUpdateScope(
            controller: controller,
            child: OperitTheme(
              initialThemePreferenceSnapshot:
                  UserPreferencesManager.defaultThemePreferenceSnapshot,
              initialThemeIsReady: false,
              unconfiguredChildEnabled: true,
              hostInteractionHostsEnabled: false,
              child: Builder(
                builder: (context) => Theme(
                  data: Theme.of(context).copyWith(
                    textTheme: Theme.of(context).textTheme.apply(
                      fontFamily: output.isEmpty ? null : 'UpdateChinese',
                    ),
                    dialogTheme: Theme.of(context).dialogTheme.copyWith(
                      titleTextStyle: Theme.of(context)
                          .dialogTheme
                          .titleTextStyle
                          ?.copyWith(
                            fontFamily: output.isEmpty ? null : 'UpdateChinese',
                          ),
                    ),
                  ),
                  child: Scaffold(
                    body: Row(
                      children: [
                        SizedBox(
                          width: 72,
                          child: CollapsedDrawerContent(
                            navigationEntries:
                                ScreenRouteRegistry.mainSidebarEntries(
                                  AppLocalizations.of(context)!,
                                ),
                            pluginEntries: const [],
                            selectedRouteId: ScreenRouteRegistry.routeIdOf(
                              ScreenRouteRegistry.aiChat,
                            ),
                            appearance: const NavigationDrawerAppearance(
                              containerColor: Colors.white,
                              titleColor: Colors.black,
                              statusAvailableColor: Colors.blue,
                              itemColor: Colors.black87,
                              buttonContainerColor: Colors.white70,
                              selectedContainerColor: Colors.blue,
                              selectedContentColor: Colors.white,
                              dividerColor: Colors.black12,
                              transparentSurfaceEnabled: false,
                            ),
                            histories: const [],
                            currentChatId: null,
                            bridge: NativeSidebarBridge(),
                            onNavigationEntrySelected: (_) {},
                            onConversationNavigationPressed: () {},
                            onConversationActivated: () {},
                          ),
                        ),
                        SizedBox(
                          width: 296,
                          child: DrawerContent(
                            conversationsOnly: true,
                            navigationEntries: const [],
                            pluginEntries: const [],
                            selectedRouteId: ScreenRouteRegistry.routeIdOf(
                              ScreenRouteRegistry.aiChat,
                            ),
                            appearance: const NavigationDrawerAppearance(
                              containerColor: Colors.white,
                              titleColor: Colors.black,
                              statusAvailableColor: Colors.blue,
                              itemColor: Colors.black87,
                              buttonContainerColor: Colors.white70,
                              selectedContainerColor: Colors.blue,
                              selectedContentColor: Colors.white,
                              dividerColor: Colors.black12,
                              transparentSurfaceEnabled: false,
                            ),
                            histories: const [],
                            activeStreamingChatIds: const {},
                            currentChatId: null,
                            errorMessage: null,
                            loading: false,
                            bridge: NativeSidebarBridge(),
                            onNavigationEntrySelected: (_) {},
                            onConversationActivated: () {},
                          ),
                        ),
                        const VerticalDivider(width: 1),
                        const Expanded(
                          child: Center(
                            child: Column(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                Text(
                                  'Operit',
                                  style: TextStyle(
                                    fontSize: 36,
                                    fontWeight: FontWeight.bold,
                                  ),
                                ),
                                SizedBox(height: 12),
                                Text(
                                  '选择对话，开始新的探索',
                                  style: TextStyle(
                                    fontSize: 16,
                                    color: Colors.grey,
                                  ),
                                ),
                                SizedBox(height: 32),
                                Text(
                                  '更新交互预览 · 示例版本与下载进度',
                                  style: TextStyle(
                                    fontSize: 12,
                                    color: Colors.grey,
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.byKey(const ValueKey('app-update-badge')), findsOneWidget);
      final sidebar = find.byType(CollapsedDrawerContent);
      final logo = find.descendant(
        of: sidebar,
        matching: find.byType(OperitLogoMark),
      );
      expect(tester.getCenter(logo).dx, tester.getCenter(sidebar).dx);
      expect(
        find.descendant(
          of: sidebar,
          matching: find.byKey(const ValueKey('app-update-badge')),
        ),
        findsNothing,
      );
      expect(backend.downloads, 0);
      Future<void> capture(String name) async {
        if (output.isEmpty) return;
        final boundary =
            boundaryKey.currentContext!.findRenderObject()!
                as RenderRepaintBoundary;
        await tester.runAsync(() async {
          final image = await boundary.toImage(pixelRatio: 1.5);
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          await Directory(output).create(recursive: true);
          await File(
            '$output/$name.png',
          ).writeAsBytes(bytes!.buffer.asUint8List());
          image.dispose();
        });
      }

      expect(find.byType(AppUpdateDialog), findsOneWidget);
      expect(find.text('立即更新'), findsOneWidget);
      await capture('update-available-dialog');
      await tester.tap(find.text('取消'));
      await tester.pumpAndSettle();
      expect(backend.downloads, 0);
      await capture('update-badge');
      await tester.tap(find.byKey(const ValueKey('app-update-badge')));
      expect(backend.downloads, 0);
      await tester.pump();
      expect(find.byType(AppUpdateDialog), findsOneWidget);
      await tester.pump(const Duration(milliseconds: 300));
      expect(backend.downloads, 1);
      expect(find.text('40%'), findsOneWidget);
      await capture('update-download-dialog');
      await tester.tap(find.text('后台更新'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      await tester.tap(find.byKey(const ValueKey('app-update-badge')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(backend.downloads, 1);
      await tester.tap(find.text('取消更新'));
      await tester.pump();
      expect(controller.phase, AppUpdatePhase.available);
      expect(backend.installs, 0);
      await tester.tap(find.text('关闭'));
      await tester.pump();
      await tester.pumpWidget(const SizedBox());
      controller.dispose();
      expect(tester.takeException(), isNull);
    },
  );
}
