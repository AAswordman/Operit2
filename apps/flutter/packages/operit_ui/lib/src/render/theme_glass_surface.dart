import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import 'theme_glass_scroll_behavior.dart';

enum ThemeGlassSurfaceLayer { panel, card, control }

/// Shares matching frost filters within one non-overlapping surface collection.
class ThemeGlassSurfaceGroup extends StatefulWidget {
  /// Creates a collection with a separate backdrop identity for each blur radius.
  const ThemeGlassSurfaceGroup({super.key, required this.child});

  final Widget child;

  /// Creates stable backdrop identities for this collection's lifetime.
  @override
  State<ThemeGlassSurfaceGroup> createState() => _ThemeGlassSurfaceGroupState();
}

class _ThemeGlassSurfaceGroupState extends State<ThemeGlassSurfaceGroup> {
  final _keys = <ThemeGlassSurfaceLayer, BackdropKey>{
    for (final layer in ThemeGlassSurfaceLayer.values) layer: BackdropKey(),
  };

  /// Shares blur identities and composes scrolling stretch with the live backdrop.
  @override
  Widget build(BuildContext context) {
    return ThemeGlassScrollConfiguration(
      child: _ThemeGlassSurfaceScope(keys: _keys, child: widget.child),
    );
  }
}

class _ThemeGlassSurfaceScope extends InheritedWidget {
  /// Associates sibling surfaces with their collection's blur identities.
  const _ThemeGlassSurfaceScope({required this.keys, required super.child});

  final Map<ThemeGlassSurfaceLayer, BackdropKey> keys;

  /// Resolves the optional grouping identity accepted by Flutter's filter API.
  static BackdropKey? keyOf(
    BuildContext context,
    ThemeGlassSurfaceLayer layer,
  ) {
    return context
        .dependOnInheritedWidgetOfExactType<_ThemeGlassSurfaceScope>()
        ?.keys[layer];
  }

  /// Updates descendants only when the owning collection changes.
  @override
  bool updateShouldNotify(_ThemeGlassSurfaceScope oldWidget) {
    return keys != oldWidget.keys;
  }
}

class ThemeGlassSurface extends StatelessWidget {
  /// Creates the original frosted material for general theme surfaces.
  const ThemeGlassSurface({
    super.key,
    required this.child,
    required this.color,
    required this.transparentSurface,
    this.borderRadius = BorderRadius.zero,
    this.border,
    this.shadows = const <BoxShadow>[],
    this.layer = ThemeGlassSurfaceLayer.card,
    this.transparentAlpha,
    this.material = false,
    this.enableBackdropFilter = true,
    this.clip = true,
  });

  final Widget child;
  final Color color;
  final bool transparentSurface;
  final BorderRadius borderRadius;
  final BoxBorder? border;
  final List<BoxShadow> shadows;
  final ThemeGlassSurfaceLayer layer;
  final double? transparentAlpha;
  final bool material;

  /// Keeps translucent styling without paying for a per-surface blur pass.
  final bool enableBackdropFilter;
  final bool clip;

  /// Builds the configured opaque or frosted surface.
  @override
  Widget build(BuildContext context) {
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
    final decoration = BoxDecoration(
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
    );
    if (!enableBackdropFilter) {
      return _clipIfNeeded(
        DecoratedBox(decoration: decoration, child: content),
      );
    }
    return _clipBackdrop(
      BackdropFilter(
        backdropGroupKey: _ThemeGlassSurfaceScope.keyOf(context, layer),
        filter: ui.ImageFilter.blur(sigmaX: style.blur, sigmaY: style.blur),
        child: DecoratedBox(
          decoration: decoration,
          // Nested surfaces sample this surface's result, not its backdrop.
          child: ThemeGlassSurfaceGroup(child: content),
        ),
      ),
    );
  }

  /// Applies the requested corner clipping to the decorated content.
  Widget _clipIfNeeded(Widget child) {
    if (!clip) {
      return child;
    }
    return ClipRRect(borderRadius: borderRadius, child: child);
  }

  /// Bounds the blur region even when rounded content clipping is disabled.
  Widget _clipBackdrop(Widget child) {
    if (!clip) {
      return ClipRect(child: child);
    }
    return ClipRRect(borderRadius: borderRadius, child: child);
  }
}

class _ThemeGlassSurfaceStyle {
  /// Stores the original blur radius and tint strength for each surface layer.
  const _ThemeGlassSurfaceStyle({required this.blur, required this.alpha});

  final double blur;
  final double alpha;
}

/// Returns the established frosted styling for a theme surface layer.
_ThemeGlassSurfaceStyle _styleForLayer(ThemeGlassSurfaceLayer layer) {
  return switch (layer) {
    ThemeGlassSurfaceLayer.panel => const _ThemeGlassSurfaceStyle(
      blur: 16,
      alpha: 0.045,
    ),
    ThemeGlassSurfaceLayer.card => const _ThemeGlassSurfaceStyle(
      blur: 14,
      alpha: 0.16,
    ),
    ThemeGlassSurfaceLayer.control => const _ThemeGlassSurfaceStyle(
      blur: 10,
      alpha: 0.28,
    ),
  };
}
