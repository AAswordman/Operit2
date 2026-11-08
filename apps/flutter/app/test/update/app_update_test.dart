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
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/features/update/AppUpdateBadge.dart';
import 'package:operit2/ui/features/update/AppUpdateDialog.dart';
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
  int downloads = 0, installs = 0;
  bool failInstall = false;
  bool failCancel = false;
  AppUpdateInstallState state = AppUpdateInstallState.installerOpened;
  Completer<String>? downloadPending;
  AppUpdateProgress? onProgress;
  @override
  Future<AppUpdateRelease?> check() async => release;
  @override
  Future<String> download(
    AppUpdateRelease r,
    AppUpdateProgress progress,
  ) async {
    downloads++;
    onProgress = progress;
    progress(48 * 1024 * 1024, 120 * 1024 * 1024);
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
  Future<void> restartToInstall() async {}
}

void main() {
  setUpAll(ClientLogger.initialize);
  test(
    'installation retry reuses package and respects native permission/restart states',
    () async {
      final backend = Backend()..failInstall = true;
      final controller = AppUpdateController(backend: backend);
      await controller.check();
      await controller.start();
      expect(controller.phase, AppUpdatePhase.failed);
      expect(controller.packageReceipt, isNotNull);
      await controller.check();
      expect(controller.phase, AppUpdatePhase.failed);
      backend
        ..failInstall = false
        ..state = AppUpdateInstallState.permissionRequired;
      await controller.start();
      expect(controller.phase, AppUpdatePhase.awaitingPermission);
      expect(backend.downloads, 1);
      backend.state = AppUpdateInstallState.restartRequired;
      await controller.start();
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
      await controller.cancelDownload();
      expect(controller.error, contains('取消下载失败'));
      expect(controller.phase, AppUpdatePhase.downloading);
      backend.downloadPending!.complete('fixture-update-receipt');
      await pending;
      expect(controller.phase, AppUpdatePhase.available);
      expect(backend.installs, 0);
      await controller.start();
      expect(backend.downloads, 1);
      expect(backend.installs, 1);
      controller.dispose();
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

      await capture('update-badge');
      await tester.tap(find.byKey(const ValueKey('app-update-badge')));
      expect(backend.downloads, 0);
      await tester.pump();
      expect(find.byType(AppUpdateDialog), findsOneWidget);
      await tester.pump(const Duration(milliseconds: 300));
      expect(backend.downloads, 1);
      expect(find.text('40%'), findsOneWidget);
      await capture('update-download-dialog');
      await tester.tap(find.text('后台下载'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      await tester.tap(find.byKey(const ValueKey('app-update-badge')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(backend.downloads, 1);
      await tester.tap(find.text('取消下载'));
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
