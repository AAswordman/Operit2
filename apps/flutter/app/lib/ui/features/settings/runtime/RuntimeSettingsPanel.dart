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
import 'SpaceIdentityLabels.dart';

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
    this.clients,
  });

  final bool embedded;
  final VoidCallback onOpenProfile;

  /// Supplies Core clients for tests; the platform link is used by default.
  final GeneratedCoreProxyClients? clients;

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

  late final GeneratedCoreProxyClients _clients =
      widget.clients ??
      const GeneratedCoreProxyClients(
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

  /// Builds the device rows of the device section.
  ///
  /// Membership and direct pairing are independent axes: a relayed member stays
  /// visible here even though this device never paired with it.
  List<Widget> _deviceRows(AppLocalizations l10n) {
    final names = _deviceNames();
    return <Widget>[
      ..._pairedRows(l10n, names),
      ..._memberRows(l10n, names),
    ];
  }

  /// Renders the device section content, or the empty hint without any device.
  List<Widget> _deviceSectionChildren(AppLocalizations l10n) {
    final rows = _deviceRows(l10n);
    if (rows.isEmpty) {
      return <Widget>[
        Text(
          l10n.settingsRuntimeNoPairedRemote,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        ),
      ];
    }
    return <Widget>[
      for (var index = 0; index < rows.length; index++) ...<Widget>[
        if (index > 0) const SizedBox(height: 10),
        rows[index],
      ],
    ];
  }

  /// Maps device identifiers to display names for relay-path resolution.
  Map<String, String> _deviceNames() {
    return <String, String>{
      for (final device
          in _topology?.devices ??
              const <generated.RuntimeDeviceSpaceDevice>[])
        device.deviceId: device.deviceName,
    };
  }

  /// Returns the synchronized projection of one Space member, when it exists.
  generated.RuntimeDeviceSpaceDevice? _projectedDevice(String deviceId) {
    for (final device
        in _topology?.devices ?? const <generated.RuntimeDeviceSpaceDevice>[]) {
      if (device.deviceId == deviceId) {
        return device;
      }
    }
    return null;
  }

  /// Builds one row per device with a local pairing record.
  List<Widget> _pairedRows(AppLocalizations l10n, Map<String, String> names) {
    return <Widget>[
      for (final device in _pairedDevices.values) _pairedRow(l10n, names, device),
    ];
  }

  /// Builds the row of one paired device.
  ///
  /// Pairing is a local trust record, so the row stays readable even when the
  /// device is no longer in the Space: identity and route are simply absent.
  Widget _pairedRow(
    AppLocalizations l10n,
    Map<String, String> names,
    generated.RuntimePairedDevice device,
  ) {
    final deviceId = device.deviceId;
    final probeState =
        _pairedRemoteStates[deviceId] ?? _PairedRemoteProbeState.checking;
    final request = _spaceJoins[deviceId];
    final pendingJoin = request != null && spaceJoinIsActive(request.status);
    final member = _currentDeviceSpace?.members.contains(deviceId) ?? false;
    final projected = _projectedDevice(deviceId);
    final status =
        '${_remoteProbeLabel(probeState, l10n)}'
        '${_reachabilitySuffix(l10n, projected)}';
    final path = _relayPathText(l10n, names, projected);
    return _DeviceSpaceRow(
      leading: _pairedLeading(probeState),
      title: _deviceInfoName(device.deviceInfo),
      identity: _identityLabel(l10n, projected),
      status: status,
      tag: pendingJoin
          ? '${spaceJoinStatusText(request, l10n)}${request.reviewerName == null ? '' : ' · ${request.reviewerName}'}'
          : member
          ? l10n.deviceSpaceMemberPaired
          : l10n.deviceSpacePairedOnly,
      path: path,
      actions: _pairedActions(
        l10n,
        device,
        request,
        probeState,
        pendingJoin,
        member,
      ),
      onTap: () => _showDeviceDetails(
        icon: _remoteProbeIconData(probeState),
        name: _deviceInfoName(device.deviceInfo),
        deviceId: deviceId,
        statusText: status,
        pathText: path,
        hardwareText: _hardwareText(projected, device.deviceInfo),
        identityLabel: _identityLabel(l10n, projected),
        capabilityLabels: _capabilityLabels(l10n, projected),
      ),
    );
  }

  /// Builds the paired-device entries that only touch local trust records.
  ///
  /// Joining, reviewing, and unpairing are applicant-side actions, so they are
  /// offered to every identity; a removed-from-space device keeps its recovery
  /// action visible until the local record is dropped.
  List<Widget> _pairedActions(
    AppLocalizations l10n,
    generated.RuntimePairedDevice device,
    generated.SpaceJoinRequest? request,
    _PairedRemoteProbeState probeState,
    bool pendingJoin,
    bool member,
  ) {
    return <Widget>[
      if (pendingJoin && request != null)
        MenuItemButton(
          onPressed: _busy ? null : () => _showJoinProgress(request),
          child: Text(l10n.deviceSpaceViewRequest),
        ),
      if (device.outbound &&
          !pendingJoin &&
          !member &&
          probeState == _PairedRemoteProbeState.online)
        MenuItemButton(
          onPressed: _busy
              ? null
              : () => _offerJoiningExistingPairedDeviceSpace(device),
          child: Text(l10n.settingsRuntimeJoinSpace),
        ),
      if (probeState == _PairedRemoteProbeState.removedFromSpace)
        MenuItemButton(
          onPressed: _busy ? null : _handleRemovedFromSpace,
          child: Text(l10n.settingsRuntimeRemovedFromSpaceConfirm),
        ),
      MenuItemButton(
        onPressed: _busy ? null : () => _deletePairedDevice(device.deviceId),
        child: Text(l10n.deviceSpaceUnpair),
      ),
    ];
  }

  /// Opens the join-request progress of one paired device.
  Future<void> _showJoinProgress(generated.SpaceJoinRequest request) async {
    final space = await showSpaceJoinProgress(
      context,
      clients: _clients,
      request: request,
    );
    if (mounted && space != null) {
      await _handleJoinedDeviceSpace(space);
    }
  }

  /// Builds one row per Space member without a direct pairing.
  List<Widget> _memberRows(AppLocalizations l10n, Map<String, String> names) {
    final topology = _topology;
    if (topology == null) {
      return const <Widget>[];
    }
    return <Widget>[
      for (final device in topology.devices)
        if (device.deviceId != topology.currentDeviceId &&
            !_pairedDevices.containsKey(device.deviceId))
          _memberRow(l10n, names, device),
    ];
  }

  /// Builds the row of one relayed Space member.
  Widget _memberRow(
    AppLocalizations l10n,
    Map<String, String> names,
    generated.RuntimeDeviceSpaceDevice device,
  ) {
    final scheme = Theme.of(context).colorScheme;
    final icon = device.online ? Icons.lan_outlined : Icons.link_off_outlined;
    final status =
        '${_onlineText(l10n, device.online)}'
        '${_reachabilitySuffix(l10n, device)}';
    final path = _relayPathText(l10n, names, device);
    return _DeviceSpaceRow(
      leading: Icon(icon, color: device.online ? scheme.primary : scheme.error),
      title: device.deviceName,
      identity: _identityLabel(l10n, device),
      status: status,
      tag: l10n.deviceSpaceMemberNotPaired,
      path: path,
      actions: _memberActions(l10n, device),
      onTap: () => _showDeviceDetails(
        icon: icon,
        name: device.deviceName,
        deviceId: device.deviceId,
        statusText: status,
        pathText: path,
        hardwareText: _hardwareText(device, null),
        identityLabel: _identityLabel(l10n, device),
        capabilityLabels: _capabilityLabels(l10n, device),
      ),
    );
  }

  /// Builds the device-management entries the local identity may run.
  ///
  /// Entries appear only when the synchronized policy grants the matching
  /// capability; an identity without any of them gets no menu at all.
  List<Widget> _memberActions(
    AppLocalizations l10n,
    generated.RuntimeDeviceSpaceDevice device,
  ) {
    return <Widget>[
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
  }

  /// Opens the read-only device details reachable without any capability.
  ///
  /// The Space projection already carries every member's identity and route,
  /// so the information stays visible to identities that cannot manage it.
  Future<void> _showDeviceDetails({
    required IconData icon,
    required String name,
    required String deviceId,
    required String statusText,
    required String? pathText,
    required String? hardwareText,
    required String? identityLabel,
    required List<String> capabilityLabels,
  }) async {
    final l10n = AppLocalizations.of(context)!;
    await showDialog<void>(
      context: context,
      builder: (dialogContext) {
        final scheme = Theme.of(dialogContext).colorScheme;
        final textTheme = Theme.of(dialogContext).textTheme;
        final lines = <Widget>[
          if (identityLabel != null)
            Text(
              '${l10n.settingsRuntimeControlCurrentIdentity}: $identityLabel',
              style: textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w700),
            ),
          if (capabilityLabels.isNotEmpty)
            Text(
              '${l10n.settingsRuntimeControlCurrentCapabilities}: ${capabilityLabels.join('、')}',
              style: textTheme.bodySmall?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            ),
          Text(
            statusText,
            style: textTheme.bodySmall?.copyWith(color: scheme.onSurfaceVariant),
          ),
          if (pathText != null)
            Text(
              pathText,
              style: textTheme.bodySmall?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            ),
          if (hardwareText != null)
            Text(
              hardwareText,
              style: textTheme.bodySmall?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            ),
          Text(
            '${l10n.settingsRuntimeControlDeviceId}: $deviceId',
            style: textTheme.bodySmall?.copyWith(color: scheme.onSurfaceVariant),
          ),
        ];
        return AlertDialog(
          title: Row(
            children: <Widget>[
              Icon(icon, color: scheme.primary),
              const SizedBox(width: 10),
              Flexible(
                child: Text(
                  name,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
          content: SizedBox(
            width: 380,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                for (final line in lines)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 6),
                    child: line,
                  ),
              ],
            ),
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(),
              child: Text(
                MaterialLocalizations.of(dialogContext).closeButtonLabel,
              ),
            ),
          ],
        );
      },
    );
  }

  /// Names one device state.
  String _onlineText(AppLocalizations l10n, bool online) => online
      ? l10n.settingsRuntimePairedOnline
      : l10n.settingsRuntimePairedOffline;

  /// Describes how one member is currently reached: directly or through relays.
  String _reachabilitySuffix(
    AppLocalizations l10n,
    generated.RuntimeDeviceSpaceDevice? device,
  ) {
    if (device == null || !device.online) {
      return '';
    }
    final hops = device.relayHops;
    return ' · ${hops == null ? l10n.deviceSpaceDirectLink : l10n.deviceSpaceRelayHops(hops)}';
  }

  /// Renders the planned relay path from this device through every hop to the member.
  String? _relayPathText(
    AppLocalizations l10n,
    Map<String, String> names,
    generated.RuntimeDeviceSpaceDevice? device,
  ) {
    final path = device?.relayPath ?? const <String>[];
    if (path.isEmpty) {
      return null;
    }
    return '${l10n.deviceSpaceRelayPath}: ${[
      l10n.deviceSpaceLocalDevice,
      for (final hop in path) names[hop] ?? hop,
    ].join(' → ')}';
  }

  /// Names the Space identity of one member, when the projection carries it.
  String? _identityLabel(
    AppLocalizations l10n,
    generated.RuntimeDeviceSpaceDevice? device,
  ) {
    final identity = device?.currentIdentity;
    return identity == null
        ? null
        : spaceIdentityDisplayName(identity.displayName, l10n);
  }

  /// Lists the readable capabilities of one member's identity.
  List<String> _capabilityLabels(
    AppLocalizations l10n,
    generated.RuntimeDeviceSpaceDevice? device,
  ) {
    final identity = device?.currentIdentity;
    if (identity == null) {
      return const <String>[];
    }
    return spaceCapabilityValues(identity.capabilities)
        .map((value) => spaceCapabilityLabel(value, l10n))
        .toList(growable: false);
  }

  /// Describes platform and Core version of one device for the details view.
  String? _hardwareText(
    generated.RuntimeDeviceSpaceDevice? device,
    generated.LinkDeviceInfo? fallback,
  ) {
    final values = device != null
        ? <String>[device.platform, device.coreVersion ?? '']
        : <String>[fallback?.platform ?? '', fallback?.model ?? ''];
    final text = values
        .where((value) => value.trim().isNotEmpty)
        .join(' · ');
    return text.isEmpty ? null : text;
  }

  /// Builds the leading status glyph of one paired device.
  Widget _pairedLeading(_PairedRemoteProbeState state) {
    if (state == _PairedRemoteProbeState.checking) {
      return const SizedBox(
        width: 24,
        height: 24,
        child: Center(child: M3LoadingIndicator(size: 18)),
      );
    }
    return Icon(
      _remoteProbeIconData(state),
      color: _remoteProbeColor(Theme.of(context).colorScheme, state),
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
          ..._deviceSectionChildren(l10n),
        ],
      ),
      _SectionCard(
        title: l10n.settingsRuntimeIdentitiesAndPermissions,
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

/// Presents one device identically whether or not it has a local pairing.
///
/// The row body always carries the readable information; only the trailing
/// overflow menu depends on the local identity's device capabilities.
class _DeviceSpaceRow extends StatelessWidget {
  const _DeviceSpaceRow({
    required this.leading,
    required this.title,
    required this.identity,
    required this.status,
    required this.tag,
    required this.path,
    required this.actions,
    required this.onTap,
  });

  final Widget leading;
  final String title;
  final String? identity;
  final String status;
  final String? tag;
  final String? path;
  final List<Widget> actions;
  final VoidCallback onTap;

  /// Builds the row and its tap target for the read-only details.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Material(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.22),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(
          color: colorScheme.outlineVariant.withValues(alpha: 0.45),
        ),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(12, 8, 6, 8),
          child: Row(
            children: <Widget>[
              leading,
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Row(
                      children: <Widget>[
                        Flexible(
                          child: Text(
                            title,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: textTheme.labelLarge?.copyWith(
                              fontWeight: FontWeight.w700,
                            ),
                          ),
                        ),
                        if (identity != null) ...<Widget>[
                          const SizedBox(width: 6),
                          _IdentityBadge(label: identity!),
                        ],
                      ],
                    ),
                    const SizedBox(height: 2),
                    Text(
                      '$status${tag == null ? '' : ' · $tag'}',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.labelSmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                    if (path != null) ...<Widget>[
                      const SizedBox(height: 2),
                      Text(
                        path!,
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
              const SizedBox(width: 6),
              _DeviceRowMenu(actions: actions),
            ],
          ),
        ),
      ),
    );
  }
}

/// Keeps the trailing action slot stable whether or not a menu is available.
class _DeviceRowMenu extends StatelessWidget {
  const _DeviceRowMenu({required this.actions});

  final List<Widget> actions;

  /// Builds the overflow menu, or the empty slot that holds its place.
  @override
  Widget build(BuildContext context) {
    if (actions.isEmpty) {
      return const SizedBox(width: 40, height: 40);
    }
    return MenuAnchor(
      builder: (context, controller, _) => IconButton(
        tooltip: AppLocalizations.of(context)!.deviceSpaceManageDevice,
        onPressed: controller.open,
        visualDensity: VisualDensity.compact,
        icon: const Icon(Icons.more_vert_outlined),
      ),
      menuChildren: actions,
    );
  }
}

/// Shows one device's Space identity beside its name.
class _IdentityBadge extends StatelessWidget {
  const _IdentityBadge({required this.label});

  final String label;

  /// Builds the compact identity badge.
  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 120),
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: scheme.secondaryContainer.withValues(alpha: 0.6),
          borderRadius: BorderRadius.circular(6),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
          child: Text(
            label,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: scheme.onSecondaryContainer,
              fontWeight: FontWeight.w700,
            ),
          ),
        ),
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

/// Returns the status glyph of one paired-device probe state.
IconData _remoteProbeIconData(_PairedRemoteProbeState state) {
  return switch (state) {
    _PairedRemoteProbeState.checking => Icons.cloud_queue_outlined,
    _PairedRemoteProbeState.online => Icons.cloud_done_outlined,
    _PairedRemoteProbeState.offline => Icons.cloud_off_outlined,
    _PairedRemoteProbeState.invalid => Icons.link_off_outlined,
    _PairedRemoteProbeState.error => Icons.error_outline,
    _PairedRemoteProbeState.removedFromSpace => Icons.person_remove_outlined,
  };
}

/// Returns the status color of one paired-device probe state.
Color _remoteProbeColor(ColorScheme scheme, _PairedRemoteProbeState state) {
  return switch (state) {
    _PairedRemoteProbeState.checking => scheme.onSurfaceVariant,
    _PairedRemoteProbeState.online => scheme.primary,
    _PairedRemoteProbeState.offline => scheme.error,
    _PairedRemoteProbeState.invalid => scheme.error,
    _PairedRemoteProbeState.error => scheme.error,
    _PairedRemoteProbeState.removedFromSpace => scheme.error,
  };
}

/// Names one paired-device probe state.
String _remoteProbeLabel(_PairedRemoteProbeState state, AppLocalizations l10n) {
  return switch (state) {
    _PairedRemoteProbeState.checking => l10n.settingsRuntimePairedChecking,
    _PairedRemoteProbeState.online => l10n.settingsRuntimePairedOnline,
    _PairedRemoteProbeState.offline => l10n.settingsRuntimePairedOffline,
    _PairedRemoteProbeState.invalid => l10n.settingsRuntimePairedInvalid,
    _PairedRemoteProbeState.error => l10n.settingsRuntimePairedError,
    _PairedRemoteProbeState.removedFromSpace =>
      l10n.settingsRuntimePairedRemovedFromSpace,
  };
}
