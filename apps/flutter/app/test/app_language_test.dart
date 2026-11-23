import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/settings/global_behavior/AppLanguageControl.dart';
import 'package:operit2/ui/theme/OperitTheme.dart' show operitColorScheme;
import 'package:operit_ui/operit_ui.dart' show ThemeRenderRoot;

/// Verifies language selection, root localization, and system language changes.
void main() {
  tearDown(() => UserPreferencesManager.appLanguage.value = AppLanguage.system);

  test(
    'language choices match the available translations and reject invalid data',
    () {
      expect(
        AppLanguage.values
            .map((language) => language.locale)
            .whereType<Locale>(),
        unorderedEquals(AppLocalizations.supportedLocales),
      );
      expect(AppLanguage.fromPreference(null), AppLanguage.system);
      expect(() => AppLanguage.fromPreference('de'), throwsFormatException);
      expect(() => AppLanguage.fromPreference(1), throwsFormatException);
      expect(
        AppLanguage.system.resolveLocale(const <Locale>[Locale('fr', 'CA')]),
        const Locale('fr'),
      );
      expect(
        AppLanguage.english.resolveLocale(const <Locale>[Locale('zh')]),
        const Locale('en'),
      );
    },
  );

  testWidgets('selection switches the real app root and retains entered text', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(390, 844);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    tester.platformDispatcher.localesTestValue = const <Locale>[
      Locale('zh', 'CN'),
    ];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
    final preferences = _LanguagePreferences();
    await tester.pumpWidget(_application(preferences));
    await tester.pumpAndSettle();
    expect(find.text('应用语言'), findsOneWidget);
    await tester.enterText(find.byType(TextField), 'keep my draft');
    await tester.tap(find.byKey(const ValueKey('settings.appLanguage')));
    await tester.pumpAndSettle();
    for (final label in ['跟随系统', '中文', 'English', 'Français']) {
      expect(find.text(label), findsWidgets);
    }
    await tester.tap(
      find.byKey(const ValueKey('settings.appLanguage.english')),
    );
    await tester.pumpAndSettle();
    expect(find.text('App language'), findsOneWidget);
    expect(find.text('keep my draft'), findsOneWidget);
    expect(preferences.saved, <AppLanguage>[AppLanguage.english]);

    tester.platformDispatcher.localesTestValue = const <Locale>[Locale('fr')];
    await tester.pumpAndSettle();
    expect(find.text('App language'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('settings.appLanguage')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('settings.appLanguage.system')));
    await tester.pumpAndSettle();
    expect(find.text('Langue de l’application'), findsOneWidget);
    expect(find.text('keep my draft'), findsOneWidget);
    expect(UserPreferencesManager.resolvedAppLocale, const Locale('fr'));

    tester.platformDispatcher.localesTestValue = const <Locale>[Locale('zh')];
    await tester.pumpAndSettle();
    expect(find.text('应用语言'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('a failed save shows an error and keeps the committed language', (
    tester,
  ) async {
    final preferences = _LanguagePreferences()..failSave = true;
    UserPreferencesManager.appLanguage.value = AppLanguage.english;
    await tester.pumpWidget(_application(preferences));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('settings.appLanguage')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('settings.appLanguage.french')));
    await tester.pumpAndSettle();
    expect(UserPreferencesManager.appLanguage.value, AppLanguage.english);
    expect(find.textContaining('Could not save app language:'), findsOneWidget);
    expect(find.text('App language'), findsOneWidget);
  });
}

/// Exercises the shared rendering root used by both main and detached apps.
Widget _application(UserPreferencesManager preferences) {
  final snapshot = UserPreferencesManager.defaultThemePreferenceSnapshot;
  return ValueListenableBuilder<AppLanguage>(
    valueListenable: UserPreferencesManager.appLanguage,
    builder: (context, language, _) => ThemeRenderRoot(
      themePreferenceSnapshot: snapshot,
      lightColorScheme: operitColorScheme(Brightness.light, snapshot),
      darkColorScheme: operitColorScheme(Brightness.dark, snapshot),
      fontFamily: 'Roboto',
      fontFamilyFallback: const <String>[],
      themeMode: ThemeMode.light,
      locale: language.locale,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      builder: (context, child) => child!,
      home: Scaffold(
        body: Builder(
          builder: (context) => Column(
            children: <Widget>[
              Text(AppLocalizations.of(context)!.settingsAppLanguageTitle),
              AppLanguageControl(preferences: preferences),
              const TextField(),
            ],
          ),
        ),
      ),
    ),
  );
}

class _LanguagePreferences extends UserPreferencesManager {
  final saved = <AppLanguage>[];
  bool failSave = false;

  /// Simulates successful and failed commits for the language picker.
  @override
  Future<void> saveAppLanguage(AppLanguage language) async {
    if (failSave) throw StateError('preference store unavailable');
    saved.add(language);
    UserPreferencesManager.appLanguage.value = language;
  }
}
