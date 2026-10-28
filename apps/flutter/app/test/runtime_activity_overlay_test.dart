import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/startup/RuntimeActivityOverlay.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

class _Bridge extends OperitRuntimeBridge {
  final events = StreamController<CoreEvent>.broadcast();
  final calls = <String>[];
  final watches = <String>[];
  int spaceFailures = 0;

  void emit(String property, Object? value) => events.add(
    CoreEvent.raw(
      requestId: property,
      target: property == 'pluginLoadingProgressFlow'
          ? 'core/application'
          : 'core/server.runtimeRemoteLinkService',
      propertyName: property,
      kind: 'Snapshot',
      valueBytes: encodeCoreLink(value),
      decodeValue: decodeCoreLink<Object?>,
    ),
  );

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    watches.add(request.propertyName);
    if (request.propertyName == 'deviceSpaceSnapshotFlow' &&
        spaceFailures > 0) {
      spaceFailures--;
      return Stream<CoreEvent>.error(
        StateError('Device space connection record is empty'),
        StackTrace.empty,
      );
    }
    return events.stream.where((e) => e.propertyName == request.propertyName);
  }

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request.methodName);
    return encodeCoreLink([0, null]);
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();
}

core.SpaceSyncProgress progress({
  core.SpaceSyncStatus status = core.SpaceSyncStatus.syncing,
  core.SpaceSyncStage stage = core.SpaceSyncStage.records,
  int fileBytes = 0,
  int fileSize = 0,
}) {
  final now = DateTime.now().millisecondsSinceEpoch;
  return core.SpaceSyncProgress(
    spaceId: 'space',
    peerNodeId: 'tablet',
    startedAt: now - 10000,
    updatedAt: now,
    status: status,
    stage: stage,
    exchangedRecords: 1200,
    transferredBytes: 2097152,
    fileBytes: fileBytes,
    fileSize: fileSize,
    retryAt: status == core.SpaceSyncStatus.waiting ? now + 2000 : null,
    error: status == core.SpaceSyncStatus.waiting ? 'offline' : null,
  );
}

void main() {
  setUpAll(ClientLogger.initialize);
  late _Bridge bridge;
  Future<void> mount(WidgetTester tester, {int spaceFailures = 0}) async {
    bridge = _Bridge()..spaceFailures = spaceFailures;
    addTearDown(() async {
      await tester.pumpWidget(const SizedBox.shrink());
      await bridge.events.close();
    });
    tester.binding.platformDispatcher.localesTestValue = const [Locale('zh')];
    addTearDown(tester.binding.platformDispatcher.clearLocalesTestValue);
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: Stack(
          children: [
            const Scaffold(body: Center(child: Text('任意页面'))),
            RuntimeActivityOverlay(
              enabled: true,
              clients: GeneratedCoreProxyClients(bridge),
            ),
          ],
        ),
      ),
    );
    await tester.pump();
    await tester.pump();
  }

  Future<void> emitSync(
    WidgetTester tester,
    core.SpaceSyncProgress item,
  ) async {
    bridge.emit('spaceSyncProgressFlow', [item.toJson()]);
    await tester.pump();
    await tester.pump();
  }

  Future<void> emitPackages(WidgetTester tester) async {
    bridge.emit(
      'pluginLoadingProgressFlow',
      const core.PluginLoadingProgress(
        visible: true,
        forceExpanded: false,
        progress: 0.4,
        phase: 'loading',
        currentTask: '工具包',
        pluginsStarted: 2,
        pluginsTotal: 5,
        plugins: [],
      ).toJson(),
    );
    await tester.pump();
    await tester.pump();
  }

  testWidgets(
    'failed space watch retries without becoming an unhandled error',
    (tester) async {
      await mount(tester, spaceFailures: 1);
      expect(tester.takeException(), isNull);
      expect(
        bridge.watches.where((name) => name == 'deviceSpaceSnapshotFlow'),
        hasLength(1),
      );
      await tester.pump(const Duration(seconds: 1));
      await tester.pump();
      expect(
        bridge.watches.where((name) => name == 'deviceSpaceSnapshotFlow'),
        hasLength(2),
      );
      await emitSync(tester, progress());
      bridge.emit('deviceSpaceSnapshotFlow', {
        'space': {
          'spaceId': 'new-space',
          'spaceName': '新空间',
          'spaceRevision': 1,
          'members': ['self'],
        },
        'topology': {
          'currentDeviceId': 'self',
          'devices': [],
          'connections': [],
        },
      });
      await tester.pump();
      await tester.pump();
      expect(find.text('正在同步'), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'activity stream errors are logged and other progress stays usable',
    (tester) async {
      await mount(tester);
      bridge.events.addError(
        StateError('watch delivery failed'),
        StackTrace.current,
      );
      await tester.pump();
      await tester.pump();
      expect(tester.takeException(), isNull);
      await emitPackages(tester);
      expect(find.text('加载扩展'), findsOneWidget);
      await tester.pump(const Duration(seconds: 1));
      await tester.pump();
      expect(
        bridge.watches.where((name) => name == 'deviceSpaceSnapshotFlow'),
        hasLength(2),
      );
    },
  );

  testWidgets('disposing cancels pending space watch retries', (tester) async {
    await mount(tester, spaceFailures: 1);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump(const Duration(seconds: 30));
    expect(
      bridge.watches.where((name) => name == 'deviceSpaceSnapshotFlow'),
      hasLength(1),
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('idle and small sync snapshots stay silent', (tester) async {
    await mount(tester);
    bridge.emit('spaceSyncProgressFlow', []);
    await tester.pump();
    await tester.pump();
    expect(find.byType(LinearProgressIndicator), findsNothing);
    expect(bridge.watches.toSet(), {
      'pluginLoadingProgressFlow',
      'spaceSyncProgressFlow',
      'deviceSpaceSnapshotFlow',
    });
    expect(find.text('任意页面'), findsOneWidget);
  });

  testWidgets('file bytes drive progress; record exchange is indeterminate', (
    tester,
  ) async {
    await mount(tester);
    await emitSync(tester, progress());
    expect(find.text('正在同步'), findsOneWidget);
    expect(
      tester
          .widget<LinearProgressIndicator>(find.byType(LinearProgressIndicator))
          .value,
      isNull,
    );
    await emitSync(
      tester,
      progress(
        stage: core.SpaceSyncStage.files,
        fileBytes: 1048576,
        fileSize: 4194304,
      ),
    );
    expect(
      tester
          .widget<LinearProgressIndicator>(find.byType(LinearProgressIndicator))
          .value,
      0.25,
    );
    expect(find.text('1.0 MB / 4.0 MB'), findsOneWidget);
  });

  testWidgets(
    'waiting pauses animation and completion dismisses after three seconds',
    (tester) async {
      await mount(tester);
      await emitSync(tester, progress(status: core.SpaceSyncStatus.waiting));
      expect(find.text('等待重试'), findsOneWidget);
      expect(find.byType(LinearProgressIndicator), findsNothing);
      await tester.pump(const Duration(seconds: 30));
      expect(find.text('等待重试'), findsOneWidget);
      await emitSync(tester, progress(status: core.SpaceSyncStatus.completed));
      expect(find.text('同步完成'), findsOneWidget);
      await tester.pump(const Duration(seconds: 3));
      expect(find.text('同步完成'), findsNothing);
    },
  );

  testWidgets(
    'packages and sync share one surface; skip only affects packages',
    (tester) async {
      await mount(tester);
      await emitPackages(tester);
      expect(find.text('加载扩展'), findsOneWidget);
      await emitSync(tester, progress());
      expect(find.text('+1'), findsOneWidget);
      expect(find.byType(LinearProgressIndicator), findsOneWidget);
      await tester.tap(find.text('正在同步'));
      await tester.pump();
      await tester.pump();
      expect(find.text('后台任务'), findsOneWidget);
      expect(find.text('加载扩展'), findsWidgets);
      expect(find.byType(LinearProgressIndicator), findsNWidgets(2));
      await tester.tap(find.text('跳过'));
      await tester.pump();
      await tester.pump();
      expect(bridge.calls, contains('skipPluginLoading'));
      bridge.emit(
        'pluginLoadingProgressFlow',
        const core.PluginLoadingProgress(
          visible: false,
          forceExpanded: false,
          progress: 1,
          phase: 'completed',
          currentTask: '',
          pluginsStarted: 5,
          pluginsTotal: 5,
          plugins: [],
        ).toJson(),
      );
      await tester.pump();
      await tester.pump();
      expect(find.byType(LinearProgressIndicator), findsOneWidget);
      expect(find.text('后台任务'), findsOneWidget);
    },
  );

  testWidgets('switching space removes progress for the previous membership', (
    tester,
  ) async {
    await mount(tester);
    await emitSync(tester, progress());
    expect(find.text('正在同步'), findsOneWidget);
    bridge.emit('deviceSpaceSnapshotFlow', {
      'space': {
        'spaceId': 'new-space',
        'spaceName': '新空间',
        'spaceRevision': 1,
        'members': ['self'],
      },
      'topology': {'currentDeviceId': 'self', 'devices': [], 'connections': []},
    });
    await tester.pump();
    await tester.pump();
    expect(find.text('正在同步'), findsNothing);
    expect(find.byType(LinearProgressIndicator), findsNothing);
  });

  testWidgets(
    'compact surface fits narrow screens and enlarged text; can drag and expand',
    (tester) async {
      tester.view.physicalSize = const Size(320, 640);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      tester.platformDispatcher.textScaleFactorTestValue = 1.6;
      addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
      await mount(tester);
      await emitPackages(tester);
      await tester.drag(find.text('加载扩展'), const Offset(-80, 120));
      await tester.pump();
      await tester.pump();
      expect(tester.takeException(), isNull);
      await tester.tap(find.text('加载扩展'));
      await tester.pump();
      await tester.pump();
      expect(find.text('后台任务'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );
}
