// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart';

import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';

class ThemeAssetImport {
  /// Creates imported theme asset metadata.
  const ThemeAssetImport({
    required this.storagePath,
    required this.bytes,
    required this.fileName,
  });

  final String storagePath;
  final Uint8List bytes;
  final String fileName;
}

class ThemeAssetStore implements ThemeAssetResolver {
  /// Creates a store backed by the runtime storage repository.
  ThemeAssetStore({
    GeneratedRepositoryRuntimeStorageRepositoryCoreProxy? runtimeStorage,
  }) : _runtimeStorage =
           runtimeStorage ??
           const GeneratedCoreProxyClients(
             ProxyCoreRuntimeBridge(),
           ).repositoryRuntimeStorageRepository;

  final GeneratedRepositoryRuntimeStorageRepositoryCoreProxy _runtimeStorage;

  /// Imports one selected file into the runtime theme asset pool.
  Future<ThemeAssetImport> importFile(XFile file) async {
    final bytes = await file.readAsBytes();
    final fileName = _themeAssetFileName(file);
    return importBytes(bytes: bytes, fileName: fileName);
  }

  /// Imports prepared bytes into the runtime theme asset pool.
  Future<ThemeAssetImport> importBytes({
    required Uint8List bytes,
    required String fileName,
  }) async {
    final extension = _themeAssetExtension(fileName);
    final directory = await _runtimeStorage.themeAssetsDirPath();
    final digest = sha256.convert(bytes).toString().substring(0, 16);
    final storagePath = '$directory/$digest.$extension';
    await _runtimeStorage.writeBase64(
      path: storagePath,
      base64Content: base64Encode(bytes),
    );
    return ThemeAssetImport(
      storagePath: storagePath,
      bytes: bytes,
      fileName: fileName,
    );
  }

  /// Reads imported theme asset bytes from runtime storage.
  @override
  Future<Uint8List> readBytes(String storagePath) async {
    final base64Content = await _runtimeStorage.readBase64(path: storagePath);
    if (base64Content == null) {
      throw StateError('theme asset is missing: $storagePath');
    }
    return base64Decode(base64Content);
  }
}

class ThemeAssetImage extends StatelessWidget {
  /// Creates an image widget backed by a runtime theme asset.
  const ThemeAssetImage({
    super.key,
    required this.storagePath,
    required this.fit,
  });

  final String storagePath;
  final BoxFit fit;

  static final ThemeAssetStore _runtimeStore = ThemeAssetStore();

  @override
  Widget build(BuildContext context) {
    final scoped = ThemeAssetScope.of(context);
    final resolver = storagePath.startsWith('assets/') ? scoped : _runtimeStore;
    return ResolvedThemeImage(
      resolver: resolver,
      reference: storagePath,
      fit: fit,
    );
  }
}

/// Returns the browser video MIME type for one imported theme asset.
String themeAssetVideoMimeType(String storagePath) {
  final extension = _themeAssetExtension(storagePath);
  return switch (extension) {
    'mp4' => 'video/mp4',
    'mov' => 'video/quicktime',
    'm4v' => 'video/x-m4v',
    'webm' => 'video/webm',
    'mkv' => 'video/x-matroska',
    'avi' => 'video/x-msvideo',
    _ => throw StateError('theme video extension is not supported: $extension'),
  };
}

/// Returns the selected file name used for theme asset validation.
String _themeAssetFileName(XFile file) {
  final name = file.name.trim();
  if (name.isEmpty) {
    throw StateError('theme asset file name is empty');
  }
  return name;
}

/// Returns a checked lower-case extension for one theme asset name.
String _themeAssetExtension(String fileName) {
  final normalized = fileName.replaceAll('\\', '/');
  final slashIndex = normalized.lastIndexOf('/');
  final baseName = normalized.substring(slashIndex + 1).toLowerCase();
  final dotIndex = baseName.lastIndexOf('.');
  if (dotIndex <= 0 || dotIndex == baseName.length - 1) {
    throw StateError('theme asset extension is missing: $fileName');
  }
  final extension = baseName.substring(dotIndex + 1);
  const supportedExtensions = <String>{
    'jpg',
    'jpeg',
    'png',
    'svg',
    'webp',
    'bmp',
    'gif',
    'mp4',
    'mov',
    'm4v',
    'webm',
    'mkv',
    'avi',
    'ttf',
    'otf',
    'ttc',
  };
  if (!supportedExtensions.contains(extension)) {
    throw StateError('theme asset extension is not supported: $extension');
  }
  return extension;
}
