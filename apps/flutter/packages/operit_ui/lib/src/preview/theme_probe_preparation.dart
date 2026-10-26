import 'dart:ui' as ui;

import 'package:crypto/crypto.dart';
import 'package:flutter/foundation.dart';

import '../assets/theme_asset_resolver.dart';
import '../assets/theme_image_decoder.dart';
import '../fonts/theme_font_registry.dart';

enum ThemeProbePhase { idle, preparing, prepared, rendered, failed }

/// Trusted probe / already validated resources only; this is not a ZIP importer.
class ThemeProbeResources implements ThemeDecodedImageResolver {
  ThemeProbeResources._({
    required this.sans,
    required this.mono,
    required this.user,
    required this.ai,
    required Map<String, Uint8List> bytes,
    required Map<String, ui.Image> images,
    required this.assetSha256,
  }) : _bytes = bytes,
       _images = images;

  final PreparedThemeFont sans;
  final PreparedThemeFont mono;
  final PreparedThemeFont user;
  final PreparedThemeFont ai;
  final Map<String, Uint8List> _bytes;
  final Map<String, ui.Image> _images;
  final Map<String, String> assetSha256;
  bool _disposed = false;

  static Future<ThemeProbeResources> prepare({
    required Future<PreparedThemeFont> sans,
    required Future<PreparedThemeFont> mono,
    required Future<PreparedThemeFont> user,
    required Future<PreparedThemeFont> ai,
    required Map<String, Uint8List> assets,
    required Set<String> imageReferences,
  }) async {
    if (assets.length > 24) {
      throw StateError('Probe asset entry budget exceeded');
    }
    if (!imageReferences.every(assets.containsKey)) {
      throw StateError('Probe image references have missing bytes');
    }
    final fonts = await Future.wait(<Future<PreparedThemeFont>>[
      sans,
      mono,
      user,
      ai,
    ]);
    final sansFont = fonts[0];
    final monoFont = fonts[1];
    final copies = <String, Uint8List>{};
    final decoded = <String, ui.Image>{};
    final digests = <String, String>{};
    try {
      var totalPixels = 0;
      var totalBytes = 0;
      for (final entry in assets.entries) {
        totalBytes += entry.value.length;
        final limit = imageReferences.contains(entry.key)
            ? 16 * 1024 * 1024
            : themeFontMaxBytes;
        if (entry.value.isEmpty ||
            entry.value.length > limit ||
            totalBytes > 128 * 1024 * 1024) {
          throw StateError('Probe asset byte budget exceeded: ${entry.key}');
        }
        final copy = Uint8List.fromList(entry.value);
        if (imageReferences.contains(entry.key)) {
          final image = await decodeThemeImageBytes(entry.key, copy);
          decoded[entry.key] = image;
          final pixels = image.width * image.height;
          totalPixels += pixels;
          if (totalPixels > 64000000) {
            throw StateError('Probe image pixel budget exceeded: ${entry.key}');
          }
        }
        copies[entry.key] = copy;
        digests[entry.key] = sha256.convert(copy).toString();
      }
      return ThemeProbeResources._(
        sans: sansFont,
        mono: monoFont,
        user: fonts[2],
        ai: fonts[3],
        bytes: Map.unmodifiable(copies),
        images: Map.unmodifiable(decoded),
        assetSha256: Map.unmodifiable(digests),
      );
    } catch (_) {
      for (final image in decoded.values) {
        image.dispose();
      }
      rethrow;
    }
  }

  @override
  Future<Uint8List> readBytes(String reference) async {
    if (_disposed) {
      throw StateError('Probe resources are disposed');
    }
    final bytes = _bytes[reference];
    if (bytes == null) throw StateError('Probe asset is missing: $reference');
    return Uint8List.fromList(bytes);
  }

  @override
  ui.Image cloneImage(String reference) {
    if (_disposed) throw StateError('Probe resources are disposed');
    final image = _images[reference];
    if (image == null) {
      throw StateError('Decoded probe image is missing: $reference');
    }
    return image.clone();
  }

  @override
  ui.Image? tryCloneImage(String reference) {
    if (_disposed) return null;
    return _images[reference]?.clone();
  }

  void dispose() {
    if (_disposed) return;
    _disposed = true;
    for (final image in _images.values) {
      image.dispose();
    }
  }
}

class ThemeProbeRenderedReceipt {
  ThemeProbeRenderedReceipt._({
    required this.revision,
    required this.fixtureVersion,
    required this.fixtureSourceSha256,
    required this.themeSha256,
    required ThemeProbeResources resources,
  }) : fonts = List.unmodifiable(<Map<String, Object>>[
         Map.unmodifiable(resources.sans.toJson()),
         Map.unmodifiable(resources.mono.toJson()),
         Map.unmodifiable(resources.user.toJson()),
         Map.unmodifiable(resources.ai.toJson()),
       ]),
       assetSha256 = resources.assetSha256;

  final int revision;
  final String fixtureVersion;

  /// Raw fixture source hash, deliberately NOT the Rust canonical ThemeSpec hash.
  final String fixtureSourceSha256;
  final String? themeSha256;
  final List<Map<String, Object>> fonts;
  final Map<String, String> assetSha256;

  Map<String, Object?> toJson() => <String, Object?>{
    'scope': 'client-shell',
    // The fixture currently duplicates parts of the runtime-owned sidebar
    // and composer. A rendered frame is not yet a proven 1:1 export source.
    'parityVerified': false,
    'revision': revision,
    'fixtureVersion': fixtureVersion,
    'fixtureSourceSha256': fixtureSourceSha256,
    if (themeSha256 != null) 'themeSha256': themeSha256,
    'fonts': fonts,
    'assetSha256': assetSha256,
  };
}

/// Invalidates the receipt immediately on every requested revision. Only the
/// latest prepared resources and its actual rendered frame may produce a receipt.
class ThemeProbePreparationController extends ChangeNotifier {
  int _revision = 0;
  bool _disposed = false;
  ThemeProbePhase _phase = ThemeProbePhase.idle;
  ThemeProbeResources? _resources;
  ThemeProbeRenderedReceipt? _receipt;
  Object? _error;

  int get revision => _revision;
  ThemeProbePhase get phase => _phase;
  ThemeProbeResources? get resources => _resources;
  ThemeProbeRenderedReceipt? get receipt => _receipt;
  Object? get error => _error;

  void failFrame({required int revision, required Object error}) {
    if (_disposed || revision != _revision) return;
    _receipt = null;
    _error = error;
    _resources?.dispose();
    _resources = null;
    _phase = ThemeProbePhase.failed;
    notifyListeners();
  }

  Future<void> prepare({
    required int revision,
    required Future<ThemeProbeResources> Function() load,
  }) async {
    if (_disposed) {
      throw StateError('Probe controller is disposed');
    }
    if (revision <= _revision) {
      throw ArgumentError('Probe revision must increase');
    }
    _revision = revision;
    _phase = ThemeProbePhase.preparing;
    _receipt = null;
    _error = null;
    _resources?.dispose();
    _resources = null;
    notifyListeners();
    try {
      final prepared = await load();
      if (_disposed ||
          revision != _revision ||
          _phase != ThemeProbePhase.preparing) {
        prepared.dispose();
        return;
      }
      _resources = prepared;
      _phase = ThemeProbePhase.prepared;
      notifyListeners();
    } catch (error) {
      if (_disposed || revision != _revision) return;
      _error = error;
      _phase = ThemeProbePhase.failed;
      notifyListeners();
    }
  }

  /// Caller waits for Flutter's endOfFrame for this exact rendered revision.
  void markRendered({
    required int revision,
    required String fixtureVersion,
    required String fixtureSourceSha256,
    String? themeSha256,
  }) {
    if (_disposed || revision != _revision) return;
    if (_phase != ThemeProbePhase.prepared || _resources == null) {
      throw StateError('Probe resources must be prepared before rendering');
    }
    if (fixtureVersion.isEmpty ||
        !RegExp(r'^[a-f0-9]{64}$').hasMatch(fixtureSourceSha256)) {
      throw ArgumentError('Fixture metadata is invalid');
    }
    if (themeSha256 != null &&
        !RegExp(r'^[a-f0-9]{64}$').hasMatch(themeSha256)) {
      throw ArgumentError('Theme SHA-256 metadata is invalid');
    }
    _receipt = ThemeProbeRenderedReceipt._(
      revision: revision,
      fixtureVersion: fixtureVersion,
      fixtureSourceSha256: fixtureSourceSha256,
      themeSha256: themeSha256,
      resources: _resources!,
    );
    _phase = ThemeProbePhase.rendered;
    notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    _revision++;
    _receipt = null;
    _resources?.dispose();
    _resources = null;
    super.dispose();
  }
}
