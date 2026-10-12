import 'dart:convert';
import 'dart:js_interop';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';

@JS('operitThemeContractInitialize')
external JSPromise<JSAny?> _initialize(JSString rendererReleaseId);

Future<void> initializeThemeContractWorker(String rendererReleaseId) async {
  await _initialize(rendererReleaseId.toJS).toDart;
}

@JS('operitThemeContractValidate')
external JSPromise<JSAny?> _validate(
  JSString rendererReleaseId,
  JSString themeJson,
  JSArray<JSAny?> assets,
);

@JS('operitThemeContractClose')
external void closeThemeContractWorker();

/// This value is constructed from the preview's own Rust/WASM worker result,
/// never from a parent page's claim that it checked a hash or a font.
class CheckedThemePreviewInput {
  CheckedThemePreviewInput._({
    required this.canonicalThemeJson,
    required this.themeSha256,
    required this.assets,
    required this.assetSha256,
  });

  final String canonicalThemeJson;
  final String themeSha256;
  final Map<String, Uint8List> assets;
  final Map<String, String> assetSha256;

  static Future<CheckedThemePreviewInput> validate({
    required String rendererReleaseId,
    required String themeJson,
    required Map<String, Uint8List> assets,
    required Map<String, String> mediaTypes,
  }) async {
    final copies = <String, Uint8List>{
      for (final entry in assets.entries)
        entry.key: Uint8List.fromList(entry.value).asUnmodifiableView(),
    };
    final response = (await _validate(
      rendererReleaseId.toJS,
      themeJson.toJS,
      <JSAny?>[
        for (final entry in copies.entries)
          <String, Object?>{
            'path': entry.key,
            'mediaType': mediaTypes[entry.key],
            'bytes': entry.value.toJS,
          }.jsify(),
      ].toJS,
    ).toDart)?.dartify();
    if (response is! Map || response['ok'] != true) {
      throw FormatException(
        'Rust/WASM preview validation failed: ${jsonEncode(response)}',
      );
    }
    final canonical = response['canonicalThemeJson'];
    final digest = response['themeSha256'];
    final receipt = response['receipt'];
    if (canonical is! String ||
        digest is! String ||
        receipt is! Map ||
        utf8.encode(canonical).length > 256 * 1024 ||
        sha256.convert(utf8.encode(canonical)).toString() != digest ||
        receipt['validatorVersion'] != 1 ||
        receipt['trust'] != 'untrusted' ||
        receipt['themeSha256'] != digest ||
        receipt['files'] is! List) {
      throw FormatException('Rust/WASM preview validation receipt is invalid');
    }
    final declarations = receipt['files'] as List;
    final digests = <String, String>{};
    for (final raw in declarations) {
      if (raw is! Map || raw['path'] is! String || raw['sha256'] is! String) {
        throw FormatException('Preview asset receipt is invalid');
      }
      final path = raw['path'] as String;
      final bytes = copies[path];
      if (bytes == null ||
          digests.containsKey(path) ||
          raw['bytes'] != bytes.length ||
          sha256.convert(bytes).toString() != raw['sha256']) {
        throw FormatException(
          'Preview asset receipt differs from immutable bytes: $path',
        );
      }
      digests[path] = raw['sha256'] as String;
    }
    if (digests.length != copies.length) {
      throw FormatException('Preview asset receipt is incomplete');
    }
    return CheckedThemePreviewInput._(
      canonicalThemeJson: canonical,
      themeSha256: digest,
      assets: Map.unmodifiable(copies),
      assetSha256: Map.unmodifiable(digests),
    );
  }
}
