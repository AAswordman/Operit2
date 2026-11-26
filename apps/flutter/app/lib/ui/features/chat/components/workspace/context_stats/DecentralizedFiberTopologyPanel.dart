// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../common/components/M3LoadingIndicator.dart';
import '../../../../../common/device/DeviceSphere.dart';
import 'TopBarCapsuleModels.dart';

/// Unified decentralized node and curved optical-fiber routing topology panel.
///
/// Merges device selection, online connectivity, relay prism state, and live
/// execution / handoff telemetry into a single non-redundant visual canvas.
class DecentralizedFiberTopologyPanel extends StatefulWidget {
  const DecentralizedFiberTopologyPanel({
    super.key,
    required this.topology,
    required this.topologyFailed,
    required this.bindingStatus,
    required this.routeFailed,
    required this.processingState,
    required this.switchingDeviceId,
    required this.switchError,
    required this.ownerDeviceId,
    required this.displayMode,
    required this.onDisplayModeCycle,
    required this.canSelectDevice,
    required this.onSelectDevice,
    required this.onAddDevice,
  });

  final core_proxy.RuntimeDeviceSpaceTopology? topology;
  final bool topologyFailed;
  final core_proxy.BindingRouteStatus? bindingStatus;
  final bool routeFailed;
  final core_proxy.InputProcessingState processingState;
  final String? switchingDeviceId;
  final String? switchError;
  final String? ownerDeviceId;
  final NodeLabelDisplayMode displayMode;
  final VoidCallback onDisplayModeCycle;
  final bool Function(core_proxy.RuntimeDeviceSpaceDevice device)
  canSelectDevice;
  final ValueChanged<core_proxy.RuntimeDeviceSpaceDevice> onSelectDevice;
  final VoidCallback onAddDevice;

  @override
  State<DecentralizedFiberTopologyPanel> createState() =>
      _DecentralizedFiberTopologyPanelState();
}

class _DecentralizedFiberTopologyPanelState
    extends State<DecentralizedFiberTopologyPanel>
    with SingleTickerProviderStateMixin {
  late final AnimationController _fiberController;
  Timer? _tickerTimer;
  int _tickerPhase = 0;
  String? _hoveredDeviceId;

  bool get _hasActiveFiberMotion {
    if (widget.switchingDeviceId != null || _hoveredDeviceId != null) {
      return true;
    }
    final status = widget.bindingStatus;
    if (status != null &&
        status.activeNodeId.isNotEmpty &&
        status.activeNodeId != status.ownerNodeId) {
      return true;
    }
    return _isExecutingCall(widget.processingState);
  }

  bool _isExecutingCall(core_proxy.InputProcessingState state) {
    return switch (state.tag) {
      'ExecutingTool' ||
      'ToolProgress' ||
      'ProcessingToolResult' ||
      'Receiving' ||
      'Connecting' ||
      'Processing' ||
      'ExecutingPlan' ||
      'Summarizing' => true,
      _ => false,
    };
  }

  @override
  void initState() {
    super.initState();
    _fiberController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1200),
    );
    _syncAnimation();
    _syncTickerTimer();
  }

  @override
  void didUpdateWidget(covariant DecentralizedFiberTopologyPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    _syncAnimation();
    _syncTickerTimer();
  }

  void _syncAnimation() {
    if (_hasActiveFiberMotion) {
      if (!_fiberController.isAnimating) {
        _fiberController.repeat();
      }
    } else {
      _fiberController.stop();
      _fiberController.value = 0;
    }
  }

  void _syncTickerTimer() {
    final shouldCycle =
        widget.displayMode == NodeLabelDisplayMode.auto &&
        widget.ownerDeviceId != null &&
        _isExecutingCall(widget.processingState);
    if (shouldCycle) {
      _tickerTimer ??= Timer.periodic(const Duration(milliseconds: 2600), (_) {
        if (!mounted) return;
        setState(() => _tickerPhase = 1 - _tickerPhase);
      });
    } else {
      _tickerTimer?.cancel();
      _tickerTimer = null;
      _tickerPhase = 0;
    }
  }

  @override
  void dispose() {
    _tickerTimer?.cancel();
    _fiberController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final topology = widget.topology;
    final ownerInList = topology?.devices.any(
      (device) => device.deviceId == widget.ownerDeviceId,
    );

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Row(
          children: <Widget>[
            Expanded(
              child: Text(
                '节点与路由',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.labelLarge?.copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            const SizedBox(width: 6),
            Flexible(
              flex: 0,
              child: Tooltip(
                message: '切换节点显示（名称 / 任务状态）',
                child: InkWell(
                  onTap: widget.onDisplayModeCycle,
                  borderRadius: BorderRadius.circular(999),
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 6,
                      vertical: 2,
                    ),
                    decoration: BoxDecoration(
                      color: colors.onSurface.withValues(alpha: 0.06),
                      borderRadius: BorderRadius.circular(999),
                    ),
                    child: Text(
                      switch (widget.displayMode) {
                        NodeLabelDisplayMode.auto => '名/态',
                        NodeLabelDisplayMode.nameOnly => '设备名',
                        NodeLabelDisplayMode.taskOnly => '任务态',
                      },
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.labelSmall?.copyWith(
                        fontSize: 10.5,
                        color: colors.onSurfaceVariant,
                      ),
                    ),
                  ),
                ),
              ),
            ),
            IconButton(
              tooltip: '添加设备',
              onPressed: widget.onAddDevice,
              icon: const Icon(Icons.add_rounded, size: 16),
              color: colors.onSurfaceVariant,
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 26, minHeight: 26),
              visualDensity: VisualDensity.compact,
            ),
          ],
        ),
        const SizedBox(height: 4),
        if (topology == null)
          Text(
            widget.topologyFailed
                ? l10n.contextStatsDevicesLoadFailed
                : l10n.loading,
            style: widget.topologyFailed
                ? theme.textTheme.bodySmall?.copyWith(color: colors.error)
                : theme.textTheme.bodySmall,
          )
        else
          _buildTopologyCanvas(theme, colors, topology),
        if (widget.ownerDeviceId != null && ownerInList == false) ...<Widget>[
          const SizedBox(height: 6),
          Text(
            '执行设备不在当前空间中',
            style: theme.textTheme.bodySmall?.copyWith(color: colors.error),
          ),
        ],
        if (widget.switchError case final error?) ...<Widget>[
          const SizedBox(height: 6),
          Tooltip(
            message: error,
            child: Text(
              error,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall?.copyWith(color: colors.error),
            ),
          ),
        ],
        if (widget.routeFailed) ...<Widget>[
          const SizedBox(height: 6),
          Text(
            '无法读取执行设备状态',
            style: theme.textTheme.bodySmall?.copyWith(color: colors.error),
          ),
        ],
      ],
    );
  }

  Widget _buildTopologyCanvas(
    ThemeData theme,
    ColorScheme colors,
    core_proxy.RuntimeDeviceSpaceTopology topology,
  ) {
    core_proxy.RuntimeDeviceSpaceDevice? localDevice;
    final remoteDevices = <core_proxy.RuntimeDeviceSpaceDevice>[];
    for (final device in topology.devices) {
      if (device.deviceId == topology.currentDeviceId && localDevice == null) {
        localDevice = device;
      } else {
        remoteDevices.add(device);
      }
    }

    if (localDevice == null || remoteDevices.isEmpty) {
      return Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          for (var i = 0; i < topology.devices.length; i++) ...<Widget>[
            if (i > 0) const SizedBox(height: 4),
            _buildPeerCapsule(
              device: topology.devices[i],
              isLocal: topology.devices[i].deviceId == topology.currentDeviceId,
              isExecuting: topology.devices[i].deviceId == widget.ownerDeviceId,
            ),
          ],
        ],
      );
    }

    final local = localDevice;
    final fiberLinks = remoteDevices
        .map((device) => _resolveFiberSpec(device))
        .toList(growable: false);

    return Container(
      padding: const EdgeInsets.all(6),
      decoration: BoxDecoration(
        color: colors.onSurface.withValues(alpha: 0.035),
        borderRadius: BorderRadius.circular(11),
        border: Border.all(
          color: colors.outlineVariant.withValues(alpha: 0.5),
        ),
      ),
      child: IntrinsicHeight(
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Center(child: _buildLocalOrb(colors, local)),
            SizedBox(
              width: 40,
              child: AnimatedBuilder(
                animation: _fiberController,
                builder: (context, _) {
                  return CustomPaint(
                    painter: _CurvedFiberBundlePainter(
                      links: fiberLinks,
                      t: _fiberController.value,
                      primary: colors.primary,
                      secondary: colors.tertiary,
                      errorColor: colors.error,
                      idleColor: colors.onSurface.withValues(alpha: 0.22),
                      cardSurface: colors.surfaceContainer,
                    ),
                  );
                },
              ),
            ),
            Expanded(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  for (var i = 0; i < remoteDevices.length; i++) ...<Widget>[
                    if (i > 0) const SizedBox(height: 4),
                    _buildPeerCapsule(
                      device: remoteDevices[i],
                      isLocal: false,
                      isExecuting:
                          remoteDevices[i].deviceId == widget.ownerDeviceId,
                    ),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  _FiberLinkSpec _resolveFiberSpec(core_proxy.RuntimeDeviceSpaceDevice device) {
    final isExec = device.deviceId == widget.ownerDeviceId;
    final online = isExec
        ? (widget.bindingStatus?.ownerReachable ?? device.online)
        : device.online;
    final isSwitchingTo = widget.switchingDeviceId == device.deviceId;
    final isRouteHandoff =
        isExec &&
        widget.bindingStatus != null &&
        widget.bindingStatus!.activeNodeId.isNotEmpty &&
        widget.bindingStatus!.activeNodeId != widget.bindingStatus!.ownerNodeId;
    final isHovered =
        !isExec &&
        _hoveredDeviceId == device.deviceId &&
        widget.canSelectDevice(device);
    final isRelay = device.relayHops != null && device.relayHops! > 0;

    final FiberLinkState state;
    if (!online) {
      state = FiberLinkState.broken;
    } else if (isSwitchingTo || isRouteHandoff) {
      state = FiberLinkState.handoff;
    } else if (isExec && _isExecutingCall(widget.processingState)) {
      state = FiberLinkState.bidirectional;
    } else if (isHovered) {
      state = FiberLinkState.preview;
    } else if (isExec) {
      state = FiberLinkState.active;
    } else {
      state = FiberLinkState.idle;
    }

    return _FiberLinkSpec(state: state, hasRelay: isRelay);
  }

  Widget _buildLocalOrb(
    ColorScheme colors,
    core_proxy.RuntimeDeviceSpaceDevice local,
  ) {
    final isExecuting = local.deviceId == widget.ownerDeviceId;
    final isSwitching = widget.switchingDeviceId == local.deviceId;
    final online = isExecuting
        ? (widget.bindingStatus?.ownerReachable ?? local.online)
        : local.online;
    final canSelect = widget.canSelectDevice(local);
    final connection = _connectionLabel(local, online);
    final status = <String>[
      '本机',
      if (isExecuting) '当前执行设备',
      if (isSwitching) '正在切换',
      connection,
    ].join('，');
    final onlineColor = colors.brightness == Brightness.dark
        ? Colors.green.shade400
        : Colors.green.shade700;

    return Semantics(
      container: true,
      label: '${local.deviceName}，$status',
      excludeSemantics: true,
      button: true,
      selected: isExecuting,
      enabled: canSelect,
      onTap: canSelect ? () => widget.onSelectDevice(local) : null,
      child: Tooltip(
        message: '本机 · ${local.deviceName}',
        child: MouseRegion(
          onEnter: (_) => _setHovered(local.deviceId),
          onExit: (_) => _clearHovered(local.deviceId),
          child: InkWell(
            onTap: canSelect ? () => widget.onSelectDevice(local) : null,
            borderRadius: BorderRadius.circular(10),
            child: AnimatedContainer(
              duration: const Duration(milliseconds: 200),
              width: 34,
              height: 34,
              decoration: BoxDecoration(
                color: isExecuting
                    ? colors.primary.withValues(alpha: 0.16)
                    : colors.surfaceContainer,
                borderRadius: BorderRadius.circular(10),
                border: Border.all(
                  color: !online
                      ? colors.error
                      : isExecuting
                      ? colors.primary
                      : colors.outlineVariant,
                  width: isExecuting ? 1.5 : 1,
                ),
                boxShadow: isExecuting
                    ? <BoxShadow>[
                        BoxShadow(
                          color: colors.primary.withValues(alpha: 0.24),
                          blurRadius: 8,
                        ),
                      ]
                    : null,
              ),
              child: Stack(
                clipBehavior: Clip.none,
                alignment: Alignment.center,
                children: <Widget>[
                  if (isSwitching)
                    const M3LoadingIndicator(size: 14)
                  else
                    Icon(
                      devicePlatformIcon(local.platform),
                      size: 15,
                      color: isExecuting
                          ? colors.primary
                          : colors.onSurfaceVariant,
                    ),
                  Positioned(
                    right: 2.5,
                    bottom: 2.5,
                    child: Container(
                      width: 7,
                      height: 7,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: !online
                            ? colors.error
                            : isExecuting
                            ? colors.primary
                            : onlineColor,
                        border: Border.all(
                          color: colors.surfaceContainer,
                          width: 1.2,
                        ),
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

  Widget _buildPeerCapsule({
    required core_proxy.RuntimeDeviceSpaceDevice device,
    required bool isLocal,
    required bool isExecuting,
  }) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final isSwitching = widget.switchingDeviceId == device.deviceId;
    final online = isExecuting
        ? (widget.bindingStatus?.ownerReachable ?? device.online)
        : device.online;
    final canSelect = widget.canSelectDevice(device);
    final connection = _connectionLabel(device, online);
    final status = <String>[
      if (isLocal) '本机',
      if (isExecuting) '当前执行设备',
      if (isSwitching) '正在切换',
      connection,
    ].join('，');

    final onlineColor = theme.brightness == Brightness.dark
        ? Colors.green.shade400
        : Colors.green.shade700;
    final isHovered = _hoveredDeviceId == device.deviceId && canSelect;
    final activeCall = _isExecutingCall(widget.processingState);

    final String dynamicLabel;
    final Color labelColor;
    if (!online) {
      dynamicLabel = '${device.deviceName} · 离线';
      labelColor = colors.error;
    } else if (isSwitching) {
      dynamicLabel = '交接 → ${device.deviceName}';
      labelColor = colors.primary;
    } else if (isHovered) {
      dynamicLabel = '切换 → ${device.deviceName}';
      labelColor = colors.primary;
    } else if (isExecuting &&
        (widget.displayMode == NodeLabelDisplayMode.taskOnly ||
            (widget.displayMode == NodeLabelDisplayMode.auto &&
                (_tickerPhase == 1 || activeCall)))) {
      dynamicLabel = _activeTaskText(widget.processingState);
      labelColor = colors.primary;
    } else {
      dynamicLabel = device.deviceName;
      labelColor = colors.onSurface;
    }

    return Semantics(
      container: true,
      label: '${device.deviceName}，$status',
      excludeSemantics: true,
      button: true,
      selected: isExecuting,
      enabled: canSelect,
      onTap: canSelect ? () => widget.onSelectDevice(device) : null,
      child: Tooltip(
        message: '${device.deviceName} · $connection',
        child: MouseRegion(
          onEnter: (_) => _setHovered(device.deviceId),
          onExit: (_) => _clearHovered(device.deviceId),
          child: InkWell(
            onTap: canSelect ? () => widget.onSelectDevice(device) : null,
            borderRadius: BorderRadius.circular(9),
            child: AnimatedContainer(
              duration: const Duration(milliseconds: 180),
              constraints: const BoxConstraints(minHeight: 30),
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
              decoration: BoxDecoration(
                color: !online
                    ? colors.error.withValues(alpha: 0.08)
                    : isExecuting || isSwitching
                    ? colors.primary.withValues(alpha: 0.13)
                    : colors.surfaceContainer,
                borderRadius: BorderRadius.circular(9),
                border: Border.all(
                  color: !online
                      ? colors.error.withValues(alpha: 0.45)
                      : isExecuting || isSwitching
                      ? colors.primary.withValues(alpha: 0.65)
                      : isHovered
                      ? colors.primary.withValues(alpha: 0.5)
                      : colors.outlineVariant.withValues(alpha: 0.55),
                ),
              ),
              child: Row(
                children: <Widget>[
                  Container(
                    width: 6,
                    height: 6,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      color: !online
                          ? colors.error
                          : isExecuting || isSwitching
                          ? colors.primary
                          : onlineColor,
                    ),
                  ),
                  const SizedBox(width: 6),
                  Icon(
                    devicePlatformIcon(device.platform),
                    size: 13.5,
                    color: isExecuting || isSwitching
                        ? colors.primary
                        : colors.onSurfaceVariant,
                  ),
                  const SizedBox(width: 6),
                  Expanded(
                    child: AnimatedSwitcher(
                      duration: const Duration(milliseconds: 200),
                      switchInCurve: Curves.easeOutCubic,
                      child: Align(
                        key: ValueKey<String>(
                          '${device.deviceId}-$dynamicLabel',
                        ),
                        alignment: AlignmentDirectional.centerStart,
                        child: Text(
                          dynamicLabel,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: theme.textTheme.bodySmall?.copyWith(
                            fontSize: 12,
                            color: labelColor,
                            fontWeight: isExecuting
                                ? FontWeight.w600
                                : FontWeight.w500,
                          ),
                        ),
                      ),
                    ),
                  ),
                  if (isSwitching) ...<Widget>[
                    const SizedBox(width: 5),
                    const M3LoadingIndicator(size: 13),
                  ] else if (isExecuting) ...<Widget>[
                    const SizedBox(width: 5),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 5,
                        vertical: 1,
                      ),
                      decoration: BoxDecoration(
                        color: colors.primary,
                        borderRadius: BorderRadius.circular(999),
                      ),
                      child: Text(
                        activeCall ? '调用' : '运行',
                        style: theme.textTheme.labelSmall?.copyWith(
                          fontSize: 10.5,
                          color: colors.onPrimary,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ] else if (isHovered) ...<Widget>[
                    const SizedBox(width: 5),
                    Text(
                      '切换',
                      style: theme.textTheme.labelSmall?.copyWith(
                        fontSize: 10.5,
                        color: colors.primary,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  void _setHovered(String deviceId) {
    if (_hoveredDeviceId == deviceId) return;
    setState(() => _hoveredDeviceId = deviceId);
    _syncAnimation();
  }

  void _clearHovered(String deviceId) {
    if (_hoveredDeviceId != deviceId) return;
    setState(() => _hoveredDeviceId = null);
    _syncAnimation();
  }

  String _connectionLabel(
    core_proxy.RuntimeDeviceSpaceDevice device,
    bool online,
  ) {
    if (!online) return '离线';
    if (device.relayHops != null && device.relayHops! > 0) {
      return '在线 · ${device.relayHops} 跳中继';
    }
    return '在线';
  }

  String _activeTaskText(core_proxy.InputProcessingState state) {
    return switch (state.tag) {
      'ExecutingTool' || 'ToolProgress' =>
        state.toolName.trim().isNotEmpty
            ? '执行 · ${state.toolName.trim()}'
            : '执行工具中',
      'ProcessingToolResult' =>
        state.toolName.trim().isNotEmpty
            ? '处理 · ${state.toolName.trim()}'
            : '整理工具结果',
      'Summarizing' => '压缩上下文中',
      'Receiving' => '模型流式输出中',
      'Connecting' => '连接模型中',
      'ExecutingPlan' => '执行任务计划中',
      'Processing' => '处理请求中',
      _ => '当前执行核心',
    };
  }
}

class _FiberLinkSpec {
  const _FiberLinkSpec({required this.state, required this.hasRelay});

  final FiberLinkState state;
  final bool hasRelay;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is _FiberLinkSpec &&
          state == other.state &&
          hasRelay == other.hasRelay;

  @override
  int get hashCode => Object.hash(state, hasRelay);
}

class _CurvedFiberBundlePainter extends CustomPainter {
  const _CurvedFiberBundlePainter({
    required this.links,
    required this.t,
    required this.primary,
    required this.secondary,
    required this.errorColor,
    required this.idleColor,
    required this.cardSurface,
  });

  final List<_FiberLinkSpec> links;
  final double t;
  final Color primary;
  final Color secondary;
  final Color errorColor;
  final Color idleColor;
  final Color cardSurface;

  @override
  void paint(Canvas canvas, Size size) {
    if (links.isEmpty || size.isEmpty) return;
    final start = Offset(0, size.height * 0.5);
    const gap = 6.0;
    final totalGaps = gap * math.max(0, links.length - 1);
    final itemHeight = (size.height - totalGaps) / links.length;

    for (var i = 0; i < links.length; i++) {
      final link = links[i];
      final yEnd = i * (itemHeight + gap) + itemHeight * 0.5;
      final end = Offset(size.width, yEnd);
      final c1 = Offset(size.width * 0.44, start.dy);
      final c2 = Offset(size.width * 0.56, end.dy);

      final path = Path()
        ..moveTo(start.dx, start.dy)
        ..cubicTo(c1.dx, c1.dy, c2.dx, c2.dy, end.dx, end.dy);

      switch (link.state) {
        case FiberLinkState.broken:
          _paintBrokenCurve(canvas, size, start, end);
        case FiberLinkState.handoff:
          _paintHandoffCurve(canvas, path, start, c1, c2, end);
        case FiberLinkState.bidirectional:
          _paintBidirectionalCurves(canvas, size, start, end);
        case FiberLinkState.preview:
          _paintLitCurve(canvas, path, glowAlpha: 0.28, animated: true);
        case FiberLinkState.active:
          _paintLitCurve(canvas, path, glowAlpha: 0.20, animated: false);
        case FiberLinkState.idle:
          canvas.drawPath(
            path,
            Paint()
              ..color = idleColor
              ..style = PaintingStyle.stroke
              ..strokeWidth = 1.6
              ..strokeCap = StrokeCap.round,
          );
      }

      if (link.hasRelay && link.state != FiberLinkState.broken) {
        final metric = path.computeMetrics().first;
        final tangent = metric.getTangentForOffset(metric.length * 0.5);
        if (tangent != null) {
          _paintRelayDiamond(
            canvas,
            tangent.position,
            lit: link.state != FiberLinkState.idle,
          );
        }
      }
    }
  }

  void _paintLitCurve(
    Canvas canvas,
    Path path, {
    required double glowAlpha,
    required bool animated,
  }) {
    canvas.drawPath(
      path,
      Paint()
        ..color = primary.withValues(alpha: glowAlpha)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 4.2
        ..strokeCap = StrokeCap.round,
    );
    canvas.drawPath(
      path,
      Paint()
        ..color = primary.withValues(alpha: 0.85)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.6
        ..strokeCap = StrokeCap.round,
    );
    if (animated) {
      _paintMovingPhoton(canvas, path, t, primary, radius: 2.3);
    }
  }

  void _paintHandoffCurve(
    Canvas canvas,
    Path mainPath,
    Offset start,
    Offset c1,
    Offset c2,
    Offset end,
  ) {
    final altPath = Path()
      ..moveTo(start.dx, start.dy)
      ..cubicTo(c1.dx * 0.85, start.dy + 10, c2.dx, end.dy - 10, end.dx, end.dy);

    canvas.drawPath(
      mainPath,
      Paint()
        ..color = primary.withValues(alpha: 0.26)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 5
        ..strokeCap = StrokeCap.round,
    );
    canvas.drawPath(
      mainPath,
      Paint()
        ..color = primary.withValues(alpha: 0.45)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.4,
    );
    canvas.drawPath(
      altPath,
      Paint()
        ..color = secondary.withValues(alpha: 0.30)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.2,
    );

    _paintFiberPulseSegment(canvas, mainPath, t, primary, width: 2.3);
    _paintFiberPulseSegment(
      canvas,
      altPath,
      (t + 0.32) % 1.0,
      secondary,
      width: 1.5,
    );
    _paintMovingPhoton(canvas, mainPath, t, primary, radius: 2.6);
  }

  void _paintBidirectionalCurves(
    Canvas canvas,
    Size size,
    Offset start,
    Offset end,
  ) {
    final upper = Path()
      ..moveTo(start.dx, start.dy - 2)
      ..cubicTo(
        size.width * 0.40,
        start.dy - 14,
        size.width * 0.60,
        end.dy - 14,
        end.dx,
        end.dy - 2,
      );
    final lower = Path()
      ..moveTo(end.dx, end.dy + 2)
      ..cubicTo(
        size.width * 0.60,
        end.dy + 14,
        size.width * 0.40,
        start.dy + 14,
        start.dx,
        start.dy + 2,
      );

    for (final entry in <(Path, Color, double)>[
      (upper, primary, t),
      (lower, secondary, (t + 0.35) % 1.0),
    ]) {
      canvas.drawPath(
        entry.$1,
        Paint()
          ..color = entry.$2.withValues(alpha: 0.24)
          ..style = PaintingStyle.stroke
          ..strokeWidth = 3.6
          ..strokeCap = StrokeCap.round,
      );
      _paintFiberPulseSegment(
        canvas,
        entry.$1,
        entry.$3,
        entry.$2,
        width: 1.7,
      );
      _paintMovingPhoton(canvas, entry.$1, entry.$3, entry.$2, radius: 2.1);
    }
  }

  void _paintBrokenCurve(Canvas canvas, Size size, Offset start, Offset end) {
    final mid = Offset.lerp(start, end, 0.5)!;
    final left = Path()
      ..moveTo(start.dx, start.dy)
      ..quadraticBezierTo(size.width * 0.24, start.dy + 6, mid.dx - 7, mid.dy + 5);
    final right = Path()
      ..moveTo(mid.dx + 7, mid.dy - 5)
      ..quadraticBezierTo(size.width * 0.76, end.dy - 6, end.dx, end.dy);
    final errPaint = Paint()
      ..color = errorColor.withValues(alpha: 0.85)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.7
      ..strokeCap = StrokeCap.round;
    canvas.drawPath(left, errPaint);
    canvas.drawPath(right, errPaint);
    canvas.drawLine(mid + const Offset(-3.5, -3.5), mid + const Offset(3.5, 3.5), errPaint);
    canvas.drawLine(mid + const Offset(3.5, -3.5), mid + const Offset(-3.5, 3.5), errPaint);
  }

  void _paintFiberPulseSegment(
    Canvas canvas,
    Path path,
    double phase,
    Color color, {
    required double width,
  }) {
    final metrics = path.computeMetrics().toList(growable: false);
    if (metrics.isEmpty || metrics.first.length <= 0) return;
    final metric = metrics.first;
    final head = metric.length * phase;
    final tail = math.max(0.0, head - metric.length * 0.32);
    final segment = metric.extractPath(tail, head);
    canvas.drawPath(
      segment,
      Paint()
        ..color = color
        ..style = PaintingStyle.stroke
        ..strokeWidth = width
        ..strokeCap = StrokeCap.round,
    );
  }

  void _paintMovingPhoton(
    Canvas canvas,
    Path path,
    double phase,
    Color color, {
    required double radius,
  }) {
    final metrics = path.computeMetrics().toList(growable: false);
    if (metrics.isEmpty || metrics.first.length <= 0) return;
    final metric = metrics.first;
    final tangent = metric.getTangentForOffset(metric.length * phase);
    if (tangent == null) return;
    canvas.drawCircle(
      tangent.position,
      radius * 1.8,
      Paint()..color = color.withValues(alpha: 0.28),
    );
    canvas.drawCircle(tangent.position, radius, Paint()..color = color);
  }

  void _paintRelayDiamond(Canvas canvas, Offset center, {required bool lit}) {
    const r = 4.2;
    final diamond = Path()
      ..moveTo(center.dx, center.dy - r)
      ..lineTo(center.dx + r, center.dy)
      ..lineTo(center.dx, center.dy + r)
      ..lineTo(center.dx - r, center.dy)
      ..close();
    canvas.drawPath(diamond, Paint()..color = cardSurface);
    canvas.drawPath(
      diamond,
      Paint()
        ..color = lit ? primary : idleColor
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.3,
    );
    canvas.drawCircle(center, 1.4, Paint()..color = lit ? primary : idleColor);
  }

  @override
  bool shouldRepaint(covariant _CurvedFiberBundlePainter oldDelegate) =>
      oldDelegate.t != t ||
      !listEquals(oldDelegate.links, links) ||
      oldDelegate.primary != primary ||
      oldDelegate.secondary != secondary;
}
