// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter/foundation.dart' show visibleForTesting;
import 'package:flutter/services.dart';

import 'package:image_picker_ios/image_picker_ios.dart';
import 'package:image_picker_platform_interface/image_picker_platform_interface.dart';

import 'src/messages.g.dart';

/// An implementation of [FileSelectorPlatform] for iOS.
class FileSelectorIOS extends FileSelectorPlatform {
  /// Creates a new plugin implementation instance.
  FileSelectorIOS({
    @visibleForTesting FileSelectorApi? api,
    @visibleForTesting ImagePickerPlatform? imagePicker,
  }) : _hostApi = api ?? FileSelectorApi(),
       _imagePicker = imagePicker ?? ImagePickerIOS();

  final FileSelectorApi _hostApi;
  final ImagePickerPlatform _imagePicker;

  static const MethodChannel _directoryChannel = MethodChannel(
    'dev.flutter.packages.file_selector_ios/directory',
  );
  static const MethodChannel _saveChannel = MethodChannel(
    'dev.flutter.packages.file_selector_ios/save',
  );

  /// Registers the iOS implementation.
  static void registerWith() {
    FileSelectorPlatform.instance = FileSelectorIOS();
  }

  @override
  Future<XFile?> openFile({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    final media = await _pickMedia(acceptedTypeGroups, multiple: false);
    if (media != null) {
      return media.isEmpty ? null : media.first;
    }
    final List<String> path = await _hostApi.openFile(
      FileSelectorConfig(
        utis: _allowedUtiListFromTypeGroups(acceptedTypeGroups),
      ),
    );
    return path.isEmpty ? null : XFile(path.first);
  }

  @override
  Future<List<XFile>> openFiles({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    final media = await _pickMedia(acceptedTypeGroups, multiple: true);
    if (media != null) {
      return media;
    }
    final List<String> pathList = await _hostApi.openFile(
      FileSelectorConfig(
        utis: _allowedUtiListFromTypeGroups(acceptedTypeGroups),
        allowMultiSelection: true,
      ),
    );
    return pathList.map((String path) => XFile(path)).toList();
  }

  // Only media-only filters belong in Photos. Mixed document filters and
  // unrestricted requests continue to use Files. An empty result is cancellation;
  // null means the request is not a photo-library request.
  Future<List<XFile>?> _pickMedia(
    List<XTypeGroup>? groups, {
    required bool multiple,
  }) async {
    final utis = _allowedUtiListFromTypeGroups(groups);
    const images = <String>{
      'public.image',
      'public.jpeg',
      'public.png',
      'com.compuserve.gif',
      'org.webmproject.webp',
      'com.microsoft.bmp',
      'public.heic',
      'public.heif',
      'public.tiff',
    };
    const videos = <String>{
      'public.movie',
      'public.video',
      'public.mpeg-4',
      'com.apple.quicktime-movie',
      'public.avi',
      'org.webmproject.matroska',
      'public.3gpp',
    };
    if (utis.isEmpty ||
        !utis.every((uti) => images.contains(uti) || videos.contains(uti))) {
      return null;
    }
    if (utis.every(images.contains)) {
      if (multiple) {
        return _imagePicker.getMultiImageWithOptions(
          options: const MultiImagePickerOptions(
            imageOptions: ImageOptions(requestFullMetadata: false),
          ),
        );
      }
      final image = await _imagePicker.getImageFromSource(
        source: ImageSource.gallery,
        options: const ImagePickerOptions(requestFullMetadata: false),
      );
      return image == null ? <XFile>[] : <XFile>[image];
    }
    if (utis.every(videos.contains)) {
      if (multiple) {
        return _imagePicker.getMultiVideoWithOptions();
      }
      final video = await _imagePicker.getVideo(source: ImageSource.gallery);
      return video == null ? <XFile>[] : <XFile>[video];
    }
    return _imagePicker.getMedia(
      options: MediaOptions(
        allowMultiple: multiple,
        imageOptions: const ImageOptions(requestFullMetadata: false),
      ),
    );
  }

  /// Opens a scoped file reference so Flutter can count its reads without an opaque import copy.
  @override
  Future<XFile?> openFileWithProgress({
    required void Function(int completed, int? total) onProgress,
    List<XTypeGroup>? acceptedTypeGroups,
  }) async {
    final path = await _saveChannel.invokeMethod<String>('openFileReference', {
      'utis': _allowedUtiListFromTypeGroups(acceptedTypeGroups),
    });
    return path == null ? null : XFile(path);
  }

  /// Selects a scoped output folder so the storage Host can report each written chunk.
  @override
  Future<FileSaveLocation?> saveGeneratedFileWithProgress({
    required Future<XFile> Function() generate,
    required void Function() onChooseDestination,
    required Future<void> Function(XFile file, String destination) write,
    required void Function(int completed, int total) onProgress,
    List<XTypeGroup>? acceptedTypeGroups,
    SaveDialogOptions options = const SaveDialogOptions(),
  }) async {
    final directory = await getDirectoryPath(
      initialDirectory: options.initialDirectory,
    );
    if (directory == null) return null;
    final file = await generate();
    final destination = Uri.directory(directory)
        .resolveUri(Uri(pathSegments: [options.suggestedName ?? file.name]))
        .toFilePath();
    onProgress(0, await file.length());
    await write(file, destination);
    return FileSaveLocation(destination);
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

  /// Saves [file] through the iOS document export picker.
  ///
  /// iOS's upstream file_selector implementation only exposes import and
  /// directory pickers. The repository-owned implementation passes the bytes
  /// to native code so the export picker can create the destination document.
  Future<FileSaveLocation?> saveFile({
    required XFile file,
    List<XTypeGroup>? acceptedTypeGroups,
    SaveDialogOptions options = const SaveDialogOptions(),
  }) async {
    final String? path = await _saveChannel
        .invokeMethod<String>('saveFile', <String, Object?>{
          'bytes': await file.readAsBytes(),
          'name': options.suggestedName ?? file.name,
          'mimeType': file.mimeType ?? 'application/octet-stream',
          'initialDirectory': options.initialDirectory,
        });
    return path == null ? null : FileSaveLocation(path);
  }

  /// Compatibility implementation for callers still using the deprecated API.
  @override
  Future<String?> getSavePath({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? suggestedName,
    String? confirmButtonText,
  }) async {
    return _saveChannel.invokeMethod<String>('saveFile', <String, Object?>{
      'bytes': Uint8List(0),
      'name': suggestedName ?? 'untitled',
      'mimeType': 'application/octet-stream',
      'initialDirectory': initialDirectory,
    });
  }

  /// Opens the native iOS document picker in directory mode.
  ///
  /// iOS's upstream file_selector implementation only supports files. This
  /// repository-owned implementation keeps the standard platform API while
  /// adding directory selection through a small method channel.
  @override
  Future<String?> getDirectoryPath({
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    final List<String>? paths = await _directoryChannel
        .invokeListMethod<String>('pickDirectory', <String, Object?>{
          'initialDirectory': initialDirectory,
          'multiple': false,
        });
    return paths == null || paths.isEmpty ? null : paths.first;
  }

  /// Opens the native iOS document picker for multiple directories.
  @override
  Future<List<String>> getDirectoryPaths({
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    final List<String>? paths = await _directoryChannel
        .invokeListMethod<String>('pickDirectory', <String, Object?>{
          'initialDirectory': initialDirectory,
          'multiple': true,
        });
    return paths ?? <String>[];
  }

  // Converts the type group list into a list of all allowed UTIs, since
  // iOS doesn't support filter groups.
  List<String> _allowedUtiListFromTypeGroups(List<XTypeGroup>? typeGroups) {
    // iOS requires a list of allowed types, so allowing all is expressed via
    // a root type rather than an empty list.
    const allowAny = <String>['public.data'];

    if (typeGroups == null || typeGroups.isEmpty) {
      return allowAny;
    }
    final allowedUTIs = <String>[];
    for (final XTypeGroup typeGroup in typeGroups) {
      // If any group allows everything, no filtering should be done.
      if (typeGroup.allowsAny) {
        return allowAny;
      }
      final uniformTypeIdentifiers = typeGroup.uniformTypeIdentifiers;
      if (uniformTypeIdentifiers?.isNotEmpty ?? false) {
        allowedUTIs.addAll(uniformTypeIdentifiers!);
        continue;
      }

      // The app commonly specifies filters by file extension. iOS's native
      // picker only accepts UTIs, so translate the extensions here instead of
      // throwing before the picker can be opened. Unknown app-specific
      // extensions use public.data; the caller still validates the file after
      // selection and the picker remains usable for custom formats.
      final extensions = typeGroup.extensions;
      if (extensions?.isNotEmpty ?? false) {
        allowedUTIs.addAll(extensions!.map(_utiForExtension));
        continue;
      }

      // MIME types and web wildcards do not have a lossless representation in
      // the legacy UIDocumentPicker API. A broad data UTI keeps the picker
      // usable; callers still validate the selected file where required.
      if (typeGroup.mimeTypes?.isNotEmpty ?? false) {
        allowedUTIs.addAll(typeGroup.mimeTypes!.map(_utiForMimeType));
        continue;
      }
      if (typeGroup.webWildCards?.isNotEmpty ?? false) {
        allowedUTIs.add('public.data');
        continue;
      }

      // Be defensive about future XTypeGroup fields. Opening the picker is
      // preferable to propagating an argument error from a user tap.
      allowedUTIs.add('public.data');
    }
    return allowedUTIs;
  }

  static String _utiForExtension(String extension) {
    switch (extension.toLowerCase()) {
      case 'json':
        return 'public.json';
      case 'zip':
        return 'public.zip-archive';
      case 'jpg':
      case 'jpeg':
        return 'public.jpeg';
      case 'png':
        return 'public.png';
      case 'gif':
        return 'com.compuserve.gif';
      case 'webp':
        return 'org.webmproject.webp';
      case 'bmp':
        return 'com.microsoft.bmp';
      case 'heif':
        return 'public.heif';
      case 'heic':
        return 'public.heic';
      case 'tif':
      case 'tiff':
        return 'public.tiff';
      case 'mp4':
      case 'm4v':
        return 'public.mpeg-4';
      case 'mov':
        return 'com.apple.quicktime-movie';
      case 'avi':
        return 'public.avi';
      case 'mkv':
      case 'webm':
        return 'org.webmproject.matroska';
      case '3gp':
        return 'public.3gpp';
      case 'ttf':
        return 'public.truetype-font';
      case 'otf':
        return 'public.opentype-font';
      case 'ttc':
        return 'public.truetype-collection';
      case 'txt':
        return 'public.plain-text';
      case 'js':
      case 'mjs':
        return 'com.netscape.javascript-source';
      default:
        return 'public.data';
    }
  }

  static String _utiForMimeType(String mimeType) {
    switch (mimeType.toLowerCase()) {
      case 'application/json':
        return 'public.json';
      case 'application/zip':
      case 'application/x-zip-compressed':
        return 'public.zip-archive';
      case 'text/plain':
        return 'public.plain-text';
      case 'image/jpeg':
      case 'image/jpg':
        return 'public.jpeg';
      case 'image/png':
        return 'public.png';
      case 'image/gif':
        return 'com.compuserve.gif';
      case 'image/webp':
        return 'org.webmproject.webp';
      case 'video/mp4':
        return 'public.mpeg-4';
      case 'video/quicktime':
        return 'com.apple.quicktime-movie';
      case 'image/*':
        return 'public.image';
      case 'video/*':
        return 'public.movie';
      case 'audio/*':
        return 'public.audio';
      default:
        return 'public.data';
    }
  }
}
