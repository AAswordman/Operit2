// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:operit_system_fonts/operit_system_fonts.dart';
import 'package:operit_ui/operit_ui.dart';

/// A device font whose exact face has completed Flutter registration.
class PreparedDeviceFont {
  /// Retains the display name, immutable resource bytes, and registered alias.
  const PreparedDeviceFont({
    required this.name,
    required this.bytes,
    required this.family,
  });
  final String name;
  final Uint8List bytes;
  final String family;
}

/// Owns device catalog access independently of synchronized runtime preferences.
class DeviceFontService {
  /// Allows host and registration substitution for isolated tests.
  DeviceFontService({
    SystemFontsPlatform? platform,
    ThemeFontRegistry? registry,
  }) : _platform = platform ?? SystemFontsPlatform.instance,
       _registry = registry ?? ThemeFontRegistry();

  final SystemFontsPlatform _platform;
  final ThemeFontRegistry _registry;

  /// Requests the real client catalog directly from the selection gesture.
  Future<List<String>> listFamilies() => _platform.listFamilies();

  /// Registers only the chosen face so preview and saved appearance use identical bytes.
  Future<PreparedDeviceFont> prepare(String name) async {
    final bytes = await _platform.readFamily(name);
    final prepared = await _registry.registerBytes(
      reference: name,
      bytes: bytes,
    );
    return PreparedDeviceFont(
      name: name,
      bytes: bytes,
      family: prepared.family,
    );
  }
}
