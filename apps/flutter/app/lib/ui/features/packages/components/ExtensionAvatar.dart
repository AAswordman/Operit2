// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import 'ExtensionMotion.dart';
import 'StoreVisuals.dart';

/// Corner dot that flags an extension needing attention.
enum ExtensionAvatarBadge { none, update, error }

/// Theme-harmonized tones of one extension's monogram tile.
class ExtensionAvatarPalette {
  const ExtensionAvatarPalette._({
    required this.container,
    required this.onContainer,
    required this.accent,
  });

  final Color container;
  final Color onContainer;
  final Color accent;

  /// Hue slots that stay clean as both light and dark containers.
  static const List<double> _hues = <double>[
    0,
    16,
    30,
    148,
    166,
    184,
    200,
    216,
    232,
    250,
    272,
    318,
  ];

  static final Map<String, ExtensionAvatarPalette> _cache =
      <String, ExtensionAvatarPalette>{};

  /// Tones for [seed] under the ambient theme; the same seed keeps its color.
  static ExtensionAvatarPalette of(BuildContext context, String seed) {
    final colorScheme = Theme.of(context).colorScheme;
    final slot = _hash(seed) % _hues.length;
    final key =
        '$slot|${colorScheme.primary.toARGB32()}|${colorScheme.brightness.name}';
    return _cache.putIfAbsent(key, () {
      final hue = _harmonize(_hues[slot], colorScheme.primary);
      final scheme = ColorScheme.fromSeed(
        seedColor: HSLColor.fromAHSL(1, hue, 0.62, 0.5).toColor(),
        brightness: colorScheme.brightness,
      );
      return ExtensionAvatarPalette._(
        container: scheme.primaryContainer,
        onContainer: scheme.onPrimaryContainer,
        accent: scheme.primary,
      );
    });
  }

  /// Rotates [hue] toward the theme primary by half the gap, at most 15°.
  static double _harmonize(double hue, Color primary) {
    final source = HSLColor.fromColor(primary);
    if (source.saturation < 0.15) {
      return hue;
    }
    final delta = (source.hue - hue + 540) % 360 - 180;
    final rotation = math.min(delta.abs() * 0.5, 15.0) * delta.sign;
    return (hue + rotation + 360) % 360;
  }

  /// 32-bit FNV-1a that gives the same result on native and web integers.
  static int _hash(String value) {
    var hash = 0x811c9dc5;
    for (final unit in value.codeUnits) {
      hash = (hash ^ unit) & 0xffffffff;
      // hash * 16777619, split so every intermediate stays below 2^53.
      hash = (hash * 403 + ((hash << 24) & 0xffffffff)) & 0xffffffff;
    }
    return hash;
  }
}

final RegExp _latinOrDigit = RegExp(r'[A-Za-z0-9]');
final RegExp _wordBreak = RegExp(r'[\s_\-.]+');
final RegExp _camelBreak = RegExp(r'(?<=[a-z0-9])(?=[A-Z])');

/// The first character of [label] when it is not Latin; otherwise the
/// uppercase initials of its first two words ("Google Search" → "GS").
String extensionMonogram(String label) {
  final text = label.trim();
  if (text.isEmpty) {
    return '';
  }
  final first = text.characters.first;
  if (!_latinOrDigit.hasMatch(first)) {
    return first;
  }
  final words = <String>[
    for (final part in text.split(_wordBreak)) ...part.split(_camelBreak),
  ].where((word) => word.isNotEmpty && _latinOrDigit.hasMatch(word[0]));
  return words.take(2).map((word) => word[0].toUpperCase()).join();
}

/// Identity of a marketplace entry, shared with the installed copy when the
/// entry names its runtime package.
String marketEntryAvatarSeed(core_proxy.MarketEntrySummary entry) {
  final kind = StoreKind.of(entry.type);
  final packageId =
      entry.latestVersion?.runtimePackageId ??
      entry.artifact?.runtimePackageId;
  return '${kind.id}:${packageId ?? entry.id}';
}

/// Monogram tile, or the extension's own logo, that stands for one extension
/// everywhere.
class ExtensionAvatar extends StatelessWidget {
  const ExtensionAvatar({
    super.key,
    required this.seed,
    required this.label,
    this.kind,
    this.size = 40,
    this.logo,
    this.enabled = true,
    this.busy = false,
    this.badge = ExtensionAvatarBadge.none,
  });

  /// Stable identity, usually `kind:id`; it picks the tile color.
  final String seed;

  /// Display name the monogram is taken from.
  final String label;

  /// Shows the kind in a corner badge; pass it only in mixed-kind lists.
  final StoreKind? kind;
  final double size;

  /// Square, full-bleed logo clipped to the monogram tile's shape, with no
  /// plate behind it.
  final Widget? logo;
  final bool enabled;
  final bool busy;
  final ExtensionAvatarBadge badge;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final palette = ExtensionAvatarPalette.of(context, seed);
    final motion = ExtensionMotion.of(context);
    final dark = theme.brightness == Brightness.dark;
    final radius = BorderRadius.circular(size * 0.3);
    final logo = this.logo;
    final Widget face;
    if (logo == null) {
      final monogram = extensionMonogram(label);
      final pair = monogram.characters.length > 1;
      face = DecoratedBox(
        decoration: ShapeDecoration(
          shape: RoundedSuperellipseBorder(
            borderRadius: radius,
            side: BorderSide(
              color: palette.onContainer.withValues(alpha: dark ? 0.12 : 0.07),
            ),
          ),
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: <Color>[
              Color.lerp(palette.container, Colors.white, dark ? 0.07 : 0.4)!,
              palette.container,
            ],
          ),
        ),
        child: Center(
          child: Text(
            monogram,
            maxLines: 1,
            softWrap: false,
            textScaler: TextScaler.noScaling,
            style: TextStyle(
              color: palette.onContainer,
              fontSize: size * (pair ? 0.36 : 0.44),
              fontWeight: FontWeight.w700,
              height: 1,
              letterSpacing: pair ? -size * 0.012 : 0,
            ),
          ),
        ),
      );
    } else {
      // The logo brings its own square, full-bleed tile with no plate or
      // padding behind it; the clip gives it the monogram tile's shape.
      face = ClipRSuperellipse(borderRadius: radius, child: logo);
    }
    final kind = this.kind;
    final dot = math.max(8.0, size * 0.2);
    final kindBadge = math.max(14.0, size * 0.4);
    return SizedBox.square(
      dimension: size,
      child: Stack(
        clipBehavior: Clip.none,
        children: <Widget>[
          Positioned.fill(
            child: TweenAnimationBuilder<double>(
              tween: Tween<double>(end: enabled ? 1 : 0),
              duration: motion.standard,
              curve: ExtensionMotion.enter,
              child: face,
              builder: (context, saturation, child) => saturation >= 1
                  ? child!
                  : Opacity(
                      opacity: 0.5 + 0.5 * saturation,
                      child: ColorFiltered(
                        colorFilter: _saturationFilter(saturation),
                        child: child,
                      ),
                    ),
            ),
          ),
          if (busy)
            Positioned(
              left: -3,
              top: -3,
              right: -3,
              bottom: -3,
              child: _BusyTrace(
                radius: size * 0.3 + 3,
                color: colorScheme.primary,
                animate: motion.ambient,
              ),
            ),
          if (badge != ExtensionAvatarBadge.none)
            Positioned(
              right: -dot * 0.2,
              top: -dot * 0.2,
              child: Container(
                width: dot,
                height: dot,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: badge == ExtensionAvatarBadge.error
                      ? colorScheme.error
                      : colorScheme.primary,
                  border: Border.all(color: colorScheme.surface, width: 2),
                ),
              ),
            ),
          if (kind != null && size >= 36)
            Positioned(
              right: -kindBadge * 0.18,
              bottom: -kindBadge * 0.18,
              child: Container(
                width: kindBadge,
                height: kindBadge,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: colorScheme.surface,
                  boxShadow: <BoxShadow>[
                    BoxShadow(
                      color: Colors.black.withValues(alpha: dark ? 0.4 : 0.14),
                      blurRadius: 3,
                      offset: const Offset(0, 1),
                    ),
                  ],
                ),
                child: Icon(
                  kind.icon,
                  size: kindBadge * 0.62,
                  color: palette.accent,
                ),
              ),
            ),
        ],
      ),
    );
  }
}

ColorFilter _saturationFilter(double saturation) {
  const r = 0.2126;
  const g = 0.7152;
  const b = 0.0722;
  final rest = 1 - saturation;
  return ColorFilter.matrix(<double>[
    r * rest + saturation, g * rest, b * rest, 0, 0, //
    r * rest, g * rest + saturation, b * rest, 0, 0, //
    r * rest, g * rest, b * rest + saturation, 0, 0, //
    0, 0, 0, 1, 0,
  ]);
}

/// A short highlight running around the tile while an operation is pending.
class _BusyTrace extends StatefulWidget {
  const _BusyTrace({
    required this.radius,
    required this.color,
    required this.animate,
  });

  final double radius;
  final Color color;
  final bool animate;

  @override
  State<_BusyTrace> createState() => _BusyTraceState();
}

class _BusyTraceState extends State<_BusyTrace>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 1300),
  );

  @override
  void initState() {
    super.initState();
    _sync();
  }

  @override
  void didUpdateWidget(covariant _BusyTrace oldWidget) {
    super.didUpdateWidget(oldWidget);
    _sync();
  }

  void _sync() {
    if (widget.animate) {
      if (!_controller.isAnimating) {
        _controller.repeat();
      }
    } else {
      _controller.stop();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return IgnorePointer(
      child: RepaintBoundary(
        child: CustomPaint(
          painter: _BusyTracePainter(
            progress: _controller,
            radius: widget.radius,
            color: widget.color,
            animate: widget.animate,
          ),
        ),
      ),
    );
  }
}

class _BusyTracePainter extends CustomPainter {
  _BusyTracePainter({
    required this.progress,
    required this.radius,
    required this.color,
    required this.animate,
  }) : super(repaint: progress);

  final Animation<double> progress;
  final double radius;
  final Color color;
  final bool animate;

  @override
  void paint(Canvas canvas, Size size) {
    final path = RoundedSuperellipseBorder(
      borderRadius: BorderRadius.circular(radius),
    ).getOuterPath(Offset.zero & size);
    final paint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2
      ..strokeCap = StrokeCap.round;
    if (!animate) {
      canvas.drawPath(path, paint..color = color.withValues(alpha: 0.55));
      return;
    }
    canvas.drawPath(path, paint..color = color.withValues(alpha: 0.16));
    final metric = path.computeMetrics().first;
    final length = metric.length;
    final segment = length * 0.28;
    final start = progress.value * length;
    paint.color = color;
    canvas.drawPath(
      metric.extractPath(start, math.min(start + segment, length)),
      paint,
    );
    if (start + segment > length) {
      canvas.drawPath(metric.extractPath(0, start + segment - length), paint);
    }
  }

  @override
  bool shouldRepaint(covariant _BusyTracePainter oldDelegate) =>
      oldDelegate.radius != radius ||
      oldDelegate.color != color ||
      oldDelegate.animate != animate;
}

/// Caches ToolPkg logo reads per Core connection, package version and resource.
class ExtensionLogoCache {
  ExtensionLogoCache._();

  static final Expando<Map<String, Future<core_proxy.ToolPkgLogoBytes?>>>
  _reads = Expando<Map<String, Future<core_proxy.ToolPkgLogoBytes?>>>();

  static Future<core_proxy.ToolPkgLogoBytes?> load(
    GeneratedCoreProxyClients clients, {
    required String packageName,
    required String resourceKey,
    required String version,
  }) {
    final reads = _reads[clients.bridge] ??=
        <String, Future<core_proxy.ToolPkgLogoBytes?>>{};
    final key = '$packageName|$version|$resourceKey';
    final cached = reads[key];
    if (cached != null) {
      return cached;
    }
    final read = clients.application.packageManager().readToolPkgLogoBytes(
      packageName: packageName,
    );
    reads[key] = read;
    // A failed read is retried by the next avatar that asks for it.
    unawaited(read.then<void>((_) {}, onError: (Object _) => reads.remove(key)));
    return read;
  }
}

/// Renders logo bytes in the MIME types Core accepts for ToolPkg logos.
class ExtensionLogoImage extends StatelessWidget {
  const ExtensionLogoImage({super.key, required this.logo});

  final core_proxy.ToolPkgLogoBytes logo;

  @override
  Widget build(BuildContext context) {
    return switch (logo.mimeType) {
      'image/svg+xml' => SvgPicture.memory(logo.bytes, fit: BoxFit.contain),
      'image/png' || 'image/jpeg' || 'image/webp' => Image.memory(
        logo.bytes,
        fit: BoxFit.contain,
        gaplessPlayback: true,
        filterQuality: FilterQuality.medium,
      ),
      _ => throw StateError(
        'Unsupported ToolPkg logo MIME type: ${logo.mimeType}',
      ),
    };
  }
}

/// Avatar of an installed extension; ToolPkgs with a logo show it once loaded.
class CatalogEntryAvatar extends StatelessWidget {
  const CatalogEntryAvatar({
    super.key,
    required this.clients,
    required this.entry,
    this.size = 40,
    this.enabled = true,
    this.busy = false,
    this.badge = ExtensionAvatarBadge.none,
    this.showKind = false,
  });

  final GeneratedCoreProxyClients clients;
  final core_proxy.ExtensionCatalogEntry entry;
  final double size;
  final bool enabled;
  final bool busy;
  final ExtensionAvatarBadge badge;
  final bool showKind;

  @override
  Widget build(BuildContext context) {
    final resourceKey = entry.logoResourceKey;
    if (resourceKey == null) {
      return _avatar(null);
    }
    return FutureBuilder<core_proxy.ToolPkgLogoBytes?>(
      future: ExtensionLogoCache.load(
        clients,
        packageName: entry.id,
        resourceKey: resourceKey,
        version: entry.version,
      ),
      builder: (context, snapshot) {
        if (snapshot.connectionState != ConnectionState.done) {
          return _avatar(const SizedBox.expand());
        }
        final logo = snapshot.data;
        if (snapshot.hasError || logo == null) {
          if (snapshot.hasError) {
            debugPrint(
              'Failed to read logo of ${entry.id}: ${snapshot.error}',
            );
          }
          return _avatar(
            Icon(
              Icons.broken_image_outlined,
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          );
        }
        return _avatar(ExtensionLogoImage(logo: logo));
      },
    );
  }

  Widget _avatar(Widget? logo) {
    return ExtensionAvatar(
      seed: '${entry.kind}:${entry.id}',
      label: entry.displayName,
      kind: showKind ? StoreKind.of(entry.kind) : null,
      size: size,
      logo: logo,
      enabled: enabled,
      busy: busy,
      badge: badge,
    );
  }
}
