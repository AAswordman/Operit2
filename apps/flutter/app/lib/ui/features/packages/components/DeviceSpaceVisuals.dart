// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/device/DeviceSphere.dart';

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
