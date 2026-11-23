import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies exact custom accents, independent secondary tones and neutral palettes.
void main() {
  for (final brightness in Brightness.values) {
    for (final (primary, secondary, primaryText, secondaryText)
        in <(int, int, Color, Color)>[
          (0xFFFF00FF, 0xFF00FFFF, Colors.white, Colors.black),
          (0xFF123456, 0xFFFFB020, Colors.white, Colors.white),
          (0xFFFFFF00, 0xFF000080, Colors.black, Colors.white),
          (0xFF000000, 0xFFFFFFFF, Colors.white, Colors.black),
          (0xFFFFFFFF, 0xFF000000, Colors.black, Colors.white),
          (0xFF808080, 0xFF333333, Colors.white, Colors.white),
          (0xFF5B8C41, 0xFFB88D35, Colors.white, Colors.white),
          (0xFFE8E8E8, 0xFFFFF5CC, Colors.black, Colors.black),
        ]) {
      test('custom pair $primary / $secondary stays exact in $brightness', () {
        final snapshot = ThemePreferenceSnapshot.fromJson(
          UserPreferencesManager.defaultThemePreferenceSnapshot.toJson()
            ..['useCustomColors'] = true
            ..['customPrimaryColor'] = primary
            ..['customSecondaryColor'] = secondary,
        );
        final scheme = operitColorScheme(brightness, snapshot);
        expect(scheme.primary.toARGB32(), primary);
        expect(scheme.primaryContainer.toARGB32(), primary);
        expect(scheme.secondary.toARGB32(), secondary);
        expect(scheme.secondaryContainer.toARGB32(), secondary);
        expect(scheme.onPrimary, primaryText);
        expect(scheme.onPrimaryContainer, primaryText);
        expect(scheme.onSecondary, secondaryText);
        expect(scheme.onSecondaryContainer, secondaryText);
      });
    }

    test('secondary tones are independent of the primary in $brightness', () {
      final schemes = <ColorScheme>[];
      for (final primary in [0xFFFF00FF, 0xFF000000, 0xFF123456]) {
        schemes.add(
          operitColorScheme(
            brightness,
            ThemePreferenceSnapshot.fromJson(
              UserPreferencesManager.defaultThemePreferenceSnapshot.toJson()
                ..['useCustomColors'] = true
                ..['customPrimaryColor'] = primary
                ..['customSecondaryColor'] = 0xFF00FFFF,
            ),
          ),
        );
      }
      for (final scheme in schemes.skip(1)) {
        expect(scheme.secondary, schemes.first.secondary);
        expect(scheme.onSecondary, schemes.first.onSecondary);
        expect(scheme.secondaryContainer, schemes.first.secondaryContainer);
        expect(scheme.onSecondaryContainer, schemes.first.onSecondaryContainer);
        expect(scheme.secondaryFixed, schemes.first.secondaryFixed);
        expect(scheme.secondaryFixedDim, schemes.first.secondaryFixedDim);
        expect(scheme.onSecondaryFixed, schemes.first.onSecondaryFixed);
        expect(
          scheme.onSecondaryFixedVariant,
          schemes.first.onSecondaryFixedVariant,
        );
      }
    });

    test('disabled saved accents do not change the brand in $brightness', () {
      final values =
          UserPreferencesManager.defaultThemePreferenceSnapshot.toJson()
            ..['useCustomColors'] = false
            ..['customPrimaryColor'] = 0xFFFF00FF
            ..['customSecondaryColor'] = 0xFF00FFFF;
      expect(
        operitColorScheme(brightness, ThemePreferenceSnapshot.fromJson(values)),
        operitColorScheme(
          brightness,
          UserPreferencesManager.defaultThemePreferenceSnapshot,
        ),
      );
    });

    for (final seed in <int>[0xFF000000, 0xFFFFFFFF, 0xFF808080]) {
      test('neutral seed $seed stays grayscale in $brightness', () {
        final snapshot = ThemePreferenceSnapshot.fromJson(
          UserPreferencesManager.defaultThemePreferenceSnapshot.toJson()
            ..['useCustomColors'] = true
            ..['customPrimaryColor'] = seed,
        );
        final restored = ThemePreferenceSnapshot.fromJson(snapshot.toJson());
        expect(restored.customPrimaryColor, seed);
        final scheme = operitColorScheme(brightness, restored);
        expect(scheme.brightness, brightness);
        for (final color in <Color>[
          scheme.primary,
          scheme.onPrimary,
          scheme.primaryContainer,
          scheme.onPrimaryContainer,
          scheme.primaryFixed,
          scheme.primaryFixedDim,
          scheme.onPrimaryFixed,
          scheme.onPrimaryFixedVariant,
          scheme.secondary,
          scheme.onSecondary,
          scheme.secondaryContainer,
          scheme.onSecondaryContainer,
          scheme.secondaryFixed,
          scheme.secondaryFixedDim,
          scheme.onSecondaryFixed,
          scheme.onSecondaryFixedVariant,
          scheme.tertiary,
          scheme.onTertiary,
          scheme.tertiaryContainer,
          scheme.onTertiaryContainer,
          scheme.tertiaryFixed,
          scheme.tertiaryFixedDim,
          scheme.onTertiaryFixed,
          scheme.onTertiaryFixedVariant,
          scheme.surface,
          scheme.onSurface,
          scheme.onSurfaceVariant,
          scheme.surfaceDim,
          scheme.surfaceBright,
          scheme.surfaceContainerLowest,
          scheme.surfaceContainerLow,
          scheme.surfaceContainer,
          scheme.surfaceContainerHigh,
          scheme.surfaceContainerHighest,
          scheme.outline,
          scheme.outlineVariant,
          scheme.inverseSurface,
          scheme.onInverseSurface,
          scheme.inversePrimary,
          scheme.surfaceTint,
        ]) {
          expect(
            color.r,
            color.g,
            reason: '$color must have no red/green tint',
          );
          expect(
            color.g,
            color.b,
            reason: '$color must have no green/blue tint',
          );
        }
        expect(_contrast(scheme.primary, scheme.onPrimary), greaterThan(3));
        expect(_contrast(scheme.surface, scheme.onSurface), greaterThan(4.5));
      });
    }

    test(
      'black and white seeds keep readable secondary pairs in $brightness',
      () {
        final snapshot = ThemePreferenceSnapshot.fromJson(
          UserPreferencesManager.defaultThemePreferenceSnapshot.toJson()
            ..['useCustomColors'] = true
            ..['customPrimaryColor'] = 0xFF000000
            ..['customSecondaryColor'] = 0xFFFFFFFF,
        );
        final scheme = operitColorScheme(brightness, snapshot);
        expect(scheme.secondary.r, scheme.secondary.g);
        expect(scheme.secondary.g, scheme.secondary.b);
        expect(
          _contrast(scheme.secondary, scheme.onSecondary),
          greaterThan(4.5),
        );
        expect(
          _contrast(scheme.secondaryContainer, scheme.onSecondaryContainer),
          greaterThan(4.5),
        );
      },
    );

    test(
      'neutral seed retains an explicitly chosen secondary in $brightness',
      () {
        final snapshot = ThemePreferenceSnapshot.fromJson(
          UserPreferencesManager.defaultThemePreferenceSnapshot.toJson()
            ..['useCustomColors'] = true
            ..['customPrimaryColor'] = 0xFF000000
            ..['customSecondaryColor'] = 0xFF15985D,
        );
        expect(
          operitColorScheme(brightness, snapshot).secondary,
          const Color(0xFF15985D),
        );
      },
    );
  }
}

/// Computes the contrast ratio of a generated foreground and background pair.
double _contrast(Color first, Color second) {
  final firstLuminance = first.computeLuminance();
  final secondLuminance = second.computeLuminance();
  final lighter = firstLuminance > secondLuminance
      ? firstLuminance
      : secondLuminance;
  final darker = firstLuminance < secondLuminance
      ? firstLuminance
      : secondLuminance;
  return (lighter + 0.05) / (darker + 0.05);
}
