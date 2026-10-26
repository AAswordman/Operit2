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
import 'package:operit2/ui/features/settings/local_models/LocalModelSettingsPanel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  setUp(ClientLogger.initialize);
  for (final size in [const Size(1000, 1200), const Size(390, 1000)]) {
    testWidgets(
      'local LLM inference and chat selection preserve its route at ${size.width}',
      (tester) async {
        tester.view.physicalSize = size;
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final bridge = _Bridge();
        await tester.pumpWidget(
          OperitTheme(
            initialThemePreferenceSnapshot:
                UserPreferencesManager.defaultThemePreferenceSnapshot,
            initialThemeIsReady: false,
            unconfiguredChildEnabled: true,
            hostInteractionHostsEnabled: false,
            child: Scaffold(
              body: LocalModelSettingsPanel(
                clients: GeneratedCoreProxyClients(bridge),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final l10n = AppLocalizations.of(
          tester.element(find.byType(LocalModelSettingsPanel)),
        )!;
        expect(
          find.text(l10n.localModelsDeviceOnlyDescription),
          findsOneWidget,
        );
        await tester.ensureVisible(find.text(l10n.localModelsRunChat));
      await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.localModelsRunChat));
        await tester.pumpAndSettle();
        final prompt = find.byWidgetPredicate(
          (w) =>
              w is TextField &&
              w.decoration?.labelText == l10n.localModelsPrompt,
        );
        await tester.enterText(prompt, 'Hello from this device');
        await tester.tap(find.text(l10n.localModelsGenerate));
        await tester.pumpAndSettle();
        expect(find.text('Generated on the model device'), findsOneWidget);
        expect(bridge.inference?['prompt'], 'Hello from this device');
        final settings = bridge.inference?['settings'] as Map;
        expect(settings['inferenceBindingKey'], 'local-llm:test-node:tiny@v1');
        await tester.tap(find.text(l10n.localModelsUseForChat));
        await tester.pumpAndSettle();
        expect(bridge.saved?['localRuntime'], settings);
        expect(bridge.chat?['providerId'], 'LOCAL_MODEL');
        expect(bridge.chat?['modelId'], 'tiny@v1');
        expect(tester.takeException(), isNull);
        await tester.pump(const Duration(seconds: 5));
        await tester.pumpAndSettle();
        await tester.pumpWidget(const SizedBox.shrink());
        await tester.pumpAndSettle();
      },
    );
  }
}

class _Bridge extends OperitRuntimeBridge {
  Map<String, Object?>? inference, saved, chat;
  static final manifest = core.LocalModelManifest(
    id: 'tiny',
    version: 'v1',
    displayName: 'Tiny Chat GGUF',
    description: 'Offline chat',
    kind: core.LocalModelKind.chat,
    engine: core.LocalEngineKind.llamaCpp,
    license: 'Apache-2.0',
    homepage: '',
    languages: const ['en'],
    tags: const ['llm'],
    engineRequirement: null,
    driver: core.LocalModelDriver.llamaCpp(model: 'tiny.gguf'),
    sources: const [],
    installSource: core.LocalModelInstallSource.files(),
    files: const [],
  );
  static const settings = core.LocalModelRuntimeSettings(
    inferenceBindingKey: 'local-llm:test-node:tiny@v1',
    mnnForwardType: 0,
    mnnThreadCount: 4,
    llamaThreadCount: 4,
    llamaContextSize: 2048,
    llamaBatchSize: 512,
    llamaUBatchSize: 512,
    llamaGpuLayers: 0,
    llamaUseMmap: true,
    llamaFlashAttention: false,
    llamaKvUnified: true,
    llamaOffloadKqv: false,
  );
  Map<String, Object?> get profile => {
    'id': 'tiny@v1',
    'pricingOverride': null,
    'contextOverride': null,
    'capabilitiesOverride': null,
    'builtinToolsOverride': null,
    'requestOverride': null,
    'parameters': <Object?>[],
    'summary': const core.ModelSummarySettings(
      enableSummary: true,
      summaryTokenThreshold: 0.7,
      enableSummaryByMessageCount: false,
      summaryMessageCountThreshold: 0,
    ).toJson(),
    'localRuntime': settings.toJson(),
  };
  core.InstalledLocalModel get installed => core.InstalledLocalModel(
    manifest: manifest,
    storagePath: 'runtime/models/local/chat/tiny',
    installedAtMs: 1,
    verifiedAtMs: 1,
  );
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    Object? result;
    switch (request.methodName) {
      case 'getCatalogStatus':
        result = [
          core.LocalModelCatalogStatus(
            manifest: manifest,
            installedModel: installed,
            engineManifest: null,
            installedEngine: null,
            platformCompatible: true,
            inferenceBindingKey: settings.inferenceBindingKey,
          ).toJson(),
        ];
      case 'getRegistry':
        result = core.LocalModelRegistrySnapshot(
          installedModels: [installed],
          installedEngines: const [],
          preferredSourceKind: null,
          customModels: const [],
        ).toJson();
      case 'getPlatformTarget':
        result = const core.LocalPlatformTarget(
          platform: core.LocalPlatform.windows,
          architecture: core.LocalArchitecture.x8664,
        ).toJson();
      case 'getInstallStatuses':
        result = <Object?>[];
      case 'getModelProfile':
        result = profile;
      case 'runChatInference':
        inference = Map<String, Object?>.from(request.args as Map);
        result = {
          'text': 'Generated on the model device',
          'metadata': <String, Object?>{},
        };
      case 'updateModelProfile':
        saved = Map<String, Object?>.from(
          (request.args as Map)['model'] as Map,
        );
        result = saved;
      case 'setModelForFunction':
        chat = Map<String, Object?>.from(request.args as Map);
        result = null;
      default:
        throw UnsupportedError('Unexpected call: ${request.methodName}');
    }
    return encodeCoreLink(<Object?>[0, result]);
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnsupportedError('unused');
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnsupportedError('unused');
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      const Stream.empty();
}
