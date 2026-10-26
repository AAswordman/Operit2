import 'package:flutter/material.dart';

import 'bubble_surface.dart';
import 'liquid_glass.dart';

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
      surface = ClipPath(
        clipper: ShapeBorderClipper(shape: shape),
        child: LiquidGlassSurface(
          borderRadius: borderRadius,
          tint: const Color(0x09FFFFFF),
          child: decorated,
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
      surface = ClipPath(
        clipper: ShapeBorderClipper(shape: shape),
        child: LiquidGlassSurface(
          borderRadius: borderRadius,
          tint: const Color(0x09FFFFFF),
          child: decorated,
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
