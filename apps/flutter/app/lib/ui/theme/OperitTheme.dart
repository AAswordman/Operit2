// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:dynamic_color/dynamic_color.dart';

import 'Color.dart';
import '../../l10n/generated/app_localizations.dart';
import '../features/chat/components/workspace/browser/automation/WorkspaceBrowserAutomationHost.dart';
import '../permissions/ToolApprovalHost.dart';

class OperitTheme extends StatelessWidget {
  const OperitTheme({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return DynamicColorBuilder(
      builder: (lightDynamic, darkDynamic) {
        final lightColorScheme = lightDynamic ?? _lightColorScheme;
        final darkColorScheme = darkDynamic ?? _darkColorScheme;
        return MaterialApp(
          title: 'Operit2',
          debugShowCheckedModeBanner: false,
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          theme: _themeData(lightColorScheme),
          darkTheme: _themeData(darkColorScheme),
          themeMode: ThemeMode.system,
          home: WorkspaceBrowserAutomationHost(
            child: ToolApprovalHost(child: child),
          ),
        );
      },
    );
  }
}

ThemeData _themeData(ColorScheme colorScheme) {
  return ThemeData(
    colorScheme: colorScheme,
    scaffoldBackgroundColor: colorScheme.surface,
    canvasColor: colorScheme.surface,
    fontFamily: _fontFamily,
    fontFamilyFallback: _fontFamilyFallback,
    useMaterial3: true,
  );
}

const String _fontFamily = 'Aptos';

const List<String> _fontFamilyFallback = <String>[
  'Calibri',
  'Segoe UI',
  'Microsoft YaHei UI',
  'Microsoft YaHei',
  'SimHei',
  'Noto Sans CJK SC',
  'Source Han Sans SC',
  'Roboto',
  'Arial',
];

const ColorScheme _darkColorScheme = ColorScheme.dark(
  primary: purple80,
  onPrimary: darkOnPrimary,
  primaryContainer: darkPrimaryContainer,
  onPrimaryContainer: darkOnPrimaryContainer,
  secondary: purpleGrey80,
  onSecondary: darkOnSecondary,
  secondaryContainer: darkSecondaryContainer,
  onSecondaryContainer: darkOnSecondaryContainer,
  tertiary: pink80,
  onTertiary: darkOnTertiary,
  tertiaryContainer: darkTertiaryContainer,
  onTertiaryContainer: darkOnTertiaryContainer,
  error: darkError,
  onError: darkOnError,
  errorContainer: darkErrorContainer,
  onErrorContainer: darkOnErrorContainer,
  surface: darkSurface,
  onSurface: darkOnSurface,
  surfaceDim: darkSurfaceDim,
  surfaceBright: darkSurfaceBright,
  surfaceContainerLowest: darkSurfaceContainerLowest,
  surfaceContainerLow: darkSurfaceContainerLow,
  surfaceContainer: darkSurfaceContainer,
  surfaceContainerHigh: darkSurfaceContainerHigh,
  surfaceContainerHighest: darkSurfaceContainerHighest,
  onSurfaceVariant: darkOnSurfaceVariant,
  outline: darkOutline,
  outlineVariant: darkOutlineVariant,
  inverseSurface: darkInverseSurface,
  onInverseSurface: darkOnInverseSurface,
  inversePrimary: darkInversePrimary,
  surfaceTint: purple80,
);

const ColorScheme _lightColorScheme = ColorScheme.light(
  primary: purple40,
  onPrimary: lightOnPrimary,
  primaryContainer: lightPrimaryContainer,
  onPrimaryContainer: lightOnPrimaryContainer,
  secondary: purpleGrey40,
  onSecondary: lightOnSecondary,
  secondaryContainer: lightSecondaryContainer,
  onSecondaryContainer: lightOnSecondaryContainer,
  tertiary: pink40,
  onTertiary: lightOnTertiary,
  tertiaryContainer: lightTertiaryContainer,
  onTertiaryContainer: lightOnTertiaryContainer,
  error: lightError,
  onError: lightOnError,
  errorContainer: lightErrorContainer,
  onErrorContainer: lightOnErrorContainer,
  surface: lightSurface,
  onSurface: lightOnSurface,
  surfaceDim: lightSurfaceDim,
  surfaceBright: lightSurfaceBright,
  surfaceContainerLowest: lightSurfaceContainerLowest,
  surfaceContainerLow: lightSurfaceContainerLow,
  surfaceContainer: lightSurfaceContainer,
  surfaceContainerHigh: lightSurfaceContainerHigh,
  surfaceContainerHighest: lightSurfaceContainerHighest,
  onSurfaceVariant: lightOnSurfaceVariant,
  outline: lightOutline,
  outlineVariant: lightOutlineVariant,
  inverseSurface: lightInverseSurface,
  onInverseSurface: lightOnInverseSurface,
  inversePrimary: lightInversePrimary,
  surfaceTint: purple40,
);
