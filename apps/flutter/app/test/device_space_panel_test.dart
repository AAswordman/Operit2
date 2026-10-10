import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/settings/runtime/NetworkControlPanel.dart';
import 'package:operit2/ui/features/settings/runtime/RuntimeSettingsPanel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

const List<String> _userCapabilities = <String>[
  'network.devices.view',
  'network.user',
  'chat.read',
  'network.relay',
  'runtime.execute',
];

const List<String> _administratorCapabilities = <String>['*'];

/// Builds one projected Space member.
Map<String, Object?> _member({
  required String deviceId,
  required String deviceName,
  bool online = true,
  int? relayHops,
  List<String>? relayPath,
  String identityName = 'User',
  List<String> capabilities = _userCapabilities,
}) {
  return <String, Object?>{
    'deviceId': deviceId,
    'userName': '',
    'deviceName': deviceName,
    'platform': 'linux',
    'model': 'x86_64',
    'coreVersion': '2.0.0',
    'online': online,
    'relayHops': relayHops,
    'relayPath': relayPath,
    'currentIdentity': <String, Object?>{
      'displayName': identityName,
      'capabilities': capabilities,
    },
  };
}

/// Builds one topology with the current device in front.
Map<String, Object?> _topology({
  List<String> selfCapabilities = _userCapabilities,
  String selfIdentityName = 'User',
}) {
  return <String, Object?>{
    'currentDeviceId': 'self',
    'devices': <Map<String, Object?>>[
      _member(
        deviceId: 'self',
        deviceName: 'this-linux',
        identityName: selfIdentityName,
        capabilities: selfCapabilities,
      ),
      _member(
        deviceId: 'tablet',
        deviceName: 'android-tablet',
        relayHops: 1,
        relayPath: <String>['relay'],
        identityName: 'Relay',
        capabilities: <String>['network.relay'],
      ),
      _member(deviceId: 'relay', deviceName: 'relay-linux'),
    ],
    'connections': <Object?>[],
  };
}

/// Builds one synchronized control state with the two built-in identities.
Map<String, Object?> _control() {
  return <String, Object?>{
    'spaceId': 'space',
    'initialized': true,
    'memberNodeIds': <String>['self', 'tablet', 'relay'],
    'roles': <String, Object?>{
      'admin': <String, Object?>{
        'roleId': 'admin',
        'displayName': 'Administrator',
        'capabilities': _administratorCapabilities,
      },
      'user': <String, Object?>{
        'roleId': 'user',
        'displayName': 'User',
        'capabilities': _userCapabilities,
      },
    },
    'deviceIdentityIds': <String, String>{
      'self': 'user',
      'tablet': 'relay',
      'relay': 'user',
    },
    'disconnectedNodeIds': <Object?>[],
    'policies': <String, String>{},
  };
}

/// Serves device-space projections and pairing streams to the settings panels.
class DeviceSpacePanelBridge extends OperitRuntimeBridge {
  DeviceSpacePanelBridge({required this.topology, required this.control});

  Map<String, Object?> topology;
  Map<String, Object?> control;
  List<Map<String, Object?>> paired = <Map<String, Object?>>[];
  Map<String, String> statuses = <String, String>{};
  final List<String> calls = <String>[];
  final StreamController<CoreEvent> prompts =
      StreamController<CoreEvent>.broadcast();

  /// Serves the read calls the panels issue.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request.methodName);
    switch (request.methodName) {
      case 'deviceSpace':
        return encodeCoreLink([
          0,
          <String, Object?>{
            'spaceId': 'space',
            'spaceName': '测试空间',
            'spaceRevision': 3,
            'members': <String>['self', 'tablet', 'relay'],
          },
        ]);
      case 'deviceSpaceTopology':
        return encodeCoreLink([0, topology]);
      case 'deviceSpaceControl':
        return encodeCoreLink([0, control]);
      case 'deviceSpaceControlAudit':
        return encodeCoreLink([0, <Object?>[]]);
      case 'outgoingDeviceSpaceJoins':
      case 'incomingDeviceSpaceJoins':
        return encodeCoreLink([0, <Object?>[]]);
      case 'discoverPeers':
        return encodeCoreLink([0, <Object?>[]]);
      default:
        throw StateError('Unexpected ${request.methodName}');
    }
  }

  /// Keeps each watched property on its own stream, as the platform proxies do.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    return prompts.stream.where(
      (event) => event.propertyName == request.propertyName,
    );
  }

  /// Rejects snapshots the panels do not request.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) {
    throw UnimplementedError();
  }

  /// Rejects pushes the panels do not open.
  @override
  Future<CorePushSink> push(CorePushRequest request) {
    throw UnimplementedError();
  }

  /// Publishes the paired-device snapshot the page subscribes to.
  void emitPairedDevices() {
    prompts.add(
      CoreEvent.raw(
        requestId: 'paired-devices-watch',
        target: 'core/server.runtimeRemoteLinkService',
        propertyName: 'pairedDevicesFlow',
        kind: 'Snapshot',
        valueBytes: encodeCoreLink(<String, Object?>{
          for (final device in paired) device['deviceId'] as String: device,
        }),
        decodeValue: decodeCoreLink<Object?>,
      ),
    );
  }

  /// Publishes the direct-link statuses the page subscribes to.
  void emitPairedStatuses() {
    prompts.add(
      CoreEvent.raw(
        requestId: 'paired-statuses-watch',
        target: 'core/server.runtimeRemoteLinkService',
        propertyName: 'pairedDeviceStatusesFlow',
        kind: 'Snapshot',
        valueBytes: encodeCoreLink(statuses),
        decodeValue: decodeCoreLink<Object?>,
      ),
    );
  }
}

void main() {
  /// Mounts the device-space page with injected Core clients.
  Future<void> mountPanel(
    WidgetTester tester,
    DeviceSpacePanelBridge bridge,
  ) async {
    tester.view.physicalSize = const Size(900, 1600);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    tester.binding.platformDispatcher.localesTestValue = const <Locale>[
      Locale('zh'),
    ];
    addTearDown(tester.binding.platformDispatcher.clearLocalesTestValue);
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: Scaffold(
          body: RuntimeSettingsPanel(
            clients: GeneratedCoreProxyClients(bridge),
            onOpenProfile: () {},
          ),
        ),
      ),
    );
    await tester.pump();
    bridge.emitPairedDevices();
    bridge.emitPairedStatuses();
    await tester.pump();
    await tester.pump();
  }

  /// Mounts only the identity and permission panel.
  Future<void> mountNetworkPanel(
    WidgetTester tester,
    DeviceSpacePanelBridge bridge,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('zh'),
        supportedLocales: AppLocalizations.supportedLocales,
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        home: Scaffold(
          body: NetworkControlPanel(
            clients: GeneratedCoreProxyClients(bridge),
            onChanged: () async {},
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump();
  }

  /// Unmounts the panels and closes their event stream.
  Future<void> unmount(
    WidgetTester tester,
    DeviceSpacePanelBridge bridge,
  ) async {
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump();
    await bridge.prompts.close();
  }

  testWidgets(
    'member rows stay readable without device-management capabilities',
    (tester) async {
      final bridge = DeviceSpacePanelBridge(
        topology: _topology(),
        control: _control(),
      );
      await mountPanel(tester, bridge);
      expect(find.text('android-tablet'), findsOneWidget);
      expect(find.text('中继节点'), findsOneWidget);
      expect(find.textContaining('经中继 1 跳'), findsWidgets);
      expect(find.textContaining('空间成员 · 未直连'), findsWidgets);
      expect(find.textContaining('路径: 本机 → relay-linux'), findsWidgets);
      // Link-layer pairing is not a Space capability: a relayed member offers
      // the direct-link entry even without any management capability, and
      // nothing else.
      final menus = find.byIcon(Icons.more_vert_outlined);
      expect(menus, findsWidgets);
      await tester.tap(menus.first);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));
      expect(find.widgetWithText(MenuItemButton, '建立直连'), findsOneWidget);
      expect(find.widgetWithText(MenuItemButton, '移除设备'), findsNothing);
      expect(find.widgetWithText(MenuItemButton, '断开设备'), findsNothing);
      expect(tester.takeException(), isNull);
      await unmount(tester, bridge);
    },
  );

  testWidgets('tapping a member row opens the read-only device details', (
    tester,
  ) async {
    final bridge = DeviceSpacePanelBridge(
      topology: _topology(),
      control: _control(),
    );
    await mountPanel(tester, bridge);
    await tester.tap(find.text('android-tablet'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.text('当前身份: 中继节点'), findsOneWidget);
    expect(find.text('当前能力: 中继网络流量'), findsOneWidget);
    expect(find.text('设备 ID: tablet'), findsOneWidget);
    expect(
      find.textContaining('在线 · 经中继 1 跳'),
      findsWidgets,
    );
    expect(find.textContaining('路径: 本机 → relay-linux'), findsWidgets);
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });

  testWidgets('administrator member rows offer every device command', (
    tester,
  ) async {
    final bridge = DeviceSpacePanelBridge(
      topology: _topology(
        selfCapabilities: _administratorCapabilities,
        selfIdentityName: 'Administrator',
      ),
      control: _control(),
    );
    await mountPanel(tester, bridge);
    expect(find.text('中继节点'), findsOneWidget);
    final menus = find.byIcon(Icons.more_vert_outlined);
    expect(menus, findsWidgets);
    await tester.tap(menus.first);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.widgetWithText(MenuItemButton, '重新接纳设备'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '重置为默认身份'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '设置设备身份'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '断开设备'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '移除设备'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '建立直连'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });

  testWidgets('tapping the direct-link entry opens the focused pairing dialog', (
    tester,
  ) async {
    final bridge = DeviceSpacePanelBridge(
      topology: _topology(),
      control: _control(),
    );
    await mountPanel(tester, bridge);
    final menus = find.byIcon(Icons.more_vert_outlined);
    await tester.tap(menus.first);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    await tester.tap(find.widgetWithText(MenuItemButton, '建立直连'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(
      find.text('该设备已是本空间成员。与它配对成功即建立直连，无需重新加入空间；配对需要对方显示、由你输入六位确认码。'),
      findsOneWidget,
    );
    expect(find.text('暂未发现附近设备'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });

  testWidgets('paired Space members keep the managed device commands', (
    tester,
  ) async {
    final bridge = DeviceSpacePanelBridge(
      topology: _topology(
        selfCapabilities: _administratorCapabilities,
        selfIdentityName: 'Administrator',
      ),
      control: _control(),
    )..paired = <Map<String, Object?>>[
      <String, Object?>{
        'deviceId': 'tablet',
        'deviceInfo': <String, Object?>{'platform': 'android', 'model': 'elish'},
        'inbound': true,
        'outbound': true,
      },
    ]
    ..statuses = <String, String>{'tablet': 'Online'};
    await mountPanel(tester, bridge);
    expect(find.text('android-elish'), findsOneWidget);
    expect(find.textContaining('空间成员 · 已直连'), findsOneWidget);
    final menus = find.byIcon(Icons.more_vert_outlined);
    expect(menus, findsWidgets);
    await tester.tap(menus.first);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.widgetWithText(MenuItemButton, '断开设备'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '移除设备'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '重置为默认身份'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '解除配对'), findsOneWidget);
    // A paired member already has its edge; the pairing entry would be noise.
    expect(find.widgetWithText(MenuItemButton, '建立直连'), findsNothing);
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });

  testWidgets('paired devices outside the Space keep local pairing actions only', (
    tester,
  ) async {
    final bridge = DeviceSpacePanelBridge(
      topology: _topology(
        selfCapabilities: _administratorCapabilities,
        selfIdentityName: 'Administrator',
      ),
      control: _control(),
    )..paired = <Map<String, Object?>>[
      <String, Object?>{
        'deviceId': 'stranger',
        'deviceInfo': <String, Object?>{
          'platform': 'android',
          'model': 'stranger',
        },
        'inbound': true,
        'outbound': true,
      },
    ]
    ..statuses = <String, String>{'stranger': 'Online'};
    await mountPanel(tester, bridge);
    expect(find.text('android-stranger'), findsOneWidget);
    expect(find.textContaining('已配对'), findsWidgets);
    final menus = find.byIcon(Icons.more_vert_outlined);
    await tester.tap(menus.first);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.widgetWithText(MenuItemButton, '解除配对'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '加入设备空间'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '断开设备'), findsNothing);
    expect(find.widgetWithText(MenuItemButton, '移除设备'), findsNothing);
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });

  testWidgets('the identity panel never repeats the device list', (
    tester,
  ) async {
    final bridge = DeviceSpacePanelBridge(
      topology: _topology(),
      control: _control(),
    );
    await mountNetworkPanel(tester, bridge);
    expect(find.text('3 设备'), findsNothing);
    expect(find.text('android-tablet'), findsNothing);
    expect(find.byIcon(Icons.more_vert_outlined), findsNothing);
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });

  testWidgets('administrators see collapsed identity and audit sections', (
    tester,
  ) async {
    final bridge = DeviceSpacePanelBridge(
      topology: _topology(
        selfCapabilities: _administratorCapabilities,
        selfIdentityName: 'Administrator',
      ),
      control: _control(),
    );
    await mountNetworkPanel(tester, bridge);
    expect(find.text('当前身份: 管理员'), findsOneWidget);
    expect(find.text('当前能力: 全部权限'), findsWidgets);
    expect(find.text('设置设备身份'), findsOneWidget);
    // Both long lists start collapsed and only advertise their size.
    expect(find.text('身份定义'), findsOneWidget);
    expect(find.text('共 2 个身份'), findsOneWidget);
    expect(find.text('添加角色'), findsOneWidget);
    expect(find.text('管理员'), findsNothing);
    expect(find.text('审计'), findsOneWidget);
    expect(find.text('共 0 条记录'), findsOneWidget);
    expect(find.text('暂无控制命令记录'), findsNothing);
    await tester.tap(find.text('身份定义'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.text('管理员'), findsOneWidget);
    await tester.tap(find.text('审计'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.text('暂无控制命令记录'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });

  testWidgets('members without capabilities only see their own identity', (
    tester,
  ) async {
    final bridge = DeviceSpacePanelBridge(
      topology: _topology(),
      control: _control(),
    );
    await mountNetworkPanel(tester, bridge);
    expect(find.text('当前身份: 普通用户'), findsOneWidget);
    expect(
      find.text('当前能力: 查看设备与当前身份、使用网络、读取聊天记录、中继网络流量、执行任务'),
      findsOneWidget,
    );
    expect(find.text('身份定义'), findsNothing);
    expect(find.text('添加角色'), findsNothing);
    expect(find.text('设置设备身份'), findsNothing);
    expect(find.text('审计'), findsNothing);
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });
}
