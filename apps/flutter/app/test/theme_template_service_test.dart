import 'dart:convert';

import 'package:crypto/crypto.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/ui/theme/ThemeTemplateService.dart';

/// Verifies template creation copies complete appearance and resources through the ordinary owners.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'template sources follow the current plugin registry without a library cache',
    () async {
      final bridge = _TemplateBridge();
      final service = ThemeTemplateService(GeneratedCoreProxyClients(bridge));
      final before = await service.listTemplates();
      expect(before.single.packageName, 'dev.example.plugin');
      expect(before.single.templateId, 'night');
      bridge.installed = false;
      expect(await service.listTemplates(), isEmpty);
      expect(bridge.calls.map((call) => call.methodName), <String>[
        'getToolPkgContainerRuntimes',
        'getToolPkgContainerRuntimes',
      ]);
    },
  );

  test('removing a creation source reports failure without writing user assets', () async {
    final bridge = _TemplateBridge();
    final service = ThemeTemplateService(GeneratedCoreProxyClients(bridge));
    final source = (await service.listTemplates()).single;
    bridge.installed = false;
    await expectLater(service.createSnapshot(source), throwsStateError);
    expect(bridge.files, isEmpty);
    expect(bridge.calls.map((call) => call.methodName), <String>[
      'getToolPkgContainerRuntimes', 'previewToolPkgTheme',
    ]);
  });

  test(
    'template creation copies images and fonts without mutating appearance or binding its plugin',
    () async {
      final bridge = _TemplateBridge();
      final visual =
          jsonDecode(
                await rootBundle.loadString(
                  'packages/operit_ui/assets/fixtures/theme-v1.json',
                ),
              )
              as Map<String, dynamic>;
      final image = Uint8List.fromList(
        utf8.encode(
          '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="12" viewBox="0 0 16 12">'
          '<path d="M1 1 L15 11" fill="#334455"/></svg>',
        ),
      );
      (visual['background'] as Map<String, dynamic>)['asset'] =
          'assets/images/background.svg';
      (visual['palette']['light'] as Map<String, dynamic>)['primary'] =
          '#123456FF';
      final digest = sha256.convert(image).toString();
      bridge.payload = core.ThemePreviewReceipt(
        validation: core.ThemeValidationReceipt(
          validatorVersion: 1,
          themeId: 'dev.example.plugin:night',
          archiveSha256: 'a' * 64,
          archiveBytes: 128,
          expandedBytes: image.length,
          manifestSha256: 'b' * 64,
          themeSha256: 'c' * 64,
          files: <core.ThemeFileValidation>[
            core.ThemeFileValidation(
              path: 'assets/images/background.svg',
              bytes: image.length,
              sha256: digest,
            ),
          ],
          media: const <Object?>[],
          trust: core.ThemeTemplateTrust.untrusted,
        ),
        themeId: 'dev.example.plugin:night',
        version: '1.0',
        themeJson: jsonEncode(visual),
        assets: <core.ThemePreviewAsset>[
          core.ThemePreviewAsset(
            path: 'assets/images/background.svg',
            bytes: image,
          ),
        ],
      );
      final service = ThemeTemplateService(GeneratedCoreProxyClients(bridge));
      final source = (await service.listTemplates()).single;
      final snapshot = await service.createSnapshot(source);
      expect(
        snapshot.backgroundImageUri,
        startsWith('runtime/data/user_assets/theme/'),
      );
      expect(bridge.files[snapshot.backgroundImageUri], image);
      expect(jsonDecode(snapshot.themePaletteJson!), visual['palette']);
      expect(
        snapshot.customFontPath,
        startsWith('runtime/data/user_assets/theme/'),
      );
      expect(snapshot.bubbleUserCustomFontPath, snapshot.customFontPath);
      expect(snapshot.bubbleAiCustomFontPath, snapshot.customFontPath);
      expect(bridge.files[snapshot.customFontPath]!.length, greaterThan(1000));
      expect(bridge.files, hasLength(2));
      expect(bridge.calls.map((call) => call.methodName).toSet(), <String>{
        'getToolPkgContainerRuntimes',
        'previewToolPkgTheme',
        'themeAssetsDirPath',
        'writeBase64',
      });
      bridge.installed = false;
      expect(await service.listTemplates(), isEmpty);
      expect(bridge.files[snapshot.backgroundImageUri], image);
      expect(bridge.files.containsKey(snapshot.customFontPath), isTrue);
    },
  );
}

/// Supplies only the existing registry, validated preview, and shared asset repository boundaries.
class _TemplateBridge extends OperitRuntimeBridge {
  bool installed = true;
  core.ThemePreviewReceipt? payload;
  final calls = <CoreCallRequest>[];
  final files = <String, Uint8List>{};

  /// Returns a valid registered package fixture with a template contribution.
  Map<String, Object?> package() => <String, Object?>{
    'packageName': 'dev.example.plugin',
    'displayName': <String, Object?>{
      'values': <String, String>{'en': 'Plugin'},
    },
    'description': <String, Object?>{
      'values': <String, String>{'en': 'Example'},
    },
    'logoResource': null,
    'marketOrigin': null,
    'version': '1.0',
    'apiVersion': '1',
    'mainEntry': '',
    'sourceType': 'EXTERNAL',
    'sourcePath': 'host:///plugin.toolpkg',
    'themes': <Object?>[
      <String, Object?>{
        'id': 'night',
        'path': 'theme.json',
        'preview': 'preview.png',
        'files': <Object?>[],
      },
    ],
    for (final key in <String>[
      'requires',
      'author',
      'subpackages',
      'resources',
      'wasmModules',
      'workflowTemplates',
      'workspaceTemplates',
      'uiModules',
      'uiRoutes',
      'chatComposerSlots',
      'navigationEntries',
      'desktopWidgets',
      'appLifecycleHooks',
      'messageProcessingPlugins',
      'xmlRenderPlugins',
      'inputMenuTogglePlugins',
      'chatInputHooks',
      'chatViewHooks',
      'chatMessageHooks',
      'chatLifecycleHooks',
      'chatMessageMenuItems',
      'chatRuntimeHooks',
      'hostEventHooks',
      'toolLifecycleHooks',
      'promptInputHooks',
      'promptHistoryHooks',
      'promptEstimateHistoryHooks',
      'systemPromptComposeHooks',
      'toolPromptComposeHooks',
      'promptFinalizeHooks',
      'promptEstimateFinalizeHooks',
      'summaryGenerateHooks',
      'coreCommands',
      'aiProviders',
      'manifestExtensionHandlers',
    ])
      key: <Object?>[],
  };

  /// Records resource writes and rejects any unrelated installation or appearance mutation.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    final args = request.args as Map;
    final Object? result;
    switch (request.methodName) {
      case 'getToolPkgContainerRuntimes':
        result = <Object?>[if (installed) package()];
      case 'previewToolPkgTheme':
        if (!installed) throw StateError('ToolPkg was removed');
        result = payload!.toJson();
      case 'themeAssetsDirPath':
        result = 'runtime/data/user_assets/theme';
      case 'writeBase64':
        files[args['path'] as String] = base64Decode(
          args['base64Content'] as String,
        );
        result = null;
      default:
        throw StateError(
          'Unexpected template operation: ${request.methodName}',
        );
    }
    return encodeCoreLink(<Object?>[0, result]);
  }

  /// Rejects subscription paths because template sources are read only during creation.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Rejects subscription snapshots outside the template creation contract.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Rejects push operations outside the template creation contract.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();
}
