import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:flutter/services.dart';

import '../assets/theme_asset_resolver.dart';

const String kNotoSansScFontId = 'noto-sans-sc';
const String kCascadiaMonoFontId = 'cascadia-mono';
const int themeFontMaxBytes = 32 * 1024 * 1024;

final RegExp _fontAssetReferencePattern = RegExp(
  r'^assets/fonts/([a-z0-9_-]+/)?[a-z0-9_-]+\.ttf$',
);
final RegExp _sha256Pattern = RegExp(r'^[a-f0-9]{64}$');

/// Registered v1 references only, never a system family name or remote URI.
String validateThemeFontReference(String reference) {
  if (reference == kNotoSansScFontId || reference == kCascadiaMonoFontId) {
    return reference;
  }
  if (_fontAssetReferencePattern.hasMatch(reference)) return reference;
  throw ArgumentError.value(
    reference,
    'reference',
    'unregistered theme v1 font',
  );
}

String themeFontFamilyForSha256(String digest) {
  if (!_sha256Pattern.hasMatch(digest)) {
    throw ArgumentError.value(
      digest,
      'digest',
      'a lowercase SHA-256 is required',
    );
  }
  return 'OperitThemeFont_$digest';
}

class PreparedThemeFont {
  const PreparedThemeFont._({
    required this.reference,
    required this.sha256,
    required this.family,
    required this.bytes,
  });

  final String reference;
  final String sha256;
  final String family;
  final int bytes;

  Map<String, Object> toJson() => <String, Object>{
    'reference': reference,
    'sha256': sha256,
    'family': family,
    'bytes': bytes,
  };
}

typedef ThemeFontRegistration =
    Future<void> Function(String family, Uint8List bytes);

/// Font registration is not a media safety validator. Public bytes must have
/// passed L4 before calling this; trusted bundled and private preset fonts are
/// separate inputs. Only immutable content digests are cached, never paths.
class ThemeFontRegistry {
  ThemeFontRegistry({ThemeFontRegistration? registration})
    : _registration = registration ?? _registerFlutterFont;

  final ThemeFontRegistration _registration;
  final Map<String, Future<void>> _registrations = <String, Future<void>>{};
  static final Map<String, Future<void>> _flutterRegistrations =
      <String, Future<void>>{};

  Future<PreparedThemeFont> prepare(
    String reference, {
    required ThemeAssetResolver resolver,
    required String expectedSha256,
  }) async {
    validateThemeFontReference(reference);
    themeFontFamilyForSha256(expectedSha256);
    final bytes = await resolver.readBytes(reference);
    if (bytes.length > themeFontMaxBytes) {
      throw StateError('Public theme font byte budget exceeded: $reference');
    }
    return registerBytes(
      reference: reference,
      bytes: bytes,
      expectedSha256: expectedSha256,
    );
  }

  /// Private presets may use runtime asset references and legacy font formats.
  /// This method does not make such content exportable as a public v1 theme.
  Future<PreparedThemeFont> registerBytes({
    required String reference,
    required Uint8List bytes,
    String? expectedSha256,
  }) async {
    if (reference.isEmpty) throw ArgumentError('Font reference is empty');
    if (bytes.isEmpty || bytes.length > themeFontMaxBytes) {
      throw StateError('Theme font byte budget exceeded: $reference');
    }
    if (expectedSha256 != null) themeFontFamilyForSha256(expectedSha256);
    // Copy before hashing/loading so the caller cannot change bytes in flight.
    final immutableBytes = Uint8List.fromList(bytes);
    final digest = sha256.convert(immutableBytes).toString();
    if (expectedSha256 != null && digest != expectedSha256) {
      throw StateError('Theme font SHA-256 mismatch: $reference');
    }
    final family = themeFontFamilyForSha256(digest);
    await _registrations.putIfAbsent(
      family,
      () => _registration(family, immutableBytes),
    );
    return PreparedThemeFont._(
      reference: reference,
      sha256: digest,
      family: family,
      bytes: immutableBytes.length,
    );
  }

  static Future<void> _registerFlutterFont(String family, Uint8List bytes) =>
      _flutterRegistrations.putIfAbsent(family, () {
        final loader = FontLoader(family)
          ..addFont(Future<ByteData>.value(ByteData.sublistView(bytes)));
        return loader.load();
      });
}
