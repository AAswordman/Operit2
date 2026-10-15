import 'package:flutter/material.dart';

import 'theme_glass_surface.dart';

/// Exact sidebar container paint shared by the desktop client and Studio.
/// Navigation data and interactions remain owned by the caller.
class ThemeDesktopSidebarSurface extends StatelessWidget {
  const ThemeDesktopSidebarSurface({
    super.key,
    required this.child,
    required this.color,
    required this.transparentSurface,
  });

  final Widget child;
  final Color color;
  final bool transparentSurface;

  @override
  Widget build(BuildContext context) {
    return ThemeGlassSurface(
      color: color,
      transparentSurface: transparentSurface,
      layer: ThemeGlassSurfaceLayer.panel,
      transparentAlpha: 0.035,
      borderRadius: BorderRadius.zero,
      shadows: <BoxShadow>[
        BoxShadow(blurRadius: 4, color: Colors.black.withValues(alpha: 0.12)),
      ],
      child: child,
    );
  }
}

/// Preserves the original sidebar surface in opaque and transparent modes.
Color themeSidebarContainerColor(
  ColorScheme scheme, {
  required bool transparentSurface,
}) => transparentSurface
    ? scheme.surface.withValues(alpha: 0.04)
    : scheme.surface.withValues(alpha: 0.88);
