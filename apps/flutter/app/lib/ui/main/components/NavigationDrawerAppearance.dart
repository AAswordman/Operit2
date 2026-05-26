// ignore_for_file: file_names

import 'package:flutter/material.dart';

class NavigationDrawerAppearance {
  const NavigationDrawerAppearance({
    required this.containerColor,
    required this.titleColor,
    required this.statusAvailableColor,
    required this.itemColor,
    required this.buttonContainerColor,
    required this.selectedContainerColor,
    required this.selectedContentColor,
    required this.dividerColor,
    required this.waterGlassEnabled,
    required this.buttonLiquidGlassEnabled,
  });

  final Color containerColor;
  final Color titleColor;
  final Color statusAvailableColor;
  final Color itemColor;
  final Color buttonContainerColor;
  final Color selectedContainerColor;
  final Color selectedContentColor;
  final Color dividerColor;
  final bool waterGlassEnabled;
  final bool buttonLiquidGlassEnabled;
}

NavigationDrawerAppearance navigationDrawerAppearanceOf(BuildContext context) {
  final colorScheme = Theme.of(context).colorScheme;
  final defaultTitleColor = colorScheme.primary;
  return NavigationDrawerAppearance(
    containerColor: colorScheme.surface,
    titleColor: defaultTitleColor,
    statusAvailableColor: colorScheme.primary,
    itemColor: colorScheme.onSurfaceVariant,
    buttonContainerColor: colorScheme.surfaceContainerHighest,
    selectedContainerColor: colorScheme.primaryContainer,
    selectedContentColor: colorScheme.primary,
    dividerColor: defaultTitleColor.withValues(alpha: 0.42),
    waterGlassEnabled: false,
    buttonLiquidGlassEnabled: false,
  );
}
