import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:liquid_glass_widgets/widgets/shared/glass_effect.dart';

import 'bubble_surface.dart';

class AgentInputSurface extends StatelessWidget {
  /// Creates the agent surface with its original system-inset placement.
  const AgentInputSurface({
    super.key,
    required this.color,
    required this.floating,
    required this.shape,
    required this.borderRadius,
    required this.child,
    required this.width,
    required this.margin,
    required this.transparentSurface,
    this.imageStyle,
  });

  final Color color;
  final bool floating;
  final ShapeBorder shape;
  final BorderRadius borderRadius;
  final Widget child;
  final double width;
  final EdgeInsetsGeometry margin;
  final bool transparentSurface;
  final BubbleImageStyle? imageStyle;

  /// Retains the input surface paint while keyboard motion changes its position.
  @override
  Widget build(BuildContext context) {
    final useImage = imageStyle != null;
    final effectiveColor = (transparentSurface || useImage)
        ? Colors.transparent
        : color;
    final decorated = RepaintBoundary(
      child: SizedBox(
        width: width,
        child: ClipPath(
          clipper: ShapeBorderClipper(shape: shape),
          child: Stack(
            children: <Widget>[
              Positioned.fill(
                child: DecoratedBox(
                  key: const ValueKey<String>('chat.input.surface'),
                  decoration: ShapeDecoration(
                    color: effectiveColor,
                    shape: shape,
                    shadows: (transparentSurface || useImage)
                        ? const <BoxShadow>[]
                        : <BoxShadow>[
                            BoxShadow(
                              color: Colors.black.withValues(alpha: 0.08),
                              blurRadius: 18,
                              spreadRadius: 1,
                              offset: const Offset(0, -4),
                            ),
                            BoxShadow(
                              color: Colors.black.withValues(alpha: 0.035),
                              blurRadius: 5,
                              spreadRadius: 0,
                              offset: const Offset(0, -1),
                            ),
                          ],
                  ),
                ),
              ),
              if (useImage)
                Positioned.fill(
                  child: BubbleImageBackgroundSurface(imageStyle: imageStyle!),
                ),
              SafeArea(
                top: false,
                left: false,
                right: false,
                bottom: !floating,
                child: child,
              ),
            ],
          ),
        ),
      ),
    );
    final Widget surface;
    if (transparentSurface && !useImage) {
      final settings = _inputTransparentGlassSettings(context);
      surface = ClipPath(
        clipper: ShapeBorderClipper(shape: shape),
        child: Stack(
          children: <Widget>[
            Positioned.fill(
              child: BackdropFilter(
                filter: ui.ImageFilter.blur(
                  sigmaX: settings.blur,
                  sigmaY: settings.blur,
                ),
                child: const ColoredBox(color: Colors.transparent),
              ),
            ),
            Positioned.fill(
              child: GlassEffect(
                quality: GlassQuality.standard,
                shape: LiquidRoundedSuperellipse(
                  borderRadius: borderRadius.topLeft.x,
                ),
                settings: settings,
                interactionIntensity: 0.85,
                ambientRim: 0.11,
                baseAlphaMultiplier: 0.75,
                edgeAlphaMultiplier: 0.75,
                rimThickness: 1.48,
                rimSmoothing: 8,
                child: const SizedBox.expand(),
              ),
            ),
            decorated,
          ],
        ),
      );
    } else {
      surface = decorated;
    }
    return SafeArea(
      top: false,
      left: false,
      right: false,
      bottom: floating,
      child: Padding(padding: margin, child: surface),
    );
  }
}

LiquidGlassSettings _inputTransparentGlassSettings(BuildContext context) {
  final dark = Theme.of(context).brightness == Brightness.dark;
  return LiquidGlassSettings(
    glassColor: dark ? const Color(0x08FFFFFF) : const Color(0x07FFFFFF),
    thickness: 80,
    blur: 8,
    chromaticAberration: 0.18,
    lightIntensity: 0.42,
    ambientStrength: 0.2,
    refractiveIndex: 1.16,
    saturation: 1.04,
    glowIntensity: 0.17,
    specularSharpness: GlassSpecularSharpness.soft,
    standardOpacityMultiplier: 0.12,
  );
}

class ClassicInputSurface extends StatelessWidget {
  /// Creates the classic surface with its original system-inset placement.
  const ClassicInputSurface({
    super.key,
    required this.color,
    required this.floating,
    required this.shape,
    required this.borderRadius,
    required this.transparentSurface,
    required this.width,
    required this.margin,
    required this.child,
    this.imageStyle,
  });

  final Color color;
  final bool floating;
  final ShapeBorder shape;
  final BorderRadius borderRadius;
  final bool transparentSurface;
  final double width;
  final EdgeInsetsGeometry margin;
  final Widget child;
  final BubbleImageStyle? imageStyle;

  /// Builds the classic input surface with opaque, nine-slice or glass styling.
  @override
  Widget build(BuildContext context) {
    final useImage = imageStyle != null;
    final effectiveColor = (transparentSurface || useImage)
        ? Colors.transparent
        : color;
    final decorated = SizedBox(
      width: width,
      child: ClipPath(
        clipper: ShapeBorderClipper(shape: shape),
        child: Stack(
          children: <Widget>[
            Positioned.fill(
              child: DecoratedBox(
                key: const ValueKey<String>('chat.input.surface'),
                decoration: ShapeDecoration(
                  color: effectiveColor,
                  shape: shape,
                  shadows: (transparentSurface || useImage)
                      ? const <BoxShadow>[]
                      : <BoxShadow>[
                          BoxShadow(
                            color: Colors.black.withValues(alpha: 0.08),
                            blurRadius: 14,
                            offset: const Offset(0, -3),
                          ),
                        ],
                ),
              ),
            ),
            if (useImage)
              Positioned.fill(
                child: BubbleImageBackgroundSurface(imageStyle: imageStyle!),
              ),
            SafeArea(
              top: false,
              left: false,
              right: false,
              bottom: !floating,
              child: child,
            ),
          ],
        ),
      ),
    );
    final Widget surface;
    if (transparentSurface && !useImage) {
      final settings = _classicInputGlassSettings(context);
      surface = ClipPath(
        clipper: ShapeBorderClipper(shape: shape),
        child: Stack(
          children: <Widget>[
            Positioned.fill(
              child: BackdropFilter(
                filter: ui.ImageFilter.blur(
                  sigmaX: settings.blur,
                  sigmaY: settings.blur,
                ),
                child: const ColoredBox(color: Colors.transparent),
              ),
            ),
            Positioned.fill(
              child: GlassEffect(
                quality: GlassQuality.standard,
                shape: LiquidRoundedSuperellipse(
                  borderRadius: borderRadius.topLeft.x,
                ),
                settings: settings,
                interactionIntensity: 0.85,
                ambientRim: 0.11,
                baseAlphaMultiplier: 0.75,
                edgeAlphaMultiplier: 0.75,
                rimThickness: 1.48,
                rimSmoothing: 8,
                child: const SizedBox.expand(),
              ),
            ),
            decorated,
          ],
        ),
      );
    } else {
      surface = decorated;
    }
    return SafeArea(
      top: false,
      left: false,
      right: false,
      bottom: floating,
      child: Padding(padding: margin, child: surface),
    );
  }
}

/// Builds the glass settings used by the classic transparent input surface.
LiquidGlassSettings _classicInputGlassSettings(BuildContext context) {
  final dark = Theme.of(context).brightness == Brightness.dark;
  return LiquidGlassSettings(
    glassColor: dark ? const Color(0x08FFFFFF) : const Color(0x07FFFFFF),
    thickness: 80,
    blur: 8,
    chromaticAberration: 0.18,
    lightIntensity: 0.42,
    ambientStrength: 0.2,
    refractiveIndex: 1.16,
    saturation: 1.04,
    glowIntensity: 0.17,
    specularSharpness: GlassSpecularSharpness.soft,
    standardOpacityMultiplier: 0.12,
  );
}
