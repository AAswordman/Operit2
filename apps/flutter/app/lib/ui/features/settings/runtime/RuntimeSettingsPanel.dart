// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;
import 'package:flutter/material.dart';

import '../../../../core/bridge/PlatformCoreProxy.dart';
import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as generated;
import '../../../../core/runtime/RuntimeBootstrapManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/DeviceSpaceDiscoveryPanel.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';
import '../profile/UserProfileSummaryTile.dart';

class RuntimeSettingsPanel extends StatefulWidget {
  const RuntimeSettingsPanel({
    super.key,
    this.embedded = false,
    required this.onOpenProfile,
  });

  final bool embedded;
  final VoidCallback onOpenProfile;

  @override
  State<RuntimeSettingsPanel> createState() => _RuntimeSettingsPanelState();
}

class _RuntimeSettingsPanelState extends State<RuntimeSettingsPanel> {
  bool _busy = false;
  String? _connectionMessage;
  bool _connectionFailed = false;
  generated.CoreSpace? _currentDeviceSpace;
  generated.RuntimeDeviceSpaceTopology? _topology;
  Map<String, _PairedRemoteProbeState> _pairedRemoteStates =
      <String, _PairedRemoteProbeState>{};
  Map<String, generated.RuntimePairedDevice> _pairedDevices =
      <String, generated.RuntimePairedDevice>{};
  StreamSubscription<Map<String, generated.RuntimePairedDevice>>?
  _pairedDevicesSubscription;
  StreamSubscription<Map<String, generated.RuntimePairedDeviceStatus>>?
  _pairedDeviceStatusesSubscription;

  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(coreProxy: platformCoreProxy),
  );

  @override
  void initState() {
    super.initState();
    unawaited(_refreshCurrentDeviceSpace());
    _watchPairedDevices();
    _watchPairedDeviceStatuses();
  }

  @override
  void dispose() {
    final pairedDevicesSubscription = _pairedDevicesSubscription;
    if (pairedDevicesSubscription != null) {
      unawaited(pairedDevicesSubscription.cancel());
    }
    final pairedDeviceStatusesSubscription = _pairedDeviceStatusesSubscription;
    if (pairedDeviceStatusesSubscription != null) {
      unawaited(pairedDeviceStatusesSubscription.cancel());
    }
    super.dispose();
  }

  /// Subscribes to pairing changes produced by both connection directions.
  void _watchPairedDevices() {
    _pairedDevicesSubscription = _clients.server.runtimeRemoteLinkService
        .pairedDevicesFlow()
        .listen(
          _applyPairedDevices,
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

  /// Subscribes to direct Peer Link status changes for paired devices.
  void _watchPairedDeviceStatuses() {
    _pairedDeviceStatusesSubscription = _clients.server.runtimeRemoteLinkService
        .pairedDeviceStatusesFlow()
        .listen(
          _applyPairedDeviceStatuses,
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

  /// Applies peer-driven online states without changing pairing validity prompts.
  void _applyPairedDeviceStatuses(
    Map<String, generated.RuntimePairedDeviceStatus> statuses,
  ) {
    if (!mounted) {
      return;
    }
    setState(() {
      final nextStates = Map<String, _PairedRemoteProbeState>.from(
        _pairedRemoteStates,
      )..removeWhere((deviceId, _) => !statuses.containsKey(deviceId));
      for (final entry in statuses.entries) {
        final currentState = nextStates[entry.key];
        if (currentState == _PairedRemoteProbeState.invalid ||
            currentState == _PairedRemoteProbeState.removedFromSpace) {
          continue;
        }
        nextStates[entry.key] = _pairedRemoteStateFromStatus(entry.value);
      }
      _pairedRemoteStates = nextStates;
    });
  }

  /// Reads the synchronized device space projection from the current device.
  Future<void> _refreshCurrentDeviceSpace() async {
    try {
      final deviceSpace = await _clients.server.runtimeRemoteLinkService
          .deviceSpace();
      final topology = await _clients.server.runtimeRemoteLinkService
          .deviceSpaceTopology();
      if (mounted) {
        setState(() {
          _currentDeviceSpace = deviceSpace;
          _topology = topology;
        });
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
      final topology = await _clients.server.runtimeRemoteLinkService
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
    await _clients.server.runtimeRemoteLinkService
        .disconnectDeviceSpaceConnection(deviceId: deviceId);
    final refreshedDeviceSpace = await _clients.server.runtimeRemoteLinkService
        .deviceSpace();
    final refreshedTopology = await _clients.server.runtimeRemoteLinkService
        .deviceSpaceTopology();
    if (mounted) {
      setState(() {
        _currentDeviceSpace = refreshedDeviceSpace;
        _topology = refreshedTopology;
      });
    }
    return _clients.server.runtimeRemoteLinkService.deviceSpaceTopology();
  }

  /// Applies one paired-device snapshot without network probing.
  void _applyPairedDevices(Map<String, generated.RuntimePairedDevice> devices) {
    if (!mounted) {
      return;
    }
    setState(() {
      _pairedDevices = devices;
      _pairedRemoteStates = <String, _PairedRemoteProbeState>{
        for (final deviceId in devices.keys)
          deviceId:
              _pairedRemoteStates[deviceId] ?? _PairedRemoteProbeState.checking,
      };
    });
  }

  /// Removes every local pairing record associated with one device.
  Future<void> _deletePairedDevice(String deviceId) async {
    setState(() => _busy = true);
    try {
      await _clients.server.runtimeRemoteLinkService.removePairedDevice(
        deviceId: deviceId,
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Confirms leaving the removed Space and creates a standalone local Space.
  Future<void> _handleRemovedFromSpace() async {
    if (!mounted) {
      return;
    }
    final l10n = AppLocalizations.of(context)!;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: Text(l10n.settingsRuntimeRemovedFromSpaceTitle),
        content: Text(l10n.settingsRuntimeRemovedFromSpaceMessage),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: Text(l10n.settingsRuntimeRemovedFromSpaceConfirm),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) {
      return;
    }
    await _leaveCurrentDeviceSpaceNow();
  }

  /// Leaves the current device space without asking for a second confirmation.
  Future<void> _leaveCurrentDeviceSpaceNow() async {
    setState(() => _busy = true);
    try {
      final deviceSpace = await _clients.server.runtimeRemoteLinkService
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
      await _clients.server.runtimeRemoteLinkService.setPairedRemoteTransport(
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
      final renamed = await _clients.server.runtimeRemoteLinkService
          .renameDeviceSpace(spaceName: spaceName);
      if (mounted) {
        setState(() => _currentDeviceSpace = renamed);
      }
      await _refreshTopology();
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
      final deviceSpace = await _clients.server.runtimeRemoteLinkService
          .leaveDeviceSpace();
      if (mounted) {
        setState(() {
          _currentDeviceSpace = deviceSpace;
          _connectionMessage = null;
          _connectionFailed = false;
        });
      }
      await _refreshTopology();
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
    final topology = await _clients.server.runtimeRemoteLinkService
        .deviceSpaceTopology();
    setState(() {
      _currentDeviceSpace = deviceSpace;
      _topology = topology;
      _connectionMessage = null;
      _connectionFailed = false;
    });
  }

  /// Refreshes the visible device graph after a space mutation.
  Future<void> _refreshTopology() async {
    final topology = await _clients.server.runtimeRemoteLinkService
        .deviceSpaceTopology();
    if (mounted) {
      setState(() => _topology = topology);
    }
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
      _DeviceSpaceOverviewCard(
        deviceSpace: _currentDeviceSpace,
        topology: _topology,
        busy: _busy,
        onViewTopology: _openDeviceSpaceTopology,
        onRename: _renameCurrentDeviceSpace,
        onLeave: _leaveCurrentDeviceSpace,
        onOpenProfile: widget.onOpenProfile,
        connectionMessage: _connectionMessage,
        connectionFailed: _connectionFailed,
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
            onRemovedFromSpace: _handleRemovedFromSpace,
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
      return _DeviceSpaceBackdrop(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: children,
        ),
      );
    }
    return _DeviceSpaceBackdrop(
      child: ListView(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
        children: children,
      ),
    );
  }
}

class _DeviceSpaceBackdrop extends StatelessWidget {
  const _DeviceSpaceBackdrop({required this.child});

  final Widget child;

  /// Paints the violet starfield behind the complete device-space workflow.
  @override
  Widget build(BuildContext context) {
    return CustomPaint(
      painter: _DeviceSpaceBackdropPainter(
        base: Theme.of(context).colorScheme.surface,
        glow: Theme.of(context).colorScheme.primary,
        secondary: Theme.of(context).colorScheme.secondary,
      ),
      child: child,
    );
  }
}

class _DeviceSpaceBackdropPainter extends CustomPainter {
  const _DeviceSpaceBackdropPainter({
    required this.base,
    required this.glow,
    required this.secondary,
  });

  final Color base;
  final Color glow;
  final Color secondary;

  /// Draws a subtle diagonal gradient and fixed sparse stars.
  @override
  void paint(Canvas canvas, Size size) {
    final rect = Offset.zero & size;
    canvas.drawRect(
      rect,
      Paint()
        ..shader = LinearGradient(
          begin: Alignment.topRight,
          end: Alignment.bottomLeft,
          colors: <Color>[
            Color.alphaBlend(glow.withValues(alpha: 0.18), base),
            Color.alphaBlend(secondary.withValues(alpha: 0.08), base),
            base,
          ],
          stops: const <double>[0, 0.42, 1],
        ).createShader(rect),
    );
    final starPaint = Paint()..color = glow.withValues(alpha: 0.26);
    final stars = <Offset>[
      Offset(size.width * 0.07, size.height * 0.12),
      Offset(size.width * 0.22, size.height * 0.23),
      Offset(size.width * 0.42, size.height * 0.13),
      Offset(size.width * 0.64, size.height * 0.3),
      Offset(size.width * 0.83, size.height * 0.18),
      Offset(size.width * 0.91, size.height * 0.57),
      Offset(size.width * 0.35, size.height * 0.72),
      Offset(size.width * 0.74, size.height * 0.84),
    ];
    for (final star in stars) {
      canvas.drawCircle(star, 1.8, starPaint);
    }
  }

  /// Repaints the backdrop when the active theme colors change.
  @override
  bool shouldRepaint(covariant _DeviceSpaceBackdropPainter oldDelegate) {
    return oldDelegate.base != base ||
        oldDelegate.glow != glow ||
        oldDelegate.secondary != secondary;
  }
}

enum _PairedRemoteProbeState {
  checking,
  online,
  offline,
  invalid,
  error,
  removedFromSpace,
}

/// Converts the generated paired-device status into the UI probe state.
_PairedRemoteProbeState _pairedRemoteStateFromStatus(
  generated.RuntimePairedDeviceStatus status,
) {
  return switch (status) {
    generated.RuntimePairedDeviceStatus.online =>
      _PairedRemoteProbeState.online,
    generated.RuntimePairedDeviceStatus.offline =>
      _PairedRemoteProbeState.offline,
    generated.RuntimePairedDeviceStatus.invalid =>
      _PairedRemoteProbeState.invalid,
    generated.RuntimePairedDeviceStatus.removedFromSpace =>
      _PairedRemoteProbeState.removedFromSpace,
  };
}

class _DeviceSpaceOverviewCard extends StatelessWidget {
  const _DeviceSpaceOverviewCard({
    required this.deviceSpace,
    required this.topology,
    required this.busy,
    required this.onViewTopology,
    required this.onRename,
    required this.onLeave,
    required this.onOpenProfile,
    required this.connectionMessage,
    required this.connectionFailed,
  });

  final generated.CoreSpace? deviceSpace;
  final generated.RuntimeDeviceSpaceTopology? topology;
  final bool busy;
  final VoidCallback onViewTopology;
  final VoidCallback onRename;
  final VoidCallback onLeave;
  final VoidCallback onOpenProfile;
  final String? connectionMessage;
  final bool connectionFailed;

  /// Builds the visual device-space overview shown at the top of settings.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final scheme = Theme.of(context).colorScheme;
    final space = deviceSpace;
    if (space == null) {
      return const _SectionCard(
        title: '设备空间',
        children: <Widget>[
          SizedBox(
            height: 190,
            child: Center(child: M3LoadingIndicator(size: 24)),
          ),
        ],
      );
    }
    final graph = topology;
    if (graph == null) {
      return const _SectionCard(
        title: '设备空间',
        children: <Widget>[
          SizedBox(
            height: 190,
            child: Center(child: M3LoadingIndicator(size: 24)),
          ),
        ],
      );
    }
    final devices = graph.devices;
    final onlineCount = devices.where((device) => device.online).length;
    final currentDevice = devices.firstWhere(
      (device) => device.deviceId == graph.currentDeviceId,
    );
    final remoteDevices = devices
        .where((device) => device.deviceId != graph.currentDeviceId)
        .take(3)
        .toList(growable: false);
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: Color.alphaBlend(
          scheme.primary.withValues(alpha: 0.08),
          scheme.surfaceContainerHighest.withValues(alpha: 0.42),
        ),
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: scheme.primary.withValues(alpha: 0.3)),
        material: true,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(18, 16, 18, 14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              _DeviceSpaceIdentityChip(onTap: onOpenProfile),
              const SizedBox(height: 14),
              Row(
                children: <Widget>[
                  Icon(Icons.hub_outlined, color: scheme.primary, size: 24),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Text(
                      l10n.settingsRuntimeCurrentSpace,
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w800,
                      ),
                    ),
                  ),
                  IconButton(
                    tooltip: l10n.settingsRuntimeRenameSpace,
                    onPressed: busy ? null : onRename,
                    icon: const Icon(Icons.edit_outlined),
                  ),
                  IconButton(
                    tooltip: l10n.settingsRuntimeLeaveSpace,
                    onPressed: busy || space.members.length <= 1
                        ? null
                        : onLeave,
                    icon: const Icon(Icons.logout_outlined),
                  ),
                ],
              ),
              const SizedBox(height: 2),
              Text(
                space.spaceName,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                  fontWeight: FontWeight.w800,
                  color: scheme.primary,
                ),
              ),
              const SizedBox(height: 2),
              Text(
                l10n.settingsRuntimeSpaceId(space.spaceId),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: scheme.onSurfaceVariant),
              ),
              const SizedBox(height: 8),
              Text(
                l10n.settingsRuntimeOverviewDescription,
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: scheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 8),
              Row(
                children: <Widget>[
                  Text('${space.members.length} 台设备'),
                  const SizedBox(width: 8),
                  Icon(Icons.circle, size: 8, color: scheme.primary),
                  const SizedBox(width: 5),
                  Text('$onlineCount 台在线'),
                ],
              ),
              const SizedBox(height: 4),
              SizedBox(
                height: 242,
                child: _AnimatedSpaceGraph(
                  color: scheme.primary,
                  currentDevice: currentDevice,
                  remoteDevices: remoteDevices,
                ),
              ),
              const Divider(height: 18),
              Row(
                children: <Widget>[
                  Expanded(
                    child: Text(
                      l10n.settingsRuntimeSpaceId(space.spaceId),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: scheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                  TextButton.icon(
                    onPressed: busy ? null : onViewTopology,
                    icon: const Icon(Icons.account_tree_outlined, size: 18),
                    label: Text(l10n.settingsRuntimeViewSpaceTopology),
                  ),
                ],
              ),
              if (connectionMessage != null) ...<Widget>[
                const SizedBox(height: 4),
                _InlineStatus(
                  message: connectionMessage!,
                  failed: connectionFailed,
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _DeviceSpaceIdentityChip extends StatelessWidget {
  const _DeviceSpaceIdentityChip({required this.onTap});

  final VoidCallback onTap;

  /// Builds the identity selector shown above the current-space title.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final identity = RuntimeBootstrapManager.instance.activeIdentity;
    final name = runtimeIdentityDisplayName(identity, l10n);
    final scheme = Theme.of(context).colorScheme;
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(22),
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: scheme.surfaceContainerHighest.withValues(alpha: 0.5),
          borderRadius: BorderRadius.circular(22),
          border: Border.all(
            color: scheme.outlineVariant.withValues(alpha: 0.4),
          ),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(8, 5, 11, 5),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              CircleAvatar(
                radius: 15,
                backgroundColor: scheme.primary.withValues(alpha: 0.15),
                child: Icon(
                  Icons.person_outline,
                  size: 19,
                  color: scheme.primary,
                ),
              ),
              const SizedBox(width: 8),
              Text(
                name,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(
                  context,
                ).textTheme.labelLarge?.copyWith(fontWeight: FontWeight.w700),
              ),
              const SizedBox(width: 8),
              Icon(
                Icons.chevron_right_rounded,
                size: 18,
                color: scheme.primary,
              ),
              const SizedBox(width: 4),
              Text(
                '用户与身份',
                style: Theme.of(context).textTheme.labelMedium?.copyWith(
                  color: scheme.primary,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _AnimatedSpaceGraph extends StatefulWidget {
  const _AnimatedSpaceGraph({
    required this.color,
    required this.currentDevice,
    required this.remoteDevices,
  });

  final Color color;
  final generated.RuntimeDeviceSpaceDevice currentDevice;
  final List<generated.RuntimeDeviceSpaceDevice> remoteDevices;

  /// Creates the ticker that drives orbit and connection motion.
  @override
  State<_AnimatedSpaceGraph> createState() => _AnimatedSpaceGraphState();
}

class _AnimatedSpaceGraphState extends State<_AnimatedSpaceGraph>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  /// Starts the continuous ambient motion for the device graph.
  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 12),
    )..repeat();
  }

  /// Releases the graph animation ticker.
  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  /// Builds animated orbit rings, connection dashes, and device nodes.
  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _controller,
      builder: (context, child) {
        final pulse = (math.sin(_controller.value * math.pi * 2) + 1) / 2;
        return Stack(
          fit: StackFit.expand,
          children: <Widget>[
            CustomPaint(
              painter: _SpaceOrbitPainter(
                color: widget.color,
                phase: _controller.value,
                pulse: pulse,
              ),
            ),
            Center(
              child: _SpaceDeviceNode(
                label: widget.currentDevice.deviceName,
                current: true,
                online: widget.currentDevice.online,
                pulse: pulse,
              ),
            ),
            for (var index = 0; index < widget.remoteDevices.length; index++)
              Align(
                alignment: index == 0
                    ? Alignment.centerRight
                    : index == 1
                    ? Alignment.centerLeft
                    : Alignment.topRight,
                child: Padding(
                  padding: EdgeInsets.only(
                    right: index == 0 ? 8 : 20,
                    left: index == 1 ? 8 : 0,
                    top: index == 2 ? 18 : 0,
                  ),
                  child: _SpaceDeviceNode(
                    label: widget.remoteDevices[index].deviceName,
                    current: false,
                    online: widget.remoteDevices[index].online,
                    pulse: pulse,
                  ),
                ),
              ),
          ],
        );
      },
    );
  }
}

class _SpaceDeviceNode extends StatelessWidget {
  const _SpaceDeviceNode({
    required this.label,
    required this.current,
    required this.online,
    required this.pulse,
  });

  final String label;
  final bool current;
  final bool online;
  final double pulse;

  /// Builds one glowing current-device node or compact remote node.
  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final accent = online ? scheme.primary : scheme.error;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Transform.scale(
          scale: 1 + (current ? pulse * 0.045 : pulse * 0.018),
          child: Container(
            width: current ? 88 : 52,
            height: current ? 88 : 52,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              gradient: current
                  ? RadialGradient(
                      colors: <Color>[
                        Colors.white.withValues(alpha: 0.95),
                        accent.withValues(alpha: 0.86),
                        const Color(0xff7750d9).withValues(alpha: 0.92),
                      ],
                      stops: const <double>[0.05, 0.48, 1],
                    )
                  : RadialGradient(
                      colors: <Color>[
                        Colors.white.withValues(alpha: 0.92),
                        const Color(0xffbbc0d3).withValues(alpha: 0.74),
                        scheme.surface.withValues(alpha: 0.42),
                      ],
                      stops: const <double>[0.06, 0.5, 1],
                    ),
              border: Border.all(
                color: accent.withValues(alpha: 0.78),
                width: current ? 2 : 1.5,
              ),
              boxShadow: current
                  ? <BoxShadow>[
                      BoxShadow(
                        color: accent.withValues(alpha: 0.42),
                        blurRadius: 22 + pulse * 12,
                        spreadRadius: 5 + pulse * 6,
                      ),
                    ]
                  : const <BoxShadow>[],
            ),
            child: Icon(
              current
                  ? Icons.laptop_mac_outlined
                  : Icons.devices_other_outlined,
              size: current ? 38 : 24,
              color: accent,
            ),
          ),
        ),
        const SizedBox(height: 6),
        ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 126),
          child: Text(
            label,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            textAlign: TextAlign.center,
            style: Theme.of(context).textTheme.labelMedium?.copyWith(
              fontWeight: current ? FontWeight.w800 : FontWeight.w600,
            ),
          ),
        ),
        if (current)
          Text(
            '当前设备',
            style: Theme.of(
              context,
            ).textTheme.labelSmall?.copyWith(color: scheme.onSurfaceVariant),
          ),
        if (!current)
          Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(Icons.circle, size: 7, color: accent),
              const SizedBox(width: 4),
              Text(online ? '在线' : '离线'),
            ],
          ),
      ],
    );
  }
}

class _SpaceOrbitPainter extends CustomPainter {
  const _SpaceOrbitPainter({
    required this.color,
    required this.phase,
    required this.pulse,
  });

  final Color color;
  final double phase;
  final double pulse;

  /// Draws the concentric orbit rings behind the device nodes.
  @override
  void paint(Canvas canvas, Size size) {
    final center = size.center(Offset.zero);
    final shortest = math.min(size.width, size.height);
    final paint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1
      ..color = color.withValues(alpha: 0.18);
    for (final factor in <double>[0.28, 0.48, 0.68, 0.88]) {
      canvas.drawCircle(center, shortest * factor / 2, paint);
    }
    final stars = <Offset>[
      Offset(size.width * 0.58, size.height * 0.18),
      Offset(size.width * 0.73, size.height * 0.34),
      Offset(size.width * 0.79, size.height * 0.78),
      Offset(size.width * 0.24, size.height * 0.68),
      Offset(size.width * 0.14, size.height * 0.37),
    ];
    final starPaint = Paint()
      ..color = color.withValues(alpha: 0.36 + pulse * 0.18);
    for (final star in stars) {
      canvas.drawCircle(star, 2.2, starPaint);
    }
    final dashPaint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.4
      ..color = color.withValues(alpha: 0.42);
    canvas.drawArc(
      Rect.fromCircle(center: center, radius: shortest * 0.36),
      -0.35,
      1.7,
      false,
      dashPaint,
    );
    final connectionPaint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.2
      ..color = color.withValues(alpha: 0.48);
    const dashLength = 7.0;
    const gapLength = 6.0;
    final dashOffset = phase * (dashLength + gapLength);
    final start = Offset(center.dx + shortest * 0.08, center.dy);
    final end = Offset(size.width * 0.85, center.dy);
    final delta = end - start;
    final distance = delta.distance;
    final direction = delta / distance;
    for (
      double traveled = -dashOffset;
      traveled < distance;
      traveled += dashLength + gapLength
    ) {
      final dashStart = start + direction * traveled;
      final dashEnd =
          start + direction * math.min(traveled + dashLength, distance);
      canvas.drawLine(dashStart, dashEnd, connectionPaint);
    }
  }

  /// Repaints the orbit when the active theme color changes.
  @override
  bool shouldRepaint(covariant _SpaceOrbitPainter oldDelegate) {
    return oldDelegate.color != color ||
        oldDelegate.phase != phase ||
        oldDelegate.pulse != pulse;
  }
}

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

class _DeviceSpaceTopologyDialog extends StatefulWidget {
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

  /// Creates state that keeps the visible topology synchronized.
  @override
  State<_DeviceSpaceTopologyDialog> createState() =>
      _DeviceSpaceTopologyDialogState();
}

class _DeviceSpaceTopologyDialogState
    extends State<_DeviceSpaceTopologyDialog> {
  late generated.RuntimeDeviceSpaceTopology _topology;

  /// Initializes the dialog with the topology loaded before opening.
  @override
  void initState() {
    super.initState();
    _topology = widget.topology;
  }

  /// Applies a refreshed topology to every visible dialog section.
  void _applyTopology(generated.RuntimeDeviceSpaceTopology topology) {
    setState(() => _topology = topology);
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
                          l10n.settingsRuntimeSpaceTopologyTitle(
                            widget.spaceName,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: Theme.of(context).textTheme.titleMedium
                              ?.copyWith(fontWeight: FontWeight.w800),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          l10n.settingsRuntimeSpaceTopologySummary(
                            _topology.devices.length,
                            _topology.connections.length,
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
                  topology: _topology,
                  onDisconnectDevice: widget.onDisconnectDevice,
                  onTopologyChanged: _applyTopology,
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
    required this.onTopologyChanged,
  });

  final generated.RuntimeDeviceSpaceTopology topology;
  final Future<generated.RuntimeDeviceSpaceTopology> Function(String deviceId)
  onDisconnectDevice;
  final ValueChanged<generated.RuntimeDeviceSpaceTopology> onTopologyChanged;

  /// Creates the state that handles edge selection without rebuilding the dialog.
  @override
  State<_DeviceSpaceTopologyGraph> createState() =>
      _DeviceSpaceTopologyGraphState();
}

class _DeviceSpaceTopologyGraphState extends State<_DeviceSpaceTopologyGraph> {
  /// Builds a pannable and zoomable canvas for the complete device graph.
  @override
  Widget build(BuildContext context) {
    final topology = widget.topology;
    return LayoutBuilder(
      builder: (context, constraints) {
        final viewportWidth = constraints.maxWidth;
        final viewportHeight = constraints.maxHeight;
        const nodeSize = Size(148, 112);
        final canvasSize = _topologyCanvasSize(
          viewportWidth,
          topology.devices.length,
          nodeSize,
        );
        final centers = _deviceCenters(
          canvasSize,
          topology.devices.length,
          nodeSize,
        );
        final centerByDeviceId = <String, Offset>{
          for (var index = 0; index < topology.devices.length; index++)
            topology.devices[index].deviceId: centers[index],
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
                          topology.connections,
                          nodeSize,
                        );
                        if (connection != null) {
                          unawaited(_showConnectionDetails(connection));
                        }
                      },
                      child: CustomPaint(
                        painter: _DeviceSpaceTopologyPainter(
                          centers: centerByDeviceId,
                          connections: topology.connections,
                          nodeSize: nodeSize,
                          onlineColor: colorScheme.outline,
                          offlineColor: colorScheme.error,
                          mismatchColor: colorScheme.tertiary,
                          unknownColor: colorScheme.outlineVariant,
                        ),
                      ),
                    ),
                  ),
                  for (var index = 0; index < topology.devices.length; index++)
                    Positioned(
                      left: centers[index].dx - nodeSize.width / 2,
                      top: centers[index].dy - nodeSize.height / 2,
                      width: nodeSize.width,
                      height: nodeSize.height,
                      child: _DeviceSpaceTopologyNode(
                        device: topology.devices[index],
                        current:
                            topology.devices[index].deviceId ==
                            topology.currentDeviceId,
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
    final topology = widget.topology;
    final devicesById = <String, generated.RuntimeDeviceSpaceDevice>{
      for (final device in topology.devices) device.deviceId: device,
    };
    final first = devicesById[connection.firstDeviceId]!;
    final second = devicesById[connection.secondDeviceId]!;
    final currentDeviceId = topology.currentDeviceId;
    final target = connection.firstDeviceId == currentDeviceId
        ? second
        : connection.secondDeviceId == currentDeviceId
        ? first
        : null;
    final canDisconnect = target != null;
    await showDialog<void>(
      context: context,
      builder: (context) {
        final colorScheme = Theme.of(context).colorScheme;
        return AlertDialog(
          title: Text('${first.deviceName} → ${second.deviceName}'),
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
      widget.onTopologyChanged(refreshedTopology);
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
  Size nodeSize,
) {
  const hitDistance = 16.0;
  generated.RuntimeDeviceSpaceConnection? nearest;
  var nearestDistance = double.infinity;
  for (final connection in connections) {
    final segment = _connectionSegment(
      connection,
      centers,
      connections,
      nodeSize,
    );
    final distance = _distanceToSegment(point, segment.start, segment.end);
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

/// Holds one drawable topology connection segment.
class _TopologyConnectionSegment {
  /// Creates one segment clipped to topology node bounds.
  const _TopologyConnectionSegment({required this.start, required this.end});

  final Offset start;
  final Offset end;
}

/// Calculates a drawable directed segment between two device node bounds.
_TopologyConnectionSegment _connectionSegment(
  generated.RuntimeDeviceSpaceConnection connection,
  Map<String, Offset> centers,
  List<generated.RuntimeDeviceSpaceConnection> connections,
  Size nodeSize,
) {
  final sourceCenter = centers[connection.firstDeviceId]!;
  final targetCenter = centers[connection.secondDeviceId]!;
  final direction = targetCenter - sourceCenter;
  final distance = direction.distance;
  if (distance == 0) {
    throw StateError('device-space topology contains a zero-length connection');
  }
  final unit = direction / distance;
  final sideOffset =
      Offset(-unit.dy, unit.dx) *
      _parallelConnectionOffset(connection, connections);
  final shiftedSourceCenter = sourceCenter + sideOffset;
  final shiftedTargetCenter = targetCenter + sideOffset;
  final sourceRect = _topologyNodeRect(sourceCenter, nodeSize);
  final targetRect = _topologyNodeRect(targetCenter, nodeSize);
  const nodeGap = 6.0;
  final start =
      _pointOnRectBoundary(
        sourceRect,
        shiftedSourceCenter,
        shiftedTargetCenter,
      ) +
      unit * nodeGap;
  final end =
      _pointOnRectBoundary(
        targetRect,
        shiftedTargetCenter,
        shiftedSourceCenter,
      ) -
      unit * nodeGap;
  return _TopologyConnectionSegment(start: start, end: end);
}

/// Returns the rectangle occupied by one topology node.
Rect _topologyNodeRect(Offset center, Size nodeSize) {
  return Rect.fromCenter(
    center: center,
    width: nodeSize.width,
    height: nodeSize.height,
  );
}

/// Calculates the point where a ray exits a node rectangle.
Offset _pointOnRectBoundary(Rect rect, Offset inside, Offset outside) {
  final delta = outside - inside;
  final candidates = <double>[];
  if (delta.dx > 0) {
    candidates.add((rect.right - inside.dx) / delta.dx);
  }
  if (delta.dx < 0) {
    candidates.add((rect.left - inside.dx) / delta.dx);
  }
  if (delta.dy > 0) {
    candidates.add((rect.bottom - inside.dy) / delta.dy);
  }
  if (delta.dy < 0) {
    candidates.add((rect.top - inside.dy) / delta.dy);
  }
  final positiveCandidates = candidates
      .where((candidate) => candidate > 0)
      .toList(growable: false);
  if (positiveCandidates.isEmpty) {
    throw StateError('device-space topology edge cannot reach node boundary');
  }
  final scale = positiveCandidates.reduce(math.min);
  return inside + delta * scale;
}

/// Returns a side offset for opposite directed connections between two devices.
double _parallelConnectionOffset(
  generated.RuntimeDeviceSpaceConnection connection,
  List<generated.RuntimeDeviceSpaceConnection> connections,
) {
  final hasOpposite = connections.any(
    (other) =>
        other.firstDeviceId == connection.secondDeviceId &&
        other.secondDeviceId == connection.firstDeviceId,
  );
  if (!hasOpposite) {
    return 0;
  }
  final currentKey =
      '${connection.firstDeviceId}\u0000${connection.secondDeviceId}';
  final oppositeKey =
      '${connection.secondDeviceId}\u0000${connection.firstDeviceId}';
  return currentKey.compareTo(oppositeKey) < 0 ? -8.0 : 8.0;
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
    required this.nodeSize,
    required this.onlineColor,
    required this.offlineColor,
    required this.mismatchColor,
    required this.unknownColor,
  });

  final Map<String, Offset> centers;
  final List<generated.RuntimeDeviceSpaceConnection> connections;
  final Size nodeSize;
  final Color onlineColor;
  final Color offlineColor;
  final Color mismatchColor;
  final Color unknownColor;

  /// Draws every directed direct-device edge and marks unhealthy links with an X.
  @override
  void paint(Canvas canvas, Size size) {
    for (final connection in connections) {
      final segment = _connectionSegment(
        connection,
        centers,
        connections,
        nodeSize,
      );
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
        _drawArrowLine(canvas, segment.start, segment.end, paint);
      } else {
        _drawDashedLine(canvas, segment.start, segment.end, paint);
        _drawArrowHead(canvas, segment.start, segment.end, paint.color);
      }
      if (connection.status ==
              generated.RuntimeDeviceSpaceConnectionStatus.offline ||
          connection.status ==
              generated.RuntimeDeviceSpaceConnectionStatus.versionMismatch) {
        _drawConnectionCross(
          canvas,
          Offset.lerp(segment.start, segment.end, 0.5)!,
          color,
        );
      }
    }
  }

  /// Draws a solid directed segment with an arrow head at the target side.
  static void _drawArrowLine(
    Canvas canvas,
    Offset first,
    Offset second,
    Paint paint,
  ) {
    canvas.drawLine(first, second, paint);
    _drawArrowHead(canvas, first, second, paint.color);
  }

  /// Draws the arrow head for one directed topology connection.
  static void _drawArrowHead(
    Canvas canvas,
    Offset first,
    Offset second,
    Color color,
  ) {
    final direction = second - first;
    final length = direction.distance;
    if (length == 0) {
      throw StateError(
        'device-space topology contains a zero-length connection',
      );
    }
    final unit = direction / length;
    final normal = Offset(-unit.dy, unit.dx);
    const arrowLength = 12.0;
    const arrowWidth = 6.0;
    final path = Path()
      ..moveTo(second.dx, second.dy)
      ..lineTo(
        second.dx - unit.dx * arrowLength + normal.dx * arrowWidth,
        second.dy - unit.dy * arrowLength + normal.dy * arrowWidth,
      )
      ..lineTo(
        second.dx - unit.dx * arrowLength - normal.dx * arrowWidth,
        second.dy - unit.dy * arrowLength - normal.dy * arrowWidth,
      )
      ..close();
    canvas.drawPath(
      path,
      Paint()
        ..color = color
        ..style = PaintingStyle.fill,
    );
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
        oldDelegate.nodeSize != nodeSize ||
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
    required this.onRemovedFromSpace,
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
  final VoidCallback onRemovedFromSpace;

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
            onRemovedFromSpace: onRemovedFromSpace,
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
    required this.onRemovedFromSpace,
  });

  final generated.RuntimePairedDevice device;
  final bool busy;
  final _PairedRemoteProbeState? state;
  final bool inCurrentSpace;
  final VoidCallback? onJoin;
  final VoidCallback onDelete;
  final ValueChanged<generated.LinkTransportPreference> onTransportChanged;
  final VoidCallback onRemovedFromSpace;

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
      _PairedRemoteProbeState.invalid => colorScheme.error,
      _PairedRemoteProbeState.error => colorScheme.error,
      _PairedRemoteProbeState.removedFromSpace => colorScheme.error,
    };
    final canJoin =
        !inCurrentSpace &&
        onJoin != null &&
        probeState != _PairedRemoteProbeState.invalid &&
        probeState != _PairedRemoteProbeState.removedFromSpace;
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
            if (canJoin)
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
          if (probeState == _PairedRemoteProbeState.removedFromSpace)
            Align(
              alignment: Alignment.centerLeft,
              child: FilledButton.tonalIcon(
                onPressed: busy ? null : onRemovedFromSpace,
                icon: const Icon(Icons.person_remove_outlined, size: 18),
                label: Text(l10n.settingsRuntimeRemovedFromSpaceConfirm),
              ),
            ),
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
      _PairedRemoteProbeState.invalid => Icon(
        Icons.link_off_outlined,
        color: colorScheme.error,
      ),
      _PairedRemoteProbeState.error => Icon(
        Icons.error_outline,
        color: colorScheme.error,
      ),
      _PairedRemoteProbeState.removedFromSpace => Icon(
        Icons.person_remove_outlined,
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
      _PairedRemoteProbeState.invalid => l10n.settingsRuntimePairedInvalid,
      _PairedRemoteProbeState.error => l10n.settingsRuntimePairedError,
      _PairedRemoteProbeState.removedFromSpace =>
        l10n.settingsRuntimePairedRemovedFromSpace,
    };
    final color = switch (state) {
      _PairedRemoteProbeState.checking => colorScheme.onSurfaceVariant,
      _PairedRemoteProbeState.online => colorScheme.primary,
      _PairedRemoteProbeState.offline => colorScheme.error,
      _PairedRemoteProbeState.invalid => colorScheme.error,
      _PairedRemoteProbeState.error => colorScheme.error,
      _PairedRemoteProbeState.removedFromSpace => colorScheme.error,
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
