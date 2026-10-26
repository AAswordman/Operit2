import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/theme/PluginThemeSnapshot.dart';

/// Verifies exact UI colors and alpha across the public theme boundary.
void main() {
  test('theme snapshots preserve custom colors and CSS alpha order', () {
    final scheme = ColorScheme.fromSeed(
      seedColor: Colors.green,
      brightness: Brightness.dark,
    ).copyWith(primary: const Color(0x80345678));
    final snapshot = pluginThemeSnapshot(scheme);
    expect(snapshot['brightness'], 'dark');
    final colors = snapshot['colors'] as Map<String, String>;
    expect(colors['primary'], '#34567880');
    expect(colors['background'], '#00000000');
    expect(colors['surface'], isNot('#00000000'));
    expect(
      colors.keys,
      containsAll(['surfaceContainer', 'tertiary', 'onError']),
    );
    expect(
      colors.values.every((value) => RegExp(r'^#[0-9a-f]{8}$').hasMatch(value)),
      isTrue,
    );
  });
}
