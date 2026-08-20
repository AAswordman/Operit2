// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;
import 'package:flutter/material.dart';

import '../../../../core/bridge/PlatformCoreProxy.dart';
import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as generated;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/DeviceSpaceDiscoveryPanel.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

class RuntimeSettingsPanel extends StatefulWidget {
  const RuntimeSettingsPanel({super.key, this.embedded = false});

  final bool embedded;

  @override
  State<RuntimeSettingsPanel> createState() => _RuntimeSettingsPanelState();
}

class _RuntimeSettingsPanelState extends State<RuntimeSettingsPanel> {
  bool _busy = false;
  String? _connectionMessage;
  bool _connectionFailed = false;
  generated.CoreSpace? _currentDeviceSpace;
  Map<String, _PairedRemoteProbeState> _pairedRemoteStates =
      <String, _PairedRemoteProbeState>{};
  Map<String, generated.RuntimePairedDevice> _pairedDevices =
      <String, generated.RuntimePairedDevice>{};
  int _pairedRemoteProbeGeneration = 0;
  StreamSubscription<Map<String, generated.RuntimePairedDevice>>?
  _pairedDevicesSubscription;

  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(coreProxy: platformCoreProxy),
  );

  @override
  void initState() {
    super.initState();
    unawaited(_refreshCurrentDeviceSpace());
    _watchPairedDevices();
  }

  @override
  void dispose() {
    final pairedDevicesSubscription = _pairedDevicesSubscription;
    if (pairedDevicesSubscription != null) {
      unawaited(pairedDevicesSubscription.cancel());
    }
    super.dispose();
  }

  /// Subscribes to pairing changes produced by both connection directions.
  void _watchPairedDevices() {
    _pairedDevicesSubscription = _clients.runtimeRemoteLinkService
        .pairedDevicesFlow()
        .listen(
          (devices) => unawaited(_applyPairedDevices(devices)),
          onError: (Object error, StackTrace stackTrace) {
            if (!mounted) {
              return;
            }
            setState(() {
              _connectionMessage = error.toString();
              _connectionFailed = true;
            });
          },
        );
  }

  /// Reads the synchronized device space projection from the current device.
  Future<void> _refreshCurrentDeviceSpace() async {
    try {
      final deviceSpace = await _clients.runtimeRemoteLinkService.deviceSpace();
      if (mounted) {
        setState(() => _currentDeviceSpace = deviceSpace);
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = error.toString();
          _connectionFailed = true;
        });
      }
    }
  }

  /// Opens the synchronized direct-connection graph for the current device space.
  Future<void> _openDeviceSpaceTopology() async {
    final currentDeviceSpace = _currentDeviceSpace;
    if (currentDeviceSpace == null) {
      throw StateError('current device space is not loaded');
    }
    setState(() => _busy = true);
    try {
      final topology = await _clients.runtimeRemoteLinkService
          .deviceSpaceTopology();
      if (!mounted) {
        return;
      }
      await _DeviceSpaceTopologyDialog.show(
        context,
        spaceName: currentDeviceSpace.spaceName,
        topology: topology,
        onDisconnectDevice: _disconnectDeviceSpaceConnection,
      );
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = error.toString();
          _connectionFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Disconnects one direct device-space connection and returns the refreshed topology.
  Future<generated.RuntimeDeviceSpaceTopology> _disconnectDeviceSpaceConnection(
    String deviceId,
  ) async {
    await _clients.runtimeRemoteLinkService.disconnectDeviceSpaceConnection(
      deviceId: deviceId,
    );
    return _clients.runtimeRemoteLinkService.deviceSpaceTopology();
  }

  /// Applies one paired-device snapshot and refreshes direct connection states.
  Future<void> _applyPairedDevices(
    Map<String, generated.RuntimePairedDevice> devices,
  ) async {
    final generation = ++_pairedRemoteProbeGeneration;
    if (!mounted || generation != _pairedRemoteProbeGeneration) {
      return;
    }
    setState(() {
      _pairedDevices = devices;
      _pairedRemoteStates = <String, _PairedRemoteProbeState>{
        for (final deviceId in devices.keys)
          deviceId: _PairedRemoteProbeState.checking,
      };
    });
    final results = await Future.wait(
      devices.keys.map((deviceId) async {
        final state = await _probePairedDevice(deviceId);
        return MapEntry(deviceId, state);
      }),
    );
    if (!mounted || generation != _pairedRemoteProbeGeneration) {
      return;
    }
    setState(() {
      _pairedRemoteStates = Map<String, _PairedRemoteProbeState>.fromEntries(
        results,
      );
    });
  }

  /// Reads the active direct-connection state for one paired device.
  Future<_PairedRemoteProbeState> _probePairedDevice(String deviceId) async {
    try {
      final online = await _clients.runtimeRemoteLinkService.pairedDeviceOnline(
        deviceId: deviceId,
      );
      return online
          ? _PairedRemoteProbeState.online
          : _PairedRemoteProbeState.offline;
    } catch (_) {
      return _PairedRemoteProbeState.offline;
    }
  }

  /// Removes every local pairing record associated with one device.
  Future<void> _deletePairedDevice(String deviceId) async {
    setState(() => _busy = true);
    try {
      await _clients.runtimeRemoteLinkService.removePairedDevice(
        deviceId: deviceId,
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Persists the explicit Link carrier selected for one outbound paired device.
  Future<void> _setPairedDeviceTransport(
    generated.RuntimePairedDevice device,
    generated.LinkTransportPreference transport,
  ) async {
    final name = device.outboundSessionName;
    if (name == null) {
      return;
    }
    setState(() => _busy = true);
    try {
      await _clients.runtimeRemoteLinkService.setPairedRemoteTransport(
        name: name,
        transport: transport,
      );
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = error.toString();
          _connectionFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Prompts for and persists a new name for the current device space.
  Future<void> _renameCurrentDeviceSpace() async {
    final currentDeviceSpace = _currentDeviceSpace;
    if (currentDeviceSpace == null) {
      return;
    }
    final spaceName = await _RenameCurrentDeviceSpaceDialog.show(
      context,
      initialName: currentDeviceSpace.spaceName,
    );
    if (spaceName == null) {
      return;
    }
    setState(() => _busy = true);
    try {
      final renamed = await _clients.runtimeRemoteLinkService.renameDeviceSpace(
        spaceName: spaceName,
      );
      if (mounted) {
        setState(() => _currentDeviceSpace = renamed);
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = error.toString();
          _connectionFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Leaves the shared device space after an explicit user confirmation.
  Future<void> _leaveCurrentDeviceSpace() async {
    final l10n = AppLocalizations.of(context)!;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: Text(l10n.settingsRuntimeLeaveSpaceTitle),
        content: Text(l10n.settingsRuntimeLeaveSpaceDescription),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: Text(l10n.settingsRuntimeLeaveSpaceConfirm),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) {
      return;
    }
    setState(() => _busy = true);
    try {
      final deviceSpace = await _clients.runtimeRemoteLinkService
          .leaveDeviceSpace();
      if (mounted) {
        setState(() {
          _currentDeviceSpace = deviceSpace;
          _connectionMessage = null;
          _connectionFailed = false;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = error.toString();
          _connectionFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Confirms and joins the device space exposed by an existing paired device.
  Future<void> _offerJoiningExistingPairedDeviceSpace(
    generated.RuntimePairedDevice device,
  ) async {
    final sessionName = device.outboundSessionName;
    if (sessionName == null) {
      throw StateError('joining a device space requires an outbound pairing');
    }
    final deviceInfo = device.deviceInfo;
    setState(() => _busy = true);
    try {
      final joined = await confirmAndJoinPairedDeviceSpace(
        context: context,
        clients: _clients,
        sessionName: sessionName,
        deviceName: '${deviceInfo.platform}-${deviceInfo.model}',
      );
      if (mounted && joined != null) {
        setState(() {
          _currentDeviceSpace = joined;
          _connectionMessage = null;
          _connectionFailed = false;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _connectionMessage = error.toString();
          _connectionFailed = true;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Applies a device space returned by the shared discovery workflow.
  Future<void> _handleJoinedDeviceSpace(generated.CoreSpace deviceSpace) async {
    if (!mounted) {
      return;
    }
    setState(() {
      _currentDeviceSpace = deviceSpace;
      _connectionMessage = null;
      _connectionFailed = false;
    });
  }

  /// Mirrors discovery activity so the surrounding settings actions stay stable.
  void _handleDiscoveryBusyChanged(bool busy) {
    if (mounted) {
      setState(() => _busy = busy);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final children = <Widget>[
      _SectionCard(
        title: l10n.settingsRuntimeCurrentSpace,
        children: <Widget>[
          _CurrentDeviceSpaceLine(
            deviceSpace: _currentDeviceSpace,
            busy: _busy,
            onViewTopology: _openDeviceSpaceTopology,
            onRename: _renameCurrentDeviceSpace,
            onLeave: _leaveCurrentDeviceSpace,
          ),
          if (_connectionMessage != null) ...<Widget>[
            const SizedBox(height: 8),
            _InlineStatus(
              message: _connectionMessage!,
              failed: _connectionFailed,
            ),
          ],
        ],
      ),
      _SectionCard(
        title: l10n.settingsRuntimeRemoteTitle,
        children: <Widget>[
          Text(
            l10n.settingsRuntimeRemoteDescription,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 8),
          _PairedDeviceList(
            devices: _pairedDevices,
            busy: _busy,
            states: _pairedRemoteStates,
            currentMemberIds:
                _currentDeviceSpace?.members.toSet() ?? <String>{},
            onJoin: _offerJoiningExistingPairedDeviceSpace,
            onDelete: _deletePairedDevice,
            onTransportChanged: _setPairedDeviceTransport,
          ),
        ],
      ),
      _SectionCard(
        title: l10n.settingsRuntimeDiscoverSpaces,
        children: <Widget>[
          DeviceSpaceDiscoveryPanel(
            clients: _clients,
            enabled: !_busy,
            onJoined: _handleJoinedDeviceSpace,
            onBusyChanged: _handleDiscoveryBusyChanged,
          ),
        ],
      ),
    ];
    if (widget.embedded) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: children,
      );
    }
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
      children: children,
    );
  }
}

enum _PairedRemoteProbeState { checking, online, offline }

class _SectionCard extends StatelessWidget {
  const _SectionCard({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 10),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                title,
                style: SettingsControlStyles.sectionTitleTextStyle(context),
              ),
              const SizedBox(height: 8),
              ...children,
            ],
          ),
        ),
      ),
    );
  }
}

class _CurrentDeviceSpaceLine extends StatelessWidget {
  const _CurrentDeviceSpaceLine({
    required this.deviceSpace,
    required this.busy,
    required this.onViewTopology,
    required this.onRename,
    required this.onLeave,
  });

  final generated.CoreSpace? deviceSpace;
  final bool busy;
  final VoidCallback onViewTopology;
  final VoidCallback onRename;
  final VoidCallback onLeave;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final currentDeviceSpace = deviceSpace;
    if (currentDeviceSpace == null) {
      return const SizedBox(
        height: 48,
        child: Align(
          alignment: Alignment.centerLeft,
          child: M3LoadingIndicator(size: 20),
        ),
      );
    }
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Icon(Icons.hub_outlined, color: colorScheme.primary),
        const SizedBox(width: 10),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                currentDeviceSpace.spaceName,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(
                  context,
                ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w800),
              ),
              const SizedBox(height: 4),
              Text(
                l10n.settingsRuntimeSpaceId(currentDeviceSpace.spaceId),
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 2),
              Semantics(
                button: true,
                label: l10n.settingsRuntimeViewSpaceTopology,
                child: InkWell(
                  onTap: busy ? null : onViewTopology,
                  child: Padding(
                    padding: const EdgeInsets.symmetric(vertical: 4),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        Flexible(
                          child: Text(
                            l10n.settingsRuntimeSpaceDeviceCount(
                              currentDeviceSpace.members.length,
                            ),
                            style: Theme.of(context).textTheme.bodySmall
                                ?.copyWith(
                                  color: colorScheme.primary,
                                  fontWeight: FontWeight.w700,
                                ),
                          ),
                        ),
                        const SizedBox(width: 2),
                        Icon(
                          Icons.chevron_right_rounded,
                          size: 18,
                          color: colorScheme.primary,
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
        Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            IconButton(
              tooltip: l10n.settingsRuntimeRenameSpace,
              icon: const Icon(Icons.edit_outlined),
              onPressed: busy ? null : onRename,
            ),
            IconButton(
              tooltip: l10n.settingsRuntimeLeaveSpace,
              icon: const Icon(Icons.logout_outlined),
              onPressed: busy || currentDeviceSpace.members.length <= 1
                  ? null
                  : onLeave,
            ),
          ],
        ),
      ],
    );
  }
}

class _DeviceSpaceTopologyDialog extends StatelessWidget {
  const _DeviceSpaceTopologyDialog({
    required this.spaceName,
    required this.topology,
    required this.onDisconnectDevice,
  });

  final String spaceName;
  final generated.RuntimeDeviceSpaceTopology topology;
  final Future<generated.RuntimeDeviceSpaceTopology> Function(String deviceId)
  onDisconnectDevice;

  /// Opens the device-space topology as one responsive modal surface.
  static Future<void> show(
    BuildContext context, {
    required String spaceName,
    required generated.RuntimeDeviceSpaceTopology topology,
    required Future<generated.RuntimeDeviceSpaceTopology> Function(
      String deviceId,
    )
    onDisconnectDevice,
  }) {
    return showDialog<void>(
      context: context,
      builder: (context) => _DeviceSpaceTopologyDialog(
        spaceName: spaceName,
        topology: topology,
        onDisconnectDevice: onDisconnectDevice,
      ),
    );
  }

  /// Builds the topology header, graph, and connection summary.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final maxHeight = math.min(MediaQuery.sizeOf(context).height - 32, 640.0);
    return Dialog(
      insetPadding: const EdgeInsets.all(16),
      child: ConstrainedBox(
        constraints: BoxConstraints(maxWidth: 820, maxHeight: maxHeight),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 14, 8, 10),
              child: Row(
                children: <Widget>[
                  Icon(Icons.hub_outlined, color: colorScheme.primary),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          l10n.settingsRuntimeSpaceTopologyTitle(spaceName),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: Theme.of(context).textTheme.titleMedium
                              ?.copyWith(fontWeight: FontWeight.w800),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          l10n.settingsRuntimeSpaceTopologySummary(
                            topology.devices.length,
                            topology.connections.length,
                          ),
                          style: Theme.of(context).textTheme.bodySmall
                              ?.copyWith(color: colorScheme.onSurfaceVariant),
                        ),
                      ],
                    ),
                  ),
                  IconButton(
                    tooltip: MaterialLocalizations.of(
                      context,
                    ).closeButtonTooltip,
                    onPressed: () => Navigator.of(context).pop(),
                    icon: const Icon(Icons.close_rounded),
                  ),
                ],
              ),
            ),
            Divider(height: 1, color: colorScheme.outlineVariant),
            Flexible(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(16, 18, 16, 20),
                child: _DeviceSpaceTopologyGraph(
                  topology: topology,
                  onDisconnectDevice: onDisconnectDevice,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _DeviceSpaceTopologyGraph extends StatefulWidget {
  const _DeviceSpaceTopologyGraph({
    required this.topology,
    required this.onDisconnectDevice,
  });

  final generated.RuntimeDeviceSpaceTopology topology;
  final Future<generated.RuntimeDeviceSpaceTopology> Function(String deviceId)
  onDisconnectDevice;

  /// Creates the state that handles edge selection without rebuilding the dialog.
  @override
  State<_DeviceSpaceTopologyGraph> createState() =>
      _DeviceSpaceTopologyGraphState();
}

class _DeviceSpaceTopologyGraphState extends State<_DeviceSpaceTopologyGraph> {
  late generated.RuntimeDeviceSpaceTopology _topology;

  @override
  void initState() {
    super.initState();
    _topology = widget.topology;
  }

  /// Builds a pannable and zoomable canvas for the complete device graph.
  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final viewportWidth = constraints.maxWidth;
        final viewportHeight = constraints.maxHeight;
        const nodeSize = Size(148, 112);
        final canvasSize = _topologyCanvasSize(
          viewportWidth,
          _topology.devices.length,
          nodeSize,
        );
        final centers = _deviceCenters(
          canvasSize,
          _topology.devices.length,
          nodeSize,
        );
        final centerByDeviceId = <String, Offset>{
          for (var index = 0; index < _topology.devices.length; index++)
            _topology.devices[index].deviceId: centers[index],
        };
        final colorScheme = Theme.of(context).colorScheme;
        return SizedBox(
          width: viewportWidth,
          height: viewportHeight,
          child: InteractiveViewer(
            constrained: false,
            boundaryMargin: const EdgeInsets.all(220),
            minScale: 0.35,
            maxScale: 3.5,
            panEnabled: true,
            scaleEnabled: true,
            trackpadScrollCausesScale: true,
            child: SizedBox(
              width: canvasSize.width,
              height: canvasSize.height,
              child: Stack(
                clipBehavior: Clip.none,
                children: <Widget>[
                  Positioned.fill(
                    child: GestureDetector(
                      behavior: HitTestBehavior.opaque,
                      onTapUp: (details) {
                        final connection = _connectionAtPoint(
                          details.localPosition,
                          centerByDeviceId,
                          _topology.connections,
                        );
                        if (connection != null) {
                          unawaited(_showConnectionDetails(connection));
                        }
                      },
                      child: CustomPaint(
                        painter: _DeviceSpaceTopologyPainter(
                          centers: centerByDeviceId,
                          connections: _topology.connections,
                          onlineColor: colorScheme.outline,
                          offlineColor: colorScheme.error,
                          mismatchColor: colorScheme.tertiary,
                          unknownColor: colorScheme.outlineVariant,
                        ),
                      ),
                    ),
                  ),
                  for (var index = 0; index < _topology.devices.length; index++)
                    Positioned(
                      left: centers[index].dx - nodeSize.width / 2,
                      top: centers[index].dy - nodeSize.height / 2,
                      width: nodeSize.width,
                      height: nodeSize.height,
                      child: _DeviceSpaceTopologyNode(
                        device: _topology.devices[index],
                        current:
                            _topology.devices[index].deviceId ==
                            _topology.currentDeviceId,
                      ),
                    ),
                ],
              ),
            ),
          ),
        );
      },
    );
  }

  /// Shows the backend-provided reason for a selected connection.
  Future<void> _showConnectionDetails(
    generated.RuntimeDeviceSpaceConnection connection,
  ) async {
    if (!mounted) {
      return;
    }
    final devicesById = <String, generated.RuntimeDeviceSpaceDevice>{
      for (final device in _topology.devices) device.deviceId: device,
    };
    final first = devicesById[connection.firstDeviceId]!;
    final second = devicesById[connection.secondDeviceId]!;
    final currentDeviceId = _topology.currentDeviceId;
    final target = connection.firstDeviceId == currentDeviceId
        ? second
        : connection.secondDeviceId == currentDeviceId
        ? first
        : null;
    final canDisconnect =
        target != null &&
        connection.status !=
            generated.RuntimeDeviceSpaceConnectionStatus.offline;
    await showDialog<void>(
      context: context,
      builder: (context) {
        final colorScheme = Theme.of(context).colorScheme;
        return AlertDialog(
          title: Text('${first.deviceName} ↔ ${second.deviceName}'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                children: <Widget>[
                  Icon(
                    _connectionStatusIcon(connection.status),
                    color: _connectionStatusColor(
                      connection.status,
                      _topologyPainterColors(colorScheme),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(_connectionStatusLabel(connection.status)),
                ],
              ),
              const SizedBox(height: 12),
              Text(connection.reason),
              const SizedBox(height: 12),
              if (first.coreVersion case final firstVersion?)
                Text(
                  '${first.deviceName}: $firstVersion',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              if (second.coreVersion case final secondVersion?)
                Text(
                  '${second.deviceName}: $secondVersion',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
            ],
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: Text(MaterialLocalizations.of(context).closeButtonLabel),
            ),
            if (canDisconnect)
              FilledButton.icon(
                onPressed: () => unawaited(
                  _confirmDisconnect(target, AppLocalizations.of(context)!),
                ),
                icon: const Icon(Icons.link_off),
                label: Text(
                  AppLocalizations.of(
                    context,
                  )!.settingsRuntimeDisconnectConnection,
                ),
              ),
          ],
        );
      },
    );
  }

  /// Confirms and executes an owner-initiated direct connection disconnect.
  Future<void> _confirmDisconnect(
    generated.RuntimeDeviceSpaceDevice target,
    AppLocalizations l10n,
  ) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l10n.settingsRuntimeDisconnectConnectionTitle),
        content: Text(
          l10n.settingsRuntimeDisconnectConnectionMessage(target.deviceName),
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
    if (confirmed != true || !mounted) {
      return;
    }
    try {
      final refreshedTopology = await widget.onDisconnectDevice(
        target.deviceId,
      );
      if (!mounted) {
        return;
      }
      setState(() => _topology = refreshedTopology);
      Navigator.of(context).pop();
    } catch (error) {
      if (!mounted) {
        return;
      }
      await showDialog<void>(
        context: context,
        builder: (context) => AlertDialog(
          title: Text(l10n.settingsRuntimeDisconnectConnectionFailed),
          content: Text(error.toString()),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: Text(MaterialLocalizations.of(context).closeButtonLabel),
            ),
          ],
        ),
      );
    }
  }
}

/// Calculates a canvas that grows with the number of device nodes.
Size _topologyCanvasSize(double viewportWidth, int deviceCount, Size nodeSize) {
  if (deviceCount == 0) {
    throw StateError('device-space topology contains no devices');
  }
  final columns = math.max(1, math.sqrt(deviceCount).ceil()).toInt();
  final rows = ((deviceCount + columns - 1) / columns).ceil();
  final width = math.max(viewportWidth, columns * 190.0 + 80.0);
  final height = math.max(320.0, rows * 156.0 + nodeSize.height);
  return Size(width, height);
}

/// Calculates stable grid centers for every device in the topology snapshot.
List<Offset> _deviceCenters(Size graphSize, int deviceCount, Size nodeSize) {
  if (deviceCount == 0) {
    throw StateError('device-space topology contains no devices');
  }
  final columns = math.max(1, math.sqrt(deviceCount).ceil()).toInt();
  const horizontalGap = 42.0;
  const verticalGap = 44.0;
  final cellWidth = nodeSize.width + horizontalGap;
  final cellHeight = nodeSize.height + verticalGap;
  return List<Offset>.generate(deviceCount, (index) {
    final row = index ~/ columns;
    final column = index % columns;
    final rowItemCount = math.min(columns, deviceCount - row * columns);
    final rowWidth = rowItemCount * cellWidth - horizontalGap;
    final startX = (graphSize.width - rowWidth) / 2 + nodeSize.width / 2;
    final startY = 28.0 + nodeSize.height / 2;
    return Offset(startX + column * cellWidth, startY + row * cellHeight);
  }, growable: false);
}

/// Finds the nearest connection under a canvas tap.
generated.RuntimeDeviceSpaceConnection? _connectionAtPoint(
  Offset point,
  Map<String, Offset> centers,
  List<generated.RuntimeDeviceSpaceConnection> connections,
) {
  const hitDistance = 16.0;
  generated.RuntimeDeviceSpaceConnection? nearest;
  var nearestDistance = double.infinity;
  for (final connection in connections) {
    final first = centers[connection.firstDeviceId]!;
    final second = centers[connection.secondDeviceId]!;
    final distance = _distanceToSegment(point, first, second);
    if (distance <= hitDistance && distance < nearestDistance) {
      nearest = connection;
      nearestDistance = distance;
    }
  }
  return nearest;
}

/// Returns the shortest distance from one point to a line segment.
double _distanceToSegment(Offset point, Offset first, Offset second) {
  final delta = second - first;
  final lengthSquared = delta.distanceSquared;
  if (lengthSquared == 0) {
    throw StateError('device-space topology contains a zero-length connection');
  }
  final projection =
      ((point - first).dx * delta.dx + (point - first).dy * delta.dy) /
      lengthSquared;
  final clamped = projection.clamp(0.0, 1.0);
  final nearest = first + delta * clamped;
  return (point - nearest).distance;
}

class _DeviceSpaceTopologyNode extends StatelessWidget {
  const _DeviceSpaceTopologyNode({required this.device, required this.current});

  final generated.RuntimeDeviceSpaceDevice device;
  final bool current;

  /// Builds one labeled device node with its current reachability state.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final statusColor = device.online ? colorScheme.primary : colorScheme.error;
    final backgroundColor = current
        ? colorScheme.primaryContainer
        : device.online
        ? colorScheme.surfaceContainerHighest
        : colorScheme.errorContainer.withValues(alpha: 0.72);
    final foregroundColor = current
        ? colorScheme.onPrimaryContainer
        : colorScheme.onSurface;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 7),
      decoration: BoxDecoration(
        color: backgroundColor,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: current ? colorScheme.primary : statusColor,
          width: current ? 2 : 1.4,
        ),
      ),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: <Widget>[
          Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: <Widget>[
              Icon(
                Icons.devices_other_outlined,
                size: 16,
                color: foregroundColor,
              ),
              const SizedBox(width: 4),
              Icon(
                device.online
                    ? Icons.cloud_done_outlined
                    : Icons.cloud_off_outlined,
                size: 15,
                color: statusColor,
              ),
              if (current) ...<Widget>[
                const SizedBox(width: 4),
                Flexible(
                  child: Text(
                    l10n.settingsRuntimeCurrentDevice,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.labelSmall?.copyWith(
                      color: foregroundColor,
                      fontWeight: FontWeight.w800,
                    ),
                  ),
                ),
              ],
            ],
          ),
          const SizedBox(height: 4),
          Text(
            _deviceUserName(l10n, device.userName),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            textAlign: TextAlign.center,
            style: Theme.of(context).textTheme.labelMedium?.copyWith(
              fontWeight: FontWeight.w800,
              color: foregroundColor,
            ),
          ),
          const SizedBox(height: 1),
          Text(
            device.deviceName,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            textAlign: TextAlign.center,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: foregroundColor.withValues(alpha: 0.82),
            ),
          ),
          if (device.coreVersion case final version?) ...<Widget>[
            const SizedBox(height: 1),
            Text(
              version,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.center,
              style: Theme.of(context).textTheme.labelSmall?.copyWith(
                color: foregroundColor.withValues(alpha: 0.64),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _DeviceSpaceTopologyPainter extends CustomPainter {
  const _DeviceSpaceTopologyPainter({
    required this.centers,
    required this.connections,
    required this.onlineColor,
    required this.offlineColor,
    required this.mismatchColor,
    required this.unknownColor,
  });

  final Map<String, Offset> centers;
  final List<generated.RuntimeDeviceSpaceConnection> connections;
  final Color onlineColor;
  final Color offlineColor;
  final Color mismatchColor;
  final Color unknownColor;

  /// Draws every direct-device edge and marks unhealthy links with an X.
  @override
  void paint(Canvas canvas, Size size) {
    for (final connection in connections) {
      final first = centers[connection.firstDeviceId]!;
      final second = centers[connection.secondDeviceId]!;
      final color = _connectionStatusColor(
        connection.status,
        _TopologyPainterColors(
          online: onlineColor,
          offline: offlineColor,
          mismatch: mismatchColor,
          unknown: unknownColor,
        ),
      );
      final paint = Paint()
        ..color = color
        ..strokeWidth =
            connection.status ==
                generated.RuntimeDeviceSpaceConnectionStatus.online
            ? 2
            : 2.2
        ..style = PaintingStyle.stroke;
      if (connection.status ==
          generated.RuntimeDeviceSpaceConnectionStatus.online) {
        canvas.drawLine(first, second, paint);
      } else {
        _drawDashedLine(canvas, first, second, paint);
      }
      if (connection.status ==
              generated.RuntimeDeviceSpaceConnectionStatus.offline ||
          connection.status ==
              generated.RuntimeDeviceSpaceConnectionStatus.versionMismatch) {
        _drawConnectionCross(canvas, Offset.lerp(first, second, 0.5)!, color);
      }
    }
  }

  /// Draws a dashed segment without relying on platform-specific painting APIs.
  static void _drawDashedLine(
    Canvas canvas,
    Offset first,
    Offset second,
    Paint paint,
  ) {
    final direction = second - first;
    final length = direction.distance;
    if (length == 0) {
      throw StateError(
        'device-space topology contains a zero-length connection',
      );
    }
    final unit = direction / length;
    const dashLength = 8.0;
    const gapLength = 5.0;
    var distance = 0.0;
    while (distance < length) {
      final dashEnd = math.min(distance + dashLength, length);
      canvas.drawLine(first + unit * distance, first + unit * dashEnd, paint);
      distance += dashLength + gapLength;
    }
  }

  /// Draws the cross marker used for offline and incompatible links.
  static void _drawConnectionCross(Canvas canvas, Offset center, Color color) {
    final paint = Paint()
      ..color = color
      ..strokeWidth = 3
      ..strokeCap = StrokeCap.round;
    const radius = 8.0;
    canvas.drawLine(
      center + const Offset(-radius, -radius),
      center + const Offset(radius, radius),
      paint,
    );
    canvas.drawLine(
      center + const Offset(radius, -radius),
      center + const Offset(-radius, radius),
      paint,
    );
  }

  /// Repaints when the graph snapshot, colors, or connection states change.
  @override
  bool shouldRepaint(covariant _DeviceSpaceTopologyPainter oldDelegate) {
    return oldDelegate.centers != centers ||
        oldDelegate.connections != connections ||
        oldDelegate.onlineColor != onlineColor ||
        oldDelegate.offlineColor != offlineColor ||
        oldDelegate.mismatchColor != mismatchColor ||
        oldDelegate.unknownColor != unknownColor;
  }
}

/// Holds the painter colors needed by connection status helpers.
class _TopologyPainterColors {
  const _TopologyPainterColors({
    required this.online,
    required this.offline,
    required this.mismatch,
    required this.unknown,
  });

  final Color online;
  final Color offline;
  final Color mismatch;
  final Color unknown;
}

/// Returns the visual color associated with one connection status.
Color _connectionStatusColor(
  generated.RuntimeDeviceSpaceConnectionStatus status,
  _TopologyPainterColors colors,
) {
  return switch (status) {
    generated.RuntimeDeviceSpaceConnectionStatus.online => colors.online,
    generated.RuntimeDeviceSpaceConnectionStatus.offline => colors.offline,
    generated.RuntimeDeviceSpaceConnectionStatus.versionMismatch =>
      colors.mismatch,
    generated.RuntimeDeviceSpaceConnectionStatus.unknown => colors.unknown,
  };
}

/// Converts a Material color scheme into graph painter colors.
_TopologyPainterColors _topologyPainterColors(ColorScheme colorScheme) {
  return _TopologyPainterColors(
    online: colorScheme.outline,
    offline: colorScheme.error,
    mismatch: colorScheme.tertiary,
    unknown: colorScheme.outlineVariant,
  );
}

/// Returns the icon used to describe one connection status.
IconData _connectionStatusIcon(
  generated.RuntimeDeviceSpaceConnectionStatus status,
) {
  return switch (status) {
    generated.RuntimeDeviceSpaceConnectionStatus.online => Icons.link,
    generated.RuntimeDeviceSpaceConnectionStatus.offline => Icons.link_off,
    generated.RuntimeDeviceSpaceConnectionStatus.versionMismatch =>
      Icons.sync_problem_outlined,
    generated.RuntimeDeviceSpaceConnectionStatus.unknown => Icons.help_outline,
  };
}

/// Returns the short human-readable label for one connection status.
String _connectionStatusLabel(
  generated.RuntimeDeviceSpaceConnectionStatus status,
) {
  return switch (status) {
    generated.RuntimeDeviceSpaceConnectionStatus.online => 'Online',
    generated.RuntimeDeviceSpaceConnectionStatus.offline => 'Offline',
    generated.RuntimeDeviceSpaceConnectionStatus.versionMismatch =>
      'Core version mismatch',
    generated.RuntimeDeviceSpaceConnectionStatus.unknown => 'Unknown',
  };
}

/// Returns the configured user name or the explicit unconfigured label.
String _deviceUserName(AppLocalizations l10n, String userName) {
  final normalized = userName.trim();
  return normalized.isEmpty ? l10n.settingsUserProfileUnnamed : normalized;
}

/// Collects the replacement name while owning the text controller lifecycle.
class _RenameCurrentDeviceSpaceDialog extends StatefulWidget {
  /// Creates a dialog initialized with the current device space name.
  const _RenameCurrentDeviceSpaceDialog({required this.initialName});

  final String initialName;

  /// Displays the dialog and returns the submitted space name.
  static Future<String?> show(
    BuildContext context, {
    required String initialName,
  }) {
    return showDialog<String>(
      context: context,
      builder: (_) => _RenameCurrentDeviceSpaceDialog(initialName: initialName),
    );
  }

  /// Creates the state that owns the name input controller.
  @override
  State<_RenameCurrentDeviceSpaceDialog> createState() =>
      _RenameCurrentDeviceSpaceDialogState();
}

/// Owns the name input controller until the dialog route is removed.
class _RenameCurrentDeviceSpaceDialogState
    extends State<_RenameCurrentDeviceSpaceDialog> {
  late final TextEditingController _controller;

  /// Initializes the controller from the displayed space name.
  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.initialName);
  }

  /// Releases the controller after the dialog route finishes its exit transition.
  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  /// Closes the dialog with the trimmed input value.
  void _submit() {
    Navigator.of(context).pop(_controller.text.trim());
  }

  /// Builds the editable device space name dialog.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.settingsRuntimeRenameSpace),
      content: TextField(
        controller: _controller,
        autofocus: true,
        maxLength: 80,
        decoration: InputDecoration(
          labelText: l10n.settingsRuntimeSpaceName,
          border: const OutlineInputBorder(),
        ),
        onSubmitted: (_) => _submit(),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _submit, child: Text(l10n.save)),
      ],
    );
  }
}

class _PairedDeviceList extends StatelessWidget {
  const _PairedDeviceList({
    required this.devices,
    required this.busy,
    required this.states,
    required this.currentMemberIds,
    required this.onJoin,
    required this.onDelete,
    required this.onTransportChanged,
  });

  final Map<String, generated.RuntimePairedDevice> devices;
  final bool busy;
  final Map<String, _PairedRemoteProbeState> states;
  final Set<String> currentMemberIds;
  final ValueChanged<generated.RuntimePairedDevice> onJoin;
  final ValueChanged<String> onDelete;
  final void Function(
    generated.RuntimePairedDevice,
    generated.LinkTransportPreference,
  )
  onTransportChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final entries = devices.entries.toList(growable: false);
    if (entries.isEmpty) {
      return Text(
        l10n.settingsRuntimeNoPairedRemote,
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      );
    }
    return Column(
      children: <Widget>[
        for (var index = 0; index < entries.length; index++) ...<Widget>[
          _PairedDeviceTile(
            device: entries[index].value,
            busy: busy,
            state: states[entries[index].key],
            inCurrentSpace: currentMemberIds.contains(entries[index].key),
            onJoin: entries[index].value.outboundSessionName == null
                ? null
                : () => onJoin(entries[index].value),
            onDelete: () => onDelete(entries[index].key),
            onTransportChanged: (transport) =>
                onTransportChanged(entries[index].value, transport),
          ),
          if (index < entries.length - 1) const SizedBox(height: 10),
        ],
      ],
    );
  }
}

class _PairedDeviceTile extends StatelessWidget {
  const _PairedDeviceTile({
    required this.device,
    required this.busy,
    required this.state,
    required this.inCurrentSpace,
    required this.onJoin,
    required this.onDelete,
    required this.onTransportChanged,
  });

  final generated.RuntimePairedDevice device;
  final bool busy;
  final _PairedRemoteProbeState? state;
  final bool inCurrentSpace;
  final VoidCallback? onJoin;
  final VoidCallback onDelete;
  final ValueChanged<generated.LinkTransportPreference> onTransportChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final probeState = state ?? _PairedRemoteProbeState.checking;
    final outboundBaseUrl = device.outboundBaseUrl;
    final statusColor = switch (probeState) {
      _PairedRemoteProbeState.checking => colorScheme.onSurfaceVariant,
      _PairedRemoteProbeState.online => colorScheme.primary,
      _PairedRemoteProbeState.offline => colorScheme.error,
    };
    return Container(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.22),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.45),
        ),
      ),
      child: ExpansionTile(
        dense: true,
        visualDensity: VisualDensity.compact,
        tilePadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 2),
        childrenPadding: const EdgeInsets.fromLTRB(53, 0, 12, 8),
        backgroundColor: Colors.transparent,
        collapsedBackgroundColor: Colors.transparent,
        shape: const Border(),
        collapsedShape: const Border(),
        leading: Container(
          width: 34,
          height: 34,
          decoration: BoxDecoration(
            color: statusColor.withValues(alpha: 0.12),
            borderRadius: BorderRadius.circular(10),
          ),
          child: Center(child: _RemoteProbeIcon(state: probeState)),
        ),
        title: Row(
          children: <Widget>[
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Text(
                    '${device.deviceInfo.platform}-${device.deviceInfo.model}',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Row(
                    children: <Widget>[
                      Icon(
                        Icons.link_outlined,
                        size: 13,
                        color: colorScheme.onSurfaceVariant,
                      ),
                      const SizedBox(width: 4),
                      Expanded(
                        child: Text(
                          outboundBaseUrl ??
                              l10n.settingsRuntimeConnectionInitiatedByOtherDevice,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Container(
                        width: 7,
                        height: 7,
                        decoration: BoxDecoration(
                          color: statusColor,
                          shape: BoxShape.circle,
                        ),
                      ),
                      const SizedBox(width: 5),
                      Flexible(child: _RemoteProbeText(state: probeState)),
                      if (inCurrentSpace) ...<Widget>[
                        const SizedBox(width: 8),
                        Flexible(
                          child: Text(
                            l10n.settingsRuntimeDeviceInCurrentSpace,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: textTheme.labelSmall?.copyWith(
                              color: colorScheme.primary,
                              fontWeight: FontWeight.w700,
                            ),
                          ),
                        ),
                      ],
                    ],
                  ),
                ],
              ),
            ),
            if (!inCurrentSpace && onJoin != null)
              IconButton(
                tooltip: l10n.settingsRuntimeJoinSpace,
                visualDensity: VisualDensity.compact,
                icon: const Icon(Icons.group_add_outlined, size: 20),
                onPressed: busy || probeState != _PairedRemoteProbeState.online
                    ? null
                    : onJoin,
              ),
            IconButton(
              tooltip: l10n.delete,
              visualDensity: VisualDensity.compact,
              icon: const Icon(Icons.link_off_outlined, size: 20),
              onPressed: busy ? null : onDelete,
            ),
          ],
        ),
        children: <Widget>[
          if (device.outboundTransport != null)
            Row(
              children: <Widget>[
                Icon(
                  Icons.swap_horiz_outlined,
                  size: 15,
                  color: colorScheme.onSurfaceVariant,
                ),
                const SizedBox(width: 5),
                Text(
                  'Link transport',
                  style: textTheme.labelSmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
                const Spacer(),
                SizedBox(
                  width: 104,
                  child: SegmentedButton<generated.LinkTransportPreference>(
                    segments:
                        const <
                          ButtonSegment<generated.LinkTransportPreference>
                        >[
                          ButtonSegment(
                            value: generated.LinkTransportPreference.http,
                            label: Text('HTTP'),
                          ),
                          ButtonSegment(
                            value: generated.LinkTransportPreference.webSocket,
                            label: Text('WS'),
                          ),
                        ],
                    selected: <generated.LinkTransportPreference>{
                      device.outboundTransport!,
                    },
                    showSelectedIcon: false,
                    style: ButtonStyle(
                      visualDensity: VisualDensity.compact,
                      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                      textStyle: WidgetStatePropertyAll<TextStyle?>(
                        textTheme.labelSmall,
                      ),
                      padding: const WidgetStatePropertyAll<EdgeInsets>(
                        EdgeInsets.zero,
                      ),
                      minimumSize: const WidgetStatePropertyAll<Size>(
                        Size(0, 28),
                      ),
                    ),
                    onSelectionChanged: busy
                        ? null
                        : (selection) => onTransportChanged(selection.first),
                  ),
                ),
              ],
            ),
        ],
      ),
    );
  }
}

class _InlineStatus extends StatelessWidget {
  const _InlineStatus({required this.message, required this.failed});

  final String message;
  final bool failed;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Text(
      message,
      style: Theme.of(context).textTheme.bodySmall?.copyWith(
        color: failed ? colorScheme.error : colorScheme.primary,
        fontWeight: FontWeight.w700,
      ),
    );
  }
}

class _RemoteProbeIcon extends StatelessWidget {
  const _RemoteProbeIcon({required this.state});

  final _PairedRemoteProbeState state;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return switch (state) {
      _PairedRemoteProbeState.checking => const SizedBox(
        width: 24,
        height: 24,
        child: Center(child: M3LoadingIndicator(size: 18)),
      ),
      _PairedRemoteProbeState.online => Icon(
        Icons.cloud_done_outlined,
        color: colorScheme.primary,
      ),
      _PairedRemoteProbeState.offline => Icon(
        Icons.cloud_off_outlined,
        color: colorScheme.error,
      ),
    };
  }
}

class _RemoteProbeText extends StatelessWidget {
  const _RemoteProbeText({required this.state});

  final _PairedRemoteProbeState state;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final label = switch (state) {
      _PairedRemoteProbeState.checking => l10n.settingsRuntimePairedChecking,
      _PairedRemoteProbeState.online => l10n.settingsRuntimePairedOnline,
      _PairedRemoteProbeState.offline => l10n.settingsRuntimePairedOffline,
    };
    final color = switch (state) {
      _PairedRemoteProbeState.checking => colorScheme.onSurfaceVariant,
      _PairedRemoteProbeState.online => colorScheme.primary,
      _PairedRemoteProbeState.offline => colorScheme.error,
    };
    return Text(
      label,
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
      style: Theme.of(context).textTheme.bodySmall?.copyWith(
        color: color,
        fontWeight: FontWeight.w700,
      ),
    );
  }
}
