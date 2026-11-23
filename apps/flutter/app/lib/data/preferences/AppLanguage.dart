// ignore_for_file: file_names

import 'package:flutter/widgets.dart';

import '../../l10n/generated/app_localizations.dart';

/// Defines persisted application language choices independently of saved themes.
enum AppLanguage {
  system(null, null),
  chinese('zh', '中文'),
  english('en', 'English'),
  french('fr', 'Français');

  /// Creates a language choice with a stable storage code and native name.
  const AppLanguage(this.code, this.nativeName);

  final String? code;
  final String? nativeName;

  /// Leaves locale selection to Flutter when following the system.
  Locale? get locale => code == null ? null : Locale(code!);

  /// Resolves system preferences through Flutter's supported locale matching.
  Locale resolveLocale(List<Locale> systemLocales) => basicLocaleListResolution(
    locale == null ? systemLocales : <Locale>[locale!],
    AppLocalizations.supportedLocales,
  );

  /// Decodes the initial system choice and rejects unsupported stored values.
  static AppLanguage fromPreference(Object? value) => switch (value) {
    null || 'system' => system,
    'zh' => chinese,
    'en' => english,
    'fr' => french,
    _ => throw FormatException('Invalid application language: $value'),
  };
}
