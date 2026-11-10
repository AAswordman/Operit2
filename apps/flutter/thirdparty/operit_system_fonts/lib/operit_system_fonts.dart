import 'package:flutter/services.dart';

import 'src/font_face.dart';

/// Client-owned font access; remote runtimes never enumerate the user's device.
abstract class SystemFontsPlatform {
  static SystemFontsPlatform instance = MethodChannelSystemFonts();

  /// Enumerates accessible installed font families using the device font catalog.
  Future<List<String>> listFamilies();

  /// Reads a regular face from a family returned by this adapter's catalog.
  Future<Uint8List> readFamily(String family);
}

/// Uses the registered native platform plugin without platform checks in app code.
class MethodChannelSystemFonts implements SystemFontsPlatform {
  static const _channel = MethodChannel('operit/system_fonts');

  /// Requests the current installed catalog and retains unique sorted family names.
  @override
  Future<List<String>> listFamilies() async {
    final names = await _channel.invokeListMethod<String>('listFamilies');
    if (names == null)
      throw StateError('Device font catalog returned no result');
    return names.toSet().toList()..sort();
  }

  /// Extracts the selected collection face before Flutter registers the font.
  @override
  Future<Uint8List> readFamily(String family) async {
    final result = await _channel.invokeMapMethod<String, Object?>(
      'readFamily',
      family,
    );
    if (result == null) throw StateError('Device font returned no data');
    return extractFontFace(
      result['bytes'] as Uint8List,
      index: result['index'] as int?,
      postscriptName: result['postscriptName'] as String?,
    );
  }
}
