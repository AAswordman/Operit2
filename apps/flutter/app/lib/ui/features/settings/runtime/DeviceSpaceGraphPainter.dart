// ignore_for_file: file_names

part of 'DeviceSpaceGraph.dart';

class _GraphPainter extends CustomPainter {
  _GraphPainter({
    required this.layout,
    required this.frames,
    required this.topology,
    required this.scheme,
    required this.mode,
    required this.arrival,
    required this.phase,
    required this.selectedId,
    required this.reduceMotion,
  }) : super(repaint: phase);

  final _GraphLayout layout;
  final Map<String, _GraphNodeFrame> frames;
  final generated.RuntimeDeviceSpaceTopology topology;
  final ColorScheme scheme;
  final double mode;
  final double arrival;
  final Animation<double> phase;
  final String? selectedId;
  final bool reduceMotion;

  double get clock => reduceMotion ? 0.25 : phase.value;
  _GraphNodeMetrics get metrics => layout.metrics;
  _GraphPalette get palette => _GraphPalette(scheme);
  double get paintScale => metrics.scale;
  double stroke(double width) => metrics.stroke(width);

  @override
  void paint(Canvas canvas, Size size) {
    final center = frames[topology.currentDeviceId]!.center;
    final breath = (math.sin(clock * math.pi * 4) + 1) / 2;
    final glowRect = Rect.fromCircle(
      center: center,
      radius: metrics.currentDiameter * (1.95 + breath * 0.1),
    );
    canvas.drawOval(
      glowRect,
      Paint()
        ..shader = RadialGradient(
          colors: <Color>[
            palette.glow(arrival, breath),
            scheme.primary.withValues(alpha: 0),
          ],
        ).createShader(glowRect),
    );

    _drawAtmosphere(canvas, size);
    if (mode < 1) _drawOrbits(canvas, 1 - mode);
    if (mode > 0) _drawGrid(canvas, size);

    // Membership guides fade away before the recorded topology takes focus.
    final membershipOpacity = (1 - mode) * arrival;
    if (membershipOpacity > 0) {
      for (var index = 0; index < layout.devices.length; index++) {
        final device = layout.devices[index];
        if (device.deviceId == topology.currentDeviceId) continue;
        final frame = frames[device.deviceId]!;
        final end = frame.center;
        final delta = end - center;
        final bend = Offset(-delta.dy, delta.dx) * 0.13;
        final path = Path()
          ..moveTo(center.dx, center.dy)
          ..quadraticBezierTo(
            (center.dx + end.dx) / 2 + bend.dx,
            (center.dy + end.dy) / 2 + bend.dy,
            end.dx,
            end.dy,
          );
        final color = deviceSpaceVersionMismatch(topology, device)
            ? deviceCompatibilityColor(scheme)
            : device.online
            ? scheme.primary
            : scheme.outline;
        canvas.drawPath(
          path,
          Paint()
            ..color = palette.membership(
              color,
              0.2 * membershipOpacity * frame.opacity,
            )
            ..style = PaintingStyle.stroke
            ..strokeWidth = stroke(1),
        );
        if (device.deviceId == selectedId) {
          canvas.drawPath(
            path,
            Paint()
              ..color = palette.membership(color, 0.55 * membershipOpacity)
              ..style = PaintingStyle.stroke
              ..strokeWidth = stroke(1.4),
          );
        }
      }
    }

    if (mode > 0) {
      for (var index = 0; index < topology.connections.length; index++) {
        final edge = topology.connections[index];
        _drawConnection(canvas, edge, index);
      }
    }

    // A slow halo remains visible even in a single-device or offline space.
    final halo = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = stroke(1);
    for (var index = 0; index < 2; index++) {
      final progress = (clock * 2 + index * 0.5) % 1;
      halo.color = palette.halo(1 - progress, arrival);
      canvas.drawCircle(
        center,
        metrics.currentDiameter * (0.63 + progress * 0.37),
        halo,
      );
    }
    final ring = Rect.fromCircle(
      center: center,
      radius: metrics.currentDiameter * 0.62,
    );
    canvas.drawArc(
      ring,
      -math.pi / 2 + clock * math.pi * 2,
      math.pi * 0.75,
      false,
      Paint()
        ..color = palette.sweep(arrival)
        ..style = PaintingStyle.stroke
        ..strokeWidth = stroke(1.6)
        ..strokeCap = StrokeCap.round,
    );
  }

  void _drawAtmosphere(Canvas canvas, Size size) {
    for (var index = 0; index < 34; index++) {
      final position = Offset(
        ((index * 0.61803398875 + 0.13) % 1) * size.width,
        ((index * 0.38196601125 + index * index * 0.017) % 1) * size.height,
      );
      final twinkle = (math.sin(clock * math.pi * 2 + index * 1.7) + 1) / 2;
      canvas.drawCircle(
        position,
        stroke(index % 5 == 0 ? 1.3 : 0.75),
        Paint()
          ..color = palette.atmosphere(twinkle, arrival),
      );
    }
  }

  void _drawOrbits(Canvas canvas, double opacity) {
    final rings = <double>[
      0.68,
      for (var ring = 0; ring < layout.ringCount; ring++)
        layout.ringScale(ring),
    ];
    for (var index = 0; index < rings.length; index++) {
      final scale = rings[index];
      // Draw the far half lighter than the near half, on the same projection.
      for (var half = 0; half < 2; half++) {
        final path = Path();
        for (var step = 0; step <= 48; step++) {
          final point = layout.orbitPoint(
            half * math.pi + step / 48 * math.pi,
            ring: scale,
          );
          if (step == 0) {
            path.moveTo(point.dx, point.dy);
          } else {
            path.lineTo(point.dx, point.dy);
          }
        }
        canvas.drawPath(
          path,
          Paint()
            ..color = palette.orbit(
              (half == 0 ? 0.24 : 0.08) * (index == 0 ? 0.5 : 1),
              opacity,
              arrival,
            )
            ..style = PaintingStyle.stroke
            ..strokeWidth = stroke(half == 0 ? 1.1 : 0.7),
        );
      }
      final angle = clock * math.pi * 2 + index * 2.1;
      for (var step = 0; step < 12; step++) {
        final first = layout.orbitPoint(angle + step * 0.028, ring: scale);
        final second = layout.orbitPoint(
          angle + (step + 1) * 0.028,
          ring: scale,
        );
        canvas.drawLine(
          first,
          second,
          Paint()
            ..color = palette.orbit(step / 12 * 0.5, opacity, arrival)
            ..strokeWidth = stroke(1.6)
            ..strokeCap = StrokeCap.round,
        );
      }
      _drawSpark(
        canvas,
        layout.orbitPoint(angle + 12 * 0.028, ring: scale),
        scheme.primary,
        opacity * arrival * 0.7,
      );
    }
  }

  void _drawGrid(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = palette.grid(mode, arrival);
    final origin = 16 * paintScale;
    final step = 28 * paintScale;
    for (var x = origin; x < size.width; x += step) {
      for (var y = origin; y < size.height; y += step) {
        canvas.drawCircle(Offset(x, y), stroke(0.8), paint);
      }
    }
  }

  void _drawConnection(
    Canvas canvas,
    generated.RuntimeDeviceSpaceConnection edge,
    int index,
  ) {
    final firstFrame = frames[edge.firstDeviceId]!;
    final secondFrame = frames[edge.secondDeviceId]!;
    final first = firstFrame.center;
    final second = secondFrame.center;
    final delta = second - first;
    if (delta.distance < 1) return;
    final unit = delta / delta.distance;
    final start = _port(
      first,
      unit,
      edge.firstDeviceId == topology.currentDeviceId,
      firstFrame.scale,
    );
    final end = _port(
      second,
      -unit,
      edge.secondDeviceId == topology.currentDeviceId,
      secondFrame.scale,
    );
    final path = Path()..moveTo(start.dx, start.dy);
    if (delta.dx.abs() > delta.dy.abs()) {
      final midX = (start.dx + end.dx) / 2;
      path.cubicTo(midX, start.dy, midX, end.dy, end.dx, end.dy);
    } else {
      final midY = (start.dy + end.dy) / 2;
      path.cubicTo(start.dx, midY, end.dx, midY, end.dx, end.dy);
    }
    final online =
        edge.status == generated.RuntimeDeviceSpaceConnectionStatus.online;
    final relevant =
        selectedId == null ||
        selectedId == topology.currentDeviceId ||
        edge.firstDeviceId == selectedId ||
        edge.secondDeviceId == selectedId;
    final opacity = mode * arrival * (relevant ? 1 : 0.2);
    final color = _connectionColor(edge.status, scheme);
    final paint = Paint()
      ..color = palette.connection(color, (online ? 0.7 : 0.5) * opacity)
      ..style = PaintingStyle.stroke
      ..strokeWidth = stroke(online ? 1.5 : 1.1)
      ..strokeCap = StrokeCap.round;
    if (online) {
      canvas.drawPath(
        path,
        Paint()
          ..color = palette.connection(color, 0.07 * opacity)
          ..style = PaintingStyle.stroke
          ..strokeWidth = stroke(7),
      );
      canvas.drawPath(path, paint);
    }
    for (final metric in path.computeMetrics()) {
      if (!online) {
        final dash = 11 * paintScale;
        final dashSpan = 4 * paintScale;
        for (var distance = 0.0; distance < metric.length; distance += dash) {
          canvas.drawPath(
            metric.extractPath(
              distance,
              math.min(distance + dashSpan, metric.length),
            ),
            paint,
          );
        }
      } else {
        for (var particle = 0; particle < 2; particle++) {
          final progress = (clock * 3 + index * 0.19 + particle * 0.5) % 1;
          final tangent = metric.getTangentForOffset(metric.length * progress);
          if (tangent == null) continue;
          final trailStart = math.max(
            0.0,
            metric.length * progress - 22 * paintScale,
          );
          canvas.drawPath(
            metric.extractPath(trailStart, metric.length * progress),
            Paint()
              ..color = palette.connection(color, 0.6 * opacity)
              ..style = PaintingStyle.stroke
              ..strokeWidth = stroke(2.2)
              ..strokeCap = StrokeCap.round,
          );
          _drawSpark(canvas, tangent.position, color, opacity);
        }
      }
    }
    for (final point in <Offset>[start, end]) {
      canvas.drawCircle(
        point,
        stroke(3),
        Paint()..color = palette.connection(color, 0.7 * opacity),
      );
      canvas.drawCircle(
        point,
        stroke(1.25),
        Paint()..color = scheme.surface.withValues(alpha: opacity),
      );
    }
  }

  void _drawSpark(Canvas canvas, Offset point, Color color, double opacity) {
    canvas.drawCircle(
      point,
      stroke(5),
      Paint()..color = palette.connection(color, 0.08 * opacity),
    );
    canvas.drawCircle(
      point,
      stroke(2.5),
      Paint()..color = palette.connection(color, 0.22 * opacity),
    );
    canvas.drawCircle(
      point,
      stroke(1.35),
      Paint()..color = palette.connection(color, 0.95 * opacity),
    );
  }

  Offset _port(Offset center, Offset unit, bool current, double scale) {
    final distance =
        (current ? metrics.currentDiameter : metrics.remoteDiameter) *
            scale /
            2 +
        6 * paintScale;
    return center + unit * distance;
  }

  @override
  bool shouldRepaint(covariant _GraphPainter oldDelegate) =>
      oldDelegate.layout != layout ||
      oldDelegate.frames != frames ||
      oldDelegate.mode != mode ||
      oldDelegate.arrival != arrival ||
      oldDelegate.topology != topology ||
      oldDelegate.scheme != scheme ||
      oldDelegate.selectedId != selectedId ||
      oldDelegate.reduceMotion != reduceMotion;
}
