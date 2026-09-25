import 'package:file_selector/file_selector.dart' as selector;
import 'package:flutter/services.dart';

/// Unified folder selection. Platform registration, not business code, chooses
/// the implementation. Returned paths remain host paths, not OS capabilities.
abstract final class OperitFolderAccess {
  /// Returns an empty list when the user cancels. Platform limitations and
  /// permission errors are propagated to the caller.
  static Future<List<String>> pickDirectories({String? initialDirectory}) =>
      OperitFolderAccessPlatform.instance.pickDirectories(
        initialDirectory: initialDirectory,
      );

  static Future<String?> pickDirectory({String? initialDirectory}) =>
      OperitFolderAccessPlatform.instance.pickDirectory(
        initialDirectory: initialDirectory,
      );
}

/// Defaults to file_selector and its registered native/web platform delegates.
abstract class OperitFolderAccessPlatform {
  static OperitFolderAccessPlatform instance = _FileSelectorFolderAccess();

  Future<String?> pickDirectory({String? initialDirectory});

  Future<List<String>> pickDirectories({String? initialDirectory});
}

class _FileSelectorFolderAccess extends OperitFolderAccessPlatform {
  @override
  Future<List<String>> pickDirectories({String? initialDirectory}) async =>
      (await selector.getDirectoryPaths(
        initialDirectory: initialDirectory,
      )).whereType<String>().toList(growable: false);

  @override
  Future<String?> pickDirectory({String? initialDirectory}) =>
      selector.getDirectoryPath(initialDirectory: initialDirectory);
}

/// Installed by Flutter's generated macOS plugin registrant. No runtime
/// platform checks or distribution policy live in Dart.
class OperitFolderAccessMacOS extends OperitFolderAccessPlatform {
  static const MethodChannel _channel = MethodChannel('operit/folder_access');

  @override
  Future<List<String>> pickDirectories({String? initialDirectory}) async =>
      await _channel.invokeListMethod<String>(
        'pickDirectories',
        <String, Object?>{'initialDirectory': initialDirectory},
      ) ??
      const <String>[];

  static void registerWith() {
    OperitFolderAccessPlatform.instance = OperitFolderAccessMacOS();
  }

  @override
  Future<String?> pickDirectory({String? initialDirectory}) =>
      _channel.invokeMethod<String>('pickDirectory', <String, Object?>{
        'initialDirectory': initialDirectory,
      });
}
