// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// ignore_for_file: public_member_api_docs

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'file_selector_api.g.dart';
import 'types/native_illegal_argument_exception.dart';

/// An implementation of [FileSelectorPlatform] for Android.
class FileSelectorAndroid extends FileSelectorPlatform {
  static const MethodChannel _saveChannel = MethodChannel(
    'dev.flutter.packages.file_selector_android/save',
  );

  FileSelectorAndroid({@visibleForTesting FileSelectorApi? api})
    : _api = api ?? FileSelectorApi();

  final FileSelectorApi _api;

  /// Registers this class as the implementation of the file_selector platform interface.
  static void registerWith() {
    FileSelectorPlatform.instance = FileSelectorAndroid();
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
    final List<FileResponse> files = await _api.openFiles(
      initialDirectory,
      _fileTypesFromTypeGroups(acceptedTypeGroups),
    );
    return files.map<XFile>(_xFileFromFileResponse).toList();
  }

  /// Receives source-copy counters while Android stages a selected document off its UI thread.
  @override
  Future<XFile?> openFileWithProgress({
    required void Function(int completed, int? total) onProgress,
    List<XTypeGroup>? acceptedTypeGroups,
  }) async {
    final types = _fileTypesFromTypeGroups(acceptedTypeGroups);
    if (types.mimeTypes.isEmpty)
      throw ArgumentError('Streaming document selection requires MIME types');
    _saveChannel.setMethodCallHandler((call) async {
      if (call.method != 'readProgress')
        throw StateError('Unknown source event: ${call.method}');
      final values = Map<String, Object?>.from(call.arguments as Map);
      onProgress(values['completed']! as int, values['total'] as int?);
    });
    try {
      final result = await _saveChannel.invokeMapMethod<String, Object?>(
        'openFileWithProgress',
        {'mimeTypes': types.mimeTypes},
      );
      if (result == null) return null;
      return XFile(result['path']! as String, length: result['size']! as int);
    } finally {
      _saveChannel.setMethodCallHandler(null);
    }
  }

  /// Releases only the Android cache copy owned by the selected document operation.
  @override
  Future<void> releaseFile(XFile file) async {
    await _saveChannel.invokeMethod<void>('releaseSelectedFile', {
      'path': file.path,
    });
  }

  /// Reports bounded SAF document writes through the existing platform save channel.
  @override
  Future<FileSaveLocation?> saveGeneratedFileWithProgress({
    required Future<XFile> Function() generate,
    required void Function() onChooseDestination,
    required Future<void> Function(XFile file, String destination) write,
    required void Function(int completed, int total) onProgress,
    List<XTypeGroup>? acceptedTypeGroups,
    SaveDialogOptions options = const SaveDialogOptions(),
  }) async {
    final file = await generate();
    _saveChannel.setMethodCallHandler((call) async {
      if (call.method != 'saveProgress')
        throw StateError('Unknown save event: ${call.method}');
      final values = Map<String, Object?>.from(call.arguments as Map);
      onProgress(values['completed']! as int, values['total']! as int);
    });
    try {
      final path = await _saveChannel
          .invokeMethod<String>('saveFileFromPath', <String, Object?>{
            'sourcePath': file.path,
            'name': options.suggestedName ?? file.name,
            'mimeType': file.mimeType ?? 'application/octet-stream',
            'initialDirectory': options.initialDirectory,
            'reportProgress': true,
          });
      return path == null ? null : FileSaveLocation(path);
    } finally {
      _saveChannel.setMethodCallHandler(null);
    }
  }

  /// Saves a host file through the native picker without reading its bytes in Dart.
  @override
  Future<FileSaveLocation?> saveFileReference({
    required XFile file,
    List<XTypeGroup>? acceptedTypeGroups,
    SaveDialogOptions options = const SaveDialogOptions(),
  }) async {
    final path = await _saveChannel
        .invokeMethod<String>('saveFileFromPath', <String, Object?>{
          'sourcePath': file.path,
          'name': options.suggestedName ?? file.name,
          'mimeType': file.mimeType ?? 'application/octet-stream',
          'initialDirectory': options.initialDirectory,
        });
    return path == null ? null : FileSaveLocation(path);
  }

  /// Saves [file] through Android's Storage Access Framework document creator.
  @override
  Future<FileSaveLocation?> saveFile({
    required XFile file,
    List<XTypeGroup>? acceptedTypeGroups,
    SaveDialogOptions options = const SaveDialogOptions(),
  }) async {
    final path = await _saveChannel
        .invokeMethod<String>('saveFile', <String, Object?>{
          'bytes': await file.readAsBytes(),
          'name': options.suggestedName ?? file.name,
          'mimeType': file.mimeType ?? 'application/octet-stream',
          'initialDirectory': options.initialDirectory,
        });
    return path == null ? null : FileSaveLocation(path);
  }

  @override
  Future<String?> getDirectoryPath({
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    return _api.getDirectoryPath(initialDirectory);
  }

  XFile _xFileFromFileResponse(FileResponse file) {
    if (file.fileSelectorNativeException != null) {
      _resolveErrorCodeAndMaybeThrow(file.fileSelectorNativeException!);
    }
    // Android has already made a bounded file-backed cache copy. Reading is lazy;
    // selection must not retain an in-memory copy of the entire selected file.
    return XFile(
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

    final mimeTypes = <String>{};
    final extensions = <String>{};

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

  /// Translates a [FileSelectorExceptionCode] to its corresponding error and
  /// handles throwing.
  void _resolveErrorCodeAndMaybeThrow(
    FileSelectorNativeException fileSelectorNativeException,
  ) {
    switch (fileSelectorNativeException.fileSelectorExceptionCode) {
      case FileSelectorExceptionCode.illegalArgumentException:
        throw NativeIllegalArgumentException(
          fileSelectorNativeException.message,
        );
      case (FileSelectorExceptionCode.illegalStateException ||
          FileSelectorExceptionCode.ioException ||
          FileSelectorExceptionCode.securityException):
      // unused for now
    }
  }
}
