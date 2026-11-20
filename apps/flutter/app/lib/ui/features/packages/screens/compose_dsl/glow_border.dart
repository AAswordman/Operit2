import 'dart:math' as math;

import 'package:flutter/material.dart';

/// A transparent, host-animated perimeter. Animation never crosses plugin IPC.
class ComposeGlowBorder extends StatefulWidget {
  const ComposeGlowBorder({
    super.key,
    required this.colors,
    this.cornerRadius = 0,
    this.strokeWidth = 3,
    this.glowWidth = 56,
    this.duration = const Duration(seconds: 6),
    this.running = true,
  });

  final List<Color> colors;
  final double cornerRadius, strokeWidth, glowWidth;
  final Duration duration;
  final bool running;

  @override
  State<ComposeGlowBorder> createState() => _ComposeGlowBorderState();
}

class _ComposeGlowBorderState extends State<ComposeGlowBorder>
    with SingleTickerProviderStateMixin {
  late final _animation = AnimationController(vsync: this);

  void _syncAnimation() {
    _animation.duration = widget.duration;
    if (widget.running && !MediaQuery.disableAnimationsOf(context)) {
      if (!_animation.isAnimating) _animation.repeat();
    } else {
      _animation.stop();
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _syncAnimation();
  }

  @override
  void didUpdateWidget(ComposeGlowBorder oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.duration != widget.duration) _animation.stop();
    _syncAnimation();
  }

  @override
  void dispose() {
    _animation.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => IgnorePointer(
    child: RepaintBoundary(
      child: CustomPaint(
        painter: _GlowBorderPainter(
          animation: _animation,
          colors: widget.colors,
          cornerRadius: widget.cornerRadius,
          strokeWidth: widget.strokeWidth,
          glowWidth: widget.glowWidth,
        ),
        child: const SizedBox.expand(),
      ),
    ),
  );
}

class _GlowBorderPainter extends CustomPainter {
  _GlowBorderPainter({
    required this.animation,
    required this.colors,
    required this.cornerRadius,
    required this.strokeWidth,
    required this.glowWidth,
  }) : super(repaint: animation);

  final Animation<double> animation;
  final List<Color> colors;
  final double cornerRadius, strokeWidth, glowWidth;

  @override
  void paint(Canvas canvas, Size size) {
    if (size.shortestSide <= 2 || colors.length < 2) return;
    final stroke = strokeWidth.clamp(0.5, size.shortestSide / 4);
    final glow = glowWidth.clamp(stroke, size.shortestSide / 4);
    final bounds = Offset.zero & size;
    final frame = bounds.deflate(stroke / 2 + 0.5);
    final path = Path()
      ..addRRect(
        RRect.fromRectAndRadius(
          frame,
          Radius.circular(cornerRadius.clamp(0, frame.shortestSide / 2)),
        ),
      );
    final phase = animation.value * math.pi * 2;
    final breath = 0.82 + 0.18 * math.sin(phase * 2);
    final palette = [...colors, colors.first];
    Shader shader(double alpha) => SweepGradient(
      colors: palette
          .map((color) => color.withValues(alpha: color.a * alpha))
          .toList(),
      transform: GradientRotation(phase),
    ).createShader(bounds);

    // Clip only the outer surface, leaving a wide, feathered glow inside it.
    canvas.save();
    canvas.clipRect(bounds);
    for (final layer in <(double, double, double)>[
      (glow, glow * 0.45, 0.42 * breath),
      (glow * 0.42, glow * 0.18, 0.64 * breath),
      (stroke, 0, 0.92),
    ]) {
      final paint = Paint()
        ..style = PaintingStyle.stroke
        ..strokeJoin = StrokeJoin.miter
        ..strokeWidth = layer.$1
        ..shader = shader(layer.$3);
      if (layer.$2 > 0) {
        paint.maskFilter = MaskFilter.blur(BlurStyle.normal, layer.$2);
      }
      canvas.drawPath(path, paint);
    }

    // Two soft glints travel at constant perimeter speed, even on tall screens.
    final metric = path.computeMetrics().first;
    final trailLength = math.min(metric.length * 0.16, 260.0);
    for (final offset in [0.0, 0.5]) {
      final head = ((animation.value + offset) % 1) * metric.length;
      for (var segment = 0; segment < 16; segment++) {
        final start =
            (head - trailLength + trailLength * segment / 16) % metric.length;
        final end = start + trailLength / 16;
        final trail = metric.extractPath(start, math.min(end, metric.length));
        if (end > metric.length) {
          trail.addPath(
            metric.extractPath(0, end - metric.length),
            Offset.zero,
          );
        }
        canvas.drawPath(
          trail,
          Paint()
            ..style = PaintingStyle.stroke
            ..strokeWidth = stroke * 1.25
            ..strokeCap = StrokeCap.round
            ..color = Colors.white.withValues(
              alpha: math.pow((segment + 1) / 16, 2).toDouble() * 0.72,
            )
            ..maskFilter = const MaskFilter.blur(BlurStyle.normal, 1.2),
        );
      }
    }
    canvas.restore();
  }

  @override
  bool shouldRepaint(_GlowBorderPainter oldDelegate) =>
      oldDelegate.animation != animation ||
      oldDelegate.colors != colors ||
      oldDelegate.cornerRadius != cornerRadius ||
      oldDelegate.strokeWidth != strokeWidth ||
      oldDelegate.glowWidth != glowWidth;
}
