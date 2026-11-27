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
  testWidgets('provider limit fields reject non-integer text and save integers', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(1000, 1600);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(tester.view.resetPhysicalSize);
    final bridge = _ProviderLimitBridge();
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
    await tester.tap(find.byTooltip(l10n.settingsModelEditProvider));
    await tester.pumpAndSettle();

    final dialog = find.byType(AlertDialog);
    expect(dialog, findsOneWidget);
    await tester.tap(
      find.descendant(of: dialog, matching: find.text(l10n.settingsAdvanced)),
    );
    await tester.pumpAndSettle();

    final limitField = find.descendant(
      of: dialog,
      matching: find.widgetWithText(TextFormField, l10n.settingsModelRequestLimit),
    );
    final concurrentField = find.descendant(
      of: dialog,
      matching: find.widgetWithText(
        TextFormField,
        l10n.settingsModelMaxConcurrent,
      ),
    );
    final saveButton = find.descendant(
      of: dialog,
      matching: find.text(l10n.save),
    );

    // Hardware-keyboard text must not reach int.parse in the save callback.
    await tester.enterText(limitField, 'abc');
    await tester.enterText(concurrentField, '1.5');
    await tester.ensureVisible(saveButton);
    await tester.tap(saveButton);
    await tester.pump();
    expect(tester.takeException(), isNull);
    expect(bridge.savedProvider, isNull);
    expect(dialog, findsOneWidget);
    expect(tester.widget<TextFormField>(concurrentField).controller!.text, '1.5');

    final testButton = find.descendant(
      of: dialog,
      matching: find.text(l10n.settingsModelPoolTest),
    );
    await tester.ensureVisible(testButton);
    await tester.tap(testButton);
    await tester.pump();
    expect(bridge.keyTestCalls, 0);
    await tester.pump(const Duration(seconds: 5));

    // Valid integers still save normally.
    await tester.enterText(limitField, '12');
    await tester.enterText(concurrentField, '3');
    await tester.ensureVisible(saveButton);
    await tester.tap(saveButton);
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(bridge.savedProvider, isNotNull);
    expect(bridge.savedProvider!.requestLimitPerMinute, 12);
    expect(bridge.savedProvider!.maxConcurrentRequests, 3);
  });
}

class _ProviderLimitBridge extends OperitRuntimeBridge {
  core.ProviderProfile? savedProvider;
  int keyTestCalls = 0;

  static const binding = core.FunctionModelBinding(
    providerId: 'test-provider',
    modelId: 'test-model',
    followsChat: false,
  );
  static const capabilities = core.ModelCapabilities(
    directImage: false,
    directAudio: false,
    directVideo: false,
    toolCall: true,
  );
  static const contextSpec = core.ModelContextSpec(maxContextTokens: 200000);
  static const summarySettings = core.ModelSummarySettings(
    enableSummary: true,
    summaryTokenThreshold: 0.7,
  );
  static const requestSettings = core.ModelRequestSpec(
    supportsStructuredTools: false,
    enableClaude1HPromptCache: false,
  );
  static const localRuntime = core.LocalModelRuntimeSettings(
    inferenceBindingKey: '',
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

  core.ModelProfile get model => core.ModelProfile(
    id: binding.modelId,
    pricingOverride: null,
    contextOverride: contextSpec,
    capabilitiesOverride: core.ModelCapabilityOverrides(
      directImage: capabilities.directImage,
      directAudio: capabilities.directAudio,
      directVideo: capabilities.directVideo,
      toolCall: capabilities.toolCall,
    ),
    builtinToolsOverride: const [],
    requestOverride: core.ModelRequestOverrides(
      supportsStructuredTools: requestSettings.supportsStructuredTools,
      enableClaude1HPromptCache: requestSettings.enableClaude1HPromptCache,
    ),
    listed: null,
    thinkingOverride: null,
    thinkingLevel: null,
    parameters: const [],
    summary: summarySettings,
    localRuntime: localRuntime,
  );

  core.ProviderProfile get provider => core.ProviderProfile(
    id: binding.providerId,
    name: 'Test Provider',
    providerTypeId: 'OPENAI',
    providerType: core.ApiProviderType.openai,
    endpoint: 'https://example.com/v1',
    apiKey: '',
    useMultipleApiKeys: true,
    apiKeyPool: const [core.ApiKeyInfo(
      id: 'key-1', key: 'fake-key', name: 'test', isEnabled: true,
      availabilityStatus: core.ApiKeyAvailabilityStatus.untested,
      usageCount: 0, lastUsed: 0, errorCount: 0,
    )],
    currentKeyIndex: 0,
    keyRotationMode: 'ROUND_ROBIN',
    customHeaders: '{}',
    requestLimitPerMinute: 0,
    maxConcurrentRequests: 1,
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
    thinking: const core.ModelThinkingSpec(
      supported: false,
      toggle: false,
      efforts: [],
      budget: null,
    ),
    thinkingLevel: null,
    summary: summarySettings,
    localRuntime: localRuntime,
  );

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    final Object? result;
    switch (request.methodName) {
      case 'testProviderApiKey':
        keyTestCalls++;
        throw StateError('Invalid limits must not trigger a key test');
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
            context: contextSpec,
            pricing: null,
            thinking: const core.ModelThinkingSpec(
              supported: false,
              toggle: false,
              efforts: [],
              budget: null,
            ),
          ).toJson(),
        ];
      case 'getResolvedModelConfig':
        result = config.toJson();
      case 'getThinkingSettingsForModel':
        result = const core.ModelThinkingSettings(
          spec: core.ModelThinkingSpec(
            supported: false,
            toggle: false,
            efforts: [],
            budget: null,
          ),
          source: core.ThinkingSpecSource.unknown,
          catalog: null,
          levels: [],
          current: null,
          chosen: false,
        ).toJson();
      case 'updateProviderProfile':
        savedProvider = core.ProviderProfile.fromJson(
          (request.args as Map)['provider'] as Map<String, Object?>,
        );
        result = savedProvider!.toJson();
      case 'getAllTtsConfigs':
      case 'getAllSttConfigs':
        result = <Object?>[];
      case 'getCurrentTtsConfigId':
      case 'getSelectedSttConfigId':
        result = '';
      case 'getProviderCatalogEntries':
        result = [
          const core.ProviderCatalogEntry(
            providerTypeId: 'OPENAI',
            displayName: 'OpenAI',
            defaultEndpoint: 'https://api.openai.com/v1',
            endpointOptions: [],
            operations: [],
            models: [],
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
