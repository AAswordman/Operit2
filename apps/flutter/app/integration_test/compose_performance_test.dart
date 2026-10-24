import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' show FramePhase, FrameTiming;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:path_provider/path_provider.dart';
import 'package:operit2/ui/features/packages/screens/ComposeDslImageSource.dart';
import 'package:operit2/core/bridge/PlatformCoreProxy.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/host/ComposeWebViewControllerBridge.dart';
import 'package:operit2/core/host/RuntimeHostInteractionSubscriber.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/common/contributions/ChatSidebarTabHost.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';

/// Retains existing encoded buffers only in diagnostic runs. Disk writes happen
/// after all timed samples; capture never decodes, re-encodes or changes a frame.
class _PayloadCaptureBridge extends OperitRuntimeBridge {
  _PayloadCaptureBridge(this.inner);
  final OperitRuntimeBridge inner;
  String? workload;
  final payloads = <String, Uint8List>{};

  void retain(String channel, Uint8List? bytes) {
    final name = workload;
    if (name == null || bytes == null) return;
    final key = '$name-$channel';
    if (bytes.length > (payloads[key]?.length ?? 0)) payloads[key] = bytes;
  }

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    final bytes = await inner.callBytes(request);
    retain('call', bytes);
    return bytes;
  }

  @override
  Future<Uint8List> callControlBytes(CoreCallRequest request) =>
      inner.callControlBytes(request);

  @override
  Future<CorePushSink> push(CorePushRequest request) => inner.push(request);

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      inner.watchSnapshot(request);

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      inner.watchStream(request).map((event) {
        retain('watch', event.valueBytes);
        return event;
      });

  Future<List<Map<String, Object>>> writeFiles(String output) async {
    final directory = Directory('$output/wire-payloads');
    await directory.create(recursive: true);
    final reports = <Map<String, Object>>[];
    for (final entry in payloads.entries) {
      final file = File('${directory.path}/${entry.key}.msgpack');
      await file.writeAsBytes(entry.value);
      reports.add({
        'name': entry.key,
        'bytes': entry.value.length,
        'path': file.path,
      });
    }
    return reports;
  }
}

/// Waits for production lifecycle events while the native frame clock runs.
/// Repeated pump()/pumpAndSettle() would change the frame wait being measured.
Future<String> waitForLifecycle(WidgetTester tester) async {
  final watch = Stopwatch()..start();
  while (watch.elapsed < const Duration(seconds: 30)) {
    final error = tester.takeException();
    if (error != null) throw error;
    final log = await ClientLogger.readText();
    if (log.contains('event=compose_first_tree_frame ') &&
        log.contains('event=compose_onload_frame ')) {
      return log;
    }
    await Future<void>.delayed(const Duration(milliseconds: 20));
  }
  throw TimeoutException(
    'Real page did not finish first tree/onLoad frames.\n'
    '${await ClientLogger.readText()}',
  );
}

/// Reads actual business DOM through the existing native controller. The page still
/// obtains all business data through its normal JS interface/IPC/toolCall chain.
Future<Map<String, Object?>> waitForWebContent(
  WidgetTester tester,
  String log,
  int rows,
) async {
  final context = RegExp(
    r'event=compose_first_tree_frame .*?context=([^\s]+)',
  ).firstMatch(log)?.group(1);
  expect(context, isNotNull);
  final watch = Stopwatch()..start();
  Object? lastResult;
  while (watch.elapsed < const Duration(seconds: 30)) {
    final error = tester.takeException();
    if (error != null) throw error;
    lastResult = jsonDecode(
      await ComposeDslWebViewHostRegistry.handleControllerCommand(
        jsonEncode({
          'executionContextKey': context,
          'key': 'character-memory-web',
          'command': 'evaluateJavascript',
          'payload': {
            'script':
                "(() => { if (!(document.readyState === 'complete' && "
                "document.querySelectorAll('[data-action=\"edit-card\"]').length === $rows && "
                "document.body.innerText.includes('Benchmark character 0') && "
                "performance.getEntriesByName('character-memory.content-ready').length === 1)) return false; "
                "return { timeOrigin: performance.timeOrigin, observedAt: performance.now(), "
                "navigation: performance.getEntriesByType('navigation').map(e => e.toJSON()), "
                "entries: performance.getEntries().filter(e => e.name.startsWith('character-memory.'))"
                ".map(e => ({ name: e.name, entryType: e.entryType, startTime: e.startTime, duration: e.duration })) }; })()",
          },
        }),
      ),
    );
    if (lastResult is Map &&
        lastResult['success'] == true &&
        lastResult['data'] is Map) {
      return Map<String, Object?>.from(lastResult['data'] as Map);
    }
    await Future<void>.delayed(const Duration(milliseconds: 50));
  }
  throw TimeoutException(
    'WebView did not render real seeded cards: $lastResult',
  );
}

/// Bound test-driven mount/unmount waits as well as production lifecycle waits.
Future<void> mountSurface(WidgetTester tester, Widget widget) => tester
    .pumpWidget(widget)
    .timeout(
      const Duration(seconds: 30),
      onTimeout: () {
        throw TimeoutException(
          'Native Flutter frame did not complete during page switching',
        );
      },
    );

void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  binding.framePolicy = LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;
  const samples = int.fromEnvironment(
    'OPERIT_PERFORMANCE_SAMPLES',
    defaultValue: 8,
  );
  const rows = int.fromEnvironment('OPERIT_PERFORMANCE_ROWS', defaultValue: 20);
  const package = 'com.operit.character_cards';

  testWidgets(
    'real native sidebar and main page performance',
    (tester) async {
      expect(
        Platform.isMacOS,
        isTrue,
        reason: 'Requires production macOS host',
      );
      expect(samples, greaterThanOrEqualTo(2));
      expect(rows, greaterThan(0));
      final reports = <Map<String, Object?>>[];
      final startup = Stopwatch()..start();
      final support = await getApplicationSupportDirectory();
      final benchmarkRoot = await Directory(
        '${support.path}/compose-performance',
      ).create(recursive: true);
      final directory = await benchmarkRoot.createTemp('run-');
      binding.reportData = <String, dynamic>{
        'scope':
            'Production macOS platform host, CoreLink, toolCall, plugin storage, '
            'ChatSidebarTabHost and ToolPkgUiLauncherScreen. Independent runtime roots; '
            'normal platform secrets. Memory client logger; full Operit shell omitted. '
            'WebView readiness separately verifies all seeded cards in real DOM (50ms polling). '
            'First tree/onLoad frames do not imply GPU presentation.',
        'traceStages': const bool.fromEnvironment(
          'OPERIT_PROFILE_COMPOSE_STAGES',
        ),
        'mode': kProfileMode ? 'profile' : (kReleaseMode ? 'release' : 'debug'),
        'runtimeRoot': directory.path,
        'rows': rows,
        'repetitions': samples,
        'samples': reports,
      };
      // This precedes every Core request; production host roots are immutable afterwards.
      await platformCoreProxy.setRuntimeStorageRoots(
        '${directory.path}/runtime',
        '${directory.path}/workspace',
      );
      await ClientLogger.initialize();
      await LiquidGlassWidgets.initialize();
      final unregisterWebView = const ComposeWebViewControllerBridge()
          .registerHandler(
            ComposeDslWebViewHostRegistry.handleControllerCommand,
          );
      addTearDown(unregisterWebView);
      RuntimeHostInteractionSubscriber.install();
      addTearDown(RuntimeHostInteractionSubscriber.uninstall);
      final capture =
          const bool.fromEnvironment('OPERIT_PROFILE_COMPOSE_STAGES')
          ? _PayloadCaptureBridge(const ProxyCoreRuntimeBridge())
          : null;
      final clients = GeneratedCoreProxyClients(
        capture ?? const ProxyCoreRuntimeBridge(),
      );
      final manager = clients.application.packageManager();
      await manager.getToolPkgPublicApiOwners(
        apiName: ChatSidebarPreparation.api,
      );
      final plugin = await manager.getToolPkgContainerRuntime(
        containerPackageName: package,
      );
      expect(plugin, isNotNull);
      final entries = await manager.getToolPkgNavigationEntries(
        useEnglish: false,
      );
      final sidebar = entries.firstWhere(
        (entry) =>
            entry.containerPackageName == package &&
            entry.surface == 'chat_sidebar_tabs',
      );
      final routes = await manager.getToolPkgUiRoutes(
        runtime: 'compose_dsl',
        useEnglish: false,
      );
      final mainRoute = routes.firstWhere(
        (route) =>
            route.containerPackageName == package &&
            route.routeId == 'toolpkg:$package:ui:main',
      );
      binding.reportData!['bootstrapMs'] = startup.elapsedMicroseconds / 1000;

      // Use actual public APIs, chat creation hooks and durable stores. Seeding is untimed.
      for (var i = 0; i < rows; i++) {
        final card = await manager.invokeToolPkgPublicApi(
          packageName: package,
          methodName: 'character.create',
          payload: {
            'values': {'name': 'Benchmark character $i'},
          },
        );
        await clients.chatRuntimeHolderMain.createNewChat(
          setAsCurrentChat: false,
          sourceChatId: null,
          parentChatId: null,
          input: {
            package: {'version': 1, 'selection': 'card:${(card as Map)['id']}'},
          },
        );
      }
      final cards = await manager.invokeToolPkgPublicApi(
        packageName: package,
        methodName: 'character.list',
        payload: const {},
      );
      expect(cards, isA<List>());
      final expectedCardCount = (cards as List).length;
      binding.reportData!['totalCards'] = expectedCardCount;
      final chats = await clients.chatRuntimeHolderMain
          .chatHistoryListItemsFlow()
          .first;
      expect(chats, hasLength(rows));
      final id = chatSidebarTabIdentity(package, sidebar.entryId);
      await UserPreferencesManager(clients: clients).saveChatSidebarTab(id);
      final preparation = ChatSidebarPreparation(clients: clients);
      addTearDown(preparation.dispose);
      final prepareWatch = Stopwatch()..start();
      await preparation.prepare(chats);
      expect(
        preparation.read(id, chats),
        isNotNull,
        reason: 'Do not silently benchmark fallback',
      );
      binding.reportData!['preparationMs'] =
          prepareWatch.elapsedMicroseconds / 1000;

      Widget sidebarWidget(bool prepared) {
        final host = SizedBox(
          width: 360,
          child: ChatSidebarTabHost(
            clients: clients,
            chats: chats,
            currentChatId: chats.first.id,
            activeStreamingChatIds: const {},
            workspaceBuilder: (_) => const Text('Workspace'),
            onActivateChat: (_) async {},
          ),
        );
        return prepared
            ? ChatSidebarPreparationScope(preparation: preparation, child: host)
            : host;
      }

      final frames = <FrameTiming>[];
      void recordFrames(List<FrameTiming> value) => frames.addAll(value);
      binding.addTimingsCallback(recordFrames);
      try {
        await binding.watchPerformance(() async {
          // Alternate paths each repetition so changes in clock/power affect all paths.
          for (var index = 0; index < samples; index++) {
            final workloads = index.isEven
                ? ['sidebar-unprepared', 'sidebar-prepared', 'main']
                : ['main', 'sidebar-prepared', 'sidebar-unprepared'];
            for (final workload in workloads) {
              capture?.workload = workload;
              binding.reportData!['pendingWorkload'] = {
                'workload': workload,
                'iteration': index,
                'stage': 'unmount',
              };
              await mountSurface(tester, const SizedBox.shrink());
              await Future<void>.delayed(const Duration(milliseconds: 150));
              await ClientLogger.clear();
              final startedWallMicros = DateTime.now().microsecondsSinceEpoch;
              final watch = Stopwatch()..start();
              binding.reportData!['pendingWorkload'] = {
                'workload': workload,
                'iteration': index,
                'stage': 'mount',
              };
              final child = workload == 'main'
                  ? ToolPkgUiLauncherScreen(
                      clients: clients,
                      plugin: plugin!,
                      initialRouteId: mainRoute.routeId,
                      embeddedScreenPath: mainRoute.screen,
                      initialModuleSpec: mainRoute.moduleSpec,
                      showLauncherChrome: false,
                    )
                  : sidebarWidget(workload == 'sidebar-prepared');
              await mountSurface(
                tester,
                MaterialApp(
                  locale: const Locale('zh'),
                  home: Scaffold(
                    body: Align(alignment: Alignment.topLeft, child: child),
                  ),
                ),
              );
              binding.reportData!['pendingWorkload'] = {
                'workload': workload,
                'iteration': index,
                'stage': 'lifecycle',
              };
              final log = await waitForLifecycle(tester);
              final observedMs = watch.elapsedMicroseconds / 1000;
              expect(
                find.byKey(const ValueKey('chat-sidebar-tab-error')),
                findsNothing,
              );
              // Prove actual seeded business content was rendered, not just a spinner.
              Map<String, Object?>? webTiming;
              List<String>? renderedRoleLabels;
              if (workload == 'main') {
                webTiming = await waitForWebContent(
                  tester,
                  log,
                  expectedCardCount,
                );
              } else {
                // Lazy rows follow the live sidebar order; card 0 can be outside
                // the viewport. Require real seeded roles in the visible tree.
                renderedRoleLabels = tester
                    .widgetList<Text>(
                      find.textContaining(RegExp(r'^Benchmark character \d+$')),
                    )
                    .map((text) => text.data ?? text.textSpan!.toPlainText())
                    .toSet()
                    .toList();
                expect(renderedRoleLabels, isNotEmpty);
              }
              final images = tester
                  .widgetList<Image>(find.byType(Image))
                  .toList();
              final memoryImages = images
                  .map((image) => image.image)
                  .whereType<MemoryImage>()
                  .toSet();
              final vfsImages = images
                  .map((image) => image.image)
                  .whereType<ComposeDslVfsImage>()
                  .toSet();
              final contentReadyMs = watch.elapsedMicroseconds / 1000;
              final finishedWallMicros = DateTime.now().microsecondsSinceEpoch;
              reports.add({
                'workload': workload,
                'iteration': index,
                'firstObserved': index == 0,
                'hostThroughOnLoadObservedMs': observedMs,
                'businessContentReadyObservedMs': contentReadyMs,
                'startedWallMicros': startedWallMicros,
                'finishedWallMicros': finishedWallMicros,
                'log': log,
                'webTiming': ?webTiming,
                'renderedRoleLabels': ?renderedRoleLabels,
                'imageWidgets': images.length,
                'uniqueMemoryImages': memoryImages.length,
                'uniqueVfsImages': vfsImages.length,
                'memoryImageBytes': memoryImages.fold<int>(
                  0,
                  (sum, image) => sum + image.bytes.length,
                ),
              });
            }
          }
          // Timings may arrive once per second. Keep one observer for the entire run,
          // then assign frames by native raster wall time, not callback delivery time.
          await Future<void>.delayed(const Duration(seconds: 2));
        }, reportKey: 'engineFrameSummary');
      } finally {
        binding.removeTimingsCallback(recordFrames);
      }
      for (final report in reports) {
        report['frames'] = [
          for (final frame in frames)
            if (frame.timestampInMicroseconds(
                      FramePhase.rasterFinishWallTime,
                    ) >=
                    (report['startedWallMicros'] as int) &&
                frame.timestampInMicroseconds(
                      FramePhase.rasterFinishWallTime,
                    ) <=
                    (report['finishedWallMicros'] as int))
              {
                'buildMs': frame.buildDuration.inMicroseconds / 1000,
                'rasterMs': frame.rasterDuration.inMicroseconds / 1000,
                'totalSpanMs': frame.totalSpan.inMicroseconds / 1000,
              },
        ];
        expect(
          report['frames'],
          isNotEmpty,
          reason: 'Native frame timings required',
        );
      }
      if (capture != null) {
        capture.workload = null;
        binding.reportData!['wirePayloads'] = await capture.writeFiles(
          directory.path,
        );
      }
      binding.reportData!.remove('pendingWorkload');
      final view = tester.view;
      binding.reportData!['view'] = {
        'width': view.physicalSize.width / view.devicePixelRatio,
        'height': view.physicalSize.height / view.devicePixelRatio,
        'devicePixelRatio': view.devicePixelRatio,
      };
      await mountSurface(tester, const SizedBox.shrink());
      // Native owns open stores until process exit. Temporary roots are retained in the report for inspection.
    },
    timeout: const Timeout(Duration(minutes: 8)),
  );
}
