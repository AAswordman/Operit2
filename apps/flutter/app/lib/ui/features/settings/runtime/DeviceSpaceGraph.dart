// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as generated;
import '../../../../l10n/generated/app_localizations.dart';

part 'DeviceSpaceGraphLayout.dart';
part 'DeviceSpaceGraphPainter.dart';
part 'DeviceSpaceGraphSphere.dart';

/// One continuous scene: space membership unfolds into recorded connections.
class DeviceSpaceGraph extends StatefulWidget {
  const DeviceSpaceGraph({
    super.key,
    required this.topology,
    required this.onDisconnectDevice,
    required this.busy,
  });

  final generated.RuntimeDeviceSpaceTopology topology;
  final Future<generated.RuntimeDeviceSpaceTopology> Function(String deviceId)
  onDisconnectDevice;
  final bool busy;

  @override
  State<DeviceSpaceGraph> createState() => _DeviceSpaceGraphState();
}

class _DeviceSpaceGraphState extends State<DeviceSpaceGraph>
    with TickerProviderStateMixin {
  late final AnimationController _mode = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 860),
  )..addStatusListener((_) => _syncOrbitMotion());
  late final AnimationController _arrival = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 1100),
  );
  late final AnimationController _ambient = AnimationController(
    vsync: this,
    duration: const Duration(seconds: 12),
  );
  late final AnimationController _orbit = AnimationController(
    vsync: this,
    duration: const Duration(seconds: 28),
  );
  bool _topologyMode = false;
  bool _reduceMotion = false;
  bool _disconnecting = false;
  String? _selectedId;
  String? _hoveredId;
  String? _focusedId;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _reduceMotion = MediaQuery.disableAnimationsOf(context);
    if (_reduceMotion) {
      _ambient.stop();
      _arrival.value = 1;
      _mode.value = _topologyMode ? 1 : 0;
    } else {
      if (!_ambient.isAnimating) _ambient.repeat();
      if (!_arrival.isCompleted) _arrival.forward();
    }
    _syncOrbitMotion();
  }

  @override
  void didUpdateWidget(covariant DeviceSpaceGraph oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!widget.topology.devices.any(
      (device) => device.deviceId == _selectedId,
    )) {
      _selectedId = null;
    }
    if (!widget.topology.devices.any(
      (device) => device.deviceId == _hoveredId,
    )) {
      _hoveredId = null;
    }
    if (!widget.topology.devices.any(
      (device) => device.deviceId == _focusedId,
    )) {
      _focusedId = null;
    }
    _syncOrbitMotion();
  }

  @override
  void dispose() {
    _mode.dispose();
    _arrival.dispose();
    _ambient.dispose();
    _orbit.dispose();
    super.dispose();
  }

  void _toggleMode() {
    setState(() {
      _topologyMode = !_topologyMode;
      _selectedId = widget.topology.currentDeviceId;
    });
    if (_reduceMotion) {
      _mode.value = _topologyMode ? 1 : 0;
    } else {
      // Reversing an unfinished transition preserves the current positions.
      _mode.animateTo(_topologyMode ? 1 : 0, curve: Curves.easeInOutCubic);
    }
    _syncOrbitMotion();
  }

  void _syncOrbitMotion() {
    if (!mounted) return;
    final inspectingRemote =
        _selectedId != null && _selectedId != widget.topology.currentDeviceId;
    final paused =
        _reduceMotion ||
        _topologyMode ||
        _mode.isAnimating ||
        _hoveredId != null ||
        _focusedId != null ||
        inspectingRemote ||
        widget.topology.devices.length < 2;
    if (paused) {
      _orbit.stop();
    } else if (!_orbit.isAnimating) {
      _orbit.repeat();
    }
  }

  void _closeDetails() {
    if (_focusedId != null) FocusScope.of(context).unfocus();
    setState(() => _selectedId = null);
    _syncOrbitMotion();
  }

  void _selectDevice(String deviceId) {
    if (_selectedId == deviceId) {
      _closeDetails();
      return;
    }
    setState(() => _selectedId = deviceId);
    _syncOrbitMotion();
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final copy = _GraphCopy(context);
    final devices = widget.topology.devices;
    final online = devices.where((device) => device.online).length;
    final selected = devices.where((device) => device.deviceId == _selectedId);
    final selectedDevice = selected.isEmpty ? null : selected.single;
    final duration = _reduceMotion
        ? Duration.zero
        : const Duration(milliseconds: 260);

    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = constraints.maxWidth < 540;
        final inset = compact ? 12.0 : 20.0;
        final palette = _GraphPalette(scheme);
        return DecoratedBox(
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(24),
            border: Border.all(color: palette.cardBorder),
            gradient: LinearGradient(
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
              colors: <Color>[palette.cardStart, palette.cardEnd],
            ),
          ),
          child: ClipRRect(
            borderRadius: BorderRadius.circular(24),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                Padding(
                  padding: EdgeInsets.fromLTRB(
                    inset,
                    compact ? 12 : 18,
                    inset,
                    0,
                  ),
                  child: LayoutBuilder(
                    builder: (context, constraints) {
                      final title = Row(
                        mainAxisSize: MainAxisSize.min,
                        children: <Widget>[
                          AnimatedRotation(
                            turns: _topologyMode ? 0.25 : 0,
                            duration: duration,
                            child: Icon(
                              Icons.hub_outlined,
                              size: 19,
                              color: scheme.primary,
                            ),
                          ),
                          const SizedBox(width: 9),
                          Flexible(
                            child: AnimatedSwitcher(
                              duration: duration,
                              child: Text(
                                _topologyMode
                                    ? copy.topology
                                    : copy.relationships,
                                key: ValueKey(_topologyMode),
                                style: Theme.of(context).textTheme.titleSmall
                                    ?.copyWith(fontWeight: FontWeight.w700),
                              ),
                            ),
                          ),
                        ],
                      );
                      final stats = Wrap(
                        spacing: compact ? 8 : 12,
                        runSpacing: 5,
                        crossAxisAlignment: WrapCrossAlignment.center,
                        children: <Widget>[
                          Text(
                            compact
                                ? copy.deviceCount(devices.length)
                                : l10n.settingsRuntimeSpaceDeviceCount(
                                    devices.length,
                                  ),
                            style: Theme.of(context).textTheme.labelMedium
                                ?.copyWith(color: scheme.onSurfaceVariant),
                          ),
                          _GraphStatus(
                            color: scheme.primary,
                            text: '$online ${l10n.settingsRuntimePairedOnline}',
                          ),
                        ],
                      );
                      if (constraints.maxWidth < (copy.zh ? 260 : 400) ||
                          MediaQuery.textScalerOf(context).scale(14) > 18) {
                        return Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: <Widget>[
                            title,
                            SizedBox(height: compact ? 4 : 8),
                            stats,
                          ],
                        );
                      }
                      return Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: <Widget>[
                          Expanded(child: title),
                          const SizedBox(width: 12),
                          stats,
                        ],
                      );
                    },
                  ),
                ),
                LayoutBuilder(
                  builder: (context, constraints) {
                    final pageHeight = MediaQuery.sizeOf(context).height;
                    final viewport = Size(
                      constraints.maxWidth,
                      (constraints.maxWidth * (compact ? 0.78 : 0.58)).clamp(
                        pageHeight * 0.26,
                        pageHeight * 0.46,
                      ),
                    );
                    final layout = _GraphLayout.create(
                      viewport,
                      widget.topology,
                      metrics: _GraphNodeMetrics.of(
                        viewport,
                        compact: compact,
                      ),
                    );
                    return SizedBox(
                      height: viewport.height,
                      width: viewport.width,
                      child: ClipRect(
                        child: FittedBox(
                          fit: BoxFit.contain,
                          child: SizedBox.fromSize(
                            size: layout.size,
                            child: _buildScene(context, layout),
                          ),
                        ),
                      ),
                    );
                  },
                ),
                AnimatedSize(
                  duration: duration,
                  curve: Curves.easeOutCubic,
                  alignment: Alignment.topCenter,
                  child: AnimatedSwitcher(
                    duration: duration,
                    child: selectedDevice == null
                        ? _GraphFooter(
                            key: ValueKey(_topologyMode),
                            topologyMode: _topologyMode,
                            empty: devices.length == 1,
                            reduceMotion: _reduceMotion,
                            compact: compact,
                          )
                        : _GraphDeviceDetails(
                            key: ValueKey(selectedDevice.deviceId),
                            device: selectedDevice,
                            compact: compact,
                            topology: widget.topology,
                            busy: widget.busy || _disconnecting,
                            onClose: _closeDetails,
                            onDisconnect: () => _disconnect(selectedDevice),
                          ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }

  Widget _buildScene(BuildContext context, _GraphLayout layout) {
    final motion = Listenable.merge([_mode, _orbit]);
    // Reuse interaction subtrees; transforms and material track the same phase.
    final nodes = <String, Widget>{
      for (var index = 0; index < layout.devices.length; index++)
        layout.devices[index].deviceId: FocusTraversalOrder(
          order: NumericFocusOrder(index.toDouble()),
          child: RepaintBoundary(child: _buildNode(layout, index, motion)),
        ),
    };
    return FocusTraversalGroup(
      policy: OrderedTraversalPolicy(),
      child: AnimatedBuilder(
        animation: Listenable.merge([_mode, _arrival, _orbit]),
        builder: (context, child) {
          final frames = layout.frames(_mode.value, _orbit.value);
          final drawOrder = layout.devices.toList()
            ..sort((first, second) {
              final depth = frames[first.deviceId]!.depth.compareTo(
                frames[second.deviceId]!.depth,
              );
              return depth != 0
                  ? depth
                  : first.deviceId.compareTo(second.deviceId);
            });
          return Stack(
            clipBehavior: Clip.none,
            children: <Widget>[
              Positioned.fill(
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onTap: _closeDetails,
                  child: RepaintBoundary(
                    child: CustomPaint(
                      painter: _GraphPainter(
                        layout: layout,
                        frames: frames,
                        topology: widget.topology,
                        scheme: Theme.of(context).colorScheme,
                        mode: _mode.value,
                        arrival: _arrival.value,
                        phase: _ambient,
                        selectedId: _selectedId,
                        reduceMotion: _reduceMotion,
                      ),
                    ),
                  ),
                ),
              ),
              for (final device in drawOrder)
                _positionNode(
                  device.deviceId,
                  frames[device.deviceId]!,
                  layout.metrics,
                  layout.devices.indexOf(device),
                  nodes[device.deviceId]!,
                ),
            ],
          );
        },
      ),
    );
  }

  Widget _positionNode(
    String id,
    _GraphNodeFrame frame,
    _GraphNodeMetrics metrics,
    int index,
    Widget child,
  ) {
    final start = math.min(index * 0.065, 0.4);
    final entry = Curves.easeOutCubic.transform(
      ((_arrival.value - start) / (1 - start)).clamp(0.0, 1.0),
    );
    return Positioned(
      key: ValueKey(id),
      left: frame.center.dx - metrics.extent / 2,
      top: frame.center.dy - metrics.extent / 2 + (1 - entry) * 18 * metrics.scale,
      width: metrics.extent,
      height: metrics.extent,
      child: Opacity(
        opacity: entry * frame.opacity,
        child: Transform.scale(
          scale: (0.8 + 0.2 * entry) * frame.scale,
          child: child,
        ),
      ),
    );
  }

  Widget _buildNode(_GraphLayout layout, int index, Listenable motion) {
    final device = layout.devices[index];
    final id = device.deviceId;
    final current = id == widget.topology.currentDeviceId;
    return _GraphDeviceNode(
      device: device,
      metrics: layout.metrics,
      motion: motion,
      frame: () => layout.frameAt(index, _mode.value, _orbit.value),
      current: current,
      selected: _selectedId == id,
      topologyMode: _topologyMode,
      reduceMotion: _reduceMotion,
      onHover: (value) {
        if (value) {
          _hoveredId = id;
        } else if (_hoveredId == id) {
          _hoveredId = null;
        }
        _syncOrbitMotion();
      },
      onFocus: (value) {
        if (value) {
          _focusedId = id;
        } else if (_focusedId == id) {
          _focusedId = null;
        }
        _syncOrbitMotion();
      },
      onTap: current ? _toggleMode : () => _selectDevice(id),
    );
  }

  Future<void> _disconnect(generated.RuntimeDeviceSpaceDevice device) async {
    if (_disconnecting || widget.busy) return;
    final l10n = AppLocalizations.of(context)!;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l10n.settingsRuntimeDisconnectConnectionTitle),
        content: Text(
          l10n.settingsRuntimeDisconnectConnectionMessage(device.deviceName),
        ),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(MaterialLocalizations.of(context).cancelButtonLabel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(l10n.settingsRuntimeDisconnectConnection),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;
    setState(() => _disconnecting = true);
    try {
      await widget.onDisconnectDevice(device.deviceId);
      if (mounted) _closeDetails();
    } catch (error) {
      if (!mounted) return;
      await showDialog<void>(
        context: context,
        builder: (context) => AlertDialog(
          title: Text(l10n.settingsRuntimeDisconnectConnectionFailed),
          content: SelectableText(error.toString()),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: Text(MaterialLocalizations.of(context).closeButtonLabel),
            ),
          ],
        ),
      );
    } finally {
      if (mounted) setState(() => _disconnecting = false);
    }
  }
}

class _GraphDeviceNode extends StatefulWidget {
  const _GraphDeviceNode({
    required this.device,
    required this.metrics,
    required this.motion,
    required this.frame,
    required this.current,
    required this.selected,
    required this.topologyMode,
    required this.reduceMotion,
    required this.onHover,
    required this.onFocus,
    required this.onTap,
  });

  final generated.RuntimeDeviceSpaceDevice device;
  final _GraphNodeMetrics metrics;
  final Listenable motion;
  final ValueGetter<_GraphNodeFrame> frame;
  final bool current;
  final bool selected;
  final bool topologyMode;
  final bool reduceMotion;
  final ValueChanged<bool> onHover;
  final ValueChanged<bool> onFocus;
  final VoidCallback onTap;

  @override
  State<_GraphDeviceNode> createState() => _GraphDeviceNodeState();
}

class _GraphDeviceNodeState extends State<_GraphDeviceNode>
    with SingleTickerProviderStateMixin {
  final GlobalKey<TooltipState> _tooltip = GlobalKey<TooltipState>();
  late final AnimationController _elevation = AnimationController(
    vsync: this,
    value: widget.selected ? 1 : 0.35,
    duration: const Duration(milliseconds: 220),
  );
  bool _hovered = false;
  bool _focused = false;
  bool _pressed = false;

  @override
  void didUpdateWidget(covariant _GraphDeviceNode oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.selected != widget.selected ||
        oldWidget.reduceMotion != widget.reduceMotion) {
      _updateElevation();
    }
  }

  void _updateElevation() {
    final target = _pressed
        ? 0.0
        : (_hovered || _focused || widget.selected ? 1.0 : 0.35);
    if (widget.reduceMotion) {
      _elevation.value = target;
    } else {
      _elevation.animateTo(target, curve: Curves.easeOutCubic);
    }
  }

  @override
  void dispose() {
    _elevation.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final copy = _GraphCopy(context);
    final current = widget.current;
    final metrics = widget.metrics;
    final highlighted = _hovered || _focused || widget.selected;
    final active = current || widget.device.online;
    final accent = active ? scheme.primary : scheme.outline;
    final duration = widget.reduceMotion
        ? Duration.zero
        : const Duration(milliseconds: 200);
    final label = current
        ? '${l10n.settingsRuntimeCurrentDevice} · ${widget.device.deviceName}'
        : widget.device.deviceName;
    final action = widget.topologyMode
        ? copy.showRelationships
        : copy.showTopology;
    final status = widget.device.online
        ? l10n.settingsRuntimePairedOnline
        : l10n.settingsRuntimePairedOffline;
    final diameter = current ? metrics.currentDiameter : metrics.remoteDiameter;

    return Semantics(
      button: true,
      selected: widget.selected,
      label: '$label, $status',
      hint: current ? action : copy.deviceDetails,
      child: Tooltip(
        key: _tooltip,
        message:
            '$label\n$status · ${widget.device.platform}${current ? '\n$action' : ''}',
        excludeFromSemantics: true,
        triggerMode: TooltipTriggerMode.manual,
        waitDuration: const Duration(milliseconds: 180),
        preferBelow: false,
        child: AnimatedScale(
          duration: duration,
          curve: Curves.easeOutCubic,
          scale: _pressed ? 0.95 : (highlighted ? 1.055 : 1),
          child: Material(
            type: MaterialType.transparency,
            child: InkWell(
              onTap: widget.onTap,
              onHover: (value) {
                setState(() => _hovered = value);
                _updateElevation();
                widget.onHover(value);
              },
              onFocusChange: (value) {
                setState(() => _focused = value);
                _updateElevation();
                widget.onFocus(value);
                if (value) _tooltip.currentState?.ensureTooltipVisible();
              },
              onHighlightChanged: (value) {
                setState(() => _pressed = value);
                _updateElevation();
              },
              customBorder: const CircleBorder(),
              hoverColor: Colors.transparent,
              focusColor: Colors.transparent,
              splashFactory: NoSplash.splashFactory,
              highlightColor: Colors.transparent,
              child: Center(
                child: Stack(
                  clipBehavior: Clip.none,
                  children: <Widget>[
                    _GraphDeviceSphere(
                      diameter: diameter,
                      icon: _deviceIcon(widget.device.platform),
                      iconSize: current
                          ? metrics.currentIcon
                          : metrics.remoteIcon,
                      current: current,
                      online: widget.device.online,
                      motion: widget.motion,
                      elevation: _elevation,
                      frame: widget.frame,
                    ),
                    if (current)
                      Positioned(
                        right: -3 * metrics.scale,
                        bottom: -3 * metrics.scale,
                        child: AnimatedRotation(
                          turns: widget.topologyMode ? 0.5 : 0,
                          duration: duration,
                          child: Container(
                            padding: EdgeInsets.all(4 * metrics.scale),
                            decoration: BoxDecoration(
                              color: scheme.primary,
                              shape: BoxShape.circle,
                              border: Border.all(
                                color: scheme.surface,
                                width: 3 * metrics.scale,
                              ),
                            ),
                            child: Icon(
                              Icons.swap_horiz_rounded,
                              size: 14 * metrics.scale,
                              color: scheme.onPrimary,
                            ),
                          ),
                        ),
                      )
                    else
                      Positioned(
                        right: 1 * metrics.scale,
                        bottom: 1 * metrics.scale,
                        child: Container(
                          width: 11 * metrics.scale,
                          height: 11 * metrics.scale,
                          decoration: BoxDecoration(
                            color: accent,
                            shape: BoxShape.circle,
                            border: Border.all(
                              color: scheme.surface,
                              width: 2.5 * metrics.scale,
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
      ),
    );
  }
}

class _GraphStatus extends StatelessWidget {
  const _GraphStatus({required this.color, required this.text});

  final Color color;
  final String text;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Container(
          width: 5,
          height: 5,
          decoration: BoxDecoration(color: color, shape: BoxShape.circle),
        ),
        const SizedBox(width: 6),
        Text(
          text,
          style: Theme.of(context).textTheme.labelSmall?.copyWith(color: color),
        ),
      ],
    );
  }
}

class _GraphFooter extends StatelessWidget {
  const _GraphFooter({
    super.key,
    required this.topologyMode,
    required this.empty,
    required this.reduceMotion,
    required this.compact,
  });

  final bool topologyMode;
  final bool empty;
  final bool reduceMotion;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final copy = _GraphCopy(context);
    return Padding(
      padding: compact
          ? const EdgeInsets.fromLTRB(12, 0, 12, 10)
          : const EdgeInsets.fromLTRB(20, 0, 20, 18),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Divider(
            height: 1,
            color: _GraphPalette(scheme).divider,
          ),
          SizedBox(height: compact ? 8 : 12),
          Row(
            children: <Widget>[
              Icon(
                topologyMode ? Icons.route_outlined : Icons.touch_app_outlined,
                size: 15,
                color: scheme.primary,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  empty
                      ? copy.empty
                      : compact
                      ? (topologyMode
                            ? copy.compactTopologyHint
                            : copy.compactRelationshipsHint)
                      : (topologyMode
                            ? copy.topologyHint
                            : copy.relationshipsHint),
                  style: Theme.of(context).textTheme.labelSmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
              ),
              const SizedBox(width: 12),
              for (var i = 0; i < 2; i++)
                AnimatedContainer(
                  duration: reduceMotion
                      ? Duration.zero
                      : const Duration(milliseconds: 300),
                  margin: const EdgeInsets.only(left: 4),
                  width: (topologyMode ? i == 1 : i == 0) ? 18 : 5,
                  height: 5,
                  decoration: BoxDecoration(
                    color: (topologyMode ? i == 1 : i == 0)
                        ? scheme.primary
                        : scheme.outlineVariant,
                    borderRadius: BorderRadius.circular(5),
                  ),
                ),
            ],
          ),
        ],
      ),
    );
  }
}

class _GraphDeviceDetails extends StatelessWidget {
  const _GraphDeviceDetails({
    super.key,
    required this.device,
    required this.compact,
    required this.topology,
    required this.busy,
    required this.onClose,
    required this.onDisconnect,
  });

  final generated.RuntimeDeviceSpaceDevice device;
  final bool compact;
  final generated.RuntimeDeviceSpaceTopology topology;
  final bool busy;
  final VoidCallback onClose;
  final VoidCallback onDisconnect;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final copy = _GraphCopy(context);
    final peers = {
      for (final member in topology.devices) member.deviceId: member,
    };
    final connections = topology.connections
        .where(
          (edge) =>
              edge.firstDeviceId == device.deviceId ||
              edge.secondDeviceId == device.deviceId,
        )
        .toList();
    final current = device.deviceId == topology.currentDeviceId;
    final direct =
        !current &&
        connections.any(
          (edge) =>
              edge.firstDeviceId == topology.currentDeviceId ||
              edge.secondDeviceId == topology.currentDeviceId,
        );
    return Container(
      margin: compact
          ? const EdgeInsets.fromLTRB(8, 0, 8, 8)
          : const EdgeInsets.fromLTRB(12, 0, 12, 12),
      padding: compact
          ? const EdgeInsets.fromLTRB(10, 6, 4, 8)
          : const EdgeInsets.fromLTRB(14, 10, 8, 12),
      decoration: BoxDecoration(
        color: _GraphPalette(scheme).detailsFill,
        borderRadius: BorderRadius.circular(18),
        border: Border.all(color: _GraphPalette(scheme).detailsBorder),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            children: <Widget>[
              Icon(
                _deviceIcon(device.platform),
                color: scheme.primary,
                size: 22,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      current
                          ? '${l10n.settingsRuntimeCurrentDevice} · ${device.deviceName}'
                          : device.deviceName,
                      style: Theme.of(context).textTheme.labelLarge?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    Text(
                      [
                        device.online
                            ? l10n.settingsRuntimePairedOnline
                            : l10n.settingsRuntimePairedOffline,
                        if (!current && device.online)
                          device.relayHops == null
                              ? l10n.deviceSpaceDirectLink
                              : l10n.deviceSpaceRelayHops(device.relayHops!),
                        device.platform,
                        if (device.coreVersion != null) device.coreVersion!,
                      ].join(' · '),
                      style: Theme.of(context).textTheme.labelSmall?.copyWith(
                        color: scheme.onSurfaceVariant,
                      ),
                    ),
                    if ((device.relayPath ?? const <String>[]).isNotEmpty)
                      Text(
                        '${l10n.deviceSpaceRelayPath}: ${[
                          l10n.deviceSpaceLocalDevice,
                          for (final hop in device.relayPath!)
                            peers[hop]?.deviceName ?? hop,
                        ].join(' → ')}',
                        style: Theme.of(context).textTheme.labelSmall?.copyWith(
                          color: scheme.onSurfaceVariant,
                        ),
                      ),
                  ],
                ),
              ),
              if (direct)
                IconButton(
                  tooltip: l10n.settingsRuntimeDisconnectConnection,
                  onPressed: busy ? null : onDisconnect,
                  icon: const Icon(Icons.link_off_rounded, size: 19),
                  color: scheme.error,
                ),
              IconButton(
                tooltip: MaterialLocalizations.of(context).closeButtonTooltip,
                onPressed: onClose,
                icon: const Icon(Icons.close_rounded, size: 18),
              ),
            ],
          ),
          if (connections.isEmpty)
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: Text(
                copy.noConnections,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
          for (final edge in connections)
            Padding(
              padding: const EdgeInsets.only(top: 9, right: 6),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text.rich(
                    TextSpan(
                      children: <InlineSpan>[
                        TextSpan(
                          text:
                              '${peers[edge.firstDeviceId == device.deviceId ? edge.secondDeviceId : edge.firstDeviceId]!.deviceName}  ·  ',
                        ),
                        TextSpan(
                          text: copy.connectionStatus(edge.status),
                          style: TextStyle(
                            color: _connectionColor(edge.status, scheme),
                          ),
                        ),
                      ],
                    ),
                    style: Theme.of(context).textTheme.labelMedium,
                  ),
                  if (edge.reason.isNotEmpty)
                    Text(
                      edge.reason,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: scheme.onSurfaceVariant,
                      ),
                    ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}

/// Small scene-specific copy follows the surrounding settings' locale.
class _GraphCopy {
  _GraphCopy(BuildContext context)
    : zh = Localizations.localeOf(context).languageCode == 'zh';

  final bool zh;
  String get relationships => zh ? '设备关系' : 'Device relationships';
  String get topology => zh ? '连接拓扑' : 'Connection topology';
  String get showTopology => zh ? '点击展开拓扑' : 'Explore connections';
  String get showRelationships => zh ? '点击回到关系' : 'Back to relationships';
  String get deviceDetails => zh ? '查看设备与连接详情' : 'View device and connections';
  String get relationshipsHint => zh
      ? '悬停或点击查看设备 · 点击中心展开拓扑'
      : 'Hover or select a device · Select the center to explore connections';
  String get topologyHint => zh
      ? '实线直连在线 · 虚线已宣告 · 点击当前设备返回'
      : 'Solid: direct online · Dashed: announced · Select your device to return';
  String get empty => zh
      ? '空间已就绪，连接另一台设备，让协作从这里开始'
      : 'Your space is ready. Connect another device to get started.';
  String deviceCount(int count) => zh ? '$count 台设备' : '$count devices';
  String get compactRelationshipsHint => zh
      ? '点击查看设备 · 点击中心展开拓扑'
      : 'Tap a device for details · Tap the center for connections';
  String get compactTopologyHint =>
      zh ? '实线直连在线 · 点击当前设备返回' : 'Solid: direct online · Select your device to return';
  String get noConnections => zh ? '尚无连接记录' : 'No recorded connections';

  String connectionStatus(
    generated.RuntimeDeviceSpaceConnectionStatus status,
  ) => switch (status) {
    generated.RuntimeDeviceSpaceConnectionStatus.online => zh ? '在线' : 'Online',
    generated.RuntimeDeviceSpaceConnectionStatus.offline =>
      zh ? '离线' : 'Offline',
    generated.RuntimeDeviceSpaceConnectionStatus.versionMismatch =>
      zh ? 'Core 版本不匹配' : 'Core version mismatch',
    generated.RuntimeDeviceSpaceConnectionStatus.announced =>
      zh ? '已宣告' : 'Announced',
    generated.RuntimeDeviceSpaceConnectionStatus.unknown =>
      zh ? '状态未知' : 'Unknown',
  };
}

IconData _deviceIcon(String platform) {
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

Color _connectionColor(
  generated.RuntimeDeviceSpaceConnectionStatus status,
  ColorScheme scheme,
) => switch (status) {
  generated.RuntimeDeviceSpaceConnectionStatus.online => scheme.primary,
  generated.RuntimeDeviceSpaceConnectionStatus.offline => scheme.outline,
  generated.RuntimeDeviceSpaceConnectionStatus.versionMismatch => scheme.error,
  generated.RuntimeDeviceSpaceConnectionStatus.announced => scheme.secondary,
  generated.RuntimeDeviceSpaceConnectionStatus.unknown => scheme.tertiary,
};
