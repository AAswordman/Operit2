// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../l10n/generated/app_localizations.dart';
import '../features/chat/components/workspace/browser/automation/WorkspaceBrowserAutomationHost.dart';
import '../features/chat/components/workspace/browser/automation/WorkspaceWebVisitHost.dart';
import '../permissions/ToolApprovalHost.dart';

class OperitTheme extends StatefulWidget {
  const OperitTheme({super.key, required this.child});

  final Widget child;

  static OperitThemeController of(BuildContext context) {
    final scope = context
        .dependOnInheritedWidgetOfExactType<_OperitThemeScope>();
    if (scope == null) {
      throw StateError('OperitTheme scope not found');
    }
    return scope.controller;
  }

  @override
  State<OperitTheme> createState() => _OperitThemeState();
}

class _OperitThemeState extends State<OperitTheme> {
  late final OperitThemeController _controller = OperitThemeController(
    onChanged: () => setState(() {}),
  );

  @override
  Widget build(BuildContext context) {
    return _OperitThemeScope(
      controller: _controller,
      child: _OperitMaterialApp(
        themeMode: _controller.themeMode,
        child: widget.child,
      ),
    );
  }
}

class _OperitMaterialApp extends StatelessWidget {
  const _OperitMaterialApp({required this.themeMode, required this.child});

  final ThemeMode themeMode;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final lightColorScheme = _seedColorScheme(Brightness.light);
    final darkColorScheme = _seedColorScheme(Brightness.dark);
    return MaterialApp(
      title: 'Operit2',
      debugShowCheckedModeBanner: false,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      theme: _themeData(lightColorScheme),
      darkTheme: _themeData(darkColorScheme),
      themeMode: themeMode,
      home: WorkspaceBrowserAutomationHost(
        child: WorkspaceWebVisitHost(
          child: ToolApprovalHost(child: child),
        ),
      ),
    );
  }
}

class OperitThemeController {
  OperitThemeController({required VoidCallback onChanged})
    : _onChanged = onChanged;

  final VoidCallback _onChanged;
  ThemeMode _themeMode = ThemeMode.system;

  ThemeMode get themeMode => _themeMode;

  bool isDark(BuildContext context) {
    return Theme.of(context).brightness == Brightness.dark;
  }

  void toggle(BuildContext context) {
    _themeMode = isDark(context) ? ThemeMode.light : ThemeMode.dark;
    _onChanged();
  }
}

class _OperitThemeScope extends InheritedWidget {
  const _OperitThemeScope({required this.controller, required super.child});

  final OperitThemeController controller;

  @override
  bool updateShouldNotify(_OperitThemeScope oldWidget) {
    return controller.themeMode != oldWidget.controller.themeMode;
  }
}

ThemeData _themeData(ColorScheme colorScheme) {
  return ThemeData(
    colorScheme: colorScheme,
    scaffoldBackgroundColor: colorScheme.surface,
    canvasColor: colorScheme.surface,
    // ignore: deprecated_member_use
    progressIndicatorTheme: const ProgressIndicatorThemeData(year2023: false),
    appBarTheme: AppBarTheme(
      backgroundColor: colorScheme.surface,
      foregroundColor: colorScheme.onSurface,
      surfaceTintColor: Colors.transparent,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
      toolbarHeight: 64,
      titleTextStyle: TextStyle(
        color: colorScheme.onSurface,
        fontFamily: _fontFamily,
        fontFamilyFallback: _fontFamilyFallback,
        fontSize: 14,
        fontWeight: FontWeight.w600,
      ),
    ),
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

const Color _brandSeedColor = Color(0xFFBBDEFB);

ColorScheme _seedColorScheme(Brightness brightness) {
  return ColorScheme.fromSeed(
    seedColor: _brandSeedColor,
    brightness: brightness,
    dynamicSchemeVariant: DynamicSchemeVariant.tonalSpot,
  );
}
