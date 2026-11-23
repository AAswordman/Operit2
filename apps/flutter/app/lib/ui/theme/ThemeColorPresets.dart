// ignore_for_file: file_names

import 'package:operit_ui/operit_ui.dart';

class ThemeColorPreset {
  /// Defines the persisted identity and seed colors of a built-in preset.
  const ThemeColorPreset({
    required this.id,
    required this.primaryColor,
    required this.secondaryColor,
    required this.useCustomColors,
  });

  final String id;
  final int? primaryColor;
  final int? secondaryColor;
  final bool useCustomColors;
}

const List<ThemeColorPreset> themeColorPresets = <ThemeColorPreset>[
  ThemeColorPreset(
    id: 'default',
    primaryColor: null,
    secondaryColor: null,
    useCustomColors: false,
  ),
  ThemeColorPreset(
    id: 'sky',
    primaryColor: 0xFF4C9EEB,
    secondaryColor: 0xFF32B8C6,
    useCustomColors: true,
  ),
  ThemeColorPreset(
    id: 'matcha',
    primaryColor: 0xFF5C8D48,
    secondaryColor: 0xFFB08B42,
    useCustomColors: true,
  ),
  ThemeColorPreset(
    id: 'ember',
    primaryColor: 0xFFE46F3D,
    secondaryColor: 0xFF9C6A2F,
    useCustomColors: true,
  ),
  ThemeColorPreset(
    id: 'rose',
    primaryColor: 0xFFD85C7F,
    secondaryColor: 0xFF8E6AD8,
    useCustomColors: true,
  ),
];

/// Matches the same persisted color pair used by the appearance selector.
ThemeColorPreset? themeColorPresetForSnapshot(
  ThemePreferenceSnapshot snapshot,
) {
  if (!snapshot.useCustomColors) return themeColorPresets.first;
  for (final preset in themeColorPresets) {
    if (preset.useCustomColors &&
        preset.primaryColor == snapshot.customPrimaryColor &&
        preset.secondaryColor == snapshot.customSecondaryColor) {
      return preset;
    }
  }
  return null;
}
