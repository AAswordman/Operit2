// ignore_for_file: file_names

part of 'DeviceSpaceGraph.dart';

/// Shared measurements keep circular hit targets and edge ports aligned.
class _GraphNodeMetrics {
  const _GraphNodeMetrics({
    required this.compact,
    required this.scale,
    required this.extent,
    required this.currentDiameter,
    required this.remoteDiameter,
  });

  factory _GraphNodeMetrics.of(Size viewport, {required bool compact}) {
    final shortest = math.min(viewport.width, viewport.height);
    final currentDiameter = shortest * 0.205;
    return _GraphNodeMetrics(
      compact: compact,
      scale: currentDiameter / 82,
      extent: currentDiameter * 1.22,
      currentDiameter: currentDiameter,
      remoteDiameter: currentDiameter * 0.71,
    );
  }

  final bool compact;
  final double scale;
  final double extent;
  final double currentDiameter;
  final double remoteDiameter;

  double get currentIcon => currentDiameter * 0.415;
  double get remoteIcon => remoteDiameter * 0.431;
  double get margin => (compact ? 24.0 : 32.0) * scale;
  double stroke(double width) => width * scale;
}

class _GraphNodeFrame implements DeviceSpherePose {
  const _GraphNodeFrame({
    required this.center,
    required this.depth,
    required this.lateral,
    required this.scale,
    required this.opacity,
  });

  final Offset center;
  @override
  final double depth;
  @override
  final double lateral;
  final double scale;
  final double opacity;
}

/// Both modes share bounds for the entire orbit, so rotation never resizes it.
class _GraphLayout {
  const _GraphLayout({
    required this.size,
    required this.devices,
    required this.center,
    required this.topology,
    required this.orbitRadius,
    required this.metrics,
  });

  static const orbitTilt = -0.16;
  static const perspective = 0.14;

  final Size size;
  final List<generated.RuntimeDeviceSpaceDevice> devices;
  final Offset center;
  final Map<String, Offset> topology;
  final Offset orbitRadius;
  final _GraphNodeMetrics metrics;

  int get ringCount => math.max(1, ((devices.length - 1) / 8).ceil());
  double ringScale(int ring) => 1 + ring * 0.46;

  factory _GraphLayout.create(
    Size viewport,
    generated.RuntimeDeviceSpaceTopology graph, {
    required _GraphNodeMetrics metrics,
  }) {
    final current = graph.devices.singleWhere(
      (device) => device.deviceId == graph.currentDeviceId,
    );
    final remotes =
        graph.devices
            .where((device) => device.deviceId != graph.currentDeviceId)
            .toList()
          ..sort((a, b) => a.deviceId.compareTo(b.deviceId));
    final devices = <generated.RuntimeDeviceSpaceDevice>[current, ...remotes];
    final padding = metrics.extent * 1.16 + metrics.margin;
    final radius = Offset(
      math.max(metrics.extent * 1.2, (viewport.width - padding) / 2.2),
      math.max(
        metrics.currentDiameter * 0.66,
        (viewport.height - padding) / 2.85,
      ),
    );

    // Layers represent recorded graph distance, including offline connections.
    final neighbors = <String, Set<String>>{
      for (final device in devices) device.deviceId: <String>{},
    };
    for (final edge in graph.connections) {
      neighbors[edge.firstDeviceId]!.add(edge.secondDeviceId);
      neighbors[edge.secondDeviceId]!.add(edge.firstDeviceId);
    }
    final depths = <String, int>{current.deviceId: 0};
    final queue = <String>[current.deviceId];
    for (var index = 0; index < queue.length; index++) {
      final id = queue[index];
      for (final neighbor in neighbors[id]!) {
        if (depths.containsKey(neighbor)) continue;
        depths[neighbor] = depths[id]! + 1;
        queue.add(neighbor);
      }
    }
    final disconnectedDepth = depths.values.reduce(math.max) + 1;
    final layers = <int, List<String>>{};
    for (final device in devices) {
      final depth = depths.containsKey(device.deviceId)
          ? depths[device.deviceId]!
          : disconnectedDepth;
      layers.putIfAbsent(depth, () => <String>[]).add(device.deviceId);
    }
    final topologyPoints = <String, Offset>{};
    final layerStep =
        metrics.extent + (metrics.compact ? 30 : 80) * metrics.scale;
    final crossStep =
        metrics.extent + (metrics.compact ? 16 : 30) * metrics.scale;
    for (final layer in layers.entries) {
      for (var index = 0; index < layer.value.length; index++) {
        final cross = index - (layer.value.length - 1) / 2;
        topologyPoints[layer.value[index]] = metrics.compact
            ? Offset(cross * crossStep, layer.key * layerStep)
            : Offset(layer.key * layerStep, cross * crossStep);
      }
    }
    final topologyBounds = _bounds(topologyPoints.values);
    final ringCount = math.max(1, (remotes.length / 8).ceil());
    final outerScale = 1 + (ringCount - 1) * 0.46;
    final cosTilt = math.cos(orbitTilt);
    final sinTilt = math.sin(orbitTilt);
    // Include perspective, node scale, hover feedback and all orbital angles.
    final orbitHalfWidth =
        math.sqrt(
          math.pow(radius.dx * cosTilt, 2) + math.pow(radius.dy * sinTilt, 2),
        ) *
        (1 + perspective) *
        outerScale;
    final orbitHalfHeight =
        math.sqrt(
          math.pow(radius.dx * sinTilt, 2) + math.pow(radius.dy * cosTilt, 2),
        ) *
        (1 + perspective) *
        outerScale;
    final width = math.max(
      viewport.width,
      math.max(orbitHalfWidth * 2, topologyBounds.width) + padding,
    );
    final height = math.max(
      viewport.height,
      math.max(orbitHalfHeight * 2, topologyBounds.height) + padding,
    );
    final center = Offset(width / 2, height / 2);
    return _GraphLayout(
      size: Size(width, height),
      devices: devices,
      center: center,
      topology: {
        for (final point in topologyPoints.entries)
          point.key: point.value - topologyBounds.center + center,
      },
      orbitRadius: radius,
      metrics: metrics,
    );
  }

  /// Projects an orbit tilted towards the viewer, with a shared vanishing point.
  Offset orbitPoint(double angle, {double ring = 1}) {
    final depth = math.sin(angle);
    final perspectiveScale = 1 + depth * perspective;
    final x = math.cos(angle) * orbitRadius.dx * ring * perspectiveScale;
    final y = depth * orbitRadius.dy * ring * perspectiveScale;
    return center +
        Offset(
          x * math.cos(orbitTilt) - y * math.sin(orbitTilt),
          x * math.sin(orbitTilt) + y * math.cos(orbitTilt),
        );
  }

  Map<String, _GraphNodeFrame> frames(double progress, double turns) => {
    for (var index = 0; index < devices.length; index++)
      devices[index].deviceId: frameAt(index, progress, turns),
  };

  _GraphNodeFrame frameAt(int index, double progress, double turns) {
    final device = devices[index];
    final current = index == 0;
    final remoteIndex = index - 1;
    final ring = current ? 0 : remoteIndex ~/ 8;
    final count = math.min(8, devices.length - 1 - ring * 8);
    final angle = current
        ? 0.0
        : -math.pi / 4 +
              remoteIndex % 8 * math.pi * 2 / count +
              turns * math.pi * 2 +
              ring * 0.37;
    final depth = current ? 0.0 : math.sin(angle);
    final orbit = current ? center : orbitPoint(angle, ring: ringScale(ring));
    final scale = current ? 1.0 : 0.9 + 0.17 * depth;
    final opacity = current ? 1.0 : 0.76 + 0.24 * (depth + 1) / 2;
    return _GraphNodeFrame(
      center: Offset.lerp(orbit, topology[device.deviceId], progress)!,
      depth: depth * (1 - progress),
      lateral: current ? 0 : math.cos(angle) * (1 - progress),
      scale: scale + (1 - scale) * progress,
      opacity: opacity + (1 - opacity) * progress,
    );
  }

  static Rect _bounds(Iterable<Offset> points) {
    var left = double.infinity;
    var top = double.infinity;
    var right = double.negativeInfinity;
    var bottom = double.negativeInfinity;
    for (final point in points) {
      left = math.min(left, point.dx);
      top = math.min(top, point.dy);
      right = math.max(right, point.dx);
      bottom = math.max(bottom, point.dy);
    }
    return Rect.fromLTRB(left, top, right, bottom);
  }
}

/// Light and dark keep the same scene, with separate contrast for paper vs night.
class _GraphPalette extends DeviceSpherePalette {
  const _GraphPalette(super.scheme);

  Color get cardBorder => accent.withValues(alpha: dark ? 0.16 : 0.22);
  Color get cardStart => Color.alphaBlend(
    accent.withValues(alpha: dark ? 0.04 : 0.10),
    dark ? scheme.surfaceContainerLow : scheme.surfaceContainerLowest,
  );
  Color get cardEnd => dark
      ? scheme.surface
      : Color.alphaBlend(
          scheme.primary.withValues(alpha: 0.02),
          scheme.surface,
        );

  Color get detailsFill => dark
      ? scheme.surfaceContainerHigh
      : Color.alphaBlend(accent.withValues(alpha: 0.07), scheme.surface);
  Color get detailsBorder => accent.withValues(alpha: dark ? 0.20 : 0.26);
  Color get divider =>
      scheme.outlineVariant.withValues(alpha: dark ? 0.25 : 0.55);

  Color _paint(Color color, double alpha) =>
      color.withValues(alpha: alpha.clamp(0.0, 1.0));

  Color glow(double arrival, double breath) => _paint(
    accent,
    ((dark ? 0.13 : 0.22) + breath * (dark ? 0.045 : 0.08)) * arrival,
  );

  Color atmosphere(double twinkle, double arrival) => _paint(
    Color.lerp(ink, accent, dark ? 0.72 : 0.38)!,
    ((dark ? 0.07 : 0.11) + twinkle * (dark ? 0.15 : 0.20)) * arrival,
  );

  Color orbit(double strength, double opacity, double arrival) => _paint(
    Color.lerp(accent, ink, dark ? 0.0 : 0.32)!,
    strength * (dark ? 1 : 1.7) * opacity * arrival,
  );

  Color halo(double fade, double arrival) =>
      _paint(accent, fade * (dark ? 0.19 : 0.30) * arrival);

  Color sweep(double arrival) => _paint(accent, (dark ? 0.55 : 0.78) * arrival);

  Color grid(double mode, double arrival) =>
      _paint(accent, (dark ? 0.09 : 0.16) * mode * arrival);

  Color membership(Color color, double opacity) =>
      _paint(color, opacity * (dark ? 1 : 1.45));

  Color connection(Color color, double opacity) =>
      _paint(color, opacity * (dark ? 1 : 1.15));
}
