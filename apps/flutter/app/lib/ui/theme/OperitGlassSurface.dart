// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart';

import 'OperitTheme.dart';

typedef OperitGlassSurfaceLayer = ThemeGlassSurfaceLayer;

class OperitGlassSurface extends StatelessWidget {
  const OperitGlassSurface({
    super.key,
    required this.child,
    required this.color,
    this.borderRadius = BorderRadius.zero,
    this.border,
    this.shadows = const <BoxShadow>[],
    this.layer = OperitGlassSurfaceLayer.card,
    this.transparentAlpha,
    this.material = false,
    this.enableBackdropFilter = true,
    this.clip = true,
  });

  final Widget child;
  final Color color;
  final BorderRadius borderRadius;
  final BoxBorder? border;
  final List<BoxShadow> shadows;
  final OperitGlassSurfaceLayer layer;
  final double? transparentAlpha;
  final bool material;

  /// Keeps translucent styling without paying for a per-surface blur pass.
  final bool enableBackdropFilter;
  final bool clip;

  @override
  Widget build(BuildContext context) {
    return ThemeGlassSurface(
      transparentSurface: OperitTheme.of(
        context,
      ).themePreferenceSnapshot.transparentSurfaceEnabled,
      child: child,
      color: color,
      borderRadius: borderRadius,
      border: border,
      shadows: shadows,
      layer: layer,
      transparentAlpha: transparentAlpha,
      material: material,
      enableBackdropFilter: enableBackdropFilter,
      clip: clip,
    );
  }
}
