// ignore_for_file: file_names

import 'dart:async';
import 'package:flutter/material.dart';

import '../../../common/components/PageActivityMixin.dart';

import '../../../../core/bridge/PlatformCoreProxy.dart';
import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as generated;
import '../../../../core/runtime/RuntimeBootstrapManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/DeviceSpaceDiscoveryPanel.dart';
import '../../../common/SpaceJoinWidgets.dart';
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../../../theme/OperitTheme.dart';
import '../components/SettingsControlStyles.dart';
import '../profile/UserProfileSummaryTile.dart';
import 'DeviceSpaceGraph.dart';
import 'NetworkControlPanel.dart';

String _deviceInfoName(generated.LinkDeviceInfo info) {
  final parts = [
    info.platform.trim(),
    info.model.trim(),
  ].where((part) => part.isNotEmpty).toList();
  return parts.isEmpty ? 'Unknown device' : parts.join('-');
}

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

class _RuntimeSettingsPanelState extends State<RuntimeSettingsPanel>
    with PageActivityMixin<RuntimeSettingsPanel> {
  bool _busy = false;
  int _pageGeneration = 0;
  Map<String, generated.SpaceJoinRequest> _spaceJoins = {};
  String? _connectionMessage;
  bool _connectionFailed = false;
  generated.CoreSpace? _currentDeviceSpace;
  generated.RuntimeDeviceSpaceTopology? _topology;
  generated.NetworkControlState? _control;
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

  /// Watches device presentation state only while this page is active.
  @override
  void onPageActivityChanged(bool active) {
    _pageGeneration += 1;
    if (!active) {
      _pairedDevicesSubscription?.cancel();
      _pairedDevicesSubscription = null;
      _pairedDeviceStatusesSubscription?.cancel();
      _pairedDeviceStatusesSubscription = null;
      return;
    }
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
    if (!mounted || !isPageActive) {
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
    unawaited(_refreshCurrentDeviceSpace());
  }

  /// Reads the synchronized device space projection from the current device.
  Future<void> _refreshCurrentDeviceSpace() async {
    if (!mounted || !isPageActive) return;
    final generation = _pageGeneration;
    try {
      final deviceSpace = await _clients.server.runtimeRemoteLinkService
          .deviceSpace();
      if (!mounted || !isPageActive || generation != _pageGeneration) return;
      final topology = await _clients.server.runtimeRemoteLinkService
          .deviceSpaceTopology();
      if (!mounted || !isPageActive || generation != _pageGeneration) return;
      final control = await _clients.server.runtimeRemoteLinkService
          .deviceSpaceControl();
      if (mounted && isPageActive && generation == _pageGeneration) {
        setState(() {
          _currentDeviceSpace = deviceSpace;
          _topology = topology;
          _control = control;
        });
      }
    } catch (error) {
      if (mounted && isPageActive && generation == _pageGeneration) {
        setState(() {
          _connectionMessage = error.toString();
          _connectionFailed = true;
        });
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

  /// Builds one management row for every Space member without a direct pairing.
  ///
  /// Membership and direct pairing are independent axes: a relayed member stays visible
  /// and manageable here even though this device never paired with it.
  List<Widget> _unpairedSpaceMemberTiles(AppLocalizations l10n) {
    final topology = _topology;
    if (topology == null) {
      return const <Widget>[];
    }
    final members = topology.devices
        .where(
          (device) =>
              device.deviceId != topology.currentDeviceId &&
              !_pairedDevices.containsKey(device.deviceId),
        )
        .toList(growable: false);
    if (members.isEmpty) {
      return const <Widget>[];
    }
    final names = <String, String>{
      for (final device in topology.devices) device.deviceId: device.deviceName,
    };
    return <Widget>[
      for (final device in members) ...<Widget>[
        const SizedBox(height: 10),
        _DeviceMemberTile(
          device: device,
          statusText: _memberStatusText(l10n, device),
          pathText: _memberPathText(l10n, names, device),
          actions: _memberManagementMenu(l10n, device),
        ),
      ],
    ];
  }

  /// Describes how one member is currently reached: directly, through relays, or not at all.
  String _memberStatusText(
    AppLocalizations l10n,
    generated.RuntimeDeviceSpaceDevice device,
  ) {
    if (!device.online) {
      return l10n.settingsRuntimePairedOffline;
    }
    final hops = device.relayHops;
    if (hops == null) {
      return '${l10n.settingsRuntimePairedOnline} · ${l10n.deviceSpaceDirectLink}';
    }
    return '${l10n.settingsRuntimePairedOnline} · ${l10n.deviceSpaceRelayHops(hops)}';
  }

  /// Renders the planned relay path from this device through every hop to the member.
  String? _memberPathText(
    AppLocalizations l10n,
    Map<String, String> names,
    generated.RuntimeDeviceSpaceDevice device,
  ) {
    final path = device.relayPath ?? const <String>[];
    if (path.isEmpty) {
      return null;
    }
    return '${l10n.deviceSpaceRelayPath}: ${[
      l10n.deviceSpaceLocalDevice,
      for (final hop in path) names[hop] ?? hop,
    ].join(' → ')}';
  }

  /// Builds the device-management menu offered by the current identity's capabilities.
  Widget? _memberManagementMenu(
    AppLocalizations l10n,
    generated.RuntimeDeviceSpaceDevice device,
  ) {
    final actions = <Widget>[
      if (_localHasCapability('network.members.join'))
        MenuItemButton(
          onPressed: () => _runDeviceCommand(
            () => _clients.server.runtimeRemoteLinkService
                .admitDeviceSpaceMember(deviceId: device.deviceId),
            l10n.settingsRuntimeControlDeviceAdmitted,
          ),
          child: Text(l10n.settingsRuntimeControlAdmitDevice),
        ),
      if (device.currentIdentity != null &&
          _localHasCapability('network.identity.manage'))
        MenuItemButton(
          onPressed: () => _runDeviceCommand(
            () => _clients.server.runtimeRemoteLinkService
                .clearDeviceSpaceIdentity(nodeId: device.deviceId),
            l10n.settingsRuntimeControlIdentityResetDone,
          ),
          child: Text(l10n.settingsRuntimeControlClearIdentity),
        ),
      if (_localHasCapability('network.identity.assign') &&
          _assignableRoles().isNotEmpty)
        MenuItemButton(
          onPressed: () => unawaited(_assignMemberIdentity(device, l10n)),
          child: Text(l10n.settingsRuntimeControlAssignIdentity),
        ),
      if (_localHasCapability('network.connections.disconnect'))
        MenuItemButton(
          onPressed: () => _runDeviceCommand(
            () => _clients.server.runtimeRemoteLinkService
                .disconnectDeviceSpaceNode(deviceId: device.deviceId),
            l10n.settingsRuntimeControlDisconnected,
          ),
          child: Text(l10n.settingsRuntimeControlDisconnectDevice),
        ),
      if (_localHasCapability('network.members.remove'))
        MenuItemButton(
          onPressed: () => _runDeviceCommand(
            () => _clients.server.runtimeRemoteLinkService
                .removeDeviceSpaceMember(deviceId: device.deviceId),
            l10n.settingsRuntimeControlRemoved,
          ),
          child: Text(l10n.settingsRuntimeControlRemoveDevice),
        ),
    ];
    if (actions.isEmpty) {
      return null;
    }
    return MenuAnchor(
      builder: (context, controller, _) => IconButton(
        tooltip: l10n.deviceSpaceManageDevice,
        onPressed: controller.open,
        icon: const Icon(Icons.more_vert_outlined),
      ),
      menuChildren: actions,
    );
  }

  /// Reports whether the current device identity holds one capability.
  bool _localHasCapability(String capability) {
    final topology = _topology;
    if (topology == null) {
      return false;
    }
    for (final device in topology.devices) {
      if (device.deviceId != topology.currentDeviceId) {
        continue;
      }
      final identity = device.currentIdentity;
      if (identity == null) {
        return false;
      }
      return identity.capabilities.contains('*') ||
          identity.capabilities.contains(capability);
    }
    return false;
  }

  /// Lists the identities this device may assign: every capability must already be held.
  List<generated.NetworkControlRole> _assignableRoles() {
    final control = _control;
    if (control == null) {
      return const <generated.NetworkControlRole>[];
    }
    return control.roles.values
        .where(
          (role) => _capabilityValues(
            role.capabilities,
          ).every(_localHasCapability),
        )
        .toList(growable: false);
  }

  /// Executes one device-management command and refreshes the projection.
  Future<void> _runDeviceCommand(
    Future<void> Function() command,
    String feedback,
  ) async {
    try {
      await command();
      await _refreshCurrentDeviceSpace();
      if (mounted) {
        _showDeviceFeedback(feedback);
      }
    } catch (error) {
      if (mounted) {
        _showDeviceFeedback(error.toString());
      }
    }
  }

  /// Assigns one of the locally assignable identities to a Space member.
  Future<void> _assignMemberIdentity(
    generated.RuntimeDeviceSpaceDevice device,
    AppLocalizations l10n,
  ) async {
    final roles = _assignableRoles();
    if (roles.isEmpty) {
      return;
    }
    final roleId = await showDialog<String>(
      context: context,
      builder: (dialogContext) => SimpleDialog(
        title: Text(l10n.settingsRuntimeControlAssignIdentity),
        children: <Widget>[
          for (final role in roles)
            SimpleDialogOption(
              onPressed: () => Navigator.of(dialogContext).pop(role.roleId),
              child: Text(role.displayName),
            ),
        ],
      ),
    );
    if (roleId == null || !mounted) {
      return;
    }
    await _runDeviceCommand(
      () => _clients.server.runtimeRemoteLinkService.setDeviceSpaceIdentity(
        assignment: generated.NetworkControlIdentityAssignment(
          nodeId: device.deviceId,
          roleId: roleId,
        ),
      ),
      l10n.settingsRuntimeControlIdentityAssignment,
    );
  }

  /// Surfaces one device-management outcome without leaving the settings page.
  void _showDeviceFeedback(String message) {
    ScaffoldMessenger.maybeOf(
      context,
    )?.showSnackBar(SnackBar(content: Text(message)));
  }

  /// Normalizes the generated identity capability container into plain strings.
  List<String> _capabilityValues(Object? capabilities) {
    if (capabilities is Iterable<Object?>) {
      return capabilities
          .map((capability) => capability.toString())
          .toList(growable: false);
    }
    return const <String>[];
  }

  /// Applies one paired-device snapshot without network probing.
  void _applyPairedDevices(Map<String, generated.RuntimePairedDevice> devices) {
    if (!mounted || !isPageActive) {
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
    if (!device.outbound) {
      throw StateError('joining a device space requires an outbound pairing');
    }
    final deviceInfo = device.deviceInfo;
    setState(() => _busy = true);
    try {
      final joined = await confirmAndJoinPairedDeviceSpace(
        context: context,
        clients: _clients,
        deviceId: device.deviceId,
        deviceName: _deviceInfoName(deviceInfo),
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

  void _applySpaceJoins(List<generated.SpaceJoinRequest> requests) {
    final next = <String, generated.SpaceJoinRequest>{};
    for (final request in requests) {
      final old = next[request.targetDeviceId];
      if (old == null || request.createdAt > old.createdAt) {
        next[request.targetDeviceId] = request;
      }
    }
    if (next.length == _spaceJoins.length &&
        next.entries.every((e) {
          final old = _spaceJoins[e.key];
          return old?.requestId == e.value.requestId &&
              old?.status == e.value.status &&
              old?.assignmentVersion == e.value.assignmentVersion &&
              old?.reviewerName == e.value.reviewerName;
        })) {
      return;
    }
    if (mounted) setState(() => _spaceJoins = next);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final children = <Widget>[
      _DeviceSpaceOverviewCard(
        deviceSpace: _currentDeviceSpace,
        topology: _topology,
        busy: _busy,
        onRename: _renameCurrentDeviceSpace,
        onLeave: _leaveCurrentDeviceSpace,
        onOpenProfile: widget.onOpenProfile,
        onDisconnectDevice: _disconnectDeviceSpaceConnection,
        connectionMessage: _connectionMessage,
        connectionFailed: _connectionFailed,
      ),
      _SectionCard(
        title: l10n.deviceSpaceDevices,
        headerActions: DeviceSpaceDiscoveryPanel(
          clients: _clients,
          enabled: !_busy,
          onJoined: _handleJoinedDeviceSpace,
          onBusyChanged: _handleDiscoveryBusyChanged,
          onRequestsChanged: _applySpaceJoins,
        ),
        children: <Widget>[
          _PairedDeviceList(
            devices: _pairedDevices,
            busy: _busy,
            states: _pairedRemoteStates,
            requests: _spaceJoins,
            onRequest: (request) async {
              final space = await showSpaceJoinProgress(
                context,
                clients: _clients,
                request: request,
              );
              if (mounted && space != null) {
                await _handleJoinedDeviceSpace(space);
              }
            },
            currentMemberIds:
                _currentDeviceSpace?.members.toSet() ?? <String>{},
            onJoin: _offerJoiningExistingPairedDeviceSpace,
            onDelete: _deletePairedDevice,
            onRemovedFromSpace: _handleRemovedFromSpace,
          ),
          ..._unpairedSpaceMemberTiles(l10n),
        ],
      ),
      _SectionCard(
        title: l10n.settingsRuntimeNetworkControl,
        children: <Widget>[
          NetworkControlPanel(
            clients: _clients,
            onChanged: _refreshCurrentDeviceSpace,
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
    required this.onRename,
    required this.onLeave,
    required this.onOpenProfile,
    required this.onDisconnectDevice,
    required this.connectionMessage,
    required this.connectionFailed,
  });

  final generated.CoreSpace? deviceSpace;
  final generated.RuntimeDeviceSpaceTopology? topology;
  final bool busy;
  final VoidCallback onRename;
  final VoidCallback onLeave;
  final VoidCallback onOpenProfile;
  final Future<generated.RuntimeDeviceSpaceTopology> Function(String deviceId)
  onDisconnectDevice;
  final String? connectionMessage;
  final bool connectionFailed;

  /// Builds the visual device-space overview shown at the top of settings.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final scheme = Theme.of(context).colorScheme;
    final space = deviceSpace;
    if (space == null) {
      return _SectionCard(
        title: l10n.settingsRuntimeCurrentSpace,
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
      return _SectionCard(
        title: l10n.settingsRuntimeCurrentSpace,
        children: <Widget>[
          SizedBox(
            height: 190,
            child: Center(child: M3LoadingIndicator(size: 24)),
          ),
        ],
      );
    }
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: scheme.surface.withValues(alpha: 0.88),
        borderRadius: BorderRadius.circular(24),
        border: Border.all(
          color: scheme.outlineVariant.withValues(alpha: 0.24),
        ),
        material: true,
        child: LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 568;
            return Padding(
              padding: compact
                  ? const EdgeInsets.all(10)
                  : const EdgeInsets.fromLTRB(14, 14, 14, 12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.center,
                    children: <Widget>[
                      Expanded(
                        child: _DeviceSpaceIdentityChip(onTap: onOpenProfile),
                      ),
                      const SizedBox(width: 8),
                      PopupMenuButton<_DeviceSpaceMenuAction>(
                        tooltip: '空间操作',
                        onSelected: (action) {
                          switch (action) {
                            case _DeviceSpaceMenuAction.rename:
                              onRename();
                            case _DeviceSpaceMenuAction.leave:
                              onLeave();
                          }
                        },
                        itemBuilder: (context) =>
                            <PopupMenuEntry<_DeviceSpaceMenuAction>>[
                              PopupMenuItem<_DeviceSpaceMenuAction>(
                                value: _DeviceSpaceMenuAction.rename,
                                enabled: !busy,
                                child: ListTile(
                                  contentPadding: EdgeInsets.zero,
                                  leading: const Icon(Icons.edit_outlined),
                                  title: Text(l10n.settingsRuntimeRenameSpace),
                                ),
                              ),
                              PopupMenuItem<_DeviceSpaceMenuAction>(
                                value: _DeviceSpaceMenuAction.leave,
                                enabled: !busy && space.members.length > 1,
                                child: ListTile(
                                  contentPadding: EdgeInsets.zero,
                                  leading: const Icon(Icons.logout_outlined),
                                  title: Text(l10n.settingsRuntimeLeaveSpace),
                                ),
                              ),
                            ],
                        child: const Icon(Icons.more_horiz_rounded),
                      ),
                    ],
                  ),
                  SizedBox(height: compact ? 8 : 12),
                  DeviceSpaceGraph(
                    topology: graph,
                    onDisconnectDevice: onDisconnectDevice,
                    busy: busy,
                  ),
                  if (connectionMessage != null) ...<Widget>[
                    const SizedBox(height: 8),
                    _InlineStatus(
                      message: connectionMessage!,
                      failed: connectionFailed,
                    ),
                  ],
                ],
              ),
            );
          },
        ),
      ),
    );
  }
}

enum _DeviceSpaceMenuAction { rename, leave }

class _DeviceSpaceIdentityChip extends StatefulWidget {
  const _DeviceSpaceIdentityChip({required this.onTap});

  final VoidCallback onTap;

  @override
  State<_DeviceSpaceIdentityChip> createState() =>
      _DeviceSpaceIdentityChipState();
}

class _DeviceSpaceIdentityChipState extends State<_DeviceSpaceIdentityChip> {
  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(coreProxy: platformCoreProxy),
  );

  String? _githubAvatarUrl;
  String? _avatarLookupKey;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _resolveFallbackAvatar();
  }

  void _resolveFallbackAvatar() {
    final customAvatarUri = OperitTheme.of(
      context,
    ).themePreferenceSnapshot.customUserAvatarUri?.trim();
    final identityId = RuntimeBootstrapManager.instance.activeIdentity.id;
    final lookupKey = '$identityId|${customAvatarUri ?? ''}';
    if (_avatarLookupKey == lookupKey) {
      return;
    }
    _avatarLookupKey = lookupKey;
    if (customAvatarUri != null && customAvatarUri.isNotEmpty) {
      if (_githubAvatarUrl != null && mounted) {
        setState(() => _githubAvatarUrl = null);
      }
      return;
    }
    unawaited(_loadGithubAvatar());
  }

  Future<void> _loadGithubAvatar() async {
    try {
      final user = await _clients.preferencesGitHubAuthPreferences
          .getCurrentUserInfo();
      final avatarUrl = user?.avatarUrl.trim();
      if (!mounted) {
        return;
      }
      setState(() {
        _githubAvatarUrl = avatarUrl == null || avatarUrl.isEmpty
            ? null
            : avatarUrl;
      });
    } catch (_) {
      if (mounted) {
        setState(() => _githubAvatarUrl = null);
      }
    }
  }

  /// Builds the identity selector shown above the current-space title.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final identity = RuntimeBootstrapManager.instance.activeIdentity;
    final name = runtimeIdentityDisplayName(identity, l10n);
    final scheme = Theme.of(context).colorScheme;
    final customAvatarUri = OperitTheme.of(
      context,
    ).themePreferenceSnapshot.customUserAvatarUri;
    final suffix = Localizations.localeOf(context).languageCode == 'zh'
        ? '的设备空间'
        : l10n.settingsRuntimeCurrentSpace;
    return LayoutBuilder(
      builder: (context, constraints) {
        return InkWell(
          onTap: widget.onTap,
          borderRadius: BorderRadius.circular(20),
          child: ConstrainedBox(
            constraints: BoxConstraints(
              maxWidth: constraints.hasBoundedWidth
                  ? constraints.maxWidth
                  : double.infinity,
            ),
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: scheme.surfaceContainerHighest.withValues(alpha: 0.5),
                borderRadius: BorderRadius.circular(20),
                border: Border.all(
                  color: scheme.outlineVariant.withValues(alpha: 0.4),
                ),
              ),
              child: Padding(
                padding: const EdgeInsets.fromLTRB(7, 4, 9, 4),
                child: Row(
                  children: <Widget>[
                    _IdentityChipAvatar(
                      customAvatarUri: customAvatarUri,
                      githubAvatarUrl: _githubAvatarUrl,
                    ),
                    const SizedBox(width: 10),
                    Flexible(
                      flex: 2,
                      child: Text(
                        name,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.titleSmall?.copyWith(
                          color: scheme.primary,
                          fontWeight: FontWeight.w800,
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Flexible(
                      flex: 3,
                      child: Text(
                        suffix,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.titleSmall?.copyWith(
                          color: scheme.onSurface,
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

class _IdentityChipAvatar extends StatelessWidget {
  const _IdentityChipAvatar({
    required this.customAvatarUri,
    required this.githubAvatarUrl,
  });

  final String? customAvatarUri;
  final String? githubAvatarUrl;

  @override
  Widget build(BuildContext context) {
    final customPath = customAvatarUri?.trim();
    if (customPath != null && customPath.isNotEmpty) {
      return UserProfileAvatar(storagePath: customPath, size: 40);
    }
    final githubUrl = githubAvatarUrl?.trim();
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      width: 40,
      height: 40,
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: colorScheme.surfaceContainerHighest,
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.45),
        ),
      ),
      child: githubUrl == null || githubUrl.isEmpty
          ? Icon(Icons.person_outline, size: 24, color: colorScheme.primary)
          : Image.network(
              githubUrl,
              fit: BoxFit.cover,
              errorBuilder: (context, error, stackTrace) => Icon(
                Icons.person_outline,
                size: 24,
                color: colorScheme.primary,
              ),
            ),
    );
  }
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({
    required this.title,
    required this.children,
    this.headerActions,
  });

  final String title;
  final List<Widget> children;
  final Widget? headerActions;

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
              Row(
                children: [
                  Expanded(
                    child: Text(
                      title,
                      style: SettingsControlStyles.sectionTitleTextStyle(
                        context,
                      ),
                    ),
                  ),
                  ?headerActions,
                ],
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
    required this.requests,
    required this.onRequest,
    required this.currentMemberIds,
    required this.onJoin,
    required this.onDelete,
    required this.onRemovedFromSpace,
  });

  final Map<String, generated.RuntimePairedDevice> devices;
  final bool busy;
  final Map<String, _PairedRemoteProbeState> states;
  final Map<String, generated.SpaceJoinRequest> requests;
  final ValueChanged<generated.SpaceJoinRequest> onRequest;
  final Set<String> currentMemberIds;
  final ValueChanged<generated.RuntimePairedDevice> onJoin;
  final ValueChanged<String> onDelete;

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
            request: requests[entries[index].key],
            onRequest: () {
              final request = requests[entries[index].key];
              if (request != null) onRequest(request);
            },
            inCurrentSpace: currentMemberIds.contains(entries[index].key),
            onJoin: !entries[index].value.outbound
                ? null
                : () => onJoin(entries[index].value),
            onDelete: () => onDelete(entries[index].key),
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
    required this.request,
    required this.onRequest,
    required this.onJoin,
    required this.onDelete,
    required this.onRemovedFromSpace,
  });

  final generated.RuntimePairedDevice device;
  final bool busy;
  final _PairedRemoteProbeState? state;
  final bool inCurrentSpace;
  final generated.SpaceJoinRequest? request;
  final VoidCallback onRequest;
  final VoidCallback? onJoin;
  final VoidCallback onDelete;
  final VoidCallback onRemovedFromSpace;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final probeState = state ?? _PairedRemoteProbeState.checking;
    final statusColor = switch (probeState) {
      _PairedRemoteProbeState.checking => colorScheme.onSurfaceVariant,
      _PairedRemoteProbeState.online => colorScheme.primary,
      _PairedRemoteProbeState.offline => colorScheme.error,
      _PairedRemoteProbeState.invalid => colorScheme.error,
      _PairedRemoteProbeState.error => colorScheme.error,
      _PairedRemoteProbeState.removedFromSpace => colorScheme.error,
    };
    final pendingJoin = request != null && spaceJoinIsActive(request!.status);
    final canJoin =
        !pendingJoin &&
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
                    _deviceInfoName(device.deviceInfo),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  if (pendingJoin)
                    Padding(
                      padding: const EdgeInsets.only(top: 3),
                      child: Text(
                        '${spaceJoinStatusText(request!, l10n)}${request!.reviewerName == null ? '' : ' · ${request!.reviewerName}'}',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: textTheme.bodySmall?.copyWith(
                          color: colorScheme.primary,
                        ),
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
                          device.deviceId,
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
            if (pendingJoin)
              IconButton(
                tooltip: l10n.deviceSpaceViewRequest,
                icon: const Icon(Icons.schedule_rounded, size: 20),
                onPressed: busy ? null : onRequest,
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

/// Renders one Space member that has no direct pairing with the current device.
///
/// The tile keeps relayed members first-class: it shows how the member is reached and
/// hosts the same management surface used for paired devices.
class _DeviceMemberTile extends StatelessWidget {
  const _DeviceMemberTile({
    required this.device,
    required this.statusText,
    required this.pathText,
    required this.actions,
  });

  final generated.RuntimeDeviceSpaceDevice device;
  final String statusText;
  final String? pathText;
  final Widget? actions;

  /// Builds the member row.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Container(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.22),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.45),
        ),
      ),
      padding: const EdgeInsets.fromLTRB(12, 10, 6, 10),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Padding(
            padding: const EdgeInsets.only(top: 2),
            child: Icon(
              device.online ? Icons.lan_outlined : Icons.link_off_outlined,
              color: device.online ? colorScheme.primary : colorScheme.error,
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  device.deviceName,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.labelLarge?.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  '$statusText · ${l10n.deviceSpaceMemberNotPaired}',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.labelSmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
                if (pathText != null) ...<Widget>[
                  const SizedBox(height: 2),
                  Text(
                    pathText!,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: textTheme.labelSmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ],
            ),
          ),
          ?actions,
        ],
      ),
    );
  }
}
