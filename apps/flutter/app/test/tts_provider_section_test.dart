import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/components/M3LoadingIndicator.dart';
import 'package:operit2/ui/features/settings/tts/TtsSettingsPanel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies global voice selection preserves the existing provider widget tree.
void main() {
  testWidgets(
    'global selection keeps the section and both providers expanded',
    (tester) async {
      final bridge = _TtsSettingsBridge();
      await _pumpSection(tester, bridge);
      await tester.tap(find.text('TTS 供应商'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('SiliconFlow'));
      await tester.pumpAndSettle();

      final sectionState = tester.state(find.byType(ExpansionTile));
      final sectionSize = tester.getSize(find.byType(TtsProviderSection));
      final initialReads = bridge.calls.length;
      expect(find.text('remote-model · remote-voice'), findsOneWidget);
      expect(find.text('系统默认音色'), findsOneWidget);

      await tester.tap(find.text('设为全局'));
      await tester.pump();
      expect(find.byType(M3LoadingIndicator), findsNothing);
      expect(find.text('remote-model · remote-voice'), findsOneWidget);
      expect(find.text('系统默认音色'), findsOneWidget);
      expect(bridge.calls, hasLength(initialReads + 1));
      expect(bridge.calls.last.methodName, 'setCurrentTtsConfigId');
      expect(bridge.calls.last.args, <String, Object?>{'id': 'remote'});

      bridge.completeSelection();
      await tester.pump();
      expect(find.byType(M3LoadingIndicator), findsNothing);
      await tester.pumpAndSettle();

      expect(tester.state(find.byType(ExpansionTile)), same(sectionState));
      expect(tester.getSize(find.byType(TtsProviderSection)), sectionSize);
      expect(find.text('remote-model · remote-voice'), findsOneWidget);
      expect(find.text('系统默认音色'), findsOneWidget);
      expect(bridge.calls, hasLength(initialReads + 1));
      expect(
        tester.getTopLeft(find.text('全局当前')).dy,
        lessThan(tester.getTopLeft(find.text('系统 TTS')).dy),
      );
      expect(tester.takeException(), isNull);

      await tester.tap(find.text('设为全局'));
      bridge.completeSelection();
      await tester.pumpAndSettle();
      expect(bridge.calls, hasLength(initialReads + 2));
      expect(
        tester.getTopLeft(find.text('全局当前')).dy,
        greaterThan(tester.getTopLeft(find.text('系统 TTS')).dy),
      );
      expect(tester.getSize(find.byType(TtsProviderSection)), sectionSize);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('selection does not reopen a manually collapsed provider', (
    tester,
  ) async {
    final bridge = _TtsSettingsBridge();
    await _pumpSection(tester, bridge);
    await tester.tap(find.text('TTS 供应商'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('系统 TTS'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('SiliconFlow'));
    await tester.pumpAndSettle();
    expect(find.text('系统默认音色'), findsNothing);

    await tester.tap(find.text('设为全局'));
    bridge.completeSelection();
    await tester.pumpAndSettle();

    expect(find.text('remote-model · remote-voice'), findsOneWidget);
    expect(find.text('系统默认音色'), findsNothing);
    expect(find.text('全局当前'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'inactive TTS deletion uses only the independent configuration manager',
    (tester) async {
      final bridge = _TtsSettingsBridge();
      await _pumpSection(tester, bridge);
      await tester.tap(find.text('TTS 供应商'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('SiliconFlow'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('remote-model · remote-voice'));
      await tester.pumpAndSettle();

      expect(find.text('编辑 TTS 音色'), findsOneWidget);
      expect(find.widgetWithText(TextButton, '删除'), findsOneWidget);
      await tester.tap(find.widgetWithText(TextButton, '删除'));
      await tester.pumpAndSettle();

      final deletions = bridge.calls.where(
        (call) => call.methodName == 'deleteTtsConfig',
      );
      expect(deletions, hasLength(1));
      expect(deletions.single.args, <String, Object?>{'id': 'remote'});
      expect(bridge.configs.map((config) => config.id), <String>['system']);
      expect(bridge.currentConfigId, 'system');
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'the independent current TTS configuration still cannot be deleted',
    (tester) async {
      final bridge = _TtsSettingsBridge();
      await _pumpSection(tester, bridge);
      await tester.tap(find.text('TTS 供应商'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('系统默认音色'));
      await tester.pumpAndSettle();

      expect(find.text('编辑 TTS 音色'), findsOneWidget);
      final l10n = AppLocalizations.of(
        tester.element(find.byType(TtsProviderSection)),
      )!;
      expect(
        find.text(l10n.settingsTtsCurrentConfigCannotDelete),
        findsOneWidget,
      );
      expect(find.widgetWithText(TextButton, '删除'), findsNothing);
      expect(
        bridge.calls.where((call) => call.methodName == 'deleteTtsConfig'),
        isEmpty,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('a pending selection can complete after the section is removed', (
    tester,
  ) async {
    final bridge = _TtsSettingsBridge();
    await _pumpSection(tester, bridge);
    await tester.tap(find.text('TTS 供应商'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('SiliconFlow'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('设为全局'));

    await tester.pumpWidget(const SizedBox.shrink());
    bridge.completeSelection();
    await tester.pump();
    expect(tester.takeException(), isNull);
  });
}

/// Mounts the same initially collapsed TTS section used by model settings.
Future<void> _pumpSection(
  WidgetTester tester,
  _TtsSettingsBridge bridge,
) async {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = const Size(600, 1200);
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(tester.view.resetPhysicalSize);
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: Scaffold(
        body: SingleChildScrollView(
          child: TtsProviderSection(
            clients: GeneratedCoreProxyClients(bridge),
            initiallyExpanded: false,
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

/// Creates a complete voice configuration for the generated Core protocol.
core.TtsConfig _config({
  required String id,
  required String name,
  required String providerType,
  String model = '',
  String voice = '',
}) {
  return core.TtsConfig(
    id: id,
    name: name,
    providerType: providerType,
    endpoint: '',
    apiKey: '',
    model: model,
    voice: voice,
    responseFormat: 'wav',
    speed: 1,
    httpMethod: 'POST',
    requestBody: '',
    contentType: 'application/json',
    headers: const [],
    responsePipeline: const [],
    createdAt: 0,
    updatedAt: 0,
  );
}

/// Implements only independent TTS configuration calls and rejects domain lookups.
class _TtsSettingsBridge extends OperitRuntimeBridge {
  final List<CoreCallRequest> calls = <CoreCallRequest>[];
  final List<core.TtsConfig> configs = <core.TtsConfig>[
    _config(id: 'system', name: '系统 TTS', providerType: 'SYSTEM_TTS'),
    _config(
      id: 'remote',
      name: 'SiliconFlow',
      providerType: 'SILICONFLOW_TTS',
      model: 'remote-model',
      voice: 'remote-voice',
    ),
  ];
  String currentConfigId = 'system';
  late Completer<Uint8List> selection;
  late String requestedConfigId;

  /// Completes persistence separately from the button tap to expose loading frames.
  void completeSelection() {
    currentConfigId = requestedConfigId;
    selection.complete(encodeCoreLink(<Object?>[0, currentConfigId]));
  }

  /// Records exact method calls and returns encoded Core response values.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    final Object? result;
    switch (request.methodName) {
      case 'getAllTtsConfigs':
        result = configs.map((config) => config.toJson()).toList();
      case 'getCurrentTtsConfigId':
        result = currentConfigId;
      case 'getProviderCatalogEntries':
        result = <Object?>[
          const core.TtsProviderCatalogEntry(
            providerTypeId: 'SYSTEM_TTS',
            displayName: '系统 TTS',
            defaultEndpoint: '',
            defaultModel: '',
            defaultResponseFormat: 'wav',
            defaultHttpMethod: 'POST',
            defaultContentType: 'application/json',
            defaultRequestBody: '',
            defaultHeaders: [],
            defaultResponsePipeline: [],
            operations: [],
          ).toJson(),
        ];
      case 'deleteTtsConfig':
        final id = (request.args as Map)['id'] as String;
        if (id == currentConfigId) {
          throw StateError('The current TTS configuration cannot be deleted');
        }
        final count = configs.length;
        configs.removeWhere((config) => config.id == id);
        result = configs.length != count;
      case 'setCurrentTtsConfigId':
        requestedConfigId = (request.args as Map)['id'] as String;
        selection = Completer<Uint8List>();
        return selection.future;
      default:
        throw UnsupportedError('Unexpected Core call: ${request.methodName}');
    }
    return encodeCoreLink(<Object?>[0, result]);
  }

  /// Rejects push streams because voice selection only issues direct calls.
  @override
  Future<CorePushSink> push(CorePushRequest request) {
    throw UnsupportedError('Push streams are not used by this test');
  }

  /// Rejects watch snapshots because this section loads data through direct calls.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) {
    throw UnsupportedError('Watch snapshots are not used by this test');
  }

  /// Rejects watch streams because this section loads data through direct calls.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    throw UnsupportedError('Watch streams are not used by this test');
  }
}
