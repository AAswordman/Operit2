import 'dart:async';
import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/DeviceSpaceDiscoveryPanel.dart';
import 'package:operit2/ui/common/components/OperitDialog.dart';
import 'space_join_dialog_test.dart' show JoinBridge, joinRequest;

class DeviceBridge extends JoinBridge {
  Map<String, Object?>? config = {
    'bindAddress': '0.0.0.0:37195',
    'token': 'runtime-owned-token',
    'transports': ['http', 'webSocket'],
    'discoveryEnabled': true,
    'portMode': 'fixed',
    'updatedAt': 1,
  };
  Completer<Uint8List>? discovery;
  Map<String, Object?>? pairingArgs;
  Map<String, Object?>? finishArgs;
  List<Map<String, Object?>> peers = [
    {
      'nodeId': 'ios',
      'displayName': '我的 iPhone',
      'address': 'http://192.168.1.2:37195',
    },
  ];

  /// Responds to device discovery and connection settings requests.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    switch (request.methodName) {
      case 'discoverPeers':
        calls.add(request.methodName);
        if (discovery != null) return discovery!.future;
        return encodeCoreLink([0, peers]);
      case 'startPairing':
        calls.add(request.methodName);
        pairingArgs = Map<String, Object?>.from(request.args as Map);
        return encodeCoreLink([
          0,
          {
            'pairingId': 'pair-1',
            'peerNodeId': 'ios',
            'displayName': '我的 iPhone',
          },
        ]);
      case 'finishPairing':
        calls.add(request.methodName);
        finishArgs = Map<String, Object?>.from(request.args as Map);
        return encodeCoreLink([
          0,
          {
            'nodeId': 'ios',
            'displayName': '我的 iPhone',
            'inbound': false,
            'outbound': true,
          },
        ]);
      case 'cancelPairing':
        calls.add(request.methodName);
        return encodeCoreLink([0, null]);
      case 'listenerCapabilities':
        return encodeCoreLink([
          0,
          {
            'transports': ['http', 'webSocket', 'tcp', 'bluetooth'],
            'discoveryAdvertisement': true,
          },
        ]);
      case 'localHostConfig':
        return encodeCoreLink([0, config]);
      case 'saveLocalHostConfig':
        config = Map<String, Object?>.from(
          (request.args as Map)['config'] as Map,
        );
        return encodeCoreLink([0, null]);
      case 'stopListening':
      case 'startListening':
        return encodeCoreLink([0, null]);
      default:
        return super.callBytes(request);
    }
  }
}

/// Verifies device space actions and the add-device dialog.
void main() {
  /// Mounts the localized device space toolbar.
  Future<void> mount(
    WidgetTester tester,
    DeviceBridge bridge, {
    double textScale = 1,
    ThemeMode themeMode = ThemeMode.light,
  }) async {
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('zh'),
        theme: ThemeData(),
        darkTheme: ThemeData.dark(),
        themeMode: themeMode,
        builder: (context, child) => MediaQuery(
          data: MediaQuery.of(
            context,
          ).copyWith(textScaler: TextScaler.linear(textScale)),
          child: child!,
        ),
        supportedLocales: AppLocalizations.supportedLocales,
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        home: Scaffold(
          body: Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                const Expanded(child: Text('设备')),
                DeviceSpaceDiscoveryPanel(
                  clients: GeneratedCoreProxyClients(bridge),
                  onJoined: (_) async {},
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  /// Unmounts the toolbar and closes its event stream.
  Future<void> dispose(WidgetTester tester, DeviceBridge bridge) async {
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();
    await bridge.prompts.close();
  }

  testWidgets(
    'landing page has one add action, no stacked discovery/requests/settings sections',
    (tester) async {
      final bridge = DeviceBridge();
      await mount(tester, bridge);
      expect(find.text('添加设备'), findsOneWidget);
      expect(find.text('空间加入申请'), findsNothing);
      expect(find.text('连接设置'), findsNothing);
      expect(find.byType(ExpansionTile), findsNothing);
      expect(find.byType(Switch), findsNothing);
      expect(find.byType(FilterChip), findsNothing);
      expect(
        bridge.calls,
        isNot(contains('discoverPeers')),
      ); // Discovery is intentional, not a page-load side effect.
      await dispose(tester, bridge);
    },
  );
  testWidgets('add device opens focused nearby picker, not advanced settings', (
    tester,
  ) async {
    final bridge = DeviceBridge();
    await mount(tester, bridge);
    await tester.tap(find.text('添加设备'));
    await tester.pumpAndSettle();
    expect(find.byType(OperitDialogScaffold), findsOneWidget);
    expect(find.byType(AlertDialog), findsNothing);
    expect(find.text('附近设备'), findsNothing);
    expect(find.byType(LinearProgressIndicator), findsNothing);
    expect(find.byIcon(Icons.refresh_rounded), findsOneWidget);
    expect(find.text('我的 iPhone'), findsOneWidget);
    expect(find.text('通过地址连接'), findsOneWidget);
    final dialog = find.byType(OperitDialogScaffold);
    final title = find.descendant(of: dialog, matching: find.text('添加设备'));
    expect(
      (tester.getCenter(title).dy -
              tester.getCenter(find.byIcon(Icons.refresh_rounded)).dy)
          .abs(),
      lessThan(1),
    );
    expect(
      find.descendant(
        of: find.byType(OperitDialogActionBar),
        matching: find.text('通过地址连接'),
      ),
      findsOneWidget,
    );
    expect(find.byType(Switch), findsNothing);
    expect(find.byType(FilterChip), findsNothing);
    expect(bridge.calls.where((c) => c == 'discoverPeers').length, 1);
    await tester.tap(find.byIcon(Icons.refresh_rounded));
    await tester.pumpAndSettle();
    expect(bridge.calls.where((c) => c == 'discoverPeers').length, 2);
    await dispose(tester, bridge);
  });
  testWidgets('device discovery has no loading bar or secondary title', (
    tester,
  ) async {
    final discovery = Completer<Uint8List>();
    final bridge = DeviceBridge()..discovery = discovery;
    await mount(tester, bridge);
    await tester.tap(find.text('添加设备'));
    await tester.pumpAndSettle();
    final dialog = find.byType(OperitDialogScaffold);
    final refresh = find.descendant(
      of: dialog,
      matching: find.byType(IconButton),
    );
    expect(
      find.descendant(of: dialog, matching: find.text('添加设备')),
      findsOneWidget,
    );
    expect(find.text('附近设备'), findsNothing);
    expect(find.byType(LinearProgressIndicator), findsNothing);
    expect(find.byIcon(Icons.refresh_rounded), findsOneWidget);
    expect(tester.widget<IconButton>(refresh).onPressed, isNull);
    expect(find.text('扫描中…'), findsOneWidget);
    expect(
      tester.widget<OutlinedButton>(find.byType(OutlinedButton)).onPressed,
      isNull,
    );
    discovery.complete(encodeCoreLink([0, <Object?>[]]));
    await tester.pumpAndSettle();
    expect(tester.widget<IconButton>(refresh).onPressed, isNotNull);
    expect(find.text('扫描中…'), findsNothing);
    expect(find.text('暂未发现附近设备'), findsOneWidget);
    await dispose(tester, bridge);
  });
  testWidgets('address entry returns to the redesigned dialog on cancel', (
    tester,
  ) async {
    final bridge = DeviceBridge();
    await mount(tester, bridge);
    await tester.tap(find.text('添加设备'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('通过地址连接'));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsOneWidget);
    expect(find.byType(TextField), findsNWidgets(2));
    await tester.tap(
      find.descendant(of: find.byType(AlertDialog), matching: find.text('取消')),
    );
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    expect(find.byType(OperitDialogScaffold), findsOneWidget);
    expect(
      tester
          .widget<IconButton>(
            find.widgetWithIcon(IconButton, Icons.refresh_rounded),
          )
          .onPressed,
      isNotNull,
    );
    await tester.tap(find.text('取消'));
    await tester.pumpAndSettle();
    expect(find.byType(OperitDialogScaffold), findsNothing);
    await dispose(tester, bridge);
  });
  testWidgets('device cards start pairing with the selected peer', (
    tester,
  ) async {
    final bridge = DeviceBridge();
    await mount(tester, bridge);
    await tester.tap(find.text('添加设备'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('我的 iPhone'));
    await tester.pumpAndSettle();
    expect(bridge.pairingArgs!['address'], 'http://192.168.1.2:37195');
    expect(bridge.pairingArgs!['nodeId'], 'ios');
    expect(bridge.pairingArgs!['token'], isNull);
    expect(find.byType(AlertDialog), findsOneWidget);
    expect(find.byType(TextField), findsOneWidget);
    await tester.tap(
      find.descendant(of: find.byType(AlertDialog), matching: find.text('取消')),
    );
    await tester.pumpAndSettle();
    expect(bridge.calls.where((c) => c == 'cancelPairing').length, 1);
    expect(
      tester
          .widget<IconButton>(
            find.widgetWithIcon(IconButton, Icons.refresh_rounded),
          )
          .onPressed,
      isNotNull,
    );
    await dispose(tester, bridge);
  });
  testWidgets('discovery errors are visible and refresh remains available', (
    tester,
  ) async {
    final discovery = Completer<Uint8List>();
    final bridge = DeviceBridge()..discovery = discovery;
    await mount(tester, bridge);
    await tester.tap(find.text('添加设备'));
    await tester.pumpAndSettle();
    discovery.completeError(StateError('Discovery unavailable'));
    await tester.pumpAndSettle();
    expect(find.text('Bad state: Discovery unavailable'), findsOneWidget);
    expect(find.text('暂未发现附近设备'), findsNothing);
    expect(find.byIcon(Icons.error_outline_rounded), findsOneWidget);
    expect(
      tester
          .widget<IconButton>(
            find.widgetWithIcon(IconButton, Icons.refresh_rounded),
          )
          .onPressed,
      isNotNull,
    );
    bridge.discovery = null;
    await tester.tap(find.byTooltip('刷新'));
    await tester.pumpAndSettle();
    expect(find.text('Bad state: Discovery unavailable'), findsNothing);
    expect(find.text('我的 iPhone'), findsOneWidget);
    await dispose(tester, bridge);
  });
  testWidgets('long device lists scroll without moving the header or actions', (
    tester,
  ) async {
    final bridge = DeviceBridge()
      ..peers = List.generate(
        20,
        (index) => {
          'nodeId': 'device-$index',
          'displayName': 'Device $index',
          'address': 'http://192.168.1.${index + 1}:37195',
        },
      );
    await mount(tester, bridge);
    await tester.tap(find.text('添加设备'));
    await tester.pumpAndSettle();
    final refreshPosition = tester.getCenter(find.byTooltip('刷新'));
    final manualPosition = tester.getCenter(find.text('通过地址连接'));
    await tester.scrollUntilVisible(
      find.text('Device 19'),
      200,
      scrollable: find.byType(Scrollable),
    );
    expect(find.text('Device 19'), findsOneWidget);
    expect(tester.getCenter(find.byTooltip('刷新')), refreshPosition);
    expect(tester.getCenter(find.text('通过地址连接')), manualPosition);
    expect(tester.takeException(), isNull);
    await dispose(tester, bridge);
  });
  testWidgets(
    'connection settings are secondary, discoverable switch remains accessible',
    (tester) async {
      final bridge = DeviceBridge();
      await mount(tester, bridge);
      await tester.tap(find.byTooltip('更多设备操作'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('连接设置'));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.byType(Switch), findsOneWidget);
      expect(find.byType(FilterChip), findsNothing);
      expect(find.text('保存'), findsOneWidget);
      expect(find.text('确定'), findsNothing);
      await tester.tap(find.byType(Switch));
      await tester.pumpAndSettle();
      expect(bridge.config!['discoveryEnabled'], true);
      await tester.tap(find.text('高级选项'));
      await tester.pumpAndSettle();
      expect(find.byType(FilterChip), findsNWidgets(5));
      expect(find.text('保存'), findsOneWidget);
      await tester.tap(find.text('保存'));
      await tester.pumpAndSettle();
      expect(bridge.config!['discoveryEnabled'], false);
      expect(find.byType(AlertDialog), findsNothing);
      await dispose(tester, bridge);
    },
  );
  testWidgets(
    'requests are a dialog with applicant/reviewer tabs, never a landing-page expansion',
    (tester) async {
      final bridge = DeviceBridge()..outgoing = [joinRequest()];
      await mount(tester, bridge);
      expect(find.text('等待批准'), findsNothing);
      await tester.tap(find.byTooltip('更多设备操作'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('空间加入申请'));
      await tester.pumpAndSettle();
      expect(find.byType(TabBar), findsOneWidget);
      expect(find.text('我发出的'), findsOneWidget);
      expect(find.text('待我审批'), findsOneWidget);
      expect(find.text('审批人：我的 iPhone\n等待批准'), findsOneWidget);
      expect(find.byType(Switch), findsNothing);
      expect(find.byType(ExpansionTile), findsNothing);
      await dispose(tester, bridge);
    },
  );
  testWidgets('compact toolbar and picker fit a small phone without overflow', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(320, 650);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final bridge = DeviceBridge();
    await mount(tester, bridge);
    expect(tester.takeException(), isNull);
    await tester.tap(find.text('添加设备'));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    await dispose(tester, bridge);
  });
  for (final size in [const Size(740, 360), const Size(320, 650)]) {
    testWidgets('dialog handles empty results and large text at $size', (
      tester,
    ) async {
      tester.view.physicalSize = size;
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final bridge = DeviceBridge()..peers = [];
      await mount(tester, bridge, textScale: 1.8, themeMode: ThemeMode.dark);
      await tester.tap(find.text('添加设备'));
      await tester.pumpAndSettle();
      expect(find.byType(OperitDialogScaffold), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.scrollUntilVisible(
        find.text('暂未发现附近设备'),
        80,
        scrollable: find.byType(Scrollable),
      );
      expect(find.text('暂未发现附近设备'), findsOneWidget);
      await tester.tap(find.text('取消'));
      await tester.pumpAndSettle();
      expect(find.byType(OperitDialogScaffold), findsNothing);
      await dispose(tester, bridge);
    });
  }
  testWidgets(
    'pairing an admitted member ends at the direct link without a join request',
    (tester) async {
      final bridge = DeviceBridge();
      await mount(tester, bridge);
      await tester.tap(find.text('添加设备'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('我的 iPhone'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), '123456');
      await tester.tap(find.text('完成连接'));
      await tester.pumpAndSettle();
      expect(bridge.finishArgs!['confirmationCode'], '123456');
      expect(bridge.calls, isNot(contains('requestDeviceSpaceJoin')));
      expect(find.text('已建立直连：我的 iPhone'), findsOneWidget);
      await dispose(tester, bridge);
    },
  );
  testWidgets('pairing a device outside the Space still asks to join', (
    tester,
  ) async {
    final bridge = DeviceBridge()..members = <String>['mac'];
    await mount(tester, bridge);
    await tester.tap(find.text('添加设备'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('我的 iPhone'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), '123456');
    await tester.tap(find.text('完成连接'));
    await tester.pumpAndSettle();
    expect(bridge.calls, contains('requestDeviceSpaceJoin'));
    expect(find.textContaining('已建立直连'), findsNothing);
    await dispose(tester, bridge);
  });
  testWidgets('focused pairing dialog scans only the selected device', (
    tester,
  ) async {
    final bridge = DeviceBridge()
      ..peers = [
        {
          'nodeId': 'ios',
          'displayName': '我的 iPhone',
          'address': 'http://192.168.1.2:37195',
        },
        {
          'nodeId': 'other',
          'displayName': '其他设备',
          'address': 'http://192.168.1.3:37195',
        },
      ];
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('zh'),
        supportedLocales: AppLocalizations.supportedLocales,
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        home: Scaffold(
          body: Builder(
            builder: (context) => TextButton(
              onPressed: () => showDeviceSpacePairingDialog(
                context,
                clients: GeneratedCoreProxyClients(bridge),
                title: '建立直连',
                hint: '该设备已是本空间成员。',
                onlyNodeId: 'ios',
              ),
              child: const Text('打开'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('打开'));
    await tester.pumpAndSettle();
    expect(find.text('建立直连'), findsOneWidget);
    expect(find.text('该设备已是本空间成员。'), findsOneWidget);
    expect(find.text('我的 iPhone'), findsOneWidget);
    expect(find.text('其他设备'), findsNothing);
    await dispose(tester, bridge);
  });
}
