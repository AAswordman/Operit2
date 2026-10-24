// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';

/// Saves payloads and host-backed files through the registered file-selector implementation.
class FileSaveService {
  /// Prevents instantiation of the file save service.
  FileSaveService._();

  /// Saves generated data with progress supplied by the owning storage Host.
  static Future<String?> saveGeneratedFileWithProgress({
    required Future<XFile> Function() generate,
    required void Function() onChooseDestination,
    required Future<void> Function(XFile file, String destination) write,
    required void Function(int completed, int total) onProgress,
    required String name,
    required List<XTypeGroup> acceptedTypeGroups,
  }) async {
    final location = await FileSelectorPlatform.instance
        .saveGeneratedFileWithProgress(
          generate: generate,
          onChooseDestination: onChooseDestination,
          write: write,
          onProgress: onProgress,
          acceptedTypeGroups: acceptedTypeGroups,
          options: SaveDialogOptions(suggestedName: name),
        );
    return location?.path;
  }

  /// Saves a generated host file without materializing its bytes in Flutter.
  static Future<String?> saveGeneratedFile({
    required Future<XFile> Function() generate,
    required String name,
    required List<XTypeGroup> acceptedTypeGroups,
  }) async {
    final location = await FileSelectorPlatform.instance.saveGeneratedFile(
      generate: generate,
      acceptedTypeGroups: acceptedTypeGroups,
      options: SaveDialogOptions(suggestedName: name),
    );
    return location?.path;
  }

  /// Saves [bytes] to a native location selected by the user.
  static Future<String?> saveBytes({
    required Uint8List bytes,
    required String name,
    required String mimeType,
    required List<XTypeGroup> acceptedTypeGroups,
  }) async {
    final location = await FileSelectorPlatform.instance.saveFile(
      file: XFile.fromData(bytes, name: name, mimeType: mimeType),
      acceptedTypeGroups: acceptedTypeGroups,
      options: SaveDialogOptions(suggestedName: name),
    );
    return location?.path;
  }
}
