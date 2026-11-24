import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart' show ThemePreferenceSnapshot;

import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/common/markdown/MarkdownCodeTypeface.dart';

void main() {
  test('the bundled face is the default code font family', () {
    // A generic 'monospace' primary would resolve through fontconfig, which
    // on Linux machines with Windows fonts installed picks serif CJK faces
    // (SimSun/NSimSun) and renders code-block Han as Song type.
    expect(markdownCodeFontFamily, 'OperitCodeMono');
    expect(
      markdownCodeFontFamilyFallback.contains(markdownCodeFontFamily),
      isFalse,
      reason: 'the bundled face must stay out of its own fallback list',
    );
  });

  test('the default snapshot resolves to the bundled code face', () {
    final snapshot = UserPreferencesManager.defaultThemePreferenceSnapshot;
    expect(snapshot.codeFontType, 'system');
    expect(operitCodeFontFamily(snapshot), markdownCodeFontFamily);
    expect(
      operitCodeFontFamilyFallback(snapshot),
      markdownCodeFontFamilyFallback,
    );
    expect(operitCodeFontFamily(null), markdownCodeFontFamily);
  });

  test('unprepared custom code font assets fall back to the bundled face', () {
    final json = UserPreferencesManager.defaultThemePreferenceSnapshot.toJson()
      ..['codeFontType'] = 'file'
      ..['codeFontPath'] = 'unprepared-font.ttf';
    final snapshot = ThemePreferenceSnapshot.fromJson(json);
    expect(operitCodeFontFamily(snapshot), markdownCodeFontFamily);
    expect(
      operitCodeFontFamilyFallback(snapshot),
      markdownCodeFontFamilyFallback,
    );
  });

  test('snapshot round trip preserves the code font slot', () {
    final json = UserPreferencesManager.defaultThemePreferenceSnapshot.toJson()
      ..['codeFontType'] = 'device'
      ..['codeFontSystemName'] = 'Hack'
      ..['codeFontPath'] = 'device-font.ttf';
    final snapshot = ThemePreferenceSnapshot.fromJson(json);
    expect(snapshot.codeFontType, 'device');
    expect(snapshot.codeFontSystemName, 'Hack');
    expect(snapshot.codeFontPath, 'device-font.ttf');
    // Older payloads without the code font keys keep the system default.
    final legacyJson = UserPreferencesManager.defaultThemePreferenceSnapshot
        .toJson()
      ..remove('codeFontType')
      ..remove('codeFontSystemName')
      ..remove('codeFontPath');
    final legacy = ThemePreferenceSnapshot.fromJson(legacyJson);
    expect(legacy.codeFontType, 'system');
    expect(legacy.codeFontPath, isNull);
  });
}
