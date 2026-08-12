// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';

/// Saves byte payloads through the registered file-selector platform implementation.
class FileSaveService {
  /// Prevents instantiation of the file save service.
  FileSaveService._();

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
