// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import 'OperitLogoMark.dart';

class RuntimeBootstrapScreen extends StatefulWidget {
  const RuntimeBootstrapScreen({
    super.key,
    this.message = '正在准备本地运行时',
    this.errorText,
  });

  final String message;
  final String? errorText;

  /// Creates the animated runtime bootstrap screen state.
  @override
  State<RuntimeBootstrapScreen> createState() => _RuntimeBootstrapScreenState();
}

class _RuntimeBootstrapScreenState extends State<RuntimeBootstrapScreen>
    with TickerProviderStateMixin {
  late final AnimationController _entranceController;
  late final AnimationController _progressController;

  /// Starts the repeating runtime bootstrap animation.
  @override
  void initState() {
    super.initState();
    _entranceController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 760),
    )..forward();
    _progressController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 2200),
    )..repeat();
  }

  /// Releases the runtime bootstrap animation controller.
  @override
  void dispose() {
    _entranceController.dispose();
    _progressController.dispose();
    super.dispose();
  }

  /// Builds the shared runtime bootstrap visual.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Material(
      color: colorScheme.surface,
      child: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 32),
          child: AnimatedBuilder(
            animation: Listenable.merge(<Listenable>[
              _entranceController,
              _progressController,
            ]),
            builder: (context, child) {
              final entrance = Curves.easeOutCubic.transform(
                _entranceController.value,
              );
              return Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Transform.scale(
                    scale: 0.96 + entrance * 0.04,
                    child: Opacity(
                      opacity: entrance,
                      child: OperitLogoMark(
                        size: 96,
                        color: colorScheme.primary,
                      ),
                    ),
                  ),
                  const SizedBox(height: 18),
                  RuntimeBootstrapBrandText(
                    opacity: entrance,
                    fontSize: textTheme.headlineMedium?.fontSize ?? 28,
                    height: 1.18,
                  ),
                  const SizedBox(height: 28),
                  RuntimeBootstrapLiquidProgress(
                    color: colorScheme.primary,
                    trackColor: colorScheme.surfaceContainerHighest,
                    progress: _progressController.value,
                  ),
                  const SizedBox(height: 12),
                  Text(
                    widget.message,
                    textAlign: TextAlign.center,
                    style: textTheme.labelMedium?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                      fontWeight: FontWeight.w700,
                      letterSpacing: 0,
                    ),
                  ),
                  if (widget.errorText != null) ...<Widget>[
                    const SizedBox(height: 18),
                    ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 560),
                      child: SelectableText(
                        widget.errorText!,
                        textAlign: TextAlign.center,
                        style: textTheme.bodySmall?.copyWith(
                          color: colorScheme.error,
                          height: 1.4,
                        ),
                      ),
                    ),
                  ],
                ],
              );
            },
          ),
        ),
      ),
    );
  }
}

class RuntimeBootstrapLiquidProgress extends StatelessWidget {
  const RuntimeBootstrapLiquidProgress({
    super.key,
    required this.color,
    required this.trackColor,
    required this.progress,
  });

  final Color color;
  final Color trackColor;
  final double progress;

  /// Builds the shared liquid runtime progress indicator.
  @override
  Widget build(BuildContext context) {
    return CustomPaint(
      size: const Size(168, 16),
      painter: _RuntimeBootstrapLiquidProgressPainter(
        color: color,
        trackColor: trackColor,
        progress: progress,
      ),
    );
  }
}

class _RuntimeBootstrapLiquidProgressPainter extends CustomPainter {
  const _RuntimeBootstrapLiquidProgressPainter({
    required this.color,
    required this.trackColor,
    required this.progress,
  });

  final Color color;
  final Color trackColor;
  final double progress;

  /// Paints the animated liquid runtime progress indicator.
  @override
  void paint(Canvas canvas, Size size) {
    final radius = Radius.circular(size.height / 2);
    final rect = Offset.zero & size;
    final rrect = RRect.fromRectAndRadius(rect, radius);
    canvas.drawRRect(
      rrect,
      Paint()..color = trackColor.withValues(alpha: 0.58),
    );

    canvas.save();
    canvas.clipRRect(rrect);

    final phase = progress * math.pi * 2;
    final fillWidth = size.width * (0.42 + 0.18 * math.sin(progress * math.pi));
    final fillLeft = ((size.width + fillWidth) * progress - fillWidth)
        .clamp(-fillWidth, size.width)
        .toDouble();
    final fillRect = Rect.fromLTWH(fillLeft, 0, fillWidth, size.height);
    final fillPaint = Paint()
      ..shader = LinearGradient(
        colors: <Color>[
          color.withValues(alpha: 0.16),
          color.withValues(alpha: 0.92),
          Color.lerp(color, Colors.white, 0.28)!.withValues(alpha: 0.82),
          color.withValues(alpha: 0.24),
        ],
        stops: const <double>[0, 0.42, 0.68, 1],
      ).createShader(fillRect);
    canvas.drawRect(fillRect, fillPaint);

    final wavePaint = Paint()
      ..color = Color.lerp(color, Colors.white, 0.46)!.withValues(alpha: 0.38)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.2
      ..strokeCap = StrokeCap.round;
    final wave = Path();
    for (var x = 0.0; x <= size.width; x += 4) {
      final y =
          size.height * 0.5 +
          math.sin((x / size.width * math.pi * 2) + phase) * 2.2;
      if (x == 0) {
        wave.moveTo(x, y);
      } else {
        wave.lineTo(x, y);
      }
    }
    canvas.drawPath(wave, wavePaint);
    canvas.restore();
  }

  /// Returns whether the liquid progress indicator needs repainting.
  @override
  bool shouldRepaint(_RuntimeBootstrapLiquidProgressPainter oldDelegate) {
    return color != oldDelegate.color ||
        trackColor != oldDelegate.trackColor ||
        progress != oldDelegate.progress;
  }
}

class RuntimeBootstrapBrandText extends StatelessWidget {
  const RuntimeBootstrapBrandText({
    super.key,
    required this.opacity,
    required this.fontSize,
    required this.height,
  });

  final double opacity;
  final double fontSize;
  final double? height;

  /// Builds the shared Operit bootstrap brand text.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final textOpacity = opacity.clamp(0.0, 1.0).toDouble();
    final textStyle = (textTheme.headlineMedium ?? const TextStyle()).copyWith(
      color: Colors.white.withValues(alpha: textOpacity),
      fontSize: fontSize,
      fontWeight: FontWeight.w900,
      height: height,
      letterSpacing: 0,
      shadows: <Shadow>[
        Shadow(
          color: colorScheme.primary.withValues(alpha: 0.18 * textOpacity),
          blurRadius: 18,
          offset: const Offset(0, 8),
        ),
      ],
    );

    return ShaderMask(
      blendMode: BlendMode.srcIn,
      shaderCallback: (bounds) {
        return LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: <Color>[
            Color.lerp(colorScheme.primary, colorScheme.onSurface, 0.22)!,
            colorScheme.onSurface,
            Color.lerp(colorScheme.tertiary, colorScheme.primary, 0.32)!,
          ],
          stops: const <double>[0, 0.56, 1],
        ).createShader(bounds);
      },
      child: Text('Operit', textAlign: TextAlign.center, style: textStyle),
    );
  }
}
