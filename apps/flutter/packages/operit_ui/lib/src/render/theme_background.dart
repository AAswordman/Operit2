import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import '../model/theme_preference_snapshot.dart';
import 'liquid_glass.dart';

const _backgroundDuration = Duration(milliseconds: 360);

/// Composition is shared; the application alone supplies private video media.
class ThemeBackground extends StatelessWidget {
  /// Creates a scene with one source texture shared by all liquid lenses.
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

  /// Builds the shared theme background composition.
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
    return LiquidGlassBackdrop(
      fit: fit,
      background: Stack(
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
      child: child,
    );
  }
}

class ThemeBackgroundMedia extends StatelessWidget {
  /// Creates the configured media opacity and blur presentation.
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

  /// Applies the user's media opacity and background blur preferences.
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
