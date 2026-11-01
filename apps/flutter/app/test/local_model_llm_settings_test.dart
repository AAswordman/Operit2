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
        expect(tester.takeException(), isNull, reason: 'catalog layout');
        final l10n = AppLocalizations.of(
          tester.element(find.byType(LocalModelSettingsPanel)),
        )!;
        expect(find.text(l10n.localModelsDeviceOnlyDescription), findsNothing);
        await tester.ensureVisible(find.text(l10n.localModelsRunChat));
        await tester.pumpAndSettle();
        await _tapVisible(tester, find.text(l10n.localModelsRunChat));
        await tester.pumpAndSettle();
        expect(
          tester.takeException(),
          isNull,
          reason: 'inference dialog layout',
        );
        final prompt = find.byWidgetPredicate(
          (w) =>
              w is TextField &&
              w.decoration?.labelText == l10n.localModelsPrompt,
        );
        await tester.enterText(prompt, 'Hello from this device');
        await _tapVisible(tester, find.text(l10n.localModelsGenerate));
        await tester.pumpAndSettle();
        expect(find.text('Generated on the model device'), findsOneWidget);
        expect(bridge.inference?['prompt'], 'Hello from this device');
        final settings = bridge.inference?['settings'] as Map;
        expect(settings['inferenceBindingKey'], 'local-llm:test-node:tiny@v1');
        await _tapVisible(tester, find.text(l10n.localModelsUseForChat));
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
  for (final locale in [const Locale('zh'), const Locale('en')]) {
    for (final width in [320.0, 1000.0]) {
      testWidgets('catalog filters, details and import at $width in $locale', (
        tester,
      ) async {
        final bridge = _Bridge()..catalog = _catalog();
        final l10n = await _pumpCatalog(tester, bridge, width, locale);
        expect(l10n.localeName, locale.languageCode);
        expect(tester.takeException(), isNull);
        expect(find.text(l10n.localModelsDeviceOnlyDescription), findsNothing);
        expect(find.text(l10n.localModelsDeviceOnly), findsNothing);
        expect(find.text(l10n.localModelsResultCount(5, 5)), findsOneWidget);

        await _tapVisible(tester, find.text(l10n.localModelsFilterInstalled));
        await tester.pumpAndSettle();
        expect(find.text(l10n.localModelsResultCount(1, 5)), findsOneWidget);
        expect(find.text('Tiny Chat GGUF'), findsOneWidget);
        await _tapVisible(
          tester,
          find.byTooltip(l10n.localModelsMoreActions).first,
        );
        await tester.pumpAndSettle();
        await _tapVisible(tester, find.text(l10n.localModelsDetails));
        await tester.pumpAndSettle();
        expect(find.text('Apache-2.0'), findsOneWidget);
        expect(find.text('tiny @ v1'), findsOneWidget);
        expect(tester.takeException(), isNull);
        await _tapVisible(
          tester,
          find.byTooltip(
            MaterialLocalizations.of(
              tester.element(find.byType(LocalModelSettingsPanel)),
            ).closeButtonTooltip,
          ),
        );
        await tester.pumpAndSettle();
        await _tapVisible(tester, find.text(l10n.localModelsFilterAll));
        await tester.pumpAndSettle();
        await _tapVisible(
          tester,
          find.text(l10n.localModelsCategorySpeechToText).first,
        );
        await tester.pumpAndSettle();
        expect(find.text(l10n.localModelsResultCount(1, 5)), findsOneWidget);
        expect(find.text('Voice Recognition'), findsOneWidget);
        final search = find.byType(TextField).first;
        await tester.enterText(search, '   MISSING   ');
        await tester.pumpAndSettle();
        expect(find.text(l10n.localModelsNoMatches), findsOneWidget);
        await _tapVisible(tester, find.text(l10n.localModelsResetFilters));
        await tester.pumpAndSettle();
        expect(tester.widget<TextField>(search).controller!.text, isEmpty);
        await tester.enterText(search, '  SPECIAL-TAG  ');
        await tester.pumpAndSettle();
        expect(find.text('Voice Synthesis'), findsOneWidget);
        expect(find.text(l10n.localModelsResultCount(1, 5)), findsOneWidget);
        await _tapVisible(tester, find.byTooltip(l10n.clear));
        await tester.pumpAndSettle();
        await _tapVisible(tester, find.text(l10n.localModelsFilterDownloads));
        await tester.pumpAndSettle();
        expect(find.text(l10n.localModelsNoMatches), findsOneWidget);
        await _tapVisible(
          tester,
          width < 360
              ? find.byTooltip(l10n.localModelsAddModel)
              : find.text(l10n.localModelsAddModel),
        );
        await tester.pumpAndSettle();
        expect(find.text(l10n.localModelsExploreHub), findsOneWidget);
        expect(tester.takeException(), isNull, reason: 'Hub search layout');
        await _tapVisible(tester, find.text(l10n.localModelsHubRepositoryTab));
        await tester.pumpAndSettle();
        expect(
          find.text(l10n.localModelsHubDirectImportPrompt),
          findsOneWidget,
        );
        expect(
          tester.takeException(),
          isNull,
          reason: 'repository import layout',
        );
        await tester.pumpWidget(const SizedBox.shrink());
        await tester.pumpAndSettle();
      });
    }
  }
  testWidgets('large catalog builds visible rows and finds the last model', (
    tester,
  ) async {
    final template = _catalog().first;
    final bridge = _Bridge()
      ..catalog = [
        for (var index = 0; index < 250; index++)
          core.LocalModelCatalogStatus.fromJson({
            ...template.toJson(),
            'manifest': {
              ...template.manifest.toJson(),
              'id': 'catalog-$index',
              'displayName': 'Catalog Model $index',
            },
            'installedModel': null,
          }),
      ];
    final l10n = await _pumpCatalog(tester, bridge, 1000, const Locale('zh'));
    expect(find.text(l10n.localModelsResultCount(250, 250)), findsOneWidget);
    expect(find.text('Catalog Model 249'), findsNothing);
    await tester.enterText(find.byType(TextField).first, 'catalog-249');
    await tester.pumpAndSettle();
    expect(find.text('Catalog Model 249'), findsOneWidget);
    expect(find.text(l10n.localModelsResultCount(1, 250)), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets(
    'downloads show paused and failed tasks and refresh on completion',
    (tester) async {
      final bridge = _Bridge()..catalog = _catalog();
      core.LocalModelInstallStatus task(core.LocalModelInstallPhase phase) =>
          core.LocalModelInstallStatus(
            operationId: 'download',
            modelId: 'model-1',
            version: 'v1',
            phase: phase,
            currentFile: null,
            downloadedBytes: 50,
            totalBytes: 100,
            error: phase == core.LocalModelInstallPhase.failed
                ? 'Connection interrupted'
                : null,
          );
      bridge.installStatuses = [task(core.LocalModelInstallPhase.cancelled)];
      final l10n = await _pumpCatalog(tester, bridge, 390, const Locale('zh'));
      await _tapVisible(tester, find.text(l10n.localModelsFilterDownloads));
      await tester.pumpAndSettle();
      expect(find.text(l10n.localModelsResultCount(1, 5)), findsOneWidget);
      expect(find.text(l10n.localModelsResumeDownload), findsOneWidget);
      expect(find.byTooltip(l10n.localModelsPauseDownload), findsNothing);

      bridge.installStatuses = [task(core.LocalModelInstallPhase.failed)];
      await tester.pump(const Duration(milliseconds: 500));
      await tester.pumpAndSettle();
      expect(find.text('Connection interrupted'), findsOneWidget);
      expect(find.text(l10n.retry), findsOneWidget);
      expect(find.byTooltip(l10n.localModelsPauseDownload), findsNothing);

      bridge.installStatuses = [task(core.LocalModelInstallPhase.model)];
      await tester.pump(const Duration(milliseconds: 500));
      await tester.pumpAndSettle();
      expect(find.byTooltip(l10n.localModelsPauseDownload), findsOneWidget);
      final pending = bridge.catalog![1];
      bridge.catalog![1] = core.LocalModelCatalogStatus.fromJson({
        ...pending.toJson(),
        'installedModel': core.InstalledLocalModel(
          manifest: pending.manifest,
          storagePath: 'models/model-1',
          installedAtMs: 1,
          verifiedAtMs: 1,
        ).toJson(),
      });
      bridge.installStatuses = [task(core.LocalModelInstallPhase.completed)];
      await tester.pump(const Duration(milliseconds: 500));
      await tester.pumpAndSettle();
      expect(find.text(l10n.localModelsNoMatches), findsOneWidget);
      await _tapVisible(tester, find.text(l10n.localModelsFilterInstalled));
      await tester.pumpAndSettle();
      expect(find.text(l10n.localModelsResultCount(2, 5)), findsOneWidget);
      expect(find.text('Voice Recognition'), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );
}

Future<void> _tapVisible(WidgetTester tester, Finder finder) async {
  await tester.ensureVisible(finder);
  await tester.pumpAndSettle();
  await tester.tap(finder);
}

Future<AppLocalizations> _pumpCatalog(
  WidgetTester tester,
  _Bridge bridge,
  double width,
  Locale locale,
) async {
  tester.view.physicalSize = Size(width, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  tester.platformDispatcher.localesTestValue = [locale];
  addTearDown(tester.platformDispatcher.clearLocalesTestValue);
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
  return AppLocalizations.of(
    tester.element(find.byType(LocalModelSettingsPanel)),
  )!;
}

List<core.LocalModelCatalogStatus> _catalog() => [
  for (final (index, name, kind) in [
    (0, 'Tiny Chat GGUF', core.LocalModelKind.chat),
    (1, 'Voice Recognition', core.LocalModelKind.speechToText),
    (2, 'Voice Synthesis', core.LocalModelKind.textToSpeech),
    (3, 'Embedding Multilingual', core.LocalModelKind.embedding),
    (4, 'Another Chat Model', core.LocalModelKind.chat),
  ])
    core.LocalModelCatalogStatus(
      manifest: core.LocalModelManifest.fromJson({
        ..._Bridge.manifest.toJson(),
        'id': index == 0 ? 'tiny' : 'model-$index',
        'displayName': name,
        'kind': kind.toJson(),
        'tags': index == 2 ? ['special-tag'] : [],
      }),
      installedModel: index == 0 ? _Bridge().installed : null,
      installedEngine: null,
      engineManifest: null,
      platformCompatible: true,
      inferenceBindingKey: null,
    ),
];

class _Bridge extends OperitRuntimeBridge {
  List<core.LocalModelCatalogStatus>? catalog;
  List<core.LocalModelInstallStatus> installStatuses = [];
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
        result =
            catalog?.map((status) => status.toJson()).toList() ??
            [
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
        result = installStatuses.map((status) => status.toJson()).toList();
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
