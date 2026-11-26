// ignore_for_file: file_names

import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import 'TopBarCapsuleModels.dart';

/// Elongated dynamic status capsule rendered in the chat top bar.
///
/// Renders only from [TopBarCapsulePresentation]: a dynamic kinetic icon on the
/// left, an animated status label on the right, and an expansion chevron.
class TopBarDynamicCapsule extends StatefulWidget {
  const TopBarDynamicCapsule({
    super.key,
    required this.presentation,
    required this.isOpen,
    required this.onPressed,
  });

  final TopBarCapsulePresentation presentation;
  final bool isOpen;
  final VoidCallback onPressed;

  @override
  State<TopBarDynamicCapsule> createState() => _TopBarDynamicCapsuleState();
}

class _TopBarDynamicCapsuleState extends State<TopBarDynamicCapsule>
    with SingleTickerProviderStateMixin {
  late final AnimationController _shimmerController;

  @override
  void initState() {
    super.initState();
    _shimmerController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1600),
    );
    _syncShimmer();
  }

  @override
  void didUpdateWidget(covariant TopBarDynamicCapsule oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.presentation.showShimmer != widget.presentation.showShimmer) {
      _syncShimmer();
    }
  }

  void _syncShimmer() {
    if (widget.presentation.showShimmer) {
      if (!_shimmerController.isAnimating) {
        _shimmerController.repeat();
      }
    } else {
      _shimmerController.stop();
      _shimmerController.value = 0;
    }
  }

  @override
  void dispose() {
    _shimmerController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final presentation = widget.presentation;
    final palette = _CapsulePalette.fromTone(
      tone: presentation.tone,
      colors: colors,
    );

    final screenWidth = MediaQuery.sizeOf(context).width;
    final maxCapsuleWidth = math.min(208.0, math.max(96.0, screenWidth * 0.44));

    return Tooltip(
      message: presentation.tooltip,
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: widget.onPressed,
          borderRadius: BorderRadius.circular(999),
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 220),
            curve: Curves.easeOutCubic,
            constraints: BoxConstraints(maxWidth: maxCapsuleWidth),
            decoration: BoxDecoration(
              color: palette.background,
              borderRadius: BorderRadius.circular(999),
              border: Border.all(
                color: widget.isOpen ? colors.primary : palette.border,
              ),
              boxShadow:
                  presentation.tone == TopBarCapsuleTone.active ||
                      presentation.tone == TopBarCapsuleTone.success
                  ? <BoxShadow>[
                      BoxShadow(
                        color: palette.accent.withValues(alpha: 0.22),
                        blurRadius: 10,
                        offset: const Offset(0, 2),
                      ),
                    ]
                  : null,
            ),
            child: ClipRRect(
              borderRadius: BorderRadius.circular(999),
              child: Stack(
                alignment: Alignment.centerLeft,
                children: <Widget>[
                  if (presentation.progressFraction case final fraction?)
                    Positioned.fill(
                      child: Align(
                        alignment: AlignmentDirectional.centerStart,
                        child: AnimatedFractionallySizedBox(
                          duration: const Duration(milliseconds: 360),
                          curve: Curves.easeOutCubic,
                          widthFactor: fraction.clamp(0.0, 1.0),
                          heightFactor: 1,
                          child: ColoredBox(
                            color: palette.accent.withValues(alpha: 0.14),
                          ),
                        ),
                      ),
                    ),
                  if (presentation.showShimmer)
                    Positioned.fill(
                      child: AnimatedBuilder(
                        animation: _shimmerController,
                        builder: (context, _) {
                          return CustomPaint(
                            painter: _ShimmerSweepPainter(
                              progress: _shimmerController.value,
                              color: colors.onSurface.withValues(alpha: 0.10),
                            ),
                          );
                        },
                      ),
                    ),
                  Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 3.5,
                    ),
                    child: AnimatedSize(
                      duration: const Duration(milliseconds: 220),
                      curve: Curves.easeOutCubic,
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: <Widget>[
                          _KineticCapsuleIcon(
                            iconKey: presentation.iconKey,
                            kind: presentation.iconKind,
                            accent: palette.accent,
                            trackColor: colors.onSurface.withValues(alpha: 0.18),
                            percentage: presentation.contextPercentage,
                          ),
                          if (presentation.label.isNotEmpty) ...<Widget>[
                            const SizedBox(width: 5),
                            Flexible(
                              child: AnimatedSwitcher(
                                duration: const Duration(milliseconds: 220),
                                switchInCurve: Curves.easeOutCubic,
                                switchOutCurve: Curves.easeInCubic,
                                transitionBuilder: (child, animation) {
                                  final offset = Tween<Offset>(
                                    begin: const Offset(0, 0.35),
                                    end: Offset.zero,
                                  ).animate(animation);
                                  return FadeTransition(
                                    opacity: animation,
                                    child: SlideTransition(
                                      position: offset,
                                      child: child,
                                    ),
                                  );
                                },
                                child: Text(
                                  presentation.label,
                                  key: ValueKey<String>(presentation.key),
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: theme.textTheme.labelSmall?.copyWith(
                                    color: palette.foreground,
                                    fontWeight: FontWeight.w500,
                                    fontFeatures: const <FontFeature>[
                                      FontFeature.tabularFigures(),
                                    ],
                                  ),
                                ),
                              ),
                            ),
                          ],
                          const SizedBox(width: 3),
                          AnimatedRotation(
                            turns: widget.isOpen ? 0.5 : 0.0,
                            duration: const Duration(milliseconds: 180),
                            curve: Curves.easeOutCubic,
                            child: Icon(
                              Icons.expand_more_rounded,
                              size: 14,
                              color: colors.onSurfaceVariant,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _CapsulePalette {
  const _CapsulePalette({
    required this.background,
    required this.border,
    required this.foreground,
    required this.accent,
  });

  final Color background;
  final Color border;
  final Color foreground;
  final Color accent;

  static _CapsulePalette fromTone({
    required TopBarCapsuleTone tone,
    required ColorScheme colors,
  }) {
    final successColor = colors.brightness == Brightness.dark
        ? Colors.green.shade400
        : Colors.green.shade700;
    final warnColor = colors.brightness == Brightness.dark
        ? Colors.amber.shade300
        : Colors.orange.shade800;

    return switch (tone) {
      TopBarCapsuleTone.idle => _CapsulePalette(
        background: colors.surfaceContainerHigh.withValues(alpha: 0.72),
        border: colors.outlineVariant.withValues(alpha: 0.7),
        foreground: colors.onSurface,
        accent: colors.primary,
      ),
      TopBarCapsuleTone.active => _CapsulePalette(
        background: colors.primary.withValues(alpha: 0.13),
        border: colors.primary.withValues(alpha: 0.52),
        foreground: colors.onSurface,
        accent: colors.primary,
      ),
      TopBarCapsuleTone.success => _CapsulePalette(
        background: successColor.withValues(alpha: 0.14),
        border: successColor.withValues(alpha: 0.52),
        foreground: colors.onSurface,
        accent: successColor,
      ),
      TopBarCapsuleTone.warning => _CapsulePalette(
        background: warnColor.withValues(alpha: 0.12),
        border: warnColor.withValues(alpha: 0.45),
        foreground: colors.onSurface,
        accent: warnColor,
      ),
      TopBarCapsuleTone.error => _CapsulePalette(
        background: colors.error.withValues(alpha: 0.14),
        border: colors.error.withValues(alpha: 0.52),
        foreground: colors.error,
        accent: colors.error,
      ),
    };
  }
}

class _ShimmerSweepPainter extends CustomPainter {
  const _ShimmerSweepPainter({required this.progress, required this.color});

  final double progress;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final centerX = -0.3 + progress * 1.6;
    final paint = Paint()
      ..shader = ui.Gradient.linear(
        Offset((centerX - 0.25) * size.width, 0),
        Offset((centerX + 0.25) * size.width, size.height),
        <Color>[Colors.transparent, color, Colors.transparent],
        const <double>[0.0, 0.5, 1.0],
      );
    canvas.drawRect(Offset.zero & size, paint);
  }

  @override
  bool shouldRepaint(covariant _ShimmerSweepPainter oldDelegate) =>
      oldDelegate.progress != progress || oldDelegate.color != color;
}

class _KineticCapsuleIcon extends StatefulWidget {
  const _KineticCapsuleIcon({
    required this.iconKey,
    required this.kind,
    required this.accent,
    required this.trackColor,
    required this.percentage,
  });

  final String iconKey;
  final TopBarCapsuleIconKind kind;
  final Color accent;
  final Color trackColor;
  final double? percentage;

  @override
  State<_KineticCapsuleIcon> createState() => _KineticCapsuleIconState();
}

class _KineticCapsuleIconState extends State<_KineticCapsuleIcon>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  bool get _shouldLoop => switch (widget.kind) {
    TopBarCapsuleIconKind.compacting ||
    TopBarCapsuleIconKind.routingFiber ||
    TopBarCapsuleIconKind.executingCompute => true,
    _ => false,
  };

  bool get _shouldPlayOnce =>
      widget.kind == TopBarCapsuleIconKind.completedBurst ||
      widget.kind == TopBarCapsuleIconKind.linkAlert;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: _durationForKind(widget.kind),
    );
    _syncAnimation(resetIfOneShot: true);
  }

  @override
  void didUpdateWidget(covariant _KineticCapsuleIcon oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.kind != widget.kind || oldWidget.iconKey != widget.iconKey) {
      _controller.duration = _durationForKind(widget.kind);
      _syncAnimation(resetIfOneShot: true);
    }
  }

  Duration _durationForKind(TopBarCapsuleIconKind kind) => switch (kind) {
    TopBarCapsuleIconKind.compacting => const Duration(milliseconds: 900),
    TopBarCapsuleIconKind.completedBurst => const Duration(milliseconds: 700),
    TopBarCapsuleIconKind.routingFiber => const Duration(milliseconds: 1100),
    TopBarCapsuleIconKind.executingCompute => const Duration(milliseconds: 1800),
    TopBarCapsuleIconKind.linkAlert => const Duration(milliseconds: 520),
    TopBarCapsuleIconKind.usageRing => const Duration(milliseconds: 350),
  };

  void _syncAnimation({required bool resetIfOneShot}) {
    if (_shouldLoop) {
      _controller.repeat();
    } else if (_shouldPlayOnce) {
      if (resetIfOneShot) {
        _controller.forward(from: 0);
      }
    } else {
      _controller.stop();
      _controller.value = 1;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (widget.kind == TopBarCapsuleIconKind.usageRing) {
      final fraction = ((widget.percentage ?? 0) / 100).clamp(0.0, 1.0);
      return TweenAnimationBuilder<double>(
        tween: Tween<double>(end: fraction),
        duration: const Duration(milliseconds: 420),
        curve: Curves.easeOutCubic,
        builder: (context, value, _) {
          return CustomPaint(
            size: const Size.square(13),
            painter: _UsageRingIconPainter(
              fraction: widget.percentage == null ? null : value,
              accent: widget.accent,
              trackColor: widget.trackColor,
            ),
          );
        },
      );
    }

    return AnimatedBuilder(
      animation: _controller,
      builder: (context, _) {
        final t = _controller.value;
        return CustomPaint(
          size: const Size.square(14),
          painter: switch (widget.kind) {
            TopBarCapsuleIconKind.compacting => _CompactingIconPainter(
              t: t,
              accent: widget.accent,
            ),
            TopBarCapsuleIconKind.completedBurst => _CompletedBurstPainter(
              t: t,
              accent: widget.accent,
            ),
            TopBarCapsuleIconKind.routingFiber => _RoutingFiberIconPainter(
              t: t,
              accent: widget.accent,
            ),
            TopBarCapsuleIconKind.executingCompute =>
              _ExecutingComputePainter(t: t, accent: widget.accent),
            TopBarCapsuleIconKind.linkAlert => _LinkAlertPainter(
              t: t,
              accent: widget.accent,
            ),
            TopBarCapsuleIconKind.usageRing => _UsageRingIconPainter(
              fraction: 0,
              accent: widget.accent,
              trackColor: widget.trackColor,
            ),
          },
        );
      },
    );
  }
}

class _UsageRingIconPainter extends CustomPainter {
  const _UsageRingIconPainter({
    required this.fraction,
    required this.accent,
    required this.trackColor,
  });

  final double? fraction;
  final Color accent;
  final Color trackColor;

  @override
  void paint(Canvas canvas, Size size) {
    final center = size.center(Offset.zero);
    final radius = size.shortestSide / 2 - 1.6;
    final trackPaint = Paint()
      ..color = trackColor
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.1
      ..strokeCap = StrokeCap.round;
    canvas.drawCircle(center, radius, trackPaint);
    final activeFraction = fraction;
    if (activeFraction == null || activeFraction <= 0) return;
    final arcPaint = Paint()
      ..color = accent
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.1
      ..strokeCap = StrokeCap.round;
    canvas.drawArc(
      Rect.fromCircle(center: center, radius: radius),
      -math.pi / 2,
      math.pi * 2 * activeFraction.clamp(0.04, 1.0),
      false,
      arcPaint,
    );
  }

  @override
  bool shouldRepaint(covariant _UsageRingIconPainter oldDelegate) =>
      oldDelegate.fraction != fraction ||
      oldDelegate.accent != accent ||
      oldDelegate.trackColor != trackColor;
}

class _CompactingIconPainter extends CustomPainter {
  const _CompactingIconPainter({required this.t, required this.accent});

  final double t;
  final Color accent;

  @override
  void paint(Canvas canvas, Size size) {
    final squeeze = 0.5 - 0.5 * math.cos(t * math.pi * 2);
    final w = size.width;
    final h = size.height;
    final stroke = Paint()
      ..color = accent
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.7
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round;

    final topOffset = squeeze * 2.2;
    final botOffset = -squeeze * 2.2;
    canvas.drawLine(
      Offset(w * 0.22, h * 0.24 + topOffset),
      Offset(w * 0.78, h * 0.24 + topOffset),
      stroke,
    );
    final topChevron = Path()
      ..moveTo(w * 0.36, h * 0.08 + topOffset)
      ..lineTo(w * 0.50, h * 0.21 + topOffset)
      ..lineTo(w * 0.64, h * 0.08 + topOffset);
    canvas.drawPath(topChevron, stroke);

    final halfCore = w * (0.34 - squeeze * 0.14);
    canvas.drawLine(
      Offset(w * 0.5 - halfCore, h * 0.5),
      Offset(w * 0.5 + halfCore, h * 0.5),
      stroke,
    );

    canvas.drawLine(
      Offset(w * 0.22, h * 0.76 + botOffset),
      Offset(w * 0.78, h * 0.76 + botOffset),
      stroke,
    );
    final botChevron = Path()
      ..moveTo(w * 0.36, h * 0.92 + botOffset)
      ..lineTo(w * 0.50, h * 0.79 + botOffset)
      ..lineTo(w * 0.64, h * 0.92 + botOffset);
    canvas.drawPath(botChevron, stroke);
  }

  @override
  bool shouldRepaint(covariant _CompactingIconPainter oldDelegate) =>
      oldDelegate.t != t || oldDelegate.accent != accent;
}

class _CompletedBurstPainter extends CustomPainter {
  const _CompletedBurstPainter({required this.t, required this.accent});

  final double t;
  final Color accent;

  @override
  void paint(Canvas canvas, Size size) {
    if (size.isEmpty) return;
    final center = size.center(Offset.zero);
    final baseRadius = size.shortestSide * 0.38;

    // Expanding ripple ring
    final ringT = Curves.easeOut.transform(t);
    final ripplePaint = Paint()
      ..color = accent.withValues(alpha: (1 - ringT) * 0.75)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.4;
    canvas.drawCircle(center, baseRadius * (0.7 + ringT * 0.6), ripplePaint);

    // Subtle static ring
    canvas.drawCircle(
      center,
      baseRadius,
      Paint()
        ..color = accent.withValues(alpha: 0.28)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.4,
    );

    // Self-drawing check path
    final drawT = ((t - 0.1) / 0.55).clamp(0.0, 1.0);
    final check = Path()
      ..moveTo(size.width * 0.31, size.height * 0.52)
      ..lineTo(size.width * 0.45, size.height * 0.66)
      ..lineTo(size.width * 0.71, size.height * 0.37);
    final metrics = check.computeMetrics().toList(growable: false);
    if (metrics.isEmpty || metrics.first.length <= 0) return;
    final metric = metrics.first;
    final subPath = metric.extractPath(0, metric.length * drawT);
    canvas.drawPath(
      subPath,
      Paint()
        ..color = accent
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.85
        ..strokeCap = StrokeCap.round
        ..strokeJoin = StrokeJoin.round,
    );
  }

  @override
  bool shouldRepaint(covariant _CompletedBurstPainter oldDelegate) =>
      oldDelegate.t != t || oldDelegate.accent != accent;
}

class _RoutingFiberIconPainter extends CustomPainter {
  const _RoutingFiberIconPainter({required this.t, required this.accent});

  final double t;
  final Color accent;

  @override
  void paint(Canvas canvas, Size size) {
    if (size.isEmpty) return;
    final p0 = Offset(size.width * 0.16, size.height * 0.68);
    final p1 = Offset(size.width * 0.84, size.height * 0.32);

    final fiber = Path()
      ..moveTo(p0.dx, p0.dy)
      ..cubicTo(
        size.width * 0.38,
        size.height * 0.16,
        size.width * 0.62,
        size.height * 0.84,
        p1.dx,
        p1.dy,
      );
    final secondary = Path()
      ..moveTo(p0.dx, p0.dy)
      ..cubicTo(
        size.width * 0.42,
        size.height * 0.84,
        size.width * 0.58,
        size.height * 0.16,
        p1.dx,
        p1.dy,
      );

    canvas.drawPath(
      fiber,
      Paint()
        ..color = accent.withValues(alpha: 0.25)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2.6
        ..strokeCap = StrokeCap.round,
    );
    canvas.drawPath(
      secondary,
      Paint()
        ..color = accent.withValues(alpha: 0.22)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.4
        ..strokeCap = StrokeCap.round,
    );

    final metrics = fiber.computeMetrics().toList(growable: false);
    if (metrics.isEmpty || metrics.first.length <= 0) return;
    final metric = metrics.first;
    final head = metric.length * t;
    final tail = math.max(0.0, head - metric.length * 0.36);
    final beam = metric.extractPath(tail, head);
    canvas.drawPath(
      beam,
      Paint()
        ..color = accent
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.7
        ..strokeCap = StrokeCap.round,
    );

    final tangent = metric.getTangentForOffset(head);
    if (tangent != null) {
      canvas.drawCircle(
        tangent.position,
        2.1,
        Paint()..color = accent,
      );
    }

    canvas.drawCircle(p0, 1.8, Paint()..color = accent);
    canvas.drawCircle(
      p1,
      2.0,
      Paint()
        ..color = accent
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.5,
    );
    canvas.drawCircle(
      p1,
      1.8 + t * 3.4,
      Paint()
        ..color = accent.withValues(alpha: (1 - t) * 0.7)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.1,
    );
  }

  @override
  bool shouldRepaint(covariant _RoutingFiberIconPainter oldDelegate) =>
      oldDelegate.t != t || oldDelegate.accent != accent;
}

class _ExecutingComputePainter extends CustomPainter {
  const _ExecutingComputePainter({required this.t, required this.accent});

  final double t;
  final Color accent;

  @override
  void paint(Canvas canvas, Size size) {
    final center = size.center(Offset.zero);
    final outerR = size.shortestSide * 0.40;
    final innerR = size.shortestSide * 0.24;
    final angle = t * math.pi * 2;

    final outerPaint = Paint()
      ..color = accent
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.6
      ..strokeCap = StrokeCap.round;
    canvas.drawArc(
      Rect.fromCircle(center: center, radius: outerR),
      angle,
      math.pi * 0.75,
      false,
      outerPaint,
    );
    canvas.drawArc(
      Rect.fromCircle(center: center, radius: outerR),
      angle + math.pi,
      math.pi * 0.75,
      false,
      outerPaint,
    );

    final innerPaint = Paint()
      ..color = accent.withValues(alpha: 0.7)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.5
      ..strokeCap = StrokeCap.round;
    canvas.drawArc(
      Rect.fromCircle(center: center, radius: innerR),
      -angle * 1.4,
      math.pi * 1.1,
      false,
      innerPaint,
    );
    canvas.drawCircle(center, 1.8, Paint()..color = accent);
  }

  @override
  bool shouldRepaint(covariant _ExecutingComputePainter oldDelegate) =>
      oldDelegate.t != t || oldDelegate.accent != accent;
}

class _LinkAlertPainter extends CustomPainter {
  const _LinkAlertPainter({required this.t, required this.accent});

  final double t;
  final Color accent;

  @override
  void paint(Canvas canvas, Size size) {
    final jitter = math.sin(t * math.pi * 6) * (1 - t) * 1.2;
    canvas.save();
    canvas.translate(jitter, 0);
    final stroke = Paint()
      ..color = accent
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.7
      ..strokeCap = StrokeCap.round;
    final left = Path()
      ..moveTo(size.width * 0.14, size.height * 0.62)
      ..quadraticBezierTo(
        size.width * 0.30,
        size.height * 0.44,
        size.width * 0.44,
        size.height * 0.56,
      );
    final right = Path()
      ..moveTo(size.width * 0.58, size.height * 0.42)
      ..quadraticBezierTo(
        size.width * 0.72,
        size.height * 0.54,
        size.width * 0.86,
        size.height * 0.36,
      );
    canvas.drawPath(left, stroke);
    canvas.drawPath(right, stroke);
    canvas.drawLine(
      Offset(size.width * 0.44, size.height * 0.36),
      Offset(size.width * 0.58, size.height * 0.64),
      stroke,
    );
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant _LinkAlertPainter oldDelegate) =>
      oldDelegate.t != t || oldDelegate.accent != accent;
}
