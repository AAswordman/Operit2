// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';

import 'FileSaveService.dart';

/// Saves Compose exports through Flutter's registered cross-platform file selector.
class ComposeDslFileSaveService {
  /// Prevents instantiation of this owner service.
  ComposeDslFileSaveService._();

  /// Saves the exact text or binary export and reports cancellation separately from errors.
  static Future<String> save(String requestJson) async {
    final request = jsonDecode(requestJson) as Map<String, dynamic>;
    final options = request['options'] as Map<String, dynamic>;
    final name = options['suggestedName'] as String;
    final content = options['content'];
    final contentBase64 = options['contentBase64'];
    if ((content == null) == (contentBase64 == null) ||
        (content != null && content is! String) ||
        (contentBase64 != null && contentBase64 is! String)) {
      throw const FormatException(
        'Supply exactly one string: content or contentBase64',
      );
    }
    // Decode before displaying a selector; binary files must never contain base64 text.
    final bytes = content != null
        ? Uint8List.fromList(utf8.encode(content as String))
        : base64Decode(contentBase64 as String);
    final mimeType = options['mimeType'] as String;
    final extensions = (options['extensions'] as List<dynamic>).cast<String>();
    final path = await FileSaveService.saveBytes(
      bytes: bytes,
      name: name,
      mimeType: mimeType,
      acceptedTypeGroups: <XTypeGroup>[
        XTypeGroup(
          label: mimeType,
          extensions: extensions,
          mimeTypes: <String>[mimeType],
          webWildCards: <String>[mimeType],
        ),
      ],
    );
    return jsonEncode(<String, Object?>{
      'cancelled': path == null,
      'path': path,
    });
  }
}
