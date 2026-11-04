// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

/// Icon for a device platform reported by the device space.
IconData devicePlatformIcon(String platform) {
  final name = platform.trim().toLowerCase();
  if (name.contains('android')) return Icons.android_rounded;
  if (name.contains('windows')) return Icons.laptop_windows_rounded;
  if (name.contains('mac') || name.contains('darwin')) {
    return Icons.laptop_mac_rounded;
  }
  if (name.contains('linux')) return Icons.dns_outlined;
  if (name.contains('ios')) return Icons.phone_iphone_rounded;
  return Icons.devices_other_rounded;
}

/// Where a sphere sits on its orbit; light and glyph tilt follow it.
abstract interface class DeviceSpherePose {
  /// -1 far side to 1 near side.
  double get depth;

  /// -1 left to 1 right of the orbit center.
  double get lateral;
}

/// The pose of a sphere that is not orbiting.
class DeviceSphereRest implements DeviceSpherePose {
  const DeviceSphereRest();

  @override
  double get depth => 0;

  @override
  double get lateral => 0;
}

/// Sphere and glyph tones; light and dark keep the same scene with separate
/// contrast for paper and night.
class DeviceSpherePalette {
  const DeviceSpherePalette(this.scheme);

  final ColorScheme scheme;

  bool get dark => scheme.brightness == Brightness.dark;
  Color get accent => scheme.primary;
  Color get ink => scheme.onSurface;

  Color sphereBase({required bool current, required bool online}) {
    final wash = current
        ? (dark ? 0.38 : 0.22)
        : online
        ? (dark ? 0.20 : 0.13)
        : (dark ? 0.12 : 0.14);
    final ground = current
        ? scheme.primaryContainer
        : dark
        ? scheme.surfaceContainerHigh
        : scheme.surfaceContainerHighest;
    return Color.alphaBlend(accent.withValues(alpha: wash), ground);
  }

  Color sphereLit(Color base, {required bool current}) => Color.lerp(
    base,
    Colors.white,
    current ? (dark ? 0.56 : 0.20) : (dark ? 0.40 : 0.10),
  )!;

  Color sphereShade(Color base) =>
      Color.lerp(base, Colors.black, dark ? 0.72 : 0.52)!;

  double get contactShadow => dark ? 0.45 : 0.22;
  double get aura => dark ? 1 : 1.45;
  double get glint => dark ? 0.62 : 0.22;
  double get rimWhite => dark ? 0.68 : 0.12;
  double get rimArc => dark ? 0.32 : 0.20;
  double get glyphHighlight => dark ? 0.70 : 0.18;
  double get glyphShadow => dark ? 0.18 : 0.10;

  Color glyphCurrent() => dark
      ? Color.lerp(accent, Colors.white, 0.86)!
      : scheme.onPrimaryContainer;

  Color glyphRemote({required bool online}) => Color.lerp(
    ink,
    accent,
    online ? (dark ? 0.20 : 0.34) : (dark ? 0.08 : 0.18),
  )!;

  Color glyphEdge(Color accentColor) =>
      Color.lerp(accentColor, Colors.black, dark ? 0.62 : 0.48)!;
}

/// A lit sphere with a raised device glyph, the app's picture of a device.
///
/// The device space graph drives [pose] and [elevation] from its orbit; other
/// screens use [DeviceSphere.still].
class DeviceSphere extends StatelessWidget {
  const DeviceSphere({
    super.key,
    required this.diameter,
    required this.icon,
    required this.iconSize,
    required this.current,
    required this.online,
    required this.motion,
    required this.elevation,
    required this.pose,
  });

  /// A resting sphere; [lift] raises it like a hovered graph node.
  const DeviceSphere.still({
    super.key,
    required this.diameter,
    required this.icon,
    required this.current,
    required this.online,
    Animation<double> lift = kAlwaysDismissedAnimation,
  }) : iconSize = diameter * (current ? 0.42 : 0.44),
       motion = lift,
       elevation = lift,
       pose = _restPose;

  static DeviceSpherePose _restPose() => const DeviceSphereRest();

  final double diameter;
  final IconData icon;
  final double iconSize;
  final bool current;
  final bool online;
  final Listenable motion;
  final Animation<double> elevation;
  final ValueGetter<DeviceSpherePose> pose;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final palette = DeviceSpherePalette(scheme);
    final accent = current || online ? scheme.primary : scheme.outline;
    final glyphColor = current
        ? palette.glyphCurrent()
        : palette.glyphRemote(online: online);
    final animation = Listenable.merge(<Listenable>[motion, elevation]);
    return ExcludeSemantics(
      child: SizedBox.square(
        dimension: diameter,
        child: Stack(
          clipBehavior: Clip.none,
          children: <Widget>[
            Positioned.fill(
              child: CustomPaint(
                painter: _DeviceSpherePainter(
                  scheme: scheme,
                  current: current,
                  online: online,
                  elevation: elevation,
                  pose: pose,
                  animation: animation,
                ),
              ),
            ),
            Positioned.fill(
              child: AnimatedBuilder(
                animation: animation,
                child: DeviceRaisedGlyph(
                  icon: icon,
                  size: iconSize,
                  color: glyphColor,
                  accent: accent,
                  shadow: scheme.shadow,
                ),
                builder: (context, child) {
                  final pose = this.pose();
                  final lift = elevation.value;
                  final tilt = Matrix4.identity()
                    ..setEntry(3, 2, 0.0018)
                    ..rotateX(-0.18 + pose.depth * 0.07 - lift * 0.04)
                    ..rotateY(-0.18 + pose.lateral * 0.2)
                    ..rotateZ(-0.035 + pose.lateral * 0.025);
                  return Transform.translate(
                    offset: Offset(
                      (-0.012 + pose.lateral * 0.024) * diameter,
                      (-0.022 - lift * 0.021 + pose.depth * 0.008) * diameter,
                    ),
                    child: Transform(
                      alignment: Alignment.center,
                      transform: tilt,
                      child: Center(child: child),
                    ),
                  );
                },
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Stacked silhouettes form the side wall, bevel and illuminated icon face.
class DeviceRaisedGlyph extends StatelessWidget {
  const DeviceRaisedGlyph({
    super.key,
    required this.icon,
    required this.size,
    required this.color,
    required this.accent,
    required this.shadow,
  });

  final IconData icon;
  final double size;
  final Color color;
  final Color accent;
  final Color shadow;

  @override
  Widget build(BuildContext context) {
    final palette = DeviceSpherePalette(Theme.of(context).colorScheme);
    final thickness = size * 0.075;
    final edgeColor = palette.glyphEdge(accent);
    return SizedBox.square(
      dimension: size + thickness * 4,
      child: Stack(
        alignment: Alignment.center,
        clipBehavior: Clip.none,
        children: <Widget>[
          Transform.translate(
            offset: Offset(thickness * 1.2, thickness * 1.9),
            child: Icon(
              icon,
              size: size,
              color: shadow.withValues(alpha: palette.glyphShadow),
              shadows: <Shadow>[
                Shadow(
                  color: shadow.withValues(alpha: palette.dark ? 0.48 : 0.28),
                  blurRadius: thickness * 1.8,
                  offset: Offset(0, thickness * 0.4),
                ),
              ],
            ),
          ),
          for (var layer = 3; layer > 0; layer--)
            Transform.translate(
              offset: Offset(thickness * layer * 0.23, thickness * layer / 3),
              child: Icon(
                icon,
                size: size,
                color: Color.lerp(edgeColor, accent, (3 - layer) * 0.16),
              ),
            ),
          Transform.translate(
            offset: const Offset(-0.45, -0.65),
            child: Icon(
              icon,
              size: size,
              color: Color.lerp(color, Colors.white, palette.glyphHighlight),
            ),
          ),
          ShaderMask(
            blendMode: BlendMode.srcIn,
            shaderCallback: (bounds) => LinearGradient(
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
              colors: <Color>[
                Color.lerp(color, Colors.white, palette.dark ? 0.8 : 0.22)!,
                color,
                Color.lerp(color, accent, 0.38)!,
              ],
              stops: const <double>[0, 0.5, 1],
            ).createShader(bounds),
            child: Icon(icon, size: size, color: Colors.white),
          ),
        ],
      ),
    );
  }
}

class _DeviceSpherePainter extends CustomPainter {
  _DeviceSpherePainter({
    required this.scheme,
    required this.current,
    required this.online,
    required this.elevation,
    required this.pose,
    required Listenable animation,
  }) : super(repaint: animation);

  final ColorScheme scheme;
  final bool current;
  final bool online;
  final Animation<double> elevation;
  final ValueGetter<DeviceSpherePose> pose;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.save();
    canvas.scale(size.width / 100, size.height / 100);
    const center = Offset(50, 50);
    const radius = 48.5;
    const sphere = Rect.fromLTWH(1.5, 1.5, 97, 97);
    final pose = this.pose();
    final lift = elevation.value;
    final palette = DeviceSpherePalette(scheme);
    final accent = current || online ? scheme.primary : scheme.outline;
    final base = palette.sphereBase(current: current, online: online);
    final lit = palette.sphereLit(base, current: current);
    final shade = palette.sphereShade(base);
    final lightPosition = Alignment(
      -0.42 + pose.lateral * 0.18,
      -0.52 + pose.depth * 0.09,
    );
    final lightStrength = 0.84 + pose.depth * 0.12 + lift * 0.08;

    canvas.drawOval(
      Rect.fromCenter(
        center: Offset(54 - pose.lateral * 2, 92 + lift * 2),
        width: 77 - lift * 5,
        height: 20,
      ),
      Paint()
        ..color = scheme.shadow.withValues(alpha: palette.contactShadow)
        ..maskFilter = const MaskFilter.blur(BlurStyle.normal, 4),
    );
    canvas.drawCircle(
      center,
      54,
      Paint()
        ..shader = RadialGradient(
          colors: <Color>[
            accent.withValues(alpha: 0),
            accent.withValues(
              alpha: ((current ? 0.14 : 0.05) + lift * 0.09) * palette.aura,
            ),
            accent.withValues(alpha: 0),
          ],
          stops: const <double>[0.66, 0.87, 1],
        ).createShader(Rect.fromCircle(center: center, radius: 54)),
    );

    canvas.saveLayer(
      sphere,
      Paint()..color = Colors.white.withValues(alpha: 0.78),
    );
    canvas.drawCircle(
      center,
      radius,
      Paint()
        ..shader = RadialGradient(
          center: lightPosition,
          radius: 1.08,
          colors: <Color>[
            lit,
            Color.lerp(base, accent, current ? 0.35 : 0.16)!,
            base,
            shade,
          ],
          stops: const <double>[0, 0.24, 0.56, 1],
        ).createShader(sphere),
    );

    canvas.save();
    canvas.clipPath(Path()..addOval(sphere));
    canvas.drawCircle(
      center,
      radius,
      Paint()
        ..shader = RadialGradient(
          center: Alignment(0.28 - pose.lateral * 0.12, 0.85),
          radius: 0.8,
          colors: <Color>[
            accent.withValues(
              alpha: (current ? 0.24 : 0.16) * (palette.dark ? 1 : 0.72),
            ),
            accent.withValues(alpha: 0),
          ],
        ).createShader(sphere),
    );
    final glint = Offset(
      33 + pose.lateral * 7,
      22 + pose.depth * 3 - lift * 1.5,
    );
    canvas.save();
    canvas.translate(glint.dx, glint.dy);
    canvas.rotate(-0.45 + pose.lateral * 0.12);
    canvas.scale(1, 0.56);
    canvas.drawCircle(
      Offset.zero,
      20,
      Paint()
        ..shader = RadialGradient(
          colors: <Color>[
            Colors.white.withValues(alpha: palette.glint * lightStrength),
            Colors.white.withValues(
              alpha: (palette.dark ? 0.17 : 0.08) * lightStrength,
            ),
            Colors.white.withValues(alpha: 0),
          ],
          stops: const <double>[0, 0.35, 1],
        ).createShader(const Rect.fromLTWH(-20, -20, 40, 40)),
    );
    canvas.restore();
    canvas.restore();

    canvas.drawCircle(
      center,
      radius,
      Paint()
        ..shader = SweepGradient(
          colors: <Color>[
            shade.withValues(alpha: 0.85),
            accent.withValues(alpha: 0.6),
            shade.withValues(alpha: 0.48),
            Colors.white.withValues(alpha: palette.rimWhite * lightStrength),
            accent.withValues(alpha: 0.62),
            shade.withValues(alpha: 0.85),
          ],
          stops: const <double>[0, 0.2, 0.46, 0.64, 0.84, 1],
        ).createShader(sphere)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.6 + lift * 0.5,
    );
    canvas.drawArc(
      sphere.deflate(3),
      math.pi * 1.05 + pose.lateral * 0.08,
      math.pi * 0.44,
      false,
      Paint()
        ..color = Colors.white.withValues(alpha: palette.rimArc * lightStrength)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 0.85
        ..strokeCap = StrokeCap.round,
    );
    canvas.restore();
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant _DeviceSpherePainter oldDelegate) =>
      oldDelegate.scheme != scheme ||
      oldDelegate.current != current ||
      oldDelegate.online != online ||
      oldDelegate.pose != pose ||
      oldDelegate.elevation != elevation;
}
