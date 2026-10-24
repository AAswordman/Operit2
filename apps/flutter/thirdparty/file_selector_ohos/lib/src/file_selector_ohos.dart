// Copyright 2013 The Flutter Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// ignore_for_file: public_member_api_docs

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter/cupertino.dart';
import 'package:flutter/services.dart';

import 'file_selector_api.g.dart';
import 'uri_xfile.dart';

/// An implementation of [FileSelectorPlatform] for OpenHarmony.
class FileSelectorOhos extends FileSelectorPlatform {
  FileSelectorOhos({@visibleForTesting FileSelectorApi? api})
      : _api = api ?? FileSelectorApi();

  final FileSelectorApi _api;
  static const MethodChannel _saveChannel = MethodChannel(
    'dev.flutter.packages.file_selector_ohos/save',
  );

  /// Saves into a document URI while the platform counts each completed write.
  @override
  Future<FileSaveLocation?> saveGeneratedFileWithProgress({
    required Future<XFile> Function() generate,
    required void Function() onChooseDestination,
    required Future<void> Function(XFile file, String destination) write,
    required void Function(int completed, int total) onProgress,
    List<XTypeGroup>? acceptedTypeGroups,
    SaveDialogOptions options = const SaveDialogOptions(),
  }) async {
    final location = await getSaveLocation(
        acceptedTypeGroups: acceptedTypeGroups, options: options);
    if (location == null) return null;
    final file = await generate();
    _saveChannel.setMethodCallHandler((call) async {
      if (call.method != 'saveProgress')
        throw StateError('Unknown document save event: ${call.method}');
      final progress = Map<String, Object?>.from(call.arguments as Map);
      onProgress(progress['completed']! as int, progress['total']! as int);
    });
    try {
      await _saveChannel.invokeMethod<void>('copyFileFromPathWithProgress', {
        'sourcePath': file.path,
        'destinationUri': location.path,
      });
      return location;
    } finally {
      _saveChannel.setMethodCallHandler(null);
    }
  }

  /// Copies a host-backed snapshot into the selected document without reading its bytes in Dart.
  @override
  Future<FileSaveLocation?> saveFileReference({
    required XFile file,
    List<XTypeGroup>? acceptedTypeGroups,
    SaveDialogOptions options = const SaveDialogOptions(),
  }) async {
    final location = await getSaveLocation(
        acceptedTypeGroups: acceptedTypeGroups, options: options);
    if (location == null) {
      return null;
    }
    await _saveChannel.invokeMethod<void>('copyFileFromPath', <String, Object?>{
      'sourcePath': file.path,
      'destinationUri': location.path,
    });
    return location;
  }

  /// Registers this class as the implementation of the file_selector platform interface.
  static void registerWith() {
    FileSelectorPlatform.instance = FileSelectorOhos();
  }

  @override
  Future<XFile?> openFile({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    final FileResponse? file = await _api.openFile(
      initialDirectory,
      _fileTypesFromTypeGroups(acceptedTypeGroups),
    );
    return file == null ? null : _xFileFromFileResponse(file);
  }

  @override
  Future<List<XFile>> openFiles({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    final List<FileResponse?> files = await _api.openFiles(
      initialDirectory,
      _fileTypesFromTypeGroups(acceptedTypeGroups),
    );
    return files
        .cast<FileResponse>()
        .map<XFile>(_xFileFromFileResponse)
        .toList();
  }

  /// Opens the OpenHarmony document save dialog and returns its writable URI.
  @override
  Future<String?> getSavePath({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? suggestedName,
    String? confirmButtonText,
  }) {
    return _api.getSavePath(
      initialDirectory,
      suggestedName,
      _fileTypesFromTypeGroups(acceptedTypeGroups),
    );
  }

  @override
  Future<String?> getDirectoryPath({
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    return _api.getDirectoryPath(initialDirectory);
  }

  XFile _xFileFromFileResponse(FileResponse file) {
    return UriXFile(
      file.path,
      name: file.name,
      length: file.size,
      mimeType: file.mimeType,
    );
  }

  FileTypes _fileTypesFromTypeGroups(List<XTypeGroup>? typeGroups) {
    if (typeGroups == null) {
      return FileTypes(extensions: <String>[], mimeTypes: <String>[]);
    }

    final Set<String> mimeTypes = <String>{};
    final Set<String> extensions = <String>{};

    for (final XTypeGroup group in typeGroups) {
      if (!group.allowsAny &&
          group.mimeTypes == null &&
          group.extensions == null) {
        throw ArgumentError(
          'Provided type group $group does not allow all files, but does not '
          'set any of the Android supported filter categories. At least one of '
          '"extensions" or "mimeTypes" must be non-empty for Android.',
        );
      }

      mimeTypes.addAll(group.mimeTypes ?? <String>{});
      extensions.addAll(group.extensions ?? <String>{});
    }

    return FileTypes(
      mimeTypes: mimeTypes.toList(),
      extensions: extensions.toList(),
    );
  }
}
