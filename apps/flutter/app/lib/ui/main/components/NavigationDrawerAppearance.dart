// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../theme/OperitTheme.dart';

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
    required this.transparentSurfaceEnabled,
  });

  final Color containerColor;
  final Color titleColor;
  final Color statusAvailableColor;
  final Color itemColor;
  final Color buttonContainerColor;
  final Color selectedContainerColor;
  final Color selectedContentColor;
  final Color dividerColor;
  final bool transparentSurfaceEnabled;
}

NavigationDrawerAppearance navigationDrawerAppearanceOf(BuildContext context) {
  final colorScheme = Theme.of(context).colorScheme;
  final transparentSurface = OperitTheme.of(
    context,
  ).themePreferenceSnapshot.transparentSurfaceEnabled;
  return NavigationDrawerAppearance(
    containerColor: transparentSurface
        ? colorScheme.surface.withValues(alpha: 0.04)
        : colorScheme.surface.withValues(alpha: 0.88),
    titleColor: colorScheme.onSurface,
    statusAvailableColor: colorScheme.primary,
    itemColor: colorScheme.onSurfaceVariant,
    buttonContainerColor: transparentSurface
        ? colorScheme.surfaceContainerLow.withValues(alpha: 0.18)
        : colorScheme.surfaceContainerLow.withValues(alpha: 0.72),
    selectedContainerColor: transparentSurface
        ? colorScheme.secondaryContainer.withValues(alpha: 0.34)
        : colorScheme.secondaryContainer.withValues(alpha: 0.78),
    selectedContentColor: colorScheme.onSecondaryContainer,
    dividerColor: transparentSurface
        ? colorScheme.outlineVariant.withValues(alpha: 0.34)
        : colorScheme.outlineVariant.withValues(alpha: 0.62),
    transparentSurfaceEnabled: transparentSurface,
  );
}
