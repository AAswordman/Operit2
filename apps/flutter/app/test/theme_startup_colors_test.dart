import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/common/OperitLogoMark.dart';
import 'package:operit2/ui/common/RuntimeBootstrapScreen.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit_ui/operit_ui.dart' show ThemeVisualSpecV1;

/// Verifies that startup keeps application colors before runtime appearance is ready.
void main() {
  for (final brightness in Brightness.values) {
    final mode = brightness == Brightness.light
        ? ThemeMode.light
        : ThemeMode.dark;
    for (final custom in <bool>[false, true]) {
      testWidgets(
        'startup retains ${custom ? 'custom' : 'brand'} colors in $brightness',
        (tester) async {
          final values = UserPreferencesManager.defaultThemePreferenceSnapshot
              .toJson();
          if (custom) {
            values['useCustomColors'] = true;
            values['customPrimaryColor'] = 0xFFAC2855;
            values['customSecondaryColor'] = 0xFF15985D;
          }
          final snapshot = ThemePreferenceSnapshot.fromJson(values);
          final expected = ColorScheme.fromSeed(
            seedColor: Color(custom ? 0xFFAC2855 : 0xFFBBDEFB),
            brightness: brightness,
            dynamicSchemeVariant: DynamicSchemeVariant.tonalSpot,
          );
          await _pumpStartup(tester, snapshot, mode);
          final scheme = Theme.of(
            tester.element(find.byType(RuntimeBootstrapScreen)),
          ).colorScheme;
          expect(scheme.brightness, brightness);
          expect(scheme.primary, expected.primary);
          expect(
            scheme.secondary,
            custom ? const Color(0xFF15985D) : expected.secondary,
          );
          _expectLogoAndProgress(tester, scheme.primary);
        },
      );
    }

    testWidgets('startup retains a complete palette in $brightness', (
      tester,
    ) async {
      final encoded = await tester.runAsync(
        () => rootBundle.loadString(
          'packages/operit_ui/assets/fixtures/theme-v1.json',
        ),
      );
      final fixture = jsonDecode(encoded!) as Map<String, dynamic>;
      final palette = fixture['palette'] as Map<String, dynamic>;
      final snapshot = ThemePreferenceSnapshot.fromJson(
        UserPreferencesManager.defaultThemePreferenceSnapshot.toJson()
          ..['themePaletteJson'] = jsonEncode(palette),
      );
      await _pumpStartup(tester, snapshot, mode);
      final scheme = Theme.of(
        tester.element(find.byType(RuntimeBootstrapScreen)),
      ).colorScheme;
      expect(
        scheme,
        ThemeVisualSpecV1.colorSchemeFromPalette(palette, brightness),
      );
      _expectLogoAndProgress(tester, scheme.primary);
    });
  }

  testWidgets('startup honors system dark mode', (tester) async {
    tester.platformDispatcher.platformBrightnessTestValue = Brightness.dark;
    addTearDown(tester.platformDispatcher.clearPlatformBrightnessTestValue);
    await _pumpStartup(
      tester,
      UserPreferencesManager.defaultThemePreferenceSnapshot,
      ThemeMode.system,
    );
    final scheme = Theme.of(
      tester.element(find.byType(RuntimeBootstrapScreen)),
    ).colorScheme;
    expect(scheme.brightness, Brightness.dark);
    _expectLogoAndProgress(tester, scheme.primary);
  });

  testWidgets('startup does not require pending custom font assets', (
    tester,
  ) async {
    final snapshot = ThemePreferenceSnapshot.fromJson(
      UserPreferencesManager.defaultThemePreferenceSnapshot.toJson()
        ..['fontType'] = UserPreferencesManager.FONT_TYPE_FILE
        ..['useCustomFont'] = true
        ..['customFontPath'] = 'user_assets/theme/pending.ttf',
    );
    await _pumpStartup(tester, snapshot, ThemeMode.light);
    expect(find.byType(RuntimeBootstrapScreen), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

/// Mounts the actual startup branch without mounting runtime-dependent application content.
Future<void> _pumpStartup(
  WidgetTester tester,
  ThemePreferenceSnapshot snapshot,
  ThemeMode mode,
) async {
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot: snapshot,
      initialThemeMode: mode,
      initialThemeIsReady: false,
      hostInteractionHostsEnabled: false,
      child: const Text('Runtime application'),
    ),
  );
  await tester.pump(const Duration(milliseconds: 800));
  expect(find.text('Runtime application'), findsNothing);
  expect(tester.takeException(), isNull);
}

/// Checks visible brand elements against the active application primary color.
void _expectLogoAndProgress(WidgetTester tester, Color primary) {
  expect(
    tester.widget<OperitLogoMark>(find.byType(OperitLogoMark)).color,
    primary,
  );
  expect(
    tester
        .widget<RuntimeBootstrapLiquidProgress>(
          find.byType(RuntimeBootstrapLiquidProgress),
        )
        .color,
    primary,
  );
}
