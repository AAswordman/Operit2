// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

class M3LoadingIndicator extends StatefulWidget {
  const M3LoadingIndicator({super.key, this.size = 36, this.color});

  final double size;
  final Color? color;

  @override
  State<M3LoadingIndicator> createState() => _M3LoadingIndicatorState();
}

class _M3LoadingIndicatorState extends State<M3LoadingIndicator>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 1320),
  )..repeat();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final color = widget.color ?? Theme.of(context).colorScheme.primary;
    final size = widget.size;
    return SizedBox.square(
      dimension: size,
      child: AnimatedBuilder(
        animation: _controller,
        builder: (context, _) {
          return CustomPaint(
            painter: _M3LoadingPolygonPainter(
              color: color,
              progress: _controller.value,
            ),
          );
        },
      ),
    );
  }
}

class _M3LoadingPolygonPainter extends CustomPainter {
  const _M3LoadingPolygonPainter({required this.color, required this.progress});

  final Color color;
  final double progress;

  static const List<int> _sides = <int>[5, 6, 8, 10];

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final radius = math.min(size.width, size.height) * 0.34;
    final phase = progress * _sides.length;
    final index = phase.floor() % _sides.length;
    final nextIndex = (index + 1) % _sides.length;
    final morph = Curves.easeInOutCubic.transform(phase - phase.floor());
    final rotation = progress * math.pi * 2;

    _drawPolygon(
      canvas,
      center: center,
      radius: radius * (1 - morph * 0.08),
      sides: _sides[index],
      rotation: rotation,
      alpha: 1 - morph,
    );
    _drawPolygon(
      canvas,
      center: center,
      radius: radius * (0.92 + morph * 0.08),
      sides: _sides[nextIndex],
      rotation: rotation + morph * math.pi / 6,
      alpha: morph,
    );
  }

  void _drawPolygon(
    Canvas canvas, {
    required Offset center,
    required double radius,
    required int sides,
    required double rotation,
    required double alpha,
  }) {
    if (alpha <= 0) {
      return;
    }
    final path = Path();
    for (var i = 0; i < sides; i++) {
      final angle = rotation - math.pi / 2 + (math.pi * 2 * i / sides);
      final point = Offset(
        center.dx + math.cos(angle) * radius,
        center.dy + math.sin(angle) * radius,
      );
      if (i == 0) {
        path.moveTo(point.dx, point.dy);
      } else {
        path.lineTo(point.dx, point.dy);
      }
    }
    path.close();
    canvas.drawPath(path, Paint()..color = color.withValues(alpha: alpha));
  }

  @override
  bool shouldRepaint(_M3LoadingPolygonPainter oldDelegate) {
    return oldDelegate.color != color || oldDelegate.progress != progress;
  }
}

class M3LoadingPane extends StatelessWidget {
  const M3LoadingPane({super.key, this.size = 38});

  final double size;

  @override
  Widget build(BuildContext context) {
    return Center(child: M3LoadingIndicator(size: size));
  }
}

class M3LoadingOverlay extends StatelessWidget {
  const M3LoadingOverlay({super.key});

  @override
  Widget build(BuildContext context) {
    return const M3LoadingPane(size: 32);
  }
}

class M3LoadingFooter extends StatelessWidget {
  const M3LoadingFooter({super.key});

  @override
  Widget build(BuildContext context) {
    return const Center(child: M3LoadingIndicator(size: 24));
  }
}
