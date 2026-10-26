import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

/// Verifies the compact creation flow exposes localized template and inheritance sources.
void main() {
  for (final language in <String>['zh', 'en']) {
    test('creation sources are readable in $language', () {
      final l10n = lookupAppLocalizations(Locale(language));
      final labels = <String>[
        l10n.settingsAppearanceThemeConfigSource,
        l10n.settingsAppearanceThemeConfigFromDefaults,
        l10n.settingsAppearanceThemeConfigFromCurrent,
        l10n.settingsAppearanceThemeConfigInherit('Example'),
        l10n.settingsAppearanceThemeConfigFromTemplate('Example'),
        l10n.settingsAppearanceThemeConfigSourceHint,
        l10n.settingsAppearanceThemeConfigDefaultsHint,
        l10n.settingsAppearanceThemeConfigCurrentHint,
        l10n.settingsAppearanceThemeConfigSavedSources,
        l10n.settingsAppearanceThemeConfigPluginSources,
      ];
      for (final label in labels) {
        expect(label, isNotEmpty);
        expect(label, isNot(contains('???')));
        expect(label, isNot(contains('\uFFFD')));
      }
    });
  }
}
