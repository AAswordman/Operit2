// ignore_for_file: file_names

import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import 'OperitTheme.dart';

enum OperitGlassSurfaceLayer { panel, card, control }

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
  final bool clip;

  @override
  Widget build(BuildContext context) {
    final transparentSurface = OperitTheme.of(
      context,
    ).themePreferenceSnapshot.transparentSurfaceEnabled;
    final content = material
        ? Material(color: Colors.transparent, child: child)
        : child;
    if (!transparentSurface) {
      return _clipIfNeeded(
        DecoratedBox(
          decoration: BoxDecoration(
            color: color,
            borderRadius: borderRadius,
            border: border,
            boxShadow: shadows,
          ),
          child: content,
        ),
      );
    }

    final style = _styleForLayer(layer);
    return _clipBackdrop(
      BackdropFilter(
        filter: ui.ImageFilter.blur(sigmaX: style.blur, sigmaY: style.blur),
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: color.withValues(alpha: transparentAlpha ?? style.alpha),
            borderRadius: borderRadius,
            border: border,
            boxShadow: shadows
                .map(
                  (shadow) => BoxShadow(
                    color: shadow.color.withValues(alpha: 0.08),
                    offset: shadow.offset,
                    blurRadius: shadow.blurRadius,
                    spreadRadius: shadow.spreadRadius,
                  ),
                )
                .toList(growable: false),
          ),
          child: content,
        ),
      ),
    );
  }

  Widget _clipIfNeeded(Widget child) {
    if (!clip) {
      return child;
    }
    return ClipRRect(borderRadius: borderRadius, child: child);
  }

  Widget _clipBackdrop(Widget child) {
    if (!clip) {
      return ClipRect(child: child);
    }
    return ClipRRect(borderRadius: borderRadius, child: child);
  }
}

class _OperitGlassSurfaceStyle {
  const _OperitGlassSurfaceStyle({required this.blur, required this.alpha});

  final double blur;
  final double alpha;
}

_OperitGlassSurfaceStyle _styleForLayer(OperitGlassSurfaceLayer layer) {
  return switch (layer) {
    OperitGlassSurfaceLayer.panel => const _OperitGlassSurfaceStyle(
      blur: 16,
      alpha: 0.045,
    ),
    OperitGlassSurfaceLayer.card => const _OperitGlassSurfaceStyle(
      blur: 14,
      alpha: 0.16,
    ),
    OperitGlassSurfaceLayer.control => const _OperitGlassSurfaceStyle(
      blur: 10,
      alpha: 0.28,
    ),
  };
}
