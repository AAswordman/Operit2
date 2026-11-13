// ignore_for_file: file_names

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';
import 'package:share_plus/share_plus.dart';

/// Shares generated files through the local system share sheet.
class FileShareService {
  FileShareService._();

  static Future<void> shareBytes({
    required Uint8List bytes,
    required String name,
    required String mimeType,
    required Rect sharePositionOrigin,
  }) async {
    final file = XFile.fromData(bytes, name: name, mimeType: mimeType);
    if (!kIsWeb && defaultTargetPlatform.name == 'ohos') {
      // OpenHarmony already exposes its share resolver through the owner channel.
      final temporaryDirectory = await getTemporaryDirectory();
      final path = '${temporaryDirectory.path}/$name';
      await file.saveTo(path);
      final response = await const MethodChannel('operit/runtime')
          .invokeMapMethod<String, Object?>('ownerFileShare', <String, Object?>{
            'path': path,
            'title': name,
          });
      if (response?['success'] != true) {
        throw StateError(response?['error']?.toString() ?? '无法打开系统分享');
      }
      return;
    }
    await SharePlus.instance.share(
      ShareParams(
        files: <XFile>[file],
        fileNameOverrides: <String>[name],
        sharePositionOrigin: sharePositionOrigin,
      ),
    );
  }
}
