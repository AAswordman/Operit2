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
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/settings/model/ModelSettingsPanel.dart';
import 'package:operit2/ui/features/chat/components/style/input/agent/AgentChatInputSection.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/features/settings/model/ProviderLogo.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  setUp(ClientLogger.initialize);
  testWidgets(
    'inline capability saves refresh the outer function model snapshot',
    (tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(1000, 1600);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final bridge = _ModelSettingsBridge();
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
      final closeTooltip = MaterialLocalizations.of(
        panelKey.currentContext!,
      ).closeButtonTooltip;
      final originalSnapshot = await panelKey.currentState!.loadFuture;
      expect(
        originalSnapshot.summaries.single.capabilities.directImage,
        isFalse,
      );

      await tester.tap(find.text('Test Provider'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('test-model').last);
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.settingsModelDirectImage).last);
      await tester.ensureVisible(find.widgetWithText(FilledButton, l10n.save));
      await tester.tap(find.widgetWithText(FilledButton, l10n.save));
      await tester.pumpAndSettle();
      expect(bridge.capabilities.directImage, isTrue);
      final updatedSnapshot = await panelKey.currentState!.loadFuture;
      expect(updatedSnapshot.summaries.single.capabilities.directImage, isTrue);
      expect(updatedSnapshot.currentConfig!.capabilities.directImage, isTrue);

      await tester.tap(find.byTooltip(closeTooltip).last);
      await tester.pumpAndSettle();
      await tester.ensureVisible(
        find.text(l10n.settingsModelFunctionMappingsSection),
      );
      await tester.tap(find.text(l10n.settingsModelFunctionMappingsSection));
      await tester.pumpAndSettle();
      final advanced = find.text('更多高级与多模态配置 (6)');
      await tester.ensureVisible(advanced);
      await tester.tap(advanced);
      await tester.pumpAndSettle();
      expect(
        find.text(l10n.settingsModelFunctionImageUnsupported),
        findsNothing,
      );
      expect(tester.takeException(), isNull);
    },
  );

  for (final missingProvider in [false, true]) {
    testWidgets(
      'model settings keeps an orphaned ${missingProvider ? 'provider' : 'model'} binding repairable',
      (tester) async {
        final bridge = _ModelSettingsBridge()
          ..chatBinding = core.FunctionModelBinding(
            providerId: missingProvider
                ? 'removed-provider'
                : _ModelSettingsBridge.binding.providerId,
            modelId: 'deepseek-flash',
            followsChat: false,
          );
        final panelKey = GlobalKey<ModelSettingsPanelState>();
        await _pumpModelSettings(tester, bridge, panelKey);
        final snapshot = await panelKey.currentState!.loadFuture;
        expect(snapshot.currentConfig, isNull);
        expect(snapshot.chatBinding.toJson(), bridge.chatBinding.toJson());
        expect(snapshot.providers, isNotEmpty);
        expect(bridge.resolutionCalls, 0);
        final l10n = AppLocalizations.of(panelKey.currentContext!)!;
        expect(
          find.text(
            l10n.settingsModelFunctionMappingsMissing(
              bridge.chatBinding.providerId,
              bridge.chatBinding.modelId,
            ),
          ),
          findsWidgets,
        );
        expect(find.text('Test Provider'), findsOneWidget);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets(
    'model settings tolerates a model disappearing between snapshot calls',
    (tester) async {
      final bridge = _ModelSettingsBridge()
        ..resolutionError = 'model not found: test-provider:test-model';
      final panelKey = GlobalKey<ModelSettingsPanelState>();
      await _pumpModelSettings(tester, bridge, panelKey);
      expect((await panelKey.currentState!.loadFuture).currentConfig, isNull);
      expect(bridge.resolutionCalls, 1);
      expect(find.text('Test Provider'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'model settings does not suppress unrelated configuration errors',
    (tester) async {
      final bridge = _ModelSettingsBridge();
      final key = GlobalKey<ModelSettingsPanelState>();
      await _pumpModelSettings(tester, bridge, key);
      bridge.resolutionError = 'missing model context: test-model';
      await expectLater(
        key.currentState!.load(),
        throwsA(isA<CoreLinkError>()),
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'composer derives its label and recovers when model profiles arrive later',
    (tester) async {
      final bridge = _ObservableModelSettingsBridge()
        ..chatBinding = const core.FunctionModelBinding(
          providerId: 'test-provider',
          modelId: 'deepseek-flash',
          followsChat: false,
        );
      addTearDown(bridge.close);
      final controller = TextEditingController();
      final focus = FocusNode();
      final viewModel = ChatViewModel(bridge: bridge);
      addTearDown(controller.dispose);
      addTearDown(focus.dispose);
      await tester.pumpWidget(
        _themed(
          AgentChatInputSection(
            controller: controller,
            focusNode: focus,
            isLoading: false,
            inputState: core.InputProcessingState.idle(),
            viewModel: viewModel,
            currentChatId: 'model-sync-chat',

            onSendMessage: () {},
            onQueueMessage: () {},
            onCancelMessage: () {},
            isSpeechRecording: false,
            isSpeechTranscribing: false,
            onSpeechInput: () {},
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('deepseek-flash'), findsOneWidget);
      ProviderLogo logo() =>
          tester.widget<ProviderLogo>(find.byType(ProviderLogo).first);
      expect(logo().providerTypeId, isEmpty);
      expect(bridge.resolutionCalls, 0);
      expect(tester.takeException(), isNull);

      bridge.modelId = bridge.chatBinding.modelId;
      bridge.emitProfiles();
      await tester.pumpAndSettle();
      expect(find.text('deepseek-flash'), findsOneWidget);
      expect(logo().providerTypeId, 'OPENAI');
      expect(logo().fallbackName, 'Test Provider');
      expect(bridge.resolutionCalls, 0);

      bridge.emitBindings(const {});
      await tester.pumpAndSettle();
      expect(find.text('deepseek-flash'), findsNothing);
      expect(find.byType(ProviderLogo), findsNothing);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      expect(bridge.hasListeners, isFalse);
    },
  );
}

Widget _themed(Widget child) => OperitTheme(
  initialThemePreferenceSnapshot:
      UserPreferencesManager.defaultThemePreferenceSnapshot,
  initialThemeIsReady: false,
  unconfiguredChildEnabled: true,
  hostInteractionHostsEnabled: false,
  child: Scaffold(body: child),
);

Future<void> _pumpModelSettings(
  WidgetTester tester,
  _ModelSettingsBridge bridge,
  GlobalKey<ModelSettingsPanelState> key,
) async {
  await tester.pumpWidget(
    _themed(
      ModelSettingsPanel(key: key, clients: GeneratedCoreProxyClients(bridge)),
    ),
  );
  await tester.pumpAndSettle();
}

class _ModelSettingsBridge extends OperitRuntimeBridge {
  core.FunctionModelBinding chatBinding = binding;
  String modelId = binding.modelId;
  String? resolutionError;
  int resolutionCalls = 0;
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
    id: modelId,
    pricingOverride: null,
    contextOverride: contextSpec,
    capabilitiesOverride: capabilities,
    builtinToolsOverride: const [],
    requestOverride: null,
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
    modelId: modelId,
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
    request: core.ModelRequestSpec.fromJson(const {
      'supportsStructuredTools': true,
    }),
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
        result = chatBinding.toJson();
      case 'getProviderProfiles':
        result = [provider.toJson()];
      case 'getAllModelSummaries':
        result = [
          core.ProviderModelSummary(
            providerId: binding.providerId,
            providerName: provider.name,
            providerTypeId: provider.providerTypeId,
            endpoint: provider.endpoint,
            modelId: modelId,
            capabilities: capabilities,
            pricing: null,
          ).toJson(),
        ];
      case 'getResolvedModelConfig':
        resolutionCalls++;
        if (resolutionError != null) {
          return encodeCoreLink([
            1,
            'COMMAND_ERROR',
            resolutionError,
            null,
            null,
            null,
          ]);
        }
        result = config.toJson();
      case 'updateCapabilitiesForModel':
        capabilities = core.ModelCapabilities.fromJson(
          (request.args as Map)['capabilities'] as Map<String, Object?>,
        );
        result = model.toJson();
      case 'updateContextForModel':
      case 'updateSummaryForModel':
      case 'updateBuiltinToolsForModel':
        result = model.toJson();
      case 'getAllTtsConfigs':
      case 'getAllSttConfigs':
      case 'getAllCharacterCards':
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
            type.toJson(): chatBinding.toJson(),
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

class _ObservableModelSettingsBridge extends _ModelSettingsBridge {
  final _bindings = StreamController<Object?>.broadcast();
  final _profiles = StreamController<Object?>.broadcast();

  bool get hasListeners => _bindings.hasListener || _profiles.hasListener;
  void emitBindings(
    Map<core.FunctionType, core.FunctionModelBinding> bindings,
  ) => _bindings.add({
    for (final entry in bindings.entries)
      entry.key.toJson(): entry.value.toJson(),
  });
  void emitProfiles() => _profiles.add([provider.toJson()]);
  Future<void> close() async {
    await _bindings.close();
    await _profiles.close();
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    switch (request.propertyName) {
      case 'functionModelBindingFlow':
        return _observe(request, {
          for (final type in core.FunctionType.values)
            type.toJson(): chatBinding.toJson(),
        }, _bindings.stream);
      case 'getProviderProfilesFlow':
        return _observe(request, [provider.toJson()], _profiles.stream);
      default:
        return super.watchStream(request);
    }
  }

  Stream<CoreEvent> _observe(
    CoreWatchRequest request,
    Object? initial,
    Stream<Object?> changes,
  ) {
    CoreEvent snapshot(Object? value) => CoreEvent.raw(
      requestId: request.requestId,
      target: request.target,
      propertyName: request.propertyName,
      kind: 'Snapshot',
      valueBytes: encodeCoreLink(value),
      decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
    );
    return Stream<CoreEvent>.multi((events) {
      final subscription = changes.listen(
        (value) => events.add(snapshot(value)),
        onError: events.addError,
        onDone: events.close,
      );
      events.add(snapshot(initial));
      events.onCancel = subscription.cancel;
    });
  }
}
