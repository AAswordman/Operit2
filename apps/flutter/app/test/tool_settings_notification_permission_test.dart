import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/host/AppToastController.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/settings/tools/ToolSettingsPanel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  const channel = MethodChannel('operit/runtime');
  var status = 'Missing';
  var failRequest = false;
  late List<MethodCall> requests;

  setUpAll(ClientLogger.initialize);
  setUp(() {
    status = 'Missing';
    failRequest = false;
    requests = [];
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          switch (call.method) {
            case 'hostOnboardingPermissionSnapshot':
              expectSync(call.arguments, {'hostId': 'macos'});
              return {
                'macos.notifications': {
                  'id': 'macos.notifications',
                  'status': status,
                },
              };
            case 'hostOnboardingRequestPermission':
              requests.add(call);
              if (failRequest) {
                throw PlatformException(
                  code: 'MACOS_NOTIFICATION_PERMISSION_ERROR',
                  message: 'Cannot request notifications',
                );
              }
              status = 'Satisfied';
              return null;
            default:
              return null;
          }
        });
  });
  tearDown(() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  Future<void> pumpSettings(WidgetTester tester) async {
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: Scaffold(
          body: ToolSettingsPanel(
            clients: GeneratedCoreProxyClients(_SettingsBridge()),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('notifications use the shared system authorization UI', (
    tester,
  ) async {
    await pumpSettings(tester);
    expect(find.text('系统授权'), findsOneWidget);
    expect(find.text('通知'), findsOneWidget);
    expect(find.text('未授予'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, '授予'));
    await tester.pumpAndSettle();

    expect(requests.single.arguments, {
      'hostId': 'macos',
      'requirementId': 'macos.notifications',
    });
    expect(find.text('未授予'), findsNothing);
    expect(find.widgetWithText(FilledButton, '已授予'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('returning from system settings refreshes the authorization', (
    tester,
  ) async {
    await pumpSettings(tester);
    expect(find.text('未授予'), findsOneWidget);

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    status = 'Satisfied';
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();

    expect(find.text('未授予'), findsNothing);
    expect(find.widgetWithText(FilledButton, '已授予'), findsOneWidget);
    expect(requests, isEmpty);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('system request errors stay in the authorization page', (
    tester,
  ) async {
    failRequest = true;
    await pumpSettings(tester);
    await tester.tap(find.widgetWithText(FilledButton, '授予'));
    await tester.pumpAndSettle();

    expect(find.text('未授予'), findsOneWidget);
    expect(find.textContaining('授权失败：'), findsOneWidget);
    AppToastController.instance.clearSnackBars();
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}

class _SettingsBridge extends OperitRuntimeBridge {
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    final Object value = switch (request.methodName) {
      'runtimeHostDescriptor' => {
        'id': 'macos',
        'displayName': 'macOS',
        'platform': 'Macos',
        'privilege': 'Normal',
        'isolation': 'None',
        'pathStyleDescriptionEn': '',
        'pathStyleDescriptionCn': '',
        'examplePaths': <Object?>[],
        'usesEnvironmentParameter': false,
        'environmentParameterDescriptionEn': '',
        'environmentParameterDescriptionCn': '',
        'capabilities': ['system.notifications.send'],
        'structuredCapabilities': <Object?>[],
        'workspaceRoots': <Object?>[],
        for (final field in [
          'fileSystemHost',
          'webVisitHost',
          'systemOperationHost',
          'audioPlaybackHost',
          'ttsSynthesisHost',
          'ttsPlaybackHost',
          'systemTtsPlaybackHost',
          'managedRuntimeHost',
          'runtimeStorageHost',
          'runtimeSqliteHost',
          'browserAutomationHost',
          'composeDslWebViewHost',
          'terminalHost',
          'hostRuntimeEventHost',
        ])
          field: true,
        'onboardingRequirements': [
          {
            'id': 'macos.notifications',
            'title': '通知',
            'description': '允许 Operit 发送回复完成和工具批准通知。',
            'capabilityIds': ['system.notifications.send'],
            'isRequired': true,
            'status': 'Missing',
            'action': 'RuntimePermission',
          },
        ],
      },
      'getAiPermissionMode' => 'ReadOnly',
      'getMcpStartupTimeoutSeconds' => 30,
      'getToolPkgPreHookTimeoutSeconds' => 30,
      _ => throw StateError('Unexpected Core call: ${request.methodName}'),
    };
    return encodeCoreLink([0, value]);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      const Stream<CoreEvent>.empty();

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();
}
