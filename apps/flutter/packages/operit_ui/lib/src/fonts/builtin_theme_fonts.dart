import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:flutter/services.dart';

import '../assets/theme_asset_resolver.dart';
import 'theme_font_glyphs.dart';
import 'theme_font_registry.dart';

class BuiltinThemeFont {
  const BuiltinThemeFont._({
    required this.id,
    required this.asset,
    required this.sha256,
    required this.bytes,
    required this.licenseAsset,
    required this.licenseSha256,
    required this.source,
  });

  final String id;
  final String asset;
  final String sha256;
  final int bytes;
  final String licenseAsset;
  final String licenseSha256;
  final String source;
}

/// Only trusted, bundled catalog entries are loaded. Source URLs are provenance
/// text, never runtime network requests or alternate asset locations.
class BuiltinThemeFonts implements ThemeAssetResolver {
  BuiltinThemeFonts._(this._bundle, this._entries);

  final AssetBundle _bundle;
  final Map<String, BuiltinThemeFont> _entries;
  static const _packagePrefix = 'packages/operit_ui/';

  static Future<BuiltinThemeFonts> load({required AssetBundle bundle}) async {
    final json =
        jsonDecode(
              await bundle.loadString(
                '${_packagePrefix}assets/fonts/catalog.v1.json',
              ),
            )
            as Map<String, dynamic>;
    if (json['formatVersion'] != 1)
      throw FormatException('Unknown builtin font catalog');
    final entries = <String, BuiltinThemeFont>{};
    for (final raw in json['fonts'] as List<dynamic>) {
      final row = raw as Map<String, dynamic>;
      final id = row['id'] as String;
      if (id != kNotoSansScFontId && id != kCascadiaMonoFontId) {
        throw FormatException('Unknown builtin font ID: $id');
      }
      if (entries.containsKey(id))
        throw FormatException('Duplicate builtin font ID');
      final digest = row['sha256'] as String;
      final licenseDigest = row['licenseSha256'] as String;
      themeFontFamilyForSha256(digest);
      themeFontFamilyForSha256(licenseDigest);
      final bytes = row['bytes'] as int;
      if (bytes <= 0 ||
          bytes > themeFontMaxBytes ||
          row['license'] != 'OFL-1.1' ||
          row['asset'] != 'assets/fonts/$id.ttf' ||
          row['licenseAsset'] != 'assets/licenses/$id.txt') {
        throw FormatException('Invalid builtin font metadata: $id');
      }
      entries[id] = BuiltinThemeFont._(
        id: id,
        asset: row['asset'] as String,
        sha256: digest,
        bytes: bytes,
        licenseAsset: row['licenseAsset'] as String,
        licenseSha256: licenseDigest,
        source: row['source'] as String,
      );
    }
    if (entries.length != 2)
      throw FormatException('Both pinned builtin fonts are required');
    return BuiltinThemeFonts._(bundle, Map.unmodifiable(entries));
  }

  BuiltinThemeFont declaration(String id) {
    final entry = _entries[id];
    if (entry == null)
      throw ArgumentError('Builtin font is not registered: $id');
    return entry;
  }

  @override
  Future<Uint8List> readBytes(String reference) async {
    final entry = declaration(reference);
    final data = await _bundle.load('$_packagePrefix${entry.asset}');
    final bytes = Uint8List.fromList(
      data.buffer.asUint8List(data.offsetInBytes, data.lengthInBytes),
    );
    if (bytes.length != entry.bytes ||
        sha256.convert(bytes).toString() != entry.sha256) {
      throw StateError('Bundled font bytes or SHA-256 differ: $reference');
    }
    final license = await _bundle.load('$_packagePrefix${entry.licenseAsset}');
    final licenseBytes = license.buffer.asUint8List(
      license.offsetInBytes,
      license.lengthInBytes,
    );
    if (licenseBytes.length > 128 * 1024 ||
        sha256.convert(licenseBytes).toString() != entry.licenseSha256) {
      throw StateError('Bundled font license differs: $reference');
    }
    return bytes;
  }

  Future<PreparedThemeFont> prepare(
    String id, {
    required ThemeFontRegistry registry,
    required String fixtureText,
  }) async {
    final bytes = await readBytes(id);
    ThemeFontGlyphs(bytes).requireText(fixtureText, reference: id);
    return registry.registerBytes(
      reference: id,
      bytes: bytes,
      expectedSha256: declaration(id).sha256,
    );
  }
}
