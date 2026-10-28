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
  'network.relay',
  'runtime.execute',
  'storage.provide',
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
        capabilities: <String>[
          'network.devices.view',
          'network.relay',
          'storage.provide',
        ],
      ),
      _member(deviceId: 'relay', deviceName: 'relay-linux'),
    ],
    'connections': <Object?>[],
  };
}

/// Builds one synchronized control state with all four built-in identities.
Map<String, Object?> _control({List<String> disconnectedNodeIds = const []}) {
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
      'relay': <String, Object?>{
        'roleId': 'relay',
        'displayName': 'Relay',
        'capabilities': <String>[
          'network.devices.view',
          'network.relay',
          'storage.provide',
        ],
      },
      'edge': <String, Object?>{
        'roleId': 'edge',
        'displayName': 'Edge',
        'capabilities': <String>['network.devices.view', 'runtime.execute'],
      },
    },
    'deviceIdentityIds': <String, String>{
      'self': 'user',
      'tablet': 'relay',
      'relay': 'user',
    },
    'disconnectedNodeIds': <Object?>[...disconnectedNodeIds],
    'policies': <String, String>{},
  };
}

/// Serves device-space projections and pairing streams to the settings panels.
class DeviceSpacePanelBridge extends OperitRuntimeBridge {
  DeviceSpacePanelBridge({required this.topology, required this.control});

  Map<String, Object?> topology;
  Map<String, Object?> control;
  List<Map<String, Object?>> audit = <Map<String, Object?>>[];
  List<Map<String, Object?>> paired = <Map<String, Object?>>[];
  Map<String, String> statuses = <String, String>{};
  final List<String> calls = <String>[];
  final failures = <String>{};
  bool left = false;
  final StreamController<CoreEvent> prompts =
      StreamController<CoreEvent>.broadcast();

  /// Serves the read calls the panels issue.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request.methodName);
    if (failures.contains(request.methodName)) {
      throw StateError('injected failure: ${request.methodName}');
    }
    switch (request.methodName) {
      case 'deviceSpace':
        return encodeCoreLink([
          0,
          <String, Object?>{
            'spaceId': 'space',
            'spaceName': left ? '恢复后的空间' : '测试空间',
            'spaceRevision': 3,
            'members': <String>['self', 'tablet', 'relay'],
          },
        ]);
      case 'leaveDeviceSpace':
        left = true;
        failures.clear();
        topology = <String, Object?>{
          ...topology,
          'devices': <Object?>[(topology['devices']! as List).first],
          'connections': <Object?>[],
        };
        return encodeCoreLink([
          0,
          <String, Object?>{
            'spaceId': 'new-space',
            'spaceName': '恢复后的空间',
            'spaceRevision': 1,
            'members': <String>['self'],
          },
        ]);
      case 'deviceSpaceTopology':
        return encodeCoreLink([0, topology]);
      case 'deviceSpaceControl':
        return encodeCoreLink([0, control]);
      case 'deviceSpaceControlAudit':
        return encodeCoreLink([0, audit]);
      case 'outgoingDeviceSpaceJoins':
      case 'incomingDeviceSpaceJoins':
        return encodeCoreLink([0, <Object?>[]]);
      case 'discoverPeers':
        return encodeCoreLink([0, <Object?>[]]);
      case 'updatePairedDeviceEndpoint':
        return encodeCoreLink([0, null]);
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

  /// Publishes the synchronized space-plus-topology snapshot the page
  /// subscribes to.
  void emitDeviceSpaceSnapshot() {
    final devices = topology['devices']! as List<Map<String, Object?>>;
    prompts.add(
      CoreEvent.raw(
        requestId: 'device-space-snapshot-watch',
        target: 'core/server.runtimeRemoteLinkService',
        propertyName: 'deviceSpaceSnapshotFlow',
        kind: 'Snapshot',
        valueBytes: encodeCoreLink(<String, Object?>{
          'space': <String, Object?>{
            'spaceId': 'space',
            'spaceName': '测试空间',
            'spaceRevision': 3,
            'members': <String>[
              for (final device in devices) device['deviceId'] as String,
            ],
          },
          'topology': topology,
        }),
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
    DeviceSpacePanelBridge bridge, {
    double width = 800,
    double textScale = 1,
    Brightness brightness = Brightness.light,
    TargetPlatform platform = TargetPlatform.android,
  }) async {
    tester.view.physicalSize = Size(width, 1200);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await tester.pumpWidget(
      MaterialApp(
        theme: ThemeData(
          platform: platform,
          colorScheme: ColorScheme.fromSeed(
            seedColor: Colors.blue,
            brightness: brightness,
          ),
        ),
        builder: (context, child) => MediaQuery(
          data: MediaQuery.of(
            context,
          ).copyWith(textScaler: TextScaler.linear(textScale)),
          child: child!,
        ),
        locale: const Locale('zh'),
        supportedLocales: AppLocalizations.supportedLocales,
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        home: Scaffold(
          body: SingleChildScrollView(
            padding: const EdgeInsets.all(14),
            child: NetworkControlPanel(
              clients: GeneratedCoreProxyClients(bridge),
              onChanged: () async {},
            ),
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

  for (final failed in ['deviceSpace', 'deviceSpaceTopology']) {
    testWidgets('$failed failure keeps local leave accessible', (tester) async {
      final bridge = DeviceSpacePanelBridge(
        topology: _topology(),
        control: _control(),
      )..failures.add(failed);
      await mountPanel(tester, bridge);
      expect(find.textContaining('injected failure: $failed'), findsWidgets);
      final l10n = AppLocalizations.of(
        tester.element(find.byType(RuntimeSettingsPanel)),
      )!;
      final leave = find.widgetWithText(
        TextButton,
        l10n.settingsRuntimeLeaveSpace,
      );
      expect(leave, findsOneWidget);
      expect(tester.widget<TextButton>(leave).onPressed, isNotNull);
      await tester.tap(leave);
      await tester.pump();
      await tester.tap(
        find.widgetWithText(
          FilledButton,
          l10n.settingsRuntimeLeaveSpaceConfirm,
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(
        bridge.calls.where((call) => call == 'leaveDeviceSpace'),
        hasLength(1),
      );
      expect(find.textContaining('injected failure: $failed'), findsNothing);
      expect(find.byIcon(Icons.more_horiz_rounded), findsWidgets);
      expect(find.text('android-tablet'), findsNothing);
      expect(tester.takeException(), isNull);
      await unmount(tester, bridge);
    });
  }

  testWidgets('control failure preserves healthy device rows', (tester) async {
    final bridge = DeviceSpacePanelBridge(
      topology: _topology(),
      control: _control(),
    )..failures.add('deviceSpaceControl');
    await mountPanel(tester, bridge);
    expect(find.text('android-tablet'), findsOneWidget);
    expect(
      find.textContaining('injected failure: deviceSpaceControl'),
      findsWidgets,
    );
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });

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
      expect(find.widgetWithText(MenuItemButton, '隔离设备'), findsNothing);
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
    expect(find.text('可用权限: 查看设备与当前身份、中继网络流量、保存共享数据副本'), findsOneWidget);
    expect(find.text('设备 ID: tablet'), findsOneWidget);
    expect(find.textContaining('在线 · 经中继 1 跳'), findsWidgets);
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
    // Isolation is one slot: an unrestricted device offers the isolate
    // command only, never the lift action alongside it.
    expect(find.widgetWithText(MenuItemButton, '隔离设备'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '重置为默认身份'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '设置设备身份'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '解除隔离设备'), findsNothing);
    expect(find.widgetWithText(MenuItemButton, '移除设备'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '建立直连'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });

  testWidgets(
    'tapping the direct-link entry opens the focused pairing dialog',
    (tester) async {
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
    },
  );

  testWidgets('paired Space members keep the managed device commands', (
    tester,
  ) async {
    final bridge =
        DeviceSpacePanelBridge(
            topology: _topology(
              selfCapabilities: _administratorCapabilities,
              selfIdentityName: 'Administrator',
            ),
            control: _control(),
          )
          ..paired = <Map<String, Object?>>[
            <String, Object?>{
              'deviceId': 'tablet',
              'deviceInfo': <String, Object?>{
                'platform': 'android',
                'model': 'elish',
              },
              'inbound': true,
              'outbound': true,
            },
          ]
          ..statuses = <String, String>{'tablet': 'Online'};
    await mountPanel(tester, bridge);
    // The synchronized Space profile name wins over the pairing-time record.
    expect(find.text('android-tablet'), findsOneWidget);
    expect(find.text('android-elish'), findsNothing);
    expect(find.textContaining('空间成员 · 已直连'), findsOneWidget);
    final menus = find.byIcon(Icons.more_vert_outlined);
    expect(menus, findsWidgets);
    await tester.tap(menus.first);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.widgetWithText(MenuItemButton, '隔离设备'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '移除设备'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '重置为默认身份'), findsOneWidget);
    expect(find.widgetWithText(MenuItemButton, '解除配对'), findsOneWidget);
    // A paired member already has its edge; the pairing entry would be noise.
    expect(find.widgetWithText(MenuItemButton, '建立直连'), findsNothing);
    // A working direct link never offers rewriting its endpoint by hand.
    expect(find.widgetWithText(MenuItemButton, '更新直连地址'), findsNothing);
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });

  testWidgets(
    'a relayed paired member reads online despite a dead direct link',
    (tester) async {
      // The direct Peer Link died with the peer's old LAN address while the
      // routed topology still reaches the member through a relay. The row
      // must follow the route (as list_core_nodes does) and the renamed
      // synchronized profile, not the direct probe or the pairing record.
      final bridge = DeviceSpacePanelBridge(
        topology: <String, Object?>{
          'currentDeviceId': 'self',
          'devices': <Map<String, Object?>>[
            _member(
              deviceId: 'self',
              deviceName: 'this-linux',
              identityName: 'Administrator',
              capabilities: _administratorCapabilities,
            ),
            _member(
              deviceId: 'tablet',
              deviceName: 'tablet-renamed',
              online: true,
              relayHops: 2,
              relayPath: <String>['relay', 'bridge'],
              identityName: 'Relay',
              capabilities: <String>[
                'network.devices.view',
                'network.relay',
                'storage.provide',
              ],
            ),
            _member(deviceId: 'relay', deviceName: 'relay-linux'),
          ],
          'connections': <Object?>[],
        },
        control: _control(),
      )
        ..paired = <Map<String, Object?>>[
          <String, Object?>{
            'deviceId': 'tablet',
            'deviceInfo': <String, Object?>{
              'platform': 'android',
              'model': 'elish',
            },
            'inbound': true,
            'outbound': true,
          },
        ]
        ..statuses = <String, String>{'tablet': 'Offline'};
      await mountPanel(tester, bridge);
      // The synchronized profile name wins over the pairing-time record.
      expect(find.text('tablet-renamed'), findsOneWidget);
      expect(find.text('android-elish'), findsNothing);
      // The route-based state wins over the offline direct-link probe.
      expect(find.textContaining('在线 · 经中继 2 跳'), findsWidgets);
      expect(find.byIcon(Icons.cloud_off_outlined), findsNothing);
      expect(tester.takeException(), isNull);
      await unmount(tester, bridge);
    },
  );

  testWidgets(
    'a failed direct link offers updating the stored address',
    (tester) async {
      // The direct link died with the peer's old address. The entry is
      // offered regardless of relay reachability, feeds the operator address
      // to the runtime, and reports the save without waiting for the dial.
      final bridge = DeviceSpacePanelBridge(
        topology: <String, Object?>{
          'currentDeviceId': 'self',
          'devices': <Map<String, Object?>>[
            _member(
              deviceId: 'self',
              deviceName: 'this-linux',
              identityName: 'Administrator',
              capabilities: _administratorCapabilities,
            ),
            _member(
              deviceId: 'tablet',
              deviceName: 'tablet-renamed',
              online: false,
              identityName: 'Relay',
              capabilities: <String>[
                'network.devices.view',
                'network.relay',
                'storage.provide',
              ],
            ),
          ],
          'connections': <Object?>[],
        },
        control: _control(),
      )
        ..paired = <Map<String, Object?>>[
          <String, Object?>{
            'deviceId': 'tablet',
            'deviceInfo': <String, Object?>{
              'platform': 'android',
              'model': 'elish',
            },
            'inbound': false,
            'outbound': true,
          },
        ]
        ..statuses = <String, String>{'tablet': 'Offline'};
      await mountPanel(tester, bridge);
      final menus = find.byIcon(Icons.more_vert_outlined);
      expect(menus, findsWidgets);
      await tester.tap(menus.first);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));
      expect(find.widgetWithText(MenuItemButton, '更新直连地址'), findsOneWidget);
      await tester.tap(find.widgetWithText(MenuItemButton, '更新直连地址'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));
      await tester.enterText(
        find.byType(TextField).last,
        'ws://10.0.0.9:37195/link',
      );
      await tester.tap(find.byType(TextButton).last);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));
      expect(bridge.calls.contains('updatePairedDeviceEndpoint'), isTrue);
      expect(find.text('直连地址已更新'), findsOneWidget);
      // Let the feedback SnackBar's auto-dismiss timer run out before unmount.
      await tester.pump(const Duration(seconds: 5));
      expect(tester.takeException(), isNull);
      await unmount(tester, bridge);
    },
  );

  testWidgets(
    'snapshot events refresh the open page without re-entering it',
    (tester) async {
      // 直连在中途建立：状态流翻转 + 同步快照推送，页面必须实时跟上，
      // 而不是停留在旧投影上等待重新进入。
      final bridge = DeviceSpacePanelBridge(
        topology: <String, Object?>{
          'currentDeviceId': 'self',
          'devices': <Map<String, Object?>>[
            _member(
              deviceId: 'self',
              deviceName: 'this-linux',
              identityName: 'Administrator',
              capabilities: _administratorCapabilities,
            ),
            _member(
              deviceId: 'tablet',
              deviceName: 'tablet-renamed',
              online: false,
              identityName: 'Relay',
              capabilities: <String>[
                'network.devices.view',
                'network.relay',
                'storage.provide',
              ],
            ),
          ],
          'connections': <Object?>[],
        },
        control: _control(),
      )
        ..paired = <Map<String, Object?>>[
          <String, Object?>{
            'deviceId': 'tablet',
            'deviceInfo': <String, Object?>{
              'platform': 'android',
              'model': 'elish',
            },
            'inbound': true,
            'outbound': true,
          },
        ]
        ..statuses = <String, String>{'tablet': 'Offline'};
      await mountPanel(tester, bridge);
      expect(find.text('tablet-renamed'), findsOneWidget);
      expect(find.textContaining('离线'), findsWidgets);
      // 对端直连建立：可用性翻转，同步快照携带新拓扑。
      bridge.statuses = <String, String>{'tablet': 'Online'};
      bridge.topology = <String, Object?>{
        'currentDeviceId': 'self',
        'devices': <Map<String, Object?>>[
          _member(
            deviceId: 'self',
            deviceName: 'this-linux',
            identityName: 'Administrator',
            capabilities: _administratorCapabilities,
          ),
          _member(
            deviceId: 'tablet',
            deviceName: 'tablet-renamed',
            online: true,
            identityName: 'Relay',
            capabilities: <String>[
              'network.devices.view',
              'network.relay',
              'storage.provide',
            ],
          ),
        ],
        'connections': <Object?>[],
      };
      bridge.emitPairedStatuses();
      bridge.emitDeviceSpaceSnapshot();
      await tester.pump();
      await tester.pump();
      expect(find.textContaining('在线 · 直连'), findsWidgets);
      expect(tester.takeException(), isNull);
      await unmount(tester, bridge);
    },
  );

  testWidgets(
    'paired devices outside the Space keep local pairing actions only',
    (tester) async {
      final bridge =
          DeviceSpacePanelBridge(
              topology: _topology(
                selfCapabilities: _administratorCapabilities,
                selfIdentityName: 'Administrator',
              ),
              control: _control(),
            )
            ..paired = <Map<String, Object?>>[
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
    },
  );

  testWidgets(
    'an inbound-only paired device outside the Space still offers joining',
    (tester) async {
      // The pairing direction is transport history: this device answered the
      // pairing, so only the inbound credential exists, and the join entry
      // must not depend on holding the outbound side.
      final bridge =
          DeviceSpacePanelBridge(
              topology: _topology(
                selfCapabilities: _administratorCapabilities,
                selfIdentityName: 'Administrator',
              ),
              control: _control(),
            )
            ..paired = <Map<String, Object?>>[
              <String, Object?>{
                'deviceId': 'stranger',
                'deviceInfo': <String, Object?>{
                  'platform': 'android',
                  'model': 'stranger',
                },
                'inbound': true,
                'outbound': false,
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
      expect(find.widgetWithText(MenuItemButton, '加入设备空间'), findsOneWidget);
      expect(find.widgetWithText(MenuItemButton, '解除配对'), findsOneWidget);
      expect(find.widgetWithText(MenuItemButton, '断开设备'), findsNothing);
      expect(find.widgetWithText(MenuItemButton, '移除设备'), findsNothing);
      expect(tester.takeException(), isNull);
      await unmount(tester, bridge);
    },
  );

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
    expect(find.text('本机身份'), findsOneWidget);
    expect(find.text('管理员'), findsOneWidget);
    expect(find.text('全部权限'), findsOneWidget);
    expect(find.text('设置设备身份'), findsNothing);
    // Both long lists start collapsed and only advertise their size.
    expect(find.text('身份权限'), findsOneWidget);
    expect(find.text('4 个身份'), findsOneWidget);
    expect(find.text('新建身份'), findsOneWidget);
    expect(find.byKey(const ValueKey('space-role-admin')), findsNothing);
    expect(find.text('操作记录'), findsOneWidget);
    expect(find.text('0 条记录'), findsOneWidget);
    expect(find.text('暂无操作记录，身份和设备的变更会显示在这里'), findsNothing);
    await tester.tap(find.text('身份权限'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.text('管理员'), findsNWidgets(2));
    expect(find.byType(Card), findsNothing);
    for (final role in <String>['admin', 'user', 'relay', 'edge']) {
      final row = find.byKey(ValueKey('space-role-$role'));
      expect(row, findsOneWidget);
      expect(
        find.descendant(of: row, matching: find.byType(IconButton)),
        findsNothing,
      );
    }
    expect(
      tester.getSize(find.byKey(const ValueKey('space-role-admin'))).height,
      lessThan(64),
    );
    await tester.tap(find.text('操作记录'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.text('暂无操作记录，身份和设备的变更会显示在这里'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });

  for (final platform in <TargetPlatform>[
    TargetPlatform.android,
    TargetPlatform.windows,
  ]) {
    testWidgets('custom identity actions stay compact on ${platform.name}', (
      tester,
    ) async {
      final bridge = DeviceSpacePanelBridge(
        topology: _topology(
          selfCapabilities: _administratorCapabilities,
          selfIdentityName: 'Administrator',
        ),
        control: _control(),
      );
      final roles = bridge.control['roles']! as Map<String, Object?>;
      roles['custom'] = <String, Object?>{
        'roleId': 'custom',
        'displayName': 'Custom identity',
        'capabilities': _userCapabilities,
      };
      await mountNetworkPanel(tester, bridge, width: 360, platform: platform);
      await tester.tap(find.text('身份权限'));
      await tester.pumpAndSettle();
      final actions = find.descendant(
        of: find.byKey(const ValueKey('space-role-custom')),
        matching: find.byType(IconButton),
      );
      expect(actions, findsNWidgets(2));
      for (var index = 0; index < 2; index++) {
        expect(tester.getSize(actions.at(index)), const Size(32, 32));
      }
      expect(tester.takeException(), isNull);
      await unmount(tester, bridge);
    });
  }

  testWidgets('members without capabilities only see their own identity', (
    tester,
  ) async {
    final bridge = DeviceSpacePanelBridge(
      topology: _topology(),
      control: _control(),
    );
    await mountNetworkPanel(tester, bridge);
    expect(find.text('本机身份'), findsOneWidget);
    expect(find.text('普通用户'), findsOneWidget);
    for (final label in <String>['查看设备与当前身份', '中继网络流量', '承载 Core 执行']) {
      expect(find.text(label), findsOneWidget);
    }
    expect(find.text('身份权限'), findsNothing);
    expect(find.text('新建身份'), findsNothing);
    expect(find.text('设置设备身份'), findsNothing);
    expect(find.text('操作记录'), findsNothing);
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });

  testWidgets(
    'audit shows readable results newest-first with details on demand',
    (tester) async {
      final bridge =
          DeviceSpacePanelBridge(
              topology: _topology(
                selfCapabilities: _administratorCapabilities,
                selfIdentityName: 'Administrator',
              ),
              control: _control(),
            )
            ..audit = <Map<String, Object?>>[
              _auditRecord(
                'older',
                'Define role Reader',
                accepted: true,
                time: 1000,
              ),
              _auditRecord(
                'newer',
                'Set identity Edge on android-tablet',
                accepted: false,
                time: 2000,
              ),
            ];
      await mountNetworkPanel(tester, bridge);
      expect(find.text('已执行'), findsNothing);
      await tester.tap(find.text('操作记录'));
      await tester.pumpAndSettle();
      expect(find.text('已执行'), findsOneWidget);
      expect(find.text('已拒绝'), findsOneWidget);
      expect(find.text('android-tablet · 边缘节点'), findsOneWidget);
      expect(find.text('Set identity Edge on android-tablet'), findsNothing);
      expect(
        find.textContaining('issuer lacks required capability'),
        findsNothing,
      );
      expect(find.textContaining('已撤销'), findsNothing);
      expect(find.byType(Card), findsNothing);
      expect(
        tester.getTopLeft(find.byKey(const ValueKey('space-audit-newer'))).dy,
        lessThan(
          tester.getTopLeft(find.byKey(const ValueKey('space-audit-older'))).dy,
        ),
      );
      await tester.tap(find.byKey(const ValueKey('space-audit-newer')));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.text('操作设备: this-linux'), findsOneWidget);
      expect(
        find.textContaining('issuer lacks required capability'),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
      await unmount(tester, bridge);
    },
  );

  testWidgets('long audit history has its own bounded scroll area', (
    tester,
  ) async {
    final bridge =
        DeviceSpacePanelBridge(
            topology: _topology(
              selfCapabilities: _administratorCapabilities,
              selfIdentityName: 'Administrator',
            ),
            control: _control(),
          )
          ..audit = List.generate(
            30,
            (index) => _auditRecord(
              '$index',
              'Disconnect device tablet-$index',
              time: index * 1000,
            ),
          );
    await mountNetworkPanel(tester, bridge);
    await tester.tap(find.text('操作记录'));
    await tester.pumpAndSettle();
    final history = find.descendant(
      of: find.byType(NetworkControlPanel),
      matching: find.byType(ListView),
    );
    expect(history, findsOneWidget);
    expect(tester.getSize(history).height, lessThanOrEqualTo(320));
    expect(find.text('tablet-29'), findsOneWidget);
    await tester.drag(history, const Offset(0, -4000));
    await tester.pumpAndSettle();
    expect(find.text('tablet-0'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });

  for (final brightness in Brightness.values) {
    testWidgets(
      'expanded sections fit narrow screens and large text in ${brightness.name} mode',
      (tester) async {
        final bridge =
            DeviceSpacePanelBridge(
                topology: _topology(
                  selfCapabilities: _administratorCapabilities,
                  selfIdentityName: 'Administrator',
                ),
                control: _control(),
              )
              ..audit = <Map<String, Object?>>[
                _auditRecord(
                  'one',
                  'Set identity Edge on tablet-with-a-long-name',
                ),
              ];
        await mountNetworkPanel(
          tester,
          bridge,
          width: 360,
          textScale: 1.5,
          brightness: brightness,
        );
        final refresh = find.byKey(const ValueKey('space-identity-refresh'));
        final heading = find.text('本机身份');
        expect(tester.getSize(refresh), const Size(32, 32));
        expect(
          tester.getTopLeft(refresh).dx,
          greaterThan(tester.getTopRight(heading).dx),
        );
        expect(
          (tester.getTopLeft(refresh).dy - tester.getTopLeft(heading).dy).abs(),
          lessThan(12),
        );
        await tester.tap(find.text('身份权限'));
        await tester.pumpAndSettle();
        await tester.ensureVisible(find.text('操作记录'));
        await tester.tap(find.text('操作记录'));
        await tester.pumpAndSettle();
        expect(find.text('已执行'), findsOneWidget);
        expect(tester.takeException(), isNull);
        await unmount(tester, bridge);
      },
    );
  }

  testWidgets('new identity action does not toggle the identity list', (
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
    await tester.tap(find.text('新建身份'));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsOneWidget);
    expect(find.byKey(const ValueKey('space-role-admin')), findsNothing);
    expect(tester.takeException(), isNull);
    await unmount(tester, bridge);
  });
}

/// Builds one wire-format history record with a meaningful rejection reason.
Map<String, Object?> _auditRecord(
  String id,
  String summary, {
  bool accepted = true,
  int time = 1791676800000,
}) => <String, Object?>{
  'commandId': id,
  'spaceId': 'space',
  'issuerNodeId': 'self',
  'originNodeId': 'self',
  'accepted': accepted,
  'reason': accepted
      ? 'accepted'
      : 'issuer lacks required capability network.identity.assign',
  'summary': summary,
  'recordedAt': time,
};
