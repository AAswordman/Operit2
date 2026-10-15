// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:operit_ui/operit_ui.dart';

import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../data/preferences/UserPreferencesManager.dart';
import 'OperitThemeAssets.dart';
import '../features/packages/utils/PackageDisplayUtils.dart';

/// Identifies one creation source contributed by a currently registered ToolPkg.
class ThemeTemplateSource {
  /// Retains the package identity and template declaration for an explicit creation.
  const ThemeTemplateSource({
    required this.packageName,
    required this.templateId,
    required this.label,
  });

  final String packageName;
  final String templateId;
  final String label;
}

/// Copies template defaults into independent ordinary appearance configurations.
class ThemeTemplateService {
  /// Uses the same package registry and storage repository as the rest of the app.
  ThemeTemplateService(this.clients)
    : _assets = ThemeAssetStore(
        runtimeStorage: clients.repositoryRuntimeStorageRepository,
      );

  final GeneratedCoreProxyClients clients;
  final ThemeAssetStore _assets;

  /// Finds template declarations dynamically through the normal plugin registry.
  Future<List<ThemeTemplateSource>> listTemplates() async {
    final packages = await clients.application
        .packageManager()
        .getToolPkgContainerRuntimes();
    return <ThemeTemplateSource>[
      for (final package in packages)
        for (final template in package.themes)
          ThemeTemplateSource(
            packageName: package.packageName,
            templateId: template.id,
            label: '${toolPkgContainerDisplayName(package)} / ${template.id}',
          ),
    ];
  }

  /// Validates the current package bytes and copies all referenced media into shared user assets.
  Future<ThemePreferenceSnapshot> createSnapshot(
    ThemeTemplateSource source,
  ) async {
    final payload = await clients.application.previewToolPkgTheme(
      packageName: source.packageName,
      templateId: source.templateId,
    );
    if (payload.themeId != '${source.packageName}:${source.templateId}') {
      throw StateError(
        'Template receipt belongs to a different ToolPkg declaration',
      );
    }
    final assets = <String, Uint8List>{};
    final digests = <String, String>{
      for (final file in payload.validation.files)
        if (file.path == 'preview.png' || file.path.startsWith('assets/'))
          file.path: file.sha256,
    };
    for (final asset in payload.assets) {
      if (!digests.containsKey(asset.path) || assets.containsKey(asset.path)) {
        throw StateError(
          'Template receipt contains an undeclared or duplicate resource',
        );
      }
      assets[asset.path] = asset.bytes;
    }
    if (assets.length != digests.length) {
      throw StateError('Template receipt is missing declared resources');
    }
    final spec = ThemeVisualSpecV1.fromCheckedContract(
      jsonDecode(payload.themeJson) as Map<String, dynamic>,
    );
    final builtin = await BuiltinThemeFonts.load(bundle: rootBundle);
    final prepared = await PreparedThemeVisual.prepare(
      spec: spec,
      assets: assets,
      assetSha256: digests,
      builtinFonts: builtin,
      fontRegistry: ThemeFontRegistry(),
    );
    try {
      final paths = <String, String>{};
      for (final reference in spec.assetReferences) {
        final imported = await _assets.importBytes(
          bytes: assets[reference]!,
          fileName: reference,
        );
        paths[reference] = imported.storagePath;
      }
      final json = spec
          .toSnapshot(UserPreferencesManager.defaultThemePreferenceSnapshot)
          .toJson();
      for (final field in const <String>[
        'backgroundImageUri',
        'bubbleUserImageUri',
        'bubbleAiImageUri',
        'inputImageUri',
        'inputSettingsAsset',
        'inputAttachmentAsset',
        'inputMicAsset',
        'inputSendAsset',
        'inputStopAsset',
      ]) {
        final reference = json[field];
        if (reference is String) json[field] = paths[reference]!;
      }
      final fonts = spec.fontReferences;
      final global = fonts['global']!;
      for (final role in <String, String>{
        '': global,
        'bubbleUser': fonts['user'] == null ? global : fonts['user']!,
        'bubbleAi': fonts['ai'] == null ? global : fonts['ai']!,
      }.entries) {
        final reference = role.value;
        if (!reference.startsWith('assets/') && !paths.containsKey(reference)) {
          final imported = await _assets.importBytes(
            bytes: await builtin.readBytes(reference),
            fileName: '$reference.ttf',
          );
          paths[reference] = imported.storagePath;
        }
        final prefix = role.key;

        /// Names an existing appearance field for one explicitly declared font role.
        String field(String suffix) => prefix.isEmpty
            ? '${suffix[0].toLowerCase()}${suffix.substring(1)}'
            : '$prefix$suffix';
        json[field('UseCustomFont')] = true;
        json[field('FontType')] = UserPreferencesManager.FONT_TYPE_FILE;
        json[field('CustomFontPath')] = paths[reference]!;
        json[field('SystemFontName')] = null;
      }
      return ThemePreferenceSnapshot.fromJson(json);
    } finally {
      prepared.dispose();
    }
  }
}
