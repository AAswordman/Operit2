import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import '../model/theme_preference_snapshot.dart';

const _backgroundDuration = Duration(milliseconds: 360);

/// Composition is shared; the application alone supplies private video media.
class ThemeBackground extends StatelessWidget {
  const ThemeBackground({
    super.key,
    required this.themePreferenceSnapshot,
    required this.child,
    required this.media,
    this.fit = StackFit.expand,
  });

  final ThemePreferenceSnapshot themePreferenceSnapshot;
  final Widget child;
  final Widget? media;
  final StackFit fit;

  @override
  Widget build(BuildContext context) {
    final path = themePreferenceSnapshot.backgroundImageUri;
    final hasMedia =
        themePreferenceSnapshot.useBackgroundImage &&
        path != null &&
        path.isNotEmpty;
    if (hasMedia && media == null) {
      throw StateError('Background media must be prepared before rendering');
    }
    return LiquidGlassScope(
      child: Stack(
        fit: fit,
        children: <Widget>[
          Positioned.fill(
            child: GlassBackgroundSource(
              child: Stack(
                fit: StackFit.expand,
                children: <Widget>[
                  AnimatedContainer(
                    duration: _backgroundDuration,
                    curve: Curves.easeOutCubic,
                    color: Theme.of(context).colorScheme.surface,
                  ),
                  if (hasMedia)
                    media!
                  else
                    const SizedBox.expand(
                      key: ValueKey<String>('empty-theme-background'),
                    ),
                ],
              ),
            ),
          ),
          child,
        ],
      ),
    );
  }
}

class ThemeBackgroundMedia extends StatelessWidget {
  const ThemeBackgroundMedia({
    super.key,
    required this.child,
    required this.opacity,
    required this.blurEnabled,
    required this.blurRadius,
  });

  final Widget child;
  final double opacity;
  final bool blurEnabled;
  final double blurRadius;

  @override
  Widget build(BuildContext context) {
    return AnimatedOpacity(
      duration: _backgroundDuration,
      curve: Curves.easeOutCubic,
      opacity: opacity.clamp(0, 1),
      child: blurEnabled
          ? ImageFiltered(
              imageFilter: ui.ImageFilter.blur(
                sigmaX: blurRadius,
                sigmaY: blurRadius,
              ),
              child: child,
            )
          : child,
    );
  }
}
