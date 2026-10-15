import 'package:flutter/material.dart';

import '../model/theme_preference_snapshot.dart';
import 'theme_data.dart';

/// Same Material rendering entry for the application and the standalone preview.
/// The caller owns lifecycle, localization, environment, and the prepared fonts.
class ThemeRenderRoot extends StatelessWidget {
  const ThemeRenderRoot({
    super.key,
    required this.themePreferenceSnapshot,
    required this.lightColorScheme,
    required this.darkColorScheme,
    required this.fontFamily,
    required this.fontFamilyFallback,
    required this.themeMode,
    required this.builder,
    required this.home,
    this.title = 'Operit2',
    this.themeAnimationDuration = kThemeAnimationDuration,
    this.localizationsDelegates,
    this.supportedLocales = const <Locale>[Locale('en', 'US')],
  });

  final ThemePreferenceSnapshot themePreferenceSnapshot;
  final ColorScheme lightColorScheme;
  final ColorScheme darkColorScheme;
  final String fontFamily;
  final List<String> fontFamilyFallback;
  final ThemeMode themeMode;
  final TransitionBuilder builder;
  final Widget home;
  final String title;
  final Duration themeAnimationDuration;
  final Iterable<LocalizationsDelegate<dynamic>>? localizationsDelegates;
  final Iterable<Locale> supportedLocales;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: title,
      debugShowCheckedModeBanner: false,
      theme: buildOperitThemeData(
        lightColorScheme,
        themePreferenceSnapshot,
        fontFamily: fontFamily,
        fontFamilyFallback: fontFamilyFallback,
      ),
      darkTheme: buildOperitThemeData(
        darkColorScheme,
        themePreferenceSnapshot,
        fontFamily: fontFamily,
        fontFamilyFallback: fontFamilyFallback,
      ),
      themeMode: themeMode,
      themeAnimationDuration: themeAnimationDuration,
      localizationsDelegates: localizationsDelegates,
      supportedLocales: supportedLocales,
      builder: builder,
      home: home,
    );
  }
}
