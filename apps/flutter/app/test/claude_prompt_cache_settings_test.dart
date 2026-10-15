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
import 'package:operit2/ui/features/settings/model/ModelSettingsPanel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  test('legacy request settings keep the default cache TTL', () {
    final settings = core.ModelRequestSpec.fromJson(const {
      'supportsStructuredTools': true,
    });
    expect(settings.enableClaude1HPromptCache, isFalse);
  });

  for (final providerType in [
    core.ApiProviderType.anthropic,
    core.ApiProviderType.anthropicGeneric,
    core.ApiProviderType.openai,
  ]) {
    testWidgets('Claude cache setting for $providerType persists and reloads', (
      tester,
    ) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(1000, 1600);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final bridge = _ClaudeSettingsBridge(providerType);
      final panelKey = GlobalKey<ModelSettingsPanelState>();
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot:
              UserPreferencesManager.defaultThemePreferenceSnapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: Scaffold(
            body: ModelSettingsPanel(
              key: panelKey,
              clients: GeneratedCoreProxyClients(bridge),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = AppLocalizations.of(panelKey.currentContext!)!;
      await tester.tap(find.text('Test Provider'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('test-model').last);
      await tester.pumpAndSettle();
      final cacheChip = find.textContaining(RegExp(r'Claude (1小时缓存|1h cache)'));
      if (providerType == core.ApiProviderType.openai) {
        expect(cacheChip, findsNothing);
        return;
      }
      expect(cacheChip, findsOneWidget);
      final save = find.widgetWithText(FilledButton, l10n.save);
      // Saving unrelated settings must not introduce an extra request update.
      await tester.ensureVisible(save);
      await tester.tap(save);
      await tester.pumpAndSettle();
      expect(bridge.requestSaves, 0);
      await tester.ensureVisible(cacheChip);
      await tester.tap(cacheChip);
      await tester.ensureVisible(save);
      await tester.tap(save);
      await tester.pumpAndSettle();
      expect(bridge.requestSaves, 1);
      expect(bridge.requestSettings.enableClaude1HPromptCache, isTrue);
      expect(bridge.requestSettings.supportsStructuredTools, isFalse);
      final snapshot = await panelKey.currentState!.loadFuture;
      expect(snapshot.currentConfig!.request.enableClaude1HPromptCache, isTrue);

      final closeTooltip = MaterialLocalizations.of(
        panelKey.currentContext!,
      ).closeButtonTooltip;
      await tester.tap(find.byTooltip(closeTooltip).last);
      await tester.pumpAndSettle();
      await tester.tap(find.text('Test Provider'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('test-model').last);
      await tester.pumpAndSettle();
      // Reloading the form must retain true; toggling now saves false.
      await tester.ensureVisible(cacheChip);
      await tester.tap(cacheChip);
      await tester.ensureVisible(save);
      await tester.tap(save);
      await tester.pumpAndSettle();
      expect(bridge.requestSaves, 2);
      expect(bridge.requestSettings.enableClaude1HPromptCache, isFalse);
      expect(bridge.requestSettings.supportsStructuredTools, isFalse);
      expect(tester.takeException(), isNull);
    });
  }
}

class _ClaudeSettingsBridge extends OperitRuntimeBridge {
  _ClaudeSettingsBridge(this.providerType);

  final core.ApiProviderType providerType;
  core.ModelRequestSpec requestSettings = const core.ModelRequestSpec(
    supportsStructuredTools: false,
    enableClaude1HPromptCache: false,
  );
  int requestSaves = 0;

  core.ModelCapabilities capabilities = const core.ModelCapabilities(
    directImage: false,
    directAudio: false,
    directVideo: false,
    toolCall: true,
  );
  static const binding = core.FunctionModelBinding(
    providerId: 'test-provider',
    modelId: 'test-model',
    followsChat: false,
  );
  static const summarySettings = core.ModelSummarySettings(
    enableSummary: true,
    summaryTokenThreshold: 0.7,
    enableSummaryByMessageCount: false,
    summaryMessageCountThreshold: 0,
  );
  static const localRuntime = core.LocalModelRuntimeSettings(
    mnnForwardType: 0,
    mnnThreadCount: 4,
    llamaThreadCount: 4,
    llamaContextSize: 2048,
    llamaBatchSize: 512,
    llamaUBatchSize: 512,
    llamaGpuLayers: 0,
    llamaUseMmap: true,
    llamaFlashAttention: false,
    llamaKvUnified: false,
    llamaOffloadKqv: false,
  );
  static const contextSpec = core.ModelContextSpec(maxContextLength: 200);

  core.ModelProfile get model => core.ModelProfile(
    id: binding.modelId,
    pricingOverride: null,
    contextOverride: contextSpec,
    capabilitiesOverride: capabilities,
    builtinToolsOverride: const [],
    requestOverride: requestSettings,
    parameters: const [],
    summary: summarySettings,
    localRuntime: localRuntime,
  );

  core.ProviderProfile get provider => core.ProviderProfile(
    id: binding.providerId,
    name: 'Test Provider',
    providerTypeId: providerType == core.ApiProviderType.anthropic
        ? 'ANTHROPIC'
        : providerType == core.ApiProviderType.anthropicGeneric
        ? 'ANTHROPIC_GENERIC'
        : 'OPENAI',
    providerType: providerType,
    endpoint: 'https://example.com/v1',
    apiKey: '',
    useMultipleApiKeys: false,
    apiKeyPool: const [],
    currentKeyIndex: 0,
    keyRotationMode: 'ROUND_ROBIN',
    customHeaders: '{}',
    requestLimitPerMinute: 0,
    maxConcurrentRequests: 1,
    thinkingConfigurations: '[]',
    thinkingOptionId: '',
    models: [model],
  );

  core.ResolvedModelConfig get config => core.ResolvedModelConfig(
    providerId: binding.providerId,
    providerName: provider.name,
    modelId: binding.modelId,
    apiKey: '',
    apiEndpoint: provider.endpoint,
    apiProviderType: provider.providerType,
    apiProviderTypeId: provider.providerTypeId,
    useMultipleApiKeys: false,
    apiKeyPool: const [],
    currentKeyIndex: 0,
    keyRotationMode: provider.keyRotationMode,
    customHeaders: '{}',
    requestLimitPerMinute: 0,
    maxConcurrentRequests: 1,
    pricing: null,
    context: contextSpec,
    capabilities: capabilities,
    builtinTools: const [],
    request: requestSettings,
    parameters: const [],
    thinkingConfigurations: '[]',
    thinkingOptionId: '',
    summary: summarySettings,
    localRuntime: localRuntime,
  );

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    final Object? result;
    switch (request.methodName) {
      case 'getModelBindingForFunction':
        result = binding.toJson();
      case 'getProviderProfiles':
        result = [provider.toJson()];
      case 'getAllModelSummaries':
        result = [
          core.ProviderModelSummary(
            providerId: binding.providerId,
            providerName: provider.name,
            providerTypeId: provider.providerTypeId,
            endpoint: provider.endpoint,
            modelId: binding.modelId,
            capabilities: capabilities,
            pricing: null,
          ).toJson(),
        ];
      case 'getResolvedModelConfig':
        result = config.toJson();
      case 'updateCapabilitiesForModel':
        capabilities = core.ModelCapabilities.fromJson(
          (request.args as Map)['capabilities'] as Map<String, Object?>,
        );
        result = model.toJson();
      case 'updateRequestForModel':
        requestSettings = core.ModelRequestSpec.fromJson(
          (request.args as Map)['request'] as Map<String, Object?>,
        );
        requestSaves += 1;
        result = model.toJson();
      case 'updateContextForModel':
      case 'updateSummaryForModel':
      case 'updateBuiltinToolsForModel':
        result = model.toJson();
      case 'getAllTtsConfigs':
      case 'getAllSttConfigs':
        result = <Object?>[];
      case 'getCurrentTtsConfigId':
      case 'getSelectedSttConfigId':
        result = '';
      case 'getProviderCatalogEntries':
        result = request.target.contains('Stt')
            ? [
                const core.SttProviderCatalogEntry(
                  providerTypeId: 'OPENAI_STT',
                  displayName: 'STT',
                  defaultEndpoint: '',
                  defaultModel: '',
                  defaultFileFieldName: 'file',
                  defaultModelFieldName: 'model',
                  defaultLanguageFieldName: 'language',
                  defaultResponseTextJsonPath: 'text',
                  defaultHeaders: [],
                ).toJson(),
              ]
            : [
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
      default:
        throw UnsupportedError('Unexpected Core call: ${request.methodName}');
    }
    return encodeCoreLink(<Object?>[0, result]);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    final Object? value;
    switch (request.propertyName) {
      case 'functionModelBindingFlow':
        value = {
          for (final type in core.FunctionType.values)
            type.toJson(): binding.toJson(),
        };
      case 'maxImageHistoryUserTurnsFlow':
      case 'maxMediaHistoryUserTurnsFlow':
        value = 5;
      default:
        throw UnsupportedError(
          'Unexpected Core watch: ${request.propertyName}',
        );
    }
    return Stream.value(
      CoreEvent.raw(
        requestId: request.requestId,
        target: request.target,
        propertyName: request.propertyName,
        kind: 'Snapshot',
        valueBytes: encodeCoreLink(value),
        decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
      ),
    );
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      watchStream(request).first;

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnsupportedError('Push streams are not used by this test');
}
