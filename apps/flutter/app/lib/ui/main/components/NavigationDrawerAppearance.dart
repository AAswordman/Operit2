// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart';

import '../../theme/OperitTheme.dart';

class NavigationDrawerAppearance extends ThemeDrawerPalette {
  const NavigationDrawerAppearance({
    required super.containerColor,
    required super.titleColor,
    required super.statusAvailableColor,
    required super.itemColor,
    required super.buttonContainerColor,
    required super.selectedContainerColor,
    required super.selectedContentColor,
    required super.dividerColor,
    required super.transparentSurfaceEnabled,
  });
}

NavigationDrawerAppearance navigationDrawerAppearanceOf(BuildContext context) {
  final colorScheme = Theme.of(context).colorScheme;
  final transparentSurface = OperitTheme.of(
    context,
  ).themePreferenceSnapshot.transparentSurfaceEnabled;
  final palette = ThemeDrawerPalette.fromScheme(
    colorScheme,
    transparentSurface: transparentSurface,
  );
  return NavigationDrawerAppearance(
    containerColor: palette.containerColor,
    titleColor: palette.titleColor,
    statusAvailableColor: palette.statusAvailableColor,
    itemColor: palette.itemColor,
    buttonContainerColor: palette.buttonContainerColor,
    selectedContainerColor: palette.selectedContainerColor,
    selectedContentColor: palette.selectedContentColor,
    dividerColor: palette.dividerColor,
    transparentSurfaceEnabled: palette.transparentSurfaceEnabled,
  );
}
