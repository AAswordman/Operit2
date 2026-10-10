// ignore_for_file: file_names
part of '../ToolPkgUiLauncherScreen.dart';

/// Native hover state stays in Flutter; moving the mouse never calls the JS runtime.
class _ComposeHoverScope extends InheritedWidget {
  const _ComposeHoverScope({
    required this.hovered,
    required this.onMenuOpenChanged,
    required super.child,
  });
  final bool hovered;
  final ValueChanged<bool> onMenuOpenChanged;
  static void menuOpenChanged(BuildContext context, bool open) => context
      .dependOnInheritedWidgetOfExactType<_ComposeHoverScope>()
      ?.onMenuOpenChanged(open);
  static bool of(BuildContext context) =>
      context
          .dependOnInheritedWidgetOfExactType<_ComposeHoverScope>()
          ?.hovered ??
      false;
  @override
  bool updateShouldNotify(_ComposeHoverScope oldWidget) =>
      hovered != oldWidget.hovered;
}

class _ComposeHoverRegion extends StatefulWidget {
  const _ComposeHoverRegion({
    required this.child,
    required this.color,
    required this.radius,
  });
  final Widget child;
  final Color? color;
  final BorderRadius radius;
  @override
  State<_ComposeHoverRegion> createState() => _ComposeHoverRegionState();
}

class _ComposeHoverRegionState extends State<_ComposeHoverRegion> {
  bool _hovered = false;
  bool _menuOpen = false;
  @override
  Widget build(BuildContext context) => MouseRegion(
    onEnter: (_) => setState(() => _hovered = true),
    onExit: (_) => setState(() => _hovered = false),
    child: _ComposeHoverScope(
      hovered: _hovered || _menuOpen,
      onMenuOpenChanged: (open) {
        if (mounted) setState(() => _menuOpen = open);
      },
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 140),
        decoration: BoxDecoration(
          color: _hovered ? widget.color : Colors.transparent,
          borderRadius: widget.radius,
        ),
        child: widget.child,
      ),
    ),
  );
}

class _ComposeHoverOnly extends StatelessWidget {
  const _ComposeHoverOnly({required this.child, required this.alwaysVisible});
  final Widget child;
  final bool alwaysVisible;
  @override
  Widget build(BuildContext context) {
    final platform = Theme.of(context).platform;
    final visible =
        alwaysVisible ||
        _ComposeHoverScope.of(context) ||
        platform == TargetPlatform.android ||
        platform == TargetPlatform.iOS;
    return IgnorePointer(
      ignoring: !visible,
      child: ExcludeSemantics(
        excluding: !visible,
        child: AnimatedOpacity(
          duration: const Duration(milliseconds: 140),
          opacity: visible ? 1 : 0,
          child: child,
        ),
      ),
    );
  }
}

/// Reproduces the native six-dot handle; its animation does not schedule JS frames.
class _ComposeActivityDots extends StatefulWidget {
  const _ComposeActivityDots({required this.props});
  final Map<String, Object?> props;
  @override
  State<_ComposeActivityDots> createState() => _ComposeActivityDotsState();
}

class _ComposeActivityDotsState extends State<_ComposeActivityDots>
    with SingleTickerProviderStateMixin {
  late final AnimationController _animation = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 1500),
  );
  @override
  void initState() {
    super.initState();
    if (_bool(widget.props['running'])) _animation.repeat();
  }

  @override
  void didUpdateWidget(_ComposeActivityDots oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_bool(widget.props['running'])) {
      if (!_animation.isAnimating) _animation.repeat();
    } else {
      _animation.stop();
    }
  }

  @override
  void dispose() {
    _animation.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final running = _bool(widget.props['running']);
    final hovered = _ComposeHoverScope.of(context);
    final resting =
        _color(context, widget.props['restingColor']) ??
        Theme.of(context).colorScheme.onSurfaceVariant;
    final active =
        _color(context, widget.props['activeColor']) ??
        Theme.of(context).colorScheme.primary;
    return AnimatedOpacity(
      duration: const Duration(milliseconds: 140),
      opacity: running
          ? 1
          : hovered
          ? 0.72
          : _bool(widget.props['selected'])
          ? 0.36
          : 0,
      child: SizedBox(
        width: 10,
        height: 13.5,
        child: AnimatedBuilder(
          animation: _animation,
          builder: (_, _) => CustomPaint(
            painter: running
                ? _ComposeMarqueeDotsPainter(
                    progress: _animation.value,
                    activeColor: active,
                    dimColor: resting.withValues(alpha: 0.18),
                  )
                : _ComposeStaticDotsPainter(color: resting),
          ),
        ),
      ),
    );
  }
}

class _ComposeMarqueeDotsPainter extends CustomPainter {
  const _ComposeMarqueeDotsPainter({
    required this.progress,
    required this.activeColor,
    required this.dimColor,
  });

  final double progress;
  final Color activeColor;
  final Color dimColor;

  // 6 dots in 2 columns, 3 rows:
  // Counter-clockwise sequence:
  // 0: Top-Left -> 1: Mid-Left -> 2: Bottom-Left -> 3: Bottom-Right -> 4: Mid-Right -> 5: Top-Right
  static const List<Offset> _positions = <Offset>[
    Offset(0.25, 0.18), // 0: Top-Left
    Offset(0.25, 0.50), // 1: Mid-Left
    Offset(0.25, 0.82), // 2: Bottom-Left
    Offset(0.75, 0.82), // 3: Bottom-Right
    Offset(0.75, 0.50), // 4: Mid-Right
    Offset(0.75, 0.18), // 5: Top-Right
  ];

  @override
  void paint(Canvas canvas, Size size) {
    final activePos = progress * 6.0;
    final baseDotRadius = size.width * 0.115;

    for (var i = 0; i < 6; i++) {
      var dist = (activePos - i) % 6.0;
      if (dist < 0) {
        dist += 6.0;
      }
      final intensity = dist < 3.2 ? (1.0 - (dist / 3.2)) : 0.0;
      final color = intensity > 0
          ? Color.lerp(dimColor, activeColor, intensity)!
          : dimColor;
      final radius = baseDotRadius * (1.0 + intensity * 0.25);
      final center = Offset(
        size.width * _positions[i].dx,
        size.height * _positions[i].dy,
      );

      if (intensity > 0.82) {
        final glowPaint = Paint()
          ..color = activeColor.withValues(alpha: 0.35 * intensity)
          ..maskFilter = const MaskFilter.blur(BlurStyle.normal, 1.4);
        canvas.drawCircle(center, radius * 1.35, glowPaint);
      }

      final dotPaint = Paint()
        ..color = color
        ..style = PaintingStyle.fill;
      canvas.drawCircle(center, radius, dotPaint);
    }
  }

  @override
  bool shouldRepaint(covariant _ComposeMarqueeDotsPainter oldDelegate) {
    return oldDelegate.progress != progress ||
        oldDelegate.activeColor != activeColor ||
        oldDelegate.dimColor != dimColor;
  }
}

class _ComposeStaticDotsPainter extends CustomPainter {
  const _ComposeStaticDotsPainter({required this.color});

  final Color color;

  static const List<Offset> _positions = <Offset>[
    Offset(0.22, 0.16), // Top-Left
    Offset(0.22, 0.50), // Mid-Left
    Offset(0.22, 0.84), // Bottom-Left
    Offset(0.78, 0.84), // Bottom-Right
    Offset(0.78, 0.50), // Mid-Right
    Offset(0.78, 0.16), // Top-Right
  ];

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..style = PaintingStyle.fill;
    final dotRadius = size.width * 0.11;

    for (final pos in _positions) {
      canvas.drawCircle(
        Offset(size.width * pos.dx, size.height * pos.dy),
        dotRadius,
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(covariant _ComposeStaticDotsPainter oldDelegate) {
    return oldDelegate.color != color;
  }
}
