import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../model/theme_preference_snapshot.dart';

/// Builds Material theme data for the effective user theme state.
ThemeData buildOperitThemeData(
  ColorScheme colorScheme,
  ThemePreferenceSnapshot themePreferenceSnapshot, {
  required String fontFamily,
  required List<String> fontFamilyFallback,
}) {
  final typography = Typography.material2021();
  final backgroundVisible =
      themePreferenceSnapshot.useBackgroundImage &&
      themePreferenceSnapshot.backgroundImageUri != null &&
      themePreferenceSnapshot.backgroundImageUri!.isNotEmpty;
  final appBarBackgroundColor =
      backgroundVisible || themePreferenceSnapshot.transparentSurfaceEnabled
      ? Colors.transparent
      : colorScheme.surface;
  final baseTextTheme = typography.englishLike.merge(
    colorScheme.brightness == Brightness.dark
        ? typography.white
        : typography.black,
  );
  final textTheme = baseTextTheme.apply(
    fontFamily: fontFamily,
    fontFamilyFallback: fontFamilyFallback,
    fontSizeFactor: themePreferenceSnapshot.fontScale,
  );
  return ThemeData(
    colorScheme: colorScheme,
    scaffoldBackgroundColor: Colors.transparent,
    canvasColor: colorScheme.surface,
    textTheme: textTheme,
    inputDecorationTheme: _inputDecorationTheme(colorScheme, textTheme),
    dropdownMenuTheme: _dropdownMenuTheme(colorScheme, textTheme),
    menuTheme: _menuTheme(colorScheme, textTheme),
    menuButtonTheme: _menuButtonTheme(colorScheme, textTheme),
    popupMenuTheme: _popupMenuTheme(colorScheme, textTheme),
    buttonTheme: _buttonTheme(colorScheme),
    dialogTheme: _dialogTheme(colorScheme, textTheme),
    // ignore: deprecated_member_use
    progressIndicatorTheme: const ProgressIndicatorThemeData(year2023: false),
    appBarTheme: AppBarTheme(
      backgroundColor: appBarBackgroundColor,
      foregroundColor: colorScheme.onSurface,
      surfaceTintColor: Colors.transparent,
      systemOverlayStyle: operitSystemUiOverlayStyle(colorScheme),
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
      toolbarHeight: 64,
      titleTextStyle: textTheme.titleSmall?.copyWith(
        color: colorScheme.onSurface,
        fontFamily: fontFamily,
        fontFamilyFallback: fontFamilyFallback,
        fontWeight: FontWeight.w600,
      ),
    ),
    fontFamily: fontFamily,
    fontFamilyFallback: fontFamilyFallback,
    useMaterial3: true,
  );
}

/// Builds the shared theme for Material 3 DropdownMenu widgets.
DropdownMenuThemeData _dropdownMenuTheme(
  ColorScheme colorScheme,
  TextTheme textTheme,
) {
  final menuShape = RoundedRectangleBorder(
    borderRadius: BorderRadius.circular(8),
  );
  return DropdownMenuThemeData(
    textStyle: textTheme.bodyMedium?.copyWith(color: colorScheme.onSurface),
    menuStyle: MenuStyle(
      backgroundColor: WidgetStatePropertyAll(colorScheme.surfaceContainerHigh),
      shadowColor: WidgetStatePropertyAll(
        colorScheme.shadow.withValues(alpha: 0.18),
      ),
      surfaceTintColor: const WidgetStatePropertyAll(Colors.transparent),
      elevation: const WidgetStatePropertyAll(3),
      padding: const WidgetStatePropertyAll(EdgeInsets.symmetric(vertical: 4)),
      shape: WidgetStatePropertyAll(menuShape),
    ),
  );
}

/// Builds the shared menu theme used by MenuAnchor and MenuItemButton.
MenuThemeData _menuTheme(ColorScheme colorScheme, TextTheme textTheme) {
  final menuShape = RoundedRectangleBorder(
    borderRadius: BorderRadius.circular(8),
  );
  return MenuThemeData(
    style: MenuStyle(
      backgroundColor: WidgetStatePropertyAll(colorScheme.surfaceContainerHigh),
      shadowColor: WidgetStatePropertyAll(
        colorScheme.shadow.withValues(alpha: 0.18),
      ),
      surfaceTintColor: const WidgetStatePropertyAll(Colors.transparent),
      elevation: const WidgetStatePropertyAll(3),
      padding: const WidgetStatePropertyAll(EdgeInsets.symmetric(vertical: 4)),
      shape: WidgetStatePropertyAll(menuShape),
    ),
  );
}

/// Builds the shared item style used inside MenuAnchor menus.
MenuButtonThemeData _menuButtonTheme(
  ColorScheme colorScheme,
  TextTheme textTheme,
) {
  return MenuButtonThemeData(
    style: ButtonStyle(
      foregroundColor: WidgetStatePropertyAll(colorScheme.onSurface),
      textStyle: WidgetStatePropertyAll(textTheme.bodyMedium),
      padding: const WidgetStatePropertyAll(
        EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      ),
      minimumSize: const WidgetStatePropertyAll(Size(0, 40)),
      shape: WidgetStatePropertyAll(
        RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
      ),
    ),
  );
}

/// Builds the shared popup menu theme used by PopupMenuButton.
PopupMenuThemeData _popupMenuTheme(
  ColorScheme colorScheme,
  TextTheme textTheme,
) {
  return PopupMenuThemeData(
    color: colorScheme.surfaceContainerHigh,
    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
    menuPadding: const EdgeInsets.symmetric(vertical: 4),
    elevation: 3,
    shadowColor: colorScheme.shadow.withValues(alpha: 0.18),
    surfaceTintColor: Colors.transparent,
    textStyle: textTheme.bodyMedium?.copyWith(color: colorScheme.onSurface),
  );
}

/// Builds the legacy button theme values still used by DropdownButton.
ButtonThemeData _buttonTheme(ColorScheme colorScheme) {
  return ButtonThemeData(alignedDropdown: true, colorScheme: colorScheme);
}

DialogThemeData _dialogTheme(ColorScheme colorScheme, TextTheme textTheme) {
  return DialogThemeData(
    backgroundColor: colorScheme.surfaceContainerHigh,
    surfaceTintColor: Colors.transparent,
    elevation: 6,
    shadowColor: colorScheme.shadow.withValues(alpha: 0.18),
    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(28)),
    clipBehavior: Clip.antiAlias,
    insetPadding: const EdgeInsets.symmetric(horizontal: 24, vertical: 24),
    actionsPadding: const EdgeInsets.fromLTRB(24, 0, 24, 20),
    titleTextStyle: textTheme.headlineSmall?.copyWith(
      color: colorScheme.onSurface,
      fontWeight: FontWeight.w700,
    ),
    contentTextStyle: textTheme.bodyMedium?.copyWith(
      color: colorScheme.onSurfaceVariant,
    ),
    iconColor: colorScheme.primary,
    barrierColor: Colors.black.withValues(alpha: 0.38),
  );
}

InputDecorationTheme _inputDecorationTheme(
  ColorScheme colorScheme,
  TextTheme textTheme,
) {
  final radius = BorderRadius.circular(8);
  final baseBorder = OutlineInputBorder(
    borderRadius: radius,
    borderSide: BorderSide(
      color: colorScheme.outlineVariant.withValues(alpha: 0.72),
      width: 1,
    ),
  );
  final disabledBorder = OutlineInputBorder(
    borderRadius: radius,
    borderSide: BorderSide(
      color: colorScheme.outlineVariant.withValues(alpha: 0.36),
      width: 1,
    ),
  );
  final focusedBorder = OutlineInputBorder(
    borderRadius: radius,
    borderSide: BorderSide(color: colorScheme.primary, width: 1.4),
  );
  final errorBorder = OutlineInputBorder(
    borderRadius: radius,
    borderSide: BorderSide(color: colorScheme.error, width: 1.2),
  );

  return InputDecorationTheme(
    filled: true,
    fillColor: colorScheme.surfaceContainerHighest.withValues(alpha: 0.34),
    isDense: true,
    contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 14),
    border: baseBorder,
    enabledBorder: baseBorder,
    disabledBorder: disabledBorder,
    focusedBorder: focusedBorder,
    errorBorder: errorBorder,
    focusedErrorBorder: errorBorder.copyWith(
      borderSide: BorderSide(color: colorScheme.error, width: 1.4),
    ),
    labelStyle: textTheme.labelSmall?.copyWith(
      color: colorScheme.onSurfaceVariant,
      fontWeight: FontWeight.w400,
    ),
    floatingLabelStyle: textTheme.labelSmall?.copyWith(
      color: colorScheme.primary,
      fontWeight: FontWeight.w500,
    ),
    hintStyle: textTheme.bodyMedium?.copyWith(
      color: colorScheme.onSurfaceVariant.withValues(alpha: 0.76),
    ),
    helperStyle: textTheme.bodySmall?.copyWith(
      color: colorScheme.onSurfaceVariant,
    ),
    errorStyle: textTheme.bodySmall?.copyWith(color: colorScheme.error),
    prefixIconColor: colorScheme.onSurfaceVariant,
    suffixIconColor: colorScheme.onSurfaceVariant,
  );
}

/// Keeps system bars transparent so application inset surfaces remain visible.
SystemUiOverlayStyle operitSystemUiOverlayStyle(ColorScheme colorScheme) {
  final iconBrightness = colorScheme.brightness == Brightness.dark
      ? Brightness.light
      : Brightness.dark;
  return SystemUiOverlayStyle(
    statusBarColor: Colors.transparent,
    statusBarIconBrightness: iconBrightness,
    statusBarBrightness: colorScheme.brightness,
    systemStatusBarContrastEnforced: false,
    systemNavigationBarColor: Colors.transparent,
    systemNavigationBarDividerColor: Colors.transparent,
    systemNavigationBarIconBrightness: iconBrightness,
    systemNavigationBarContrastEnforced: false,
  );
}
