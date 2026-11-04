// ignore_for_file: file_names

import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/device/DeviceSphere.dart';
import 'ExtensionMotion.dart';

/// Overlapping small spheres, e.g. the devices a shared extension reaches.
class DeviceSphereStack extends StatelessWidget {
  const DeviceSphereStack({
    super.key,
    required this.devices,
    required this.currentDeviceId,
    this.diameter = 22,
    this.maxShown = 5,
  });

  final List<core_proxy.RuntimeDeviceSpaceDevice> devices;
  final String? currentDeviceId;
  final double diameter;
  final int maxShown;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final shown = devices.take(maxShown).toList(growable: false);
    final hidden = devices.length - shown.length;
    final step = diameter * 0.72;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        SizedBox(
          width: shown.isEmpty ? 0 : diameter + step * (shown.length - 1),
          height: diameter,
          child: Stack(
            clipBehavior: Clip.none,
            children: <Widget>[
              for (var index = shown.length - 1; index >= 0; index -= 1)
                Positioned(
                  left: step * index,
                  child: Tooltip(
                    message: _deviceTooltip(shown[index], currentDeviceId),
                    child: DeviceSphere.still(
                      diameter: diameter,
                      icon: devicePlatformIcon(shown[index].platform),
                      current: shown[index].deviceId == currentDeviceId,
                      online: shown[index].online,
                    ),
                  ),
                ),
            ],
          ),
        ),
        if (hidden > 0) ...<Widget>[
          const SizedBox(width: 6),
          Text(
            '+$hidden',
            style: theme.textTheme.labelSmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
      ],
    );
  }
}

String _deviceTooltip(
  core_proxy.RuntimeDeviceSpaceDevice device,
  String? currentDeviceId,
) {
  final name = device.deviceName.trim().isEmpty
      ? device.platform
      : device.deviceName;
  if (device.deviceId == currentDeviceId) {
    return '$name · 本机';
  }
  return '$name · ${device.online ? '在线' : '离线'}';
}

/// This device and the other members of its space, joined by links that
/// carry flowing light while online and run faster while a sync is active.
class DeviceSpaceConstellation extends StatefulWidget {
  const DeviceSpaceConstellation({
    super.key,
    required this.current,
    required this.others,
    required this.syncOf,
    this.height = 168,
    this.showNames = true,
  });

  final core_proxy.RuntimeDeviceSpaceDevice current;
  final List<core_proxy.RuntimeDeviceSpaceDevice> others;
  final core_proxy.SpaceSyncProgress? Function(String deviceId) syncOf;
  final double height;
  final bool showNames;

  @override
  State<DeviceSpaceConstellation> createState() =>
      _DeviceSpaceConstellationState();
}

class _DeviceSpaceConstellationState extends State<DeviceSpaceConstellation>
    with SingleTickerProviderStateMixin {
  static const double _labelWidth = 96;

  late final AnimationController _flow = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 2400),
  );

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (ExtensionMotion.of(context).ambient) {
      if (!_flow.isAnimating) {
        _flow.repeat();
      }
    } else {
      _flow.stop();
    }
  }

  @override
  void dispose() {
    _flow.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final animate = ExtensionMotion.of(context).ambient;
    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth;
        final compact = width < 480;
        final currentSize = compact ? 56.0 : 68.0;
        final otherSize = compact ? 34.0 : 42.0;
        final labelSpace = widget.showNames && !compact ? 20.0 : 0.0;
        final height = widget.height;
        final centerY = (height - labelSpace) / 2;
        // Names are centred under their sphere, so the end nodes keep half a
        // label of room from the card edges.
        final edge = labelSpace > 0 ? _labelWidth / 2 : 0.0;
        final currentCenter = Offset(
          math.max(currentSize / 2 + 8, edge),
          centerY,
        );
        final count = widget.others.length;
        final startX = currentCenter.dx + currentSize / 2 + otherSize;
        final endX = width - math.max(otherSize / 2 + 6, edge);
        final centers = <Offset>[
          for (var index = 0; index < count; index += 1)
            () {
              final t = count == 1 ? 0.5 : index / (count - 1);
              final x = count == 1
                  ? math.min(startX + (endX - startX) * 0.45, endX)
                  : startX + (endX - startX) * t;
              // A gentle arc: the middle of the row rises toward the top.
              final lift = math.sin(t * math.pi) * (centerY - otherSize / 2 - 4);
              final y = centerY + (count == 1 ? 0 : (centerY * 0.35) - lift);
              return Offset(x, y.clamp(otherSize / 2, height - labelSpace - otherSize / 2));
            }(),
        ];
        final links = <_ConstellationLink>[
          for (var index = 0; index < count; index += 1)
            _ConstellationLink(
              from: currentCenter,
              to: centers[index],
              online: widget.others[index].online,
              syncing:
                  widget.syncOf(widget.others[index].deviceId)?.status ==
                  core_proxy.SpaceSyncStatus.syncing,
            ),
        ];
        return SizedBox(
          height: height,
          child: Stack(
            clipBehavior: Clip.none,
            children: <Widget>[
              Positioned.fill(
                child: RepaintBoundary(
                  child: CustomPaint(
                    painter: _ConstellationPainter(
                      links: links,
                      flow: _flow,
                      animate: animate,
                      online: theme.colorScheme.primary,
                      offline: theme.colorScheme.outline,
                    ),
                  ),
                ),
              ),
              _placed(
                center: currentCenter,
                size: currentSize,
                labelSpace: labelSpace,
                child: _SyncedSphere(
                  device: widget.current,
                  current: true,
                  diameter: currentSize,
                  sync: null,
                  flow: _flow,
                  animate: animate,
                ),
                label: widget.current.deviceName,
              ),
              for (var index = 0; index < count; index += 1)
                _placed(
                  center: centers[index],
                  size: otherSize,
                  labelSpace: labelSpace,
                  child: _SyncedSphere(
                    device: widget.others[index],
                    current: false,
                    diameter: otherSize,
                    sync: widget.syncOf(widget.others[index].deviceId),
                    flow: _flow,
                    animate: animate,
                  ),
                  label: widget.others[index].deviceName,
                ),
            ],
          ),
        );
      },
    );
  }

  Widget _placed({
    required Offset center,
    required double size,
    required double labelSpace,
    required Widget child,
    required String label,
  }) {
    final theme = Theme.of(context);
    return Positioned(
      left: center.dx - _labelWidth / 2,
      top: center.dy - size / 2,
      width: _labelWidth,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          child,
          if (labelSpace > 0) ...<Widget>[
            const SizedBox(height: 4),
            Text(
              label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.center,
              style: theme.textTheme.labelSmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _ConstellationLink {
  const _ConstellationLink({
    required this.from,
    required this.to,
    required this.online,
    required this.syncing,
  });

  final Offset from;
  final Offset to;
  final bool online;
  final bool syncing;

  Path get path {
    final middle = Offset.lerp(from, to, 0.5)!;
    final normal = Offset(-(to.dy - from.dy), to.dx - from.dx);
    final length = normal.distance;
    final bend = length == 0 ? Offset.zero : normal / length * 14;
    return Path()
      ..moveTo(from.dx, from.dy)
      ..quadraticBezierTo(
        middle.dx - bend.dx,
        middle.dy - bend.dy,
        to.dx,
        to.dy,
      );
  }
}

class _ConstellationPainter extends CustomPainter {
  _ConstellationPainter({
    required this.links,
    required this.flow,
    required this.animate,
    required this.online,
    required this.offline,
  }) : super(repaint: flow);

  final List<_ConstellationLink> links;
  final Animation<double> flow;
  final bool animate;
  final Color online;
  final Color offline;

  @override
  void paint(Canvas canvas, Size size) {
    for (final link in links) {
      final path = link.path;
      final metric = path.computeMetrics().first;
      final base = Paint()
        ..style = PaintingStyle.stroke
        ..strokeCap = StrokeCap.round
        ..strokeWidth = 1.4;
      if (!link.online) {
        base.color = offline.withValues(alpha: 0.45);
        const dash = 4.0;
        for (var start = 0.0; start < metric.length; start += dash * 2) {
          canvas.drawPath(
            metric.extractPath(start, math.min(start + dash, metric.length)),
            base,
          );
        }
        continue;
      }
      base.color = online.withValues(alpha: link.syncing ? 0.5 : 0.32);
      canvas.drawPath(path, base);
      if (!animate) {
        continue;
      }
      // Syncing links carry two pulses per cycle, idle ones one.
      final pulses = link.syncing ? 2 : 1;
      final glowLength = metric.length * 0.22;
      for (var pulse = 0; pulse < pulses; pulse += 1) {
        final phase = (flow.value * pulses + pulse / pulses) % 1;
        final head = phase * (metric.length + glowLength);
        final start = math.max(0.0, head - glowLength);
        final end = math.min(metric.length, head);
        if (end <= start) {
          continue;
        }
        final from = metric.getTangentForOffset(start)!.position;
        final to = metric.getTangentForOffset(end)!.position;
        canvas.drawPath(
          metric.extractPath(start, end),
          Paint()
            ..style = PaintingStyle.stroke
            ..strokeCap = StrokeCap.round
            ..strokeWidth = link.syncing ? 2.4 : 2
            ..shader = ui.Gradient.linear(from, to, <Color>[
              online.withValues(alpha: 0),
              online.withValues(alpha: link.syncing ? 0.95 : 0.7),
            ]),
        );
      }
    }
  }

  @override
  bool shouldRepaint(covariant _ConstellationPainter oldDelegate) =>
      oldDelegate.animate != animate ||
      oldDelegate.online != online ||
      oldDelegate.offline != offline ||
      !_sameLinks(oldDelegate.links, links);

  static bool _sameLinks(List<_ConstellationLink> a, List<_ConstellationLink> b) {
    if (a.length != b.length) {
      return false;
    }
    for (var index = 0; index < a.length; index += 1) {
      if (a[index].from != b[index].from ||
          a[index].to != b[index].to ||
          a[index].online != b[index].online ||
          a[index].syncing != b[index].syncing) {
        return false;
      }
    }
    return true;
  }
}

/// A device sphere wearing its sync state: a running arc while syncing, a
/// dotted ring while waiting and a red dot after an error.
class _SyncedSphere extends StatelessWidget {
  const _SyncedSphere({
    required this.device,
    required this.current,
    required this.diameter,
    required this.sync,
    required this.flow,
    required this.animate,
  });

  final core_proxy.RuntimeDeviceSpaceDevice device;
  final bool current;
  final double diameter;
  final core_proxy.SpaceSyncProgress? sync;
  final Animation<double> flow;
  final bool animate;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final sync = this.sync;
    final failed = sync?.error != null;
    return Tooltip(
      message: [
        _deviceTooltip(device, current ? device.deviceId : null),
        if (sync != null) _syncLabel(sync),
      ].join('\n'),
      child: SizedBox.square(
        dimension: diameter,
        child: Stack(
          clipBehavior: Clip.none,
          children: <Widget>[
            DeviceSphere.still(
              diameter: diameter,
              icon: devicePlatformIcon(device.platform),
              current: current,
              online: device.online,
            ),
            if (sync != null && sync.status != core_proxy.SpaceSyncStatus.completed)
              Positioned(
                left: -4,
                top: -4,
                right: -4,
                bottom: -4,
                child: IgnorePointer(
                  child: CustomPaint(
                    painter: _SyncRingPainter(
                      flow: flow,
                      animate: animate,
                      waiting: sync.status == core_proxy.SpaceSyncStatus.waiting,
                      color: colorScheme.primary,
                      muted: colorScheme.outline,
                    ),
                  ),
                ),
              ),
            if (failed)
              Positioned(
                right: -1,
                top: -1,
                child: Container(
                  width: 10,
                  height: 10,
                  decoration: BoxDecoration(
                    color: colorScheme.error,
                    shape: BoxShape.circle,
                    border: Border.all(color: colorScheme.surface, width: 2),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

String _syncLabel(core_proxy.SpaceSyncProgress sync) {
  final error = sync.error;
  if (error != null) {
    return '同步出错：$error';
  }
  return switch (sync.status) {
    core_proxy.SpaceSyncStatus.syncing => switch (sync.stage) {
      core_proxy.SpaceSyncStage.checking => '正在核对',
      core_proxy.SpaceSyncStage.records => '正在同步记录',
      core_proxy.SpaceSyncStage.files => '正在同步文件',
    },
    core_proxy.SpaceSyncStatus.waiting => '等待同步',
    core_proxy.SpaceSyncStatus.completed => '已同步',
  };
}

class _SyncRingPainter extends CustomPainter {
  _SyncRingPainter({
    required this.flow,
    required this.animate,
    required this.waiting,
    required this.color,
    required this.muted,
  }) : super(repaint: flow);

  final Animation<double> flow;
  final bool animate;
  final bool waiting;
  final Color color;
  final Color muted;

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Offset.zero & size;
    final paint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeWidth = 2;
    if (waiting) {
      paint.color = muted.withValues(alpha: 0.7);
      const segments = 16;
      for (var index = 0; index < segments; index += 1) {
        canvas.drawArc(
          rect.deflate(1),
          index * 2 * math.pi / segments,
          math.pi / segments,
          false,
          paint,
        );
      }
      return;
    }
    paint.color = color.withValues(alpha: 0.2);
    canvas.drawOval(rect.deflate(1), paint);
    paint.color = color;
    final start = animate ? flow.value * 2 * math.pi * 2 : -math.pi / 2;
    canvas.drawArc(rect.deflate(1), start, math.pi * 0.6, false, paint);
  }

  @override
  bool shouldRepaint(covariant _SyncRingPainter oldDelegate) =>
      oldDelegate.waiting != waiting ||
      oldDelegate.animate != animate ||
      oldDelegate.color != color ||
      oldDelegate.muted != muted;
}

/// One-line description of a sync for headers.
String deviceSpaceSyncSummary(
  List<core_proxy.RuntimeDeviceSpaceDevice> others,
  core_proxy.SpaceSyncProgress? Function(String deviceId) syncOf,
) {
  for (final device in others) {
    final sync = syncOf(device.deviceId);
    if (sync?.error != null) {
      return '与「${device.deviceName}」同步出错';
    }
  }
  for (final device in others) {
    final sync = syncOf(device.deviceId);
    if (sync?.status == core_proxy.SpaceSyncStatus.syncing) {
      return '正在与「${device.deviceName}」同步';
    }
  }
  return '';
}
