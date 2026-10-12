// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/services.dart';
import 'package:operit_theme/operit_theme.dart';

import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../core/host/SelectedFileInput.dart';

const int themeArchiveMaxBytes = 64 * 1024 * 1024;

/// Bounded system-document input; no Dart filesystem or HTTP implementation.
class ThemeArchiveInput {
  const ThemeArchiveInput(this.name, this.bytes, this.validation);

  final String name;
  final Uint8List bytes;
  final core_proxy.ThemeValidationReceipt validation;
}

class ThemeLibraryService {
  ThemeLibraryService(this.clients);

  final GeneratedCoreProxyClients clients;
  final ThemeFontRegistry _fonts = ThemeFontRegistry();
  Future<BuiltinThemeFonts>? _builtinFonts;
  final Map<String, Uint8List> _archiveBytesCache = <String, Uint8List>{};
  final Map<String, Uint8List?> _backgroundThumbnailCache =
      <String, Uint8List?>{};
  final Map<String, Future<Uint8List?>> _backgroundThumbnailFutures =
      <String, Future<Uint8List?>>{};

  void _cacheArchiveBytes(String sha256, Uint8List bytes) {
    _archiveBytesCache.remove(sha256);
    if (_archiveBytesCache.length >= 2) {
      _archiveBytesCache.remove(_archiveBytesCache.keys.first);
    }
    _archiveBytesCache[sha256] = bytes;
  }

  /// Returns cached custom background image bytes for [archiveSha256] if already loaded.
  Uint8List? cachedBackgroundImageBytes(String archiveSha256) =>
      _backgroundThumbnailCache[archiveSha256];

  /// Reports whether custom background image bytes for [archiveSha256] have been resolved.
  bool hasResolvedBackgroundImage(String archiveSha256) =>
      _backgroundThumbnailCache.containsKey(archiveSha256);

  /// Reads and caches the custom background image declared in theme.json
  /// (background.asset) of the specified theme package.
  Future<Uint8List?> readBackgroundImageBytes({
    required String themeId,
    required String archiveSha256,
  }) {
    if (_backgroundThumbnailCache.containsKey(archiveSha256)) {
      return Future<Uint8List?>.value(_backgroundThumbnailCache[archiveSha256]);
    }
    return _backgroundThumbnailFutures.putIfAbsent(archiveSha256, () async {
      try {
        var bytes = _archiveBytesCache[archiveSha256];
        if (bytes == null) {
          bytes = await clients.application.readThemePackage(
            themeId: themeId,
            archiveSha256: archiveSha256,
          );
          _cacheArchiveBytes(archiveSha256, bytes);
        }
        final payload = await clients.application.previewThemeArchive(
          archiveBytes: bytes,
        );
        if (payload.validation.archiveSha256 != archiveSha256 ||
            payload.themeId != themeId) {
          throw StateError('主题背景缩略图回执与所选包不一致');
        }
        final rawTheme = jsonDecode(payload.themeJson) as Map<String, dynamic>;
        final background = rawTheme['background'] as Map<String, dynamic>?;
        final assetPath = background?['asset'] as String?;
        if (assetPath == null || assetPath.isEmpty) {
          _backgroundThumbnailCache[archiveSha256] = null;
          return null;
        }
        for (final asset in payload.assets) {
          if (asset.path == assetPath) {
            final view = asset.bytes.asUnmodifiableView();
            _backgroundThumbnailCache[archiveSha256] = view;
            return view;
          }
        }
        _backgroundThumbnailCache[archiveSha256] = null;
        return null;
      } finally {
        _backgroundThumbnailFutures.remove(archiveSha256);
      }
    });
  }

  Future<ThemeArchiveInput?> inspectPickedArchive() async {
    final file = await SelectedFileInput.pickSnapshot(
      acceptedTypeGroups: const <XTypeGroup>[
        XTypeGroup(label: 'Operit theme', extensions: <String>['optheme']),
      ],
    );
    if (file == null) return null;
    try {
      if (!file.name.toLowerCase().endsWith('.optheme')) {
        throw FormatException('请选择 .optheme 主题包');
      }
      final byteLength = file.byteLength;
      if (byteLength == null || byteLength <= 0 || byteLength > themeArchiveMaxBytes) {
        throw FormatException('主题包必须为 1 字节至 64 MiB，且文件大小必须可读取');
      }
      final builder = BytesBuilder(copy: false);
      await for (final chunk in file.chunks()) {
        if (builder.length + chunk.length > byteLength ||
            builder.length + chunk.length > themeArchiveMaxBytes) {
          throw FormatException('主题包流长度超过声明大小');
        }
        builder.add(chunk);
      }
      final bytes = builder.takeBytes();
      if (bytes.length != byteLength) {
        throw FormatException('主题包流长度与声明大小不一致');
      }
      final validation = await clients.application.inspectThemeArchive(archiveBytes: bytes);
      final view = bytes.asUnmodifiableView();
      _cacheArchiveBytes(validation.archiveSha256, view);
      return ThemeArchiveInput(file.name, view, validation);
    } finally {
      await file.close();
    }
  }

  /// Removes a theme package from the Space-owned library and clears its byte cache.
  Future<void> deleteTheme(String themeId, String archiveSha256) async {
    await clients.application.deleteTheme(
      themeId: themeId,
      archiveSha256: archiveSha256,
    );
    _archiveBytesCache.remove(archiveSha256);
    _backgroundThumbnailCache.remove(archiveSha256);
    _backgroundThumbnailFutures.remove(archiveSha256);
  }

  /// Core revalidates the sealed archive. This does not prepare or apply it.
  Future<core_proxy.ThemeInstallReceipt> importArchive(ThemeArchiveInput input) async {
    final receipt = await clients.application.importThemeArchive(
      archiveBytes: input.bytes,
      expectedArchiveBytes: input.validation.archiveBytes,
      expectedArchiveSha256: input.validation.archiveSha256,
    );
    if (receipt.selectionChanged || receipt.selection != null ||
        (receipt.status != core_proxy.ThemeInstallStatus.imported &&
         receipt.status != core_proxy.ThemeInstallStatus.alreadyPresent) ||
        receipt.validation.archiveSha256 != input.validation.archiveSha256 ||
        receipt.validation.archiveBytes != input.bytes.length) {
      throw StateError('主题导入回执与已确认包不一致');
    }
    return receipt;
  }

  /// Every use, including restored node choices, re-enters the Rust validator.
  Future<PreparedThemeVisual> prepare(String themeId, String archiveSha256, {String? expectedVersion}) async {
    var bytes = _archiveBytesCache[archiveSha256];
    if (bytes == null) {
      bytes = await clients.application.readThemePackage(
        themeId: themeId, archiveSha256: archiveSha256,
      );
      _cacheArchiveBytes(archiveSha256, bytes);
    }
    final payload = await clients.application.previewThemeArchive(archiveBytes: bytes);
    if (payload.validation.archiveSha256 != archiveSha256 ||
        payload.validation.archiveBytes != bytes.length ||
        payload.validation.validatorVersion != 1 || payload.themeId != themeId ||
        (expectedVersion != null && payload.version != expectedVersion)) {
      throw StateError('主题预览回执与所选包不一致');
    }
    final expected = <String, String>{
      for (final file in payload.validation.files)
        if (file.path == 'preview.png' || file.path.startsWith('assets/')) file.path: file.sha256,
    };
    final assets = <String, Uint8List>{};
    for (final asset in payload.assets) {
      if (assets.containsKey(asset.path) || !expected.containsKey(asset.path)) {
        throw StateError('主题预览资源重复或未登记：${asset.path}');
      }
      assets[asset.path] = asset.bytes;
    }
    if (assets.length != expected.length) throw StateError('主题预览资源不完整');
    final rawThemeMap = jsonDecode(payload.themeJson) as Map<String, dynamic>;
    final bgAssetPath =
        (rawThemeMap['background'] as Map<String, dynamic>?)?['asset'] as String?;
    _backgroundThumbnailCache[archiveSha256] =
        bgAssetPath != null ? assets[bgAssetPath]?.asUnmodifiableView() : null;
    final spec = ThemeVisualSpecV1.fromCheckedContract(rawThemeMap);
    final builtinFonts = await (_builtinFonts ??= BuiltinThemeFonts.load(bundle: rootBundle));
    return PreparedThemeVisual.prepare(
      spec: spec, assets: assets, assetSha256: expected,
      builtinFonts: builtinFonts, fontRegistry: _fonts,
    );
  }
}
