import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/workspace/ChatContextStatsTopBarButton.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';

void main() {
  for (final direction in TextDirection.values) {
    for (final textScale in <double>[1, 2]) {
      for (final owner in <String>['local', 'offline']) {
        testWidgets(
          'device rows stay compact at $textScale / $direction / owner=$owner',
          (tester) async {
            tester.view.physicalSize = const Size(320, 800);
            tester.view.devicePixelRatio = 1;
            addTearDown(tester.view.resetPhysicalSize);
            addTearDown(tester.view.resetDevicePixelRatio);
            await tester.pumpWidget(
              _app(
                _DeviceStatsBridge(owner),
                direction: direction,
                textScale: textScale,
              ),
            );
            await tester.pumpAndSettle();
            await tester.tap(find.byIcon(Icons.data_usage_rounded));
            await tester.pumpAndSettle();

            final menu = tester.getRect(
              find
                  .ancestor(
                    of: find.text('设备'),
                    matching: find.byType(SingleChildScrollView),
                  )
                  .first,
            );
            for (final device in _devices) {
              final executing = device.deviceId == owner;
              final status = <String>[
                if (device.deviceId == 'local') '本机',
                if (executing) '当前执行设备',
                if (!device.online)
                  '离线'
                else if (device.relayHops == 2)
                  '在线 · 2 跳中继'
                else
                  '在线',
              ].join('，');
              final row = find.bySemanticsLabel('${device.deviceName}，$status');
              expect(row, findsOneWidget);
              final rect = tester.getRect(row);
              expect(rect.height, lessThanOrEqualTo(24 * textScale + 12));
              expect(rect.left, greaterThanOrEqualTo(menu.left));
              expect(rect.right, lessThanOrEqualTo(menu.right));
              final name = find.text(device.deviceName);
              expect(
                tester.getSize(name).height,
                lessThanOrEqualTo(24 * textScale),
              );
              expect(
                find.descendant(
                  of: row,
                  matching: find.byIcon(Icons.check_rounded),
                ),
                executing ? findsOneWidget : findsNothing,
              );
            }
            expect(find.byTooltip('本机'), findsOneWidget);
            expect(find.byTooltip('当前执行设备'), findsOneWidget);
            expect(find.byTooltip('离线'), findsOneWidget);
            expect(find.byTooltip('在线 · 2 跳中继'), findsOneWidget);
            expect(find.text('本机'), findsNothing);
            expect(find.text('当前执行'), findsNothing);
            expect(find.text('在线'), findsNothing);
            expect(find.text('离线'), findsNothing);
            expect(tester.takeException(), isNull);
            await tester.pumpWidget(const SizedBox.shrink());
          },
        );
      }
    }
  }
  testWidgets(
    'clicking an online device changes the confirmed execution route',
    (tester) async {
      final bridge = _DeviceStatsBridge('local')
        ..switchGate = Completer<void>();
      await tester.pumpWidget(_app(bridge));
      await tester.pumpAndSettle();
      await tester.tap(find.byIcon(Icons.data_usage_rounded));
      await tester.pumpAndSettle();
      expect(find.byIcon(Icons.play_arrow_rounded), findsNothing);
      await tester.tap(find.text(_devices[0].deviceName));
      await tester.tap(find.text(_devices[1].deviceName));
      expect(bridge.switchCalls, isEmpty);

      await tester.tap(find.text(_devices[2].deviceName));
      await tester.pump();
      expect(bridge.switchCalls, hasLength(1));
      final request = bridge.switchCalls.single;
      expect(request.target, 'core/server.runtimeRemoteLinkService');
      expect(request.args, {'chatId': 'chat', 'targetNodeId': 'relay'});
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(_selectedRow(_devices[1].deviceName), findsOneWidget);
      expect(_selectedRow(_devices[2].deviceName), findsNothing);
      await tester.tap(find.text(_devices[2].deviceName));
      expect(bridge.switchCalls, hasLength(1));
      bridge.switchGate!.complete();
      await tester.pumpAndSettle();
      expect(find.byType(CircularProgressIndicator), findsNothing);
      expect(_selectedRow(_devices[1].deviceName), findsNothing);
      expect(_selectedRow(_devices[2].deviceName), findsOneWidget);

      await tester.tap(find.text(_devices[1].deviceName));
      await tester.pumpAndSettle();
      expect(bridge.switchCalls, hasLength(2));
      expect(bridge.switchCalls.last.args, {
        'chatId': 'chat',
        'targetNodeId': 'local',
      });
      expect(_selectedRow(_devices[1].deviceName), findsOneWidget);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  testWidgets('a failed switch preserves selection and allows retry', (
    tester,
  ) async {
    final bridge = _DeviceStatsBridge('local')
      ..switchError = StateError('device unavailable');
    await tester.pumpWidget(_app(bridge));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.data_usage_rounded));
    await tester.pumpAndSettle();
    await tester.tap(find.text(_devices[2].deviceName));
    await tester.pumpAndSettle();
    expect(_selectedRow(_devices[1].deviceName), findsOneWidget);
    expect(find.textContaining('切换失败：'), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsNothing);
    bridge.switchError = null;
    await tester.tap(find.text(_devices[2].deviceName));
    await tester.pumpAndSettle();
    expect(_selectedRow(_devices[2].deviceName), findsOneWidget);
    expect(find.textContaining('切换失败：'), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('a completed switch cannot update a different chat', (
    tester,
  ) async {
    final bridge = _DeviceStatsBridge('local')..switchGate = Completer<void>();
    await tester.pumpWidget(_app(bridge));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.data_usage_rounded));
    await tester.pumpAndSettle();
    await tester.tap(find.text(_devices[2].deviceName));
    await tester.pump();
    await tester.pumpWidget(_app(bridge, chatId: 'other-chat'));
    await tester.pumpAndSettle();
    bridge.switchGate!.complete();
    await tester.pumpAndSettle();
    expect(_selectedRow(_devices[1].deviceName), findsOneWidget);
    expect(_selectedRow(_devices[2].deviceName), findsNothing);
    expect(find.byType(CircularProgressIndicator), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}

Finder _selectedRow(String name) => find.byWidgetPredicate(
  (widget) =>
      widget is Semantics &&
      widget.properties.selected == true &&
      widget.properties.label?.startsWith('$name，') == true,
);

Widget _app(
  _DeviceStatsBridge bridge, {
  TextDirection direction = TextDirection.ltr,
  double textScale = 1,
  String? chatId = 'chat',
}) => MaterialApp(
  theme: ThemeData.dark(useMaterial3: true),
  locale: const Locale('zh'),
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  builder: (context, child) => MediaQuery(
    data: MediaQuery.of(
      context,
    ).copyWith(textScaler: TextScaler.linear(textScale)),
    child: Directionality(textDirection: direction, child: child!),
  ),
  home: Scaffold(
    body: Align(
      alignment: Alignment.topRight,
      child: ChatContextStatsTopBarButton(
        viewModel: ChatViewModel(bridge: bridge),
        currentChatId: chatId,
      ),
    ),
  ),
);

final _devices = <RuntimeDeviceSpaceDevice>[
  _device('offline', 'windows-剑', online: false),
  _device('local', 'macos-arm64'),
  _device('relay', '一个名字特别长的远程设备-with-a-long-name', relayHops: 2),
];

RuntimeDeviceSpaceDevice _device(
  String id,
  String name, {
  bool online = true,
  int? relayHops,
}) => RuntimeDeviceSpaceDevice(
  deviceId: id,
  userName: '',
  deviceName: name,
  platform: 'desktop',
  model: '',
  coreVersion: 'test-core',
  online: online,
  relayHops: relayHops,
  relayPath: const [],
  currentIdentity: null,
);

class _DeviceStatsBridge extends OperitRuntimeBridge {
  _DeviceStatsBridge(this.owner);

  String owner;
  final switchCalls = <CoreCallRequest>[];
  Completer<void>? switchGate;
  Object? switchError;

  BindingRouteStatus get status => BindingRouteStatus(
    ownerNodeId: owner,
    ownerPlatform: 'desktop',
    ownerIsLocal: owner == 'local',
    ownerReachable: owner != 'offline',
    activeNodeId: owner,
    activeIsLocal: owner == 'local',
    generation: 1,
  );

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    if (request.methodName == 'changeChatExecutionDevice') {
      switchCalls.add(request);
      await switchGate?.future;
      if (switchError case final error?) throw error;
      owner = (request.args as Map)['targetNodeId'] as String;
      return encodeCoreLink(<Object?>[0, status.toJson()]);
    }
    final value = switch (request.methodName) {
      'chatInputMenuSummary' => const ChatInputMenuSummary(
        currentWindowSize: 5826,
        inputTokenCount: 8972,
        outputTokenCount: 96,
        maxContextLength: 1000,
      ).toJson(),
      'deviceSpaceTopology' => RuntimeDeviceSpaceTopology(
        currentDeviceId: 'local',
        devices: _devices,
        connections: const [],
      ).toJson(),
      _ => throw StateError('Unexpected method: ${request.methodName}'),
    };
    return encodeCoreLink(<Object?>[0, value]);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) => Stream.value(
    CoreEvent.raw(
      requestId: request.requestId,
      target: request.target,
      propertyName: request.propertyName,
      kind: 'Snapshot',
      decodeValue: decodeCoreLink<Object?>,
      valueBytes: encodeCoreLink(status.toJson()),
    ),
  );

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
}
