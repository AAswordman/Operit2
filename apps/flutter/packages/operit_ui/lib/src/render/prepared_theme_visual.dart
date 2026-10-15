import 'dart:typed_data';

import 'package:crypto/crypto.dart';

import '../assets/theme_asset_resolver.dart';
import '../fonts/builtin_theme_fonts.dart';
import '../fonts/theme_font_registry.dart';
import '../model/theme_preference_snapshot.dart';
import '../model/theme_visual_spec_v1.dart';
import '../preview/theme_probe_preparation.dart';

/// An immutable visual prepared from already L0-L4 validated bytes. No I/O,
/// package validation, installation or selection is performed by this class.
class PreparedThemeVisual {
  PreparedThemeVisual._(this.spec, this.resources);

  final ThemeVisualSpecV1 spec;
  final ThemeProbeResources resources;

  static Future<PreparedThemeVisual> prepare({
    required ThemeVisualSpecV1 spec,
    required Map<String, Uint8List> assets,
    required Map<String, String> assetSha256,
    required BuiltinThemeFonts builtinFonts,
    required ThemeFontRegistry fontRegistry,
  }) async {
    final sealedAssets = <String, Uint8List>{
      for (final entry in assets.entries)
        entry.key: Uint8List.fromList(entry.value),
    };
    final sealedDigests = Map<String, String>.unmodifiable(assetSha256);
    for (final entry in sealedAssets.entries) {
      if (sha256.convert(entry.value).toString() != sealedDigests[entry.key]) {
        throw StateError('Validated theme asset differs: ${entry.key}');
      }
    }
    if (!spec.assetReferences.every(sealedAssets.containsKey)) {
      throw StateError('Validated theme payload is incomplete');
    }
    final resolver = MemoryThemeAssetResolver(sealedAssets);
    final fonts = <String, Future<PreparedThemeFont>>{};
    Future<PreparedThemeFont> prepareFont(String reference) =>
        fonts.putIfAbsent(reference, () {
          validateThemeFontReference(reference);
          if (reference.startsWith('assets/')) {
            final digest = sealedDigests[reference];
            if (digest == null)
              throw StateError('Font has no validation digest');
            return fontRegistry.prepare(
              reference,
              resolver: resolver,
              expectedSha256: digest,
            );
          }
          return fontRegistry.prepare(
            reference,
            resolver: builtinFonts,
            expectedSha256: builtinFonts.declaration(reference).sha256,
          );
        });
    final references = spec.fontReferences;
    final globalReference = references['global']!;
    // Null role typography is contract-defined inheritance, not a system font.
    final global = prepareFont(globalReference);
    final user = prepareFont(references['user'] ?? globalReference);
    final ai = prepareFont(references['ai'] ?? globalReference);
    final resources = await ThemeProbeResources.prepare(
      sans: global,
      mono: prepareFont(kCascadiaMonoFontId),
      user: user,
      ai: ai,
      assets: sealedAssets,
      imageReferences: spec.imageAssetReferences,
    );
    return PreparedThemeVisual._(spec, resources);
  }

  /// Internal transport keeps hash-qualified font handles, never package paths
  /// as global aliases. Two packages using the same path cannot contaminate fonts.
  ThemePreferenceSnapshot snapshot(ThemePreferenceSnapshot settings) {
    final json = spec.toSnapshot(settings).toJson();
    void font(String prefix, PreparedThemeFont prepared) {
      String key(String suffix) => prefix.isEmpty
          ? '${suffix[0].toLowerCase()}${suffix.substring(1)}'
          : '$prefix$suffix';
      json[key('UseCustomFont')] = true;
      json[key('FontType')] = 'file';
      json[key('CustomFontPath')] = prepared.family;
      json[key('SystemFontName')] = null;
    }

    font('', resources.sans);
    font('bubbleUser', resources.user);
    font('bubbleAi', resources.ai);
    return ThemePreferenceSnapshot.fromJson(json);
  }

  void dispose() => resources.dispose();
}
