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
import 'package:operit2/ui/features/settings/local_models/LocalModelSettingsPanel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit_folder_access/operit_folder_access.dart';

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
        await _tapVisible(tester, find.text(l10n.localModelsAddModel));
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

  testWidgets(
    'GGUF import streams the selection and reveals the installed model',
    (tester) async {
      final bridge = _Bridge()..importCommit = Completer<void>();
      _useFilePicker([
        [],
        [
          _modelFile('notes.txt', [1]),
        ],
        [
          _modelFile('Qwen3-0.6B-Q4_K_M.gguf', [71, 71, 85, 70]),
        ],
      ]);
      final l10n = await _pumpCatalog(tester, bridge, 320, const Locale('zh'));
      await _tapVisible(tester, find.text(l10n.localModelsImportFromDevice));
      await tester.pumpAndSettle();
      expect(find.text('GGUF · llama.cpp'), findsOneWidget);
      expect(find.text('MNN'), findsOneWidget);
      final importButton = find.widgetWithText(
        FilledButton,
        l10n.localModelsHubImport,
      );
      expect(tester.widget<FilledButton>(importButton).onPressed, isNull);

      await tester.tap(find.text(l10n.selectFile));
      await tester.pumpAndSettle();
      expect(bridge.fileImport, isNull);
      await tester.tap(find.text(l10n.selectFile));
      await tester.pumpAndSettle();
      expect(find.text(l10n.localModelsImportInvalidGguf), findsOneWidget);

      await tester.tap(find.text(l10n.selectFile));
      await tester.pumpAndSettle();
      expect(find.text('Qwen3-0.6B-Q4_K_M.gguf'), findsOneWidget);
      expect(find.text('Qwen3-0.6B-Q4_K_M'), findsOneWidget);
      expect(tester.takeException(), isNull, reason: 'compact import layout');
      await tester.tap(importButton);
      await tester.pumpAndSettle();
      expect(find.text(l10n.localModelsImportFinishing), findsOneWidget);
      expect(bridge.fileImport, {
        'engine': 'LlamaCpp',
        'displayName': 'Qwen3-0.6B-Q4_K_M',
        'files': [
          {'name': 'Qwen3-0.6B-Q4_K_M.gguf', 'byteLength': 4},
        ],
      });
      expect(bridge.importChunks, [
        [71, 71, 85, 70],
        [],
      ]);

      bridge.importCommit!.complete();
      await tester.pumpAndSettle();
      expect(find.byType(Dialog), findsNothing);
      expect(find.text('Qwen3-0.6B-Q4_K_M'), findsOneWidget);
      expect(find.text(l10n.localModelsImportedFile), findsOneWidget);
      expect(find.text(l10n.localModelsImportComplete), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.pump(const Duration(seconds: 4));
      await tester.pumpAndSettle();
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  testWidgets(
    'MNN file import preserves the bundle and shows commit failures',
    (tester) async {
      final bridge = _Bridge()..importCommit = Completer<void>();
      _useFilePicker([
        [
          _modelFile('config.json', [123, 125]),
          _modelFile('llm.mnn', [1, 2, 3]),
        ],
      ]);
      final l10n = await _pumpCatalog(tester, bridge, 900, const Locale('en'));
      await _tapVisible(tester, find.text(l10n.localModelsImportFromDevice));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(ChoiceChip, 'MNN'));
      await tester.pumpAndSettle();
      expect(find.text(l10n.localModelsImportMnnHint), findsOneWidget);
      await tester.tap(find.text(l10n.selectFile));
      await tester.pumpAndSettle();
      expect(find.text(l10n.localModelsImportFileCount(2)), findsOneWidget);
      final nameField = find.widgetWithText(
        TextField,
        l10n.localModelsImportName,
      );
      await tester.enterText(nameField, 'My MNN model');
      await tester.pumpAndSettle();
      await tester.tap(
        find.widgetWithText(FilledButton, l10n.localModelsHubImport),
      );
      await tester.pumpAndSettle();
      expect(bridge.fileImport?['engine'], 'Mnn');
      expect(bridge.importChunks, [
        [123, 125],
        [1, 2, 3],
        [],
      ]);

      bridge.importCommit!.completeError(StateError('Missing bundle weight'));
      await tester.pumpAndSettle();
      expect(find.textContaining('Missing bundle weight'), findsOneWidget);
      expect(find.text(l10n.selectFile), findsOneWidget);
      expect(find.text(l10n.localModelsImportComplete), findsNothing);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  testWidgets(
    'online discovery remains available without a file import engine',
    (tester) async {
      final bridge = _Bridge()..importEngines = const [];
      final l10n = await _pumpCatalog(tester, bridge, 390, const Locale('en'));
      expect(find.text(l10n.localModelsImportFromDevice), findsNothing);
      expect(find.text(l10n.localModelsAddModel), findsOneWidget);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  for (final size in [const Size(520, 360), const Size(320, 280)]) {
    testWidgets('catalog header scrolls out of a short viewport at $size', (
      tester,
    ) async {
      final template = _catalog().first;
      final bridge = _Bridge()
        ..catalog = [
          for (var index = 0; index < 50; index++)
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
      final l10n = await _pumpCatalog(
        tester,
        bridge,
        size.width,
        const Locale('zh'),
        height: size.height,
      );
      expect(tester.takeException(), isNull, reason: 'short catalog layout');
      expect(find.text('Catalog Model 49'), findsNothing);
      expect(find.byType(TextField).hitTestable(), findsOneWidget);
      final catalogScroll = find.descendant(
        of: find.byType(LocalModelSettingsPanel),
        matching: find.byWidgetPredicate(
          (widget) =>
              widget is Scrollable &&
              widget.axisDirection == AxisDirection.down,
        ),
      );
      expect(tester.getSize(catalogScroll).height, size.height);
      await tester.drag(catalogScroll, const Offset(0, -350));
      await tester.pumpAndSettle();
      expect(find.byType(TextField).hitTestable(), findsNothing);
      final visibleModels = find.textContaining('Catalog Model').hitTestable();
      expect(visibleModels, findsWidgets);
      expect(tester.takeException(), isNull, reason: 'scrolled catalog layout');
      tester.state<ScrollableState>(catalogScroll).position.jumpTo(0);
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).first, 'catalog-49');
      await tester.pumpAndSettle();
      expect(find.text(l10n.localModelsResultCount(1, 50)), findsOneWidget);
      await tester.scrollUntilVisible(
        find.text('Catalog Model 49'),
        160,
        scrollable: catalogScroll,
      );
      expect(find.text('Catalog Model 49').hitTestable(), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    });
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

/// Taps a control after revealing it in its scrollable viewport.
Future<void> _tapVisible(WidgetTester tester, Finder finder) async {
  await tester.ensureVisible(finder);
  await tester.pumpAndSettle();
  await tester.tap(finder);
}

Future<AppLocalizations> _pumpCatalog(
  WidgetTester tester,
  _Bridge bridge,
  double width,
  Locale locale, {
  double height = 900,
}) async {
  tester.view.physicalSize = Size(width, height);
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
  List<core.LocalEngineKind> importEngines = const [
    core.LocalEngineKind.llamaCpp,
    core.LocalEngineKind.mnn,
  ];
  Map<String, Object?>? fileImport;
  final importChunks = <Uint8List>[];
  Completer<void>? importCommit;
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
    sources: const [
      core.LocalModelSource(
        id: 'hf',
        kind: core.LocalModelSourceKind.huggingFace,
        repository: 'operit/tiny',
        revision: 'v1',
        baseUrl: 'https://huggingface.co/operit/tiny/resolve/v1',
      ),
    ],
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
    'capabilitiesOverride': <String, Object?>{},
    'builtinToolsOverride': null,
    'requestOverride': <String, Object?>{},
    'listed': null,
    'parameters': <Object?>[],
    'summary': const core.ModelSummarySettings(
      enableSummary: true,
      summaryTokenThreshold: 0.7,
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
      case 'getFileImportEngines':
        result = importEngines.map((engine) => engine.toJson()).toList();
      case 'getProviderProfiles':
        result = [
          <String, Object?>{
            'id': 'LOCAL_MODEL',
            'name': 'Local Models',
            'providerTypeId': 'LOCAL_MODEL',
            'providerType': 'LOCAL_MODEL',
            'endpoint': '',
            'apiKey': '',
            'useMultipleApiKeys': false,
            'apiKeyPool': <Object?>[],
            'currentKeyIndex': 0,
            'keyRotationMode': 'ROUND_ROBIN',
            'customHeaders': '{}',
            'requestLimitPerMinute': 0,
            'maxConcurrentRequests': 0,
            'models': [profile],
          },
        ];
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

  /// Captures the import wire contract and publishes a catalog row after commit.
  @override
  Future<CorePushSink> push(CorePushRequest request) async {
    if (request.methodName != 'importModelFromFiles') {
      throw UnsupportedError('Unexpected push: ${request.methodName}');
    }
    final args = Map<String, Object?>.from(request.args as Map);
    fileImport = args;
    return _ImportSink(importChunks, () async {
      await importCommit?.future;
      final imported = core.LocalModelManifest.fromJson({
        ...manifest.toJson(),
        'id': 'local-file-test',
        'displayName': args['displayName'],
        'sources': <Object?>[],
      });
      catalog = [
        ...(catalog ?? _catalog()),
        core.LocalModelCatalogStatus(
          manifest: imported,
          installedModel: core.InstalledLocalModel(
            manifest: imported,
            storagePath: 'runtime/models/local/chat/local-file-test',
            installedAtMs: 2,
            verifiedAtMs: 2,
          ),
          engineManifest: null,
          installedEngine: null,
          platformCompatible: true,
          inferenceBindingKey: 'local-llm:test-node:local-file-test@v1',
        ),
      ];
    });
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnsupportedError('unused');
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      const Stream.empty();
}

class _ImportSink implements CorePushSink {
  /// Records bytes until the simulated runtime finishes the import.
  _ImportSink(this.chunks, this.onClose);

  final List<Uint8List> chunks;
  final Future<void> Function() onClose;

  /// Records one ordered stream item.
  @override
  Future<void> add(Object? args) async => chunks.add(args! as Uint8List);

  /// Completes the fake runtime operation.
  @override
  Future<void> close() => onClose();
}

/// Substitutes and restores the registered picker for this widget test.
void _useFilePicker(List<List<SelectedFileInput>> picks) {
  final original = OperitFolderAccessPlatform.instance;
  OperitFolderAccessPlatform.instance = _ModelFilePicker(picks);
  addTearDown(() => OperitFolderAccessPlatform.instance = original);
}

/// Supplies selected metadata and bytes without opening a system dialog.
SelectedFileInput _modelFile(String name, List<int> bytes) =>
    _ModelInput(name, Uint8List.fromList(bytes));

class _ModelInput extends SelectedFileInput {
  /// Implements document reads inside the widget test's fake clock.
  _ModelInput(String filename, this._contents)
    : super.fromStream(
        name: filename,
        byteLength: _contents.length,
        stream: const Stream.empty(),
      );

  final Uint8List _contents;

  /// Emits the document payload as one bounded chunk.
  @override
  Stream<Uint8List> chunks() async* {
    yield _contents;
  }

  /// Releases this in-memory picker result.
  @override
  Future<void> close() async {}
}

class _ModelFilePicker extends OperitFolderAccessPlatform {
  /// Returns one selection per picker invocation.
  _ModelFilePicker(this.picks);

  final List<List<SelectedFileInput>> picks;

  /// Pops the next fake file selection.
  @override
  Future<List<SelectedFileInput>> pickFileInputs({
    bool imagesOnly = false,
  }) async => picks.removeAt(0);

  /// Marks directory selection as outside this test's contract.
  @override
  Future<String?> pickDirectory({String? initialDirectory}) =>
      throw UnsupportedError('unused');

  /// Marks multi-directory selection as outside this test's contract.
  @override
  Future<List<String>> pickDirectories({String? initialDirectory}) =>
      throw UnsupportedError('unused');
}
