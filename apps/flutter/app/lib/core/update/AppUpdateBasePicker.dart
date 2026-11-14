// ignore_for_file: file_names
import 'dart:convert';
import '../host/ComposeDslFilePickerService.dart';
import '../host/ComposeDslFilePickerStorage.dart';

/// Streams an original installer into Core-managed temporary storage.
Future<String?> pickAppUpdateBase() async {
  final result =
      jsonDecode(
            await ComposeDslFilePickerService.open(
              jsonEncode({
                'picker': 'document',
                'allowMultiple': false,
                'mimeTypes': <String>[],
              }),
            ),
          )
          as Map<String, dynamic>;
  if (result['cancelled'] == true) return null;
  return ((result['files'] as List).single as Map<String, dynamic>)['path']
      as String;
}

/// Releases the picker copy after Core has verified and retained the original.
Future<void> deleteAppUpdateBaseSelection(String path) =>
    const RuntimeComposeDslFilePickerStorage().deleteFile(path);
