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

  /// Decodes a stored choice, accepting regional tags such as `zh-CN`, and
  /// uses the system default for unknown values. Flutter and Core clients
  /// share this device-local preference and may write regional language tags.
  static AppLanguage fromPreference(Object? value) {
    if (value is! String || value.isEmpty || value == 'system') {
      return system;
    }
    final tag = value.toLowerCase().replaceAll('_', '-');
    for (final language in AppLanguage.values) {
      final code = language.code;
      if (code != null && (tag == code || tag.startsWith('$code-'))) {
        return language;
      }
    }
    return system;
  }
}
