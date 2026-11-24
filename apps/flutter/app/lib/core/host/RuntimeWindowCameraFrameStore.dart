// ignore_for_file: file_names
import 'dart:typed_data';
import 'ComposeDslFilePickerStorage.dart';
import 'RuntimeWindowCamera.dart';

/// Publishes captured media through the existing general file selection store.
class RuntimeWindowCameraFrameStore {
  const RuntimeWindowCameraFrameStore({
    this.storage = const RuntimeComposeDslFilePickerStorage(),
  });
  final ComposeDslFilePickerStorage storage;

  Future<Map<String, Object?>?> save(
    Uint8List bytes, {
    required bool Function() isVisible,
  }) async {
    if (bytes.isEmpty || bytes.length > windowCameraFrameLimit) {
      throw StateError('Invalid camera frame');
    }
    if (!isVisible()) return null;
    const name = 'camera-frame.jpg';
    final path = await storage.createFile(name);
    var retained = false;
    try {
      Stream<Uint8List> chunks() async* {
        const chunkSize = 64 * 1024;
        for (var start = 0; start < bytes.length; start += chunkSize) {
          final end = (start + chunkSize).clamp(0, bytes.length);
          yield Uint8List.sublistView(bytes, start, end);
        }
      }

      await storage.writeFile(path, bytes.length, chunks());
      if (!isVisible()) return null;
      retained = true;
      return {
        'path': path,
        'name': name,
        'mimeType': 'image/jpeg',
        'size': bytes.length,
      };
    } finally {
      if (!retained) await storage.deleteFile(path);
    }
  }
}
