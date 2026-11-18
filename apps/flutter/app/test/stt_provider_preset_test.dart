import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/settings/tts/TtsSettingsPanel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Exercises creation of a file-only speech recognition preset through settings.
void main() {
  testWidgets('DeepSeek saves without a model and keeps selection explicit', (
    tester,
  ) async {
    final bridge = _SttPresetBridge();
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(800, 1200);
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
            child: SttProviderSection(
              clients: GeneratedCoreProxyClients(bridge),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('添加 STT 供应商'));
    await tester.pumpAndSettle();
    expect(find.text('此接口无需指定模型'), findsOneWidget);
    final apiKey = find.byWidgetPredicate(
      (widget) =>
          widget is TextField && widget.decoration?.labelText == 'API Key',
    );
    await tester.enterText(apiKey, 'test-key');
    await tester.tap(find.widgetWithText(FilledButton, '保存'));
    await tester.pumpAndSettle();

    final config = bridge.saved!;
    expect(config.providerType, 'DEEPSEEK_STT');
    expect(config.model, isEmpty);
    expect(config.modelFieldName, isEmpty);
    expect(config.languageFieldName, isEmpty);
    expect(config.endpoint, 'https://api.deepseek.com/v1/tools/asr');
    expect(config.apiKey, 'test-key');
    expect(bridge.methods, isNot(contains('setCurrentSttConfigId')));
    expect(tester.takeException(), isNull);
  });
}

class _SttPresetBridge extends OperitRuntimeBridge {
  final methods = <String>[];
  core.SttConfig? saved;

  /// Serves a file-only catalog and captures the configuration submitted by settings.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    methods.add(request.methodName);
    final Object? result;
    switch (request.methodName) {
      case 'getAllSttConfigs':
        result = saved == null ? <Object?>[] : <Object?>[saved!.toJson()];
      case 'getSelectedSttConfigId':
        result = null;
      case 'getProviderCatalogEntries':
        result = <Object?>[
          const core.SttProviderCatalogEntry(
            providerTypeId: 'DEEPSEEK_STT',
            displayName: 'DeepSeek',
            defaultEndpoint: 'https://api.deepseek.com/v1/tools/asr',
            defaultModel: '',
            defaultFileFieldName: 'file',
            defaultModelFieldName: '',
            defaultLanguageFieldName: '',
            defaultResponseTextJsonPath: r'$.text',
            defaultHeaders: [],
          ).toJson(),
        ];
      case 'getAvailableSttModels':
        result = <Object?>[];
      case 'createSttConfig':
        final json = Map<String, Object?>.from(
          (request.args as Map)['config'] as Map,
        );
        json['id'] = 'saved-stt';
        saved = core.SttConfig.fromJson(json);
        result = saved!.toJson();
      default:
        throw UnsupportedError('Unexpected Core call: ${request.methodName}');
    }
    return encodeCoreLink(<Object?>[0, result]);
  }

  /// Rejects push streams because this settings workflow only uses direct calls.
  @override
  Future<CorePushSink> push(CorePushRequest request) {
    throw UnsupportedError('Push streams are not used by this test');
  }

  /// Rejects watch snapshots because this settings workflow only uses direct calls.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) {
    throw UnsupportedError('Watch snapshots are not used by this test');
  }

  /// Rejects watch streams because this settings workflow only uses direct calls.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    throw UnsupportedError('Watch streams are not used by this test');
  }
}
